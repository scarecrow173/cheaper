//! Port of `packages/core/src/policy/load.ts` — the §23 load order:
//! built-in (complete) → user (partial) merge → typed policy
//! completion → project (partial) `restrict-concurrency` merge →
//! complete validation → normalize.

use serde_json::{json, Map, Value};

use crate::config::layer::{read_yaml_layer, LayerKind};
use crate::errors::{CheaperError, ErrorCode};
use crate::policy::merge::{merge_policy_layers, MergeMode};
use crate::policy::process::resolve_process_policy;
use crate::policy::schema::{
    normalize_policy, resolve_concurrency_policy, validate_policy_schema, PolicySchemaMode,
    RuntimePolicy,
};
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

fn read_policy_file(
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
/// `projectDir`).
pub struct LoadPolicyOptions<'a> {
    pub builtin_policy_path: &'a str,
    pub user_config_dir: &'a str,
    pub project_dir: &'a str,
}

/// `loadPolicy` — 引継ぎ §23 load order. An invalid policy is never
/// silently ignored.
pub fn load_policy(opts: &LoadPolicyOptions<'_>) -> Result<RuntimePolicy, CheaperError> {
    let builtin_raw =
        read_policy_file(opts.builtin_policy_path, true)?.expect("required policy file");
    let mut effective =
        validate_policy_schema(&Value::Object(builtin_raw), PolicySchemaMode::Complete)?;

    let user_path = nodepath_join(opts.user_config_dir, "policy.yaml");
    if let Some(user) = read_policy_file(&user_path, false)? {
        tracing::debug!(policy.layer = "user", "policy layer merged");
        effective = merge_policy_layers(
            &effective,
            &validate_policy_schema(&Value::Object(user), PolicySchemaMode::Partial)?,
            MergeMode::Replace,
        )
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
    if let Some(project) = read_policy_file(&project_path, false)? {
        tracing::debug!(policy.layer = "project", "policy layer merged");
        effective = merge_policy_layers(
            &effective,
            &validate_policy_schema(&Value::Object(project), PolicySchemaMode::Partial)?,
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
        })
        .unwrap();
        assert_eq!(p.execution.timeout_ms, 600_000);
        assert_eq!(p.execution.concurrency.unwrap().max_operations, 4);
        assert_eq!(p.process.timeout, "2m"); // 10m clamped to default 2m
    }

    #[test]
    fn missing_builtin_is_invalid_policy() {
        let tmp = tempfile::tempdir().unwrap();
        let e = load_policy(&LoadPolicyOptions {
            builtin_policy_path: tmp.path().join("absent.yaml").to_str().unwrap(),
            user_config_dir: tmp.path().to_str().unwrap(),
            project_dir: tmp.path().to_str().unwrap(),
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
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }
}
