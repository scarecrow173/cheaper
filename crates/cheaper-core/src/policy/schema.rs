//! Port of `packages/core/src/policy/schema.ts`.

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::duration::{parse_duration, DURATION_PATTERN};
use crate::errors::{CheaperError, ErrorCode};
use crate::js;
use crate::model::tier::{Tier, TIERS};
use crate::policy::command::{
    command_policy_properties, resolve_command_policy, ResolvedCommandPolicy,
};
use crate::policy::process::{
    process_policy_properties, resolve_process_policy, ResolvedProcessPolicy,
};
use crate::policy::workflow::{
    resolve_workflow_policy, workflow_policy_properties, WorkflowPolicy,
};
use crate::tools::vocabulary::ToolName;
use crate::worker::schema::schema_error_entries;

/// `ConcurrencyPolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConcurrencyPolicy {
    #[serde(rename = "maxOperations")]
    pub max_operations: u64,
    pub mutations: MutationsMode,
}

/// `"serialized" | "per-resource"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MutationsMode {
    #[serde(rename = "serialized")]
    Serialized,
    #[serde(rename = "per-resource")]
    PerResource,
}

impl MutationsMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Serialized => "serialized",
            Self::PerResource => "per-resource",
        }
    }
}

const DEFAULT_MAX_OPERATIONS: u64 = 8;
const DEFAULT_MUTATIONS: MutationsMode = MutationsMode::Serialized;

/// `resolveConcurrencyPolicy`: one place owning defaults and validation
/// for legacy policy and runtime options.
///
/// Input is the raw `execution.concurrency` value (`Option<&Value>`);
/// `undefined`/non-object collapse like `input?.maxOperations`.
pub fn resolve_concurrency_policy(
    input: Option<&Value>,
) -> Result<ConcurrencyPolicy, CheaperError> {
    let obj = input.and_then(Value::as_object);
    // JS `input?.maxOperations ?? 8` / `input?.mutations ?? "serialized"`
    // — explicit null coalesces to the default (nullish coalescing, not
    // "invalid field").
    let max_operations = match obj
        .and_then(|o| o.get("maxOperations"))
        .filter(|v| !v.is_null())
    {
        None => DEFAULT_MAX_OPERATIONS,
        Some(v) => js::safe_u64(v).unwrap_or(0),
    };
    let mutations_raw = obj
        .and_then(|o| o.get("mutations"))
        .filter(|v| !v.is_null());
    let mutations = match mutations_raw.and_then(Value::as_str) {
        None => DEFAULT_MUTATIONS,
        Some("serialized") => MutationsMode::Serialized,
        Some("per-resource") => MutationsMode::PerResource,
        Some(_) => MutationsMode::Serialized, // marker; validated below
    };
    let mutations_valid = match mutations_raw {
        None => true,
        Some(v) => matches!(v.as_str(), Some("serialized") | Some("per-resource")),
    };
    if max_operations < 1 || !mutations_valid {
        return Err(CheaperError::new(
            ErrorCode::InvalidRuntimePolicy,
            "invalid execution.concurrency",
        ));
    }
    Ok(ConcurrencyPolicy {
        max_operations,
        mutations,
    })
}

