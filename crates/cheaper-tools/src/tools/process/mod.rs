//! Port of `packages/tool-runtime/src/process.ts`.
//!
//! named task 解決・prepare(cwd/deny/writeScope 検査)・minimal env
//! (Windows required env 空値遮断・SystemRoot 補完)・spawn(stdin
//! ignore/stdout-stderr pipe/windowsHide/POSIX detached)・64KiB combined
//! capture・timeout(`parseDuration`)+`onTimeout`・heartbeat stderr JSON・
//! details{task,state,exitCode,signal,stdout,stderr}・64 文字 fold 表示・
//! `processOperationDeclaration`(pool + access 導出)。
//!
//! T2-7 統合済み seam(T2-5 時点のローカル定義は撤去):
//! - `AbortSignal`/`AbortController`/`AbortReason` — `crate::operation`。
//! - result は `crate::catalog::CanonicalToolResult`。
//! - `process_operation_declaration` は `crate::scheduler` の
//!   `OperationDeclaration`/`OperationPool` を返す。

pub mod executable;
pub mod tree;

use std::collections::HashSet;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cheaper_core::duration::parse_duration;
use cheaper_core::errors::{CheaperError, ErrorCode};
/// `MAX_PROCESS_OUTPUT_BYTES` re-export(TS `export { ... }` parity)。
pub use cheaper_core::policy::process::MAX_PROCESS_OUTPUT_BYTES;
use cheaper_core::policy::process::{resolve_process_task, ResolvedProcessPolicy};
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::{json, Map, Value};
use tokio_util::sync::CancellationToken;

use crate::catalog::CanonicalToolResult;
use crate::operation::{AbortController, AbortReason, AbortSignal};
use crate::scheduler::{OperationDeclaration, OperationPool};
use crate::ToolError;

mod cleanup;
mod monitor;
mod prepare;
mod spawn;
#[cfg(test)]
mod tests;

use monitor::{
    drive, read_stream, Captures, Diagnostics, StreamCapture, StreamKind, PHASE_RUNNING,
};
use prepare::{minimal_env, prepare};
use spawn::{build_command, spawn_code, spawn_failure_is_async};

pub use monitor::StreamCaptureResult;

// ---------------------------------------------------------------------------
// abort helpers(canonical `crate::operation` 型)
// ---------------------------------------------------------------------------

/// `signal.reason` の rethrow 相当。`AbortReason` 自体を error として包む
/// — `source()` 経由で error object(`CheaperError` 等)が辿れるため
/// `find_cheaper_error` の code 判定がそのまま効く。
fn aborted_err(reason: AbortReason) -> ToolError {
    Box::new(reason)
}

/// `AbortSignal.any` 相当: どちらかが abort したらその reason で解決。
/// 両方済みの場合は outer を優先する(発生順は区別不能)。
async fn wait_aborted(outer: &AbortSignal, inner: &AbortSignal) -> AbortReason {
    tokio::select! {
        _ = outer.cancelled() => outer
            .reason()
            .or_else(|| inner.reason())
            .unwrap_or_else(|| AbortReason::from(CheaperError::new(ErrorCode::Aborted, "aborted"))),
        _ = inner.cancelled() => inner
            .reason()
            .or_else(|| outer.reason())
            .unwrap_or_else(|| AbortReason::from(CheaperError::new(ErrorCode::Aborted, "aborted"))),
    }
}

fn combined_reason(outer: &AbortSignal, inner: &AbortSignal) -> Option<AbortReason> {
    outer.reason().or_else(|| inner.reason())
}

/// `ToolError`(Box<dyn Error>)を `CheaperError::with_cause`(sized
/// `impl Error` bound)へ渡すための wrapper(TS `cause: error` parity)。
#[derive(Debug)]
struct BoxedCause(Arc<dyn std::error::Error + Send + Sync + 'static>);

impl std::fmt::Display for BoxedCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for BoxedCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0
            .source()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// `Box<dyn Error>` の source chain から `CheaperError` を探す
/// (`Aborted` wrapper 越しの code 判定用)。
pub fn find_cheaper_error<'a>(
    err: &'a (dyn std::error::Error + 'static),
) -> Option<&'a CheaperError> {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = current {
        if let Some(cheaper) = e.downcast_ref::<CheaperError>() {
            return Some(cheaper);
        }
        current = e.source();
    }
    None
}

// ---------------------------------------------------------------------------
// Schemas / operation declaration / tool surface
// ---------------------------------------------------------------------------

