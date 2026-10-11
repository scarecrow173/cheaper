//! `promptWithAbortGrace`(packages/runtime-pi/src/prompt-grace.ts)の port。
//!
//! abort 検知時に `session.abort()` を呼び、settle を `graceMs`(default
//! 5000ms)まで待つ。grace を過ぎても settle しない stream(provider が
//! cancellation を無視)は待ちを打ち切り `Grace` を返す — 分類は記録済み
//! abort reason 側で行う(TS と同じ責務分担)。

use std::sync::Arc;
use std::time::Duration;

use cheaper_core::errors::CheaperError;
use cheaper_rig_adapter::RunEvent;
use cheaper_tools::operation::AbortSignal;
use futures::StreamExt;
use tokio::time::sleep;

use super::WorkerSession;

/// `promptWithAbortGrace` の outcome。
#[derive(Debug)]
pub enum PromptOutcome {
    /// stream が終了。`output` は最後の `Finished` event の連結 text
    /// (assistant message 無しなら `None`)。
    Settled { output: Option<String> },
    /// 終端 error item(adapter が abort/provider error を分類済み
    /// `CheaperError` で返す)。
    Rejected(CheaperError),
    /// abort 後 grace を過ぎても settle しなかった。
    Grace,
}

/// `promptWithAbortGrace(session, prompt, {signal, graceMs})` 相当。
///
/// `signal` は run 集約 abort(`RunAbort::signal`)。abort で
/// `session.abort()` を呼び grace timer を arm する。`on_event` は
/// item 毎に呼ばれる hook — maxTurns 判定など stream 消費側の処理用
/// (TS の per-item `onItem` 相当)。
pub async fn prompt_with_abort_grace(
    session: &Arc<dyn WorkerSession>,
    prompt: String,
    signal: &AbortSignal,
    grace_ms: f64,
    on_event: &mut dyn FnMut(&RunEvent),
) -> PromptOutcome {
    let grace_duration = Duration::from_secs_f64((grace_ms / 1000.0).max(0.0));
    let mut stream = session.stream_run(prompt, Some(signal.clone()));

    // `session.prompt()` 相当 — stream item を逐次処理し、終端 error /
    // 終了で resolve。abort 済なら以後の item は見ない(TS では stream
    // drop が cancel 相当)。
    let drive = async move {
        let mut last_output: Option<String> = None;
        while let Some(item) = stream.next().await {
            match item {
                Ok(event) => {
                    if let RunEvent::Finished { output } = &event {
                        last_output = Some(output.clone());
                    }
                    on_event(&event);
                    if signal.is_aborted() {
                        break;
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(last_output)
    };
    tokio::pin!(drive);

    // Phase 1: drive vs 初回 abort 検知。TS の onAbort listener +
    // graceTimer 起動に相当 — abort で `session.abort()` を呼び
    // grace 計測へ移る。既に abort 済なら即 grace phase。
    if !signal.is_aborted() {
        tokio::select! {
            out = &mut drive => return outcome_of(out),
            () = signal.cancelled() => {}
        }
    }
    session.abort();

    // Phase 2: settle を最大 graceMs 待つ。expiry で `Grace` —
    // drive(stream)は drop され cancel される(TS `finally` の
    // unsubscribe/dispose 相当)。
    tokio::select! {
        out = &mut drive => outcome_of(out),
        _ = sleep(grace_duration) => PromptOutcome::Grace,
    }
}

fn outcome_of(out: Result<Option<String>, CheaperError>) -> PromptOutcome {
    match out {
        Ok(output) => PromptOutcome::Settled { output },
        Err(err) => PromptOutcome::Rejected(err),
    }
}
