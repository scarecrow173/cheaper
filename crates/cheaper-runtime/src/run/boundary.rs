//! public run boundary — `run_worker` の入力/出力型と panic・
//! timeout・外部 signal を配線する outer orchestration。
//! pipeline 本体(`RunCtx`)は `mod.rs`、session seam は `session.rs`。

use std::sync::Arc;
use std::time::Duration;

use cheaper_core::model::config::ModelConfig;
use cheaper_core::policy::capability::check_capabilities;
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::worker::validate::WorkerDefinition;
use cheaper_tools::invocation::InvocationToolRuntime;
use cheaper_tools::operation::{AbortSignal, Clock};
use cheaper_tools::resources::ResourceLease;
use serde_json::{Map, Value};
use tracing::Instrument;

use super::abort::{RunAbort, RunAbortReason};
use super::checkpoint::ResumeCheckpoint;
use super::error::WorkerRunError;
use super::meta::WorkerRunMeta;
use super::panic_boundary::catch_run_panic;
use super::session::SessionFactory;
use super::RunCtx;
use crate::runtime::Runtime;

/// `promptWithAbortGrace` の既定 grace(TS `abortGraceMs` default)。
pub const DEFAULT_ABORT_GRACE_MS: f64 = 5000.0;

// ---------------------------------------------------------------------------
// deps / outcome types
// ---------------------------------------------------------------------------

/// `deps.execution`(TS `WorkerExecutionBinding`)相当 — borrowed
/// invocation tool runtime 経路。cheaper-tools の同名 struct は
/// `runtime: ToolRuntime`(scope 側)を持ち invocation を表せないため、
/// run 経路では invocation 版をここで定義する(Phase 2 命名差異)。
#[derive(Clone)]
pub struct ExecutionBinding {
    pub runtime: InvocationToolRuntime,
    pub owner_id: String,
    /// scope 引き渡しの parent lease(borrowed runtime 経路)。
    pub parent_lease: Option<ResourceLease>,
    pub resource_manifest: Option<Value>,
}

/// `RunWorkerDeps` — worker/policy/catalog/session factory 等の
/// 解決済み入力。CLI/workflow 側の解決(worker 探索・policy 解決)は
/// 呼び出し側の責務。
pub struct RunWorkerDeps<'a> {
    pub worker: &'a WorkerDefinition,
    /// run input(`build_user_prompt` / writeScope 抽出に使う)。
    pub input: &'a Value,
    /// workspace root。
    pub root: String,
    pub policy: &'a RuntimePolicy,
    /// `models.json`/`models-store.json` の user config。
    pub model_config: &'a ModelConfig,
    /// T2 runtime — catalog + auth 解決源。
    pub runtime: &'a Runtime,
    /// session 生成 seam。production は `RigSessionFactory`。
    pub session_factory: &'a dyn SessionFactory,
    /// `--model <provider>/<id>`。
    pub explicit_model: Option<&'a str>,
    /// 外部 abort signal(SIGINT 等)。`ABORTED` へ分類される。
    pub signal: Option<&'a AbortSignal>,
    /// `abortGraceMs`(既定 5000)。
    pub abort_grace_ms: Option<f64>,
    /// borrowed invocation tool runtime 経路(未指定は owned 生成)。
    pub execution: Option<ExecutionBinding>,
    /// tool operation の時間源(test 用 override)。
    pub operation_clock: Option<Clock>,
    /// `--grant` 由来の invocation grant 名(宣言順)—
    /// `cheaper.run.tool_scope` span の `grants.invocation` field へ
    /// record する。runtime 層では policy と区別不能のため cli 層から
    /// 運ぶ(DESIGN-034FntkIZGiffhdU4caSkN §(d))。未指定は空 slice。
    pub invocation_grants: &'a [String],
    /// `cheaper run --resume` 由来の前回 checkpoint(issue #111)—
    /// 観測注入・連鎖 resume 累積に使う。session/budget は新規。
    pub resume: Option<&'a ResumeCheckpoint>,
}

/// `runWorker` 成功結果。
#[derive(Debug)]
pub struct RunWorkerOk {
    /// `worker.outputSchema` を満たす validated output object。
    pub result: Map<String, Value>,
    pub meta: WorkerRunMeta,
}

// ---------------------------------------------------------------------------
// run_worker
// ---------------------------------------------------------------------------

/// `runWorker(deps)`。panic 境界を外側に敷き、panic は `INTERNAL_ERROR`
/// の `WorkerRunError`(runMeta 無し)に正規化する。
pub async fn run_worker(deps: &RunWorkerDeps<'_>) -> Result<RunWorkerOk, WorkerRunError> {
    match catch_run_panic(run_worker_inner(deps)).await {
        Ok(result) => result,
        Err(err) => Err(WorkerRunError::new(err, None)),
    }
}

async fn run_worker_inner(deps: &RunWorkerDeps<'_>) -> Result<RunWorkerOk, WorkerRunError> {
    let worker = deps.worker;
    let policy = deps.policy;
    // `checkCapabilities` — TS では try の外のため生の CheaperError が
    // 伝播し runMeta は付かない。
    let effective_tools =
        check_capabilities(worker, policy).map_err(|e| WorkerRunError::new(e, None))?;
    let grace_ms = deps.abort_grace_ms.unwrap_or(DEFAULT_ABORT_GRACE_MS);
    let run_abort = Arc::new(RunAbort::new());

    // `setTimeout(onTimeout, timeoutMs)` → issue #111 で run 経路は
    // stall window 化: 観測された `RunEvent`(進捗)が timeout を
    // リセットする。`last_progress + timeout` で起床し、起きた時点の
    // 最終進捗で再判定 — 進捗があれば次の期限へ延長、無ければ
    // `TIMEOUT` abort。sleep 中に届いた進捗で旧 deadline が過去に
    // なるケースは起床後の fresh read で救済される。
    // (workflow 経路の timeout は `engine.rs`/`spawn_deadline` の
    //  absolute deadline のまま — 意味を共有しない。)
    let timeout_task = {
        let abort = Arc::clone(&run_abort);
        let progress = run_abort.progress_handle();
        let timeout = Duration::from_millis(policy.execution.timeout_ms);
        tokio::spawn(async move {
            loop {
                let deadline = progress.last() + timeout;
                tokio::time::sleep_until(deadline).await;
                if tokio::time::Instant::now() >= progress.last() + timeout {
                    abort.request_abort(RunAbortReason::Timeout);
                    return;
                }
            }
        })
    };
    // `deps.signal.onAbort → requestAbort("signal")`。
    let signal_listener = deps.signal.map(|signal| {
        let abort = Arc::clone(&run_abort);
        signal.on_abort(move |_| abort.request_abort(RunAbortReason::Signal))
    });
    if let Some(signal) = deps.signal {
        if signal.is_aborted() {
            run_abort.request_abort(RunAbortReason::Signal);
        }
    }

    let mut ctx = RunCtx::new(deps, effective_tools);
    // `cheaper.run` — run pipeline 全体の span(design §instrumentation
    // 配置)。stage timing は `FmtSpan::CLOSE` の close event に委譲し、
    // site 毎の計時記録は持たない。
    let outcome = ctx
        .run(&run_abort, grace_ms)
        .instrument(tracing::info_span!(
            "cheaper.run",
            worker.name = worker.name.as_str(),
        ))
        .await;

    // TS `finally` 相当: timeout clear・listener 解除・dispose。
    timeout_task.abort();
    if let (Some(signal), Some(id)) = (deps.signal, signal_listener) {
        signal.remove_listener(id);
    }
    ctx.dispose().await;
    outcome
}
