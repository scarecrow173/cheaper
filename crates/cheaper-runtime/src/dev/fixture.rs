//! T4 fixture DSL — `tests/fixtures/*.json` の serde schema・loader・
//! `run_worker` 入力(worker/policy/plugin/env/sessions)への変換。
//!
//! 1 fixture = 1 `run_worker` 呼び出しを declarative に記述する:
//!
//! ```json
//! {
//!   "name": "…",
//!   "worker":   { "name"?, "description"?, "tools"?,
//!                 "write_scope"?, "model_tier"?, "output_schema"?,
//!                 "prompt"? },                  // → worker .md 生成
//!   "policy":   { "timeout_ms"?, "max_turns"?, "max_tool_calls"?,
//!                 "repair_attempts"?, "transient_attempts"? },
//!   "provider": { "api_key"? },                 // apiKey template
//!   "model":    { "id"?, "api"?, "cost"? },     // bootstrap model
//!   "explicit_model": "cheaper-fake/fake-model",
//!   "env":      { "VAR": "value" },             // ProviderEnv overlay
//!   "auth_logins": { "cheaper-fake": "key" },   // login → stored cred
//!   "input":    { … },                          // run input
//!   "sessions": [ { "steps": [ <step>… ] } ],   // main → repair 順
//!   "abort":    { "after_ms": 40, "message"? }, // signal abort
//!   "abort_grace_ms": 80,
//!   "expect":   { "status", "result"?, "error_code"?, "cause_code"?,
//!                 "error_details"?, "exit_code"?, "meta"?, … }
//! }
//! ```
//!
//! step 形(scripted stream 層の [`ScriptStep`] へ 1:1 変換):
//!
//! | fixture step                                   | 意味                        |
//! |------------------------------------------------|-----------------------------|
//! | `{"emit": {"text_delta": "…"}}`                | `RunEvent` emit(turn 中継)  |
//! | `{"delay_ms": n}`                              | 待ち(abort で中断)          |
//! | `{"tool_calls": [{"name","input"}]}`           | tool 実行(計上/journal)     |
//! | `{"usage_delta": {"input","output",…}}`        | 1 call 分の usage 加算      |
//! | `{"turn": {"tool_calls": bool}}`               | 受理 turn 計上              |
//! | `{"error": {"status","retryable","message"}}`  | provider error(retry 判定)  |
//! | `"hang_until_abort"` / `{"hang_until_abort":x}`| signal abort 待ち           |
//! | `"hang"` / `{"hang":x}`                        | signal 非観測の永久 pending |
//! | `"abort_response"` / `{"abort_response":x}`    | provider 側 cancel 応答     |
//! | `{"panic": "…"}`                               | stream 内部 panic           |
//! | `{"finish": {"text": "…"}}` / `{"finish": "…"}`| 最終 text で `Finished`     |
//!
//! `error` step の retry 意味論は adapter `RetryingCompletion` と同じで
//! fake_provider 側で分類する(`docs/` 参照 — emit 後は retry しない)。
//!
//! normalize 規則(設計書 §fixture 比較): harness は `expect` の宣言
//! field のみ比較するため `meta.durationMs`・`meta.execution.*Ms`・
//! `error.message` 本文・stderr・JSON key 順は自然に除外される。
//! `usage.costUsd` は provider 応答値ではなく pricing 層算出値を比較
//! する(fake model cost は `CheaperFakePlugin::default_cost` 固定)。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::tier::Tier;
use cheaper_core::policy::command::ResolvedCommandPolicy;
use cheaper_core::policy::process::{ProcessHeartbeat, ResolvedProcessPolicy};
use cheaper_core::policy::schema::{
    ExecutionPolicy, FilesystemPolicy, ModelPolicy, OutputPolicy, RetryPolicy, RuntimePolicy,
    TierFallback, ToolsPolicy,
};
use cheaper_core::policy::workflow::{WorkflowArtifactPolicy, WorkflowPolicy};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::parse::parse_worker_file;
use cheaper_core::worker::validate::{validate_worker_definition, WorkerDefinition, WorkerOrigin};
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_rig_adapter::{RunEvent, UsageSnapshot};
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::fake_provider::{
    CheaperFakePlugin, FakeResponse, FakeSessionFactory, ScriptStep, ScriptedSession, ToolCallSpec,
};
use crate::auth::api_key::ProviderEnv;
use crate::auth::service::{AuthInteraction, AuthPrompt};
use crate::catalog::schema::{ModelCostJson, ModelDefinitionJson};

