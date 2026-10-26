//! Port of `packages/workflow/src/engine.ts`.
//!
//! runtime/store の生成を持たず、compiled semantic edges だけを実行する。
//! Node の microtask pump は `tokio::sync::mpsc` event loop で再現する:
//! admission/executor settle・timer・外部 abort は全て event として
//! 到着順に処理され、state mutation は loop 側だけが行う。executor の
//! `finally`(lease release)は、loop 生存中は event handler が
//! `finishJob` 後に呼び、loop 終了後の detached settle は task 側が
//! send 失敗を検知して release する(TS の `finally` が return 後も
//! 走るのと同じ guarantee)。

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
// `tokio::time::Instant` — paused clock(`tokio::time::advance` / test
// `vi.useFakeTimers` 相当)を尊重するため std ではなく tokio の Instant。
use tokio::time::Instant;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::{code_unit_compare, MAX_SAFE_INTEGER};
use cheaper_core::json::number as js_number;
use cheaper_tools::execution_metrics::ExecutionMetrics;
use cheaper_tools::operation::{AbortController, AbortReason, AbortSignal};
use cheaper_tools::resources::ResourceLease;
use cheaper_tools::ToolError;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

use crate::bindings::{materialize_bindings, parse_binding, resolve_pointer, MaterializeOptions};
use crate::json::clone_json;
use crate::types::{Binding, CompiledJob, CompiledWorkflow, JobKind, WorkflowJobState};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// `WorkflowExecutionContext`。
pub struct WorkflowExecutionContext {
    pub job_id: String,
    pub job: CompiledJob,
    pub input: Value,
    pub signal: AbortSignal,
    pub parent_lease: ResourceLease,
}

/// `Omit<WorkflowExecutionContext, "parentLease">`(lease request)。
pub struct JobLeaseRequest {
    pub job_id: String,
    pub job: CompiledJob,
    pub input: Value,
    pub signal: AbortSignal,
}

/// `WorkflowExecutorResult`。
pub struct WorkflowExecutorResult {
    pub result: Value,
    /// adapter が公開可能な field のみを選択する。input/result/error
    /// 全体を入れない。
    pub meta: Option<Map<String, Value>>,
}

/// `WorkflowExecutorError` — CheaperError 形 + 選択済み metadata。
/// failure の metadata を Pi 型や raw error payload から分離して運ぶ。
#[derive(Debug)]
pub struct WorkflowExecutorError {
    inner: CheaperError,
    meta: Map<String, Value>,
}

impl WorkflowExecutorError {
    /// `new WorkflowExecutorError(code, message, {meta, details?, cause?})`
    /// — `meta` は `cloneJson(meta, "INVALID_OUTPUT")` 相当。
    pub fn new(
        code: ErrorCode,
        message: impl Into<String>,
        meta: Map<String, Value>,
    ) -> Result<Self, CheaperError> {
        let meta = clone_json(&Value::Object(meta), ErrorCode::InvalidOutput)?
            .as_object()
            .cloned()
            .unwrap_or_default();
        Ok(Self {
            inner: CheaperError::new(code, message),
            meta,
        })
    }

    pub fn with_details(mut self, details: Map<String, Value>) -> Self {
        self.inner = self.inner.with_details(details);
        self
    }

    pub fn with_cause(mut self, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.inner = self.inner.with_cause(cause);
        self
    }

    pub fn code(&self) -> ErrorCode {
        self.inner.code()
    }

    pub fn message(&self) -> &str {
        self.inner.message()
    }

    pub fn meta(&self) -> &Map<String, Value> {
        &self.meta
    }

    /// inner `CheaperError`(downcast 後の code/details 参照用)。
    pub fn cheaper_error(&self) -> &CheaperError {
        &self.inner
    }
}

impl std::fmt::Display for WorkflowExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for WorkflowExecutorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

/// `WorkerExecutor` / `ToolExecutor`(`execute` のみ)。
pub trait JobExecutor: Send + Sync {
    fn execute(
        &self,
        context: WorkflowExecutionContext,
    ) -> BoxFuture<'static, Result<WorkflowExecutorResult, ToolError>>;
}

/// Job identity and dependency context exposed at the engine lifecycle boundary.
/// This intentionally contains only workflow-level values, not CLI History types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowJobObservation {
    pub job_id: String,
    pub kind: JobKind,
    pub name: String,
    pub dependency_ids: Vec<String>,
}

/// Terminal state exposed to lifecycle observers. `Blocked` distinguishes a
/// dependency failure from a condition-based `Skipped` job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowJobTerminalStatus {
    Completed,
    Failed,
    Cancelled,
    Timeout,
    Skipped,
    Blocked,
}

/// Authoritative lifecycle transition emitted by the engine state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowJobTransition {
    Started {
        job: WorkflowJobObservation,
    },
    Terminal {
        job: WorkflowJobObservation,
        status: WorkflowJobTerminalStatus,
        error_code: Option<String>,
        duration_ms: Option<f64>,
        turns: Option<f64>,
        tool_calls: Option<f64>,
    },
}

/// Observer failure is reported to the engine and deliberately ignored there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowJobObserverError;

/// Synchronous, best-effort sink for engine-owned Job transitions.
pub trait WorkflowJobObserver: Send {
    fn observe(
        &mut self,
        transition: WorkflowJobTransition,
    ) -> Result<(), WorkflowJobObserverError>;
}