/// `RuntimePolicy` (normalized output shape).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimePolicy {
    pub workflow: WorkflowPolicy,
    pub process: ResolvedProcessPolicy,
    /// `command` grants(trusted layer のみ authoring 可能。project
    /// partial schema は section 全体を除外する)。
    pub command: ResolvedCommandPolicy,
    pub execution: ExecutionPolicy,
    pub tools: ToolsPolicy,
    pub filesystem: FilesystemPolicy,
    pub model: ModelPolicy,
    pub output: OutputPolicy,
    pub retry: RetryPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutionPolicy {
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: u64,
    /// `null` / 欠落は policy レベルの turn 上限なし(内部 runaway
    /// ガード `DEFAULT_MAX_TURNS` のみが残る)。明示整数は従来どおり
    /// `MAX_TURNS_EXCEEDED` の policy abort 境界。
    #[serde(rename = "maxTurns")]
    pub max_turns: Option<u64>,
    #[serde(rename = "maxToolCalls")]
    pub max_tool_calls: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<ConcurrencyPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolsPolicy {
    pub allow: Vec<ToolName>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FilesystemPolicy {
    pub symlinks: crate::workspace::paths::SymlinkPolicy,
    #[serde(rename = "denyPaths")]
    pub deny_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelPolicy {
    #[serde(rename = "maxTier")]
    pub max_tier: Tier,
    #[serde(rename = "tierFallback")]
    pub tier_fallback: TierFallback,
}

/// `"none" | "higher"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TierFallback {
    None,
    Higher,
}

impl TierFallback {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Higher => "higher",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutputPolicy {
    #[serde(rename = "maxTokens")]
    pub max_tokens: u64,
    #[serde(rename = "repairAttempts")]
    pub repair_attempts: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RetryPolicy {
    #[serde(rename = "transientAttempts")]
    pub transient_attempts: u64,
}

fn object_schema(required: &[&str], properties: Value) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": required,
        "properties": properties,
    })
}

fn concurrency_properties() -> Value {
    json!({
        "maxOperations": {"type": "integer", "minimum": 1, "maximum": js::MAX_SAFE_INTEGER_U64},
        "mutations": {"type": "string", "enum": ["serialized", "per-resource"]},
    })
}

fn execution_properties(concurrency_required: bool) -> Value {
    json!({
        "timeout": {"type": "string", "pattern": DURATION_PATTERN},
        "maxTurns": {"type": ["integer", "null"], "minimum": 1},
        "maxToolCalls": {"type": "integer", "minimum": 1},
        "concurrency": object_schema(
            if concurrency_required { &["maxOperations", "mutations"][..] } else { &[][..] },
            concurrency_properties(),
        ),
    })
}

fn tools_properties() -> Value {
    json!({"allow": {"type": "array", "items": {"type": "string"}}})
}

fn filesystem_properties() -> Value {
    json!({
        "symlinks": {"type": "string", "enum": ["deny", "within-workspace"]},
        "denyPaths": {"type": "array", "items": {"type": "string"}},
    })
}

fn model_properties() -> Value {
    let tiers: Vec<&str> = TIERS.iter().map(|t| t.as_str()).collect();
    json!({
        "maxTier": {"type": "string", "enum": tiers},
        "tierFallback": {"type": "string", "enum": ["none", "higher"]},
    })
}

fn output_properties() -> Value {
    json!({
        "maxTokens": {"type": "integer", "minimum": 1},
        "repairAttempts": {"type": "integer", "minimum": 0},
    })
}

fn retry_properties() -> Value {
    json!({"transientAttempts": {"type": "integer", "minimum": 0}})
}

const VERSION_PROPERTY: fn() -> Value = || json!({"type": "integer", "const": 1});

/// `POLICY_COMPLETE_SCHEMA` verbatim.
pub fn policy_complete_schema() -> Value {
    object_schema(
        &[
            "version",
            "execution",
            "tools",
            "filesystem",
            "model",
            "output",
            "retry",
        ],
        json!({
            "version": VERSION_PROPERTY(),
            "workflow": object_schema(&[], workflow_policy_properties()),
            "process": object_schema(&["maxConcurrent", "tasks"], process_policy_properties()),
            "command": object_schema(&["maxConcurrent", "grants"], command_policy_properties()),
            "execution": object_schema(&["timeout", "maxTurns", "maxToolCalls"], execution_properties(true)),
            "tools": object_schema(&["allow"], tools_properties()),
            "filesystem": object_schema(&["symlinks"], filesystem_properties()),
            "model": object_schema(&["maxTier", "tierFallback"], model_properties()),
            "output": object_schema(&["maxTokens", "repairAttempts"], output_properties()),
            "retry": object_schema(&["transientAttempts"], retry_properties()),
        }),
    )
}

/// `POLICY_PARTIAL_SCHEMA` verbatim(trusted user layer 用 — `command`
/// section の grant authoring を許す)。
pub fn policy_partial_schema() -> Value {
    object_schema(
        &["version"],
        json!({
            "version": VERSION_PROPERTY(),
            "workflow": object_schema(&[], workflow_policy_properties()),
            "process": object_schema(&[], process_policy_properties()),
            "command": object_schema(&[], command_policy_properties()),
            "execution": object_schema(&[], execution_properties(false)),
            "tools": object_schema(&[], tools_properties()),
            "filesystem": object_schema(&[], filesystem_properties()),
            "model": object_schema(&[], model_properties()),
            "output": object_schema(&[], output_properties()),
            "retry": object_schema(&[], retry_properties()),
        }),
    )
}

/// project layer 用 partial schema(Option A の信頼境界 — `command`
/// section 全体を property から除外し、`additionalProperties: false`
/// が `command` key を含む project policy を load 時に拒否する。
/// grant authoring key を個別列挙しないのは、将来 grant field が増えても
/// section 丸ごと除外なら自動的に project から閉じるため(fail-closed)。
pub fn policy_project_partial_schema() -> Value {
    object_schema(
        &["version"],
        json!({
            "version": VERSION_PROPERTY(),
            "workflow": object_schema(&[], workflow_policy_properties()),
            "process": object_schema(&[], process_policy_properties()),
            "execution": object_schema(&[], execution_properties(false)),
            "tools": object_schema(&[], tools_properties()),
            "filesystem": object_schema(&[], filesystem_properties()),
            "model": object_schema(&[], model_properties()),
            "output": object_schema(&[], output_properties()),
            "retry": object_schema(&[], retry_properties()),
        }),
    )
}

/// `PolicySchemaMode`。`Partial` は trusted user layer 用に温存し、
/// `ProjectPartial` は `command` 除外の project layer 用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicySchemaMode {
    Complete,
    Partial,
    ProjectPartial,
}

