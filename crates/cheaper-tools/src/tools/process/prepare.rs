//! prepare 責務: task 固有の manifest / writeScope coverage 検査 +
//! executable 解決(TS `prepare` 相当の process 固有部分)。
//! cwd/denylist/is_dir stat と `minimal_env` は shared
//! `crate::tools::proc` へ移設済み(design §(e))。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::policy::process::{process_task_resources, ProcessTask};
use cheaper_core::resources::normalization::{
    normalize_resource_manifest, ResourceManifestDeclaration, ScopeDeclaration,
};
use cheaper_core::resources::predicates::claim_contains;
use serde_json::{json, Map, Value};

use super::ProcessToolOptions;
use crate::tools::proc::executable::{resolve_process_executable, ProcessExecutableOptions};
use crate::tools::proc::{denied_declared_path, prepare_cwd};
use crate::ToolError;

fn deny(display: &str) -> ToolError {
    denied_declared_path(display, "process")
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
    let cwd = prepare_cwd(
        &options.root,
        &task.cwd,
        options.symlink_policy,
        options.denylist.as_ref(),
        "process",
    )?;
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
        "process",
    )?;
    Ok(Prepared { executable, cwd })
}

fn unsupported_empty_command() -> ToolError {
    CheaperError::new(
        ErrorCode::PolicyViolation,
        "process requires a supported native executable; shell and script shims are unsupported",
    )
    .into()
}
