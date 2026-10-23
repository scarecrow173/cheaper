//! unified prepare — fixed grant(process surface)の per-call 前処理
//! (issue #165 — design §(h))。旧 `tools/process/prepare.rs` を
//! unified `ExecGrant` へ汎化して移設。
//!
//! 責務: cwd 解決 → (`inputs`/`outputs` 宣言 grant のみ)manifest
//! normalize → declared-path deny → normalized claim deny →
//! writeScope coverage(`write_scope` 存在時、`outputs ⊆ writeScope`
//! でないと `POLICY_VIOLATION`)→ pin 検索 + spawn 前再検証 →
//! `PreparedExec{executable, cwd}`。
//!
//! writeScope coverage の確定分岐(M4 — `outputs` tri-state):
//! - `outputs: Some(paths)`(非空)— manifest normalize +
//!   `claim_contains` で `outputs ⊆ writeScope` を検査。
//! - `outputs: Some([])` — 「副作用を宣言しない」契約。coverage は
//!   trivially 成立(writes 空 → uncovered なし)。
//! - `outputs: None`(未宣言)— `write_scope` が `Some` のとき
//!   coverage を証明不能(副作用が resource 層へ不可視)として
//!   `POLICY_VIOLATION` で fail-closed。`write_scope` が `None`
//!   なら coverage 検査不要で通常実行。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::policy::exec::{exec_grant_resources, ExecGrant};
use cheaper_core::resources::normalization::{
    normalize_resource_manifest, ResourceManifestDeclaration, ScopeDeclaration,
};
use cheaper_core::resources::predicates::claim_contains;
use serde_json::{json, Map, Value};

use super::{grant_violation, verify_pinned_executable, ExecToolOptions};
use crate::tools::proc::{denied_declared_path, prepare_cwd};
use crate::ToolError;

fn deny(display: &str) -> ToolError {
    denied_declared_path(display, "process")
}

pub(crate) struct PreparedExec {
    pub(crate) executable: String,
    pub(crate) cwd: String,
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

/// `prepare_exec_grant`: named fixed grant → workspace cwd 解決 +
/// deny + isDirectory → (宣言 grant のみ)manifest normalize →
/// 宣言 path/claim の deny → writeScope tri-state 検査 →
/// pin map 検索 + spawn 前再検証。
pub(crate) fn prepare_exec_grant(
    options: &ExecToolOptions,
    name: &str,
    grant: &ExecGrant,
) -> Result<PreparedExec, ToolError> {
    let cwd = prepare_cwd(
        &options.root,
        &grant.cwd,
        options.symlink_policy,
        options.denylist.as_ref(),
        "process",
    )?;
    // `inputs`/`outputs` のいずれかを宣言する grant のみ manifest
    // 経路へ乗せる(未宣言 grant は conservative root manifest を
    // `exec_grant_resources` が返すが、normalize して claim 化する
    // 実効は writeScope 証明不能の fail-closed で完結するため、
    // declared contract のない normalize は skip — design §(h))。
    if grant.inputs.is_some() || grant.outputs.is_some() {
        let declaration = exec_grant_resources(&options.exec, name)?;
        let manifest = normalize_resource_manifest(
            &options.root,
            Some(&declaration_value(&declaration)),
            options.symlink_policy,
        )?;
        for input in grant
            .inputs
            .iter()
            .flatten()
            .chain(grant.outputs.iter().flatten())
        {
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
            // `outputs: Some([])` は writes 空で uncovered なし —
            // coverage は trivially 成立する(tri-state 規約)。
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
    } else if options.write_scope.is_some() {
        // `outputs: None` + `write_scope` — 副作用が resource 層へ
        // 不可視で coverage を証明不能のため fail-closed(M4 確定
        // 分岐 — design §(h))。
        return Err(CheaperError::new(
            ErrorCode::PolicyViolation,
            "process outputs exceed worker writeScope",
        )
        .into());
    }
    // pin map 検索 + spawn 前再検証 — task executable の pin は scope
    // 構築の `pin_exec_grants` 済み(旧 lazy resolve は除去 —
    // issue #165 で初めて task executable も pin 対象)。pin 不在は
    // fail-closed。
    let executable = match options.pins.get(name) {
        Some(pinned) => {
            verify_pinned_executable(name, pinned, &options.exec_deny)?;
            pinned.clone()
        }
        None => return Err(grant_violation(name, "no pinned executable")),
    };
    Ok(PreparedExec { executable, cwd })
}
