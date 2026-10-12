//! cleanup 責務: abort/exit/timeout 後の process tree 終了を deferred
//! task として起動し、wait 完了を集約する。cleanup 順序(phase store ->
//! tree take -> terminate -> EOF 待ち -> error 集約)は drive との契約。

use std::sync::atomic::{AtomicU8, Ordering};

use tokio::task::JoinHandle;

use super::monitor::{Diagnostics, PHASE_CLEANUP};
use super::tree::ProcessTree;
use crate::ToolError;

/// `start_cleanup`: phase を CLEANUP へ進め、process tree 終了を deferred
/// task として起動する。冪起動は呼び出し側が `cleanup_handle.is_none()`
/// で担保する。
pub(super) fn start_cleanup(
    phase: &AtomicU8,
    tree: &mut Option<ProcessTree>,
    cleanup_handle: &mut Option<JoinHandle<Result<(), ToolError>>>,
    kill_grace_ms: u64,
) {
    phase.store(PHASE_CLEANUP, Ordering::Relaxed);
    let t = tree.take();
    *cleanup_handle = Some(tokio::spawn(async move {
        match t {
            Some(mut tree) => tree.terminate(kill_grace_ms).await,
            None => Ok(()),
        }
    }));
}

/// 'close' 後処理: cleanup task の完了を待ち、失敗時は diagnostic emit +
/// `start_kill` で収束させる。cleanup error は呼び出し側へ返す。
pub(super) async fn finish_cleanup(
    cleanup_handle: Option<JoinHandle<Result<(), ToolError>>>,
    child: &mut tokio::process::Child,
    diagnostics: &Diagnostics,
) -> Option<ToolError> {
    let mut cleanup_error: Option<ToolError> = None;
    if let Some(handle) = cleanup_handle {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => cleanup_error = Some(e),
            Err(e) => cleanup_error = Some(e.into()),
        }
    }
    if cleanup_error.is_some() {
        diagnostics.emit("process.cleanup-failed");
        let _ = child.start_kill();
    }
    cleanup_error
}
