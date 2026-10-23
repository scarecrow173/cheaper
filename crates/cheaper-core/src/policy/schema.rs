//! Port of `packages/core/src/policy/schema.ts`.

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::duration::{parse_duration, DURATION_PATTERN};
use crate::errors::{CheaperError, ErrorCode};
use crate::js;
use crate::model::tier::{Tier, TIERS};
use crate::policy::command::command_policy_properties;
use crate::policy::exec::{build_exec_input, resolve_exec_policy, ResolvedExecPolicy};
use crate::policy::process::process_policy_properties;
use crate::policy::security::{
    resolve_exec_deny_list, resolve_security_policy, security_policy_properties, ExecDenyList,
    SecurityPolicy,
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

/// `route` section の normalized 表現(trusted layer のみ authoring
/// 可能 — `command` と同型に project partial schema は section 全体を
/// 除外する)。yaml 表現と 1:1 の構造を保つ。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RoutePolicy {
    pub rewrite: RewritePolicy,
}

/// `route.rewrite` — rewrite emit の policy gate。
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RewritePolicy {
    /// `permissionDecision:"allow"` 併記の rewrite emit(親 agent の
    /// approval prompt を素通しする応答)を許可する hook target 名
    /// (`codex`/`cursor`/`copilot`)。未指定・`null`・空配列は `[]`
    /// 等価 = bypassing emit 無効(conservative default)。
    #[serde(rename = "approvalBypass")]
    pub approval_bypass: Vec<String>,
}

impl RewritePolicy {
    /// flag 判定の単一 owner — render 層は policy path を直接たどらない。
    /// flag は emit 有効化ではなく許可であり、emit contract が未検証の
    /// target(cursor/copilot)は列挙されても emit されない。
    pub fn approval_bypass_enabled(&self, target: &str) -> bool {
        self.approval_bypass.iter().any(|t| t == target)
    }
}

/// `RuntimePolicy` (normalized output shape).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimePolicy {
    pub workflow: WorkflowPolicy,
    /// 統一 exec policy(issue #165) — resolved `command`/`process`
    /// section の単一集約。serialized form は
    /// `{"grants":{..},"command":{defaults},"process":{defaults}}`。
    /// `command.grants`(trusted layer のみ authoring 可能 — project
    /// partial schema は section 全体を除外)と `process.tasks` の
    /// grant が単一 namespace に resolve される。
    pub exec: ResolvedExecPolicy,
    /// `route` section(`command` と同型の trusted-layer 限定 —
    /// project partial schema は section 全体を除外する)。
    pub route: RoutePolicy,
    /// `security` section(`command`/`route` と同型の trusted-layer
    /// 限定 — project partial schema は section 全体を除外する)。
    /// serde-native の source string 列のみ保持し、compiled deny
    /// predicate(`ExecDenyList`)は載せない。
    pub security: SecurityPolicy,
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

