//! `workflow interface`/`workflow run` command 本体(T7) —
//! `packages/cli/src/workflow-run.ts` + `cli.ts` の workflow 分岐。
//!
//! executor/lease port の実装方針:
//! - `worker` executor — resolved worker map → `ensure_model` lazy
//!   bundle(`load_model_config` + `create_runtime` + fixture session
//!   factory)→ job 毎に `RunWorkerDeps` を組み立て `run_worker`。
//!   `execution` binding で shared invocation runtime + parent lease +
//!   manifest を渡す。
//! - `tool` executor — `ensure_invocation` → per-job `MutationJournal` +
//!   `create_worker_scope` → `scope.execute`。meta は `{toolCalls,
//!   changes}` のみ(`scope.budget`/`journal.finalize`)。
//! - `acquireJobLease`/`releaseJobLease`/`abort`/`executionMetrics` —
//!   shared invocation runtime 経由。
//! - `selectWorkerMeta` — serialize 済み map から `execution`/
//!   `durationMs` を落とす(TS の手選択と同じ key 集合)。
//! - artifact ledger — `RunArtifactStore` の publisher port wrap が
//!   `put` 成功ごとに `ArtifactReference` を畳み込む。envelope には
//!   `project_artifact_metadata({include_preview: false})` で投影。

#[cfg(feature = "dev-seam")]
use std::collections::BTreeMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cheaper_core::duration::parse_duration;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::config::{load_model_config, LoadModelConfigOptions, ModelConfig};
use cheaper_core::policy::capability::check_capabilities;
use cheaper_core::policy::restrict::{restrict_policy, PolicyRestrictions};
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::resources::types::ResourceManifest;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::deny_paths::create_path_denylist;
use cheaper_runtime::run::{run_worker, ExecutionBinding, RunWorkerDeps, SessionFactory};
use cheaper_runtime::{Runtime, WorkerRunError};
use cheaper_tools::artifact::{ArtifactPorts, ArtifactPublisher, ArtifactPut, ArtifactReference};
use cheaper_tools::invocation::{
    create_invocation_tool_runtime, InvocationToolRuntime, InvocationToolRuntimeOptions,
    JobLeaseOptions, WorkerScopeOptions,
};
use cheaper_tools::journal::MutationJournal;
use cheaper_tools::operation::{AbortController, AbortListenerId, AbortReason, AbortSignal};
use cheaper_tools::registry::{ToolCallContext, ToolRuntimeOptions};
use cheaper_tools::resources::ResourceLease;
use cheaper_tools::tools::process::find_cheaper_error;
use cheaper_tools::ToolError;
use cheaper_workflow::{
    project_artifact_metadata, run_workflow, ArtifactProjectionOptions, ArtifactStoreOptions,
    JobExecutor, JobKind, JobLeaseRequest, RunArtifactStore, WorkflowEnginePorts,
    WorkflowExecutionContext, WorkflowExecutorError, WorkflowExecutorResult, WorkflowOutcome,
    WorkflowRunOptions,
};
use serde::Serialize;
use serde_json::{Map, Value};
use tracing::Instrument;

use crate::args::{WorkflowInterfaceCliArgs, WorkflowRunCliArgs};
use crate::cli::{now, system_now, BoxError, CliHooks, CliIo, CommandOut};
use crate::envelope::{utility_success_envelope, workflow_envelope};
#[cfg(feature = "dev-seam")]
use crate::fake_stream::load_test_stream;
use crate::input::read_task_input;
use crate::run::{create_runtime, production_session_factory};
use crate::workflow_context::{prepare_workflow_context, SharedToolError};

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

// ---------------------------------------------------------------------------
// `ensureInvocation` / `ensureModel` — lazy shared state
// ---------------------------------------------------------------------------

/// `{runtime, modelConfig, streamFn}` の lazy 組立 bundle。
/// `streamFn` に相当するのは fixture `session_factory`(Rust では
/// streamFn ではなく session 全体を差し替える T3 済み seam)。
struct ModelBundle {
    runtime: Runtime,
    model_config: ModelConfig,
    session_factory: Box<dyn SessionFactory>,
    /// fixture `cheaper-fake/<model>` ref(`--model` flag は workflow
    /// args に存在しない — TS `streamFn ? {} : {}` ではなく
    /// `explicitModel` 側の fixture 補完)。
    explicit_model: Option<String>,
    /// fixture harness default(80ms)に揃える — production は `None`。
    abort_grace_ms: Option<f64>,
}