// ---------------------------------------------------------------------------
// fixture file root
// ---------------------------------------------------------------------------

/// fixture file 1 件(`tests/fixtures/*.json`)。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct RunFixture {
    /// fixture 名(診断表示用。file stem に fallback)。
    pub name: Option<String>,
    pub description: Option<String>,
    pub worker: FixtureWorker,
    pub policy: FixturePolicy,
    pub provider: FixtureProvider,
    pub model: FixtureModel,
    /// `--model` 相当。省略時は `cheaper-fake/<model.id or fake-model>`。
    pub explicit_model: Option<String>,
    /// `ProviderEnv` overlay(`api_key` の `"$VAR"` template 解決用)。
    pub env: BTreeMap<String, String>,
    /// `auth login` 相当 — providerId → api_key secret。run 前に
    /// `AuthService::login` で credential store へ永続化する。
    pub auth_logins: BTreeMap<String, String>,
    /// run input(`build_user_prompt` / writeScope 抽出に使う)。
    pub input: Value,
    /// `create_session` 毎の script(main → repair 順)。
    pub sessions: Vec<FixtureSession>,
    /// run 中の外部 signal abort。
    pub abort: Option<FixtureAbort>,
    /// `abortGraceMs` override。省略時は harness 側 default(80ms)。
    pub abort_grace_ms: Option<f64>,
    pub expect: FixtureExpect,
}

impl Default for RunFixture {
    fn default() -> Self {
        Self {
            name: None,
            description: None,
            worker: FixtureWorker::default(),
            policy: FixturePolicy::default(),
            provider: FixtureProvider::default(),
            model: FixtureModel::default(),
            explicit_model: None,
            env: BTreeMap::new(),
            auth_logins: BTreeMap::new(),
            // default `writeScope: files` の抽出対象。
            input: serde_json::json!({"files": ["a.txt"]}),
            sessions: Vec::new(),
            abort: None,
            abort_grace_ms: None,
            expect: FixtureExpect::default(),
        }
    }
}

