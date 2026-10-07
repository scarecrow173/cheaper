//! Run-level sequential accumulator — the **sole** usage collection source.
//!
//! `packages/runtime-pi/src/usage.ts` の `UsageCollector` 相当を Phase 0
//! PoC (`crates/poc-rig/src/usage.rs`) の hook 駆動集計として再実装する。
//! `PromptResponse` は error/cancel/max-turns 経路では取得できないため、
//! usage は hook イベントからのみ蓄積する。`costUsd` は provider 応答値を
//! 使わず token 数だけを保持し、pricing は runtime 層が算出する(T3 meta)。

use std::sync::{Mutex, MutexGuard};

use serde::Serialize;

use cheaper_tools::accounting::ToolBudget;
use rig::completion::Usage;

/// `Snapshot` — run の envelope meta が読む集計値。成功・error・abort・
/// max-turns のどの終端でも有効。
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSnapshot {
    /// token のみ。`costUsd` は pricing 層の責務。
    pub usage: UsageSnapshot,
    /// 受理された model turn 数(`ModelTurnFinished` 発火数)。
    /// invalid-tool retry で rollback された turn は含まない。
    pub turns: u64,
    /// `budget.toolCalls` — runtime の admission gate を通った call 数
    /// (access-denied error は rejected 側と重複計上される TS parity)。
    /// [`Accumulator::attach_budget`] 済みなら snapshot 時に budget 値を読む。
    pub tool_calls: u64,
    /// `budget.rejectedToolCalls` — maxToolCalls 上限拒否と
    /// access-denied error の合計(TS `accounting.ts` parity)。
    pub rejected_tool_calls: u64,
    /// `InvalidToolCallAction::Retry` が観測した invalid tool call 数
    /// (イベントの観測数 — retry 完了数ではない)。
    pub invalid_tool_calls: u64,
    /// repair session 起動回数(T3 が `record_repair` で加算する口)。
    pub repairs: u64,
    /// completion-call 単位 transient retry の発火回数(Pi `auto_retry_start`
    /// 相当 — cheaper 独自 retry ではなく provider 障害 retry の計上)。
    pub transient_retries: u64,
    /// 最後に受理された model turn が tool call で終わったか
    /// (`stopReason === "toolUse"` 相当 — run.ts の maxTurns 判定材料)。
    pub last_turn_tool_calls: bool,
}

/// `Usage` の token 項目の adapter 側表現(provider 未報告項目は 0 に正規化)。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
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
    fn add(&mut self, other: &UsageSnapshot) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
        self.cached_input_tokens += other.cached_input_tokens;
        self.cache_creation_input_tokens += other.cache_creation_input_tokens;
        self.reasoning_tokens += other.reasoning_tokens;
        self.tool_use_prompt_tokens += other.tool_use_prompt_tokens;
    }
}

/// Rig `Usage` → adapter snapshot の変換 seam(crate 内のみ — 公開 API は
/// `UsageSnapshot` で受ける)。
impl From<&Usage> for UsageSnapshot {
    fn from(usage: &Usage) -> Self {
        UsageSnapshot {
            input_tokens: usage.input_tokens.unwrap_or(0),
            output_tokens: usage.output_tokens.unwrap_or(0),
            total_tokens: usage.total_tokens.unwrap_or(0),
            cached_input_tokens: usage.cached_input_tokens.unwrap_or(0),
            cache_creation_input_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
            reasoning_tokens: usage.reasoning_tokens.unwrap_or(0),
            tool_use_prompt_tokens: usage.tool_use_prompt_tokens.unwrap_or(0),
        }
    }
}

