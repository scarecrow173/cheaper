//! `runWorker`(packages/runtime-pi/src/run.ts)の Rust port — worker run
//! orchestration 本体。
//!
//! 責務順序(TS と同一):
//! 1. `checkCapabilities`(worker tools ⊆ policy tools)
//! 2. run 集約 abort(`RunAbort`)— timeout / 外部 signal / maxTurns /
//!    maxToolCalls の全 trigger が `runAbort.signal` を共有し、reason は
//!    先勝ちで error 分類の正になる。
//! 3. invocation tool runtime(borrowed `deps.execution` or owned 生成)
//!    → worker scope(ledger journal・meter・budget 接続)
//! 4. model 解決(catalog → ModelInfo → `resolve_model` /
//!    `resolve_explicit_model`)→ catalog entry + auth 解決
//! 5. session 生成(`SessionFactory` — production は `RigSessionFactory`)
//!    → `prompt_with_abort_grace` で stream 駆動 + maxTurns 監視
//! 6. `extract_output` → 失敗なら `repair_output`
//! 7. `build_meta`(accumulator snapshot 正源)→ result or
//!    `WorkerRunError`(abort 分類 / PARTIAL_MUTATION wrap / runMeta)
//!
//! Session 差し替え seam(`WorkerSession`/`SessionFactory`)は T3 fake と
//! T4 stream_fn 相当の両方を支える最小 boundary。Rig type は
//! `cheaper-rig-adapter` の opaque `CompletionModel`/`RigSession` のみで、
//! runtime の public API から rig crate 型は露出しない。

use std::sync::Arc;
use std::time::Duration;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::classify::ModelInfo;
use cheaper_core::model::config::ModelConfig;
use cheaper_core::model::model_ref::parse_model_ref;
use cheaper_core::model::pricing::ModelCost;
use cheaper_core::model::resolve::{
    resolve_explicit_model, resolve_model, ModelPolicySlice, ResolveExplicitModelOptions,
    ResolveModelOptions, ResolvedModel,
};
use cheaper_core::policy::capability::check_capabilities;
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::schema::CompiledSchema;
use cheaper_core::worker::validate::WorkerDefinition;
use cheaper_core::worker::write_scope::extract_write_scope;
use cheaper_core::workspace::deny_paths::create_path_denylist;
use cheaper_rig_adapter::{
    build_system_prompt, build_user_prompt, CompletionModel, RigSession, RunEvent, RunSnapshot,
    RunStream, WorkerSessionSpec, DEFAULT_INVALID_TOOL_RETRIES, DEFAULT_MAX_TURNS,
};
use cheaper_tools::invocation::{
    create_invocation_tool_runtime, InvocationToolRuntime, InvocationToolRuntimeOptions,
    WorkerScopeOptions,
};
use cheaper_tools::journal::MutationJournal;
use cheaper_tools::operation::{AbortReason, AbortSignal, Clock};
use cheaper_tools::registry::{ToolRuntime, ToolRuntimeOptions};
use cheaper_tools::resources::ResourceLease;
use serde_json::{Map, Value};

use crate::abort::{RunAbort, RunAbortReason};
use crate::auth::ResolvedAuth;
use crate::catalog::CatalogModel;
use crate::grace::{prompt_with_abort_grace, PromptOutcome};
use crate::meta::{build_meta, merge_snapshots, MetaInputs, WorkerRunMeta};
use crate::output::{extract_output, ExtractedOutput};
use crate::panic_boundary::catch_run_panic;
use crate::repair::{repair_output, RepairOptions};
use crate::run_error::{PartialRunMeta, WorkerRunError};
use crate::runtime::Runtime;

/// `promptWithAbortGrace` の既定 grace(TS `abortGraceMs` default)。
pub const DEFAULT_ABORT_GRACE_MS: f64 = 5000.0;

// ---------------------------------------------------------------------------
// Session seam — Rig type を露出しない session 差し替え境界
// ---------------------------------------------------------------------------