/// `ledger.set(ref.artifact, ref)` を `put` 成功時に行う publisher。
/// `Map` の insertion order と同じく `Vec` push で put 順を保持する。
struct LedgerPublisher {
    store: Arc<RunArtifactStore>,
    ledger: Arc<Mutex<Vec<ArtifactReference>>>,
}

impl ArtifactPublisher for LedgerPublisher {
    fn max_artifact_bytes(&self) -> u64 {
        self.store.max_artifact_bytes()
    }

    fn put(&self, value: ArtifactPut) -> Result<ArtifactReference, ToolError> {
        let reference = self
            .store
            .put(value)
            .map_err(|e| Box::new(e) as ToolError)?;
        self.ledger
            .lock()
            .expect("ledger poisoned")
            .push(reference.clone());
        Ok(reference)
    }
}

/// ports/executor/cleanup で共有する run-scoped state。TS の
/// `store`/`invocation`/`modelPromise`/`ledger`/`abortController`
/// closure state に相当。
struct Shared {
    root: String,
    user_config_dir: String,
    project_dir: String,
    /// `effectivePolicy`(restricted)。
    policy: RuntimePolicy,
    workers: Arc<crate::workflow_context::WorkerIndex>,
    /// `NODE_ENV`/`CHEAPER_TEST_FAKE_STREAM` の snapshot — `io.env`
    /// closure は `'static` でないため executor future へ運べない。
    /// `dev-seam` feature の fake_stream seam 専用(feature off では
    /// 読み手が存在しないため field 自体を gate する)。
    #[cfg(feature = "dev-seam")]
    env: BTreeMap<String, String>,
    controller: AbortController,
    invocation: Mutex<Option<InvocationToolRuntime>>,
    store: Mutex<Option<Arc<RunArtifactStore>>>,
    ledger: Arc<Mutex<Vec<ArtifactReference>>>,
    /// `modelPromise` memo — `ensureModel()` の失敗も TS の rejected
    /// promise memoization と同じく再 throw する。
    model: Mutex<Option<Result<Arc<ModelBundle>, SharedToolError>>>,
}

impl Shared {
    fn worker(&self, name: &str) -> Option<Arc<cheaper_core::worker::validate::WorkerDefinition>> {
        self.workers.worker(name)
    }
}

/// `ensureModel()`(TS L114-122)。`loadModelConfig` → fixture load →
/// `createModelRuntime` の順(TS は `createModelRuntime` → `loadStream`
/// の順だが、Rust の `create_runtime` は fixture を引数に取るため
/// 順序が入れ替わる — 両方失敗し得るケースのみ error 優先順位が違う
/// test-only seam 内の許容差分)。失敗は `Result` で memoize し
/// `throw` 相当の再評価で同じ error を返す。
fn ensure_model(shared: &Shared) -> Result<Arc<ModelBundle>, ToolError> {
    let mut slot = shared.model.lock().expect("model slot poisoned");
    if let Some(cached) = slot.as_ref() {
        return cached.clone().map_err(|e| e.boxed());
    }
    let result = build_model_bundle(shared)
        .map(Arc::new)
        .map_err(SharedToolError::new);
    *slot = Some(result.clone());
    result.map_err(|e| e.boxed())
}