impl RunFixture {
    /// fixture JSON を parse する。
    pub fn load(path: &Path) -> Result<Self, CheaperError> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("failed to read fixture {}: {e}", path.display()),
            )
        })?;
        Self::parse(&text)
            .map_err(|e| CheaperError::new(e.code(), format!("fixture {}: {e}", path.display())))
    }

    /// fixture JSON 文字列を parse する。
    pub fn parse(text: &str) -> Result<Self, CheaperError> {
        serde_json::from_str(text).map_err(|e| {
            CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("invalid fixture json: {e}"),
            )
        })
    }

    /// catalog/auth 層 — provider(`api_key` template)+ bootstrap model を
    /// 反映した `cheaper-fake` plugin。
    pub fn plugin(&self) -> CheaperFakePlugin {
        let mut plugin = CheaperFakePlugin::new();
        if let Some(api_key) = &self.provider.api_key {
            plugin = plugin.with_api_key(api_key.clone());
        }
        if self.model.has_overrides() {
            plugin = plugin.with_models(vec![self.model_definition()]);
        }
        plugin
    }

    /// bootstrap model 定義(fixture `model` の id/api/cost 反映済み)。
    pub fn model_definition(&self) -> ModelDefinitionJson {
        let mut def = CheaperFakePlugin::default_model();
        if let Some(id) = &self.model.id {
            def.id = id.clone();
            def.name = Some(id.clone());
        }
        if let Some(api) = &self.model.api {
            def.api = Some(api.clone());
        }
        if let Some(cost) = &self.model.cost {
            def.cost = Some(cost.to_cost_json());
        }
        def
    }

    /// explicit model ref(default `cheaper-fake/<model.id or fake-model>`)。
    pub fn explicit_model(&self) -> String {
        self.explicit_model.clone().unwrap_or_else(|| {
            format!(
                "{}/{}",
                CheaperFakePlugin::PROVIDER_ID,
                self.model
                    .id
                    .as_deref()
                    .unwrap_or(CheaperFakePlugin::MODEL_ID)
            )
        })
    }

    /// fixture `env` を返す `ProviderEnv`。
    pub fn env(&self) -> FixtureEnv {
        FixtureEnv {
            vars: self.env.clone(),
        }
    }

    /// scripted stream 層 — `sessions[].steps` を `ScriptStep` へ変換した
    /// `FakeSessionFactory`。
    pub fn session_factory(&self) -> FakeSessionFactory {
        let responses = self
            .sessions
            .iter()
            .map(|s| {
                FakeResponse::Scripted(ScriptedSession {
                    steps: s.steps.iter().map(FixtureStep::to_script_step).collect(),
                })
            })
            .collect();
        FakeSessionFactory::new(responses)
    }

    /// `RuntimePolicy` — `default_policy()` に fixture `policy` override
    /// を反映する。
    pub fn policy(&self) -> RuntimePolicy {
        let mut policy = default_policy();
        self.policy.apply_to(&mut policy);
        policy
    }

    /// `worker` 節 → worker .md frontmatter + body。
    pub fn worker_markdown(&self) -> String {
        self.worker.to_markdown()
    }

    /// worker .md を parse/validate した `WorkerDefinition`。
    /// `source_path` は workspace root 直下の file path を渡す
    /// (deny_paths 等の root 相対判定に使う)。
    pub fn worker_definition(&self, source_path: &Path) -> Result<WorkerDefinition, CheaperError> {
        let md = self.worker_markdown();
        let source = source_path.to_string_lossy().to_string();
        let parsed = parse_worker_file(&md, &source)?;
        validate_worker_definition(&parsed, &source, WorkerOrigin::Explicit)
    }
}

// ---------------------------------------------------------------------------
// worker / policy / provider / model 節
// ---------------------------------------------------------------------------

/// `worker` 節 — worker .md 生成の入力。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct FixtureWorker {
    pub name: String,
    pub description: String,
    /// `tools: [read, write]`。
    pub tools: Vec<String>,
    /// `cheaper.optionalTools` — content-gate(nonempty tasks/grants)
    /// 不成立時に未登録へ縮退してよい scope-gated tool の opt-in 宣言
    /// (ADR-034ESBcMTWxNCMDI9obdb8)。非空時のみ frontmatter へ emit。
    pub optional_tools: Vec<String>,
    /// `cheaper.writeScope`。`None` で frontmatter から省略。
    pub write_scope: Option<String>,
    /// `cheaper.model.tier`。
    pub model_tier: Option<String>,
    /// `cheaper.output.schema.inline`(JSON schema object)。
    pub output_schema: Value,
    /// worker 本文 prompt。
    pub prompt: String,
}

impl Default for FixtureWorker {
    fn default() -> Self {
        Self {
            name: "fixture-worker".to_string(),
            description: "fixture worker".to_string(),
            tools: vec!["read".to_string(), "write".to_string()],
            optional_tools: Vec::new(),
            write_scope: Some("files".to_string()),
            model_tier: Some("low".to_string()),
            output_schema: serde_json::json!({
                "type": "object",
                "required": ["ok"],
                "properties": { "ok": { "type": "boolean" } }
            }),
            prompt: "Do the thing.".to_string(),
        }
    }
}

impl FixtureWorker {
    /// frontmatter(YAML)+ body へ展開する。`output_schema` は JSON inline
    /// で埋める(YAML は JSON superset — flow mapping として valid)。
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("---\n");
        out.push_str(&format!("name: {}\n", self.name));
        out.push_str(&format!("description: {}\n", self.description));
        out.push_str(&format!("tools: [{}]\n", self.tools.join(", ")));
        out.push_str("cheaper:\n");
        if !self.optional_tools.is_empty() {
            out.push_str(&format!(
                "  optionalTools: [{}]\n",
                self.optional_tools.join(", ")
            ));
        }
        if let Some(tier) = &self.model_tier {
            out.push_str(&format!("  model:\n    tier: {tier}\n"));
        }
        if let Some(scope) = &self.write_scope {
            out.push_str(&format!("  writeScope: {scope}\n"));
        }
        out.push_str("  output:\n    schema:\n      inline: ");
        out.push_str(
            &serde_json::to_string(&self.output_schema)
                .expect("fixture output_schema must serialize"),
        );
        out.push_str("\n---\n");
        out.push_str(&self.prompt);
        out.push('\n');
        out
    }
}

