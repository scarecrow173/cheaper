//! publication mode: workspace file 全文を bounded に集めて artifact
//! publisher へ put し、reference のみを返す。

use std::fs::File;

use cheaper_core::errors::{node_io_error, CheaperError, ErrorCode};
use cheaper_core::js::MAX_SAFE_INTEGER_U64;
use serde_json::json;
use serde_json::Value;

use crate::artifact::{ArtifactData, ArtifactProducer, ArtifactPublisher, ArtifactPut};
use crate::catalog::CanonicalToolResult;
use crate::limits::SNIFF_BYTES;
use crate::operation::AbortSignal;
use crate::ToolError;

use super::{err, read_at, throw_if_aborted, ReadToolOptions};

/// `readPublication`: stat size に依存せず cap+1 まで走査し、put 前に
/// overflow を拒否する。先頭 SNIFF_BYTES に NUL があれば binary error。
pub(super) fn read_publication(
    fh: &File,
    path: &str,
    cap: u64,
    chunk_bytes: usize,
    signal: Option<&AbortSignal>,
) -> Result<Vec<u8>, ToolError> {
    // `Number.isSafeInteger(cap) && cap >= 0`。
    if cap > MAX_SAFE_INTEGER_U64 {
        return Err(Box::new(CheaperError::new(
            ErrorCode::InvalidRequest,
            "invalid artifact publication cap",
        )));
    }
    let mut chunks: Vec<u8> = Vec::new();
    let mut bytes: u64 = 0;
    let mut sniffed: u64 = 0;
    loop {
        throw_if_aborted(signal)?;
        let want = std::cmp::min(chunk_bytes as u64, cap - bytes + 1) as usize;
        let mut buf = vec![0u8; want];
        let n = read_at(fh, &mut buf, bytes)
            .map_err(|e| Box::new(node_io_error("read", path, e)) as ToolError)?;
        throw_if_aborted(signal)?;
        if n == 0 {
            return Ok(chunks);
        }
        let data = &buf[..n];
        let sniff_hit = data
            .iter()
            .take(SNIFF_BYTES.saturating_sub(sniffed as usize))
            .any(|&b| b == 0);
        if sniff_hit {
            return Err(err("read: binary file (NUL byte within first 8KiB)"));
        }
        sniffed += n as u64;
        bytes += n as u64;
        if bytes > cap {
            return Err(Box::new(CheaperError::new(
                ErrorCode::PolicyViolation,
                "artifact storage limit exceeded",
            )));
        }
        chunks.extend_from_slice(data);
    }
}

/// publication mode の本体: `read_publication` で bytes を集めて
/// `publisher.put` へ渡し、artifact reference のみを返す(TS 相当)。
pub(super) fn read_as_publication(
    opts: &ReadToolOptions<'_>,
    input: &Value,
    fh: &File,
    path: &str,
    publisher: &dyn ArtifactPublisher,
    chunk_bytes: usize,
    signal: Option<&AbortSignal>,
) -> Result<CanonicalToolResult, ToolError> {
    let data = read_publication(
        fh,
        path,
        publisher.max_artifact_bytes(),
        chunk_bytes,
        signal,
    )?;
    throw_if_aborted(signal)?;
    let artifact = publisher.put(ArtifactPut {
        producer: ArtifactProducer {
            job_id: opts.owner_id.unwrap_or("tool-runtime").to_string(),
            name: input["artifact"].as_str().unwrap_or_default().to_string(),
        },
        data: ArtifactData::Bytes(data),
    })?;
    Ok(CanonicalToolResult {
        content: Vec::new(),
        details: Some(json!({
            "artifact": serde_json::to_value(&artifact)
                .expect("artifact reference serializes"),
        })),
    })
}