/// session 生成入力(TS `createWorkerSession` options 相当)。repair
/// session では `tool_runtime: None`。`output_schema` は main/repair
/// 双方で worker output schema を渡す(repair では repair schema)。
///
/// `output_schema` は provider 側 json_schema mode 等への hint —
/// `CompletionModel` 生成経路(`CompletionModelFactory`)でのみ使われ、
/// adapter の `WorkerSessionSpec` には載らない(Rig 側の配線は
/// adapter/provider の責務、未配線でも prompt + 回数上限が contract を
/// 担保する)。TS に該当 option はなく Rust 側の拡張 seam(F8)。
pub struct SessionSpec<'a> {
    /// 解決済み catalog model。
    pub model: &'a CatalogModel,
    /// 解決済み auth(api key / headers)。未設定 provider では `None`。
    pub auth: Option<&'a ResolvedAuth>,
    /// system prompt(`build_system_prompt` or repair prompt 生成済み)。
    pub system_prompt: String,
    /// tool runtime binding。`None` = tool-less session(repair)。
    pub tool_runtime: Option<ToolRuntime>,
    /// provider 向け structured-output hint。
    pub output_schema: Option<&'a CompiledSchema>,
    /// `policy.output.maxTokens`。
    pub max_output_tokens: u64,
    /// `policy.retry.transientAttempts`。
    pub transient_retries: usize,
    /// `policy.execution.maxTurns` — runtime 側の正規 turn 上限。
    /// Rig の `max_turns` は runaway 対策の内部 guard であり、cheaper
    /// の maxTurns abort(T3 が stream event から分類)より先に発火して
    /// はいけないため、factory は `max(internal_max_turns, policy_max_
    /// turns)` を内部 budget に使う(F7)。
    pub policy_max_turns: u64,
}

/// `session.prompt`/`session.abort`/`collector.snapshot` を束ねた
/// run 経路の session 面。`RigSession` 以外にも scripted fake を
/// 差せるよう runtime 側で trait 化する。
pub trait WorkerSession: Send + Sync {
    /// `session.prompt()` 相当 — streaming run の唯一の実行面。
    /// `signal` は run 集約 abort signal。
    fn stream_run(&self, prompt: String, signal: Option<AbortSignal>) -> RunStream;
    /// `session.abort()` 相当の session-local cancel 要求。signal 経路で
    /// 既に abort が伝播する実装(`RigSession`)では no-op でよい —
    /// `prompt_with_abort_grace` が grace 計測の trigger として呼ぶ。
    fn abort(&self);
    /// session accumulator snapshot(main + repair で個別に畳む)。
    fn snapshot(&self) -> RunSnapshot;
}

/// `deps.sessionFactory` 相当 — provider/model 差し替え seam。
/// runtime 呼び出し側は opaque session を受け取り、Rig type を触らない。
pub trait SessionFactory: Send + Sync {
    fn create_session(
        &self,
        spec: &SessionSpec<'_>,
    ) -> Result<Arc<dyn WorkerSession>, CheaperError>;
}

