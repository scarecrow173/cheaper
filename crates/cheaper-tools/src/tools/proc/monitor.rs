//! monitor 責務: stream capture・heartbeat/diagnostic emit・`drive`
//! (wait/timeout/abort を裁く監視 loop)。cleanup task の起動/完了は
//! [`super::cleanup`] へ委譲する。

use std::io::Write;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Map, Value};
use tokio::io::AsyncReadExt;
use tokio::task::JoinHandle;
use tokio::time::sleep;

use super::cleanup;
use super::spawn::{display, status_parts};
use super::tree::{create_process_tree, ProcessTree};
use super::{
    aborted_err, combined_reason, wait_aborted, BoxedCause, OnTimeout, ProcIdentity, ProcLimits,
};
use crate::catalog::{CanonicalToolResult, ToolContent};
use crate::operation::{AbortController, AbortReason, AbortSignal};
use crate::ToolError;

// ---------------------------------------------------------------------------
// Combined stream capture(64KiB / maxOutputBytes)
// ---------------------------------------------------------------------------

/// `StreamCapture`(`{text,bytes,capturedBytes,truncated}`)。
#[derive(Debug, Clone, PartialEq)]
pub struct StreamCaptureResult {
    pub text: String,
    pub bytes: u64,
    pub captured_bytes: u64,
    pub truncated: bool,
}

impl StreamCaptureResult {
    pub(super) fn to_json(&self) -> Value {
        json!({
            "text": self.text,
            "bytes": self.bytes,
            "capturedBytes": self.captured_bytes,
            "truncated": self.truncated,
        })
    }
}

#[derive(Default)]
pub(super) struct StreamCapture {
    chunks: Vec<Vec<u8>>,
    bytes: u64,
    captured_bytes: u64,
}

impl StreamCapture {
    /// `append(chunk, remaining)` → 実 captured 量を返す。
    pub(super) fn append(&mut self, chunk: &[u8], remaining: u64) -> u64 {
        self.bytes += chunk.len() as u64;
        let length = (chunk.len() as u64).min(remaining);
        if length > 0 {
            self.chunks.push(chunk[..length as usize].to_vec());
            self.captured_bytes += length;
        }
        length
    }

