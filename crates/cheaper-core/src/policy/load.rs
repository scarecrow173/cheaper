//! Port of `packages/core/src/policy/load.ts` — the §23 load order:
//! built-in (complete) → user (partial) merge → invocation (grants-only
//! synthetic partial) merge → typed policy completion → project
//! (partial) `restrict-concurrency` merge → complete validation →
//! normalize.
//!
//! The `invocation` layer is a trusted, non-persisted layer supplied by
//! the invoker (`--grant`) for the lifetime of a single run
//! (ADR-034FlTS3BaiV3WKz8bX1Xv, Option A). It is restricted to
//! `{"version":1,"command":{"grants":{..}}}` shape — top-level keys ⊆
//! `{version, command}` and `command` keys ⊆ `{grants}` — and merges
//! with `MergeMode::Replace` immediately after the user layer, so the
//! grant runtime semantics (argv pattern compile, executable pin,
//! env/cwd rules) are identical to user-policy grants. The project
//! layer cannot override it: `ProjectPartial` schema rejects the
//! `command` section outright.
//!
//! Accepted limitation (attribution gap): checks that only fire on the
//! merged map — e.g. `command.grants` `maxProperties: 128` exceeded by
//! the builtin + user + invocation total — are not reachable from the
//! invocation layer's standalone validation and surface as
//! `InvalidRuntimePolicy` without `details.layer`. Per-grant rule
//! violations are all caught by the layer's own Partial validate and do
//! carry `details.layer = "invocation"`.

use serde_json::{json, Map, Value};

use crate::config::layer::{read_yaml_layer, LayerKind};
use crate::errors::{CheaperError, ErrorCode};
use crate::policy::command::resolve_command_policy;
use crate::policy::merge::{merge_policy_layers, MergeMode};
use crate::policy::process::resolve_process_policy;
use crate::policy::schema::{
    normalize_policy, resolve_concurrency_policy, validate_policy_schema, PolicySchemaMode,
    RuntimePolicy,
};
use crate::policy::security::resolve_exec_deny_list;
use crate::policy::workflow::resolve_workflow_policy;

/// `readPolicyFile`: only `ENOENT`/`ENOTDIR` skip (absent file); other
/// read failures (EACCES, EISDIR, broken symlink) are errors, not silent
/// skips。
const POLICY_LAYER: LayerKind = LayerKind {
    code: ErrorCode::InvalidRuntimePolicy,
    file_label: "policy file",
    yaml_label: "policy yaml",
    object_label: "policy",
    event: "policy file read",
};

/// `policy.yaml` を読む public wrapper — `read_yaml_layer` +
/// `POLICY_LAYER` の薄い facade。`LayerKind`/`read_yaml_layer` 自体は
/// `pub(crate)` のまま(公開面は policy 用途に絞る)。absent
/// (ENOENT/ENOTDIR) は `Ok(None)`;`required` で absent は error。
/// `cheaper policy grant` の write 前 read-modify-write が本経路を
/// 使う(design §(g))。
pub fn read_policy_yaml(
    file_path: &str,
    required: bool,
) -> Result<Option<Map<String, Value>>, CheaperError> {
    read_yaml_layer(file_path, &POLICY_LAYER, required).map(|opt| {
        opt.map(|v| match v {
            Value::Object(m) => m,
            _ => unreachable!("read_yaml_layer returns objects only"),
        })
    })
}

/// `loadPolicy` options (`builtinPolicyPath`, `userConfigDir`,
/// `projectDir`, `invocation`).
pub struct LoadPolicyOptions<'a> {
    pub builtin_policy_path: &'a str,
    pub user_config_dir: &'a str,
    pub project_dir: &'a str,
    /// 合成 trusted layer — `{"version":1,"command":{"grants":{..}}}` のみ
    /// 受理する grants-only 入力。user merge 直後・typed re-resolve 直前
    /// に Partial validate + Replace merge される。
    pub invocation: Option<&'a Map<String, Value>>,
}

