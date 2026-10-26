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

pub mod abort;
pub mod checkpoint;
pub mod error;
pub mod grace;
pub mod meta;
pub mod output;
pub mod panic_boundary;
pub mod repair;

mod boundary;
mod session;

use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::classify::ModelInfo;
use cheaper_core::model::model_ref::parse_model_ref;
use cheaper_core::model::pricing::ModelCost;
use cheaper_core::model::resolve::{
    resolve_explicit_model, resolve_model, ModelPolicySlice, ResolveExplicitModelOptions,
    ResolveModelOptions, ResolvedModel,
};
use cheaper_core::policy::schema::RuntimePolicy;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::write_scope::extract_write_scope;
use cheaper_core::workspace::deny_paths::create_path_denylist;
use cheaper_rig_adapter::{build_system_prompt, build_user_prompt, RunEvent, RunSnapshot};
use cheaper_tools::invocation::{
    create_invocation_tool_runtime, InvocationToolRuntime, InvocationToolRuntimeOptions,
    WorkerScopeOptions,
};
use cheaper_tools::journal::MutationJournal;
use cheaper_tools::operation::AbortReason;
use cheaper_tools::registry::{ToolRuntime, ToolRuntimeOptions};
use cheaper_tools::resources::ResourceLease;
use serde_json::{Map, Value};
use tracing::Instrument;

use self::abort::{RunAbort, RunAbortReason};
use self::checkpoint::{save_checkpoint, CheckpointInfo, RunObservations};
use self::error::{PartialRunMeta, WorkerRunError};
use self::grace::{prompt_with_abort_grace, PromptOutcome};
use self::meta::{build_meta, merge_snapshots, MetaInputs, WorkerRunMeta};
use self::output::{extract_output, ExtractedOutput};
use self::repair::{repair_output, RepairOptions};
use crate::auth::ResolvedAuth;
use crate::catalog::CatalogModel;

