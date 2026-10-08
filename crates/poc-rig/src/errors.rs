use rig::completion::PromptError;
use rig::error::{ErrorKind, ErrorReport, ProviderError};
use serde::Serialize;

/// cheaper's error classes: only completion-level transient failures are
/// retryable; everything else is permanent for retry purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorClass {
    TransientRateLimit,
    TransientServer,
    TransientTransport,
    Permanent,
}

impl ErrorClass {
    pub fn is_transient(&self) -> bool {
        !matches!(self, Self::Permanent)
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::TransientRateLimit | Self::TransientServer | Self::TransientTransport => 75,
            Self::Permanent => 70,
        }
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