/// `policy` 節 — `default_policy()` への部分 override。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixturePolicy {
    pub timeout_ms: Option<u64>,
    pub max_turns: Option<u64>,
    pub max_tool_calls: Option<u64>,
    pub repair_attempts: Option<u64>,
    /// `SessionSpec.transient_retries` へ流れる retry 予算。
    pub transient_attempts: Option<u64>,
}

impl FixturePolicy {
    pub fn apply_to(&self, policy: &mut RuntimePolicy) {
        if let Some(v) = self.timeout_ms {
            policy.execution.timeout_ms = v;
        }
        if let Some(v) = self.max_turns {
            policy.execution.max_turns = v;
        }
        if let Some(v) = self.max_tool_calls {
            policy.execution.max_tool_calls = v;
        }
        if let Some(v) = self.repair_attempts {
            policy.output.repair_attempts = v;
        }
        if let Some(v) = self.transient_attempts {
            policy.retry.transient_attempts = v;
        }
    }
}

/// `provider` 節 — `cheaper-fake` provider の差し替え点。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixtureProvider {
    /// `apiKey` config 値(literal または `"$VAR"`/`"!cmd"` template)。
    /// 省略時は literal `"fake-key"`(keyless)。
    pub api_key: Option<String>,
}

/// `model` 節 — bootstrap `fake-model` の差し替え点。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixtureModel {
    pub id: Option<String>,
    /// model-level api kind。provider `cheaper-fake-api` と異なる値で
    /// unsupported api → `INVALID_MODEL_CONFIG` を検証できる。
    pub api: Option<String>,
    pub cost: Option<FixtureCost>,
}

impl FixtureModel {
    fn has_overrides(&self) -> bool {
        self.id.is_some() || self.api.is_some() || self.cost.is_some()
    }
}

/// `models[].cost` の fixture 形(`models.json` と同じ camelCase)。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

impl FixtureCost {
    fn to_cost_json(&self) -> ModelCostJson {
        ModelCostJson {
            input: self.input,
            output: self.output,
            cache_read: self.cache_read,
            cache_write: self.cache_write,
            tiers: None,
        }
    }
}

// ---------------------------------------------------------------------------
// sessions / steps
// ---------------------------------------------------------------------------

/// `create_session` 1 回分の script。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixtureSession {
    pub steps: Vec<FixtureStep>,
}

/// scripted step — bare string 形と `{"key": …}` object 形の両方を受ける。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FixtureStep {
    /// `"hang_until_abort"` / `"hang"` / `"abort_response"`。
    Bare(BareStep),
    /// `{"delay_ms": 50}` 等の keyed 形。
    Keyed(KeyedStep),
}

/// bare string step(終端/pending 系のみ)。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BareStep {
    HangUntilAbort,
    Hang,
    AbortResponse,
}

