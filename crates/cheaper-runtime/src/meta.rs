//! `WorkerRunMeta` 構築(run.ts `buildMeta` 相当)。
//!
//! usage の正源は adapter `Accumulator` の `RunSnapshot` — abort /
//! provider error / max-turns 等で `PromptResponse` が得られない経路でも
//! billed usage が残る。repair session の snapshot は run 単位に畳み込む
//! (`acc` が跨 session で共有される TS 相当)。
//!
//! `costUsd` は cheaper 価格(`resolved.pricing` — runtime catalog →
//! pricing snapshot → bundled snapshot の順に解決済み)× token 数から算出
//! し、provider 申告値は使わない。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::classify::TierSource;
use cheaper_core::model::pricing::ModelPricing;
use cheaper_core::model::resolve::{ModelSelection, ResolvedModel};
use cheaper_core::model::tier::Tier;
use cheaper_core::token::count::count_text_tokens;
use cheaper_core::worker::validate::{is_directly_comparable_worker, WorkerDefinition};
use cheaper_rig_adapter::RunSnapshot;
use cheaper_tools::execution_metrics::ExecutionMetrics;
use cheaper_tools::journal::ChangesSummary;
use serde::Serialize;
use serde_json::{Map, Value};

/// `meta.model`。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMeta {
    pub provider: String,
    pub id: String,
    pub tier: Tier,
    pub selection: ModelSelection,
    pub tier_source: TierSource,
    /// 解決された effective cost(tier 推定の根拠)。`None` は
    /// unpriced(working set のみ)。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_cost_percentile: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workload: Option<WorkloadMeta>,
}

/// `meta.model.workload`(`{outputPerInput}`)。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadMeta {
    pub output_per_input: f64,
}

/// `meta.usage`。token は accumulator 計上値、`costUsd` は cheaper 価格由来。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total_tokens: u64,
    pub cost_usd: f64,
}

/// `meta.savings` — 親コンテキストからの offload 実測(usage.ts
/// `WorkerSavings`)。
/// - `source_tokens`: 全 tool output text の canonical token 和
/// - `returned_tokens`: 成功時 `JSON.stringify(result)` の token 数、
///   失敗は 0
/// - `avoided_context_tokens`: `max(0, sourceTokens - returnedTokens)`
/// - `directly_comparable`: read-only worker のみ true(core
///   `isDirectlyComparableWorker`)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSavings {
    pub source_tokens: usize,
    pub returned_tokens: u64,
    pub avoided_context_tokens: u64,
    pub directly_comparable: bool,
}

/// `meta` 全体(run.ts `WorkerRunMeta`)。success でも usage-bearing
/// failure でも同一 shape。`durationMs` は CLI 層が付ける optional。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerRunMeta {
    pub model: ModelMeta,
    pub usage: WorkerUsage,
    pub turns: u64,
    pub tool_calls: u64,
    pub rejected_tool_calls: u64,
    pub repairs: u64,
    pub transient_retries: u64,
    pub changes: ChangesSummary,
    pub execution: ExecutionMetrics,
    pub savings: WorkerSavings,
    #[serde(rename = "durationMs", skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
}

/// `buildMeta` の入力。`snapshot` は主 session + repair session を
/// `merge_snapshots` で畳み込んだ run 累計を渡す。
pub struct MetaInputs<'a> {
    pub resolved: &'a ResolvedModel,
    pub worker: &'a WorkerDefinition,
    /// `toolScope.meter().sourceTokens`。tool-less(repair)経路では 0。
    pub source_tokens: usize,
    /// `journal.finalize()` 結果。
    pub changes: ChangesSummary,
    /// `invocation.executionMetrics(ownerId)`。
    pub execution: ExecutionMetrics,
    /// repair attempt 消費数(TS `repairs` — onAttempt で記録)。
    pub repairs: u64,
    /// run 累計 accumulator snapshot。
    pub snapshot: RunSnapshot,
}