    /// `finish()`: invalid UTF-8 の replacement 拡大と途中 codepoint を
    /// 同じ byte cap 内に収める(TS `toString` → 再符号化 →
    /// `subarray(0, capturedBytes)` → `TextDecoder(stream:true)` parity)。
    pub(super) fn finish(self) -> StreamCaptureResult {
        let raw: Vec<u8> = self.chunks.concat();
        let text = String::from_utf8_lossy(&raw);
        let encoded = text.as_bytes().to_vec();
        let n = (self.captured_bytes as usize).min(encoded.len());
        let head = &encoded[..n];
        // stream:true decode は末尾の不完全 codepoint を出力しない
        // (replacement も出ない)→ valid prefix に切る。
        let valid = match std::str::from_utf8(head) {
            Ok(_) => head.len(),
            Err(e) => e.valid_up_to(),
        };
        let text = String::from_utf8_lossy(&head[..valid]).into_owned();
        StreamCaptureResult {
            text,
            bytes: self.bytes,
            captured_bytes: self.captured_bytes,
            truncated: self.bytes > self.captured_bytes
                || encoded.len() > self.captured_bytes as usize,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum StreamKind {
    Stdout,
    Stderr,
}

pub(super) struct Captures {
    pub(super) stdout: StreamCapture,
    pub(super) stderr: StreamCapture,
    pub(super) remaining: u64,
}

impl Captures {
    fn append(&mut self, kind: StreamKind, chunk: &[u8]) {
        let taken = match kind {
            StreamKind::Stdout => self.stdout.append(chunk, self.remaining),
            StreamKind::Stderr => self.stderr.append(chunk, self.remaining),
        };
        self.remaining = self.remaining.saturating_sub(taken);
    }

    fn output_bytes(&self) -> u64 {
        self.stdout.bytes + self.stderr.bytes
    }
}

/// stdout/stderr を EOF まで drain し combined remaining を消費する。
pub(super) async fn read_stream<R: tokio::io::AsyncRead + Unpin>(
    mut reader: R,
    kind: StreamKind,
    captures: Arc<Mutex<Captures>>,
) {
    let mut buf = vec![0u8; 16 * 1024];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let mut locked = captures.lock().expect("captures");
                locked.append(kind, &buf[..n]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Execute
// ---------------------------------------------------------------------------
pub(super) const PHASE_RUNNING: u8 = 0;
pub(super) const PHASE_CLEANUP: u8 = 1;

pub(super) struct Diagnostics {
    /// emit 行の `task` field 値(`ProcIdentity::label` — task/grant 名)。
    pub(super) label: String,
    /// event 名 prefix(`ProcIdentity::event_prefix` — `"process"` 等)。
    pub(super) event_prefix: &'static str,
    pub(super) pid: Option<u32>,
    pub(super) started: Instant,
    pub(super) phase: Arc<AtomicU8>,
    pub(super) captures: Arc<Mutex<Captures>>,
}

impl Diagnostics {
    /// `diagnostic(event)`: heartbeat/cleanup-failed を stderr JSON 1 行で。
    /// 診断の失敗は child の結果を変えない。
    pub(super) fn emit(&self, event: &str) {
        let output_bytes = self.captures.lock().map(|c| c.output_bytes()).unwrap_or(0);
        let phase = if self.phase.load(Ordering::Relaxed) == PHASE_RUNNING {
            "running"
        } else {
            "cleanup"
        };
        let line = json!({
            "event": event,
            "task": self.label,
            "pid": self.pid,
            // TS `Math.round(performance.now() - startedAt)` 相当 —
            // `as_millis()` の切り捨てではなく最近接丸めに揃える。
            "elapsedMs": (self.started.elapsed().as_secs_f64() * 1000.0).round() as u64,
            "phase": phase,
            "outputBytes": output_bytes,
        });
        // 診断 write が lock 競合や OS pipe 満杯で blocking しても child
        // の結果を変えないよう、stderr write は detached thread へ逃がす
        // (TS の backpressure drop 相当の best-effort 化)。
        let line = line.to_string();
        std::thread::spawn(move || {
            let _ = writeln!(std::io::stderr(), "{line}");
        });
    }
}

/// `Command::spawn` の raw OS error → Node `spawn` が `error.code` に
/// 載せる libuv `uv_translate_sys_error` 名。`raw_os_error()` が無い
/// (Rust 側で構築された)error は `"SPAWN_ERROR"` に fallback する
/// (TS の `error.code ?? "SPAWN_ERROR"` 相当)。
///
/// Windows の unmapped code は libuv が `UV__UNKNOWN` を返し
/// `error.code === "UNKNOWN"` となる(Node 26 実測: corrupt exe の
/// `ERROR_EXE_MACHINE_TYPE_MISMATCH` = 216 → "UNKNOWN"、
/// `ERROR_BAD_EXE_FORMAT` = 193 → "EFTYPE" は table 上の個別写像)。
/// unix は libuv が `-errno` をそのまま UV code 化するため表外の errno
/// も既知名を持つが、Rust から symbolic name は取れないため表に無い
/// ものは `"SPAWN_ERROR"` で近似する(残留差分)。
#[allow(clippy::too_many_arguments)]
pub(super) async fn drive(
    limits: &ProcLimits,
    identity: &ProcIdentity,
    outer: &AbortSignal,
    inner: &AbortSignal,
    timeout_controller: &AbortController,
    child: &mut tokio::process::Child,
    captures: Arc<Mutex<Captures>>,
    diagnostics: Arc<Diagnostics>,
    phase: Arc<AtomicU8>,
    stdout_task: JoinHandle<()>,
    stderr_task: JoinHandle<()>,
    on_timeout: Option<&OnTimeout>,
) -> Result<CanonicalToolResult, ToolError> {
    // duration は呼出し側で `parse_duration` 済み(policy resolve で
    // 検証済みのため到達しない想定)。
    let timeout_ms = limits.timeout_ms;
    let kill_grace_ms = limits.kill_grace_ms;
    let mut tree: Option<ProcessTree> = child.id().map(create_process_tree);
    let mut cleanup_handle: Option<JoinHandle<Result<(), ToolError>>> = None;
    let mut timeout_sleep = Box::pin(sleep(std::time::Duration::from_millis(timeout_ms)));
    let mut timeout_fired = false;

    // `close` まで待つ間に abort/timeout/exit を裁く。
    let ended: std::io::Result<std::process::ExitStatus> = loop {
        tokio::select! {
            status = child.wait() => break status,
            _ = &mut timeout_sleep, if !timeout_fired => {
                timeout_fired = true;
                if combined_reason(outer, inner).is_none()
                    && phase.load(Ordering::Relaxed) == PHASE_RUNNING
                {
                    let mut details = Map::new();
                    for (key, value) in &identity.details {
                        details.insert(key.clone(), value.clone());
                    }
                    if identity.report_duration_ms {
                        details.insert(
                            "durationMs".to_string(),
                            json!(diagnostics.started.elapsed().as_millis() as u64),
                        );
                    }
                    let reason = CheaperError::new(
                        ErrorCode::Timeout,
                        "managed process execution timed out",
                    )
                    .with_details(details);
                    if let Some(on_timeout) = on_timeout {
                        on_timeout(&reason);
                    }
                    timeout_controller.abort(AbortReason::from(reason));
                }
            }
            _ = wait_aborted(outer, inner), if cleanup_handle.is_none() => {
                cleanup::start_cleanup(&phase, &mut tree, &mut cleanup_handle, kill_grace_ms);
            }
        }
    };
    // 'exit' 相当: 通常終了でも descendants cleanup を走らせる。
    if cleanup_handle.is_none() {
        cleanup::start_cleanup(&phase, &mut tree, &mut cleanup_handle, kill_grace_ms);
    }
    let status = ended?;
    // 'close' 相当: streams の EOF まで待つ。
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let cleanup_error = cleanup::finish_cleanup(cleanup_handle, child, &diagnostics).await;

    let (stdout, stderr) = {
        let mut locked = captures.lock().expect("captures");
        (
            std::mem::take(&mut locked.stdout).finish(),
            std::mem::take(&mut locked.stderr).finish(),
        )
    };
    let (exit_code, signal) = status_parts(&status);
    tracing::debug!(
        target: "cheaper.tool.process",
        task = %identity.label,
        exit.code = exit_code,
        "process exit"
    );
    let state = if exit_code == Some(0) {
        "completed"
    } else {
        "failed"
    };
    let mut details = Map::new();
    for (key, value) in &identity.details {
        details.insert(key.clone(), value.clone());
    }
    details.insert("state".to_string(), json!(state));
    details.insert(
        "exitCode".to_string(),
        exit_code.map_or(Value::Null, |c| json!(c)),
    );
    details.insert(
        "signal".to_string(),
        signal.map_or(Value::Null, |s| json!(s)),
    );
    if identity.report_duration_ms {
        details.insert(
            "durationMs".to_string(),
            json!(diagnostics.started.elapsed().as_millis() as u64),
        );
    }
    details.insert("stdout".to_string(), stdout.to_json());
    details.insert("stderr".to_string(), stderr.to_json());
    let details = Value::Object(details);

    if let Some(reason) = combined_reason(outer, inner) {
        return Err(aborted_err(reason));
    }
    if let Some(error) = cleanup_error {
        return Err(CheaperError::new(
            ErrorCode::ToolExecutionFailure,
            "managed process cleanup failed",
        )
        .with_details(details.as_object().cloned().unwrap_or_default())
        .with_cause(BoxedCause(Arc::from(error)))
        .into());
    }
    if exit_code != Some(0) {
        return Err(
            CheaperError::new(ErrorCode::ToolExecutionFailure, "managed process failed")
                .with_details(details.as_object().cloned().unwrap_or_default())
                .into(),
        );
    }
    // 表示だけを折り返す。raw stdout/stderr は details に保持する。
    let text = format!(
        "{}: completed, exitCode=0\nstdout: {}\nstderr: {}",
        identity.display,
        display(&stdout),
        display(&stderr),
    );
    Ok(CanonicalToolResult {
        content: vec![ToolContent::text(text)],
        details: Some(details),
    })
}