/// `WorkflowEnginePorts`。`acquireJobLease`/`abort`/
/// `executionMetrics`/`releaseJobLease` は callback として保持する。
#[derive(Clone)]
pub struct WorkflowEnginePorts {
    pub worker: Arc<dyn JobExecutor>,
    pub tool: Arc<dyn JobExecutor>,
    pub acquire_job_lease: Arc<
        dyn Fn(JobLeaseRequest) -> BoxFuture<'static, Result<ResourceLease, ToolError>>
            + Send
            + Sync,
    >,
    pub release_job_lease: Arc<dyn Fn(ResourceLease) + Send + Sync>,
    pub abort: Arc<dyn Fn(&AbortReason) + Send + Sync>,
    /// `executionMetrics(jobId?)` — `null` を `None` で返す。
    pub execution_metrics: Arc<ExecutionMetricsFn>,
}

/// `executionMetrics(jobId?)` callback の型。
pub type ExecutionMetricsFn = dyn Fn(Option<&str>) -> Option<ExecutionMetrics> + Send + Sync;

/// `WorkflowDiagnostic`。`code` は `ErrorCode | "DEPENDENCY_FAILED"`。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorkflowDiagnostic {
    pub code: String,
    pub message: String,
}

/// `WorkflowJobOutcome["status"]`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Admitting,
    Ready,
    Running,
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

impl JobStatus {
    fn as_outcome_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
            Self::Cancelled => "cancelled",
            _ => "pending",
        }
    }
}

impl Serialize for JobStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_outcome_str())
    }
}

/// `WorkflowJobOutcome["meta"]` — `{...executorMeta, durationMs,
/// execution}` の flat shape。executor meta が既に `durationMs`/
/// `execution` key を持つ場合、JS spread は「元の key 位置のまま値を
/// 上書き」する — 同じ挙動を custom Serialize で再現する(duplicate
/// key を出さない)。
#[derive(Debug, Clone)]
pub struct JobOutcomeMeta {
    pub extra: Map<String, Value>,
    pub duration_ms: Option<f64>,
    /// `ExecutionMetrics | null` — serde には null として出す。
    pub execution: Option<ExecutionMetrics>,
}

impl Serialize for JobOutcomeMeta {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let has_duration = self.extra.contains_key("durationMs");
        let has_execution = self.extra.contains_key("execution");
        let len = self.extra.len() + usize::from(!has_duration) + usize::from(!has_execution);
        let mut m = serializer.serialize_map(Some(len))?;
        for (key, value) in &self.extra {
            match key.as_str() {
                "durationMs" => {
                    m.serialize_entry("durationMs", &SerializeDuration(&self.duration_ms))?
                }
                "execution" => {
                    m.serialize_entry("execution", &SerializeExecution(&self.execution))?
                }
                _ => m.serialize_entry(key, value)?,
            }
        }
        if !has_duration {
            m.serialize_entry("durationMs", &SerializeDuration(&self.duration_ms))?;
        }
        if !has_execution {
            m.serialize_entry("execution", &SerializeExecution(&self.execution))?;
        }
        m.end()
    }
}

/// `durationMs: number | null` — number は JS canonical format
/// (`0.0` → `"0"`)で出す(TS `JSON.stringify` 互換)。
struct SerializeDuration<'a>(&'a Option<f64>);

impl Serialize for SerializeDuration<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Some(v) => js_number::JsNumber(*v).serialize(serializer),
            None => serializer.serialize_unit(),
        }
    }
}

/// `execution` field の `null` 投影を共有する serialize helper。
struct SerializeExecution<'a>(&'a Option<ExecutionMetrics>);

impl Serialize for SerializeExecution<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Some(m) => m.serialize(serializer),
            None => serializer.serialize_unit(),
        }
    }
}

fn serialize_metrics<S: Serializer>(
    value: &Option<ExecutionMetrics>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(m) => m.serialize(serializer),
        None => serializer.serialize_unit(),
    }
}

/// `WorkflowJobOutcome`。
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowJobOutcome {
    pub id: String,
    #[serde(serialize_with = "serialize_kind")]
    pub kind: JobKind,
    pub status: JobStatus,
    pub meta: JobOutcomeMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WorkflowDiagnostic>,
}

fn serialize_kind<S: Serializer>(kind: &JobKind, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(kind.as_str())
}

/// `WorkflowOutcome["error"]`。
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowOutcomeError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(rename = "exitCode")]
    pub exit_code: u8,
    pub retryable: bool,
}

/// `meta.outputErrors[i]`。
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowOutputError {
    pub name: String,
    pub error: WorkflowDiagnostic,
}

/// `WorkflowOutcome["meta"]`。
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowOutcomeMeta {
    #[serde(rename = "durationMs", serialize_with = "js_number::serialize")]
    pub duration_ms: f64,
    #[serde(serialize_with = "serialize_metrics")]
    pub execution: Option<ExecutionMetrics>,
    #[serde(rename = "outputErrors")]
    pub output_errors: Vec<WorkflowOutputError>,
}

/// `WorkflowOutcome`。
#[derive(Debug, Clone)]
pub struct WorkflowOutcome {
    pub ok: bool,
    pub result: Map<String, Value>,
    pub jobs: Vec<WorkflowJobOutcome>,
    pub meta: WorkflowOutcomeMeta,
    pub error: Option<WorkflowOutcomeError>,
}