/// provider 固有の model 構築(`CompletionModel::from_rig` 呼び出しは
/// provider/plugin 実装側の責務 — runtime は opaque `CompletionModel`
/// のみを受け取る)。
pub trait CompletionModelFactory: Send + Sync {
    fn completion_model(&self, spec: &SessionSpec<'_>) -> Result<CompletionModel, CheaperError>;
}

/// production session factory — `CompletionModelFactory` で opaque model
/// を得て `RigSession::build` に渡す。
pub struct RigSessionFactory<F> {
    pub model_factory: F,
    /// agent 内部 turn guard(`DEFAULT_MAX_TURNS`)。
    pub internal_max_turns: usize,
    /// invalid tool call の retry 上限(`DEFAULT_INVALID_TOOL_RETRIES`)。
    pub invalid_tool_retries: usize,
}

impl<F> RigSessionFactory<F> {
    pub fn new(model_factory: F) -> Self {
        Self {
            model_factory,
            internal_max_turns: DEFAULT_MAX_TURNS,
            invalid_tool_retries: DEFAULT_INVALID_TOOL_RETRIES,
        }
    }
}

impl<F: CompletionModelFactory> SessionFactory for RigSessionFactory<F> {
    fn create_session(
        &self,
        spec: &SessionSpec<'_>,
    ) -> Result<Arc<dyn WorkerSession>, CheaperError> {
        let model = self.model_factory.completion_model(spec)?;
        let internal_max_turns = self.effective_internal_max_turns(spec.policy_max_turns);
        Ok(Arc::new(RigSession::build(WorkerSessionSpec {
            model,
            system_prompt: spec.system_prompt.clone(),
            tool_runtime: spec.tool_runtime.clone(),
            max_output_tokens: spec.max_output_tokens,
            max_turns: internal_max_turns,
            // F6: Rig の invalid-tool retry は model-call budget
            // (`max_turns`)を消費する — retry 上限が turn guard 未満だと
            // `invalid_tool_call_retries` 枯渇の PROVIDER_FAILURE が
            // cheaper の `maxTurns` 分類(`MAX_TURNS_EXCEEDED` abort)より
            // 先に発火し得る。Pi には invalid-call 固有の予算が無く
            // turn 数のみが上限なので、内部 guard で floor する。
            invalid_tool_retries: self.invalid_tool_retries.max(internal_max_turns),
            transient_retries: spec.transient_retries,
        })))
    }
}

impl<F> RigSessionFactory<F> {
    /// Rig 内部 turn guard の実効値 — `max(internal_max_turns,
    /// policy_max_turns)`(F7)。policy maxTurns が guard を上回ると
    /// runaway 対策が cheaper の maxTurns 分類(`MAX_TURNS_EXCEEDED`
    /// abort)より先に発火し、error code・event 列ともに TS からずれる。
    fn effective_internal_max_turns(&self, policy_max_turns: u64) -> usize {
        let policy = usize::try_from(policy_max_turns).unwrap_or(usize::MAX);
        self.internal_max_turns.max(policy)
    }
}

impl WorkerSession for RigSession {
    fn stream_run(&self, prompt: String, signal: Option<AbortSignal>) -> RunStream {
        RigSession::stream_run(self, prompt, signal)
    }

    /// `RigSession` の cancel は `stream_run` の signal select が担う
    /// (agent は stateless per-prompt で stream drop が run cancel 相当)。
    /// `session.abort()` は promptWithAbortGrace の trigger として呼ば
    /// れるためここでは no-op。
    fn abort(&self) {}

    fn snapshot(&self) -> RunSnapshot {
        RigSession::snapshot(self)
    }
}

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

    // `setTimeout(onTimeout, timeoutMs)`。
    let timeout_task = {
        let abort = Arc::clone(&run_abort);
        let timeout_ms = policy.execution.timeout_ms;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
            abort.request_abort(RunAbortReason::Timeout);
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
    let outcome = ctx.run(&run_abort, grace_ms).await;

    // TS `finally` 相当: timeout clear・listener 解除・dispose。
    timeout_task.abort();
    if let (Some(signal), Some(id)) = (deps.signal, signal_listener) {
        signal.remove_listener(id);
    }
    ctx.dispose().await;
    outcome
}

// ---------------------------------------------------------------------------
// RunCtx — inner try 範囲の共有状態(TS のクロージャ変数相当)
// ---------------------------------------------------------------------------

struct RunCtx<'a> {
    deps: &'a RunWorkerDeps<'a>,
    /// `checkCapabilities` 結果(worker tools ⊆ policy)。
    effective_tools: Vec<ToolName>,
    /// `createLedgerMutationJournal()`。
    journal: Arc<MutationJournal>,
    owns_invocation: bool,
    owner_id: String,
    invocation: Option<InvocationToolRuntime>,
    tool_scope: Option<ToolRuntime>,
    resolved: Option<ResolvedModel>,
    catalog_model: Option<CatalogModel>,
    auth: Option<ResolvedAuth>,
    session: Option<Arc<dyn WorkerSession>>,
    /// repair session の snapshot 累計(TS shared acc 相当)。
    repair_snap: RunSnapshot,
    /// `repairs` — 消費 repair attempt 数。
    repairs: u64,
}

