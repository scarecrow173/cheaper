//! `AgentBuilder` 構成と `RigSession` 構築。
//!
//! `runtime-pi/src/session.ts`(`createWorkerSession`) + `run.ts`
//! (`buildSystemPrompt`) の意味論を Rig `AgentBuilder` 上に固定する:
//! - system prompt は worker instructions + output contract を文字列完全再現
//! - `default_max_turns` へ十分大きな値(ISS-11: cheaper の hook 側
//!   `policy.execution.maxTurns` abort が正の経路で、こちらは runaway 対策の
//!   内部 guard)
//! - tool 実行は直列(`tool_concurrency(1)` — same-turn batch でも実行は
//!   cheaper-tools scope が管理する順序に従う)
//! - `InvalidToolCallAction::Retry{feedback}` を有効化するため
//!   `max_invalid_tool_call_retries` を明示(既定 0 は `Fail` に退化する)
//! - completion-call transient retry は [`RetryingCompletion`] handler 経由
//!   (`register_model` は `Into<DynModel>` しか受けないため、
//!   `model_route_handler` で登録して `set_model_label` で選択する)

use std::sync::Arc;

use rig::tool::ToolContext;
use rig::{operation::Completion, AgentBuilder, DynModel};

use cheaper_core::json::stringify::stringify_pretty;
use cheaper_tools::operation::AbortSignal;
use cheaper_tools::registry::ToolRuntime;

use crate::accumulator::{Accumulator, RunSnapshot};
use crate::hooks::{AccountingHook, ToolDetails};
use crate::retry::{RetryPolicy, RetryingCompletion};
use crate::run_stream::RunStream;

/// 内部 guard としての既定 max turns(ISS-11 選定値)。cheaper の
/// `policy.execution.maxTurns` が T3 の abort 経路で先に発火する前提の
/// 十分大きな値 — ここに到達した場合は `INTERNAL_ERROR` として区別する。
pub const DEFAULT_MAX_TURNS: usize = 1024;

/// `InvalidToolCallAction::Retry` の既定 budget。budget 0 は Rig が
/// `Fail` に fail-closed するため明示必須。小さすぎると正規な自己修復を
/// 殺すので、発散を防ぎつつ feedback で回復できる程度の値を置く。
pub const DEFAULT_INVALID_TOOL_RETRIES: usize = 8;

/// `model_route_handler` で登録する retry-wrapped model の label。
const RETRY_MODEL_LABEL: &str = "cheaper-retried";

/// Provider 供給の completion model ハンドル。中身は `DynModel` だが
/// adapter 外には opaque — provider crate(`cheaper-provider-devin` 等)だけが
/// [`from_rig`](Self::from_rig) で包む。
pub struct CompletionModel {
    inner: DynModel<Completion>,
    /// catalog の `model.maxTokens`(provider が `with_max_tokens` で設定)。
    /// `None` は上限未報告 — `min` の相手方として扱わない。
    max_tokens: Option<u64>,
}

impl CompletionModel {
    /// **provider 境界の唯一の seam**: Rig-aware な provider crate が
    /// 構築した `DynModel` を包む。`cheaper-runtime` 等の adapter 利用側は
    /// この関数を呼ばない(provider が返した値をそのまま受け渡す)。
    ///
    /// Rig 型を含む唯一の公開 signature — 「crate 外向け signature に Rig
    /// 型を出さない」は provider→adapter 境界の provider 側を指し、ここは
    /// その provider 側が使う入口として明示的に残す。
    pub fn from_rig(model: impl Into<DynModel<Completion>>) -> Self {
        Self {
            inner: model.into(),
            max_tokens: None,
        }
    }

