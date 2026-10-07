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

/// Sequential accumulator hook: accepted-turn counts from
/// `ModelTurnFinished`, billed usage from completion outcomes reconciled
/// against the run total at `on_run_settled` (a turn rolled back by an
/// invalid-call retry fires neither outcome nor turn hook, so only the
/// settled `PromptResponse` carries its billed usage), and invalid-call
/// resolution that returns corrective feedback (`Retry`) without touching
/// the tool-call counters — the contract cheaper's envelope requires.
/// `tool_calls`/`rejected_tool_calls` は `Accumulator::attach_budget`
/// 経由で cheaper-tools の `ToolBudget` live view から読む。
pub(crate) struct AccountingHook {
    pub(crate) acc: Arc<Accumulator>,
    pub(crate) details: Arc<ToolDetails>,
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
        _ctx: &HookContext,
        event: &InvalidToolCallContext,
    ) -> Option<InvalidToolCallAction> {
        self.acc.record_invalid_tool_call();
        Some(InvalidToolCallAction::retry(format!(
            "Tool `{}` is not available ({:?}). Available tools: {:?}. Do not call it again.",
            event.tool_name, event.reason, event.available_tools
        )))
    }

    async fn on_outcome(&self, _ctx: &HookContext, event: OutcomeEvent<'_>) -> OutcomeAction {
        match event.kind {
            // Every billed completion resolves here, including attempts the
            // run later rolls back — `ModelTurnFinished` would miss those.
            EffectKind::Completion { .. } => {
                if let Some(completion) = event.completion() {
                    self.acc
                        .record_completion(&crate::accumulator::UsageSnapshot::from(
                            &completion.usage,
                        ));
                }
            }
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