impl Serialize for WorkflowOutcome {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let len = 4 + usize::from(self.error.is_some());
        let mut m = serializer.serialize_map(Some(len))?;
        m.serialize_entry("ok", &self.ok)?;
        m.serialize_entry("result", &self.result)?;
        m.serialize_entry("jobs", &self.jobs)?;
        m.serialize_entry("meta", &self.meta)?;
        if let Some(error) = &self.error {
            m.serialize_entry("error", error)?;
        }
        m.end()
    }
}

/// `WorkflowRunOptions`。
pub struct WorkflowRunOptions<'a> {
    pub workflow: &'a CompiledWorkflow,
    pub ports: WorkflowEnginePorts,
    /// `maxJobs` — `Number.isSafeInteger(n) && n > 0`。
    pub max_jobs: f64,
    /// `timeoutMs` — finite positive。
    pub timeout_ms: f64,
    pub signal: Option<AbortSignal>,
    /// `abortGraceMs ?? 5000`。
    pub abort_grace_ms: Option<f64>,
    /// `now` — duration 計測用 clock(`performance.now` 相当の monotonic
    /// ms)。deadline arm は tokio clock(paused 対応)を使う。
    pub now: Option<Arc<dyn Fn() -> f64 + Send + Sync>>,
}

/// `performance.now()` 相当(monotonic ms、origin は process 起動時)。
/// duration 計測の既定 clock(TS `performance.now` は fake timer の
/// 対象になるが、Rust の既定値は実時計で固定し、fake 系検証は
/// `options.now` 注入側が担う)。
fn performance_now() -> f64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let start = START.get_or_init(std::time::Instant::now);
    start.elapsed().as_secs_f64() * 1000.0
}

/// `diagnostic` — validator details/cause は公開せず、message 保持も
/// UTF-8 2048 byte cap 内に置く。code point 単位で累積する(TS の
/// `for..of` + `Buffer.byteLength` 相当; Rust の char = code point)。
fn diagnostic(error: &ToolError, fallback: ErrorCode) -> WorkflowDiagnostic {
    let code = if let Some(w) = error.downcast_ref::<WorkflowExecutorError>() {
        w.code()
    } else {
        cheaper_tools::tools::process::find_cheaper_error(error.as_ref())
            .map(|c| c.code())
            .unwrap_or(fallback)
    };
    let source = error.to_string();
    let mut message = String::new();
    let mut bytes = 0usize;
    for point in source.chars() {
        let size = point.len_utf8();
        if bytes + size > 2048 {
            break;
        }
        message.push(point);
        bytes += size;
    }
    WorkflowDiagnostic {
        code: code.as_str().to_string(),
        message,
    }
}

/// 非 Error throw(`throw "x"` 相当)は Rust では `ToolError` の
/// Display になるため、non-CheaperError は message として Display を
/// そのまま使う(TS の `error instanceof Error ? error.message :
/// "job execution failed"` に相当 — Rust では Display を持たない throw
/// は表現できない)。
fn diagnostic_of_error(error: ToolError, fallback: ErrorCode) -> WorkflowDiagnostic {
    diagnostic(&error, fallback)
}

/// `deadline(delayMs, callback, startedAt)` — Node の最大 delay を超える
/// deadline も 1ms へ丸めず、timeout 境界でのみ再 arm する。
/// cancel は token で行う。
fn spawn_deadline(
    delay_ms: f64,
    started_at: Instant,
    cancel: CancellationToken,
    fire: impl FnOnce() + Send + 'static,
) {
    tokio::spawn(async move {
        let end = started_at + std::time::Duration::from_secs_f64(delay_ms.max(0.0) / 1000.0);
        let mut fire = Some(fire);
        loop {
            let remaining = end.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                if let Some(fire) = fire.take() {
                    fire();
                }
                return;
            }
            // `Math.min(2147483647, Math.max(1, Math.ceil(remaining)))`。
            let delay = (remaining.as_secs_f64() * 1000.0)
                .ceil()
                .clamp(1.0, 2_147_483_647.0);
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = tokio::time::sleep(std::time::Duration::from_millis(delay as u64)) => {
                    // 早発火(時計遡り等)なら再 arm — loop で remaining を
                    // 再評価する。
                }
            }
        }
    });
}

struct JobExecution {
    id: String,
    job: CompiledJob,
    dependency_ids: Vec<String>,
    phase: JobStatus,
    remaining: usize,
    children: Vec<usize>,
    dependency_failed: bool,
    input: Option<Value>,
    lease: Option<ResourceLease>,
    prepared_at: Option<f64>,
    outcome: Option<WorkflowJobOutcome>,
}

enum Event {
    Admission {
        index: usize,
        result: Result<ResourceLease, ToolError>,
    },
    Executed {
        index: usize,
        result: Result<WorkflowExecutorResult, ToolError>,
    },
    Timeout,
    Grace,
    /// 外部 abort — `reason` と発火時刻 `at` を listener で捕まえる
    /// (TS では `onAbort` が `abort()` 内で同期実行され、grace は
    /// abort 瞬間起点。event 処理まで時計が進む fake timer 環境でも
    /// 同じ起点にするため時刻を運ぶ)。
    ExternalAbort {
        reason: AbortReason,
        at: Instant,
    },
}

