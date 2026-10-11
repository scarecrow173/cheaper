//! T3 run orchestration 統合テスト — `runWorker` の non-seam case。
//!
//! `dev-seam` cargo feature に依存しない case のみ置く。
//! `fake_provider`/`fixture` seam 依存の test は `run_dev_seam.rs`
//! (`required-features = ["dev-seam"]` で gate)へ分離済み。

use std::error::Error;

use cheaper_core::errors::ErrorCode;
use cheaper_runtime::run_error::WorkerRunError;

#[tokio::test]
async fn join_error_normalizes_to_internal_error() {
    use cheaper_runtime::panic_boundary::join_error_to_internal;
    let handle = tokio::spawn(async { panic!("task panicked") });
    let join_err = handle.await.unwrap_err();
    let err = join_error_to_internal(join_err);
    assert_eq!(err.code(), ErrorCode::InternalError);
    assert!(err.message().contains("task panicked"));
}

// WorkerRunError が std::error::Error として扱えること(envelope 層の前提)。
#[allow(dead_code)]
fn assert_error(e: &WorkerRunError) -> &dyn Error {
    e
}