pub use boundary::{
    run_worker, ExecutionBinding, RunWorkerDeps, RunWorkerOk, DEFAULT_ABORT_GRACE_MS,
};
pub use session::{
    CompletionModelFactory, RigSessionFactory, SessionFactory, SessionSpec, WorkerSession,
};
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
    /// main session の tool 観測 buffer — budget 枯渇 checkpoint
    /// (issue #111)の `observations` 元。`on_event` funnel が通す
    /// `ToolCommitted`/`ToolResult` を `call_id` で join する。
    observations: std::sync::Arc<std::sync::Mutex<RunObservations>>,
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
            // resume 由来の前回観測を seed し、連鎖 resume で新規
            // checkpoint が累積保存されるようにする(design §3d)。
            observations: std::sync::Arc::new(std::sync::Mutex::new(match &deps.resume {
                Some(r) => RunObservations::with_prior(r.observations.clone()),
                None => RunObservations::new(),
            })),
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
    ///
    /// budget 枯渇(`MaxTurns`/`MaxToolCalls`)では checkpoint を best-
    /// effort 保存し、envelope 最外層 error の `details.checkpoint`
    /// (`{path, observationsCount}`)へ載せる — `PARTIAL_MUTATION`
    /// wrap 時は wrapper 側に付ける(envelope 直列化が inner cause の
    /// details を落とすため)。`Timeout`/`Signal` や workflow job は
    /// 対象外(issue #111 design §3)。
    fn classified(&self, reason: RunAbortReason) -> WorkerRunError {
        let meta = match self.full_meta(None) {
            Ok(meta) => PartialRunMeta::from(meta),
            Err(meta_err) => return self.fail(meta_err),
        };
        let checkpoint = match reason {
            RunAbortReason::MaxTurns | RunAbortReason::MaxToolCalls => {
                self.save_budget_checkpoint(reason)
            }
            _ => None,
        };
        let abort_err = reason.cheaper_error();
        let mutated = meta.changes.as_ref().is_some_and(|c| c.totals.files > 0);
        let err = if mutated {
            let mut details = Map::from_iter([(
                "changes".to_string(),
                serde_json::to_value(&meta.changes).unwrap_or_default(),
            )]);
            if let Some(info) = &checkpoint {
                details.insert("checkpoint".to_string(), checkpoint_details(info));
            }
            CheaperError::new(
                ErrorCode::PartialMutation,
                "worker aborted after partial mutations",
            )
            .with_details(details)
            .with_cause(abort_err)
        } else {
            match checkpoint {
                Some(info) => {
                    let mut details = abort_err.details().cloned().unwrap_or_default();
                    details.insert("checkpoint".to_string(), checkpoint_details(&info));
                    abort_err.with_details(details)
                }
                None => abort_err,
            }
        };
        WorkerRunError::new(err, Some(meta))
    }

    /// budget abort の checkpoint 保存 — best-effort(失敗時は `None`
    /// で `details.checkpoint` を省略するだけ)。
    fn save_budget_checkpoint(&self, reason: RunAbortReason) -> Option<CheckpointInfo> {
        let observations = self.observations.lock().unwrap_or_else(|e| e.into_inner());
        let snapshot = self
            .session
            .as_ref()
            .map(|s| s.snapshot())
            .unwrap_or_default();
        save_checkpoint(
            self.deps.runtime.user_config_dir(),
            self.deps.worker.name.as_str(),
            self.deps.input,
            reason.as_str(),
            &snapshot,
            &observations,
        )
        .ok()
    }

    // -- inner body --------------------------------------------------------

    /// run pipeline — stage 順序・分岐は TS `runWorker` と同一
    /// (invocation → model 解決 → worker scope → session → prompt →
    /// extract/repair → meta)。各 stage は private method。
    async fn run(
        &mut self,
        run_abort: &Arc<RunAbort>,
        grace_ms: f64,
    ) -> Result<RunWorkerOk, WorkerRunError> {
        let (parent_lease, resource_manifest) = self.prepare_invocation(grace_ms)?;
        // --- ここから runMeta:{execution} が付く error path -------------
        self.resolve_model_stage()?;
        let scope = self
            .create_tool_scope(parent_lease, resource_manifest, run_abort, grace_ms)
            .await?;
        let session = self.create_session(scope)?;
        let raw_output = self.drive_prompt(&session, run_abort, grace_ms).await?;
        let result = self
            .extract_or_repair(raw_output, run_abort, grace_ms)
            .await?;
        // `cheaper.run.meta` stage span(`build_meta` 畳み込み — sync
        // 区間のため entered guard)。
        let _stage = tracing::info_span!("cheaper.run.meta").entered();
        self.finish(result)
    }

    /// stage: invocation tool runtime(borrowed or owned)。
    /// `cheaper.run.tool_runtime` stage span — sync 区間のため entered
    /// guard(async 区間は `Instrument` で包む — design §instrumentation
    /// 配置)。戻り値は worker scope 生成へ渡す
    /// `(parent_lease, resource_manifest)`。
    fn prepare_invocation(
        &mut self,
        grace_ms: f64,
    ) -> Result<(Option<ResourceLease>, Option<Value>), WorkerRunError> {
        let deps = self.deps;
        let policy = deps.policy;

        // --- invocation tool runtime(borrowed or owned) ------------------
        // `cheaper.run.tool_runtime` stage span — sync 区間のため entered
        // guard(async 区間は `Instrument` で包む — design §instrumentation
        // 配置)。
        let (parent_lease, resource_manifest) = {
            let _stage = tracing::info_span!("cheaper.run.tool_runtime").entered();
            match &deps.execution {
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
                        // `policy.exec` — resolved policy の機械的
                        // serialize(`security` と同型。unified exec
                        // doc `{grants,command,process}` は
                        // `resolve_exec_policy` の入力 shape)。
                        exec: Some(serde_json::to_value(&policy.exec).unwrap_or_default()),
                        // `policy.security` — resolved policy の機械的
                        // serialize(`exec` と同型。
                        // invocation runtime の parity inner 値になる)。
                        security: Some(serde_json::to_value(&policy.security).unwrap_or_default()),
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
            }
        };
        Ok((parent_lease, resource_manifest))
    }

    /// stage: model 解決(catalog → ModelInfo → resolve*)。
    /// `cheaper.run.model_resolve` stage span(sync 区間 — entered
    /// guard)。resolved model の `model.provider`/`model.id`/
    /// `model.tier` は分類値 field として record する(payload 不可)。
    fn resolve_model_stage(&mut self) -> Result<(), WorkerRunError> {
        let deps = self.deps;
        let worker = deps.worker;
        let policy = deps.policy;

        // --- model 解決(catalog → ModelInfo → resolve*) -----------------
        // `cheaper.run.model_resolve` stage span(sync 区間 — entered
        // guard)。resolved model の `model.provider`/`model.id`/
        // `model.tier` は分類値 field として record する(payload 不可)。
        {
            let stage = tracing::info_span!(
                "cheaper.run.model_resolve",
                model.provider = tracing::field::Empty,
                model.id = tracing::field::Empty,
                model.tier = tracing::field::Empty,
            );
            let _stage_guard = stage.enter();
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
            // `toResolvedPiModel` — resolved entry を catalog で引き直し
            // auth も解決。消失は INTERNAL_ERROR。
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
            stage.record("model.provider", resolved.model.provider.as_str());
            stage.record("model.id", resolved.model.id.as_str());
            stage.record("model.tier", resolved.tier.as_str());
            self.resolved = Some(resolved);
            self.catalog_model = Some(catalog_model);
            self.auth = auth;
        }
        Ok(())
    }

    /// stage: worker scope(ledger journal・meter・budget 接続)。
    /// `cheaper.run.tool_scope` stage span — scope 生成の sync 区間は
    /// `enter` で、`ready()` の async 区間は同じ span を `Instrument`
    /// で継続する(単一 span が stage 全体の close timing を持つ)。
    async fn create_tool_scope(
        &mut self,
        parent_lease: Option<ResourceLease>,
        resource_manifest: Option<Value>,
        run_abort: &Arc<RunAbort>,
        grace_ms: f64,
    ) -> Result<ToolRuntime, WorkerRunError> {
        let deps = self.deps;
        let worker = deps.worker;
        let policy = deps.policy;
        let invocation = self.invocation.as_ref().expect("set above").clone();

        // --- worker scope ------------------------------------------------
        // `cheaper.run.tool_scope` stage span — scope 生成の sync 区間は
        // `enter` で、`ready()` の async 区間は同じ span を `Instrument`
        // で継続する(単一 span が stage 全体の close timing を持つ)。
        let tool_scope_stage = tracing::info_span!(
            "cheaper.run.tool_scope",
            tools.degraded = tracing::field::Empty,
            grants.invocation = tracing::field::Empty,
        );
        let scope = {
            let _stage_guard = tool_scope_stage.enter();
            let write_scope = extract_write_scope(worker, deps.input)
                .map_err(|e| self.fail(e))?
                .map(|scope| scope.into_iter().collect::<Vec<String>>());
            let limit_abort = Arc::clone(run_abort);
            invocation
                .create_worker_scope(WorkerScopeOptions {
                    owner_id: self.owner_id.clone(),
                    runtime: ToolRuntimeOptions {
                        resource_manifest,
                        write_scope,
                        read_strictness: None,
                        artifacts: None,
                        // `policy.exec` — unified exec doc の機械的
                        // serialize(parity check 入力 — inner.exec と
                        // の一致を `resolve_exec_policy` 再 resolve +
                        // `PartialEq` で検査)。
                        exec: Some(serde_json::to_value(&policy.exec).unwrap_or_default()),
                        // `policy.security` — `exec` と同型の parity
                        // check 入力(inner.security との一致を検査)。
                        security: Some(serde_json::to_value(&policy.security).unwrap_or_default()),
                        root: deps.root.clone(),
                        // F5: invocation runtime と同じく policy 由来。
                        symlink_policy: policy.filesystem.symlinks,
                        names: self.effective_tools.clone(),
                        // worker frontmatter `cheaper.optionalTools` の
                        // opt-in 縮退許可集合(validate 済み — gated tool の
                        // `tools` 真部分集合のみが届く)。
                        degradable: worker.cheaper.optional_tools.clone(),
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
                        route_context: None,
                        execution_correlation: deps.execution_correlation.clone(),
                    },
                })
                .map_err(|e| self.fail(tool_boundary_error("worker tool scope", e)))?
        };
        // `tools.degraded` — opt-in 縮退した tool 名の宣言順 join を
        // metadata-only field として record する(design §(f))。
        // `create_worker_scope` の return 直後(ここで `_stage_guard`
        // は inner block 閉鎖とともに drop 済み)から、span を move
        // する `.instrument(tool_scope_stage)` の前までの sync 区間
        // でしか record できない。
        let degraded = scope
            .degraded_names()
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        tool_scope_stage.record("tools.degraded", degraded.as_str());
        // `grants.invocation` — `--grant` 由来 grant 名の宣言順 join を
        // metadata-only field として同じ sync 区間で record する
        // (design §(d) — invocation-sourced grant を policy-sourced
        // run と区別する audit surface)。
        tool_scope_stage.record(
            "grants.invocation",
            deps.invocation_grants.join(", ").as_str(),
        );
        scope
            .ready()
            .instrument(tool_scope_stage)
            .await
            .map_err(|reason| self.fail(setup_abort_error("tool scope ready", reason)))?;
        self.tool_scope = Some(scope.clone());
        Ok(scope)
    }

    /// stage: main session 生成(`cheaper.run.session` stage span —
    /// sync 区間のため entered guard)。
    fn create_session(
        &mut self,
        scope: ToolRuntime,
    ) -> Result<Arc<dyn WorkerSession>, WorkerRunError> {
        let deps = self.deps;
        let worker = deps.worker;
        let policy = deps.policy;

        // --- main session ------------------------------------------------
        // `cheaper.run.session` stage span(sync 区間 — entered guard)。
        let session = {
            let _stage = tracing::info_span!("cheaper.run.session").entered();
            // "Available tools" は実登録集合から組み立てる — 縮退した
            // tool は model の tool 一覧からも消える(design §(f))。
            let tool_names: Vec<String> = scope
                .definitions()
                .iter()
                .map(|d| d.name.as_str().to_string())
                .collect();
            let unavailable: Vec<String> = scope
                .degraded_names()
                .iter()
                .map(|n| n.as_str().to_string())
                .collect();
            let system_prompt = build_system_prompt(
                &worker.instructions,
                &Value::Object(worker.output_schema.schema.clone()),
                &tool_names,
                &unavailable,
            );
            deps.session_factory
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
                .map_err(|e| self.fail(e))?
        };
        self.session = Some(Arc::clone(&session));
        Ok(session)
    }

    /// stage: prompt(abort grace 付き)+ maxTurns 監視。
    /// `cheaper.run.prompt` stage span — stream 駆動 + maxTurns 監視を
    /// 含む `prompt_with_abort_grace` の async 区間を包む
    /// (on_event の boundary event はこの span context 下で emit)。
    async fn drive_prompt(
        &self,
        session: &Arc<dyn WorkerSession>,
        run_abort: &Arc<RunAbort>,
        grace_ms: f64,
    ) -> Result<Option<String>, WorkerRunError> {
        let deps = self.deps;
        let policy = deps.policy;

        // --- prompt(abort grace 付き) ------------------------------------
        if let Some(reason) = run_abort.reason() {
            return Err(self.classified(reason));
        }
        let mut user_prompt = build_user_prompt(deps.input);
        // `--resume` 継続注入(design §3c)— 前回 abort の counter と
        // 観測を user prompt 末尾へ付け、「続きから最終 JSON」を指示する。
        // session/policy/budget は新規で、観測のみ引き継ぐ。
        if let Some(resume) = deps.resume {
            user_prompt.push_str(&format!(
                "\n\nThe previous run was interrupted by {} after {} turns and {} tool calls. \
                 Observations collected so far (JSON):\n{}\n\
                 Continue the task from this state and produce the final JSON answer.",
                resume.abort_reason,
                resume.turns,
                resume.tool_calls,
                serde_json::to_string_pretty(&resume.observations)
                    .unwrap_or_else(|_| "[]".to_string()),
            ));
        }
        let max_turns = policy.execution.max_turns;
        let outcome = {
            let check_session = Arc::clone(session);
            let check_abort = Arc::clone(run_abort);
            // F1: over-limit turn が tool call で終わった場合、その pending
            // batch の完了までは abort を遅延する。Rig の `drive_tool_calls`
            // は batch 全体の outcome を buffer し、中途で stream を drop
            // すると emit 済み `ToolCall` に対応する `ToolResult` が流れず
            // tool 実行・journal・usage 記録が欠落する。emit された
            // `ToolCall` 件数と `ToolResult` 件数の差が 0 に戻った時点で
            // abort すれば最終 batch は完全に surface される。
            let mut pending_tool_calls = 0u64;
            // stall timeout(issue #111)— 観測された RunEvent 全般を
            // 進捗として記録し、boundary の timeout watcher が
            // `last_progress + timeout` まで延長する。
            let progress = run_abort.progress_handle();
            // budget 枯渇 checkpoint の観測 buffer — main session の
            // `ToolCommitted`/`ToolResult` のみが対象(repair は
            // tool-less)。
            let observations = Arc::clone(&self.observations);
            let mut on_event = move |event: &RunEvent| {
                progress.note();
                observations
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .note(event);
                match event {
                    // RunEvent boundary event(design §instrumentation
                    // 配置)— field は `call.id`/`tool.name`/usage counts
                    // の metadata のみ(arguments/output 本文・prompt は
                    // 載せない)。
                    RunEvent::ToolCall { call_id, name, .. } => {
                        pending_tool_calls += 1;
                        tracing::debug!(call.id = %call_id, tool.name = %name, "tool call");
                    }
                    RunEvent::ToolResult { call_id, name, .. } => {
                        pending_tool_calls = pending_tool_calls.saturating_sub(1);
                        tracing::debug!(call.id = %call_id, tool.name = %name, "tool result");
                    }
                    // rollback された turn(invalid-tool retry)では emit
                    // 済み `ToolCall` の result が来ない — 新しい
                    // completion call / retry 境界で pending を reset
                    // して滞留しないようにする。
                    RunEvent::CompletionCall { index, usage } => {
                        pending_tool_calls = 0;
                        tracing::debug!(
                            call.index = index,
                            usage.input_tokens = usage.input_tokens,
                            usage.output_tokens = usage.output_tokens,
                            usage.total_tokens = usage.total_tokens,
                            "completion call"
                        );
                    }
                    RunEvent::ModelTurnRetried { .. } => pending_tool_calls = 0,
                    _ => {}
                }
                // TS: turn_end 毎に `snapshot.turns >= maxTurns &&
                // lastAssistant.stopReason === "toolUse"`。hook が item
                // 送出前に turn を計上するので per-item 読みで同等以上。
                // `maxTurns: null`(`None`)は policy 上限なし — 内部
                // runaway guard のみが残り、ここでは abort しない。
                let snap = check_session.snapshot();
                if max_turns.is_some_and(|m| snap.turns >= m)
                    && snap.last_turn_tool_calls
                    && pending_tool_calls == 0
                {
                    check_abort.request_abort(RunAbortReason::MaxTurns);
                }
            };
            // `cheaper.run.prompt` stage span — stream 駆動 + maxTurns
            // 監視を含む `prompt_with_abort_grace` の async 区間を包む
            // (on_event の boundary event はこの span context 下で emit)。
            prompt_with_abort_grace(
                session,
                user_prompt,
                &run_abort.signal(),
                grace_ms,
                &mut on_event,
            )
            .instrument(tracing::info_span!("cheaper.run.prompt"))
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
            if max_turns.is_some_and(|m| snap.turns >= m) && snap.last_turn_tool_calls {
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
        Ok(raw_output)
    }

    /// stage: extract → repair。
    /// `cheaper.run.extract`/`cheaper.run.repair` stage span。
    async fn extract_or_repair(
        &mut self,
        raw_output: Option<String>,
        run_abort: &Arc<RunAbort>,
        grace_ms: f64,
    ) -> Result<Map<String, Value>, WorkerRunError> {
        let deps = self.deps;
        let worker = deps.worker;
        let policy = deps.policy;

        // --- extract → repair ---------------------------------------------
        // `cheaper.run.extract` stage span(sync 区間 — entered guard)。
        let extracted = {
            let _stage = tracing::info_span!("cheaper.run.extract").entered();
            extract_output(raw_output.as_deref(), &worker.output_schema)
        };
        let result = match extracted {
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
                        progress: run_abort.progress_handle(),
                        signal: run_abort.signal(),
                        grace_ms,
                        user_config_dir: deps.runtime.user_config_dir(),
                        on_attempt: &mut |attempt| repairs = attempt,
                    };
                    // `cheaper.run.repair` stage span(async 区間 —
                    // `Instrument` で包む)。
                    repair_output(&mut opts, &mut self.repair_snap)
                        .instrument(tracing::info_span!("cheaper.run.repair"))
                        .await
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
        Ok(result)
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

/// `details.checkpoint` の値 — `{path, observationsCount}`。
/// 観測配列自体は checkpoint file 側にのみ置き、envelope には
/// 載せない(issue #111 design §3)。
fn checkpoint_details(info: &CheckpointInfo) -> Value {
    serde_json::json!({
        "path": info.path.to_string_lossy(),
        "observationsCount": info.observations_count,
    })
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
/// 優先順序は `error_object()` downcast → `code()` 復元 →
/// `INTERNAL_ERROR` fallback(issue #94 — `abort_payload` が運ぶ
/// describeError code を setup boundary でも復元する)。message は
/// fallback と同じ `"{context} failed: {message}"` 形を維持する。
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
    // `abort_payload` 由来の code(TIMEOUT/ABORTED/...)を復元。語彙外
    // code(Node errno 等)・code 無しは `from_str` が `None` で残り
    // fallback へ流れる(adapter `abort_reason_to_cheaper` と同じ規則)。
    if let Some(code) = reason.code().and_then(ErrorCode::from_str) {
        return CheaperError::new(code, format!("{context} failed: {}", reason.message()));
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

    /// setup boundary(`scope.ready()` reject)の分類 pin(issue #94):
    /// `RunAbortReason` 全種について実際の `abort_payload` 形 reason
    /// (`RunAbort::request_abort` → `signal().reason()`)が `code()`
    /// 復元で正しい `ErrorCode` へ写る。message は fallback と同じ
    /// `"{context} failed: {reason.message()}"` 形。
    #[test]
    fn setup_abort_error_restores_abort_payload_codes() {
        for (run_reason, expected) in [
            (RunAbortReason::Timeout, ErrorCode::Timeout),
            (RunAbortReason::Signal, ErrorCode::Aborted),
            (RunAbortReason::MaxTurns, ErrorCode::MaxTurnsExceeded),
            (
                RunAbortReason::MaxToolCalls,
                ErrorCode::MaxToolCallsExceeded,
            ),
        ] {
            let abort = RunAbort::new();
            abort.request_abort(run_reason);
            let payload = abort.signal().reason().expect("signal must be aborted");
            let err = setup_abort_error("tool scope ready", payload);
            assert_eq!(err.code(), expected, "reason {run_reason:?}");
            assert_eq!(
                err.message(),
                format!(
                    "tool scope ready failed: worker run aborted: {}",
                    run_reason.as_str()
                ),
                "reason {run_reason:?}"
            );
            // `abort_payload` には details が無く復元しない。
            assert!(err.details().is_none(), "reason {run_reason:?}");
        }
    }

    /// `error_object` 保持 reason は `code()` 復元より downcast rebuild
    /// が優先 — `CheaperError` の code/message/details をそのまま
    /// 返す(既存経路不変の pin)。
    #[test]
    fn setup_abort_error_prefers_error_object_rebuild() {
        let original = CheaperError::new(ErrorCode::PolicyViolation, "write denied").with_details(
            Map::from_iter([("path".to_string(), Value::from("/restricted"))]),
        );
        let reason = AbortReason::from_error(Box::new(original));
        let err = setup_abort_error("tool scope ready", reason);
        assert_eq!(err.code(), ErrorCode::PolicyViolation);
        assert_eq!(err.message(), "write denied");
        assert_eq!(err.details().unwrap()["path"], Value::from("/restricted"));
    }

    /// `INTERNAL_ERROR` residual pin — 語彙外 code・code 無し・
    /// `error_object` が非 `CheaperError` の reason は現行挙動
    /// (`INTERNAL_ERROR` + `"{context} failed: {message}"`)を維持。
    #[test]
    fn setup_abort_error_internal_error_residuals() {
        // describeError 語彙外の code(Node errno 等)— restore は None。
        let reason = AbortReason::new("Error", "io failed", Some("ENOENT".to_string()));
        let err = setup_abort_error("tool scope ready", reason);
        assert_eq!(err.code(), ErrorCode::InternalError);
        assert_eq!(err.message(), "tool scope ready failed: io failed");

        // code 無し payload(`new Error(...)` / 文字列 reason 相当)。
        let err = setup_abort_error("tool scope ready", AbortReason::msg("plain"));
        assert_eq!(err.code(), ErrorCode::InternalError);
        assert_eq!(err.message(), "tool scope ready failed: plain");

        // `error_object` は存在するが `CheaperError` ではない residual —
        // `from_error` が `code() == Some("ENOENT")`(Node errno)を
        // 返すため「非 ErrorCode code」と「error object 非
        // CheaperError」の両 AC をこの 1 ケースで cover する。
        let node_err = cheaper_core::errors::node_io_error_with_code(
            "open",
            "/x",
            "ENOENT",
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        );
        let reason = AbortReason::from_error(Box::new(node_err));
        assert_eq!(reason.code(), Some("ENOENT"));
        let expected = format!("tool scope ready failed: {}", reason.message());
        let err = setup_abort_error("tool scope ready", reason);
        assert_eq!(err.code(), ErrorCode::InternalError);
        assert_eq!(err.message(), expected);
    }
}
