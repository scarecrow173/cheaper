//! shared managed-subprocess lifecycle 機構
//! (`docs/designs/policy-gated-command-tool.md` §(e))。
//!
//! `tools/process/` から抽出した tool 非依存の managed process 実行基盤:
//! spawn(stdin ignore/stdout-stderr pipe/env_clear/windowsHide/POSIX
//! detached)・bounded combined capture・timeout+`onTimeout`・heartbeat
//! stderr JSON・process tree cleanup・minimal env(Windows required env
//! 空値遮断・SystemRoot 補完)・cwd/denylist prepare・native executable
//! 解決を集約する。task 固有 glue(manifest/writeScope coverage 検査・
//! details identity)は `tools::process`/`tools::command` 側が組み立て、
//! [`run`] が orchestration(timeout controller、captures、heartbeat
//! spawn、`command.spawn()`、`drive`)を担う。

pub mod executable;
pub mod tree;

mod cleanup;
mod monitor;
mod spawn;
#[cfg(test)]
mod tests;

use std::sync::atomic::AtomicU8;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cheaper_core::errors::{node_io_error, CheaperError, ErrorCode};
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{resolve_workspace_path, SymlinkPolicy};
use serde_json::{json, Map, Value};
use tokio_util::sync::CancellationToken;

use crate::catalog::CanonicalToolResult;
use crate::operation::{AbortController, AbortReason, AbortSignal};
use crate::ToolError;

use monitor::{
    drive, read_stream, Captures, Diagnostics, StreamCapture, StreamKind, PHASE_RUNNING,
};
use spawn::{build_command, spawn_code, spawn_failure_is_async};

pub use monitor::StreamCaptureResult;

// ---------------------------------------------------------------------------
// abort helpers(canonical `crate::operation` 型)
// ---------------------------------------------------------------------------

/// `signal.reason` の rethrow 相当。`AbortReason` 自体を error として包む
/// — `source()` 経由で error object(`CheaperError` 等)が辿れるため
/// `find_cheaper_error` の code 判定がそのまま効く。
pub(crate) fn aborted_err(reason: AbortReason) -> ToolError {
    Box::new(reason)
}

