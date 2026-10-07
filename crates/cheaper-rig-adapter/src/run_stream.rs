//! Streaming run API — adapter の唯一の公開実行面。
//!
//! Rig の `MultiTurnStreamItem`/`StreamingError` は内部 detail であり、
//! ここで adapter 固有の `RunEvent`/`CheaperError` へ正規化する
//! (crate 外向け signature に Rig 型を出さない)。
//!
//! cancel 中継: `AbortSignal` を `inner.next()` と select し、abort 時は
//! Rig stream を drop(run 全体の cancel)したうえで reason 由来の終端
//! `CheaperError` を 1 件 emit して stream を閉じる。
//!
//! usage/accounting は stream item ではなく hook→accumulator が担う —
//! ここでは event の整形だけを行う(FinalResponse の usage を読まない)。

use std::pin::Pin;
use std::sync::Arc;

use futures::{stream, Stream, StreamExt};

use cheaper_core::errors::CheaperError;
use cheaper_tools::operation::AbortSignal;

use rig::agent::MultiTurnStreamItem;
use rig::streaming::{Item, StreamEvent, StreamedUserContent};

use crate::accumulator::UsageSnapshot;
use crate::error::{abort_reason_to_cheaper, streaming_error_to_cheaper};
use crate::hooks::ToolDetails;

/// streaming run の公開 event 列。全て adapter 固有型で Rig 型を含まない。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RunEvent {
    /// モデルの本文 delta。
    TextDelta { text: String },
    /// reasoning/thinking delta(thinkingLevel=off でも provider が emit し得る)。
    ReasoningDelta { text: String },
    /// tool call arguments の部分 JSON delta(commit 前)。
    ToolArgumentsDelta { delta: String },
    /// model が emit した tool call(turn commit 時・hook repair 前)。
    ToolCall {
        call_id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Rig が実際に実行 commit した tool call(hook patch 適用後の引数)。
    ToolCommitted {
        call_id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// tool call の結果(`details` は model 非可視 metadata — hook が
    /// `ToolContext` result slot から拾った `CanonicalToolResult.details`)。
    ToolResult {
        call_id: String,
        name: String,
        output: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<serde_json::Value>,
    },
    /// 完了した completion request の usage 報告(per-call、
    /// incremental ではない)。run 集計は accumulator が正。
    CompletionCall { index: usize, usage: UsageSnapshot },
    /// hook が turn を reject して retry した(その turn の delta は破棄
    /// すべき provisional output)。
    ModelTurnRetried { turn: usize },
    /// 終端: 最終 assistant text(FinalResponse.output 相当)。
    Finished { output: String },
}

/// `RunEvent` または終端 `CheaperError` の stream。
/// drop は run の cancel。
pub type RunStream = Pin<Box<dyn Stream<Item = Result<RunEvent, CheaperError>> + Send>>;

fn map_item(item: MultiTurnStreamItem, details: &ToolDetails) -> Option<RunEvent> {
    match item {
        MultiTurnStreamItem::StreamAssistantItem(item) => match item {
            Item::Event(event) => match event {
                StreamEvent::Text { text, .. } => Some(RunEvent::TextDelta { text }),
                StreamEvent::Reasoning { text, .. } => Some(RunEvent::ReasoningDelta { text }),
                StreamEvent::Arguments { json, .. } => {
                    Some(RunEvent::ToolArgumentsDelta { delta: json })
                }
                // part lifecycle(Start/End)は Rig 内部の区切りで、外部
                // contract には Turn/commit 系 event で足りる。
                StreamEvent::Start { .. } | StreamEvent::End { .. } => None,
            },
            // decoder が model 化していない payload — 外部へ漏らさない。
            Item::Unknown(_) => None,
        },
        MultiTurnStreamItem::ToolCall { tool_call } => Some(RunEvent::ToolCall {
            call_id: tool_call.id.wire().into_owned(),
            name: tool_call.function.name.as_str().to_owned(),
            arguments: tool_call.function.arguments,
        }),
        MultiTurnStreamItem::ToolExecutionCommitted { tool_call } => {
            Some(RunEvent::ToolCommitted {
                call_id: tool_call.id.wire().into_owned(),
                name: tool_call.function.name.as_str().to_owned(),
                arguments: tool_call.function.arguments,
            })
        }
        MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult { tool_result }) => {
            let output =
                serde_json::to_value(&tool_result.content).unwrap_or(serde_json::Value::Null);
            let call_id = tool_result.call.wire().into_owned();
            Some(RunEvent::ToolResult {
                details: details.take(&call_id),
                call_id,
                name: tool_result.name.as_str().to_owned(),
                output,
            })
        }
        MultiTurnStreamItem::CompletionCall(call) => Some(RunEvent::CompletionCall {
            index: call.call_index,
            usage: UsageSnapshot::from(&call.usage),
        }),
        MultiTurnStreamItem::ModelTurnRetried { turn } => Some(RunEvent::ModelTurnRetried { turn }),
        MultiTurnStreamItem::FinalResponse(response) => Some(RunEvent::Finished {
            output: response.output,
        }),
    }
}

struct State {
    inner: rig::agent::StreamingResult,
    signal: Option<AbortSignal>,
    details: Arc<ToolDetails>,
    done: bool,
}

/// Rig streaming run を `RunEvent` 列へ正規化する。
/// `signal` が既に abort 済みでも、走り始めてからの abort と同じ経路で
/// 終端 error を emit する(preempt しない)。
pub(crate) fn run_stream(
    inner: rig::agent::StreamingResult,
    signal: Option<AbortSignal>,
    details: Arc<ToolDetails>,
) -> RunStream {
    let stream = stream::unfold(
        State {
            inner,
            signal,
            details,
            done: false,
        },
        |mut state| async move {
            loop {
                if state.done {
                    return None;
                }
                let next = match &state.signal {
                    Some(signal) => {
                        tokio::select! {
                            biased;
                            _ = signal.cancelled() => {
                                state.done = true;
                                let error = match signal.reason() {
                                    Some(reason) => abort_reason_to_cheaper(&reason),
                                    None => abort_reason_to_cheaper(
                                        &cheaper_tools::operation::AbortReason::aborted(),
                                    ),
                                };
                                return Some((Err(error), state));
                            }
                            item = state.inner.next() => item,
                        }
                    }
                    None => state.inner.next().await,
                };
                match next {
                    // stream 終端 — state は破棄されるので done は不要。
                    None => return None,
                    Some(Err(error)) => {
                        state.done = true;
                        return Some((Err(streaming_error_to_cheaper(error)), state));
                    }
                    Some(Ok(item)) => {
                        if let Some(event) = map_item(item, &state.details) {
                            return Some((Ok(event), state));
                        }
                        // map 対象外の内部 item は skip して次を poll。
                    }
                }
            }
        },
    );
    Box::pin(stream)
}
