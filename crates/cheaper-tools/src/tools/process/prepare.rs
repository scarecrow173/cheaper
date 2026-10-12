//! prepare 責務: cwd/deny/writeScope 検査・minimal env 構築・executable
//! 解決(TS `prepare` 相当)。

use cheaper_core::errors::{node_io_error, CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::policy::process::{process_task_resources, ProcessTask};
use cheaper_core::resources::normalization::{
    normalize_resource_manifest, ResourceManifestDeclaration, ScopeDeclaration,
};
use cheaper_core::resources::predicates::claim_contains;
use cheaper_core::workspace::paths::resolve_workspace_path;
use serde_json::{json, Map, Value};

use super::executable::{resolve_process_executable, ProcessExecutableOptions};
use super::ProcessToolOptions;
use crate::ToolError;

/// `minimalEnv(explicit)`: task.env のみ + Windows required env 空値遮断 +
/// SystemRoot 補完。`NODE_V8_COVERAGE: undefined` は spawn 側の
/// `env_clear` で「未設定」を再現するためここでは未出力(TS の own
/// undefined = 継承遮断の意味と同一)。
pub(super) fn minimal_env(explicit: &Map<String, Value>) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    for (key, value) in explicit {
        let name = if cfg!(windows) {
            key.to_uppercase()
        } else {
            key.clone()
        };
        env.push((name, value.as_str().unwrap_or_default().to_string()));
    }
    #[cfg(windows)]
    {
        // libuv は未指定の Windows required env を親から補完する。空値で
        // その継承を止める。
        const REQUIRED: &[&str] = &[
            "HOMEDRIVE",
            "HOMEPATH",
            "LOGONSERVER",
            "PATH",
            "SYSTEMDRIVE",
            "TEMP",
            "USERDOMAIN",
            "USERNAME",
            "USERPROFILE",
            "WINDIR",
        ];
        let missing: Vec<&str> = REQUIRED
            .iter()
            .copied()
            .filter(|key| !env.iter().any(|(k, _)| k == key))
            .collect();
        for key in missing {
            env.push((key.to_string(), String::new()));
        }
        if !env.iter().any(|(k, _)| k == "SYSTEMROOT") {
            env.push((
                "SYSTEMROOT".to_string(),
                std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string()),
            ));
        }
    }
    env
}

fn deny(display: &str) -> ToolError {
    let mut details = Map::new();
    details.insert("path".to_string(), json!(display));
    CheaperError::new(
        ErrorCode::PolicyViolation,
        "Access denied for declared process path",
    )
    .with_details(details)
    .into()
}

pub(super) struct Prepared {
    pub(super) executable: String,
    pub(super) cwd: String,
}

fn declaration_value(declaration: &ResourceManifestDeclaration) -> Value {
    let scopes = |items: &Option<Vec<ScopeDeclaration>>| -> Value {
        json!(items
            .iter()
            .flatten()
            .map(|s| json!({"path": s.path, "scope": s.scope.as_str()}))
            .collect::<Vec<_>>())
    };
    let mut out = Map::new();
    if declaration.reads.is_some() {
        out.insert("reads".to_string(), scopes(&declaration.reads));
    }
    if declaration.writes.is_some() {
        out.insert("writes".to_string(), scopes(&declaration.writes));
    }
    if declaration.produces.is_some() {
        out.insert("produces".to_string(), scopes(&declaration.produces));
    }
    Value::Object(out)
}

/// `prepare`: named task → workspace cwd 解決 + deny + isDirectory →
/// manifest normalize → 宣言 path/claim の deny → writeScope 検査 →
/// executable 解決。
pub(super) fn prepare(
    options: &ProcessToolOptions,
    name: &str,
    task: &ProcessTask,
) -> Result<Prepared, ToolError> {
    let cwd = resolve_workspace_path(options.root.as_str(), &task.cwd, options.symlink_policy)?;
    if let Some(denylist) = &options.denylist {
        if denylist.is_denied(&cwd.display) {
            return Err(deny(&cwd.display));
        }
    }
    // `stat` 自体の失敗は Node `UVException` shape で伝播する(TS は
    // `fs.stat` の ENOENT をそのまま throw)。isDirectory=false のみ
    // INVALID_REQUEST。
    let cwd_stat = std::fs::metadata(&cwd.real)
        .map_err(|e| Box::new(node_io_error("stat", &cwd.real, e)) as ToolError)?;
    if !cwd_stat.is_dir() {
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            "process cwd must be an existing directory",
        )
        .into());
    }
    let declaration = process_task_resources(&options.process, name)?;
    let manifest = normalize_resource_manifest(
        &options.root,
        Some(&declaration_value(&declaration)),
        options.symlink_policy,
    )?;
    for input in task.inputs.iter().chain(&task.outputs) {
        if let Some(denylist) = &options.denylist {
            if denylist.is_denied(input) {
                return Err(deny(input));
            }
        }
    }
    for claim in manifest
        .reads
        .iter()
        .chain(&manifest.writes)
        .chain(&manifest.produces)
    {
        let rel = nodepath::relative(&options.root, claim.resource.path());
        let display = if rel.is_empty() {
            ".".to_string()
        } else {
            rel.split(nodepath::SEP).collect::<Vec<_>>().join("/")
        };
        if let Some(denylist) = &options.denylist {
            if denylist.is_denied(&display) {
                return Err(deny(&display));
            }
        }
    }
    if let Some(write_scope) = &options.write_scope {
        let allowed = normalize_resource_manifest(
            &options.root,
            Some(&json!({
                "reads": [],
                "writes": [],
                "produces": write_scope
                    .iter()
                    .map(|value| json!({"path": value, "scope": "exact"}))
                    .collect::<Vec<_>>(),
            })),
            options.symlink_policy,
        )?;
        let uncovered = manifest.writes.iter().any(|claim| {
            !allowed
                .produces
                .iter()
                .any(|scope| claim_contains(scope, claim))
        });
        if uncovered {
            return Err(CheaperError::new(
                ErrorCode::PolicyViolation,
                "process outputs exceed worker writeScope",
            )
            .into());
        }
    }
    let command = task
        .command
        .first()
        .ok_or_else(|| unsupported_empty_command())?;
    let executable = resolve_process_executable(
        command,
        &ProcessExecutableOptions {
            root: &options.root,
            symlink_policy: options.symlink_policy,
            denylist: options.denylist.as_ref(),
            search_path: None,
        },
    )?;
    Ok(Prepared {
        executable,
        cwd: cwd.real,
    })
}

fn unsupported_empty_command() -> ToolError {
    CheaperError::new(
        ErrorCode::PolicyViolation,
        "process requires a supported native executable; shell and script shims are unsupported",
    )
    .into()
}