/// keyed step(object 形)。unit 系も `{"hang_until_abort": true}` で
/// 書けるよう `IgnoredAny` payload を受ける。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyedStep {
    /// `{"emit": {"text_delta": "…"}}` — `RunEvent` を 1 件 emit。
    /// 現 call の item 記録になり、以後の `error` は retry 対象外。
    Emit(FixtureEvent),
    /// `{"delay_ms": n}` — n ms 待つ。abort signal で中断する。
    DelayMs(u64),
    /// `{"tool_calls": [{"name": "write", "input": {…}}]}` —
    /// `tool_runtime.execute` を逐次実行。call 境界を切る。
    ToolCalls(Vec<FixtureToolCall>),
    /// `{"usage_delta": {"input": 100, "output": 20, …}}` —
    /// 1 completion call 分の usage を加算。
    UsageDelta(FixtureUsage),
    /// `{"turn": {"tool_calls": true}}` — 受理 turn を 1 件計上。
    /// 次の `error` は新しい call の pre-item 失敗として retry 判定される。
    Turn(FixtureTurn),
    /// `{"error": {"status": 429, "retryable": true, "message": "…"}}` —
    /// provider failure。retry 条件(adapter parity)を満たせば
    /// `transientRetries` 計上 + 次 step、尽きるか permanent なら終端
    /// `PROVIDER_FAILURE`。
    Error(FixtureError),
    /// `{"panic": "…"}` — stream 内部 panic。
    Panic(String),
    /// `{"finish": {"text": "…"}}` or `{"finish": "…"}` — `Finished` emit。
    Finish(FixtureFinish),
    /// `{"hang_until_abort": true}` — signal abort 待ち(終端 abort error)。
    HangUntilAbort(IgnoredAny),
    /// `{"hang": true}` — signal を観測しない永久 pending。
    Hang(IgnoredAny),
    /// `{"abort_response": true}` — provider 側 cancel 応答
    /// (`PromptCancelled` → `ABORTED` 写像相当)。
    AbortResponse(IgnoredAny),
}

impl FixtureStep {
    /// DSL step → [`ScriptStep`] 変換(1:1)。
    pub fn to_script_step(&self) -> ScriptStep {
        match self {
            FixtureStep::Bare(bare) => match bare {
                BareStep::HangUntilAbort => ScriptStep::WaitAbort,
                BareStep::Hang => ScriptStep::Hang,
                BareStep::AbortResponse => ScriptStep::AbortResponse,
            },
            FixtureStep::Keyed(keyed) => match keyed {
                KeyedStep::Emit(event) => ScriptStep::Emit {
                    event: event.to_run_event(),
                },
                KeyedStep::DelayMs(ms) => ScriptStep::Delay { ms: *ms },
                KeyedStep::ToolCalls(calls) => ScriptStep::ToolCalls {
                    calls: calls
                        .iter()
                        .map(|c| ToolCallSpec {
                            name: c.name.clone(),
                            input: c.input.clone(),
                        })
                        .collect(),
                },
                KeyedStep::UsageDelta(usage) => ScriptStep::UsageDelta {
                    usage: usage.to_snapshot(),
                },
                KeyedStep::Turn(turn) => ScriptStep::Turn {
                    tool_calls: turn.tool_calls,
                },
                KeyedStep::Error(error) => ScriptStep::ProviderError {
                    status: error.status,
                    retryable: error.retryable,
                    message: error
                        .message
                        .clone()
                        .unwrap_or_else(|| "fixture provider error".to_string()),
                },
                KeyedStep::Panic(message) => ScriptStep::Panic {
                    message: message.clone(),
                },
                KeyedStep::Finish(finish) => ScriptStep::Finish {
                    text: finish.text().to_string(),
                },
                KeyedStep::HangUntilAbort(_) => ScriptStep::WaitAbort,
                KeyedStep::Hang(_) => ScriptStep::Hang,
                KeyedStep::AbortResponse(_) => ScriptStep::AbortResponse,
            },
        }
    }
}

/// `emit` step の `RunEvent`。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureEvent {
    /// `{"text_delta": "…"}` → `RunEvent::TextDelta`。
    TextDelta(String),
    /// `{"reasoning_delta": "…"}` → `RunEvent::ReasoningDelta`。
    ReasoningDelta(String),
}

impl FixtureEvent {
    fn to_run_event(&self) -> RunEvent {
        match self {
            FixtureEvent::TextDelta(text) => RunEvent::TextDelta { text: text.clone() },
            FixtureEvent::ReasoningDelta(text) => RunEvent::ReasoningDelta { text: text.clone() },
        }
    }
}

/// `tool_calls` step の 1 実行。
#[derive(Debug, Deserialize)]
pub struct FixtureToolCall {
    pub name: String,
    #[serde(default)]
    pub input: Value,
}