fn build_model_bundle(shared: &Shared) -> Result<ModelBundle, ToolError> {
    let model_config = load_model_config(&LoadModelConfigOptions {
        user_config_dir: &shared.user_config_dir,
        project_dir: &shared.project_dir,
    })
    .map_err(|e| Box::new(e) as ToolError)?;
    // `loadStream(io.env, runtime)` 相当 — env snapshot 経由の
    // `load_test_stream`。fixture 有効時のみ `NODE_ENV==="test"` +
    // `CHEAPER_TEST_FAKE_STREAM` が効く(`fake_stream.rs`)。
    // `dev-seam` feature off では seam 自体が compile 対象外。
    #[cfg(feature = "dev-seam")]
    let fixture = load_test_stream(&|name: &str| shared.env.get(name).cloned())
        .map_err(|e| Box::new(e) as ToolError)?;
    let runtime = create_runtime(
        Path::new(&shared.user_config_dir),
        #[cfg(feature = "dev-seam")]
        fixture.as_ref(),
    )
    .map_err(|e| Box::new(e) as ToolError)?;
    #[cfg(feature = "dev-seam")]
    let session_factory: Box<dyn SessionFactory> = match &fixture {
        Some(f) => Box::new(f.session_factory()),
        None => production_session_factory(),
    };
    #[cfg(not(feature = "dev-seam"))]
    let session_factory: Box<dyn SessionFactory> = production_session_factory();
    #[cfg(feature = "dev-seam")]
    let explicit_model = fixture.as_ref().map(|f| f.explicit_model());
    #[cfg(not(feature = "dev-seam"))]
    let explicit_model: Option<String> = None;
    #[cfg(feature = "dev-seam")]
    let abort_grace_ms = fixture.as_ref().map(|f| f.abort_grace_ms.unwrap_or(80.0));
    #[cfg(not(feature = "dev-seam"))]
    let abort_grace_ms: Option<f64> = None;
    Ok(ModelBundle {
        runtime,
        model_config,
        session_factory,
        explicit_model,
        abort_grace_ms,
    })
}

/// `ensureInvocation()`(TS L88-113)— artifact store + ledger
/// publisher + shared invocation runtime の遅延初期化。呼出し元の
/// `Option<…>` slot を保持して冪等にする。
fn ensure_invocation(shared: &Shared) -> Result<InvocationToolRuntime, ToolError> {
    let mut slot = shared.invocation.lock().expect("invocation slot poisoned");
    if let Some(rt) = slot.as_ref() {
        return Ok(rt.clone());
    }
    let store = RunArtifactStore::new(ArtifactStoreOptions {
        max_artifact_bytes: shared.policy.workflow.artifacts.max_artifact_bytes,
        max_total_bytes: shared.policy.workflow.artifacts.max_total_bytes,
        max_preview_bytes: shared.policy.workflow.artifacts.max_preview_bytes,
        max_read_bytes: shared.policy.workflow.artifacts.max_read_bytes,
        signal: Some(shared.controller.signal()),
    })
    .map_err(|e| Box::new(e) as ToolError)?;
    *shared.store.lock().expect("store slot poisoned") = Some(Arc::clone(&store));
    let publisher = Arc::new(LedgerPublisher {
        store: Arc::clone(&store),
        ledger: Arc::clone(&shared.ledger),
    });
    let rt = create_invocation_tool_runtime(InvocationToolRuntimeOptions {
        artifacts: Some(ArtifactPorts {
            reader: Some(store.reader()),
            publisher: Some(publisher),
        }),
        // `process: effectivePolicy.process` — resolved policy を
        // serialize して `Partial<ProcessPolicy>` raw JSON として渡す
        // (invocation 側が `resolve_process_policy` で再 resolve する)。
        process: Some(
            serde_json::to_value(&shared.policy.process).map_err(|e| -> ToolError {
                Box::new(CheaperError::new(ErrorCode::InternalError, e.to_string()))
            })?,
        ),
        root: shared.root.clone(),
        symlink_policy: shared.policy.filesystem.symlinks,
        signal: Some(shared.controller.signal()),
        abort_grace_ms: None,
        concurrency: shared
            .policy
            .execution
            .concurrency
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| -> ToolError {
                Box::new(CheaperError::new(ErrorCode::InternalError, e.to_string()))
            })?,
        clock: None,
    })?;
    *slot = Some(rt.clone());
    Ok(rt)
}

// ---------------------------------------------------------------------------
// executor port 実装
// ---------------------------------------------------------------------------

/// `ctx.job.manifest` を `Option<Value>` へ写す(TS `resourceManifest`
/// field は JS object — `Option<ResourceManifest>` → serialized JSON)。
fn manifest_value(manifest: Option<&ResourceManifest>) -> Option<Value> {
    manifest.and_then(|m| serde_json::to_value(m).ok())
}

