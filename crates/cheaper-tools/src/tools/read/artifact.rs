//! artifact byte read mode: `source:{kind:artifact,...}` の positioned
//! read を port 経由で実行し slice + nextOffset/eof を返す。

use serde_json::{json, Value};

use cheaper_core::errors::{CheaperError, ErrorCode};

use crate::artifact::ReadSource;
use crate::catalog::{CanonicalToolResult, ToolContent};
use crate::operation::AbortSignal;
use crate::ToolError;

use super::{err, throw_if_aborted, ReadToolOptions};

/// `source` mode: artifact reader port への bounded byte read。
pub(super) fn read_artifact_source(
    opts: &ReadToolOptions<'_>,
    input: &Value,
    signal: Option<&AbortSignal>,
) -> Result<CanonicalToolResult, ToolError> {
    throw_if_aborted(signal)?;
    let reader = opts
        .artifacts
        .and_then(|p| p.reader.as_ref())
        .ok_or_else(|| {
            Box::new(CheaperError::new(
                ErrorCode::InvalidRequest,
                "artifact reader is unavailable",
            )) as ToolError
        })?;
    let source: ReadSource = serde_json::from_value(input["source"].clone())
        .map_err(|e| err(format!("invalid artifact source: {e}")))?;
    let artifact_source = match &source {
        ReadSource::Artifact(s) => s.clone(),
        _ => return Err(err("invalid artifact source")),
    };
    let result = reader.read(
        &artifact_source,
        &crate::artifact::ArtifactReadRequest {
            offset: input["offset"].as_u64().unwrap_or(0),
            max_bytes: input["maxBytes"].as_u64().unwrap_or(1),
        },
    )?;
    throw_if_aborted(signal)?;
    Ok(CanonicalToolResult {
        content: vec![ToolContent::text(String::from_utf8_lossy(&result.data))],
        details: Some(json!({
            "offset": result.offset,
            "nextOffset": result.next_offset,
            "eof": result.eof,
        })),
    })
}