pub fn process_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "task": {
                "type": "string",
                "minLength": 1,
                "maxLength": 64,
                "pattern": "^[a-z][a-z0-9_-]*$",
                "description": "Name of a policy-defined process task",
            },
        },
        "required": ["task"],
        "additionalProperties": false,
    })
}

fn stream_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "text": {"type": "string", "maxLength": MAX_PROCESS_OUTPUT_BYTES},
            "bytes": {"type": "integer", "minimum": 0},
            "capturedBytes": {"type": "integer", "minimum": 0, "maximum": MAX_PROCESS_OUTPUT_BYTES},
            "truncated": {"type": "boolean"},
        },
        "required": ["text", "bytes", "capturedBytes", "truncated"],
        "additionalProperties": false,
    })
}

/// `processDetailsSchema`(TypeBox 生成物を literal JSON 化)。
pub fn process_details_schema() -> Value {
    let stream = stream_schema();
    json!({
        "type": "object",
        "properties": {
            "task": {"type": "string"},
            "state": {"enum": ["completed", "failed"]},
            "exitCode": {"type": ["integer", "null"]},
            "signal": {"type": ["string", "null"]},
            "stdout": stream,
            "stderr": stream,
        },
        "required": ["task", "state", "exitCode", "signal", "stdout", "stderr"],
        "additionalProperties": false,
    })
}

/// `processOperationDeclaration`: task 解決 → outputs 有無で access 導出、
/// pool は `{name:"process", maxConcurrent}`。返すのは scheduler の
/// canonical `OperationDeclaration`(T2-5 のローカル seam 型は撤去)。
pub fn process_operation_declaration(
    policy: &ResolvedProcessPolicy,
    task: &str,
) -> Result<OperationDeclaration, CheaperError> {
    let task = resolve_process_task(policy, task)?;
    let mut declaration = OperationDeclaration::new(if task.outputs.is_empty() {
        cheaper_core::resources::types::ClaimMode::Shared
    } else {
        cheaper_core::resources::types::ClaimMode::Exclusive
    });
    declaration.pool = Some(OperationPool {
        name: "process".to_string(),
        max_concurrent: policy.max_concurrent as i64,
    });
    Ok(declaration)
}

/// `onTimeout(reason)` callback type。Rust では reason を `&CheaperError`
/// で渡す(reason object を呼出し側が再送しない前提)。
pub type OnTimeout = Arc<dyn Fn(&CheaperError) + Send + Sync>;

/// `Options`(TS `interface Options`)。
pub struct ProcessToolOptions {
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    pub process: ResolvedProcessPolicy,
    /// `onTimeout(reason)`。
    pub on_timeout: Option<OnTimeout>,
    pub denylist: Option<PathDenylist>,
    pub write_scope: Option<HashSet<String>>,
}

/// `createProcessTool` が返す canonical definition。
pub struct ProcessTool {
    description: String,
    options: ProcessToolOptions,
}

/// `createProcessTool`。description に task 一覧を埋め込む。
pub fn create_process_tool(options: ProcessToolOptions) -> ProcessTool {
    let tasks: Vec<&str> = options.process.tasks.keys().map(String::as_str).collect();
    ProcessTool {
        description: format!(
            "Run a policy-defined native process task: {}. No arbitrary command or shell.",
            tasks.join(", ")
        ),
        options,
    }
}

impl ProcessTool {
    pub fn name(&self) -> &'static str {
        "process"
    }

    pub fn label(&self) -> &'static str {
        "Process"
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    /// `parameters` JSON schema(`validateToolInput` 接合対象)。
    pub fn parameters(&self) -> Value {
        process_schema()
    }

    /// `execute(input, signal)`。`input.task` の schema 検証は catalog
    /// 側責務(TS 同様)。`signal` は `signal ?? new AbortController().signal`。
    pub async fn execute(
        &self,
        input: &Value,
        signal: Option<AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let task = input
            .get("task")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        execute(
            &self.options,
            &task,
            signal.unwrap_or_else(AbortSignal::detached),
        )
        .await
    }
}

/// `createProcessTool` の `CanonicalToolDefinition` ラッパ(TS
/// `defineCanonicalTool` 相当)。registry は `name`/`label`/`description`/
/// `parameters` をここから取得し、`execute` は canonical
/// `CanonicalExecute` seam(future)で呼ぶ。tool は `Arc` 共有 — 返り値
/// future が env borrow を持たず `'static` で独立する。
pub fn process_tool_definition(
    options: ProcessToolOptions,
) -> crate::definition::CanonicalToolDefinition<'static> {
    let tool = Arc::new(create_process_tool(options));
    crate::definition::define_canonical_tool(crate::definition::CanonicalToolDefinition {
        name: cheaper_core::tools::vocabulary::ToolName::Process,
        label: tool.label().to_string(),
        description: tool.description().to_string(),
        parameters: process_schema().as_object().expect("object schema").clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input, signal).await })
        }),
    })
}