/// Hook 経由のみで更新される集計器。`snapshot()` は run 途中でも呼べる
/// (T3 が maxTurns/maxToolCalls 判定に使う)。
///
/// `tool_calls`/`rejected_tool_calls` だけは hook ではなく attach された
/// [`ToolBudget`] の live 値を返す — TS の admission 時点計上
/// (`toolCalls++` は execute 前)・access-denied の両方計上・上限拒否の
/// semantic を cheaper-tools が所有するため、hook の outcome 観察で
/// 二次推定すると contract がずれる。
#[derive(Default)]
pub struct Accumulator {
    inner: Mutex<RunSnapshot>,
    budget: Option<ToolBudget>,
}

impl Accumulator {
    /// A poisoned lock must not take down the snapshot path: a hook panic is
    /// exactly when the caller needs `snapshot()` for the error envelope.
    fn lock(&self) -> MutexGuard<'_, RunSnapshot> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// worker scope の tool budget を接続する。以降 `snapshot()` の
    /// `tool_calls`/`rejected_tool_calls` はこの live view を読む。
    pub fn attach_budget(&mut self, budget: ToolBudget) {
        self.budget = Some(budget);
    }

    /// An accepted model turn. Fires on `ModelTurnFinished`, so a turn rolled
    /// back by an invalid-tool-call retry is not counted — matching cheaper's
    /// accepted-turn semantics. `tool_calls` はその turn が tool 継続で
    /// 終わったか(`FinishReason::ToolCalls` または content 内の call 有無)。
    pub fn record_turn(&self, tool_calls: bool) {
        let mut s = self.lock();
        s.turns += 1;
        s.last_turn_tool_calls = tool_calls;
    }

    /// Usage of a billed completion call, recorded on the completion
    /// outcome.
    pub fn record_completion(&self, usage: &UsageSnapshot) {
        self.lock().usage.add(usage);
    }

    /// Reconcile recorded usage with the run's billed total at settle time.
    /// A turn rolled back by an invalid-tool-call retry fires neither
    /// `on_outcome` nor `on_model_turn_finished` (rig-agent engine), so its
    /// usage is visible only through `PromptResponse::usage` on
    /// `on_run_settled`. The billed total must cover the recorded per-call
    /// sum; `max` keeps the accumulator correct even if both paths overlap.
    pub fn reconcile_usage(&self, billed: &UsageSnapshot) {
        let mut s = self.lock();
        let u = &mut s.usage;
        u.input_tokens = u.input_tokens.max(billed.input_tokens);
        u.output_tokens = u.output_tokens.max(billed.output_tokens);
        u.total_tokens = u.total_tokens.max(billed.total_tokens);
        u.cached_input_tokens = u.cached_input_tokens.max(billed.cached_input_tokens);
        u.cache_creation_input_tokens = u
            .cache_creation_input_tokens
            .max(billed.cache_creation_input_tokens);
        u.reasoning_tokens = u.reasoning_tokens.max(billed.reasoning_tokens);
        u.tool_use_prompt_tokens = u.tool_use_prompt_tokens.max(billed.tool_use_prompt_tokens);
    }

    /// An invalid tool call observed. Counts the event, not a completed
    /// retry: the run-level budget may still reject it afterwards.
    pub fn record_invalid_tool_call(&self) {
        self.lock().invalid_tool_calls += 1;
    }

    /// A repair session start. T3 の repair loop が呼ぶ(T1 では発火しない)。
    pub fn record_repair(&self) {
        self.lock().repairs += 1;
    }

    /// One transient retry of a single completion call (the retry wrapper's
    /// accounting — `transientRetries` in the run meta).
    pub fn record_transient_retry(&self) {
        self.lock().transient_retries += 1;
    }

    /// Terminal or in-flight snapshot for the envelope's meta. Tool 計数は
    /// attach 済み budget があれば live 値で上書きする(`AttachBudget` 前や
    /// tool 無し session では内部カウンタ=0)。
    pub fn snapshot(&self) -> RunSnapshot {
        let mut snapshot = self.lock().clone();
        if let Some(budget) = &self.budget {
            snapshot.tool_calls = budget.tool_calls();
            snapshot.rejected_tool_calls = budget.rejected_tool_calls();
        }
        snapshot
    }
}