impl PolicySchemaMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::ProjectPartial => "project-partial",
        }
    }
}

fn policy_validators() -> &'static (
    jsonschema::Validator,
    jsonschema::Validator,
    jsonschema::Validator,
) {
    static V: std::sync::OnceLock<(
        jsonschema::Validator,
        jsonschema::Validator,
        jsonschema::Validator,
    )> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        (
            jsonschema::draft202012::new(&policy_complete_schema())
                .expect("complete policy schema compiles"),
            jsonschema::draft202012::new(&policy_partial_schema())
                .expect("partial policy schema compiles"),
            jsonschema::draft202012::new(&policy_project_partial_schema())
                .expect("project partial policy schema compiles"),
        )
    })
}

/// `validatePolicySchema`: schema validation, then the legacy
/// `process`/`command` section check via `resolveProcessPolicy`/
/// `resolveCommandPolicy` (their own boundary validation is preserved).
pub fn validate_policy_schema(
    raw: &Value,
    mode: PolicySchemaMode,
) -> Result<Map<String, Value>, CheaperError> {
    let (complete, partial, project_partial) = policy_validators();
    let validator = match mode {
        PolicySchemaMode::Complete => complete,
        PolicySchemaMode::Partial => partial,
        PolicySchemaMode::ProjectPartial => project_partial,
    };
    if !validator.is_valid(raw) {
        let mut details = Map::new();
        details.insert(
            "errors".to_string(),
            serde_json::to_value(schema_error_entries(validator, raw)).unwrap_or(Value::Null),
        );
        return Err(CheaperError::new(
            ErrorCode::InvalidRuntimePolicy,
            format!("invalid {} runtime policy", mode.as_str()),
        )
        .with_details(details));
    }
    let policy = raw.as_object().expect("schema enforces object").clone();
    // Preserve partial omissions while still rejecting duration overflow
    // etc. before merge.
    if let Some(process) = policy.get("process") {
        resolve_process_policy(Some(process))?;
    }
    if let Some(command) = policy.get("command") {
        resolve_command_policy(Some(command))?;
    }
    Ok(policy)
}