impl<'a> RunCtx<'a> {
    fn new(deps: &'a RunWorkerDeps<'a>, effective_tools: Vec<ToolName>) -> Self {
        Self {
            deps,
            effective_tools,
            journal: Arc::new(MutationJournal::new()),
            owns_invocation: false,
            owner_id: "worker".to_string(),
            invocation: None,
            tool_scope: None,
            resolved: None,
            catalog_model: None,
            auth: None,
            session: None,
            repair_snap: RunSnapshot::default(),
            repairs: 0,
        }
    }

    async fn dispose(self) {
        if self.owns_invocation {
            if let Some(invocation) = &self.invocation {
                invocation.dispose().await;
            }
        } else if let Some(scope) = &self.tool_scope {
            scope.dispose().await;
        }
    }

    // -- meta helpers ------------------------------------------------------

    /// outer catch の `runMeta: {execution}`(invocation 以降・meta 未構築
    /// の error path)。
    fn partial_meta(&self) -> PartialRunMeta {
        PartialRunMeta {
            execution: self
                .invocation
                .as_ref()
                .map(|i| i.execution_metrics(Some(self.owner_id.as_str()))),
            ..PartialRunMeta::default()
        }
    }

    /// `buildMeta()` — resolved 以降のみ呼べる。main + repair の
    /// snapshot を run 単位に畳み込む。
    fn full_meta(
        &self,
        result: Option<&Map<String, Value>>,
    ) -> Result<WorkerRunMeta, CheaperError> {
        let resolved = self.resolved.as_ref().ok_or_else(|| {
            CheaperError::new(
                ErrorCode::InternalError,
                "run meta requested before model resolution",
            )
        })?;
        let mut snapshot = self
            .session
            .as_ref()
            .map(|s| s.snapshot())
            .unwrap_or_default();
        merge_snapshots(&mut snapshot, &self.repair_snap);
        let source_tokens = self
            .tool_scope
            .as_ref()
            .map(|t| t.meter().source_tokens())
            .unwrap_or(0);
        build_meta(
            &MetaInputs {
                resolved,
                worker: self.deps.worker,
                source_tokens,
                changes: self.journal.finalize(),
                execution: self
                    .invocation
                    .as_ref()
                    .map(|i| i.execution_metrics(Some(self.owner_id.as_str())))
                    .unwrap_or_default(),
                repairs: self.repairs,
                snapshot,
            },
            result,
        )
    }

    /// outer catch — setup phase(invocation 以降・prompt 前)の error は
    /// `runMeta: {execution}` のみ(TS catch は buildMeta を呼ばない)。
    fn fail(&self, err: CheaperError) -> WorkerRunError {
        WorkerRunError::new(err, Some(self.partial_meta()))
    }

    /// `withRunMeta(err)` — prompt / extract / repair phase の error に
    /// full meta を付ける。meta 構築自体の失敗は TS では outer catch の
    /// INTERNAL_ERROR に落ちる(original error は失われる — 同様に畳む)。
    fn with_run_meta(&self, err: CheaperError) -> WorkerRunError {
        match self.full_meta(None) {
            Ok(meta) => WorkerRunError::new(err, Some(PartialRunMeta::from(meta))),
            Err(meta_err) => self.fail(meta_err),
        }
    }

    /// `throwClassified(err)` — abort reason を code 化し、mutation が
    /// あれば `PARTIAL_MUTATION` で wrap(cause = 元の abort error)。
    fn classified(&self, reason: RunAbortReason) -> WorkerRunError {
        let meta = match self.full_meta(None) {
            Ok(meta) => PartialRunMeta::from(meta),
            Err(meta_err) => return self.fail(meta_err),
        };
        let abort_err = reason.cheaper_error();
        let mutated = meta.changes.as_ref().is_some_and(|c| c.totals.files > 0);
        let err = if mutated {
            CheaperError::new(
                ErrorCode::PartialMutation,
                "worker aborted after partial mutations",
            )
            .with_details(Map::from_iter([(
                "changes".to_string(),
                serde_json::to_value(&meta.changes).unwrap_or_default(),
            )]))
            .with_cause(abort_err)
        } else {
            abort_err
        };
        WorkerRunError::new(err, Some(meta))
    }