/// `selectWorkerMeta`(TS L295-307)— serialize 済み map から
/// `execution`/`durationMs` を落とす。`WorkerRunMeta`/`PartialRunMeta`
/// の field 順(model→…→changes→execution→savings→durationMs)で
/// `execution`/`durationMs` だけを除くと `{model, usage, turns,
/// toolCalls, rejectedToolCalls, repairs, transientRetries, changes,
/// savings}` — TS の明示選択と同じ key 集合・順序。
fn select_worker_meta(meta: &impl Serialize) -> Map<String, Value> {
    let mut map = match serde_json::to_value(meta) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    };
    map.remove("execution");
    map.remove("durationMs");
    map
}

/// `tool` executor の meta — `{toolCalls, changes}`。
/// `JSON.parse(JSON.stringify({...}))` 相当は serde → `Map`。
fn tool_meta(budget_calls: u64, journal: &MutationJournal) -> Map<String, Value> {
    let mut meta = Map::new();
    meta.insert("toolCalls".to_string(), Value::from(budget_calls));
    meta.insert(
        "changes".to_string(),
        serde_json::to_value(journal.finalize()).unwrap_or(Value::Null),
    );
    meta
}

/// `worker.execute(ctx)` — `runWorker` を workflow job binding で
/// 呼び、`WorkflowExecutorResult`/`WorkflowExecutorError` に畳む。
struct WorkerJobExecutor {
    shared: Arc<Shared>,
}

impl JobExecutor for WorkerJobExecutor {
    fn execute(
        &self,
        ctx: WorkflowExecutionContext,
    ) -> BoxFuture<'static, Result<WorkflowExecutorResult, ToolError>> {
        let shared = Arc::clone(&self.shared);
        Box::pin(async move {
            if ctx.job.kind != JobKind::Worker {
                return Err(Box::new(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "worker executor received a tool job",
                )) as ToolError);
            }
            let Some(worker) = shared.worker(&ctx.job.name) else {
                return Err(Box::new(CheaperError::new(
                    ErrorCode::WorkerNotFound,
                    format!("worker not found: {}", ctx.job.name),
                )) as ToolError);
            };
            let bundle = ensure_model(&shared)?;
            let invocation = ensure_invocation(&shared)?;
            let job_id = ctx.job_id.clone();
            let input = ctx.input.clone();
            let signal = ctx.signal.clone();
            let parent_lease = ctx.parent_lease;
            let resource_manifest = manifest_value(ctx.job.manifest.as_ref());
            let inner = Arc::clone(&shared);
            // `run_worker` の future は `&mut dyn FnMut` コールバックを
            // await 間に保持するため `!Send` — `tokio::spawn` で駆動する
            // engine executor(`BoxFuture + Send` 要求)からは直接 await
            // できない。blocking thread 上の専用 current_thread runtime で
            // `block_on` する(auth.rs の signal relay と同じ構成)。
            // 内部の `tokio::spawn`/`sleep` はこの runtime で駆動される。
            let spawned = tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        WorkerRunError::new(
                            CheaperError::new(
                                ErrorCode::InternalError,
                                format!("worker runtime init failed: {e}"),
                            ),
                            None,
                        )
                    })?;
                rt.block_on(async move {
                    let deps = RunWorkerDeps {
                        worker: &worker,
                        input: &input,
                        root: inner.root.clone(),
                        policy: &inner.policy,
                        model_config: &bundle.model_config,
                        runtime: &bundle.runtime,
                        session_factory: bundle.session_factory.as_ref(),
                        explicit_model: bundle.explicit_model.as_deref(),
                        signal: Some(&signal),
                        abort_grace_ms: bundle.abort_grace_ms,
                        execution: Some(ExecutionBinding {
                            runtime: invocation,
                            owner_id: job_id,
                            parent_lease: Some(parent_lease),
                            resource_manifest,
                        }),
                        operation_clock: None,
                    };
                    run_worker(&deps).await
                })
            })
            .await;
            let outcome = spawned.map_err(|join| -> ToolError {
                Box::new(
                    WorkflowExecutorError::new(
                        ErrorCode::InternalError,
                        join.to_string(),
                        Map::new(),
                    )
                    .expect("empty meta は必ず構築可能"),
                )
            })?;
            match outcome {
                Ok(ok) => Ok(WorkflowExecutorResult {
                    result: Value::Object(ok.result),
                    meta: Some(select_worker_meta(&ok.meta)),
                }),
                Err(error) => Err(Box::new(worker_executor_error(&error)) as ToolError),
            }
        })
    }
}