/// `when` predicate の deterministic 評価(LLM・I/O なし)。
/// operand は `materialize_bindings` と同じ規則で解決する
/// (skipped producer の `$from` は `null`)。binding 解決失敗は
/// when-false ではなく error(呼出し側で job failure 化)。
///
/// - `exists`: `left` が非 `null` に materialize するか。
///   `$from` の producer skipped/result 不在・pointer 未解決は
///   `false`(`null`/missing は存在しない扱い)。
/// - `eq`/`ne`: JSON 等価。
/// - `gt`/`lt`: 両 operand が number のときのみ数値比較、
///   片方でも非数値なら `false`(判定不能を安全側へ倒す)。
fn evaluate_when(
    when: &Value,
    inputs: &Map<String, Value>,
    results: &HashMap<String, Value>,
    skipped: &HashSet<String>,
) -> Result<bool, CheaperError> {
    let obj = when.as_object();
    let op = obj.and_then(|o| o.get("op")).and_then(Value::as_str);
    let options = MaterializeOptions {
        skipped_from: Some(skipped),
        ..Default::default()
    };
    let (Some(obj), Some(op)) = (obj, op) else {
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            "invalid when predicate",
        ));
    };
    let left = obj.get("left").unwrap_or(&Value::Null);
    if op == "exists" {
        // `$from` operand は pointer 未解決・producer 不在を
        // 「存在しない」として `false` へ倒す(error ではない)。
        if let Some(Binding::From(b)) = parse_binding(left)? {
            if skipped.contains(&b.from) {
                return Ok(false);
            }
            return Ok(match results.get(&b.from) {
                Some(result) => resolve_pointer(result, &b.path, ErrorCode::InvalidInput)
                    .map(|v| !v.is_null())
                    .unwrap_or(false),
                None => false,
            });
        }
        return Ok(!materialize_bindings(left, inputs, results, options)?.is_null());
    }
    let left = materialize_bindings(left, inputs, results, options)?;
    let right = materialize_bindings(
        obj.get("right").unwrap_or(&Value::Null),
        inputs,
        results,
        options,
    )?;
    match op {
        "eq" => Ok(left == right),
        "ne" => Ok(left != right),
        "gt" | "lt" => match (left.as_f64(), right.as_f64()) {
            (Some(left), Some(right)) => Ok(if op == "gt" {
                left > right
            } else {
                left < right
            }),
            _ => Ok(false),
        },
        _ => Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("unknown when op: {op}"),
        )),
    }
}

/// `diagnostic(outerError)` — `CheaperError` 入力直結用。
fn diagnostic_of(error: &CheaperError) -> WorkflowDiagnostic {
    let source = error.message();
    let mut message = String::new();
    let mut bytes = 0usize;
    for point in source.chars() {
        let size = point.len_utf8();
        if bytes + size > 2048 {
            break;
        }
        message.push(point);
        bytes += size;
    }
    WorkflowDiagnostic {
        code: error.code().as_str().to_string(),
        message,
    }
}

/// `runWorkflow` の event-loop 状態 — children/remaining/status/
/// outer_error/timers を所有し、pump/admit/execute/on_* /stop を method
/// として提供する(TS の closure 捕捉 state に相当)。
struct RunState<'x, 'o> {
    workflow: &'x CompiledWorkflow,
    ports: &'x WorkflowEnginePorts,
    observer: Option<&'o mut dyn WorkflowJobObserver>,
    jobs: Vec<JobExecution>,
    results: HashMap<String, Value>,
    /// `CONDITION_SKIPPED` で終了した job id — `$from` の null
    /// materialization・`dependency_failed` 非伝播・workflow error
    /// 集約除外の 3 箇所で参照する。
    condition_skipped: HashSet<String>,
    running: usize,
    terminal: usize,
    outer_error: Option<CheaperError>,
    now: &'x Arc<dyn Fn() -> f64 + Send + Sync>,
    finished: bool,
    tx: mpsc::UnboundedSender<Event>,
    controller: AbortController,
    max_jobs: f64,
    abort_grace_ms: f64,
    timeout_cancel: CancellationToken,
    grace_cancel: CancellationToken,
}

impl<'x, 'o> RunState<'x, 'o> {
    fn job_observation(&self, index: usize) -> WorkflowJobObservation {
        let job = &self.jobs[index];
        WorkflowJobObservation {
            job_id: job.id.clone(),
            kind: job.job.kind,
            name: job.job.name.clone(),
            dependency_ids: job.dependency_ids.clone(),
        }
    }

    fn observe(&mut self, transition: WorkflowJobTransition) {
        let job_id = match &transition {
            WorkflowJobTransition::Started { job }
            | WorkflowJobTransition::Terminal { job, .. } => job.job_id.clone(),
        };
        if self
            .observer
            .as_deref_mut()
            .is_some_and(|observer| observer.observe(transition).is_err())
        {
            tracing::debug!(
                target: "cheaper.workflow",
                message = "workflow job observer failed; ignoring observation error",
                job.id = %job_id,
            );
        }
    }

