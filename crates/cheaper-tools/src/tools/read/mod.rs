//! Port of `packages/tool-runtime/src/read.ts`.
//!
//! bounded read tool(DESIGN-0019)。`IO_CHUNK_BYTES` 単位の positioned
//! read で prefix を破棄しながら `offset` 行まで走査し、最大 `limit` 行・
//! `READ_MAX_BYTES` を返す。`byteInLine` で oversized line の byte
//! 継続読み取りを行う。decode は行単位で WHATWG UTF-8(不正 byte は
//! U+FFFD)。file handle は呼び出しごとに open し cache しない
//! (`Drop` = `finally close`)。

use std::fs::{File, OpenOptions};
use std::io;

use std::sync::Arc;

use cheaper_core::errors::{node_io_error, CheaperError, ErrorCode};

use cheaper_core::nodepath;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{resolve_workspace_path, SymlinkPolicy};
use serde_json::Value;

use crate::access::AccessObserver;
use crate::artifact::ArtifactPorts;
use crate::catalog::{self, CanonicalToolResult};
use crate::definition::{define_canonical_tool, CanonicalToolDefinition};
use crate::limits::{IO_CHUNK_BYTES, READ_DEFAULT_LIMIT, READ_MAX_BYTES, READ_MAX_LIMIT};
use crate::operation::AbortSignal;
use crate::walker::{rethrow_as_denied, DeniedError};
use crate::ToolError;

mod artifact;
mod publication;
#[cfg(test)]
mod tests;
mod workspace;
const IMAGE_EXTENSIONS: [&str; 6] = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"];
pub struct ReadToolOptions<'a> {
    /// artifact ports(reader/publisher)。無しでも workspace read は動く。
    pub artifacts: Option<&'a ArtifactPorts>,
    /// publication producer の `jobId`(既定 `"tool-runtime"`)。
    pub owner_id: Option<&'a str>,
    pub observe_access: Option<&'a AccessObserver<'a>>,
    /// canonical 化済み workspace root。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    pub denylist: Option<&'a PathDenylist>,
    /// 本文 byte 上限の縮退注入(既定 `READ_MAX_BYTES`)。テスト用。
    pub max_bytes: Option<usize>,
    /// positioned-read chunk byte 数の縮退注入(既定 `IO_CHUNK_BYTES`)。
    /// テスト用。
    pub chunk_bytes: Option<usize>,
}

/// `createReadTool` の返り値。
pub struct ReadTool<'a> {
    opts: ReadToolOptions<'a>,
}

