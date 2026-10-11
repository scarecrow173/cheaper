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
use std::io::Write;
use std::process::Stdio;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cheaper_core::duration::parse_duration;
use cheaper_core::errors::{node_io_error, CheaperError, ErrorCode};
use cheaper_core::nodepath;
/// `MAX_PROCESS_OUTPUT_BYTES` re-export(TS `export { ... }` parity)。
pub use cheaper_core::policy::process::MAX_PROCESS_OUTPUT_BYTES;
use cheaper_core::policy::process::{
    process_task_resources, resolve_process_task, ProcessTask, ResolvedProcessPolicy,
};
use cheaper_core::resources::normalization::{
    normalize_resource_manifest, ResourceManifestDeclaration, ScopeDeclaration,
};
use cheaper_core::resources::predicates::claim_contains;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{resolve_workspace_path, SymlinkPolicy};
use serde_json::{json, Map, Value};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use self::executable::{resolve_process_executable, ProcessExecutableOptions};
use self::tree::{create_process_tree, ProcessTree};
use crate::catalog::{CanonicalToolResult, ToolContent};
use crate::operation::{AbortController, AbortReason, AbortSignal};
use crate::scheduler::{OperationDeclaration, OperationPool};
use crate::ToolError;

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

/// `processSchema`(TypeBox 生成物を literal JSON 化)。
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
// minimal env / prepare
// ---------------------------------------------------------------------------

/// `minimalEnv(explicit)`: task.env のみ + Windows required env 空値遮断 +
/// SystemRoot 補完。`NODE_V8_COVERAGE: undefined` は spawn 側の
/// `env_clear` で「未設定」を再現するためここでは未出力(TS の own
/// undefined = 継承遮断の意味と同一)。
fn minimal_env(explicit: &Map<String, Value>) -> Vec<(String, String)> {
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

fn deny(display: &str) -> ToolError {
    let mut details = Map::new();
    details.insert("path".to_string(), json!(display));
    CheaperError::new(
        ErrorCode::PolicyViolation,
        "Access denied for declared process path",
    )
    .with_details(details)
    .into()
}

struct Prepared {
    executable: String,
    cwd: String,
}

fn declaration_value(declaration: &ResourceManifestDeclaration) -> Value {
    let scopes = |items: &Option<Vec<ScopeDeclaration>>| -> Value {
        json!(items
            .iter()
            .flatten()
            .map(|s| json!({"path": s.path, "scope": s.scope.as_str()}))
            .collect::<Vec<_>>())
    };
    let mut out = Map::new();
    if declaration.reads.is_some() {
        out.insert("reads".to_string(), scopes(&declaration.reads));
    }
    if declaration.writes.is_some() {
        out.insert("writes".to_string(), scopes(&declaration.writes));
    }
    if declaration.produces.is_some() {
        out.insert("produces".to_string(), scopes(&declaration.produces));
    }
    Value::Object(out)
}

/// `prepare`: named task → workspace cwd 解決 + deny + isDirectory →
/// manifest normalize → 宣言 path/claim の deny → writeScope 検査 →
/// executable 解決。
fn prepare(
    options: &ProcessToolOptions,
    name: &str,
    task: &ProcessTask,
) -> Result<Prepared, ToolError> {
    let cwd = resolve_workspace_path(options.root.as_str(), &task.cwd, options.symlink_policy)?;
    if let Some(denylist) = &options.denylist {
        if denylist.is_denied(&cwd.display) {
            return Err(deny(&cwd.display));
        }
    }
    // `stat` 自体の失敗は Node `UVException` shape で伝播する(TS は
    // `fs.stat` の ENOENT をそのまま throw)。isDirectory=false のみ
    // INVALID_REQUEST。
    let cwd_stat = std::fs::metadata(&cwd.real)
        .map_err(|e| Box::new(node_io_error("stat", &cwd.real, e)) as ToolError)?;
    if !cwd_stat.is_dir() {
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            "process cwd must be an existing directory",
        )
        .into());
    }
    let declaration = process_task_resources(&options.process, name)?;
    let manifest = normalize_resource_manifest(
        &options.root,
        Some(&declaration_value(&declaration)),
        options.symlink_policy,
    )?;
    for input in task.inputs.iter().chain(&task.outputs) {
        if let Some(denylist) = &options.denylist {
            if denylist.is_denied(input) {
                return Err(deny(input));
            }
        }
    }
    for claim in manifest
        .reads
        .iter()
        .chain(&manifest.writes)
        .chain(&manifest.produces)
    {
        let rel = nodepath::relative(&options.root, claim.resource.path());
        let display = if rel.is_empty() {
            ".".to_string()
        } else {
            rel.split(nodepath::SEP).collect::<Vec<_>>().join("/")
        };
        if let Some(denylist) = &options.denylist {
            if denylist.is_denied(&display) {
                return Err(deny(&display));
            }
        }
    }
    if let Some(write_scope) = &options.write_scope {
        let allowed = normalize_resource_manifest(
            &options.root,
            Some(&json!({
                "reads": [],
                "writes": [],
                "produces": write_scope
                    .iter()
                    .map(|value| json!({"path": value, "scope": "exact"}))
                    .collect::<Vec<_>>(),
            })),
            options.symlink_policy,
        )?;
        let uncovered = manifest.writes.iter().any(|claim| {
            !allowed
                .produces
                .iter()
                .any(|scope| claim_contains(scope, claim))
        });
        if uncovered {
            return Err(CheaperError::new(
                ErrorCode::PolicyViolation,
                "process outputs exceed worker writeScope",
            )
            .into());
        }
    }
    let command = task
        .command
        .first()
        .ok_or_else(|| unsupported_empty_command())?;
    let executable = resolve_process_executable(
        command,
        &ProcessExecutableOptions {
            root: &options.root,
            symlink_policy: options.symlink_policy,
            denylist: options.denylist.as_ref(),
            search_path: None,
        },
    )?;
    Ok(Prepared {
        executable,
        cwd: cwd.real,
    })
}