    // -- inner body --------------------------------------------------------

    async fn run(
        &mut self,
        run_abort: &Arc<RunAbort>,
        grace_ms: f64,
    ) -> Result<RunWorkerOk, WorkerRunError> {
        let deps = self.deps;
        let worker = deps.worker;
        let policy = deps.policy;

        // --- invocation tool runtime(borrowed or owned) ------------------
        let (parent_lease, resource_manifest) = match &deps.execution {
            Some(binding) => {
                self.owns_invocation = false;
                self.owner_id = binding.owner_id.clone();
                self.invocation = Some(binding.runtime.clone());
                (binding.parent_lease, binding.resource_manifest.clone())
            }
            None => {
                self.owns_invocation = true;
                let invocation = create_invocation_tool_runtime(InvocationToolRuntimeOptions {
                    artifacts: None,
                    process: Some(serde_json::to_value(&policy.process).unwrap_or_default()),
                    root: deps.root.clone(),
                    // F5: `policy.filesystem.symlinks` — TS run.ts の
                    // `symlinkPolicy: policy.filesystem.symlinks` parity。
                    // hardcode した Deny では within-workspace 設定が
                    // 効かない。
                    symlink_policy: policy.filesystem.symlinks,
                    signal: None,
                    abort_grace_ms: Some(grace_ms),
                    concurrency: policy
                        .execution
                        .concurrency
                        .map(|c| serde_json::to_value(c).unwrap_or_default()),
                    clock: deps.operation_clock.clone(),
                })
                .map_err(|e| self.fail(tool_boundary_error("invocation tool runtime", e)))?;
                self.invocation = Some(invocation);
                (None, None)
            }
        };
        let invocation = self.invocation.as_ref().expect("set above").clone();
        // --- ここから runMeta:{execution} が付く error path -------------

        // --- model 解決(catalog → ModelInfo → resolve*) -----------------
        let available = deps.runtime.get_available(None).map_err(|e| self.fail(e))?;
        let available: Vec<ModelInfo> = available.iter().map(to_model_info).collect();
        let output_per_input = worker.cheaper.model.workload.map(|w| w.output_per_input);
        let resolved = match deps.explicit_model {
            None => resolve_model(&ResolveModelOptions {
                available: &available,
                requested_tier: worker.cheaper.model.tier,
                policy: model_policy_slice(policy),
                config: deps.model_config,
                output_per_input,
            })
            .map_err(|e| self.fail(e))?,
            Some(explicit) => match parse_model_ref(explicit) {
                None => {
                    return Err(self.fail(
                        CheaperError::new(
                            ErrorCode::InvalidRequest,
                            format!(
                                "invalid --model reference: {explicit} \
                                 (expected <provider>/<id>)"
                            ),
                        )
                        .with_details(Map::from_iter([
                            ("reason".to_string(), Value::from("invalid-model-ref")),
                            ("model".to_string(), Value::from(explicit)),
                        ])),
                    ));
                }
                Some(model_ref) => resolve_explicit_model(&ResolveExplicitModelOptions {
                    available: &available,
                    model_ref: &model_ref,
                    policy_max_tier: policy.model.max_tier,
                    config: deps.model_config,
                    output_per_input,
                })
                .map_err(|e| self.fail(e))?,
            },
        };
        // `toResolvedPiModel` — resolved entry を catalog で引き直し auth
        // も解決。消失は INTERNAL_ERROR。
        let (catalog_model, auth) = deps
            .runtime
            .resolve_model_auth(&resolved.model.provider, &resolved.model.id)
            .map_err(|e| self.fail(e))?
            .ok_or_else(|| {
                CheaperError::new(
                    ErrorCode::InternalError,
                    "resolved model disappeared between catalog listing and resolution",
                )
                .with_details(Map::from_iter([
                    (
                        "provider".to_string(),
                        Value::from(resolved.model.provider.clone()),
                    ),
                    ("id".to_string(), Value::from(resolved.model.id.clone())),
                ]))
            })
            .map_err(|e: CheaperError| self.fail(e))?;
        self.resolved = Some(resolved);
        self.catalog_model = Some(catalog_model);
        self.auth = auth;

        // --- worker scope ------------------------------------------------
        let write_scope = extract_write_scope(worker, deps.input)
            .map_err(|e| self.fail(e))?
            .map(|scope| scope.into_iter().collect::<Vec<String>>());
        let limit_abort = Arc::clone(run_abort);
        let scope = invocation
            .create_worker_scope(WorkerScopeOptions {
                owner_id: self.owner_id.clone(),
                runtime: ToolRuntimeOptions {
                    resource_manifest,
                    write_scope,
                    read_strictness: None,
                    artifacts: None,
                    process: Some(serde_json::to_value(&policy.process).unwrap_or_default()),
                    root: deps.root.clone(),
                    // F5: invocation runtime と同じく policy 由来。
                    symlink_policy: policy.filesystem.symlinks,
                    names: self.effective_tools.clone(),
                    max_tool_calls: policy.execution.max_tool_calls,
                    on_limit: Box::new(move || {
                        limit_abort.request_abort(RunAbortReason::MaxToolCalls);
                    }),
                    journal: Some(Arc::clone(&self.journal)),
                    denylist: Some(create_path_denylist(
                        &policy
                            .filesystem
                            .deny_paths
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>(),
                    )),
                    parent_lease,
                    signal: Some(run_abort.signal()),
                    abort_grace_ms: Some(grace_ms),
                    concurrency: policy
                        .execution
                        .concurrency
                        .map(|c| serde_json::to_value(c).unwrap_or_default()),
                    eval_spawn: None,
                },
            })
            .map_err(|e| self.fail(tool_boundary_error("worker tool scope", e)))?;
        scope
            .ready()
            .await
            .map_err(|reason| self.fail(setup_abort_error("tool scope ready", reason)))?;
        self.tool_scope = Some(scope.clone());

        // --- main session ------------------------------------------------
        let tool_names: Vec<String> = self
            .effective_tools
            .iter()
            .map(|t| t.as_str().to_string())
            .collect();
        let system_prompt = build_system_prompt(
            &worker.instructions,
            &Value::Object(worker.output_schema.schema.clone()),
            &tool_names,
        );
        let session = deps
            .session_factory
            .create_session(&SessionSpec {
                model: self.catalog_model.as_ref().expect("resolved above"),
                auth: self.auth.as_ref(),
                system_prompt,
                tool_runtime: Some(scope),
                // F8: provider 側の json_schema mode 等への hint として
                // main session にも worker output schema を渡す(TS の
                // `createWorkerSession` には outputSchema option が無く
                // prompt 埋め込みのみ — 本項目は Rust 側の拡張 seam で、
                // 未対応 provider では無視される)。
                output_schema: Some(&worker.output_schema),
                max_output_tokens: policy.output.max_tokens,
                transient_retries: policy.retry.transient_attempts as usize,
                policy_max_turns: policy.execution.max_turns,
            })
            .map_err(|e| self.fail(e))?;
        self.session = Some(Arc::clone(&session));

        // --- prompt(abort grace 付き) ------------------------------------
        if let Some(reason) = run_abort.reason() {
            return Err(self.classified(reason));
        }
        let user_prompt = build_user_prompt(deps.input);
        let max_turns = policy.execution.max_turns;
        let outcome = {
            let check_session = Arc::clone(&session);
            let check_abort = Arc::clone(run_abort);
            // F1: over-limit turn が tool call で終わった場合、その pending
            // batch の完了までは abort を遅延する。Rig の `drive_tool_calls`
            // は batch 全体の outcome を buffer し、中途で stream を drop
            // すると emit 済み `ToolCall` に対応する `ToolResult` が流れず
            // tool 実行・journal・usage 記録が欠落する。emit された
            // `ToolCall` 件数と `ToolResult` 件数の差が 0 に戻った時点で
            // abort すれば最終 batch は完全に surface される。
            let mut pending_tool_calls = 0u64;
            let mut on_event = move |event: &RunEvent| {
                match event {
                    RunEvent::ToolCall { .. } => pending_tool_calls += 1,
                    RunEvent::ToolResult { .. } => {
                        pending_tool_calls = pending_tool_calls.saturating_sub(1)
                    }
                    // rollback された turn(invalid-tool retry)では emit
                    // 済み `ToolCall` の result が来ない — 新しい
                    // completion call / retry 境界で pending を reset
                    // して滞留しないようにする。
                    RunEvent::CompletionCall { .. } | RunEvent::ModelTurnRetried { .. } => {
                        pending_tool_calls = 0
                    }
                    _ => {}
                }
                // TS: turn_end 毎に `snapshot.turns >= maxTurns &&
                // lastAssistant.stopReason === "toolUse"`。hook が item
                // 送出前に turn を計上するので per-item 読みで同等以上。
                let snap = check_session.snapshot();
                if snap.turns >= max_turns && snap.last_turn_tool_calls && pending_tool_calls == 0 {
                    check_abort.request_abort(RunAbortReason::MaxTurns);
                }
            };
            prompt_with_abort_grace(
                &session,
                user_prompt,
                &run_abort.signal(),
                grace_ms,
                &mut on_event,
            )
            .await
        };
        if let Some(reason) = run_abort.reason() {
            return Err(self.classified(reason));
        }
        // F1 fallback — over-limit turn の tool batch が stream 終端
        // (provider error 等)と衝突し deferred abort が発火しなかった
        // 場合でも、最終受理 turn が tool 継続で上限到達済みなら
        // MAX_TURNS_EXCEEDED に分類する。先勝ちの reason 記録は上で
        // 済んでいるため、ここで request しても別 reason は上書きしない。
        {
            let snap = session.snapshot();
            if snap.turns >= max_turns && snap.last_turn_tool_calls {
                run_abort.request_abort(RunAbortReason::MaxTurns);
                if let Some(reason) = run_abort.reason() {
                    return Err(self.classified(reason));
                }
            }
        }
        let raw_output = match outcome {
            PromptOutcome::Rejected(err) => return Err(self.with_run_meta(err)),
            PromptOutcome::Settled { output } => output,
            // Grace は abort 検知後のみ起きる — 上で分類済みなので
            // ここへ来るのは「signal なしで Grace」= あり得ないが、
            // extraction 対象無しとして扱う。
            PromptOutcome::Grace => None,
        };

        // --- extract → repair ---------------------------------------------
        let result = match extract_output(raw_output.as_deref(), &worker.output_schema) {
            ExtractedOutput::Ok { result, .. } => result,
            ExtractedOutput::Err {
                raw_output, errors, ..
            } => {
                // `attempts: 0` でも TS 同様 `repairOutput` に流す —
                // loop が即座に尽き INVALID_OUTPUT になる。
                // `onAttempt: (n) => { repairs = n }` — 消費 attempt 数を
                // run の repairs に記録(失敗途中分も)。
                let mut repairs = self.repairs;
                let repair_result = {
                    let mut opts = RepairOptions {
                        session_factory: deps.session_factory,
                        model: self.catalog_model.as_ref().expect("resolved above"),
                        auth: self.auth.as_ref(),
                        schema: &worker.output_schema,
                        raw_output,
                        errors,
                        attempts: policy.output.repair_attempts,
                        max_output_tokens: policy.output.max_tokens,
                        transient_retries: policy.retry.transient_attempts as usize,
                        policy_max_turns: policy.execution.max_turns,
                        signal: run_abort.signal(),
                        grace_ms,
                        on_attempt: &mut |attempt| repairs = attempt,
                    };
                    repair_output(&mut opts, &mut self.repair_snap).await
                };
                self.repairs = repairs;
                match repair_result {
                    Ok(repaired) => repaired.result,
                    Err(err) => {
                        // repair 中の timeout/signal は abort 分類を優先。
                        if let Some(reason) = run_abort.reason() {
                            return Err(self.classified(reason));
                        }
                        return Err(self.with_run_meta(err));
                    }
                }
            }
        };
        // repair が成功で解決しても、その間に abort が落ちていれば
        // success を返さない(TS と同じ post-check)。
        if let Some(reason) = run_abort.reason() {
            return Err(self.classified(reason));
        }
        self.finish(result)
    }

