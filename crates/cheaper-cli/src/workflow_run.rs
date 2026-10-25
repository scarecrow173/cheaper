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
use cheaper_tools::operation::{AbortController, AbortReason};
use cheaper_tools::registry::{ToolCallContext, ToolRuntimeOptions};
use cheaper_tools::resources::ResourceLease;
use cheaper_tools::tools::process::find_cheaper_error;
use cheaper_tools::ToolError;
use cheaper_workflow::{
    project_artifact_metadata, run_workflow, ArtifactProjectionOptions, ArtifactStoreOptions,
    CompiledWorkflow, JobExecutor, JobKind, JobLeaseRequest, RunArtifactStore, WorkflowEnginePorts,
    WorkflowExecutionContext, WorkflowExecutorError, WorkflowExecutorResult, WorkflowOutcome,
    WorkflowRunOptions,
};
use serde::Serialize;
use serde_json::{Map, Value};
use tracing::Instrument;

use crate::args::{WorkflowInterfaceCliArgs, WorkflowRunCliArgs};
use crate::cli::{now, system_now, BoxError, CliHooks, CliIo, CommandOut};
use crate::compose::create_runtime;
#[cfg(not(feature = "dev-seam"))]
use crate::compose::production_session_factory;
use crate::envelope::{utility_success_envelope, workflow_envelope};
#[cfg(feature = "dev-seam")]
use crate::fake_stream::load_test_stream;
use crate::input::read_task_input;
use crate::workflow_context::{prepare_workflow_context, SharedToolError, WorkflowContext};

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
pub(crate) struct Shared {
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
    /// `--grant` 由来の invocation grant 名(宣言順)— `kind: worker`
    /// job の `RunWorkerDeps.invocation_grants`(`grants.invocation`
    /// span field)へ渡す。
    invocation_grants: Vec<String>,
    route_exec: Option<Arc<crate::route::run::RouteExecution>>,
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
    let session_factory: Box<dyn SessionFactory> =
        crate::compose::session_factory(fixture.as_ref());
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
    let reader: Arc<dyn cheaper_tools::artifact::ArtifactReader> = match &shared.route_exec {
        Some(route_exec) => route_exec.compose_reader(store.reader()),
        None => store.reader(),
    };
    let rt = create_invocation_tool_runtime(InvocationToolRuntimeOptions {
        artifacts: Some(ArtifactPorts {
            reader: Some(reader),
            publisher: Some(publisher),
        }),
        // `exec: effectivePolicy.exec` — resolved policy を
        // serialize して unified exec-doc raw JSON として渡す
        // (invocation 側が `resolve_exec_policy` で再 resolve し
        // inner.exec と parity check する)。
        exec: Some(
            serde_json::to_value(&shared.policy.exec).map_err(|e| -> ToolError {
                Box::new(CheaperError::new(ErrorCode::InternalError, e.to_string()))
            })?,
        ),
        // `security` も `exec` と同型に resolved policy
        // を serialize して渡す(issue #164 — invocation runtime の
        // parity inner 値)。
        security: Some(serde_json::to_value(&shared.policy.security).map_err(
            |e| -> ToolError {
                Box::new(CheaperError::new(ErrorCode::InternalError, e.to_string()))
            },
        )?),
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
                        invocation_grants: &inner.invocation_grants,
                        // workflow job は checkpoint/resume 対象外
                        // (issue #111 design §3b — run 経路のみ)。
                        resume: None,
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
            let route_exec = shared
                .route_exec
                .as_ref()
                .filter(|_| ctx.job_id == "exec" && ctx.job.name == "command");
            if let Some(route_exec) = route_exec {
                if ctx.input != *route_exec.expected_input() {
                    return Err(Box::new(
                        CheaperError::new(
                            ErrorCode::PolicyViolation,
                            "routed exec job input does not match the prepared command",
                        )
                        .with_details(Map::from_iter([(
                            "reason".to_string(),
                            Value::from("exec-input-mismatch"),
                        )])),
                    ) as ToolError);
                }
            }
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
                    // `exec: None` — parity check を skip し、
                    // invocation inner の exec/security が再注入される
                    // (issue #164/#165)。
                    exec: None,
                    security: None,
                    root: shared.root.clone(),
                    symlink_policy: shared.policy.filesystem.symlinks,
                    names: vec![name],
                    // `kind: tool` job は worker frontmatter を持たず
                    // opt-in 不能(`kind: worker` job は run 経路で
                    // worker 定義の `optional_tools` が流れる)。
                    degradable: Vec::new(),
                    max_tool_calls: shared.policy.execution.max_tool_calls,
                    on_limit: Box::new(|| {}),
                    journal: Some(Arc::clone(&journal)),
                    denylist: Some(create_path_denylist(&deny)),
                    parent_lease: Some(ctx.parent_lease),
                    signal: None,
                    abort_grace_ms: None,
                    concurrency: None,
                    eval_spawn: None,
                    route_context: route_exec.map(|r| r.context()),
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
                Ok(mut result) => {
                    if let Some(route_exec) = route_exec {
                        route_exec.complete(&mut result);
                    }
                    Ok(WorkflowExecutorResult {
                        result: serde_json::to_value(&result).map_err(|e| -> ToolError {
                            Box::new(CheaperError::new(ErrorCode::InternalError, e.to_string()))
                        })?,
                        meta: Some(meta),
                    })
                }
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
    // interface 検査経路に grant を流す意味はない — `None` 固定。
    let ctx = prepare_workflow_context(args.cwd.as_deref(), io, hooks, None)?;
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
    // `--grant` 値列 → 合成 invocation layer + 宣言順 names
    // (`run` 経路と共有する `grant.rs` builder)。
    let (invocation_layer, grant_names) = crate::grant::build_invocation_layer(&args.grants)?;
    let ctx = prepare_workflow_context(args.cwd.as_deref(), io, hooks, invocation_layer.as_ref())?;
    // `grant: <names>` — `kind: tool` job は `cheaper.run.tool_scope`
    // span を通らないため workflow 経路ではこの stderr 行が唯一の
    // audit surface(run 経路と同じ diagnostics 規約、`--quiet` で抑制)。
    if !grant_names.is_empty() && !args.quiet {
        io.stderr
            .write_str(&format!("grant: {}\n", grant_names.join(", ")));
    }
    let inputs = match &args.input {
        None => Map::new(),
        Some(input) => read_task_input(input, io.stdin)?,
    };
    let timeout_ms = match &args.timeout {
        Some(t) => Some(parse_duration(t).map_err(|_| crate::errors::invalid_timeout(t))?),
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
    let (shared, outcome) = execute_compiled(CompiledRun {
        ctx: &ctx,
        workflow: &workflow,
        effective_policy: &effective_policy,
        invocation_grants: grant_names,
        route_exec: None,
        io,
        hooks,
    })
    .await?;
    let outcome = outcome?;
    Ok(CommandOut {
        envelope: assemble_envelope(&shared, outcome),
        exit_code: 0,
        model_line: None,
    })
}

pub(crate) struct CompiledRun<'ctx, 'b, 'io> {
    pub(crate) ctx: &'ctx WorkflowContext,
    pub(crate) workflow: &'b CompiledWorkflow,
    pub(crate) effective_policy: &'b RuntimePolicy,
    pub(crate) invocation_grants: Vec<String>,
    pub(crate) route_exec: Option<Arc<crate::route::run::RouteExecution>>,
    // `io` は `#[cfg(feature = "dev-seam")]` 配下の env 採取
    // (execute_compiled 内の NODE_ENV/CHEAPER_TEST_FAKE_STREAM capture)
    // でのみ読まれる — 非 seam build では未使用になるため cfg 付きで
    // dead_code を許容する(cheaper-bench の default-feature dep edge が
    // 非 seam 側 compile を初めて通す — issue #183)。
    #[cfg_attr(not(feature = "dev-seam"), allow(dead_code))]
    pub(crate) io: &'b CliIo<'io>,
    pub(crate) hooks: &'b CliHooks,
}

pub(crate) async fn execute_compiled(
    run: CompiledRun<'_, '_, '_>,
) -> Result<(Arc<Shared>, Result<WorkflowOutcome, CheaperError>), BoxError> {
    let controller = AbortController::new();
    let relay = crate::signal::SignalRelay::install_forwarding(&controller, run.hooks);
    #[cfg(feature = "dev-seam")]
    let env = ["NODE_ENV", "CHEAPER_TEST_FAKE_STREAM"]
        .into_iter()
        .filter_map(|name| (run.io.env)(name).map(|v| (name.to_string(), v)))
        .collect::<BTreeMap<_, _>>();
    let shared = Arc::new(Shared {
        root: run.ctx.root.clone(),
        user_config_dir: run.ctx.dirs.user_config_dir.to_string_lossy().into_owned(),
        project_dir: run.ctx.dirs.project_dir.to_string_lossy().into_owned(),
        policy: run.effective_policy.clone(),
        workers: run
            .ctx
            .worker_index()
            .expect("load_workflow が worker index を設定済み"),
        #[cfg(feature = "dev-seam")]
        env,
        controller: controller.clone(),
        invocation: Mutex::new(None),
        store: Mutex::new(None),
        ledger: Arc::new(Mutex::new(Vec::new())),
        model: Mutex::new(None),
        invocation_grants: run.invocation_grants,
        route_exec: run.route_exec,
    });
    let ports = build_ports(&shared);
    let now_clock: Arc<dyn Fn() -> f64 + Send + Sync> = match run.hooks.now.clone() {
        Some(f) => f,
        None => Arc::new(system_now),
    };
    let outcome = run_workflow(WorkflowRunOptions {
        workflow: run.workflow,
        ports,
        max_jobs: run.effective_policy.workflow.max_jobs as f64,
        timeout_ms: run.effective_policy.execution.timeout_ms as f64,
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
    Ok((shared, outcome))
}

pub(crate) struct RoutedRun<'ctx, 'b, 'io> {
    pub ctx: &'ctx WorkflowContext,
    pub file_root: &'b str,
    pub workflow_path: &'b str,
    pub inputs: Map<String, Value>,
    pub effective_policy: &'b RuntimePolicy,
    pub route_exec: Arc<crate::route::run::RouteExecution>,
    pub io: &'b CliIo<'io>,
    pub hooks: &'b CliHooks,
}

pub(crate) async fn execute_routed(
    run: RoutedRun<'_, '_, '_>,
) -> Result<Result<WorkflowOutcome, CheaperError>, BoxError> {
    let loaded = run
        .ctx
        .load_workflow_from(run.workflow_path, run.file_root)
        .await?;
    validate_routed_convention(loaded.definition.raw())?;
    let workflow = loaded
        .compiler
        .compile(loaded.definition.raw(), Some(&Value::Object(run.inputs)))
        .await?;
    let (_shared, outcome) = execute_compiled(CompiledRun {
        ctx: run.ctx,
        workflow: &workflow,
        effective_policy: run.effective_policy,
        invocation_grants: Vec::new(),
        route_exec: Some(run.route_exec),
        io: run.io,
        hooks: run.hooks,
    })
    .await?;
    Ok(outcome)
}

fn validate_routed_convention(raw: &Value) -> Result<(), CheaperError> {
    let invalid = |reason: &'static str| {
        CheaperError::new(
            ErrorCode::InvalidRuntimePolicy,
            "routed workflow does not satisfy the routed-exec convention",
        )
        .with_details(Map::from_iter([(
            "reason".to_string(),
            Value::from(reason),
        )]))
    };
    let exec = raw
        .get("jobs")
        .and_then(Value::as_object)
        .and_then(|jobs| jobs.get("exec"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("missing-exec-job"))?;
    if exec.get("kind").and_then(Value::as_str) != Some("tool")
        || exec.get("tool").and_then(Value::as_str) != Some("command")
        || exec.contains_key("when")
    {
        return Err(invalid("exec-job-shape"));
    }
    let exec_result = raw
        .get("outputs")
        .and_then(Value::as_object)
        .and_then(|outputs| outputs.get("execResult"))
        .and_then(Value::as_object)
        .and_then(|output| output.get("value"))
        .and_then(Value::as_object);
    let whole_exec = exec_result.is_some_and(|binding| {
        binding.get("$from").and_then(Value::as_str) == Some("exec")
            && binding
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .is_empty()
            && binding.keys().all(|k| k == "$from" || k == "path")
    });
    if !whole_exec {
        return Err(invalid("execResult-output"));
    }
    Ok(())
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
