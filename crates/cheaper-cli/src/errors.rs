//! CLI 層の error 構築 helper。`CheaperError::new(InvalidRequest, …)`
//! の組み立てを一箇所に集約し、command 間で message/details の shape
//! を揃える。

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Map, Value};

/// `INVALID_REQUEST` error。details は省略可能。
pub(crate) fn invalid_request(
    message: impl Into<String>,
    details: Option<Map<String, Value>>,
) -> CheaperError {
    let e = CheaperError::new(ErrorCode::InvalidRequest, message.into());
    match details {
        Some(d) => e.with_details(d),
        None => e,
    }
}

/// `--timeout` の不正値 error。`run`/`workflow run` で同一 shape
/// (`details: {"timeout": <value>}`)に揃える。
pub(crate) fn invalid_timeout(value: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRequest,
        format!("invalid --timeout duration: {value}"),
    )
    .with_details(Map::from_iter([("timeout".to_string(), json!(value))]))
}