/// `createReadTool`。
pub fn create_read_tool(opts: ReadToolOptions<'_>) -> ReadTool<'_> {
    ReadTool { opts }
}

fn err(msg: impl Into<String>) -> ToolError {
    io::Error::other(msg.into()).into()
}

/// `signal.throwIfAborted()` — abort 済みなら reason を投げる。
fn throw_if_aborted(signal: Option<&AbortSignal>) -> Result<(), ToolError> {
    if let Some(s) = signal {
        if s.is_aborted() {
            let msg = s
                .reason()
                .map(|r| r.message().to_string())
                .unwrap_or_else(|| "Operation aborted".to_string());
            return Err(err(msg));
        }
    }
    Ok(())
}
/// positioned read(`fh.read(buf, 0, len, pos)`)。unix/windows 双方の
/// `read_at`/`seek_read` を包む。
#[cfg(unix)]
fn read_at(fh: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt as _;
    fh.read_at(buf, pos)
}

#[cfg(windows)]
fn read_at(fh: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    use std::os::windows::fs::FileExt as _;
    fh.seek_read(buf, pos)
}

#[cfg(not(any(unix, windows)))]
fn read_at(fh: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    // 最小 fallback: clone handle で seek+read(positioned read 相当)。
    use std::io::{Read as _, Seek as _, SeekFrom};
    let mut f = fh.try_clone()?;
    f.seek(SeekFrom::Start(pos))?;
    f.read(buf)
}
impl ReadTool<'_> {
    /// `definition.execute` 相当。
    pub fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let opts = &self.opts;
        if input.get("source").is_some() {
            return artifact::read_artifact_source(opts, input, signal);
        }
        let has_artifact = input.get("artifact").is_some();
        let publisher = if has_artifact {
            opts.artifacts.and_then(|p| p.publisher.as_ref())
        } else {
            None
        };
        if has_artifact && publisher.is_none() {
            return Err(Box::new(CheaperError::new(
                ErrorCode::InvalidRequest,
                "artifact publisher is unavailable",
            )));
        }
        if has_artifact {
            throw_if_aborted(signal)?;
        }
        let workspace = if has_artifact { None } else { Some(input) };
        if workspace.and_then(|w| w.get("byteInLine")).is_some()
            && workspace.and_then(|w| w.get("offset")).is_none()
        {
            return Err(err("byteInLine requires offset"));
        }
        let path = input["path"].as_str().unwrap_or_default();
        let scan = workspace::ScanParams {
            offset: workspace.and_then(|w| w["offset"].as_u64()).unwrap_or(1) as usize,
            limit: std::cmp::min(
                workspace
                    .and_then(|w| w["limit"].as_u64())
                    .unwrap_or(READ_DEFAULT_LIMIT as u64) as usize,
                READ_MAX_LIMIT,
            ),
            byte_in_line: workspace
                .and_then(|w| w["byteInLine"].as_u64())
                .map(|v| v as usize),
            max_bytes: opts.max_bytes.unwrap_or(READ_MAX_BYTES),
            chunk_bytes: opts.chunk_bytes.unwrap_or(IO_CHUNK_BYTES),
            signal,
        };
        let result = (|| -> Result<CanonicalToolResult, ToolError> {
            let resolved = resolve_workspace_path(opts.root, path, opts.symlink_policy)
                .map_err(|e| Box::new(e) as ToolError)?;
            if opts
                .denylist
                .is_some_and(|d| d.is_denied(&resolved.display))
            {
                return Err(Box::new(DeniedError::policy()));
            }
            // stat を open 前に行い directory 等を platform 非依存の明示 error
            // にする。
            if let Some(observe) = opts.observe_access {
                observe(
                    &resolved.real,
                    cheaper_core::resources::types::ClaimMode::Shared,
                    None,
                    false,
                )?;
            }
            let st = std::fs::metadata(&resolved.real)
                .map_err(|e| Box::new(node_io_error("stat", &resolved.real, e)) as ToolError)?;
            if !st.is_file() {
                return Err(err("read: not a regular file"));
            }
            // 画像は走査前に明示 error。画像経路・全文 fallback は設けない。
            if IMAGE_EXTENSIONS.contains(&nodepath::extname(&resolved.real).to_lowercase().as_str())
            {
                return Err(err("read does not support image files"));
            }
            if let Some(observe) = opts.observe_access {
                observe(
                    &resolved.real,
                    cheaper_core::resources::types::ClaimMode::Shared,
                    None,
                    false,
                )?;
            }
            let fh = OpenOptions::new()
                .read(true)
                .open(&resolved.real)
                .map_err(|e| Box::new(node_io_error("open", &resolved.real, e)) as ToolError)?;
            if has_artifact {
                return publication::read_as_publication(
                    opts,
                    input,
                    &fh,
                    &resolved.real,
                    publisher.expect("checked above").as_ref(),
                    scan.chunk_bytes,
                    signal,
                );
            }
            workspace::read_bounded(&fh, &resolved.real, st.len(), &scan)
        })();
        result.map_err(rethrow_as_denied)
    }
}
pub fn read_tool_definition(
    opts: ReadToolOptions<'_>,
) -> Result<CanonicalToolDefinition<'_>, ToolError> {
    let tool = Arc::new(create_read_tool(opts));
    Ok(define_canonical_tool(CanonicalToolDefinition {
        name: ToolName::Read,
        label: "read".to_string(),
        description: "Choose one exclusive mode: workspace line read {path,offset?,limit?,byteInLine?} returns up to 2000 lines and 50KB; publication {path,artifact:name} returns only a bounded artifact reference; artifact byte read {source:{kind:artifact,reference},offset,maxBytes} returns a bounded slice and nextOffset/eof. Workspace offsets are 1-indexed lines; artifact offsets are 0-indexed source bytes. Binary/image workspace files are rejected. Invalid UTF-8 bytes are replaced with U+FFFD.".to_string(),
        parameters: catalog::read_schema()
            .as_object()
            .expect("object schema")
            .clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input, signal.as_ref()) })
        }),
    }))
}
