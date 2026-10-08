//! `PromptError`/provider error → `CheaperError` 写像と transient 分類。
//!
//! Phase 0 PoC (`crates/poc-rig/src/errors.rs`) の classification を土台に、
//! adapter 境界で Rig error を `CheaperError` に写す。Rig 型を上位へ漏らさない。
//!
//! 分類規則(TS `isTransient*` + Pi auto-retry 意味論の移植):
//! - HTTP 429 → transient rate limit
//! - HTTP status >= 500 → transient server
//! - その他の HTTP status → permanent
//! - `ErrorKind::Http` で応答を持たない → transient transport(timeout/reset)
//! - provider error が `retryable` を主張 → transient transport
//! - それ以外 → permanent

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{Map, Value};

use rig::agent::StreamingError;
use rig::completion::PromptError;
use rig::error::{ErrorKind, ErrorReport, ProviderError};

/// cheaper's error classes: only completion-level transient failures are
/// retryable; everything else is permanent for retry purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// HTTP 429 — rate limit。backoff 後に retry 可能。
    TransientRateLimit,
    /// HTTP 5xx — provider 側の一時障害。
    TransientServer,
    /// 応答を持たない transport 失敗(timeout/reset/unreadable)または
    /// provider が retryable と明記した失敗。
    TransientTransport,
    /// retry しても同じ結果になる恒久的失敗。
    Permanent,
}

impl ErrorClass {
    /// Whether retrying the same completion call may succeed.
    pub fn is_transient(&self) -> bool {
        !matches!(self, Self::Permanent)
    }
}

fn classify_status(status: Option<u16>) -> Option<ErrorClass> {
    match status {
        Some(429) => Some(ErrorClass::TransientRateLimit),
        Some(s) if s >= 500 => Some(ErrorClass::TransientServer),
        Some(_) => Some(ErrorClass::Permanent),
        None => None,
    }
}

/// Classify a provider error: a preserved reply is classified by its HTTP
/// status; a transport failure with no reply (timeout/reset/unreadable) is
/// transient; everything else defers to the provider's own `retryable` flag.
pub fn classify(error: &ProviderError) -> ErrorClass {
    if let Some(class) = classify_status(error.provider_response_status().map(|s| s.as_u16())) {
        return class;
    }
    match error.kind() {
        ErrorKind::Http => ErrorClass::TransientTransport,
        _ if error.is_retryable() => ErrorClass::TransientTransport,
        _ => ErrorClass::Permanent,
    }
}

/// Classify a relayed report the same way: `http_status` and `retryable`
/// survive the wire boundary, so the decision does not depend on where the
/// failure surfaced.
pub fn classify_report(report: &ErrorReport) -> ErrorClass {
    if let Some(class) = classify_status(report.http_status) {
        return class;
    }
    match report.kind {
        ErrorKind::Http => ErrorClass::TransientTransport,
        _ if report.retryable => ErrorClass::TransientTransport,
        _ => ErrorClass::Permanent,
    }
}

/// Classify a run-level prompt error.
pub fn classify_prompt(error: &PromptError) -> ErrorClass {
    match error {
        PromptError::CompletionError(e) => classify(e),
        PromptError::Report(r) => classify_report(r),
        _ => ErrorClass::Permanent,
    }
}

/// `ProviderError` → `CheaperError`。provider 起因の失敗は一律
/// `PROVIDER_FAILURE`。details は TS `lastAssistant.stopReason ===
/// "error"` 分岐(run.ts L382-390)と同じ `{errorMessage}` のみ —
/// 分類(transient/permanent)は retry 判定で消費され、envelope
/// contract には載らない。
pub fn provider_error_to_cheaper(error: &ProviderError) -> CheaperError {
    CheaperError::new(
        ErrorCode::ProviderFailure,
        format!("provider error: {error}"),
    )
    .with_details(Map::from_iter([(
        "errorMessage".to_string(),
        Value::from(error.to_string()),
    )]))
}