/// `catch` 側の wrap — `isCheaperError(error) ? error.code :
/// "TOOL_EXECUTION_FAILURE"`、`runMeta` は `selectWorkerMeta` 投影。
/// `run_worker` の `Err` は常に `WorkerRunError`(inner が
/// `CheaperError`)のため code は必ず `CheaperError` 由来。
fn worker_executor_error(error: &WorkerRunError) -> WorkflowExecutorError {
    let meta = error.run_meta().map(select_worker_meta).unwrap_or_default();
    // `WorkflowExecutorError::new` の `clone_json` 失敗は meta が
    // JSON-safe でない場合のみ — engine diagnostic fallback と同じく
    // `TOOL_EXECUTION_FAILURE` で包む(TS の非 Error throw 経路に相当)。
    WorkflowExecutorError::new(error.code(), error.message(), meta).unwrap_or_else(|_| {
        WorkflowExecutorError::new(ErrorCode::ToolExecutionFailure, "Worker failed", Map::new())
            .expect("empty meta は必ず構築可能")
    })
}

/// `tool.execute(ctx)` — per-job `MutationJournal` + `create_worker_scope`
/// + `scope.execute`。meta は `{toolCalls, changes}` のみ。
struct ToolJobExecutor {
    shared: Arc<Shared>,
}

impl JobExecutor for ToolJobExecutor {
    fn execute(
        &self,
        ctx: WorkflowExecutionContext,
    ) -> BoxFuture<'static, Result<WorkflowExecutorResult, ToolError>> {
        let shared = Arc::clone(&self.shared);
        Box::pin(async move {
            let invocation = ensure_invocation(&shared)?;
            if ctx.job.kind != JobKind::Tool {
                return Err(Box::new(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "tool executor received a Worker job",
                )) as ToolError);
            }
            let Some(name) = ToolName::from_canonical(&ctx.job.name) else {
                // admission の `tools.allow` 検査が先に弾くため到達しない
                // (TS は `ctx.job.tool as ToolName` cast で素通し —
                // 到達不能 input の防衛として canonical 名のみ通す)。
                return Err(Box::new(
                    CheaperError::new(
                        ErrorCode::UnknownTool,
                        format!("unknown or unregistered tool: {}", ctx.job.name),
                    )
                    .with_details(Map::from_iter([(
                        "tool".to_string(),
                        Value::String(ctx.job.name.clone()),
                    )])),
                ) as ToolError);
            };
            let journal = Arc::new(MutationJournal::new());
            let deny: Vec<&str> = shared
                .policy
                .filesystem
                .deny_paths
                .iter()
                .map(String::as_str)
                .collect();
            let scope = invocation.create_worker_scope(WorkerScopeOptions {
                owner_id: ctx.job_id.clone(),
                runtime: ToolRuntimeOptions {
                    resource_manifest: manifest_value(ctx.job.manifest.as_ref()),
                    write_scope: None,
                    read_strictness: None,
                    artifacts: None,
                    process: None,
                    root: shared.root.clone(),
                    symlink_policy: shared.policy.filesystem.symlinks,
                    names: vec![name],
                    max_tool_calls: shared.policy.execution.max_tool_calls,
                    on_limit: Box::new(|| {}),
                    journal: Some(Arc::clone(&journal)),
                    denylist: Some(create_path_denylist(&deny)),
                    parent_lease: Some(ctx.parent_lease),
                    signal: None,
                    abort_grace_ms: None,
                    concurrency: None,
                    eval_spawn: None,
                },
            })?;
            let executed = scope
                .execute(
                    name.as_str(),
                    &ctx.input,
                    Some(ToolCallContext {
                        signal: Some(ctx.signal.clone()),
                    }),
                )
                .await;
            // `finally { await scope.dispose() }` — meta の構築と
            // dispose は成否に関わらず実行する。
            let meta = tool_meta(scope.budget().tool_calls(), &journal);
            scope.dispose().await;
            match executed {
                Ok(result) => Ok(WorkflowExecutorResult {
                    result: serde_json::to_value(&result).map_err(|e| -> ToolError {
                        Box::new(CheaperError::new(ErrorCode::InternalError, e.to_string()))
                    })?,
                    meta: Some(meta),
                }),
                Err(error) => Err(Box::new(tool_executor_error(error.as_ref(), meta)) as ToolError),
            }
        })
    }
}