    /// catalog の `model.maxTokens` を設定(provider 側が付ける)。
    pub fn with_max_tokens(mut self, max_tokens: Option<u64>) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// Worker session の構成。`runtime-pi` の `WorkerSessionOptions` 相当。
pub struct WorkerSessionSpec {
    /// provider が解決した completion model(catalog `maxTokens` 付き)。
    pub model: CompletionModel,
    /// `buildSystemPrompt` 済みの system prompt(完全文字列)。
    pub system_prompt: String,
    /// `cheaper-tools` の worker scope(`create_worker_scope` 済み)。
    /// `None` なら tool 無し session(repair session 相当)。
    pub tool_runtime: Option<ToolRuntime>,
    /// `policy.output.maxTokens` — `model.maxTokens` と min を取って
    /// per-request の `max_tokens` へ注入(session.ts の streamFn wrap 相当)。
    pub max_output_tokens: u64,
    /// Rig run の総 model-call budget(内部 guard)。既定
    /// [`DEFAULT_MAX_TURNS`]。
    pub max_turns: usize,
    /// invalid tool call の retry budget。既定
    /// [`DEFAULT_INVALID_TOOL_RETRIES`]。
    pub invalid_tool_retries: usize,
    /// completion-call 単位の transient retry 回数
    /// (`policy.retry.transientAttempts`)。0 で無効。
    pub transient_retries: usize,
}

/// Rig `Agent` + accumulator の組。`stream_run` が唯一の実行面
/// (non-streaming API は expose しない)。
pub struct RigSession {
    agent: rig::Agent,
    acc: Arc<Accumulator>,
    details: Arc<ToolDetails>,
    max_turns: usize,
    invalid_tool_retries: usize,
    /// `min(policy.output.maxTokens, model.maxTokens)`。
    max_tokens: Option<u64>,
}

impl RigSession {
    /// `createWorkerSession` 相当: model+tools+system prompt から agent を
    /// 構築し、accounting hook・transient retry handler・tool scope を
    /// 配線する。
    pub fn build(spec: WorkerSessionSpec) -> Self {
        let mut acc_inner = Accumulator::default();
        // tool 計数は cheaper-tools の `ToolBudget` が admission 時点で
        // 計上するもの(TS `sandboxed.budget` parity) — snapshot で live
        // 値を読むため attach する。tool 無し session では 0 のまま。
        if let Some(runtime) = &spec.tool_runtime {
            acc_inner.attach_budget(runtime.budget());
        }
        let acc = Arc::new(acc_inner);
        let details = Arc::new(ToolDetails::default());
        let max_tokens = match spec.model.max_tokens {
            Some(model_max) => Some(spec.max_output_tokens.min(model_max)),
            None => Some(spec.max_output_tokens),
        };

        // transient retry handler を `model_route_handler` で登録し、
        // `set_model_label` で default に選ぶ。`named_model` の素の
        // `ModelAdapter` registration は使われないが、key 置換の暗黙
        // 動作より明示的な route 選択を優先する。
        let retry_policy = RetryPolicy::with_attempts(spec.transient_retries);
        let builder = AgentBuilder::named_model("default", spec.model.inner.clone())
            .preamble(spec.system_prompt)
            .default_max_turns(spec.max_turns)
            .model_route_handler(
                RETRY_MODEL_LABEL,
                RetryingCompletion::new(
                    RETRY_MODEL_LABEL,
                    spec.model.inner,
                    retry_policy,
                    Arc::clone(&acc),
                ),
            )
            .add_hook(AccountingHook {
                acc: Arc::clone(&acc),
                details: Arc::clone(&details),
                rolled_back_turn: std::sync::Mutex::new(None),
            });
        let mut agent = match spec.tool_runtime {
            Some(runtime) => builder
                .dynamic_tools(crate::tools::rig_tools(runtime))
                .build(),
            None => builder.build(),
        };
        agent.set_model_label(RETRY_MODEL_LABEL);
        Self {
            agent,
            acc,
            details,
            max_turns: spec.max_turns,
            invalid_tool_retries: spec.invalid_tool_retries,
            max_tokens,
        }
    }

    /// Accumulator の生 snapshot(success/error/abort どの経路でも有効)。
    pub fn snapshot(&self) -> RunSnapshot {
        self.acc.snapshot()
    }