/// `normalizePolicyTools`: canonical/alias/dup checks over `tools.allow`.
fn normalize_policy_tools(raw: Option<&Value>) -> Result<Vec<ToolName>, CheaperError> {
    let list: &[Value] = match raw {
        Some(Value::Array(a)) => a,
        _ => &[],
    };
    let mut names = Vec::with_capacity(list.len());
    for v in list {
        let name = v.as_str().and_then(|s| {
            ToolName::from_canonical(s).or_else(|| {
                crate::tools::vocabulary::TOOL_ALIASES
                    .iter()
                    .find(|(a, _)| *a == s)
                    .map(|(_, t)| *t)
            })
        });
        match name {
            Some(t) => names.push(t),
            None => {
                let mut details = Map::new();
                details.insert("tool".to_string(), v.clone());
                return Err(CheaperError::new(
                    ErrorCode::InvalidRuntimePolicy,
                    format!("unknown tool in policy: {}", js::js_string(v)),
                )
                .with_details(details));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    if !names.iter().all(|n| seen.insert(*n)) {
        let mut details = Map::new();
        details.insert("reason".to_string(), json!("tools-duplicate"));
        return Err(CheaperError::new(
            ErrorCode::InvalidRuntimePolicy,
            "policy tools.allow contains duplicates",
        )
        .with_details(details));
    }
    Ok(names)
}

fn get_u64(obj: &Map<String, Value>, key: &str) -> u64 {
    obj.get(key).and_then(js::safe_u64).unwrap_or(0)
}

/// nullable policy 値用 — `null`・欠落は `None`。非 null の不正値は
/// `Some(0)` として残る(complete schema 検証で本来弾かれる残留経路)。
fn get_optional_u64(obj: &Map<String, Value>, key: &str) -> Option<u64> {
    match obj.get(key) {
        None | Some(Value::Null) => None,
        value => Some(value.and_then(js::safe_u64).unwrap_or(0)),
    }
}

/// `normalizePolicy` (expects a complete-schema-validated policy).
///
/// Accepted divergence: several `raw.<section>` values are coerced
/// through `as_object`/`as_str` passthrough rather than re-validated —
/// e.g. `filesystem.symlinks` non-`"within-workspace"` collapses to
/// `Deny`, and unknown `model.maxTier` falls back to `Tier::High`. TS
/// relies on the input type being `unknown` here too, and the
/// schema-gated pipeline (`validate_policy_schema` → `normalize_policy`)
/// makes the unvalidated-input case unreachable, so passthrough is the
/// intended contract (same precedent as `rustDropUnknownArtifactKeys`
/// for the workflow artifact spread).
pub fn normalize_policy(raw: &Map<String, Value>) -> Result<RuntimePolicy, CheaperError> {
    let empty = Map::new();
    let execution = raw
        .get("execution")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let timeout_raw = execution.get("timeout");
    let timeout_ms = match timeout_raw.and_then(Value::as_str) {
        Some(s) => match parse_duration(s) {
            Ok(ms) => ms,
            Err(e) => {
                let mut details = Map::new();
                details.insert(
                    "timeout".to_string(),
                    timeout_raw.cloned().unwrap_or(Value::Null),
                );
                return Err(CheaperError::new(
                    ErrorCode::InvalidRuntimePolicy,
                    format!(
                        "invalid execution.timeout: {}",
                        js::js_string(timeout_raw.unwrap_or(&Value::Null))
                    ),
                )
                .with_details(details)
                .with_cause(e));
            }
        },
        // `String(execution.timeout)` for a missing/non-string value.
        None => {
            let mut details = Map::new();
            details.insert(
                "timeout".to_string(),
                timeout_raw.cloned().unwrap_or(Value::Null),
            );
            return Err(CheaperError::new(
                ErrorCode::InvalidRuntimePolicy,
                format!(
                    "invalid execution.timeout: {}",
                    timeout_raw
                        .map(js::js_string)
                        .unwrap_or_else(|| js::js_undefined().to_string())
                ),
            )
            .with_details(details));
        }
    };
    let tools = raw
        .get("tools")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let filesystem = raw
        .get("filesystem")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let model = raw
        .get("model")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let output = raw
        .get("output")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let retry = raw
        .get("retry")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    Ok(RuntimePolicy {
        workflow: resolve_workflow_policy(raw.get("workflow"))?,
        process: resolve_process_policy(raw.get("process"))?,
        command: resolve_command_policy(raw.get("command"))?,
        execution: ExecutionPolicy {
            timeout_ms,
            max_turns: get_optional_u64(execution, "maxTurns"),
            max_tool_calls: get_u64(execution, "maxToolCalls"),
            concurrency: Some(resolve_concurrency_policy(execution.get("concurrency"))?),
        },
        tools: ToolsPolicy {
            allow: normalize_policy_tools(tools.get("allow"))?,
        },
        filesystem: FilesystemPolicy {
            symlinks: match filesystem.get("symlinks").and_then(Value::as_str) {
                Some("within-workspace") => crate::workspace::paths::SymlinkPolicy::WithinWorkspace,
                _ => crate::workspace::paths::SymlinkPolicy::Deny,
            },
            deny_paths: filesystem
                .get("denyPaths")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        },
        model: ModelPolicy {
            max_tier: crate::model::tier::parse_tier(model.get("maxTier").unwrap_or(&Value::Null))
                .unwrap_or(Tier::High),
            tier_fallback: match model.get("tierFallback").and_then(Value::as_str) {
                Some("higher") => TierFallback::Higher,
                _ => TierFallback::None,
            },
        },
        output: OutputPolicy {
            max_tokens: get_u64(output, "maxTokens"),
            repair_attempts: get_u64(output, "repairAttempts"),
        },
        retry: RetryPolicy {
            transient_attempts: get_u64(retry, "transientAttempts"),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_policy() -> Value {
        json!({
            "version": 1,
            "execution": {"timeout": "2m", "maxTurns": 10, "maxToolCalls": 20},
            "tools": {"allow": ["read", "grep"]},
            "filesystem": {"symlinks": "deny"},
            "model": {"maxTier": "high", "tierFallback": "higher"},
            "output": {"maxTokens": 4096, "repairAttempts": 1},
            "retry": {"transientAttempts": 2},
        })
    }

    #[test]
    fn complete_schema_accepts_valid_policy() {
        let p = validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap();
        assert_eq!(p["version"], json!(1));
    }

    #[test]
    fn complete_schema_rejects_missing_sections() {
        let mut bad = valid_policy();
        bad.as_object_mut().unwrap().remove("tools");
        let e = validate_policy_schema(&bad, PolicySchemaMode::Complete).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        let errors = e.details().unwrap()["errors"].as_array().unwrap();
        assert_eq!(errors[0]["instancePath"], json!(""));
        // partial schema accepts the same doc (only version required).
        let mut partial = valid_policy();
        partial.as_object_mut().unwrap().remove("tools");
        partial.as_object_mut().unwrap().remove("model");
        assert!(validate_policy_schema(&partial, PolicySchemaMode::Partial).is_ok());
    }

    #[test]
    fn schema_rejects_unknown_properties() {
        let mut bad = valid_policy();
        bad.as_object_mut()
            .unwrap()
            .insert("bogus".to_string(), json!(1));
        assert!(validate_policy_schema(&bad, PolicySchemaMode::Complete).is_err());
        // nested unknown key in a section
        let mut bad2 = valid_policy();
        bad2["execution"]
            .as_object_mut()
            .unwrap()
            .insert("x".to_string(), json!(1));
        assert!(validate_policy_schema(&bad2, PolicySchemaMode::Complete).is_err());
    }

    #[test]
    fn process_section_runs_its_own_validation() {
        let mut p = valid_policy();
        p.as_object_mut().unwrap().insert(
            "process".to_string(),
            json!({"maxConcurrent": 2, "tasks": {}}),
        );
        assert!(validate_policy_schema(&p, PolicySchemaMode::Complete).is_ok());
        let mut bad = valid_policy();
        bad.as_object_mut().unwrap().insert(
            "process".to_string(),
            json!({"maxConcurrent": 1, "tasks": {"t": {
                "command": ["sh"], "inputs": [], "outputs": []}}}),
        );
        // schema passes but resolveProcessPolicy rejects shell executable.
        let e = validate_policy_schema(&bad, PolicySchemaMode::Complete).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    #[test]
    fn concurrency_defaults_and_validation() {
        let c = resolve_concurrency_policy(None).unwrap();
        assert_eq!(c.max_operations, 8);
        assert_eq!(c.mutations, MutationsMode::Serialized);
        let c = resolve_concurrency_policy(Some(&json!({
            "maxOperations": 3, "mutations": "per-resource"
        })))
        .unwrap();
        assert_eq!(c.max_operations, 3);
        assert_eq!(c.mutations, MutationsMode::PerResource);
        for raw in [
            json!({"maxOperations": 0}),
            json!({"maxOperations": 1.5}),
            json!({"mutations": "bogus"}),
            json!({"maxOperations": "x"}),
        ] {
            assert!(resolve_concurrency_policy(Some(&raw)).is_err(), "{raw}");
        }
        // `??` semantics: explicit null coalesces to the defaults.
        let c = resolve_concurrency_policy(Some(&json!({"maxOperations": null}))).unwrap();
        assert_eq!(c.max_operations, 8);
        let c = resolve_concurrency_policy(Some(&json!({"mutations": null}))).unwrap();
        assert_eq!(c.mutations, MutationsMode::Serialized);
        // A null object is the same as absent.
        let c = resolve_concurrency_policy(Some(&json!({
            "maxOperations": null, "mutations": null
        })))
        .unwrap();
        assert_eq!(
            (c.max_operations, c.mutations),
            (8, MutationsMode::Serialized)
        );
    }

    #[test]
    fn normalize_policy_fills_defaults() {
        let raw = validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap();
        let policy = normalize_policy(&raw).unwrap();
        assert_eq!(policy.execution.timeout_ms, 120_000);
        assert_eq!(policy.execution.max_turns, Some(10));
        assert_eq!(policy.execution.concurrency.unwrap().max_operations, 8);
        assert_eq!(policy.tools.allow, vec![ToolName::Read, ToolName::Grep]);
        assert_eq!(policy.model.max_tier, Tier::High);
        assert_eq!(policy.model.tier_fallback, TierFallback::Higher);
        assert_eq!(policy.output.max_tokens, 4096);
        assert_eq!(policy.retry.transient_attempts, 2);
    }

    #[test]
    fn max_turns_null_means_unbounded() {
        // `maxTurns: null` — policy レベルの turn 上限なし。
        let mut p = valid_policy();
        p["execution"]["maxTurns"] = Value::Null;
        let raw = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap();
        assert_eq!(normalize_policy(&raw).unwrap().execution.max_turns, None);
        // complete schema では maxTurns は required — 欠落は reject。
        // partial(project/user layer)では欠落可で `None` になる。
        let mut p = valid_policy();
        p["execution"].as_object_mut().unwrap().remove("maxTurns");
        assert!(validate_policy_schema(&p, PolicySchemaMode::Complete).is_err());
        let raw = validate_policy_schema(&p, PolicySchemaMode::Partial).unwrap();
        assert_eq!(normalize_policy(&raw).unwrap().execution.max_turns, None);
        // 明示整数は bounded のまま。
        assert_eq!(
            normalize_policy(
                &validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap()
            )
            .unwrap()
            .execution
            .max_turns,
            Some(10)
        );
        // 非 null の不正値は schema が拒否する。
        for bad in [json!(0), json!(-1), json!("8"), json!(1.5)] {
            let mut p = valid_policy();
            p["execution"]["maxTurns"] = bad.clone();
            assert!(
                validate_policy_schema(&p, PolicySchemaMode::Complete).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn normalize_policy_rejects_unknown_and_duplicate_tools() {
        let mut bad = valid_policy();
        bad["tools"].as_object_mut().unwrap()["allow"] = json!(["read", "bogus"]);
        let raw = validate_policy_schema(&bad, PolicySchemaMode::Complete).unwrap();
        let e = normalize_policy(&raw).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("unknown tool in policy"));

        let mut dup = valid_policy();
        dup["tools"].as_object_mut().unwrap()["allow"] = json!(["read", "Read"]);
        let raw = validate_policy_schema(&dup, PolicySchemaMode::Complete).unwrap();
        let e = normalize_policy(&raw).unwrap_err();
        assert_eq!(e.details().unwrap()["reason"], json!("tools-duplicate"));

        // Alias is normalized to the canonical name.
        let mut alias = valid_policy();
        alias["tools"].as_object_mut().unwrap()["allow"] = json!(["Read"]);
        let raw = validate_policy_schema(&alias, PolicySchemaMode::Complete).unwrap();
        let policy = normalize_policy(&raw).unwrap();
        assert_eq!(policy.tools.allow, vec![ToolName::Read]);
    }
}