/// `usage_delta` — 1 completion call 分の token usage(`usage.*`
/// camelCase は `meta.usage` の JSON field 名に揃える)。
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FixtureUsage {
    pub input: u64,
    pub output: u64,
    /// `cachedInputTokens`。
    pub cache_read: u64,
    /// `cacheCreationInputTokens`。
    pub cache_write: u64,
    /// `totalTokens`。省略時は 4 field の和。
    pub total: Option<u64>,
    pub reasoning: u64,
    pub tool_use_prompt: u64,
}

impl FixtureUsage {
    fn to_snapshot(&self) -> UsageSnapshot {
        UsageSnapshot {
            input_tokens: self.input,
            output_tokens: self.output,
            cached_input_tokens: self.cache_read,
            cache_creation_input_tokens: self.cache_write,
            total_tokens: self
                .total
                .unwrap_or(self.input + self.output + self.cache_read + self.cache_write),
            reasoning_tokens: self.reasoning,
            tool_use_prompt_tokens: self.tool_use_prompt,
        }
    }
}

/// `turn` step(`tool_calls` = 最終 turn が tool 継続終了か)。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixtureTurn {
    pub tool_calls: bool,
}

/// `error` step — `{status?, retryable?, message?}`。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixtureError {
    /// HTTP status(429/5xx → transient 分類)。
    pub status: Option<u16>,
    /// status 無し時の transient 主張。
    pub retryable: Option<bool>,
    pub message: Option<String>,
}

/// `finish` step — `{"finish": "…"}` / `{"finish": {"text": "…"}}` 両対応。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FixtureFinish {
    Text(String),
    Obj { text: String },
}

impl FixtureFinish {
    fn text(&self) -> &str {
        match self {
            FixtureFinish::Text(s) => s,
            FixtureFinish::Obj { text } => text,
        }
    }
}

/// `abort` 節 — run 中の外部 signal abort。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixtureAbort {
    /// run 開始から signal abort までの遅延(ms)。必須。
    pub after_ms: u64,
    /// abort reason message(default `"user abort"`)。
    pub message: Option<String>,
}

// ---------------------------------------------------------------------------
// expect 節
// ---------------------------------------------------------------------------

/// `expect` 節 — 宣言 field のみ比較する normalize 規則。
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FixtureExpect {
    /// `"ok"` | `"error"`。
    pub status: Option<ExpectedStatus>,
    /// ok 時の result object。
    pub result: Option<Value>,
    /// error 時の `CheaperError.code`(`"PROVIDER_FAILURE"` 等)。
    pub error_code: Option<ErrorCode>,
    /// `PARTIAL_MUTATION` 等の cause `ErrorCode`。
    pub cause_code: Option<ErrorCode>,
    /// `error.details` の subset 比較(宣言 key のみ)。
    pub error_details: Option<Map<String, Value>>,
    /// process exit code(`ERROR_TABLE`)。
    pub exit_code: Option<u8>,
    /// `meta` / `runMeta` の部分比較。
    pub meta: Option<FixtureMetaExpect>,
    /// 未消化 scripted session 数(TS `pendingResponses` 相当)。
    pub pending_responses: Option<usize>,
}

/// `expect.status`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedStatus {
    Ok,
    Error,
}

/// `expect.meta` — 宣言 field のみ比較。
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FixtureMetaExpect {
    /// `{provider?, id?, tier?}`。
    pub model: Option<FixtureModelExpect>,
    /// usage token field + `costUsd`(pricing 層算出)。
    pub usage: Option<FixtureUsageExpect>,
    pub turns: Option<u64>,
    pub tool_calls: Option<u64>,
    pub rejected_tool_calls: Option<u64>,
    pub repairs: Option<u64>,
    pub transient_retries: Option<u64>,
    /// `changes.totals.files` の厳密値。
    pub changes_files: Option<u64>,
    /// `changes.totals.files` の下限(journal 非空判定用)。
    pub changes_files_min: Option<u64>,
    /// `{directlyComparable?}`。
    pub savings: Option<FixtureSavingsExpect>,
}

/// `expect.meta.model`。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FixtureModelExpect {
    pub provider: Option<String>,
    pub id: Option<String>,
    /// `meta.model.tier`(`"low"` 等)。
    pub tier: Option<String>,
}