/// `AbortSignal.any` 相当: どちらかが abort したらその reason で解決。
/// 両方済みの場合は outer を優先する(発生順は区別不能)。
pub(crate) async fn wait_aborted(outer: &AbortSignal, inner: &AbortSignal) -> AbortReason {
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

pub(crate) fn combined_reason(outer: &AbortSignal, inner: &AbortSignal) -> Option<AbortReason> {
    outer.reason().or_else(|| inner.reason())
}

/// `ToolError`(Box<dyn Error>)を `CheaperError::with_cause`(sized
/// `impl Error` bound)へ渡すための wrapper(TS `cause: error` parity)。
#[derive(Debug)]
pub(crate) struct BoxedCause(Arc<dyn std::error::Error + Send + Sync + 'static>);

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

/// `onTimeout(reason)` callback type。Rust では reason を `&CheaperError`
/// で渡す(reason object を呼出し側が再送しない前提)。
pub type OnTimeout = Arc<dyn Fn(&CheaperError) + Send + Sync>;

/// 全量 output tee sink(design §proc の全量 output tee、R26) —
/// bounded capture とは別に stdout/stderr 両 stream の raw bytes を
/// chunk 到着順に interleaved で受け取る。sink 内の write 失敗は
/// closure が自身の状態へ記録する(postprocessFailure 縮退、R29)
/// — tee が capture を失敗させることはない。
pub type ProcTee = Arc<dyn Fn(&[u8]) + Send + Sync>;

// ---------------------------------------------------------------------------
// run の入力: prepared / limits / identity
// ---------------------------------------------------------------------------

/// `run` の lifecycle limit。duration 文字列は呼出し側で `parse_duration`
/// 済み(policy resolve が検証済みのため到達しない想定)の ms 値を取る。
/// `max_output_bytes` は combined stdout+stderr capture の cap、
/// `heartbeat_interval_ms` は `setInterval(diagnostic)` 相当の周期。
pub struct ProcLimits {
    pub timeout_ms: u64,
    pub kill_grace_ms: u64,
    pub max_output_bytes: u64,
    pub heartbeat_interval_ms: u64,
    /// 完了扱いにする exit code 集合。`None` は全 completed exit を
    /// 受理する(route scope の `accept_any_exit_code`)。signal kill
    /// 等 `exit_code` を持たない終了は常に失敗扱い。
    pub accepted_exit_codes: Option<Vec<i64>>,
}

/// 解決済みの spawn 入力。`env` は `env_clear` 後に設定する最小 env
/// (`minimal_env` 等で構築済み)。
pub struct ProcPrepared {
    pub executable: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub env: Vec<(String, String)>,
}

/// details・display text・diagnostic へ差し込む呼出し側 identity。
/// `process` は `{details: {task: name}, label: name, display:
/// "process {name}", event_prefix: "process"}` を渡して従来出力と同一にする。
/// `command`(T3)は `{grant: name, argv: args}` 相当を差し込む。
pub struct ProcIdentity {
    /// details map へ先頭挿入する identity field。
    pub details: Vec<(String, Value)>,
    /// diagnostics emit の `task` field 値(task/grant 名)。
    pub label: String,
    /// 成功 result text の先頭句(`"process {name}"` 等)。
    pub display: String,
    /// diagnostics event 名の prefix(`"process"` → `"process.heartbeat"`)。
    pub event_prefix: &'static str,
    /// details に `durationMs`(開始からの経過 ms)を載せるか。
    /// `command` details schema が `durationMs` を required とする
    /// spec-mandated 差分(design §(e))のため `command` のみ true —
    /// `process` は false で従来 details を不変に保つ(AC1)。
    pub report_duration_ms: bool,
}

// ---------------------------------------------------------------------------
// env / cwd prepare(shared、declaration 非依存)
// ---------------------------------------------------------------------------

/// `minimalEnv(explicit)`: 宣言 env のみ + Windows required env 空値遮断 +
/// SystemRoot 補完。`NODE_V8_COVERAGE: undefined` は spawn 側の
/// `env_clear` で「未設定」を再現するためここでは未出力(TS の own
/// undefined = 継承遮断の意味と同一)。
pub(crate) fn minimal_env(explicit: &Map<String, Value>) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    for (key, value) in explicit {
        let name = if cfg!(windows) {
            key.to_uppercase()
        } else {
            key.clone()
        };
        env.push((name, value.as_str().unwrap_or_default().to_string()));
    }
    #[cfg(windows)]
    {
        // libuv は未指定の Windows required env を親から補完する。空値で
        // その継承を止める。
        const REQUIRED: &[&str] = &[
            "HOMEDRIVE",
            "HOMEPATH",
            "LOGONSERVER",
            "PATH",
            "SYSTEMDRIVE",
            "TEMP",
            "USERDOMAIN",
            "USERNAME",
            "USERPROFILE",
            "WINDIR",
        ];
        let missing: Vec<&str> = REQUIRED
            .iter()
            .copied()
            .filter(|key| !env.iter().any(|(k, _)| k == key))
            .collect();
        for key in missing {
            env.push((key.to_string(), String::new()));
        }
        if !env.iter().any(|(k, _)| k == "SYSTEMROOT") {
            env.push((
                "SYSTEMROOT".to_string(),
                std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string()),
            ));
        }
    }
    env
}

/// declared path の deny error。`noun` は message の宣言種別
/// (`"process"` → `"Access denied for declared process path"`)。
pub(crate) fn denied_declared_path(display: &str, noun: &str) -> ToolError {
    let mut details = Map::new();
    details.insert("path".to_string(), json!(display));
    CheaperError::new(
        ErrorCode::PolicyViolation,
        format!("Access denied for declared {noun} path"),
    )
    .with_details(details)
    .into()
}