    fn finish_job(
        &mut self,
        index: usize,
        status: JobStatus,
        error: Option<WorkflowDiagnostic>,
        meta: Map<String, Value>,
    ) {
        if self.jobs[index].outcome.is_some() {
            return;
        }
        if self.jobs[index].phase == JobStatus::Running {
            self.running -= 1;
        }
        self.jobs[index].phase = status;
        // `CONDITION_SKIPPED` は dependents へ伝播しない — 各 dependent
        // は自身の `when` を評価するか、`when` が無ければ producer の
        // `null` result を input に実行する。
        let condition_skipped = error
            .as_ref()
            .is_some_and(|e| e.code == "CONDITION_SKIPPED");
        let prepared_at = self.jobs[index].prepared_at;
        let duration_ms = prepared_at.map(|at| ((self.now)() - at).max(0.0));
        let error_code = error.as_ref().map(|diagnostic| diagnostic.code.clone());
        let observer_status = if error_code.as_deref() == Some("TIMEOUT") {
            WorkflowJobTerminalStatus::Timeout
        } else if error_code.as_deref() == Some("DEPENDENCY_FAILED") {
            WorkflowJobTerminalStatus::Blocked
        } else {
            match status {
                JobStatus::Succeeded => WorkflowJobTerminalStatus::Completed,
                JobStatus::Failed => WorkflowJobTerminalStatus::Failed,
                JobStatus::Skipped => WorkflowJobTerminalStatus::Skipped,
                JobStatus::Cancelled => WorkflowJobTerminalStatus::Cancelled,
                _ => unreachable!("finish_job requires a terminal status"),
            }
        };
        let turns = meta.get("turns").and_then(Value::as_f64);
        let tool_calls = meta.get("toolCalls").and_then(Value::as_f64);
        let execution = (self.ports.execution_metrics)(Some(self.jobs[index].id.as_str()));
        self.jobs[index].outcome = Some(WorkflowJobOutcome {
            id: self.jobs[index].id.clone(),
            kind: self.jobs[index].job.kind,
            status,
            meta: JobOutcomeMeta {
                extra: meta,
                duration_ms,
                execution,
            },
            error,
        });
        self.terminal += 1;
        let children = std::mem::take(&mut self.jobs[index].children);
        for child in &children {
            self.jobs[*child].remaining -= 1;
            if status != JobStatus::Succeeded && !condition_skipped {
                self.jobs[*child].dependency_failed = true;
            }
        }
        // `jobs` の children は静的结构 — clone を戻す。
        self.jobs[index].children = children;
        if self.terminal == self.jobs.len() {
            self.finished = true;
        }
        self.observe(WorkflowJobTransition::Terminal {
            job: self.job_observation(index),
            status: observer_status,
            error_code,
            duration_ms,
            turns,
            tool_calls,
        });
        // `schedule()` — Rust 側は event 処理後に loop が必ず
        // `pump()` を呼ぶため flag は持たない。
    }

    fn admit(&mut self, index: usize) {
        match self.workflow.materialize_job_input(
            &self.jobs[index].id.clone(),
            &self.results,
            &self.condition_skipped,
        ) {
            Ok(input) => {
                self.jobs[index].prepared_at = Some((self.now)());
                self.jobs[index].input = Some(input.clone());
                self.jobs[index].phase = JobStatus::Admitting;
                let request = JobLeaseRequest {
                    job_id: self.jobs[index].id.clone(),
                    job: self.jobs[index].job.clone(),
                    input,
                    signal: self.controller.signal(),
                };
                let acquire = Arc::clone(&self.ports.acquire_job_lease);
                let release = Arc::clone(&self.ports.release_job_lease);
                let tx = self.tx.clone();
                tokio::spawn(async move {
                    let result = acquire(request).await;
                    if let Err(e) = tx.send(Event::Admission { index, result }) {
                        // loop 終了後の late grant — TS `admission.then`
                        // の `outerError` 経路相当。
                        if let Event::Admission {
                            result: Ok(lease), ..
                        } = e.0
                        {
                            release(lease);
                        }
                    }
                });
            }
            Err(error) => {
                let error: ToolError = Box::new(error);
                self.finish_job(
                    index,
                    JobStatus::Failed,
                    Some(diagnostic_of_error(error, ErrorCode::ToolExecutionFailure)),
                    Map::new(),
                );
            }
        }
    }

