//! Panic / `JoinError` の `INTERNAL_ERROR` 正規化境界。
//!
//! run orchestration は panic を外部へ逃がさない — envelope 層(Phase 4)
//! が常に code 付き error を観測できるよう、run 境界で `catch_unwind`
//! し panic payload を `INTERNAL_ERROR` に畳む。`tokio::spawn` 経由の
//! 失敗(`JoinError`)も同じ code へ写像する。

use std::any::Any;
use std::fmt;
use std::future::Future;
use std::panic::AssertUnwindSafe;

use cheaper_core::errors::{CheaperError, ErrorCode};
use futures::FutureExt;

/// panic payload を `INTERNAL_ERROR` の `cause` として保持する wrapper。
/// `Box<dyn Any>` は `Error` を実装しないため、message 文字列を取り出して
/// `cause` に載せる(payload 全体は Debug のみ可能なので downcast 済み
/// message が最も情報量が多い)。
#[derive(Debug)]
struct PanicCause {
    message: String,
}

impl fmt::Display for PanicCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PanicCause {}

/// panic payload → `INTERNAL_ERROR`(TS の outer catch
/// `worker run failed` INTERNAL_ERROR 相当)。
pub fn run_panicked_to_internal(payload: Box<dyn Any + Send>) -> CheaperError {
    let message = payload
        .downcast_ref::<&'static str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string panic payload".to_string());
    CheaperError::new(
        ErrorCode::InternalError,
        format!("worker run failed: panic: {message}"),
    )
    .with_cause(PanicCause { message })
}

/// `run_worker` 本体を panic 境界で包む。`AssertUnwindSafe` — runtime の
/// invariant(共有状態の不変性)は `Mutex`/immutability で守られており、
/// poison は `Mutex::lock` 側で `into_inner` 復旧される設計のため
/// unwind-safety 上書きは安全。
pub async fn catch_run_panic<F, T>(future: F) -> Result<T, CheaperError>
where
    F: Future<Output = T>,
{
    match AssertUnwindSafe(future).catch_unwind().await {
        Ok(out) => Ok(out),
        Err(payload) => Err(run_panicked_to_internal(payload)),
    }
}

/// `tokio::task::JoinError` → `INTERNAL_ERROR`。panic の場合は内側の
/// panic message も拾う(TS `Promise` rejection の normalize 相当)。
pub fn join_error_to_internal(err: tokio::task::JoinError) -> CheaperError {
    if err.is_panic() {
        return run_panicked_to_internal(err.into_panic());
    }
    if err.is_cancelled() {
        return CheaperError::new(ErrorCode::InternalError, "worker task was cancelled");
    }
    CheaperError::new(
        ErrorCode::InternalError,
        format!("worker task failed: {err}"),
    )
}