/// `ErrorReport`(bus/hook/report 経路) → `CheaperError`。
/// `ErrorKind` に応じて最も近い cheaper code へ写す。
pub fn report_to_cheaper(report: &ErrorReport) -> CheaperError {
    let code = match report.kind {
        ErrorKind::Cancelled => ErrorCode::Aborted,
        ErrorKind::Timeout => ErrorCode::Timeout,
        ErrorKind::Denied => ErrorCode::PolicyViolation,
        ErrorKind::Tool(_) => ErrorCode::ToolExecutionFailure,
        // provider 接線の失敗(transport・provider 応答・SDK 報告・request/
        // response 構築)は run.ts の `outcome.kind === "rejected" →
        // PROVIDER_FAILURE` parity で一律 PROVIDER_FAILURE。transient/
        // permanent の区別は retry 判定で消費され、details には残さない
        // (details は `{errorMessage}` のみ)。
        ErrorKind::Http
        | ErrorKind::Provider
        | ErrorKind::ProviderResponse
        | ErrorKind::Request
        | ErrorKind::Url
        | ErrorKind::Json
        | ErrorKind::Response => ErrorCode::ProviderFailure,
        // memory 未構成・内部 invariant・bus 寿命問題は adapter 側の失敗。
        _ => ErrorCode::InternalError,
    };
    let details = if code == ErrorCode::ProviderFailure {
        // PROVIDER_FAILURE の details contract は `{errorMessage}` のみ
        // (run.ts L382-390)— relay 経路でも同じ形を出す。
        Map::from_iter([(
            "errorMessage".to_string(),
            Value::from(report.message.clone()),
        )])
    } else {
        let mut details = Map::new();
        details.insert("kind".into(), Value::from(report.kind.code()));
        let class = classify_report(report);
        details.insert("class".into(), Value::from(format!("{class:?}")));
        details.insert("retryable".into(), Value::from(class.is_transient()));
        if let Some(status) = report.http_status {
            details.insert("httpStatus".into(), Value::from(status));
        }
        if let Some(code) = &report.code {
            details.insert("providerCode".into(), Value::from(code.clone()));
        }
        if let Some(id) = &report.request_id {
            details.insert("providerRequestId".into(), Value::from(id.clone()));
        }
        details
    };
    CheaperError::new(code, report.message.clone()).with_details(details)
}

/// `PromptError` → `CheaperError`。
///
/// - `CompletionError`/`Report` は分類に従い `PROVIDER_FAILURE` 系へ
/// - `PromptCancelled` は `ABORTED`(reason は runtime の abort 記録が正)
/// - `MaxTurnsError` は adapter 内部 guard(`DEFAULT_MAX_TURNS`)への到達 —
///   cheaper の policy maxTurns abort が正の経路なので `INTERNAL_ERROR` で区別
/// - `UnknownToolCall` は invalid-call retry 予算尽き後の到達 — model が
///   contract に従わなかった provider 側失敗として `PROVIDER_FAILURE`
/// - `MemoryError` は memory 未構成の adapter では到達不能に近い —
///   `INTERNAL_ERROR`
pub fn prompt_error_to_cheaper(error: PromptError) -> CheaperError {
    match error {
        PromptError::CompletionError(e) => provider_error_to_cheaper(&e),
        PromptError::Report(r) => report_to_cheaper(&r),
        PromptError::PromptCancelled { reason, .. } => CheaperError::new(
            ErrorCode::Aborted,
            format!("worker run cancelled: {reason}"),
        ),
        PromptError::MaxTurnsError { max_turns, .. } => CheaperError::new(
            ErrorCode::InternalError,
            format!(
                "internal max-turns guard reached ({max_turns}); cheaper policy abort should have fired first"
            ),
        ),
        PromptError::UnknownToolCall { tool_name, .. } => CheaperError::new(
            ErrorCode::ProviderFailure,
            format!("model kept calling unavailable tool `{tool_name}` after retry feedback"),
        ),
        PromptError::MemoryError(e) => CheaperError::new(
            ErrorCode::InternalError,
            format!("memory error on a memoryless worker run: {e}"),
        ),
    }
}

/// `StreamingError` → `PromptError` → `CheaperError`(PoC と同じ正規化)。
pub fn streaming_error_to_cheaper(error: StreamingError) -> CheaperError {
    let prompt = match error {
        StreamingError::Prompt(e) => e,
        StreamingError::Completion(e) => PromptError::CompletionError(e),
        StreamingError::Report(r) => PromptError::Report(r),
    };
    prompt_error_to_cheaper(prompt)
}

/// abort signal の `reason` を `CheaperError` へ。runtime が記録した
/// reason の code(timeout/signal/maxTurns/maxToolCalls → TIMEOUT/ABORTED/
/// MAX_TURNS_EXCEEDED/MAX_TOOL_CALLS_EXCEEDED)をそのまま引き継ぎ、
/// code を持たない汎用 reason は `ABORTED` へ。
pub fn abort_reason_to_cheaper(reason: &cheaper_tools::operation::AbortReason) -> CheaperError {
    if let Some(code) = reason.code().and_then(error_code_from_str) {
        return CheaperError::new(code, reason.message());
    }
    CheaperError::new(
        ErrorCode::Aborted,
        format!("worker run aborted: {}", reason.message()),
    )
}