    fn execute(&mut self, index: usize) {
        self.jobs[index].phase = JobStatus::Running;
        self.running += 1;
        self.observe(WorkflowJobTransition::Started {
            job: self.job_observation(index),
        });
        // job 実行境界の node event(design §instrumentation 配置) —
        // `cheaper.workflow` span 内で id/name の分類値のみ記録する。
        tracing::debug!(
            target: "cheaper.workflow.node",
            message = format_args!("workflow node start"),
            job.id = %self.jobs[index].id,
            job.name = %self.jobs[index].job.name
        );
        let lease = self.jobs[index].lease.expect("ready implies lease");
        let executor = match self.jobs[index].job.kind {
            JobKind::Worker => Arc::clone(&self.ports.worker),
            JobKind::Tool => Arc::clone(&self.ports.tool),
        };
        let context = WorkflowExecutionContext {
            job_id: self.jobs[index].id.clone(),
            job: self.jobs[index].job.clone(),
            input: self.jobs[index].input.clone().unwrap_or(Value::Null),
            signal: self.controller.signal(),
            parent_lease: lease,
        };
        let release = Arc::clone(&self.ports.release_job_lease);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = executor.execute(context).await;
            if tx.send(Event::Executed { index, result }).is_err() {
                // loop 終了後の settle — `finally` が必ず走るのと同じく
                // lease は必ず release する。
                release(lease);
            }
        });
    }

    /// `pump()` — `pumping` guard は loop 側で不要(再入しない)。
    /// `outer_error` 設定済みなら早期 return。
    fn pump(&mut self) {
        if self.outer_error.is_some() {
            return;
        }
        // 同期 validation 失敗からの skip も一つの event で伝播する。
        loop {
            let mut progressed = false;
            for index in 0..self.jobs.len() {
                if self.jobs[index].phase != JobStatus::Pending || self.jobs[index].remaining != 0 {
                    continue;
                }
                if self.jobs[index].dependency_failed {
                    self.finish_job(
                        index,
                        JobStatus::Skipped,
                        Some(WorkflowDiagnostic {
                            code: "DEPENDENCY_FAILED".to_string(),
                            message: "required dependency failed".to_string(),
                        }),
                        Map::new(),
                    );
                } else if let Some(when) = self.jobs[index].job.when.clone() {
                    // `when` は dependency 解決後・admission 前に評価する
                    // (`when` 内 `$from` は dep edge 推論済みのため
                    // producer は terminal)。
                    match evaluate_when(
                        &when,
                        &self.workflow.inputs,
                        &self.results,
                        &self.condition_skipped,
                    ) {
                        Ok(true) => self.admit(index),
                        Ok(false) => {
                            self.condition_skipped.insert(self.jobs[index].id.clone());
                            self.finish_job(
                                index,
                                JobStatus::Skipped,
                                Some(WorkflowDiagnostic {
                                    code: "CONDITION_SKIPPED".to_string(),
                                    message: "job condition evaluated to false".to_string(),
                                }),
                                Map::new(),
                            );
                        }
                        Err(error) => {
                            // `when` の binding 解決失敗は silent skip で
                            // なく job failure(compile を通過した参照が
                            // runtime で失敗するのは engine 不変条件違反)。
                            let error: ToolError = Box::new(error);
                            self.finish_job(
                                index,
                                JobStatus::Failed,
                                Some(diagnostic(&error, ErrorCode::ToolExecutionFailure)),
                                Map::new(),
                            );
                        }
                    }
                } else {
                    self.admit(index);
                }
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
        for index in 0..self.jobs.len() {
            if self.running as f64 >= self.max_jobs {
                break;
            }
            if self.jobs[index].phase == JobStatus::Ready {
                self.execute(index);
            }
        }
        if self.jobs.is_empty() {
            self.finished = true;
        }
    }

    /// admission settle(`admission.then` の ok/err callback)。
    fn on_admission(&mut self, index: usize, result: Result<ResourceLease, ToolError>) {
        match result {
            Ok(lease) => {
                if self.outer_error.is_some() {
                    (self.ports.release_job_lease)(lease);
                    return;
                }
                self.jobs[index].lease = Some(lease);
                self.jobs[index].phase = JobStatus::Ready;
            }
            Err(error) => {
                self.finish_job(
                    index,
                    JobStatus::Failed,
                    Some(diagnostic(&error, ErrorCode::ToolExecutionFailure)),
                    Map::new(),
                );
            }
        }
    }

    /// executor settle — try/catch/finally の engine 側。
    fn on_executed(&mut self, index: usize, result: Result<WorkflowExecutorResult, ToolError>) {
        match result {
            Ok(value) => {
                if self.jobs[index].outcome.is_none() {
                    if let Some(outer) = self.outer_error.as_ref() {
                        self.finish_job(
                            index,
                            JobStatus::Cancelled,
                            Some(diagnostic_of(outer)),
                            Map::new(),
                        );
                    } else {
                        match self
                            .workflow
                            .validate_job_output(&self.jobs[index].id, &value.result)
                        {
                            Ok(output) => {
                                let meta = value.meta.unwrap_or_default();
                                self.results.insert(self.jobs[index].id.clone(), output);
                                self.finish_job(index, JobStatus::Succeeded, None, meta);
                            }
                            Err(error) => {
                                let error: ToolError = Box::new(error);
                                self.finish_job(
                                    index,
                                    JobStatus::Failed,
                                    Some(diagnostic(&error, ErrorCode::ToolExecutionFailure)),
                                    Map::new(),
                                );
                            }
                        }
                    }
                }
            }
            Err(error) => {
                let meta = error
                    .downcast_ref::<WorkflowExecutorError>()
                    .map(|w| w.meta().clone())
                    .unwrap_or_default();
                let diag = match &self.outer_error {
                    Some(outer) => diagnostic_of(outer),
                    None => diagnostic(&error, ErrorCode::ToolExecutionFailure),
                };
                self.finish_job(
                    index,
                    if self.outer_error.is_some() {
                        JobStatus::Cancelled
                    } else {
                        JobStatus::Failed
                    },
                    Some(diag),
                    meta,
                );
            }
        }
        // `finally { releaseJobLease(lease); schedule() }`。
        if let Some(lease) = self.jobs[index].lease.take() {
            (self.ports.release_job_lease)(lease);
        }
    }

    /// `detach()` — grace deadline 到達時に running job を打ち切る。
    fn on_grace(&mut self) {
        for index in 0..self.jobs.len() {
            if self.jobs[index].phase == JobStatus::Running {
                let diag = self.outer_error.as_ref().map(diagnostic_of);
                self.finish_job(index, JobStatus::Cancelled, diag, Map::new());
            }
        }
    }

    /// `stop(reason)` — `at` は abort の発生時刻。grace deadline の
    /// 起点(TS では `stop()` が abort listener 内で同期実行されるため
    /// abort 瞬間起点)。
    fn stop(&mut self, reason: AbortReason, at: Instant) {
        if self.outer_error.is_some() {
            return;
        }
        let is_timeout = reason.code() == Some("TIMEOUT");
        self.outer_error = Some(if is_timeout {
            CheaperError::new(ErrorCode::Timeout, "workflow timed out")
        } else {
            CheaperError::new(ErrorCode::Aborted, "workflow aborted")
        });
        self.controller.abort(reason.clone());
        (self.ports.abort)(&reason);
        for index in 0..self.jobs.len() {
            if self.jobs[index].outcome.is_some() || self.jobs[index].phase == JobStatus::Running {
                continue;
            }
            if let Some(lease) = self.jobs[index].lease.take() {
                (self.ports.release_job_lease)(lease);
            }
            self.finish_job(
                index,
                JobStatus::Cancelled,
                self.outer_error.as_ref().map(diagnostic_of),
                Map::new(),
            );
        }
        if self.running > 0 {
            if self.abort_grace_ms == 0.0 {
                for index in 0..self.jobs.len() {
                    if self.jobs[index].phase == JobStatus::Running {
                        self.finish_job(
                            index,
                            JobStatus::Cancelled,
                            self.outer_error.as_ref().map(diagnostic_of),
                            Map::new(),
                        );
                    }
                }
            } else {
                let tx2 = self.tx.clone();
                let cancel = self.grace_cancel.clone();
                let abort_grace_ms = self.abort_grace_ms;
                spawn_deadline(abort_grace_ms, at, cancel, move || {
                    let _ = tx2.send(Event::Grace);
                });
            }
        }
        if self.terminal == self.jobs.len() {
            self.finished = true;
        }
    }
}

/// `runWorkflow`。
///
/// throw は options 検証(`INVALID_REQUEST`)のみ — job/execution の失敗は
/// outcome に載る(TS 契約)。
///
/// `cheaper.workflow` span + field `run.id` が engine の job 実行境界の
/// instrumentation 点(design §instrumentation 配置)。`run.id` は options が
/// 持たないため run 毎に採番する(node event との相関用 metadata)。
pub async fn run_workflow(
    options: WorkflowRunOptions<'_>,
) -> Result<WorkflowOutcome, CheaperError> {
    run_workflow_with_optional_observer(options, None).await
}

/// Runs a workflow with a synchronous observer for authoritative Job
/// transitions. Observer errors are ignored and cannot affect execution.
pub async fn run_workflow_with_observer(
    options: WorkflowRunOptions<'_>,
    observer: &mut dyn WorkflowJobObserver,
) -> Result<WorkflowOutcome, CheaperError> {
    run_workflow_with_optional_observer(options, Some(observer)).await
}

async fn run_workflow_with_optional_observer<'a, 'o>(
    options: WorkflowRunOptions<'a>,
    observer: Option<&'o mut dyn WorkflowJobObserver>,
) -> Result<WorkflowOutcome, CheaperError> {
    let run_id = uuid::Uuid::new_v4().to_string();
    let span = tracing::info_span!("cheaper.workflow", run.id = %run_id);
    run_workflow_inner(options, observer).instrument(span).await
}

async fn run_workflow_inner<'a, 'o>(
    options: WorkflowRunOptions<'a>,
    observer: Option<&'o mut dyn WorkflowJobObserver>,
) -> Result<WorkflowOutcome, CheaperError> {
    let WorkflowRunOptions {
        workflow,
        ports,
        max_jobs,
        timeout_ms,
        signal,
        abort_grace_ms,
        now,
    } = options;
    let abort_grace_ms = abort_grace_ms.unwrap_or(5000.0);
    let is_safe_integer = |v: f64| v.is_finite() && v.fract() == 0.0 && v.abs() <= MAX_SAFE_INTEGER;
    if !is_safe_integer(max_jobs)
        || max_jobs <= 0.0
        || !timeout_ms.is_finite()
        || timeout_ms <= 0.0
        || !abort_grace_ms.is_finite()
        || abort_grace_ms < 0.0
    {
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            "invalid workflow execution options",
        ));
    }
    let timeout_started = Instant::now();
    let now = now.unwrap_or_else(|| Arc::new(performance_now));
    let started_at = now();
    let controller = AbortController::new();
    let mut job_ids: Vec<String> = workflow.jobs.iter().map(|(id, _)| id.clone()).collect();
    job_ids.sort_by(|a, b| code_unit_compare(a, b));
    let mut jobs: Vec<JobExecution> = job_ids
        .iter()
        .map(|id| JobExecution {
            id: id.clone(),
            job: workflow
                .jobs
                .iter()
                .find(|(k, _)| k == id)
                .map(|(_, v)| v.clone())
                .expect("job id from keys"),
            dependency_ids: Vec::new(),
            phase: JobStatus::Pending,
            remaining: 0,
            children: Vec::new(),
            dependency_failed: false,
            input: None,
            lease: None,
            prepared_at: None,
            outcome: None,
        })
        .collect();
    let index_of: HashMap<String, usize> = jobs
        .iter()
        .enumerate()
        .map(|(i, job)| (job.id.clone(), i))
        .collect();
    for edge in &workflow.edges {
        let parent = index_of[edge.from.as_str()];
        let child = index_of[edge.to.as_str()];
        jobs[parent].children.push(child);
        jobs[child].remaining += 1;
        jobs[child].dependency_ids.push(edge.from.clone());
    }
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let mut state = RunState {
        workflow,
        ports: &ports,
        observer,
        jobs,
        results: HashMap::new(),
        condition_skipped: HashSet::new(),
        running: 0,
        terminal: 0,
        outer_error: None,
        now: &now,
        finished: false,
        tx,
        controller,
        max_jobs,
        abort_grace_ms,
        timeout_cancel: CancellationToken::new(),
        grace_cancel: CancellationToken::new(),
    };

    // external abort listener — `addEventListener("abort", onAbort,
    // {once:true})`。
    let abort_listener = signal.as_ref().map(|signal| {
        let tx = state.tx.clone();
        (
            signal.clone(),
            signal.on_abort(move |reason| {
                let _ = tx.send(Event::ExternalAbort {
                    reason: reason.clone(),
                    at: Instant::now(),
                });
            }),
        )
    });

    if let Some((signal, _id)) = &abort_listener {
        if signal.is_aborted() {
            // `options.signal?.aborted → onAbort()`。
            let reason = signal.reason().unwrap_or_else(AbortReason::aborted);
            state.stop(reason, Instant::now());
        }
    }
    if state.outer_error.is_none() {
        // timeout arm(入力 signal が生きている場合のみ)。
        let tx2 = state.tx.clone();
        let cancel = state.timeout_cancel.clone();
        spawn_deadline(timeout_ms, timeout_started, cancel, move || {
            let _ = tx2.send(Event::Timeout);
        });
        state.pump();
    }

    while !state.finished {
        let Some(event) = rx.recv().await else {
            break;
        };
        // `stop(reason)` 経路は `jobs` への mutable borrow を pump と
        // 共有しないよう先に処理する。
        let abort = match &event {
            Event::Timeout => Some((
                AbortReason::from_error(Box::new(CheaperError::new(
                    ErrorCode::Timeout,
                    "workflow timed out",
                )) as ToolError),
                Instant::now(),
            )),
            Event::ExternalAbort { reason, at } => Some((reason.clone(), *at)),
            _ => None,
        };
        if let Some((reason, at)) = abort {
            state.stop(reason, at);
            continue;
        }
        match event {
            Event::Admission { index, result } => state.on_admission(index, result),
            Event::Executed { index, result } => state.on_executed(index, result),
            Event::Grace => state.on_grace(),
            Event::Timeout | Event::ExternalAbort { .. } => {
                unreachable!("handled above")
            }
        }
        // `schedule()` された microtask pump — Rust 側は event ごとに
        // 呼ぶ(pump は idempotent で `outerError` なら early return)。
        state.pump();
    }

    // `finally` — timer 停止・listener 除去。
    state.timeout_cancel.cancel();
    state.grace_cancel.cancel();
    if let Some((signal, id)) = abort_listener {
        signal.remove_listener(id);
    }
    // loop 終了後の detached settle が event を送れないよう channel を
    // 閉じ、queue 済み event は drain して lease を解放する
    // (in-flight task の send は失敗 → task 側 release)。
    rx.close();
    drop(state.tx);
    while let Ok(event) = rx.try_recv() {
        match event {
            Event::Admission {
                result: Ok(lease), ..
            } => (state.ports.release_job_lease)(lease),
            Event::Executed { index, .. } => {
                if let Some(lease) = state.jobs[index].lease.take() {
                    (state.ports.release_job_lease)(lease);
                }
            }
            _ => {}
        }
    }

    let states: HashMap<String, WorkflowJobState> = state
        .jobs
        .iter()
        .map(|job| {
            (
                job.id.clone(),
                WorkflowJobState {
                    terminal: true,
                    result: state.results.get(&job.id).cloned(),
                    condition_skipped: state.condition_skipped.contains(&job.id),
                },
            )
        })
        .collect();
    let mut output_names: Vec<&String> = workflow.outputs.keys().collect();
    output_names.sort_by(|a, b| code_unit_compare(a, b));
    let mut outputs = Map::new();
    let mut output_errors: Vec<WorkflowOutputError> = Vec::new();
    for name in output_names {
        match workflow.materialize_output(name, &states) {
            Ok(value) => {
                outputs.insert(name.clone(), value);
            }
            Err(error) => {
                let error: ToolError = Box::new(error);
                output_errors.push(WorkflowOutputError {
                    name: name.clone(),
                    error: diagnostic(&error, ErrorCode::InvalidOutput),
                });
            }
        }
    }
    let error = state.outer_error.or_else(|| {
        if state.jobs.iter().any(|job| {
            job.phase != JobStatus::Succeeded && !state.condition_skipped.contains(&job.id)
        }) {
            Some(CheaperError::new(
                ErrorCode::ToolExecutionFailure,
                "workflow job failed",
            ))
        } else if !output_errors.is_empty() {
            Some(CheaperError::new(
                ErrorCode::InvalidOutput,
                "workflow output validation failed",
            ))
        } else {
            None
        }
    });
    Ok(WorkflowOutcome {
        ok: error.is_none(),
        result: outputs,
        jobs: state
            .jobs
            .into_iter()
            .map(|job| job.outcome.expect("terminal"))
            .collect(),
        meta: WorkflowOutcomeMeta {
            duration_ms: (now() - started_at).max(0.0),
            execution: (ports.execution_metrics)(None),
            output_errors,
        },
        error: error.map(|e| WorkflowOutcomeError {
            code: e.code(),
            message: e.message().to_string(),
            exit_code: e.exit_code(),
            retryable: e.retryable(),
        }),
    })
}