/// error attribution 規約 — invocation 入力の構造的失敗(malformed 値・
/// `name` 欠落/pattern 違反・grants 以外の key・同名重複・policy との
/// collision)は `InvalidRequest` + `details.layer = "invocation"` で返す。
/// grant name が関与する場合は `details.grant` に載せる。
fn invocation_invalid_request(message: impl Into<String>, details: Value) -> CheaperError {
    let mut details = details.as_object().cloned().unwrap_or_default();
    details.insert("layer".to_string(), Value::from("invocation"));
    CheaperError::new(ErrorCode::InvalidRequest, message).with_details(details)
}

/// error attribution 規約 — shared validation(`validate_policy_schema` /
/// `resolve_command_policy`)由来の失敗は inner error code(通常
/// `InvalidRuntimePolicy`)を保持し、既存 details へ
/// `layer = "invocation"` を畳み込む。同一 spec を user policy に置いた
/// 場合と error code を揃え、policy 内容由来の失敗であることを表す。
fn with_invocation_layer(e: CheaperError) -> CheaperError {
    let mut details = e.details().cloned().unwrap_or_default();
    details.insert("layer".to_string(), Value::from("invocation"));
    e.with_details(details)
}

/// `loadPolicy` — 引継ぎ §23 load order. An invalid policy is never
/// silently ignored.
pub fn load_policy(opts: &LoadPolicyOptions<'_>) -> Result<RuntimePolicy, CheaperError> {
    let builtin_raw =
        read_policy_yaml(opts.builtin_policy_path, true)?.expect("required policy file");
    let mut effective =
        validate_policy_schema(&Value::Object(builtin_raw), PolicySchemaMode::Complete)?;

    let user_path = nodepath_join(opts.user_config_dir, "policy.yaml");
    if let Some(user) = read_policy_yaml(&user_path, false)? {
        tracing::debug!(policy.layer = "user", "policy layer merged");
        effective = merge_policy_layers(
            &effective,
            &validate_policy_schema(&Value::Object(user), PolicySchemaMode::Partial)?,
            MergeMode::Replace,
        )
        .expect("replace-mode merge cannot fail");
    }
    if let Some(invocation) = opts.invocation {
        // (1) grants-only key 制約 — schema ではなく merge 前検査。
        // Partial schema だけでは `execution`/`tools`/`model` 等他
        // section が素通りするため、合成 layer を「grant 供給のみ」へ
        // 縛る構造的 guard として先に検査する。
        for key in invocation.keys() {
            if key != "version" && key != "command" {
                return Err(invocation_invalid_request(
                    format!("invocation layer may only declare command.grants: {key}"),
                    json!({"key": key}),
                ));
            }
        }
        match invocation.get("command").map(Value::as_object) {
            Some(None) => {
                return Err(invocation_invalid_request(
                    "invocation layer may only declare command.grants",
                    json!({"key": "command"}),
                ));
            }
            Some(Some(command)) => {
                for key in command.keys() {
                    if key != "grants" {
                        return Err(invocation_invalid_request(
                            format!("invocation layer may only declare command.grants: {key}"),
                            json!({"key": format!("command.{key}")}),
                        ));
                    }
                }
            }
            None => {}
        }
        // (2) Partial validate — `resolve_command_policy` が
        // `ArgPattern::compile`・env/cwd 規則を継承する。
        // shared validation 由来の失敗は inner code を保持し
        // `details.layer` のみ畳み込む。
        let validated = validate_policy_schema(
            &Value::Object(invocation.clone()),
            PolicySchemaMode::Partial,
        )
        .map_err(with_invocation_layer)?;
        // (3) builtin + user の merge 済み `command.grants` との
        // collision 検査 — Replace merge は同名 grant を黙って上書きする
        // ため fail-loud。validate 通過後に行い、malformed spec が
        // collision error を装わない。
        let existing_grants = effective
            .get("command")
            .and_then(Value::as_object)
            .and_then(|command| command.get("grants"))
            .and_then(Value::as_object);
        let incoming_grants = validated
            .get("command")
            .and_then(Value::as_object)
            .and_then(|command| command.get("grants"))
            .and_then(Value::as_object);
        if let (Some(existing), Some(incoming)) = (existing_grants, incoming_grants) {
            if let Some(name) = incoming.keys().find(|k| existing.contains_key(*k)) {
                return Err(invocation_invalid_request(
                    format!("invocation grant collides with an existing grant: {name}"),
                    json!({"grant": name}),
                ));
            }
        }
        // (4) Replace merge。(5) `policy.layer` record は sibling
        // (user/project)と同様 merge 呼出しの直前に揃える — post-merge
        // 配置だと log 行順が merge 結果の帰属を誤読させる。
        tracing::debug!(policy.layer = "invocation", "policy layer merged");
        effective = merge_policy_layers(&effective, &validated, MergeMode::Replace)
            .expect("replace-mode merge cannot fail");
    }
    // After the trusted merge and before the project restriction, fill
    // legacy/partial omissions with the typed policy resolvers.
    let mut with_defaults = effective.clone();
    with_defaults.insert(
        "workflow".to_string(),
        serde_json::to_value(resolve_workflow_policy(effective.get("workflow"))?)
            .unwrap_or(Value::Null),
    );
    let resolved_process = resolve_process_policy(effective.get("process"))?;
    // Re-embeds the resolved struct; serde field names/order reproduce
    // `{maxConcurrent, tasks, timeout, maxOutputBytes, killGrace,
    // heartbeat: {interval}}`.
    with_defaults.insert(
        "process".to_string(),
        serde_json::to_value(&resolved_process).unwrap_or(Value::Null),
    );
    // `security` section から compiled deny を 1 回作り、command の
    // re-resolve へ渡す — **merged effective(invocation grant 含む)
    // への deny 評価点**(issue #164 適用点 1 が merged map に発火)。
    // merged-map 評価のため deny 元 layer は帰属不能 —
    // `details.layer` は付かない(attribution gap は本 file 冒頭の
    // accepted limitation と同構造)。
    let exec_deny = resolve_exec_deny_list(effective.get("security"))?;
    // `command` も同型に re-embed する — user partial が `grants` のみを
    // 宣言した場合に complete schema の section required
    // (`["maxConcurrent", "grants"]`)を resolved default で満たす。
    let resolved_command = resolve_command_policy(effective.get("command"), &exec_deny)?;
    with_defaults.insert(
        "command".to_string(),
        serde_json::to_value(&resolved_command).unwrap_or(Value::Null),
    );
    let mut execution = effective
        .get("execution")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let concurrency = resolve_concurrency_policy(execution.get("concurrency"))?;
    execution.insert(
        "concurrency".to_string(),
        json!({
            "maxOperations": concurrency.max_operations,
            "mutations": concurrency.mutations.as_str(),
        }),
    );
    with_defaults.insert("execution".to_string(), Value::Object(execution));
    effective = with_defaults;

    let project_path = nodepath_join(opts.project_dir, "policy.yaml");
    if let Some(project) = read_policy_yaml(&project_path, false)? {
        tracing::debug!(policy.layer = "project", "policy layer merged");
        // Option A の信頼境界: project partial schema は `command` section
        // 全体を除外し、含む project policy は schema validation で
        // `InvalidRuntimePolicy` に落ちる(grant authoring は trusted
        // layer のみ)。
        effective = merge_policy_layers(
            &effective,
            &validate_policy_schema(&Value::Object(project), PolicySchemaMode::ProjectPartial)?,
            MergeMode::RestrictConcurrency,
        )
        .map_err(|e| {
            CheaperError::new(
                ErrorCode::InvalidRuntimePolicy,
                format!("invalid project policy duration: {}", e.input()),
            )
        })?;
    }

    let completed = validate_policy_schema(&Value::Object(effective), PolicySchemaMode::Complete)?;
    let policy = normalize_policy(&completed)?;
    tracing::debug!("policy resolved");
    Ok(policy)
}