/// `expect.meta.usage` — `WorkerUsage` の JSON field 名に揃える。
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FixtureUsageExpect {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub total_tokens: Option<u64>,
    /// pricing 層算出 `costUsd`(abs diff < 1e-9 で比較)。
    pub cost_usd: Option<f64>,
}

/// `expect.meta.savings`。
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FixtureSavingsExpect {
    pub directly_comparable: Option<bool>,
}

// ---------------------------------------------------------------------------
// env / auth login seam
// ---------------------------------------------------------------------------

/// fixture `env` overlay の `ProviderEnv`(`fileExists`/`homeDir` は
/// fixture scope 外のため常に false/None)。
pub struct FixtureEnv {
    vars: BTreeMap<String, String>,
}

impl ProviderEnv for FixtureEnv {
    fn env(&self, name: &str) -> Option<String> {
        self.vars.get(name).filter(|v| !v.is_empty()).cloned()
    }
    fn file_exists(&self, _path: &str) -> bool {
        false
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
}

/// `auth_logins` 用の scripted interaction — secret prompt に固定値を
/// 返す(secret は prompt message に関わらず宣言値)。
pub struct ScriptedAuthInteraction {
    secret: String,
}

impl ScriptedAuthInteraction {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
        }
    }
}

impl AuthInteraction for ScriptedAuthInteraction {
    fn prompt(&mut self, _prompt: &AuthPrompt<'_>) -> Result<String, CheaperError> {
        Ok(self.secret.clone())
    }
}