fn unsupported_empty_command() -> ToolError {
    CheaperError::new(
        ErrorCode::PolicyViolation,
        "process requires a supported native executable; shell and script shims are unsupported",
    )
    .into()
}

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
    fn to_json(&self) -> Value {
        json!({
            "text": self.text,
            "bytes": self.bytes,
            "capturedBytes": self.captured_bytes,
            "truncated": self.truncated,
        })
    }
}

#[derive(Default)]
struct StreamCapture {
    chunks: Vec<Vec<u8>>,
    bytes: u64,
    captured_bytes: u64,
}

impl StreamCapture {
    /// `append(chunk, remaining)` → 実 captured 量を返す。
    fn append(&mut self, chunk: &[u8], remaining: u64) -> u64 {
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
    fn finish(self) -> StreamCaptureResult {
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
enum StreamKind {
    Stdout,
    Stderr,
}

struct Captures {
    stdout: StreamCapture,
    stderr: StreamCapture,
    remaining: u64,
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
async fn read_stream<R: tokio::io::AsyncRead + Unpin>(
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

const PHASE_RUNNING: u8 = 0;
const PHASE_CLEANUP: u8 = 1;

struct Diagnostics {
    task: String,
    pid: Option<u32>,
    started: Instant,
    phase: Arc<AtomicU8>,
    captures: Arc<Mutex<Captures>>,
}

impl Diagnostics {
    /// `diagnostic(event)`: heartbeat/cleanup-failed を stderr JSON 1 行で。
    /// 診断の失敗は child の結果を変えない。
    fn emit(&self, event: &str) {
        let output_bytes = self.captures.lock().map(|c| c.output_bytes()).unwrap_or(0);
        let phase = if self.phase.load(Ordering::Relaxed) == PHASE_RUNNING {
            "running"
        } else {
            "cleanup"
        };
        let line = json!({
            "event": event,
            "task": self.task,
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
fn spawn_code(e: &std::io::Error) -> &'static str {
    if let Some(code) = e.raw_os_error() {
        #[cfg(unix)]
        let name = match code {
            libc::EPERM => Some("EPERM"),
            libc::ENOENT => Some("ENOENT"),
            libc::EINTR => Some("EINTR"),
            libc::EIO => Some("EIO"),
            libc::ENXIO => Some("ENXIO"),
            libc::E2BIG => Some("E2BIG"),
            libc::ENOEXEC => Some("ENOEXEC"),
            libc::EBADF => Some("EBADF"),
            libc::EAGAIN => Some("EAGAIN"),
            libc::ENOMEM => Some("ENOMEM"),
            libc::EACCES => Some("EACCES"),
            libc::EFAULT => Some("EFAULT"),
            libc::EBUSY => Some("EBUSY"),
            libc::EEXIST => Some("EEXIST"),
            libc::ENODEV => Some("ENODEV"),
            libc::ENOTDIR => Some("ENOTDIR"),
            libc::EISDIR => Some("EISDIR"),
            libc::EINVAL => Some("EINVAL"),
            libc::ENFILE => Some("ENFILE"),
            libc::EMFILE => Some("EMFILE"),
            libc::ETXTBSY => Some("ETXTBSY"),
            libc::ENOSPC => Some("ENOSPC"),
            libc::EROFS => Some("EROFS"),
            libc::EPIPE => Some("EPIPE"),
            libc::ENAMETOOLONG => Some("ENAMETOOLONG"),
            libc::ELOOP => Some("ELOOP"),
            _ => None,
        };
        #[cfg(windows)]
        let name = match code {
            2 | 3 | 161 => Some("ENOENT"), // FILE_NOT_FOUND/PATH_NOT_FOUND/BAD_PATHNAME
            4 => Some("EMFILE"),           // TOO_MANY_OPEN_FILES
            5 | 32 | 82 => Some("EACCES"), // ACCESS_DENIED/SHARING_VIOLATION/CANNOT_MAKE
            6 => Some("EBADF"),            // INVALID_HANDLE
            8 | 14 => Some("ENOMEM"),      // NOT_ENOUGH_MEMORY / OUTOFMEMORY
            87 => Some("EINVAL"),          // INVALID_PARAMETER
            193 => Some("EFTYPE"),         // BAD_EXE_FORMAT(Node 実測 "EFTYPE")
            267 => Some("ENOTDIR"),        // DIRECTORY
            _ => None,
        };
        #[cfg(not(any(unix, windows)))]
        let name: Option<&'static str> = None;
        if let Some(name) = name {
            return name;
        }
        if e.raw_os_error().is_some() && cfg!(windows) {
            // libuv は unmapped win32 error を `UV__UNKNOWN` に写す。
            return "UNKNOWN";
        }
    }
    match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT",
        std::io::ErrorKind::PermissionDenied => "EACCES",
        _ => "SPAWN_ERROR",
    }
}

/// spawn 失敗の message shape。Node は `ChildProcess.spawn` が
/// uv_spawn の sync error で throw する経路と、`error` event が非同期で
/// 届く経路を持つ:
///
/// - unix: `fork`+`exec` 失敗は error pipe 経由で常に非同期 — raw errno
///   を持つ失敗は全て `"managed process failed"` + `spawnCode`。
/// - win32: `uv_spawn` が `CreateProcessW` 失敗を同期返すため
///   `"managed process spawn failed"` + `cause` 相当になるが、ENOENT
///   だけは Node が非同期 `error` event として扱う(実測)ため
///   `"managed process failed"` + `spawnCode: "ENOENT"` に揃える。
/// - `raw_os_error` を持たない構築 error(引数検証等)は sync throw 経路
///   の `"managed process spawn failed"`。
///
/// 残留差分: musl 環境では Node が ENOEXEC を同期 throw し得る
/// (IR doc 参照)。
fn spawn_failure_is_async(e: &std::io::Error) -> bool {
    e.raw_os_error().is_some() && (cfg!(unix) || spawn_code(e) == "ENOENT")
}

/// unix `ExitStatus::signal()` → Node signal 名。
#[cfg(unix)]
fn signal_name(sig: i32) -> Option<String> {
    Some(
        match sig {
            libc::SIGHUP => "SIGHUP",
            libc::SIGINT => "SIGINT",
            libc::SIGQUIT => "SIGQUIT",
            libc::SIGILL => "SIGILL",
            libc::SIGTRAP => "SIGTRAP",
            libc::SIGABRT => "SIGABRT",
            libc::SIGBUS => "SIGBUS",
            libc::SIGFPE => "SIGFPE",
            libc::SIGKILL => "SIGKILL",
            libc::SIGUSR1 => "SIGUSR1",
            libc::SIGSEGV => "SIGSEGV",
            libc::SIGUSR2 => "SIGUSR2",
            libc::SIGPIPE => "SIGPIPE",
            libc::SIGALRM => "SIGALRM",
            libc::SIGTERM => "SIGTERM",
            libc::SIGCHLD => "SIGCHLD",
            libc::SIGCONT => "SIGCONT",
            libc::SIGSTOP => "SIGSTOP",
            libc::SIGTSTP => "SIGTSTP",
            libc::SIGTTIN => "SIGTTIN",
            libc::SIGTTOU => "SIGTTOU",
            libc::SIGURG => "SIGURG",
            libc::SIGXCPU => "SIGXCPU",
            libc::SIGXFSZ => "SIGXFSZ",
            libc::SIGVTALRM => "SIGVTALRM",
            libc::SIGPROF => "SIGPROF",
            libc::SIGWINCH => "SIGWINCH",
            libc::SIGIO => "SIGIO",
            _ => return None,
        }
        .to_string(),
    )
}

fn status_parts(status: &std::process::ExitStatus) -> (Option<i64>, Option<String>) {
    if let Some(code) = status.code() {
        return (Some(code as i64), None);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return (None, signal_name(sig));
        }
    }
    (None, None)
}

fn build_command(prepared: &Prepared, task: &ProcessTask, env: Vec<(String, String)>) -> Command {
    let mut command = Command::new(&prepared.executable);
    command
        .args(&task.command[1..])
        .current_dir(&prepared.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    for (key, value) in env {
        command.env(key, value);
    }
    #[cfg(windows)]
    {
        // `creation_flags` は tokio::process::Command の inherent method。
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW(windowsHide)
    }
    #[cfg(unix)]
    {
        // `detached: true`(非 win32)相当: 新 process group で
        // `kill(-pid)` 対象になる。
        command.process_group(0);
    }
    command
}

/// `display(value)`: 表示だけを 64 UTF-16 code units で折り返す。raw
/// stdout/stderr は details に保持し、同期 BPE の長い run を防ぐ。
/// 補助面 char が境界を跨がない(TS surrogate pair backoff parity)。
fn display(value: &StreamCaptureResult) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut units = 0usize;
    for ch in value.text.chars() {
        let u = ch.len_utf16();
        if units + u > 64 && !current.is_empty() {
            parts.push(std::mem::take(&mut current));
            units = 0;
        }
        current.push(ch);
        units += u;
    }
    if !current.is_empty() {
        parts.push(current);
    }
    format!(
        "{} bytes, {} captured, truncated={}\n{}",
        value.bytes,
        value.captured_bytes,
        value.truncated,
        parts.join("\n")
    )
}

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
        Ok(child) => child,
        Err(error) => {
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

#[allow(clippy::too_many_arguments)]
async fn drive(
    options: &ProcessToolOptions,
    name: &str,
    outer: &AbortSignal,
    inner: &AbortSignal,
    timeout_controller: &AbortController,
    child: &mut tokio::process::Child,
    captures: Arc<Mutex<Captures>>,
    diagnostics: Arc<Diagnostics>,
    phase: Arc<AtomicU8>,
    stdout_task: JoinHandle<()>,
    stderr_task: JoinHandle<()>,
) -> Result<CanonicalToolResult, ToolError> {
    // `parseDuration` の RangeError はそのまま伝播する(policy resolve で
    // 検証済みのため到達しない想定)。
    let timeout_ms = parse_duration(&options.process.timeout)?;
    let kill_grace_ms = parse_duration(&options.process.kill_grace)?;
    let mut tree: Option<ProcessTree> = child.id().map(create_process_tree);
    let mut cleanup: Option<JoinHandle<Result<(), ToolError>>> = None;
    let mut timeout_sleep = Box::pin(sleep(std::time::Duration::from_millis(timeout_ms)));
    let mut timeout_fired = false;

    let start_cleanup =
        |tree: &mut Option<ProcessTree>,
         cleanup: &mut Option<JoinHandle<Result<(), ToolError>>>| {
            phase.store(PHASE_CLEANUP, Ordering::Relaxed);
            let t = tree.take();
            *cleanup = Some(tokio::spawn(async move {
                match t {
                    Some(mut tree) => tree.terminate(kill_grace_ms).await,
                    None => Ok(()),
                }
            }));
        };

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
                    details.insert("task".to_string(), json!(name));
                    let reason = CheaperError::new(
                        ErrorCode::Timeout,
                        "managed process execution timed out",
                    )
                    .with_details(details);
                    if let Some(on_timeout) = &options.on_timeout {
                        on_timeout(&reason);
                    }
                    timeout_controller.abort(AbortReason::from(reason));
                }
            }
            _ = wait_aborted(outer, inner), if cleanup.is_none() => {
                start_cleanup(&mut tree, &mut cleanup);
            }
        }
    };
    // 'exit' 相当: 通常終了でも descendants cleanup を走らせる。
    if cleanup.is_none() {
        start_cleanup(&mut tree, &mut cleanup);
    }
    let status = ended?;
    // 'close' 相当: streams の EOF まで待つ。
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    let mut cleanup_error: Option<ToolError> = None;
    if let Some(handle) = cleanup {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => cleanup_error = Some(e),
            Err(e) => cleanup_error = Some(e.into()),
        }
    }
    if cleanup_error.is_some() {
        diagnostics.emit("process.cleanup-failed");
        let _ = child.start_kill();
    }

    let (stdout, stderr) = {
        let mut locked = captures.lock().expect("captures");
        (
            std::mem::take(&mut locked.stdout).finish(),
            std::mem::take(&mut locked.stderr).finish(),
        )
    };
    let (exit_code, signal) = status_parts(&status);
    let state = if exit_code == Some(0) {
        "completed"
    } else {
        "failed"
    };
    let mut details = Map::new();
    details.insert("task".to_string(), json!(name));
    details.insert("state".to_string(), json!(state));
    details.insert(
        "exitCode".to_string(),
        exit_code.map_or(Value::Null, |c| json!(c)),
    );
    details.insert(
        "signal".to_string(),
        signal.map_or(Value::Null, |s| json!(s)),
    );
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
        "process {name}: completed, exitCode=0\nstdout: {}\nstderr: {}",
        display(&stdout),
        display(&stderr),
    );
    Ok(CanonicalToolResult {
        content: vec![ToolContent::text(text)],
        details: Some(details),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn capture_finish_decodes_within_byte_cap() {
        let mut capture = StreamCapture::default();
        capture.append(&[b'a'; 10], 6);
        capture.append(&[b'b'; 10], 4);
        let done = capture.finish();
        assert_eq!(done.text, "aaaaaabbbb");
        assert_eq!(done.bytes, 20);
        assert_eq!(done.captured_bytes, 10);
        assert!(done.truncated);
    }

    #[test]
    fn capture_finish_drops_incomplete_trailing_codepoint() {
        // stream:true decode は不完全な末尾 sequence を捨てる
        // (replacement 文字すら出さない)。
        let mut capture = StreamCapture::default();
        let input = "日本語".as_bytes(); // 9 bytes / 3 chars
        capture.append(input, 7); // "日本" + 語 の先頭 1 byte
        let done = capture.finish();
        assert_eq!(done.text, "日本");
        assert!(done.truncated);
    }

    #[test]
    fn capture_finish_marks_replacement_expansion_as_truncated() {
        let mut capture = StreamCapture::default();
        capture.append(&[0xff, 0xff], 2); // invalid → U+FFFD に拡大
        let done = capture.finish();
        assert_eq!(done.bytes, 2);
        assert_eq!(done.captured_bytes, 2);
        assert!(done.truncated); // encoded(3) > captured(2)
    }

    #[test]
    fn display_folds_at_64_utf16_units() {
        let text = "a".repeat(130);
        let value = StreamCaptureResult {
            text,
            bytes: 130,
            captured_bytes: 130,
            truncated: false,
        };
        let shown = display(&value);
        let body = shown.split('\n').skip(1).collect::<Vec<_>>();
        assert_eq!(body.len(), 3);
        assert_eq!(body[0].len(), 64);
        assert_eq!(body[1].len(), 64);
        assert_eq!(body[2].len(), 2);
        assert!(shown.starts_with("130 bytes, 130 captured, truncated=false\n"));
    }

    #[test]
    fn display_does_not_split_surrogate_pair() {
        // 63 ascii + 1 BMP外 char(2 units)= 65 units → pair は次行へ。
        let text = format!("{}😀x", "a".repeat(63));
        let value = StreamCaptureResult {
            text,
            bytes: 0,
            captured_bytes: 0,
            truncated: false,
        };
        let shown = display(&value);
        let body: Vec<&str> = shown.split('\n').skip(1).collect();
        assert_eq!(body[0].chars().count(), 63);
        assert_eq!(body[1], "😀x");
    }

    #[test]
    fn minimal_env_keeps_only_explicit_on_posix() {
        if cfg!(windows) {
            return;
        }
        let mut explicit = Map::new();
        explicit.insert("ONLY_THIS".to_string(), json!("yes"));
        let env = minimal_env(&explicit);
        assert_eq!(env, vec![("ONLY_THIS".to_string(), "yes".to_string())]);
    }

    #[cfg(windows)]
    #[test]
    fn minimal_env_windows_required_env_and_systemroot() {
        let mut explicit = Map::new();
        explicit.insert("path".to_string(), json!("custom")); // → PATH へ upper
        explicit.insert("only_this".to_string(), json!("yes"));
        let env = minimal_env(&explicit);
        let get = |name: &str| env.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
        // 明示 env は upper-case 化される。
        assert_eq!(get("PATH"), Some("custom"));
        assert_eq!(get("ONLY_THIS"), Some("yes"));
        // required env は未指定時に空値で遮断される(PATH は明示済み)。
        for key in [
            "HOMEDRIVE",
            "HOMEPATH",
            "LOGONSERVER",
            "SYSTEMDRIVE",
            "TEMP",
            "USERDOMAIN",
            "USERNAME",
            "USERPROFILE",
            "WINDIR",
        ] {
            assert_eq!(get(key), Some(""), "{key}");
        }
        // SystemRoot は親値 or "C:\Windows" で補完。
        let expected = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        assert_eq!(get("SYSTEMROOT"), Some(expected.as_str()));
    }

    #[test]
    fn operation_declaration_derives_access_from_outputs() {
        let policy = cheaper_core::policy::process::resolve_process_policy(Some(&json!({
            "maxConcurrent": 3,
            "tasks": {
                "ro": {"command": ["cc"], "inputs": [], "outputs": []},
                "wr": {"command": ["cc"], "inputs": [], "outputs": ["out"]},
            },
        })))
        .unwrap();
        let ro = process_operation_declaration(&policy, "ro").unwrap();
        assert_eq!(ro.access, cheaper_core::resources::types::ClaimMode::Shared);
        let pool = ro.pool.as_ref().expect("process pool");
        assert_eq!(pool.name, "process");
        assert_eq!(pool.max_concurrent, 3);
        let wr = process_operation_declaration(&policy, "wr").unwrap();
        assert_eq!(
            wr.access,
            cheaper_core::resources::types::ClaimMode::Exclusive
        );
        let err = process_operation_declaration(&policy, "nope").unwrap_err();
        assert_eq!(err.code(), ErrorCode::PolicyViolation);
    }

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!(
                "cheaper-proc-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            std::fs::create_dir_all(&base).unwrap();
            Self(base)
        }

        fn root(&self) -> String {
            cheaper_core::workspace::paths::canonicalize_workspace_root(self.0.to_str().unwrap())
                .unwrap()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn policy(tasks: Value) -> ResolvedProcessPolicy {
        cheaper_core::policy::process::resolve_process_policy(Some(&json!({
            "tasks": tasks,
        })))
        .unwrap()
    }

    fn tool(root: String, tasks: Value) -> ProcessTool {
        create_process_tool(ProcessToolOptions {
            root,
            symlink_policy: SymlinkPolicy::Deny,
            process: policy(tasks),
            on_timeout: None,
            denylist: None,
            write_scope: None,
        })
    }

    fn cheaper_code(err: &ToolError) -> Option<ErrorCode> {
        find_cheaper_error(err.as_ref()).map(|e| e.code())
    }

    #[tokio::test]
    async fn unknown_task_is_policy_violation_before_spawn() {
        let dir = TempDir::new();
        let tool = tool(
            dir.root(),
            json!({"check": {"command": ["cc"], "inputs": [], "outputs": []}}),
        );
        let err = tool
            .execute(&json!({"task": "missing"}), None)
            .await
            .unwrap_err();
        assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
    }

    #[tokio::test]
    async fn prepare_rejects_missing_cwd() {
        let dir = TempDir::new();
        let tool = tool(
            dir.root(),
            json!({"check": {
                "command": ["cc"], "inputs": [], "outputs": [], "cwd": "missing",
            }}),
        );
        let err = tool
            .execute(&json!({"task": "check"}), None)
            .await
            .unwrap_err();
        // TS parity: `stat` の ENOENT は CheaperError で wrap せず raw
        // error で伝播する(isDirectory=false のみ INVALID_REQUEST)。
        assert_eq!(cheaper_code(&err), None);
        assert!(err.is::<std::io::Error>() || err.source().is_some());
    }

    #[tokio::test]
    async fn prepare_rejects_denied_output_and_write_scope_excess() {
        let dir = TempDir::new();
        let root = dir.root();
        // declared output が denylist に一致 → deny。
        let denylist = cheaper_core::workspace::deny_paths::create_path_denylist(&[".cheaper/**"]);
        let tool = create_process_tool(ProcessToolOptions {
            root: root.clone(),
            symlink_policy: SymlinkPolicy::Deny,
            process: policy(json!({"check": {
                "command": ["cc"], "inputs": [], "outputs": [".cheaper/out"],
            }})),
            on_timeout: None,
            denylist: Some(denylist),
            write_scope: None,
        });
        let err = tool
            .execute(&json!({"task": "check"}), None)
            .await
            .unwrap_err();
        assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
        // writeScope 外の output → POLICY_VIOLATION。
        let tool = create_process_tool(ProcessToolOptions {
            root,
            symlink_policy: SymlinkPolicy::Deny,
            process: policy(json!({"check": {
                "command": ["cc"], "inputs": [], "outputs": ["out"],
            }})),
            on_timeout: None,
            denylist: None,
            write_scope: Some(HashSet::from(["allowed".to_string()])),
        });
        let err = tool
            .execute(&json!({"task": "check"}), None)
            .await
            .unwrap_err();
        assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
    }

    /// `spawn_code`: raw OS errno → Node/libuv `error.code` 名の写像。
    /// unmapped errno と errno 無しの構築 error は kind fallback へ。
    #[test]
    fn spawn_code_maps_raw_errno_to_libuv_names() {
        #[cfg(unix)]
        {
            for (errno, want) in [
                (libc::ENOENT, "ENOENT"),
                (libc::EACCES, "EACCES"),
                (libc::ENOEXEC, "ENOEXEC"),
                (libc::E2BIG, "E2BIG"),
                (libc::ENXIO, "ENXIO"),
                (libc::ETXTBSY, "ETXTBSY"),
                // mapping に無い errno は kind fallback(ENOTSUP →
                // Uncategorized → SPAWN_ERROR)。
                (libc::ENOTSUP, "SPAWN_ERROR"),
            ] {
                assert_eq!(
                    spawn_code(&std::io::Error::from_raw_os_error(errno)),
                    want,
                    "errno {errno}"
                );
            }
        }
        #[cfg(windows)]
        {
            for (errno, want) in [
                (2, "ENOENT"),     // ERROR_FILE_NOT_FOUND(Node 実測 async error)
                (3, "ENOENT"),     // ERROR_PATH_NOT_FOUND
                (5, "EACCES"),     // ERROR_ACCESS_DENIED
                (32, "EACCES"),    // ERROR_SHARING_VIOLATION
                (193, "EFTYPE"),   // ERROR_BAD_EXE_FORMAT(Node 実測)
                (216, "UNKNOWN"),  // ERROR_EXE_MACHINE_TYPE_MISMATCH(実測)
                (267, "ENOTDIR"),  // ERROR_DIRECTORY
                (9999, "UNKNOWN"), // unmapped win32 → UV__UNKNOWN
            ] {
                assert_eq!(
                    spawn_code(&std::io::Error::from_raw_os_error(errno)),
                    want,
                    "errno {errno}"
                );
            }
        }
        // raw errno を持たない構築 error: kind による fallback。
        assert_eq!(
            spawn_code(&std::io::Error::new(std::io::ErrorKind::NotFound, "x")),
            "ENOENT"
        );
        assert_eq!(
            spawn_code(&std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "x"
            )),
            "EACCES"
        );
        assert_eq!(spawn_code(&std::io::Error::other("x")), "SPAWN_ERROR");
    }

    /// `spawn_failure_is_async`: unix は exec 失敗が error pipe 経由で
    /// 常に非同期(`"managed process failed"` shape)。win32 は ENOENT の
    /// み非同期(Node 実測)、その他は sync throw 経路。
    #[test]
    fn spawn_failure_async_shape_follows_node() {
        #[cfg(unix)]
        {
            for errno in [libc::ENOENT, libc::EACCES, libc::ENOEXEC] {
                assert!(spawn_failure_is_async(&std::io::Error::from_raw_os_error(
                    errno
                )));
            }
        }
        #[cfg(windows)]
        {
            assert!(spawn_failure_is_async(&std::io::Error::from_raw_os_error(
                2
            )));
            assert!(!spawn_failure_is_async(&std::io::Error::from_raw_os_error(
                216
            )));
            assert!(!spawn_failure_is_async(&std::io::Error::from_raw_os_error(
                193
            )));
            assert!(!spawn_failure_is_async(&std::io::Error::from_raw_os_error(
                5
            )));
        }
        assert!(!spawn_failure_is_async(&std::io::Error::other("x")));
    }

    /// magic check(MZ)を通すが exec 不可能な executable。
    /// `ERROR_EXE_MACHINE_TYPE_MISMATCH`(216)→ libuv 写像 `"UNKNOWN"`、
    /// Node は非 ENOENT error を sync throw するため
    /// `"managed process spawn failed"` + `spawnCode` + cause 相当。
    /// (unix 側は `execvpe` の ENOEXEC→`/bin/sh` fallback が干渉するため
    /// execute-level 検証は win32 のみ — mapping/shape 規則は unit test
    /// で別途担保する。)
    #[cfg(windows)]
    #[tokio::test]
    async fn spawn_errno_failure_reports_libuv_style_code() {
        let dir = TempDir::new();
        let bogus = dir.0.join("bogus.exe");
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&bogus).unwrap();
            f.write_all(b"MZnot-a-real-pe").unwrap();
        }
        let tool = tool(
            dir.root(),
            json!({"check": {"command": ["./bogus.exe"], "inputs": [], "outputs": []}}),
        );
        let err = tool
            .execute(&json!({"task": "check"}), None)
            .await
            .unwrap_err();
        let ce = find_cheaper_error(err.as_ref()).expect("CheaperError");
        assert_eq!(ce.code(), ErrorCode::ToolExecutionFailure);
        let details = ce.details().expect("details");
        assert_eq!(ce.message(), "managed process spawn failed");
        // 216(machine type mismatch)は libuv の unmapped → "UNKNOWN"。
        assert_eq!(details["spawnCode"], json!("UNKNOWN"));
        assert_eq!(details["task"], json!("check"));
        assert_eq!(details["state"], json!("failed"));
    }

    #[tokio::test]
    async fn pre_spawn_abort_rejects_with_outer_reason() {
        let dir = TempDir::new();
        let tool = tool(
            dir.root(),
            json!({"check": {"command": ["cc"], "inputs": [], "outputs": []}}),
        );
        let controller = AbortController::new();
        controller.abort(AbortReason::from(CheaperError::new(
            ErrorCode::Aborted,
            "outer wins",
        )));
        let err = tool
            .execute(&json!({"task": "check"}), Some(controller.signal()))
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "outer wins");
        assert_eq!(cheaper_code(&err), Some(ErrorCode::Aborted));
    }
}