/// `path.join(dir, "policy.yaml")` through `nodepath` so the byte shape
/// matches TS on win32.
fn nodepath_join(dir: &str, name: &str) -> String {
    crate::nodepath::join(&[dir, name])
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILTIN: &str = "\
version: 1
execution: {timeout: 5m, maxTurns: 20, maxToolCalls: 40}
tools: {allow: [read, grep, find, ls]}
filesystem: {symlinks: deny, denyPaths: []}
model: {maxTier: high, tierFallback: higher}
output: {maxTokens: 8192, repairAttempts: 1}
retry: {transientAttempts: 2}
";

    fn write(dir: &std::path::Path, name: &str, content: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(name), content).unwrap();
    }

    #[test]
    fn builtin_only_uses_defaults_for_workflow_process_concurrency() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin-policy.yaml", BUILTIN);
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin-policy.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().join("user").to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: None,
        })
        .unwrap();
        assert_eq!(p.execution.timeout_ms, 300_000);
        assert_eq!(p.execution.concurrency.unwrap().max_operations, 8);
        assert_eq!(p.workflow.max_jobs, 1);
        assert_eq!(p.process.max_concurrent, 1);
        assert_eq!(p.process.timeout, "2m");
    }

    #[test]
    fn user_partial_overrides_project_restricts() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        let user = tmp.path().join("user");
        write(
            &user,
            "policy.yaml",
            "version: 1\nexecution: {timeout: 10m, concurrency: {maxOperations: 4}}\n",
        );
        let project = tmp.path().join("proj");
        // project tries to widen maxOperations → clamped to 4.
        write(
            &project,
            "policy.yaml",
            "version: 1\nexecution: {concurrency: {maxOperations: 16}}\nprocess: {timeout: 10m}\n",
        );
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: project.to_str().unwrap(),
            invocation: None,
        })
        .unwrap();
        assert_eq!(p.execution.timeout_ms, 600_000);
        assert_eq!(p.execution.concurrency.unwrap().max_operations, 4);
        assert_eq!(p.process.timeout, "2m"); // 10m clamped to default 2m
    }

    #[test]
    fn route_approval_bypass_merges_and_project_route_fails() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        let user = tmp.path().join("user");
        write(
            &user,
            "policy.yaml",
            "version: 1\nroute:\n  rewrite:\n    approvalBypass: [codex]\n",
        );
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: None,
        })
        .unwrap();
        // user 層の `route` merge が effective policy に反映(array は
        // wholesale replace)。flag 判定は `approval_bypass_enabled` 経由。
        assert_eq!(p.route.rewrite.approval_bypass, vec!["codex".to_string()]);
        assert!(p.route.rewrite.approval_bypass_enabled("codex"));
        assert!(!p.route.rewrite.approval_bypass_enabled("cursor"));

        // project 層の `route` key は project partial schema 違反で
        // `load_policy` 全体が失敗 — untrusted project が approval
        // bypass を有効化できない構造保証(`command` section 除外と同型)。
        let project = tmp.path().join("proj");
        write(
            &project,
            "policy.yaml",
            "version: 1\nroute:\n  rewrite:\n    approvalBypass: [codex]\n",
        );
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().join("user2").to_str().unwrap(),
            project_dir: project.to_str().unwrap(),
            invocation: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    #[test]
    fn missing_builtin_is_invalid_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("absent.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    #[test]
    fn invalid_project_partial_is_not_silently_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        let project = tmp.path().join("proj");
        write(&project, "policy.yaml", "version: 2\n");
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: project.to_str().unwrap(),
            invocation: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    /// `{"version":1,"command":{"grants":{..}}}` shape の invocation
    /// layer object。
    fn invocation_layer(grants: Value) -> Map<String, Value> {
        json!({"version": 1, "command": {"grants": grants}})
            .as_object()
            .unwrap()
            .clone()
    }

    fn layer_detail(e: &CheaperError) -> Option<&Value> {
        e.details().and_then(|d| d.get("layer"))
    }

    #[test]
    fn invocation_grants_merge_after_user_layer() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        let user = tmp.path().join("user");
        write(
            &user,
            "policy.yaml",
            "version: 1\ncommand:\n  grants:\n    git-log:\n      executable: git\n",
        );
        let invocation = invocation_layer(json!({
            "cargo-fmt": {"executable": "cargo", "argv": ["fmt"]},
        }));
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: Some(&invocation),
        })
        .unwrap();
        // user grant はそのまま残り、invocation grant が追加される。
        assert_eq!(p.command.grants["git-log"].executable, "git");
        assert_eq!(
            p.command.grants["cargo-fmt"].argv,
            Some(vec!["fmt".to_string()])
        );
    }

    #[test]
    fn invocation_grants_survive_project_layer() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        let project = tmp.path().join("proj");
        write(
            &project,
            "policy.yaml",
            "version: 1\nexecution: {concurrency: {maxOperations: 2}}\n",
        );
        let invocation = invocation_layer(json!({
            "git-log": {"executable": "git", "argv": ["log", "*"]},
        }));
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().join("user").to_str().unwrap(),
            project_dir: project.to_str().unwrap(),
            invocation: Some(&invocation),
        })
        .unwrap();
        // project partial は `command` section を schema reject するため
        // invocation grant を override 不能 — trusted boundary の不変条件。
        assert!(p.command.grants.contains_key("git-log"));
        assert_eq!(p.execution.concurrency.unwrap().max_operations, 2);
    }

    #[test]
    fn invocation_rejects_non_grants_keys() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        // top-level の grants 以外 key。
        let top = json!({"version": 1, "tools": {"allow": ["read"]}})
            .as_object()
            .unwrap()
            .clone();
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: Some(&top),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRequest);
        assert_eq!(e.details().unwrap()["key"], json!("tools"));
        assert_eq!(layer_detail(&e), Some(&json!("invocation")));
        // `command` 内の grants 以外 key も同じ制約。
        let nested = json!({"version": 1, "command": {"timeout": "2m"}})
            .as_object()
            .unwrap()
            .clone();
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: Some(&nested),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRequest);
        assert_eq!(e.details().unwrap()["key"], json!("command.timeout"));
        assert_eq!(layer_detail(&e), Some(&json!("invocation")));
    }

    #[test]
    fn invocation_grant_collision_is_invalid_request() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        let user = tmp.path().join("user");
        write(
            &user,
            "policy.yaml",
            "version: 1\ncommand:\n  grants:\n    git-log:\n      executable: git\n",
        );
        let invocation = invocation_layer(json!({
            "git-log": {"executable": "git", "argv": ["status"]},
        }));
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: Some(&invocation),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRequest);
        assert_eq!(e.details().unwrap()["grant"], json!("git-log"));
        assert_eq!(layer_detail(&e), Some(&json!("invocation")));
    }

    #[test]
    fn invocation_invalid_grant_spec_keeps_inner_code_with_layer() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        // wildcard 構造違反を含む grant spec — shared validation 由来の
        // 失敗は inner code `InvalidRuntimePolicy` を保持する
        // (PATH 非依存で deterministic)。
        let invocation = invocation_layer(json!({
            "run-python": {"executable": "python", "argv": ["a*b"]},
        }));
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: Some(&invocation),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("invalid command policy"));
        assert_eq!(layer_detail(&e), Some(&json!("invocation")));
    }

    /// `security.execDenyPatterns` の merge 意味論 — user 層の配列は
    /// builtin の配列を**丸ごと置換**(`tools.allow` と同じ wholesale
    /// replace — union ではない)。未指定の user 層は builtin を継承。
    /// invocation 合成 grant も merged effective の deny 評価対象。
    #[test]
    fn security_exec_deny_patterns_merge_and_invocation_scope() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "builtin.yaml",
            &format!("{BUILTIN}\nsecurity:\n  execDenyPatterns: [from-builtin]\n"),
        );
        // builtin `[from-builtin]` + user `[from-user]` → effective
        // `[from-user]`(wholesale replace)。
        let user = tmp.path().join("user");
        write(
            &user,
            "policy.yaml",
            "version: 1\nsecurity: {execDenyPatterns: [from-user]}\n",
        );
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: None,
        })
        .unwrap();
        assert_eq!(p.security.exec_deny_patterns, vec!["from-user"]);
        // user 層が `security` を持たない場合は builtin を継承。
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().join("user2").to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: None,
        })
        .unwrap();
        assert_eq!(p.security.exec_deny_patterns, vec!["from-builtin"]);
    }

    /// user policy の deny + `--grant` 由来の matching executable で
    /// `load_policy` が `InvalidRuntimePolicy`(merged re-resolve の
    /// 適用点 1 — invocation 合成 grant を exempt しない)。
    /// merged-map 評価のため error は `details.layer` を**持たない**
    /// (deny 元 builtin/user と grant 元 invocation が別 layer —
    /// attribution gap は accepted として pin)。
    #[test]
    fn invocation_grant_hits_user_exec_deny_without_layer_attribution() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        let user = tmp.path().join("user");
        write(
            &user,
            "policy.yaml",
            "version: 1\nsecurity: {execDenyPatterns: [python.*]}\n",
        );
        let invocation = invocation_layer(json!({
            "run-python": {"executable": "python3"},
        }));
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: Some(&invocation),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("denied by security.execDenyPatterns"));
        assert_eq!(e.details().unwrap()["grant"], json!("run-python"));
        assert_eq!(layer_detail(&e), None); // accepted attribution gap
                                            // 同一 deny でも非 match の invocation grant は受理される。
        let invocation = invocation_layer(json!({
            "git-log": {"executable": "git"},
        }));
        let p = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
            invocation: Some(&invocation),
        })
        .unwrap();
        assert!(p.command.grants.contains_key("git-log"));
        assert_eq!(p.security.exec_deny_patterns, vec!["python.*"]);
    }

    /// invocation layer は `security` key を書けない — top-level
    /// keys ⊆ `{version, command}` guard が `InvalidRequest` +
    /// `details.layer="invocation"` で拒否する(既存構造 guard の
    /// pin — `command.security` も `command` keys ⊆ `{grants}` guard
    /// が同様に拒否)。
    #[test]
    fn invocation_layer_cannot_author_security() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "builtin.yaml", BUILTIN);
        // top-level `security` key。
        let top = json!({"version": 1, "security": {"execDenyPatterns": ["x"]}})
            .as_object()
            .unwrap()
            .clone();
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: Some(&top),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRequest);
        assert_eq!(e.details().unwrap()["key"], json!("security"));
        assert_eq!(layer_detail(&e), Some(&json!("invocation")));
        // `command.security` も grants 以外 key として同じ制約。
        let nested = json!({"version": 1, "command": {"security": {"execDenyPatterns": ["x"]}}})
            .as_object()
            .unwrap()
            .clone();
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: Some(&nested),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRequest);
        assert_eq!(e.details().unwrap()["key"], json!("command.security"));
        assert_eq!(layer_detail(&e), Some(&json!("invocation")));
    }

    /// invalid regex を含む `security` 節は layer 単位の schema
    /// validate で `InvalidRuntimePolicy` に fail-closed(builtin・
    /// user の両層 — warn-and-skip しない)。
    #[test]
    fn invalid_exec_deny_regex_fails_load_at_layer() {
        let tmp = tempfile::tempdir().unwrap();
        // builtin 層。
        write(
            tmp.path(),
            "builtin.yaml",
            &format!("{BUILTIN}\nsecurity:\n  execDenyPatterns: [\"(\"]\n"),
        );
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("invalid execDenyPatterns regex"));
        // user 層。
        write(tmp.path(), "builtin2.yaml", BUILTIN);
        let user = tmp.path().join("user");
        write(
            &user,
            "policy.yaml",
            "version: 1\nsecurity: {execDenyPatterns: [\"(\"]}\n",
        );
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("builtin2.yaml").to_str().unwrap(),
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
            invocation: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("invalid execDenyPatterns regex"));
    }
}
