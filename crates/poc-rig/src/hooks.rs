use std::sync::Arc;

use rig::agent::{
    AgentHook, DispatchAction, DispatchEvent, HookContext, InvalidToolCallAction,
    InvalidToolCallContext, ModelTurnAction, ModelTurnFinished, OutcomeAction, OutcomeEvent,
    RunSettled, SettledOutcome,
};
use rig::core::effect::EffectKind;

use crate::usage::Accumulator;

/// Sequential accumulator hook: accepted-turn counts from
/// `ModelTurnFinished`, billed usage from completion outcomes reconciled
/// against the run total at `on_run_settled` (a turn rolled back by an
/// invalid-call retry fires neither outcome nor turn hook, so only the
/// settled `PromptResponse` carries its billed usage), executed/rejected
/// tool counts from tool-dispatch outcomes, and invalid-call resolution that
/// returns corrective feedback (`Retry`) without touching the tool-call
/// counters — the contract cheaper's envelope requires.
pub struct AccumulatorHook {
    pub acc: Arc<Accumulator>,
}

impl AgentHook for AccumulatorHook {
    async fn on_run_settled(&self, _ctx: &HookContext, event: RunSettled<'_>) {
        if let SettledOutcome::Response(response) = event.outcome {
            self.acc.reconcile_usage(&response.usage);
        }
    }

    async fn on_model_turn_finished(
        &self,
        _ctx: &HookContext,
        _event: ModelTurnFinished<'_>,
    ) -> ModelTurnAction {
        self.acc.record_turn();
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
                    self.acc.record_completion(&completion.usage);
                }
            }
            EffectKind::ToolCall { .. } => match event.tool_result() {
                // `Ok` is not proof of execution: a denied dispatch resolves
                // to `ToolResult::skipped` and must count as rejected.
                Some(result) if result.is_skipped() => self.acc.record_tool_rejected(),
                Some(_) => self.acc.record_tool_executed(),
                // A dispatch that failed before producing a result never ran.
                None => self.acc.record_tool_rejected(),
            },
            _ => {}
        }
        OutcomeAction::proceed()
    }

    /// Denials are accounted for in `on_outcome`: a deny resolves as a
    /// skipped `ToolResult` (or a failed outcome for bus-layer denials), so
    /// counting happens where the resolution is visible.
    async fn on_dispatch(
        &self,
        _ctx: &HookContext,
        _event: DispatchEvent<'_>,
    ) -> DispatchAction {
        DispatchAction::Proceed
    }
}