/// fixture の default `RuntimePolicy` — replay harness との共有
/// baseline(`tests/run.rs` の policy と同じ shape)。
pub fn default_policy() -> RuntimePolicy {
    RuntimePolicy {
        workflow: WorkflowPolicy {
            max_jobs: 4,
            artifacts: WorkflowArtifactPolicy {
                max_artifact_bytes: 4096,
                max_total_bytes: 16384,
                max_preview_bytes: 4096,
                max_read_bytes: 4096,
            },
        },
        process: ResolvedProcessPolicy {
            max_concurrent: 4,
            tasks: Map::new(),
            timeout: "2m".into(),
            max_output_bytes: 4096,
            kill_grace: "2s".into(),
            heartbeat: ProcessHeartbeat {
                interval: "30s".into(),
            },
        },
        command: ResolvedCommandPolicy {
            max_concurrent: 4,
            grants: std::collections::BTreeMap::new(),
            timeout: "2m".into(),
            max_output_bytes: 4096,
            kill_grace: "2s".into(),
            heartbeat: ProcessHeartbeat {
                interval: "30s".into(),
            },
        },
        execution: ExecutionPolicy {
            timeout_ms: 60_000,
            max_turns: 10,
            max_tool_calls: 50,
            concurrency: None,
        },
        tools: ToolsPolicy {
            allow: vec![ToolName::Read, ToolName::Write],
        },
        filesystem: FilesystemPolicy {
            symlinks: SymlinkPolicy::Deny,
            deny_paths: vec![],
        },
        model: ModelPolicy {
            max_tier: Tier::High,
            tier_fallback: TierFallback::Higher,
        },
        output: OutputPolicy {
            max_tokens: 4096,
            repair_attempts: 2,
        },
        retry: RetryPolicy {
            transient_attempts: 2,
        },
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_fixture() {
        let f = RunFixture::parse(
            r#"{
                "sessions": [{"steps": [{"finish": {"text": "{\"ok\":true}"}}]}],
                "expect": {"status": "ok"}
            }"#,
        )
        .unwrap();
        assert_eq!(f.sessions.len(), 1);
        assert!(matches!(f.expect.status, Some(ExpectedStatus::Ok)));
        assert_eq!(f.explicit_model(), "cheaper-fake/fake-model");
    }

    #[test]
    fn step_forms_convert_to_script_steps() {
        let f = RunFixture::parse(
            r#"{
                "sessions": [{"steps": [
                    {"emit": {"text_delta": "hi"}},
                    {"delay_ms": 10},
                    {"tool_calls": [{"name": "write", "input": {"path": "a"}}]},
                    {"usage_delta": {"input": 10, "output": 5}},
                    {"turn": {"tool_calls": true}},
                    {"error": {"status": 429, "retryable": true, "message": "rate"}},
                    "hang_until_abort",
                    {"hang": true},
                    "abort_response",
                    {"panic": "boom"},
                    {"finish": "done"}
                ]}],
                "expect": {"status": "error", "error_code": "ABORTED"}
            }"#,
        )
        .unwrap();
        let steps = &f.sessions[0].steps;
        assert_eq!(steps.len(), 11);
        assert!(matches!(steps[6].to_script_step(), ScriptStep::WaitAbort));
        assert!(matches!(steps[7].to_script_step(), ScriptStep::Hang));
        assert!(matches!(
            steps[8].to_script_step(),
            ScriptStep::AbortResponse
        ));
        match steps[5].to_script_step() {
            ScriptStep::ProviderError {
                status,
                retryable,
                message,
            } => {
                assert_eq!(status, Some(429));
                assert_eq!(retryable, Some(true));
                assert_eq!(message, "rate");
            }
            _ => panic!("expected ProviderError"),
        }
    }

    #[test]
    fn usage_delta_defaults_total_to_sum() {
        let u = FixtureUsage {
            input: 1000,
            output: 2000,
            cache_read: 100,
            cache_write: 50,
            ..FixtureUsage::default()
        };
        assert_eq!(u.to_snapshot().total_tokens, 3150);
    }

    #[test]
    fn worker_markdown_generates_valid_frontmatter() {
        let f = RunFixture::default();
        let md = f.worker_markdown();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("name: fixture-worker"));
        assert!(md.contains("inline: {"));
        let dir = tempfile::tempdir().unwrap();
        // worker name = file stem が validate 規則(WorkerNameMismatch)。
        let def = f
            .worker_definition(&dir.path().join("fixture-worker.md"))
            .unwrap();
        assert_eq!(def.name, "fixture-worker");
    }

    /// `worker.optional_tools`(snake_case DSL key)が frontmatter の
    /// `cheaper.optionalTools`(camelCase contract key)へ非空時のみ
    /// emit される(design §(i) — regression harness が
    /// `worker_markdown()` を無条件で seed するため DSL knob が必須)。
    #[test]
    fn worker_optional_tools_emit_camel_frontmatter_key() {
        let f = RunFixture::parse(
            r#"{
                "worker": {
                    "name": "opt-worker",
                    "tools": ["read", "process", "command"],
                    "optional_tools": ["process", "command"],
                    "write_scope": null
                }
            }"#,
        )
        .unwrap();
        let md = f.worker_markdown();
        assert!(md.contains("  optionalTools: [process, command]\n"), "{md}");
        let dir = tempfile::tempdir().unwrap();
        let def = f
            .worker_definition(&dir.path().join("opt-worker.md"))
            .unwrap();
        assert_eq!(
            def.cheaper.optional_tools,
            [ToolName::Process, ToolName::Command]
        );
        assert!(def.cheaper.write_scope.is_none());
    }

    /// `optional_tools` 省略時は frontmatter に `optionalTools` を emit
    /// しない(既存 fixture の markdown 出力は不変)。
    #[test]
    fn worker_markdown_omits_optional_tools_when_empty() {
        let f = RunFixture::default();
        let md = f.worker_markdown();
        assert!(!md.contains("optionalTools"), "{md}");
    }

    #[test]
    fn plugin_overrides_model_cost_and_api() {
        let f = RunFixture::parse(
            r#"{
                "provider": {"api_key": "$CHEAPER_FAKE_API_KEY"},
                "model": {"id": "m2", "api": "mystery-api",
                          "cost": {"input": 3.0, "output": 4.0,
                                   "cacheRead": 0.1, "cacheWrite": 0.2}},
                "expect": {"status": "error"}
            }"#,
        )
        .unwrap();
        let def = f.model_definition();
        assert_eq!(def.id, "m2");
        assert_eq!(def.api.as_deref(), Some("mystery-api"));
        assert_eq!(def.cost.as_ref().unwrap().input, 3.0);
        assert_eq!(f.explicit_model(), "cheaper-fake/m2");
        assert_eq!(f.env().env("CHEAPER_FAKE_API_KEY"), None);
    }
}