    /// Run-scoped accumulator handle(T3 が repair/transient 計上や
    /// maxTurns 判定に使う)。
    pub fn accumulator(&self) -> Arc<Accumulator> {
        Arc::clone(&self.acc)
    }

    /// 唯一の公開実行面 — streaming run。`signal` が abort すると
    /// 残りの Rig stream を drop して終端 error を 1 件 yield する。
    /// `signal` は各 tool dispatch の `ToolContext` scope にも載るので、
    /// cheaper-tools scope 側の signal と `execute` 内で合成される。
    pub fn stream_run(&self, prompt: impl Into<String>, signal: Option<AbortSignal>) -> RunStream {
        let mut runner = self
            .agent
            .prompt(prompt.into())
            .max_turns(self.max_turns)
            .max_invalid_tool_call_retries(self.invalid_tool_retries)
            .tool_concurrency(1);
        if let Some(max_tokens) = self.max_tokens {
            runner = runner.max_tokens(max_tokens);
        }
        if let Some(signal) = &signal {
            runner = runner.tool_context(ToolContext::new().with_scope(Arc::new(signal.clone())));
        }
        crate::run_stream::run_stream(
            runner.stream(),
            signal,
            Arc::clone(&self.details),
            Arc::clone(&self.acc),
        )
    }
}

/// `buildSystemPrompt` — run.ts の output contract・batching guidance を
/// 文字列完全再現する(行の並び・文言を TS と一致させる)。
///
/// `instructions` は worker.md body、`output_schema` は `worker.outputSchema
/// .schema`、`tool_names` は sandbox 済みの canonical tool 名。
/// TS: `worker.instructions.trimEnd()` + contract lines + `JSON.stringify(
/// schema, null, 2)`。
pub fn build_system_prompt(
    instructions: &str,
    output_schema: &serde_json::Value,
    tool_names: &[String],
) -> String {
    let mut contract: Vec<String> = vec![
        String::new(),
        "## Output contract".to_string(),
        String::new(),
        "- The final message must be a single JSON object that validates against the output schema below."
            .to_string(),
        "- Do not wrap the JSON in markdown code fences or any surrounding prose.".to_string(),
        "- All file paths are workspace-relative POSIX paths.".to_string(),
        format!(
            "- Available tools: {}",
            if tool_names.is_empty() {
                "none".to_string()
            } else {
                tool_names.join(", ")
            }
        ),
    ];
    // same-turn batching ガイダンス(issue #55 / DESIGN-034zAKcWuB8SuFkOePsubg)。
    // tool 0 件の "none" と同居しないよう非空時のみ emit する(TS parity)。
    if !tool_names.is_empty() {
        contract.extend([
            "- Independent tool calls must be issued together in the same assistant turn; do not serialize independent reads, searches, or token counting across multiple turns.".to_string(),
            "- Do not poll or re-ask whether a call finished: the runtime owns timeouts and process lifecycle, and every tool result is returned automatically as input for the next turn.".to_string(),
            "- Do not batch unsafe mutations: mutation calls that depend on each other or apply multiple mutations to the same target, and reads that expect the result of a sibling write or edit, are not independent — issue them in separate turns.".to_string(),
            "- Keep each same-turn batch and its per-call arguments small enough to fit within the output token limit applied to each response; an oversized response is truncated and every tool call in it fails unexecuted.".to_string(),
            "- This batching guidance does not change the final-message contract: the last message is still a single JSON object that validates against the output schema below.".to_string(),
        ]);
    }
    contract.push(String::new());
    contract.push("Output schema:".to_string());
    // F10: provider-visible text は `JSON.stringify(…, null, 2)` 規則
    // (JS number formatting — `1.0` → `1`、`|x| >= 1e21` 等)に揃える。
    contract.push(stringify_pretty(output_schema));
    format!("{}\n{}\n", instructions.trim_end(), contract.join("\n"))
}

/// `buildUserPrompt` — run.ts の user prompt 組立(`Task input (JSON):`)。
pub fn build_user_prompt(input: &serde_json::Value) -> String {
    format!("Task input (JSON):\n{}", stringify_pretty(input))
}