// ---------------------------------------------------------------------------
// execute
// ---------------------------------------------------------------------------
async fn execute(
    options: &ProcessToolOptions,
    name: &str,
    outer: AbortSignal,
) -> Result<CanonicalToolResult, ToolError> {
    // `AbortSignal.any([outerSignal, timeoutController.signal])` 相当。
    let timeout_controller = AbortController::new();
    let inner = timeout_controller.signal();
    if let Some(reason) = combined_reason(&outer, &inner) {
        return Err(aborted_err(reason));
    }
    let task = resolve_process_task(&options.process, name)?;
    let prepared = prepare(options, name, &task)?;
    if let Some(reason) = combined_reason(&outer, &inner) {
        return Err(aborted_err(reason));
    }

    let captures = Arc::new(Mutex::new(Captures {
        stdout: StreamCapture::default(),
        stderr: StreamCapture::default(),
        remaining: options.process.max_output_bytes,
    }));

    let env = minimal_env(&task.env);
    let mut command = build_command(&prepared, &task, env);
    let mut child = match command.spawn() {
        Ok(child) => {
            tracing::debug!(target: "cheaper.tool.process", task = %name, "process spawn");
            child
        }
        Err(error) => {
            tracing::debug!(
                target: "cheaper.tool.process",
                task = %name,
                error.code = spawn_code(&error),
                "process spawn failed"
            );
            let stdout = StreamCapture::default().finish();
            let stderr = StreamCapture::default().finish();
            let mut details = Map::new();
            details.insert("task".to_string(), json!(name));
            details.insert("state".to_string(), json!("failed"));
            details.insert("exitCode".to_string(), Value::Null);
            details.insert("signal".to_string(), Value::Null);
            details.insert("stdout".to_string(), stdout.to_json());
            details.insert("stderr".to_string(), stderr.to_json());
            details.insert("spawnCode".to_string(), json!(spawn_code(&error)));
            // Node の非同期 `error` event + `close` 経路相当:
            // `"managed process failed" + details.spawnCode`(cause なし)。
            if spawn_failure_is_async(&error) {
                return Err(CheaperError::new(
                    ErrorCode::ToolExecutionFailure,
                    "managed process failed",
                )
                .with_details(details)
                .into());
            }
            // Node の sync throw 経路相当(``catch { spawn failed, cause }``)。
            return Err(CheaperError::new(
                ErrorCode::ToolExecutionFailure,
                "managed process spawn failed",
            )
            .with_details(details)
            .with_cause(error)
            .into());
        }
    };

    let stdout_pipe = child.stdout.take().expect("stdout piped");
    let stderr_pipe = child.stderr.take().expect("stderr piped");
    let stdout_task = tokio::spawn(read_stream(
        stdout_pipe,
        StreamKind::Stdout,
        Arc::clone(&captures),
    ));
    let stderr_task = tokio::spawn(read_stream(
        stderr_pipe,
        StreamKind::Stderr,
        Arc::clone(&captures),
    ));

    let phase = Arc::new(AtomicU8::new(PHASE_RUNNING));
    let diagnostics = Arc::new(Diagnostics {
        task: name.to_string(),
        pid: child.id(),
        started: Instant::now(),
        phase: Arc::clone(&phase),
        captures: Arc::clone(&captures),
    });

    // `setInterval(diagnostic("process.heartbeat"), interval)`。
    let heartbeat_interval_ms = parse_duration(&options.process.heartbeat.interval)?;
    let heartbeat_stop = CancellationToken::new();
    let heartbeat = {
        let diagnostics = Arc::clone(&diagnostics);
        let stop = heartbeat_stop.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_millis(heartbeat_interval_ms));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // tokio interval の初回 tick は即時発火するため 1 回捨てる
            // (TS `setInterval` は interval 経過後に初回発火)。
            interval.tick().await;
            loop {
                tokio::select! {
                    _ = interval.tick() => diagnostics.emit("process.heartbeat"),
                    _ = stop.cancelled() => break,
                }
            }
        })
    };

    let result = drive(
        options,
        name,
        &outer,
        &inner,
        &timeout_controller,
        &mut child,
        Arc::clone(&captures),
        Arc::clone(&diagnostics),
        phase,
        stdout_task,
        stderr_task,
    )
    .await;

    // `finally`: heartbeat/clearTimeout/readers 解放。
    heartbeat_stop.cancel();
    let _ = heartbeat.await;
    result
}
