//! Run 全体の abort 一元化(run.ts `requestAbort`/`abortError` 相当)。
//!
//! TS では `AbortController` + `abortReason` 変数で「先勝ち reason 記録 +
//! signal 発火」を行う。Rust では `cheaper-tools` の `AbortController` が
//! first-reason-wins を既に保証するため、ここでは run 固有の reason
//! 分類(timeout/signal/maxTurns/maxToolCalls → TIMEOUT/ABORTED/
//! MAX_TURNS_EXCEEDED/MAX_TOOL_CALLS_EXCEEDED)を担う。
//!
//! `AbortController::abort` に渡す payload は `CheaperError` の
//! `describeError` 形(`name="CheaperError"` + `code`)に揃える —
//! adapter 側の `abort_reason_to_cheaper` が `code()` を逆引きして
//! 分類済み code を復元するため、stream 終端 error と run 側分類が一致する。

use std::sync::OnceLock;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_tools::operation::{AbortController, AbortReason, AbortSignal};

/// run 固有の abort reason(TS `AbortReason` union)。
/// `requestAbort` で先勝ち記録され、error 分類の正になる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunAbortReason {
    /// `policy.execution.timeoutMs` 経過。
    Timeout,
    /// 外部 `AbortSignal`(CLI の SIGINT 等)。
    Signal,
    /// `turns >= policy.execution.maxTurns` かつ最終 stop が tool 継続。
    MaxTurns,
    /// `policy.execution.maxToolCalls` 上限到達。
    MaxToolCalls,
}

impl RunAbortReason {
    /// TS の reason 文字列(`details.reason` / message に入る値)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Signal => "signal",
            Self::MaxTurns => "maxTurns",
            Self::MaxToolCalls => "maxToolCalls",
        }
    }

    /// `ABORT_CODES` 写像。
    pub const fn error_code(self) -> ErrorCode {
        match self {
            Self::Timeout => ErrorCode::Timeout,
            Self::Signal => ErrorCode::Aborted,
            Self::MaxTurns => ErrorCode::MaxTurnsExceeded,
            Self::MaxToolCalls => ErrorCode::MaxToolCallsExceeded,
        }
    }

    /// `abortError(reason)` — code + `details.reason` を持つ CheaperError。
    pub fn cheaper_error(self) -> CheaperError {
        CheaperError::new(
            self.error_code(),
            format!("worker run aborted: {}", self.as_str()),
        )
        .with_details(serde_json::Map::from_iter([(
            "reason".to_string(),
            serde_json::Value::String(self.as_str().to_string()),
        )]))
    }

    /// `controller.abort()` へ渡す payload。`code` を保持させることで
    /// adapter の `abort_reason_to_cheaper` が code を復元できる
    /// (`AbortReason::from_error` の `describeError` と同じ形)。
    fn abort_payload(self) -> AbortReason {
        AbortReason::new(
            "CheaperError",
            format!("worker run aborted: {}", self.as_str()),
            Some(self.error_code().as_str().to_string()),
        )
    }
}

/// run 全体の集約 abort(TS の `runAbort` + `abortReason` + `requestAbort`)。
/// reason は先勝ち — 2 つ目以降の trigger は記録を変えない。
#[derive(Debug)]
pub struct RunAbort {
    controller: AbortController,
    reason: OnceLock<RunAbortReason>,
}

impl RunAbort {
    pub fn new() -> Self {
        Self {
            controller: AbortController::new(),
            reason: OnceLock::new(),
        }
    }

    /// `runAbort.signal` — tool scope・prompt・repair session へ渡す
    /// 集約 signal(全 trigger がこれを fire する)。
    pub fn signal(&self) -> AbortSignal {
        self.controller.signal()
    }

    /// 記録済み reason(先勝ち)。未 abort なら `None`。
    pub fn reason(&self) -> Option<RunAbortReason> {
        self.reason.get().copied()
    }

    /// `requestAbort(reason)` — 先勝ちで reason を記録し signal を fire。
    /// reason 確定後の再呼出しは no-op(TS と同じ早期 return)。
    pub fn request_abort(&self, reason: RunAbortReason) {
        if self.reason.set(reason).is_err() {
            return;
        }
        self.controller.abort(reason.abort_payload());
    }
}

impl Default for RunAbort {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cheaper_core::errors::ErrorCode;

    #[test]
    fn first_reason_wins() {
        let abort = RunAbort::new();
        abort.request_abort(RunAbortReason::Signal);
        abort.request_abort(RunAbortReason::Timeout);
        assert_eq!(abort.reason(), Some(RunAbortReason::Signal));
        assert!(abort.signal().is_aborted());
        // payload は code を保持し adapter 経路で復元できる。
        let reason = abort.signal().reason().unwrap();
        assert_eq!(reason.code(), Some("ABORTED"));
        assert_eq!(reason.message(), "worker run aborted: signal");
    }

    #[test]
    fn abort_error_codes_and_details() {
        for (reason, code) in [
            (RunAbortReason::Timeout, ErrorCode::Timeout),
            (RunAbortReason::Signal, ErrorCode::Aborted),
            (RunAbortReason::MaxTurns, ErrorCode::MaxTurnsExceeded),
            (
                RunAbortReason::MaxToolCalls,
                ErrorCode::MaxToolCallsExceeded,
            ),
        ] {
            let err = reason.cheaper_error();
            assert_eq!(err.code(), code);
            assert_eq!(
                err.details().unwrap()["reason"],
                serde_json::json!(reason.as_str())
            );
            assert_eq!(
                err.message(),
                format!("worker run aborted: {}", reason.as_str())
            );
        }
    }
}