/// cwd 解決の共有部分: `resolve_workspace_path` → denylist → `is_dir`
/// stat。`noun` は error message の宣言種別(`process` は `"process"` を
/// 渡して従来 message を維持する)。manifest/writeScope coverage 検査は
/// 呼出し側(process glue)の責務。
pub(crate) fn prepare_cwd(
    root: &str,
    cwd: &str,
    symlink_policy: SymlinkPolicy,
    denylist: Option<&PathDenylist>,
    noun: &str,
) -> Result<String, ToolError> {
    let resolved = resolve_workspace_path(root, cwd, symlink_policy)?;
    if let Some(denylist) = denylist {
        if denylist.is_denied(&resolved.display) {
            return Err(denied_declared_path(&resolved.display, noun));
        }
    }
    // `stat` 自体の失敗は Node `UVException` shape で伝播する(TS は
    // `fs.stat` の ENOENT をそのまま throw)。isDirectory=false のみ
    // INVALID_REQUEST。
    let stat = std::fs::metadata(&resolved.real)
        .map_err(|e| Box::new(node_io_error("stat", &resolved.real, e)) as ToolError)?;
    if !stat.is_dir() {
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("{noun} cwd must be an existing directory"),
        )
        .into());
    }
    Ok(resolved.real)
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

/// execute orchestration 相当: timeout controller・captures 構築・
/// `command.spawn()`・stream reader・heartbeat spawn・`drive`・finally
/// (heartbeat 停止)。spawn 失敗の details/error shape は呼出し側の
/// `identity.details` + `{state, exitCode, signal, stdout, stderr,
/// spawnCode}` で従来と同一。
pub async fn run(
    prepared: &ProcPrepared,
    limits: &ProcLimits,
    identity: &ProcIdentity,
    outer: AbortSignal,
    on_timeout: Option<&OnTimeout>,
    tee: Option<ProcTee>,
) -> Result<CanonicalToolResult, ToolError> {
    // `durationMs` の測定起点 — spawn 失敗 details にも載せるため
    // spawn 前に取る(`identity.report_duration_ms` が true の場合のみ
    // 挿入される)。
    let started = Instant::now();
    // `AbortSignal.any([outerSignal, timeoutController.signal])` 相当。
    let timeout_controller = AbortController::new();
    let inner = timeout_controller.signal();
    if let Some(reason) = combined_reason(&outer, &inner) {
        return Err(aborted_err(reason));
    }

    let captures = Arc::new(Mutex::new(Captures {
        stdout: StreamCapture::default(),
        stderr: StreamCapture::default(),
        remaining: limits.max_output_bytes,
        tee,
    }));

    let mut command = build_command(prepared);
    let mut child = match command.spawn() {
        Ok(child) => {
            tracing::debug!(target: "cheaper.tool.process", task = %identity.label, "process spawn");
            child
        }
        Err(error) => {
            tracing::debug!(
                target: "cheaper.tool.process",
                task = %identity.label,
                error.code = spawn_code(&error),
                "process spawn failed"
            );
            let stdout = StreamCapture::default().finish();
            let stderr = StreamCapture::default().finish();
            let mut details = Map::new();
            for (key, value) in &identity.details {
                details.insert(key.clone(), value.clone());
            }
            details.insert("state".to_string(), json!("failed"));
            details.insert("exitCode".to_string(), Value::Null);
            details.insert("signal".to_string(), Value::Null);
            details.insert("stdout".to_string(), stdout.to_json());
            details.insert("stderr".to_string(), stderr.to_json());
            details.insert("spawnCode".to_string(), json!(spawn_code(&error)));
            if identity.report_duration_ms {
                details.insert(
                    "durationMs".to_string(),
                    json!(started.elapsed().as_millis() as u64),
                );
            }
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
        label: identity.label.clone(),
        event_prefix: identity.event_prefix,
        pid: child.id(),
        started: Instant::now(),
        phase: Arc::clone(&phase),
        captures: Arc::clone(&captures),
    });

    // `setInterval(diagnostic("{prefix}.heartbeat"), interval)`。
    let heartbeat_interval_ms = limits.heartbeat_interval_ms;
    let heartbeat_event = format!("{}.heartbeat", identity.event_prefix);
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
                    _ = interval.tick() => diagnostics.emit(&heartbeat_event),
                    _ = stop.cancelled() => break,
                }
            }
        })
    };

    let result = drive(
        limits,
        identity,
        &outer,
        &inner,
        &timeout_controller,
        &mut child,
        Arc::clone(&captures),
        Arc::clone(&diagnostics),
        phase,
        stdout_task,
        stderr_task,
        on_timeout,
    )
    .await;

    // `finally`: heartbeat/clearTimeout/readers 解放。
    heartbeat_stop.cancel();
    let _ = heartbeat.await;
    result
}