/// `AbortReason.code()` が保持する `describeError` 済み code 文字列の逆引き。
/// `cheaper-core` に `FromStr` は無いので adapter 側で閉じて持つ。
fn error_code_from_str(code: &str) -> Option<ErrorCode> {
    Some(match code {
        "INVALID_REQUEST" => ErrorCode::InvalidRequest,
        "INVALID_INPUT" => ErrorCode::InvalidInput,
        "FILE_TOO_LARGE" => ErrorCode::FileTooLarge,
        "WORKER_NOT_FOUND" => ErrorCode::WorkerNotFound,
        "INVALID_WORKER_DEFINITION" => ErrorCode::InvalidWorkerDefinition,
        "INVALID_WORKER_NAME" => ErrorCode::InvalidWorkerName,
        "WORKER_NAME_MISMATCH" => ErrorCode::WorkerNameMismatch,
        "UNKNOWN_TOOL" => ErrorCode::UnknownTool,
        "INVALID_RUNTIME_POLICY" => ErrorCode::InvalidRuntimePolicy,
        "OUTPUT_SCHEMA_NOT_FOUND" => ErrorCode::OutputSchemaNotFound,
        "INVALID_OUTPUT_SCHEMA" => ErrorCode::InvalidOutputSchema,
        "INPUT_SCHEMA_NOT_FOUND" => ErrorCode::InputSchemaNotFound,
        "INVALID_INPUT_SCHEMA" => ErrorCode::InvalidInputSchema,
        "INVALID_MODEL_CONFIG" => ErrorCode::InvalidModelConfig,
        "POLICY_VIOLATION" => ErrorCode::PolicyViolation,
        "MODEL_TIER_UNAVAILABLE" => ErrorCode::ModelTierUnavailable,
        "PROVIDER_FAILURE" => ErrorCode::ProviderFailure,
        "TOOL_EXECUTION_FAILURE" => ErrorCode::ToolExecutionFailure,
        "INVALID_OUTPUT" => ErrorCode::InvalidOutput,
        "MAX_TURNS_EXCEEDED" => ErrorCode::MaxTurnsExceeded,
        "MAX_TOOL_CALLS_EXCEEDED" => ErrorCode::MaxToolCallsExceeded,
        "TIMEOUT" => ErrorCode::Timeout,
        "ABORTED" => ErrorCode::Aborted,
        "PARTIAL_MUTATION" => ErrorCode::PartialMutation,
        "INTERNAL_ERROR" => ErrorCode::InternalError,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::ProviderResponseError;

    /// PROVIDER_FAILURE の details contract guard: `{errorMessage}` のみ
    /// (run.ts L382-390 parity)。分類情報(status/class/retryable/
    /// providerCode 等)は retry 判定で消費され details には載らない —
    /// node-gated differential harness より先にここで extra key を検出する。
    fn assert_error_message_only(error: &CheaperError) {
        assert_eq!(error.code(), ErrorCode::ProviderFailure);
        let details = error
            .details()
            .expect("PROVIDER_FAILURE must carry details");
        assert_eq!(
            details.len(),
            1,
            "unexpected details keys: {:?}",
            details.keys().collect::<Vec<_>>()
        );
        assert!(details.contains_key("errorMessage"));
    }

    #[test]
    fn provider_failure_details_is_error_message_only() {
        // `provider_error_to_cheaper` 経路 — status 付き provider 応答でも
        // 分類情報は details に漏れない。
        let provider = ProviderError::ProviderResponse(ProviderResponseError::new(
            http::StatusCode::INTERNAL_SERVER_ERROR,
            "boom",
        ));
        assert_error_message_only(&provider_error_to_cheaper(&provider));

        // `report_to_cheaper` 経路 — report が status/code/request_id を
        // 持っていても PROVIDER_FAILURE details は `{errorMessage}` のみ。
        let report = ErrorReport {
            kind: ErrorKind::ProviderResponse,
            retryable: true,
            message: "server exploded".to_string(),
            code: Some("upstream_500".to_string()),
            http_status: Some(500),
            refusal: false,
            source_chain: vec![],
            request_id: Some("req-1".to_string()),
            provider_response: None,
            detail: None,
        };
        assert_error_message_only(&report_to_cheaper(&report));
    }
}