    /// success path — `return {result, meta: await buildMeta(result)}`。
    fn finish(&self, result: Map<String, Value>) -> Result<RunWorkerOk, WorkerRunError> {
        let meta = self.full_meta(Some(&result)).map_err(|e| self.fail(e))?;
        Ok(RunWorkerOk { result, meta })
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `catalog.getAvailable()` の `CatalogModel` → 解決用 `ModelInfo`。
/// TS `toModelInfo` は `cost`/`maxTokens` をそのまま渡す — cost は
/// catalog の必須 field を `Some` で包む。
fn to_model_info(m: &CatalogModel) -> ModelInfo {
    ModelInfo {
        provider: m.provider.clone(),
        id: m.id.clone(),
        cost: Some(ModelCost {
            input: m.cost.input,
            output: m.cost.output,
            cache_read: Some(m.cost.cache_read),
            cache_write: Some(m.cost.cache_write),
        }),
        max_tokens: Some(m.max_tokens as u64),
    }
}

fn model_policy_slice(policy: &RuntimePolicy) -> ModelPolicySlice {
    ModelPolicySlice {
        max_tier: policy.model.max_tier,
        tier_fallback: policy.model.tier_fallback,
    }
}

/// tool boundary の `ToolError`/`Box<dyn Error>` → CheaperError。
/// 中身が `CheaperError` なら code/message/details を再構築して保持
/// (cause chain は dyn error 越しに取り出せないため畳む)。
fn tool_boundary_error(
    context: &str,
    err: Box<dyn std::error::Error + Send + Sync>,
) -> CheaperError {
    if let Some(e) = err.downcast_ref::<CheaperError>() {
        let mut rebuilt = CheaperError::new(e.code(), e.message().to_string());
        if let Some(details) = e.details() {
            rebuilt = rebuilt.with_details(details.clone());
        }
        return rebuilt;
    }
    CheaperError::new(
        ErrorCode::InternalError,
        format!("worker run failed ({context}): {err}"),
    )
    .with_cause(DynErrorCause(err))
}

/// `CheaperError::with_cause` は sized `impl Error` を要求するため
/// `Box<dyn Error + Send + Sync>`(`ToolError` 等)を包む wrapper。
/// cause chain は `source()` で元 error へ繋がる。
#[derive(Debug)]
struct DynErrorCause(Box<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for DynErrorCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DynErrorCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// scope `ready()`/`sandboxed()` の `AbortReason` throw → setup error。
fn setup_abort_error(context: &str, reason: AbortReason) -> CheaperError {
    if let Some(obj) = reason.error_object() {
        if let Some(e) = obj.downcast_ref::<CheaperError>() {
            let mut rebuilt = CheaperError::new(e.code(), e.message().to_string());
            if let Some(details) = e.details() {
                rebuilt = rebuilt.with_details(details.clone());
            }
            return rebuilt;
        }
    }
    CheaperError::new(
        ErrorCode::InternalError,
        format!("{context} failed: {}", reason.message()),
    )
}
// `WorkerRunError` が repair/session 境界から来ることはない(両者とも
// `CheaperError` のみを返す)ため、TS `withRunMeta` の
// `err instanceof WorkerRunError` 分岐に相当する経路は存在しない —
// `RunCtx::with_run_meta` は常に新規に meta を付与する。

#[cfg(test)]
mod tests {
    use super::*;

    /// F7: 内部 guard は `max(DEFAULT_MAX_TURNS, policy maxTurns)`。
    /// `model_factory` は session 構築時にしか使われないため `()` で
    /// 構造体だけ作り formula だけを検証する。
    #[test]
    fn internal_max_turns_is_at_least_policy_max_turns() {
        let factory = RigSessionFactory::new(());
        assert_eq!(factory.effective_internal_max_turns(10), 1024);
        assert_eq!(factory.effective_internal_max_turns(1024), 1024);
        assert_eq!(factory.effective_internal_max_turns(2048), 2048);
        assert_eq!(factory.effective_internal_max_turns(u64::MAX), usize::MAX);
    }
}