/// `catch` 側の wrap — `isCheaperError(error) ? error.code :
/// "TOOL_EXECUTION_FAILURE"`、`message` は `error.message` 相当。
fn tool_executor_error(
    error: &(dyn std::error::Error + Send + Sync + 'static),
    meta: Map<String, Value>,
) -> WorkflowExecutorError {
    let code = find_cheaper_error(error)
        .map(|c| c.code())
        .unwrap_or(ErrorCode::ToolExecutionFailure);
    WorkflowExecutorError::new(code, error.to_string(), meta).unwrap_or_else(|_| {
        WorkflowExecutorError::new(
            ErrorCode::ToolExecutionFailure,
            "tool execution failed",
            Map::new(),
        )
        .expect("empty meta は必ず構築可能")
    })
}

// ---------------------------------------------------------------------------
// lease / abort / metrics callback
// ---------------------------------------------------------------------------

/// `acquireJobLease`/`releaseJobLease`/`abort`/`executionMetrics` を
/// shared runtime 経由に接続する。
fn build_ports(shared: &Arc<Shared>) -> WorkflowEnginePorts {
    let acquire_shared = Arc::clone(shared);
    let release_shared = Arc::clone(shared);
    let abort_shared = Arc::clone(shared);
    let metrics_shared = Arc::clone(shared);
    WorkflowEnginePorts {
        worker: Arc::new(WorkerJobExecutor {
            shared: Arc::clone(shared),
        }),
        tool: Arc::new(ToolJobExecutor {
            shared: Arc::clone(shared),
        }),
        acquire_job_lease: Arc::new(move |request: JobLeaseRequest| {
            let shared = Arc::clone(&acquire_shared);
            Box::pin(async move {
                let names = match request.job.kind {
                    JobKind::Tool => {
                        // `tools.allow` に無い tool は admission で弾く
                        // (`POLICY_VIOLATION`)。
                        match shared
                            .policy
                            .tools
                            .allow
                            .iter()
                            .copied()
                            .find(|t| t.as_str() == request.job.name)
                        {
                            Some(name) => vec![name],
                            None => {
                                return Err(Box::new(CheaperError::new(
                                    ErrorCode::PolicyViolation,
                                    format!("tool not allowed: {}", request.job.name),
                                )) as ToolError)
                            }
                        }
                    }
                    JobKind::Worker => {
                        let Some(worker) = shared.worker(&request.job.name) else {
                            return Err(Box::new(CheaperError::new(
                                ErrorCode::WorkerNotFound,
                                format!("worker not found: {}", request.job.name),
                            )) as ToolError);
                        };
                        check_capabilities(&worker, &shared.policy)
                            .map_err(|e| Box::new(e) as ToolError)?
                    }
                };
                let rt = ensure_invocation(&shared)?;
                rt.acquire_job_lease(JobLeaseOptions {
                    owner_id: request.job_id,
                    names,
                    signal: Some(request.signal),
                    resource_manifest: manifest_value(request.job.manifest.as_ref()),
                    write_scope: None,
                    read_strictness: None,
                })
                .await
                .map_err(|e| Box::new(e) as ToolError)
            }) as BoxFuture<'static, Result<ResourceLease, ToolError>>
        }),
        release_job_lease: Arc::new(move |lease: ResourceLease| {
            if let Some(rt) = release_shared
                .invocation
                .lock()
                .expect("invocation slot poisoned")
                .as_ref()
            {
                rt.release_job_lease(lease);
            }
        }),
        abort: Arc::new(move |reason: &AbortReason| {
            abort_shared.controller.abort(reason.clone());
            if let Some(rt) = abort_shared
                .invocation
                .lock()
                .expect("invocation slot poisoned")
                .as_ref()
            {
                rt.abort(Some(reason.clone()));
            }
            if let Some(store) = abort_shared
                .store
                .lock()
                .expect("store slot poisoned")
                .as_ref()
            {
                store.dispose();
            }
        }),
        execution_metrics: Arc::new(move |job_id: Option<&str>| {
            metrics_shared
                .invocation
                .lock()
                .expect("invocation slot poisoned")
                .as_ref()
                .map(|rt| rt.execution_metrics(job_id))
        }),
    }
}

// ---------------------------------------------------------------------------
// signal 中継(hooks.signal + SIGINT/SIGTERM → AbortController)
// ---------------------------------------------------------------------------

/// `hooks.signal` + `process.on("SIGINT"/"SIGTERM")` を workflow 集約
/// controller へ中継する guard。`run.rs::SignalRelay` と同じ構造だが
/// external abort の reason を forward する(TS
/// `abortController.abort(external?.reason)` — `run` 側は reason を
/// 捨てて既定 AbortError に揃える差分がある)。
struct WorkflowSignalRelay {
    os_task: tokio::task::JoinHandle<()>,
    external: Option<(AbortSignal, AbortListenerId)>,
}

impl WorkflowSignalRelay {
    fn install(hooks: &CliHooks, controller: &AbortController) -> Self {
        let external = hooks.signal.as_ref().map(|signal| {
            let ac = controller.clone();
            // `onSignal` — external reason をそのまま abort() へ渡す。
            let id = signal.on_abort(move |reason: &AbortReason| {
                ac.abort(reason.clone());
            });
            // `if (external?.aborted) onSignal()` — 登録済みでも
            // abort 済み signal は発火しないため即時反映する。
            if signal.is_aborted() {
                controller.abort(signal.reason().unwrap_or_else(AbortReason::aborted));
            }
            (signal.clone(), id)
        });
        let ac = controller.clone();
        let os_task = tokio::spawn(async move {
            wait_os_signal().await;
            ac.abort(AbortReason::aborted());
        });
        Self { os_task, external }
    }
}

impl Drop for WorkflowSignalRelay {
    fn drop(&mut self) {
        self.os_task.abort();
        if let Some((signal, id)) = self.external.take() {
            signal.remove_listener(id);
        }
    }
}

#[cfg(unix)]
async fn wait_os_signal() {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
        }
        // SIGTERM listener 構築失敗時は SIGINT のみ中継する。
        Err(_) => {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(not(unix))]
async fn wait_os_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

// ---------------------------------------------------------------------------
// `executeWorkflowInterface` / `runWorkflowCommand`
// ---------------------------------------------------------------------------

/// `executeWorkflowInterface` — metadata 検証と公開 interface 射影のみ
/// (worker/tool/model/runtime は実行しない)。
/// `cheaper.cli.command` span(design §instrumentation 配置)。
pub async fn execute_interface(
    args: &WorkflowInterfaceCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    execute_interface_inner(args, io, hooks, started_at)
        .instrument(tracing::info_span!(
            "cheaper.cli.command",
            command = "workflow-interface",
        ))
        .await
}

async fn execute_interface_inner(
    args: &WorkflowInterfaceCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    let ctx = prepare_workflow_context(args.cwd.as_deref(), io, hooks)?;
    let loaded = ctx.load_workflow(&args.path).await?;
    let interface = loaded.compiler.project_interface(loaded.definition.raw())?;
    let result = match serde_json::to_value(&interface).map_err(BoxError::from)? {
        Value::Object(m) => m,
        other => unreachable!("WorkflowInterface は常に object: {other:?}"),
    };
    Ok(CommandOut {
        envelope: utility_success_envelope(result, now(hooks) - started_at),
        exit_code: 0,
        model_line: None,
    })
}

/// `runWorkflowCommand`。
/// `cheaper.cli.command` span(design §instrumentation 配置)。
pub async fn execute_run(
    args: &WorkflowRunCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    _started_at: f64,
) -> Result<CommandOut, BoxError> {
    execute_run_inner(args, io, hooks, _started_at)
        .instrument(tracing::info_span!(
            "cheaper.cli.command",
            command = "workflow-run",
        ))
        .await
}

async fn execute_run_inner(
    args: &WorkflowRunCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    _started_at: f64,
) -> Result<CommandOut, BoxError> {
    let ctx = prepare_workflow_context(args.cwd.as_deref(), io, hooks)?;
    let inputs = match &args.input {
        None => Map::new(),
        Some(input) => read_task_input(input, io.stdin)?,
    };
    let timeout_ms = match &args.timeout {
        Some(t) => Some(parse_duration(t).map_err(|_| {
            CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("invalid --timeout duration: {t}"),
            )
        })?),
        None => None,
    };
    let effective_policy = restrict_policy(
        &ctx.policy,
        &PolicyRestrictions {
            timeout_ms,
            concurrency: None,
        },
    )?;
    // TS の `workflowPolicy === undefined` 分岐は Rust の型では
    // あり得ない(`RuntimePolicy.workflow` は必ず resolve 済み)。
    let loaded = ctx.load_workflow(&args.path).await?;
    let workflow = loaded
        .compiler
        .compile(loaded.definition.raw(), Some(&Value::Object(inputs)))
        .await?;

    let controller = AbortController::new();
    let relay = WorkflowSignalRelay::install(hooks, &controller);
    #[cfg(feature = "dev-seam")]
    let env = ["NODE_ENV", "CHEAPER_TEST_FAKE_STREAM"]
        .into_iter()
        .filter_map(|name| (io.env)(name).map(|v| (name.to_string(), v)))
        .collect::<BTreeMap<_, _>>();
    let shared = Arc::new(Shared {
        root: ctx.root.clone(),
        user_config_dir: ctx.dirs.user_config_dir.to_string_lossy().into_owned(),
        project_dir: ctx.dirs.project_dir.to_string_lossy().into_owned(),
        policy: effective_policy.clone(),
        workers: ctx
            .worker_index()
            .expect("load_workflow が worker index を設定済み"),
        #[cfg(feature = "dev-seam")]
        env,
        controller: controller.clone(),
        invocation: Mutex::new(None),
        store: Mutex::new(None),
        ledger: Arc::new(Mutex::new(Vec::new())),
        model: Mutex::new(None),
    });
    let ports = build_ports(&shared);
    let now_clock: Arc<dyn Fn() -> f64 + Send + Sync> = match hooks.now.clone() {
        Some(f) => f,
        None => Arc::new(system_now),
    };
    let outcome = run_workflow(WorkflowRunOptions {
        workflow: &workflow,
        ports,
        max_jobs: effective_policy.workflow.max_jobs as f64,
        timeout_ms: effective_policy.execution.timeout_ms as f64,
        signal: Some(controller.signal()),
        abort_grace_ms: None,
        now: Some(now_clock),
    })
    .await;
    // `finally` — listener/task/store/invocation の解放を outcome 評価
    // より先に行う。
    drop(relay);
    if let Some(store) = shared.store.lock().expect("store slot poisoned").take() {
        store.dispose();
    }
    let invocation = shared
        .invocation
        .lock()
        .expect("invocation slot poisoned")
        .take();
    if let Some(rt) = invocation {
        rt.dispose().await;
    }
    let outcome = outcome?;
    Ok(CommandOut {
        envelope: assemble_envelope(&shared, outcome),
        exit_code: 0,
        model_line: None,
    })
}

/// outcome → `workflow_envelope` 投影。`meta` は engine meta +
/// `artifacts`(put 順、preview なし)。
fn assemble_envelope(shared: &Shared, outcome: WorkflowOutcome) -> Value {
    let success = outcome.ok;
    let artifacts: Vec<Value> = shared
        .ledger
        .lock()
        .expect("ledger poisoned")
        .iter()
        .map(|reference| {
            serde_json::to_value(project_artifact_metadata(
                reference,
                ArtifactProjectionOptions {
                    include_preview: Some(false),
                },
            ))
            .unwrap_or(Value::Null)
        })
        .collect();
    let mut meta = match serde_json::to_value(&outcome.meta) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    };
    meta.insert("artifacts".to_string(), Value::Array(artifacts));
    let jobs = match serde_json::to_value(&outcome.jobs) {
        Ok(Value::Array(v)) => v,
        _ => Vec::new(),
    };
    let error = if !success {
        outcome
            .error
            .as_ref()
            .and_then(|e| serde_json::to_value(e).ok())
    } else {
        None
    };
    workflow_envelope(success, outcome.result, jobs, meta, error)
}
