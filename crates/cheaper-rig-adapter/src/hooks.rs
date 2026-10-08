//! Agent hook wiring: sequential accumulation into [`Accumulator`] and
//! invalid-tool-call recovery feedback (`InvalidToolCallAction::Retry`).
//!
//! Phase 0 PoC (`crates/poc-rig/src/hooks.rs`) の port + run.ts の
//! maxTurns 判定材料(最終 turn が tool 継続で終わったか)の記録。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rig::agent::{
    AgentHook, DispatchAction, DispatchEvent, HookContext, InvalidToolCallAction,
    InvalidToolCallContext, ModelTurnAction, ModelTurnFinished, OutcomeAction, OutcomeEvent,
    RunSettled, SettledOutcome,
};
use rig::completion::request::FinishReason;
use rig::core::effect::EffectKind;

use crate::accumulator::Accumulator;
use crate::tools::ToolCallDetails;

/// tool outcome が公開した `details`(CanonicalToolResult.details)を
/// call id で引く side-channel。streaming item 自体は details を運ばない
/// ため、`on_outcome`(dispatch 解決後・stream item 送出前に発火)で回収し、
/// `run_stream` が対応する `ToolResult` event に添付する。
/// `tool_concurrency(1)` 固定なので逐次実行では race しない。
#[derive(Default)]
pub(crate) struct ToolDetails {
    inner: Mutex<HashMap<String, serde_json::Value>>,
}

impl ToolDetails {
    fn record(&self, call_id: &str, details: serde_json::Value) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(call_id.to_owned(), details);
    }

    pub(crate) fn take(&self, call_id: &str) -> Option<serde_json::Value> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(call_id)
    }
}

/// Sequential accumulator hook: completed model calls counted at
/// `ModelTurnFinished` (accepted) plus `on_invalid_tool_call` (rolled
/// back by recovery — F6b, once per turn via `HookContext::turn` dedup),
/// billed usage recorded by `run_stream` at each `CompletionCall` item
/// (Rig emits it even for turns rolled back by an invalid-call retry)
/// and reconciled against the run total at `on_run_settled` via
/// `PromptResponse::usage`, and invalid-call resolution that returns
/// corrective feedback (`Retry`) without touching the tool-call
/// counters — the contract cheaper's envelope requires.
/// `tool_calls`/`rejected_tool_calls` は `Accumulator::attach_budget`
/// 経由で cheaper-tools の `ToolBudget` live view から読む。
pub(crate) struct AccountingHook {
    pub(crate) acc: Arc<Accumulator>,
    pub(crate) details: Arc<ToolDetails>,
    /// F6b: rollback 計上済みの最後の model-call index
    /// (`HookContext::turn`)。invalid call は turn 内で複数観測され
    /// 得るが rollback 自体は turn 単位で一度だけ起きるため dedup する。
    pub(crate) rolled_back_turn: Mutex<Option<usize>>,
}

impl AgentHook for AccountingHook {
    async fn on_run_settled(&self, _ctx: &HookContext, event: RunSettled<'_>) {
        if let SettledOutcome::Response(response) = event.outcome {
            self.acc
                .reconcile_usage(&crate::accumulator::UsageSnapshot::from(&response.usage));
        }
    }

    async fn on_model_turn_finished(
        &self,
        _ctx: &HookContext,
        event: ModelTurnFinished<'_>,
    ) -> ModelTurnAction {
        // `Stop` is already reconciled to `ToolCalls` when calls are present;
        // fall back to the parked content so an unreported finish reason still
        // feeds run.ts の `stopReason === "toolUse"` 相当判定。
        let tool_calls = matches!(event.finish_reason, Some(FinishReason::ToolCalls))
            || event.content.iter().any(|item| {
                matches!(
                    item,
                    rig::completion::message::AssistantContent::ToolCall(_)
                )
            });
        self.acc.record_turn(tool_calls);
        ModelTurnAction::continue_run()
    }

    async fn on_invalid_tool_call(
        &self,
        ctx: &HookContext,
        event: &InvalidToolCallContext,
    ) -> Option<InvalidToolCallAction> {
        self.acc.record_invalid_tool_call();
        // F6b: この hook は常に `Retry` を返す — invalid call を含む
        // turn は rollback(または retry budget 枯渇時の run 終端)で
        // 閉じる。Pi は error tool result で turn を完走させ
        // `turn_end` で計上するため、rolled-back turn も完了 model
        // call として `turns` に数える — さもないと invalid-call
        // ループが `policy.execution.maxTurns` を回避できる。
        // streamed/buffered 両解決経路がこの hook に集約されるので
        // item 経路での再計上は不要。
        let turn = ctx.turn();
        let mut last = self
            .rolled_back_turn
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if *last != Some(turn) {
            *last = Some(turn);
            drop(last);
            self.acc.record_rolled_back_turn();
        }
        Some(InvalidToolCallAction::retry(format!(
            "Tool `{}` is not available ({:?}). Available tools: {:?}. Do not call it again.",
            event.tool_name, event.reason, event.available_tools
        )))
    }

    async fn on_outcome(&self, _ctx: &HookContext, event: OutcomeEvent<'_>) -> OutcomeAction {
        match event.kind {
            // usage 記録は `run_stream` の `CompletionCall` item 境界に
            // 一本化済み(F2)。この hook は rolled-back streamed turn に
            // 発火しないが `CompletionCall` item は emit されるため、
            // stream 側が唯一の完全な recording point。
            EffectKind::Completion { .. } => {}
            EffectKind::ToolCall { .. } => {
                // tool_calls/rejected_tool_calls は attach された
                // `ToolBudget` が計上する(admission 時点計上・access-denied
                // 両方計上の TS contract は cheaper-tools 側の所有物) —
                // outcome 観察で数え直すと semantic がずれるのでここでは
                // 計数しない。ここで拾うのは `details` のみ:
                // model 非可視の host metadata — stream item に乗らないので
                // hook 側で拾って `RunEvent::ToolResult` へ渡す。
                if let (Some(context), Some(call_id)) = (event.context, event.call_id) {
                    if let Ok(Some(details)) = context.result::<ToolCallDetails>() {
                        self.details.record(&call_id.wire(), details.0);
                    }
                }
            }
            _ => {}
        }
        OutcomeAction::proceed()
    }

    /// Denials are accounted for in `on_outcome`: a deny resolves as a
    /// skipped `ToolResult` (or a failed outcome for bus-layer denials), so
    /// counting happens where the resolution is visible.
    async fn on_dispatch(&self, _ctx: &HookContext, _event: DispatchEvent<'_>) -> DispatchAction {
        DispatchAction::Proceed
    }
}