/// `route` section の properties。`approvalBypass` は
/// `execution.maxTurns`(`{"type": ["integer","null"]}`)の nullable
/// 先例に倣い `["array","null"]` — 明示 `null` ≡ 未指定 ≡ `[]` を
/// schema 層で保証する。値 domain・uniqueItems・非 object の
/// `rewrite`/`route` は schema が fail-loud に拒否し、normalize 側で
/// 二重に解釈しない。
fn route_properties() -> Value {
    json!({
        "rewrite": object_schema(&[], json!({
            "approvalBypass": {
                "type": ["array", "null"],
                "items": {"type": "string", "enum": ["codex", "cursor", "copilot"]},
                "uniqueItems": true,
            },
        })),
    })
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
            "route": object_schema(&[], route_properties()),
            "security": object_schema(&[], security_policy_properties()),
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
            "route": object_schema(&[], route_properties()),
            "security": object_schema(&[], security_policy_properties()),
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
    // etc. before merge — unified exec resolution(issue #165)が
    // section resolver を内包するため、merged `process`/`command` の
    // semantic 検査は `build_exec_input` + `resolve_exec_policy` で
    // 1 度に行う。deny 適用点 1・collision 検査もここで発火する。
    // `security` の compile は `command`/`process` の有無に関わらず
    // 無条件に評価する — security-only doc の invalid regex も
    // layer 単位の検査で fail-closed に拒否する。
    let exec_deny = resolve_exec_deny_list(policy.get("security"))?;
    let exec_doc = build_exec_input(
        policy.get("process"),
        policy.get("command"),
        &std::collections::BTreeSet::new(),
    )?;
    resolve_exec_policy(Some(&Value::Object(exec_doc)), &exec_deny)?;
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
pub fn normalize_policy(
    raw: &Map<String, Value>,
    project_task_names: &std::collections::BTreeSet<String>,
) -> Result<RuntimePolicy, CheaperError> {
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
    // `security` の passthrough 解決(`route` と同型) + 1 回の
    // compile — `resolve_command_policy` へ compiled deny を共有する
    // (declared executable の deny 照合は resolve 段階で fail-closed)。
    let security = resolve_security_policy(raw.get("security"))?;
    let exec_deny = ExecDenyList::compile(&security.exec_deny_patterns)?;
    // 統一 exec resolution(issue #165)— merged `process`/`command`
    // section を段 2 synthesis + 段 3 resolver で `RuntimePolicy.exec`
    // へ畳む。`project_task_names` は project layer 宣言 task 由来の
    // provenance(`scope:"project"`)。
    let exec_doc = build_exec_input(raw.get("process"), raw.get("command"), project_task_names)?;
    Ok(RuntimePolicy {
        workflow: resolve_workflow_policy(raw.get("workflow"))?,
        exec: resolve_exec_policy(Some(&Value::Object(exec_doc)), &exec_deny)?,
        // `route` の passthrough 解決 — `null`/欠落 → default `[]`。
        // 値 domain は schema が保証済みのため item は string のみ収集。
        route: {
            let rewrite = raw
                .get("route")
                .and_then(Value::as_object)
                .and_then(|r| r.get("rewrite"))
                .and_then(Value::as_object);
            RoutePolicy {
                rewrite: RewritePolicy {
                    approval_bypass: rewrite
                        .and_then(|r| r.get("approvalBypass"))
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default(),
                },
            }
        },
        security,
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
                "command": ["sh"], "inputs": [], "outputs": [],
                "cwd": "a/../b"}}}),
        );
        // schema passes but resolveProcessPolicy rejects the cwd traversal.
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
        let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
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
        assert_eq!(
            normalize_policy(&raw, &std::collections::BTreeSet::new())
                .unwrap()
                .execution
                .max_turns,
            None
        );
        // complete schema では maxTurns は required — 欠落は reject。
        // partial(project/user layer)では欠落可で `None` になる。
        let mut p = valid_policy();
        p["execution"].as_object_mut().unwrap().remove("maxTurns");
        assert!(validate_policy_schema(&p, PolicySchemaMode::Complete).is_err());
        let raw = validate_policy_schema(&p, PolicySchemaMode::Partial).unwrap();
        assert_eq!(
            normalize_policy(&raw, &std::collections::BTreeSet::new())
                .unwrap()
                .execution
                .max_turns,
            None
        );
        // 明示整数は bounded のまま。
        assert_eq!(
            normalize_policy(
                &validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap(),
                &std::collections::BTreeSet::new(),
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
    fn route_section_schema_and_normalize() {
        // complete/partial で受理 — `route` は optional property。
        let mut p = valid_policy();
        p["route"] = json!({"rewrite": {"approvalBypass": ["codex"]}});
        for mode in [PolicySchemaMode::Complete, PolicySchemaMode::Partial] {
            let raw = validate_policy_schema(&p, mode).unwrap();
            let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
            assert_eq!(
                policy.route.rewrite.approval_bypass,
                vec!["codex".to_string()]
            );
            assert!(policy.route.rewrite.approval_bypass_enabled("codex"));
            assert!(!policy.route.rewrite.approval_bypass_enabled("cursor"));
        }
        // domain 内の複数 target も受理される。
        p["route"] = json!({"rewrite": {"approvalBypass": ["codex", "cursor", "copilot"]}});
        let raw = validate_policy_schema(&p, PolicySchemaMode::Partial).unwrap();
        let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
        for t in ["codex", "cursor", "copilot"] {
            assert!(policy.route.rewrite.approval_bypass_enabled(t), "{t}");
        }
        // serialize は `approvalBypass`(camelCase)— `RuntimePolicy.route`
        // が serialize 出力へ載る(yaml 表現と 1:1)。
        p["route"] = json!({"rewrite": {"approvalBypass": ["codex"]}});
        let raw = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap();
        let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
        assert_eq!(
            serde_json::to_value(&policy)
                .unwrap()
                .pointer("/route/rewrite/approvalBypass")
                .unwrap(),
            &json!(["codex"])
        );

        // 未指定・`null`・空配列は `[]` 等価(default = bypass 無効)。
        for route in [
            json!({}),
            json!({"rewrite": {}}),
            json!({"rewrite": {"approvalBypass": null}}),
            json!({"rewrite": {"approvalBypass": []}}),
        ] {
            let mut p = valid_policy();
            p["route"] = route;
            let raw = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap();
            assert!(normalize_policy(&raw, &std::collections::BTreeSet::new())
                .unwrap()
                .route
                .rewrite
                .approval_bypass
                .is_empty());
        }
        // `route` 自体の欠落も `[]`。
        let raw = validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap();
        assert!(normalize_policy(&raw, &std::collections::BTreeSet::new())
            .unwrap()
            .route
            .rewrite
            .approval_bypass
            .is_empty());

        // project partial は `route` key を持つ policy を拒否
        // (section 丸ごと除外 — `command` と同型の fail-closed)。
        let mut p = valid_policy();
        p["route"] = json!({"rewrite": {"approvalBypass": ["codex"]}});
        let e = validate_policy_schema(&p, PolicySchemaMode::ProjectPartial).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);

        // 値 domain 外は fail-loud — 未知 target 名・非配列・非文字列
        // item・重複・非 object の `rewrite`/`route` は
        // `InvalidRuntimePolicy`(`null` も `route`/`rewrite` には不可)。
        for route in [
            json!({"rewrite": {"approvalBypass": ["windsurf"]}}),
            json!({"rewrite": {"approvalBypass": "codex"}}),
            json!({"rewrite": {"approvalBypass": [1]}}),
            json!({"rewrite": {"approvalBypass": ["codex", "codex"]}}),
            json!({"rewrite": null}),
            json!({"rewrite": []}),
            json!(null),
            json!([]),
            json!("codex"),
        ] {
            let mut p = valid_policy();
            p["route"] = route.clone();
            for mode in [PolicySchemaMode::Complete, PolicySchemaMode::Partial] {
                let e = validate_policy_schema(&p, mode).unwrap_err();
                assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{route}");
            }
        }
    }

    /// `security` section の schema 受理/拒否と `RuntimePolicy.
    /// security` への normalize(issue #164 —
    /// `route_section_schema_and_normalize` と同型)。
    #[test]
    fn security_section_schema_and_normalize() {
        // complete/partial で受理 — `security` は optional property。
        let mut p = valid_policy();
        p["security"] = json!({"execDenyPatterns": ["python.*", "ssh"]});
        for mode in [PolicySchemaMode::Complete, PolicySchemaMode::Partial] {
            let raw = validate_policy_schema(&p, mode).unwrap();
            let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
            assert_eq!(
                policy.security.exec_deny_patterns,
                vec!["python.*".to_string(), "ssh".to_string()]
            );
        }
        // serialize は `execDenyPatterns` — `RuntimePolicy.security`
        // が serialize 出力へ載る(yaml 表現と 1:1)。
        let raw = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap();
        let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
        assert_eq!(
            serde_json::to_value(&policy)
                .unwrap()
                .pointer("/security/execDenyPatterns")
                .unwrap(),
            &json!(["python.*", "ssh"])
        );

        // 既定形 — 未指定・`{"execDenyPatterns":null}`・
        // `{"execDenyPatterns":[]}` は全て空集合に normalize。
        for security in [
            json!({}),
            json!({"execDenyPatterns": null}),
            json!({"execDenyPatterns": []}),
        ] {
            let mut p = valid_policy();
            p["security"] = security;
            let raw = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap();
            assert!(normalize_policy(&raw, &std::collections::BTreeSet::new())
                .unwrap()
                .security
                .exec_deny_patterns
                .is_empty());
        }
        // `security` 自体の欠落も空集合。
        let raw = validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap();
        assert!(normalize_policy(&raw, &std::collections::BTreeSet::new())
            .unwrap()
            .security
            .exec_deny_patterns
            .is_empty());

        // project partial は `security` key を持つ policy を拒否
        // (section 丸ごと除外 — `command`/`route` と同型の
        // fail-closed)。
        let mut p = valid_policy();
        p["security"] = json!({"execDenyPatterns": ["x"]});
        let e = validate_policy_schema(&p, PolicySchemaMode::ProjectPartial).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);

        // malformed shape は fail-loud — 非 object の `security`
        // (`null` を含む — section は `object_schema` で nullable で
        // はない)・非配列・非 string item・空 string item・129 個超・
        // 重複 item・`security` object の未知 key は
        // `InvalidRuntimePolicy`。
        for security in [
            json!(null),
            json!([]),
            json!("python.*"),
            json!({"execDenyPatterns": "python.*"}),
            json!({"execDenyPatterns": [1]}),
            json!({"execDenyPatterns": [""]}),
            json!({"execDenyPatterns": ["a", "a"]}),
            json!({"execDenyPatterns": ["x"], "bogus": 1}),
        ] {
            let mut p = valid_policy();
            p["security"] = security.clone();
            for mode in [PolicySchemaMode::Complete, PolicySchemaMode::Partial] {
                let e = validate_policy_schema(&p, mode).unwrap_err();
                assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{security}");
            }
        }
        // maxItems 超過(129 個)も schema が拒否。
        let mut p = valid_policy();
        p["security"] = json!({
            "execDenyPatterns": (0..129).map(|i| format!("p{i}")).collect::<Vec<_>>()
        });
        let e = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    /// 全層 fail-closed — `command` section 欠落の security-only doc
    /// に含まれる invalid regex も `validate_policy_schema` が
    /// `InvalidRuntimePolicy` で拒否する(`resolve_exec_deny_list` の
    /// 無条件評価 — design 契約)。`normalize_policy` も同 error。
    #[test]
    fn security_invalid_regex_fails_closed_at_all_layers() {
        // security-only doc — `command` 欠落でも invalid regex を
        // layer 単位で拒否する。
        let partial = json!({
            "version": 1,
            "security": {"execDenyPatterns": ["("]},
        });
        let e = validate_policy_schema(&partial, PolicySchemaMode::Partial).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("invalid execDenyPatterns regex"));
        // normalize 層でも同 error(validate を潜った入力への防御)。
        let raw = validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap();
        let mut raw = raw.clone();
        raw.insert("security".to_string(), json!({"execDenyPatterns": ["("]}));
        let e = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    /// 適用点 1 — `security.execDenyPatterns` に match する declared
    /// executable を持つ grant は validate/normalize の両層で
    /// `InvalidRuntimePolicy`(issue #164)。
    #[test]
    fn security_deny_rejects_matching_declared_executable() {
        let mut p = valid_policy();
        p["security"] = json!({"execDenyPatterns": ["python.*"]});
        p["command"] = json!({
            "maxConcurrent": 1,
            "grants": {"py": {"executable": "python3"}},
        });
        // declared stem match — schema 検査後の section resolve で
        // fail-closed。
        let e = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("denied by security.execDenyPatterns"));
        assert_eq!(e.details().unwrap()["grant"], json!("py"));
        assert_eq!(e.details().unwrap()["executable"], json!("python3"));
        // normalize 層でも同 error(grants に match する executable が
        // 残る入力への同型評価)。
        let mut raw = validate_policy_schema(&valid_policy(), PolicySchemaMode::Complete).unwrap();
        raw.insert(
            "security".to_string(),
            json!({"execDenyPatterns": ["python.*"]}),
        );
        raw.insert(
            "command".to_string(),
            json!({"grants": {"py": {"executable": "python3"}}}),
        );
        let e = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        // 非 match grant + deny は受理 — `security` は normalize 出力に
        // 載る。
        let mut p = valid_policy();
        p["security"] = json!({"execDenyPatterns": ["python.*"]});
        p["command"] = json!({
            "maxConcurrent": 1,
            "grants": {"g": {"executable": "git"}},
        });
        let raw = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap();
        let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
        assert_eq!(
            policy.security.exec_deny_patterns,
            vec!["python.*".to_string()]
        );
        assert_eq!(policy.exec.grants["g"].executable, "git");
    }

    /// collision(issue #165)— `command.grants` × `process.tasks` の
    /// 同名交差は `build_exec_input` で `InvalidRuntimePolicy` +
    /// `details.{grant,task}`。`validate_policy_schema` は ∅ names で
    /// 呼ぶため per-doc collision が全 mode で発火する(layer 非依存
    /// pin — ProjectPartial は `command` section 自体を schema
    /// reject するため collision に到達しないが error code は同一)。
    #[test]
    fn cross_section_name_collision_rejected_at_all_modes() {
        let mut p = valid_policy();
        p["process"] = json!({
            "maxConcurrent": 1,
            "tasks": {"shared": {"command": ["cc"], "inputs": [], "outputs": []}},
        });
        p["command"] = json!({
            "maxConcurrent": 1,
            "grants": {"shared": {"executable": "git"}},
        });
        for mode in [PolicySchemaMode::Complete, PolicySchemaMode::Partial] {
            let e = validate_policy_schema(&p, mode).unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{mode:?}");
            assert_eq!(e.details().unwrap()["grant"], json!("shared"), "{mode:?}");
            assert_eq!(e.details().unwrap()["task"], json!("shared"), "{mode:?}");
        }
        // ProjectPartial は `command` key を schema が拒否 —
        // collision には到達しないが同じ `InvalidRuntimePolicy`。
        let e = validate_policy_schema(&p, PolicySchemaMode::ProjectPartial).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        // 別名なら受理。
        let mut ok = valid_policy();
        ok["process"] = json!({
            "maxConcurrent": 1,
            "tasks": {"t": {"command": ["cc"], "inputs": [], "outputs": []}},
        });
        ok["command"] = json!({
            "maxConcurrent": 1,
            "grants": {"g": {"executable": "git"}},
        });
        assert!(validate_policy_schema(&ok, PolicySchemaMode::Complete).is_ok());
    }

    /// serialized `RuntimePolicy.exec` shape(issue #165)—
    /// `{"grants":{name:ExecGrant},"command":{defaults},
    /// "process":{defaults}}` で top-level `process`/`command` は
    /// 出ない。task 由来 grant は `fixed:true` + `argv`=command[1..]
    /// + `exitCodes:[0]` + section lifecycle 畳み込み。serialized
    ///   exec-doc は `resolve_exec_policy` へ再投入可能(round-trip)。
    #[test]
    fn runtime_policy_serializes_unified_exec_block() {
        use crate::policy::exec::{resolve_exec_policy, ExecGrantScope};
        use crate::policy::security::ExecDenyList;
        let mut p = valid_policy();
        p["process"] = json!({
            "maxConcurrent": 2,
            "timeout": "5m",
            "tasks": {
                "build": {"command": ["cc", "-O2", "x.c"],
                          "inputs": ["src/**"], "outputs": ["out"]},
            },
        });
        p["command"] = json!({
            "maxConcurrent": 4,
            "grants": {"g": {"executable": "git", "argv": ["log", "*"]}},
        });
        let raw = validate_policy_schema(&p, PolicySchemaMode::Complete).unwrap();
        let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
        let v = serde_json::to_value(&policy).unwrap();
        assert!(v.get("process").is_none() && v.get("command").is_none());
        let exec = &v["exec"];
        let build = &exec["grants"]["build"];
        assert_eq!(build["fixed"], json!(true));
        assert_eq!(build["executable"], json!("cc"));
        assert_eq!(build["argv"], json!(["-O2", "x.c"]));
        assert_eq!(build["inputs"], json!(["src/**"]));
        assert_eq!(build["outputs"], json!(["out"]));
        assert_eq!(build["timeout"], json!("5m")); // process section default
        assert_eq!(build["exitCodes"], json!([0]));
        assert_eq!(build["scope"], json!("trusted"));
        let g = &exec["grants"]["g"];
        assert_eq!(g["fixed"], json!(false));
        assert_eq!(g["scope"], json!("trusted"));
        assert!(g.get("inputs").is_none() && g.get("outputs").is_none());
        // defaults blocks — origin section 毎に保持。
        assert_eq!(exec["command"]["maxConcurrent"], json!(4));
        assert_eq!(exec["process"]["timeout"], json!("5m"));
        // serialized exec-doc → 再 resolve 冪等(invocation parity)。
        let reparsed = resolve_exec_policy(Some(exec), &ExecDenyList::default()).unwrap();
        assert_eq!(policy.exec, reparsed);
        assert_eq!(policy.exec.grants["build"].scope, ExecGrantScope::Trusted);
    }

    #[test]
    fn normalize_policy_rejects_unknown_and_duplicate_tools() {
        let mut bad = valid_policy();
        bad["tools"].as_object_mut().unwrap()["allow"] = json!(["read", "bogus"]);
        let raw = validate_policy_schema(&bad, PolicySchemaMode::Complete).unwrap();
        let e = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("unknown tool in policy"));

        let mut dup = valid_policy();
        dup["tools"].as_object_mut().unwrap()["allow"] = json!(["read", "Read"]);
        let raw = validate_policy_schema(&dup, PolicySchemaMode::Complete).unwrap();
        let e = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap_err();
        assert_eq!(e.details().unwrap()["reason"], json!("tools-duplicate"));

        // Alias is normalized to the canonical name.
        let mut alias = valid_policy();
        alias["tools"].as_object_mut().unwrap()["allow"] = json!(["Read"]);
        let raw = validate_policy_schema(&alias, PolicySchemaMode::Complete).unwrap();
        let policy = normalize_policy(&raw, &std::collections::BTreeSet::new()).unwrap();
        assert_eq!(policy.tools.allow, vec![ToolName::Read]);
    }
}
