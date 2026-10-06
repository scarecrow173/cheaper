use serde::Serialize;
use std::sync::{Mutex, MutexGuard};

use rig::completion::Usage;

/// Run-level counters accumulated exclusively through agent hooks.
/// Mirrors cheaper's hook-driven accounting contract: PromptResponse is never
/// consulted, so aborts, errors, and max-turn exits still produce a snapshot.
#[derive(Default)]
pub struct Accumulator {
    inner: Mutex<Snapshot>,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub usage: UsageSnapshot,
    pub turns: u64,
    pub tool_calls: u64,
    pub rejected_tool_calls: u64,
    pub invalid_tool_calls: u64,
    pub repairs: u64,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub reasoning_tokens: u64,
    pub tool_use_prompt_tokens: u64,
}

impl UsageSnapshot {
    fn add(&mut self, usage: &Usage) {
        self.input_tokens += usage.input_tokens.unwrap_or(0);
        self.output_tokens += usage.output_tokens.unwrap_or(0);
        self.total_tokens += usage.total_tokens.unwrap_or(0);
        self.cached_input_tokens += usage.cached_input_tokens.unwrap_or(0);
        self.cache_creation_input_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
        self.reasoning_tokens += usage.reasoning_tokens.unwrap_or(0);
        self.tool_use_prompt_tokens += usage.tool_use_prompt_tokens.unwrap_or(0);
    }
}

impl Accumulator {
    /// A poisoned lock must not take down the snapshot path: a hook panic is
    /// exactly when the caller needs `snapshot()` for the error envelope.
    fn lock(&self) -> MutexGuard<'_, Snapshot> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// An accepted model turn. Fires on `ModelTurnFinished`, so a turn rolled
    /// back by an invalid-tool-call retry is not counted — matching cheaper's
    /// accepted-turn semantics.
    pub fn record_turn(&self) {
        self.lock().turns += 1;
    }

    /// Usage of a billed completion call, recorded on the completion
    /// outcome.
    pub fn record_completion(&self, usage: &Usage) {
        self.lock().usage.add(usage);
    }

    /// Reconcile recorded usage with the run's billed total at settle time.
    /// A turn rolled back by an invalid-tool-call retry fires neither
    /// `on_outcome` nor `on_model_turn_finished` (rig-agent engine), so its
    /// usage is visible only through `PromptResponse::usage` on
    /// `on_run_settled`. The billed total must cover the recorded per-call
    /// sum; `max` keeps the accumulator correct even if both paths overlap.
    pub fn reconcile_usage(&self, billed: &Usage) {
        let mut s = self.lock();
        let u = &mut s.usage;
        u.input_tokens = u.input_tokens.max(billed.input_tokens.unwrap_or(0));
        u.output_tokens = u.output_tokens.max(billed.output_tokens.unwrap_or(0));
        u.total_tokens = u.total_tokens.max(billed.total_tokens.unwrap_or(0));
        u.cached_input_tokens = u
            .cached_input_tokens
            .max(billed.cached_input_tokens.unwrap_or(0));
        u.cache_creation_input_tokens = u
            .cache_creation_input_tokens
            .max(billed.cache_creation_input_tokens.unwrap_or(0));
        u.reasoning_tokens = u.reasoning_tokens.max(billed.reasoning_tokens.unwrap_or(0));
        u.tool_use_prompt_tokens = u
            .tool_use_prompt_tokens
            .max(billed.tool_use_prompt_tokens.unwrap_or(0));
    }

    /// A tool call that actually ran (Ok outcome and not a hook-skipped
    /// result).
    pub fn record_tool_executed(&self) {
        self.lock().tool_calls += 1;
    }

    /// A tool call rejected at the boundary: denied dispatch (delivered as
    /// `ToolResult::skipped`) or a failed dispatch — it never ran.
    pub fn record_tool_rejected(&self) {
        self.lock().rejected_tool_calls += 1;
    }

    /// An invalid tool call observed. Counts the event, not a completed
    /// retry: the run-level budget may still reject it afterwards.
    pub fn record_invalid_tool_call(&self) {
        self.lock().invalid_tool_calls += 1;
    }

    pub fn record_repair(&self) {
        self.lock().repairs += 1;
    }

    /// Terminal snapshot for the envelope's meta, valid on success and error.
    pub fn snapshot(&self) -> Snapshot {
        self.lock().clone()
    }
}
