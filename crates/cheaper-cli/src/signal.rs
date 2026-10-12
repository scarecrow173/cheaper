//! OS signal / `hooks.signal` を `AbortController` へ中継する共有 relay。
//!
//! `run` と `workflow run` の両 command で使う。external signal の
//! abort reason 扱いは経路ごとに異なる(TS parity):
//!
//! - [`SignalRelay::install`] — `run` 側。reason を捨てて既定
//!   AbortError に揃える(TS `ac.abort()` 相当)
//! - [`SignalRelay::install_forwarding`] — `workflow run` 側。
//!   external reason をそのまま forward する(TS
//!   `abortController.abort(external?.reason)` 相当)
//!
//! first reason は `AbortSignal` 側の OnceLock が守る。Drop で OS
//! signal task を abort し `hooks.signal` listener を外す(TS
//! `finally` の `process.off`/`removeEventListener` 相当)。

use cheaper_tools::operation::{AbortController, AbortListenerId, AbortReason, AbortSignal};

use crate::cli::CliHooks;

/// signal 中継 guard。保持している間、OS signal と external
/// `hooks.signal` を `controller` へ中継する。
pub(crate) struct SignalRelay {
    controller: AbortController,
    os_task: tokio::task::JoinHandle<()>,
    external: Option<(AbortSignal, AbortListenerId)>,
}

impl SignalRelay {
    /// `run` 側: relay が所有する controller を作り、external abort の
    /// reason を捨てる(TS `ac.abort()` — 分類は ABORTED 固定)。
    pub(crate) fn install(hooks: &CliHooks) -> Self {
        Self::install_inner(AbortController::new(), hooks, false)
    }

    /// `workflow run` 側: caller が所有する集約 controller へ接続し、
    /// external abort の reason をそのまま forward する(TS
    /// `abortController.abort(external?.reason)` 相当)。
    pub(crate) fn install_forwarding(controller: &AbortController, hooks: &CliHooks) -> Self {
        Self::install_inner(controller.clone(), hooks, true)
    }

    fn install_inner(controller: AbortController, hooks: &CliHooks, forward_reason: bool) -> Self {
        let external = hooks.signal.as_ref().map(|sig| {
            let ac = controller.clone();
            let id = sig.on_abort(move |reason: &AbortReason| {
                if forward_reason {
                    ac.abort(reason.clone());
                } else {
                    // TS は `ac.abort()` で reason を捨てる(既定
                    // AbortError) — `AbortReason::aborted()` で同じ分類
                    // (signal → ABORTED)に揃える。
                    ac.abort(AbortReason::aborted());
                }
            });
            // 登録後の aborted 確認 — abort 済み signal の listener は
            // 発火しない(TS `if (hooks.signal?.aborted) ac.abort()`)。
            if sig.is_aborted() {
                let reason = if forward_reason {
                    sig.reason().unwrap_or_else(AbortReason::aborted)
                } else {
                    AbortReason::aborted()
                };
                controller.abort(reason);
            }
            (sig.clone(), id)
        });
        let ac = controller.clone();
        let os_task = tokio::spawn(async move {
            wait_os_signal().await;
            ac.abort(AbortReason::aborted());
        });
        Self {
            controller,
            os_task,
            external,
        }
    }

    /// 集約 abort 側(`deps.signal` / fixture `abort` 発火先)の signal。
    pub(crate) fn signal(&self) -> AbortSignal {
        self.controller.signal()
    }

    /// 中継先 controller(fixture `abort` の遅延発火に使う)。
    /// seam-only — `dev-seam` feature でのみ sole caller(fixture abort
    /// task)が存在する。
    #[cfg(feature = "dev-seam")]
    pub(crate) fn controller(&self) -> AbortController {
        self.controller.clone()
    }
}

impl Drop for SignalRelay {
    fn drop(&mut self) {
        self.os_task.abort();
        if let Some((sig, id)) = self.external.take() {
            sig.remove_listener(id);
        }
    }
}

#[cfg(unix)]
pub(crate) async fn wait_os_signal() {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
        }
        // SIGTERM listener 構築失敗時は SIGINT のみ中継する。
        Err(_) => {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(not(unix))]
pub(crate) async fn wait_os_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