/// `buildMeta(result?)` — `result` 無しは failure path。
pub fn build_meta(
    inputs: &MetaInputs<'_>,
    result: Option<&Map<String, Value>>,
) -> Result<WorkerRunMeta, CheaperError> {
    let snap = &inputs.snapshot;
    let resolved = inputs.resolved;
    let worker = inputs.worker;
    let source_tokens = inputs.source_tokens;
    let returned_tokens = result
        .map(|r| {
            count_text_tokens(&serde_json::to_string(r).unwrap_or_default())
                .map(|n| n as u64)
                // `countTextTokens` の throw は TS では buildMeta → outer
                // catch で INTERNAL_ERROR になる。ここでも同じ code に畳む。
                .map_err(|e| {
                    CheaperError::new(
                        ErrorCode::InternalError,
                        format!("token counting failed: {e}"),
                    )
                    .with_cause(e)
                })
        })
        .transpose()?
        .unwrap_or(0);
    let directly_comparable = is_directly_comparable_worker(worker);
    Ok(WorkerRunMeta {
        model: ModelMeta {
            provider: resolved.model.provider.clone(),
            id: resolved.model.id.clone(),
            tier: resolved.tier,
            selection: resolved.selection,
            tier_source: resolved.tier_source,
            // `resolved.auto` は tierSource === "auto" のときのみ存在。
            effective_cost: resolved.auto.as_ref().map(|a| a.effective_cost),
            effective_cost_percentile: resolved.auto.as_ref().map(|a| a.effective_cost_percentile),
            workload: resolved.auto.as_ref().map(|a| WorkloadMeta {
                output_per_input: a.output_per_input,
            }),
        },
        usage: WorkerUsage {
            input: snap.usage.input_tokens,
            output: snap.usage.output_tokens,
            cache_read: snap.usage.cached_input_tokens,
            cache_write: snap.usage.cache_creation_input_tokens,
            total_tokens: snap.usage.total_tokens,
            cost_usd: usage_cost_usd(resolved.pricing.as_ref(), snap),
        },
        turns: snap.turns,
        tool_calls: snap.tool_calls,
        rejected_tool_calls: snap.rejected_tool_calls,
        repairs: inputs.repairs,
        transient_retries: snap.transient_retries,
        changes: inputs.changes.clone(),
        execution: inputs.execution.clone(),
        savings: WorkerSavings {
            source_tokens,
            returned_tokens,
            avoided_context_tokens: (source_tokens as u64).saturating_sub(returned_tokens),
            directly_comparable,
        },
        duration_ms: None,
    })
}

/// cheaper 価格 × token で costUsd を算出(pi Usage.cost.total 式相当:
/// per-1M rate × token / 1e6)。価格不明は 0.0。
fn usage_cost_usd(pricing: Option<&ModelPricing>, snap: &RunSnapshot) -> f64 {
    let Some(p) = pricing else { return 0.0 };
    (p.input * snap.usage.input_tokens as f64
        + p.output * snap.usage.output_tokens as f64
        + p.cache_read.unwrap_or(0.0) * snap.usage.cached_input_tokens as f64
        + p.cache_write.unwrap_or(0.0) * snap.usage.cache_creation_input_tokens as f64)
        / 1_000_000.0
}

/// 主 session + repair session の snapshot を run 単位に畳む。
/// `last_turn_tool_calls` は maxTurns 判定専用のため main 側を保持する
/// (repair session は tool-less で常に false)。
pub fn merge_snapshots(base: &mut RunSnapshot, extra: &RunSnapshot) {
    base.usage.input_tokens += extra.usage.input_tokens;
    base.usage.output_tokens += extra.usage.output_tokens;
    base.usage.cache_creation_input_tokens += extra.usage.cache_creation_input_tokens;
    base.usage.cached_input_tokens += extra.usage.cached_input_tokens;
    base.usage.total_tokens += extra.usage.total_tokens;
    base.turns += extra.turns;
    base.tool_calls += extra.tool_calls;
    base.rejected_tool_calls += extra.rejected_tool_calls;
    base.invalid_tool_calls += extra.invalid_tool_calls;
    base.repairs += extra.repairs;
    base.transient_retries += extra.transient_retries;
}
