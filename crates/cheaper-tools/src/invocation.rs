//! Port of `packages/tool-runtime/src/invocation.ts`(T2-7)。
//!
//! invocation 単位の shared runtime。`OperationManager`/`ToolScheduler`/
//! `ResourceManager` を全 owner で共有し、`createWorkerScope` が per-owner
//! の `ToolRuntime` を `binding` 経由で組み立てる。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::exec::{resolve_exec_policy, ResolvedExecPolicy};
use cheaper_core::policy::security::{resolve_security_policy, ExecDenyList, SecurityPolicy};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::Value;
use tokio::sync::OnceCell;

use crate::access::{ResourceAccess, ResourceScopeOptions, ScopeAccessOptions};
use crate::artifact::ArtifactPorts;
use crate::catalog::ToolCatalog;
use crate::execution_metrics::ExecutionMetrics;
use crate::operation::{
    AbortController, AbortListenerId, AbortReason, AbortSignal, Clock, OperationManager,
    OperationManagerOptions, OperationRecord,
};
use crate::registry::{create_tool_runtime, OperationBinding, ToolRuntime, ToolRuntimeOptions};
use crate::resources::{LeaseKind, ResourceLease, ResourceManager, ResourceRequest};
use crate::scheduler::ToolScheduler;
use crate::ToolError;

/// `JobLeaseOptions extends ResourceScopeOptions`。
pub struct JobLeaseOptions {
    pub owner_id: String,
    pub names: Vec<ToolName>,
    pub signal: Option<AbortSignal>,
    // `ResourceScopeOptions` 部分。
    pub resource_manifest: Option<Value>,
    pub write_scope: Option<Vec<String>>,
    pub read_strictness: Option<crate::resources::ReadStrictness>,
}

/// `WorkerExecutionBinding`。
pub struct WorkerExecutionBinding {
    pub runtime: ToolRuntime,
    pub owner_id: String,
    pub parent_lease: Option<ResourceLease>,
    /// parent safety と literal writeScope の両方を満たす canonical child
    /// manifest。
    pub resource_manifest: Option<Value>,
}

/// `WorkerScopeOptions extends ToolRuntimeOptions`。invocation 側で
/// `artifacts`/`process`/`signal`(composite)/`abortGraceMs`(fallback)を
/// 上書きするため、`ToolRuntimeOptions` 全体をそのまま受ける。
pub struct WorkerScopeOptions {
    pub owner_id: String,
    pub runtime: ToolRuntimeOptions,
}

/// `InvocationToolRuntimeOptions`。
pub struct InvocationToolRuntimeOptions {
    pub artifacts: Option<ArtifactPorts>,
    /// unified exec-doc の raw JSON(`resolve_exec_policy` 入力 —
    /// issue #165)。pin は行わない(`names` を持たないため) —
    /// declared-shape の `resolve_exec_policy` のみ実行し
    /// `inner.exec` に保持する(design §(b)/(g)/(k))。
    pub exec: Option<Value>,
    /// `Partial<SecurityPolicy>` の raw JSON(`resolve_security_policy`
    /// 入力 — `command` と同型の raw JSON 規約、issue #164)。
    pub security: Option<Value>,
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    pub signal: Option<AbortSignal>,
    pub abort_grace_ms: Option<f64>,
    /// `Partial<ConcurrencyPolicy>` の raw JSON。
    pub concurrency: Option<Value>,
    /// operation timestamp と snapshot に共通の monotonic clock。
    pub clock: Option<Clock>,
}

fn invalid_binding(reason: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRequest,
        format!("invalid execution binding: {reason}"),
    )
    .with_details(serde_json::Map::from_iter([(
        "reason".to_string(),
        Value::String(reason.to_string()),
    )]))
}

struct InvocationInner {
    controller: AbortController,
    manager: OperationManager,
    resources: ResourceManager,
    scheduler: ToolScheduler,
    artifacts: Option<ArtifactPorts>,
    /// declared-shape の resolved unified exec policy — scope parity
    /// check の比較対象(pin 済み path は含めない、design §(b)/(g)/(k))。
    exec: ResolvedExecPolicy,
    /// declared-shape の security policy — `inner.exec` と同型の
    /// parity check 比較対象(issue #164)。
    security: SecurityPolicy,
    /// `security.execDenyPatterns` の compiled 形 — scope 構築・
    /// parity 再 resolve・再注入で共有する。
    exec_deny: ExecDenyList,
    root: String,
    symlink_policy: SymlinkPolicy,
    abort_grace_ms: Option<f64>,
    external_signal: Option<AbortSignal>,
    external_listener: Option<AbortListenerId>,
    /// `scopes` — `Set<ToolRuntime>`。key は runtime inner の identity
    /// (worker scope dispose hook が自分を remove する)。
    scopes: Mutex<HashMap<usize, ToolRuntime>>,
    /// `owners` — `Set<string>`。一度登録した ownerId は scope dispose 後も
    /// 再利用不可(TS で remove されないまま残るのと同等)。
    owners: Mutex<HashSet<String>>,
    /// `disposed` promise memoization。
    disposed: OnceCell<()>,
}

/// `InvocationToolRuntime`。
#[derive(Clone)]
pub struct InvocationToolRuntime {
    inner: Arc<InvocationInner>,
}

impl std::fmt::Debug for InvocationToolRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvocationToolRuntime")
            .field("root", &self.inner.root)
            .finish_non_exhaustive()
    }
}

impl InvocationToolRuntime {
    /// `root`(TS getter — `opts.root`)。
    pub fn root(&self) -> &str {
        &self.inner.root
    }

    /// `symlinkPolicy`(TS getter)。
    pub fn symlink_policy(&self) -> SymlinkPolicy {
        self.inner.symlink_policy
    }

    /// `acquireJobLease(options)`。empty owner は `invalidBinding` throw →
    /// `AbortReason` に包んで返す(TS では同期 throw だが async boundary
    /// で caller は同じ error を受ける)。
    pub async fn acquire_job_lease(
        &self,
        options: JobLeaseOptions,
    ) -> Result<ResourceLease, AbortReason> {
        if options.owner_id.is_empty() {
            return Err(AbortReason::from(invalid_binding("empty lease owner")));
        }
        let access = ResourceAccess::new(
            ScopeAccessOptions {
                scope: ResourceScopeOptions {
                    resource_manifest: options.resource_manifest,
                    write_scope: options.write_scope,
                    read_strictness: options.read_strictness,
                },
                root: self.inner.resources.root.clone(),
                symlink_policy: self.inner.symlink_policy,
                names: options.names,
                denylist: None,
                catalog: ToolCatalog::from_resolved_policy(self.inner.exec.clone()),
            },
            None,
        )
        .map_err(AbortReason::from)?;
        let signal = match options.signal {
            Some(call) => AbortSignal::any(&[self.inner.controller.signal(), call]),
            None => self.inner.controller.signal(),
        };
        let access = Arc::new(access);
        self.inner
            .resources
            .request(ResourceRequest {
                owner_id: options.owner_id,
                kind: LeaseKind::Job,
                signal: Some(signal),
                parent_lease: None,
                access: None,
                prepare: Box::new(move || {
                    let prepared = access
                        .prepare()
                        .map(|scope| scope.lease_preparation())
                        .map_err(AbortReason::from);
                    Box::pin(async move { prepared })
                }),
                can_start: None,
                start: |lease| Ok(lease),
            })
            .await
    }

    /// `releaseJobLease(lease)`。
    pub fn release_job_lease(&self, lease: ResourceLease) {
        self.inner.resources.release(lease);
    }

    /// `createWorkerScope(options)`。binding 検証 → shared manager/
    /// scheduler/resources の `OperationBinding` で runtime を組み立て、
    /// `owners`/`scopes` に登録する。
    pub fn create_worker_scope(
        &self,
        options: WorkerScopeOptions,
    ) -> Result<ToolRuntime, ToolError> {
        let inner = &self.inner;
        if inner.controller.signal().is_aborted() {
            // TS は `new Error("Invocation aborted")` — CheaperError では
            // ない plain Error に揃える。
            return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                "Invocation aborted",
            ));
        }
        if options.runtime.root != inner.root {
            return Err(Box::new(invalid_binding("root mismatch")));
        }
        if options.runtime.symlink_policy != inner.symlink_policy {
            return Err(Box::new(invalid_binding("symlink policy mismatch")));
        }
        // unified exec の declared-shape parity check —
        // `options.runtime.exec` を再 resolve して `inner.exec` と
        // `PartialEq` 比較する。比較対象は pin 前の declared-shape で、
        // pin map は各 scope の `create_tool_runtime` が構築する
        // (design §(b)/(g)/(k))。scope 生成後により広い grant set を
        // 密輸できないようにする。caller 供給 exec の declared
        // executable が inner deny に match した場合の error は
        // `invalid_binding` ではなく `InvalidRuntimePolicy` で返る
        // (`security` parity と同じ inner-code 規約)。
        if let Some(raw) = &options.runtime.exec {
            let resolved = resolve_exec_policy(Some(raw), &inner.exec_deny)
                .map_err(|e| Box::new(e) as ToolError)?;
            if resolved != inner.exec {
                return Err(Box::new(invalid_binding("exec policy mismatch")));
            }
        }
        // `security` も `command`/`process` と同型の declared-shape
        // parity check(issue #164)。caller 供給 command の declared
        // executable が inner deny に match した場合の error は
        // `invalid_binding` ではなく `InvalidRuntimePolicy` で返る
        // (共有 validation は内側 code を維持する規約 — `load.rs`
        // の `with_invocation_layer` 先例)。
        if let Some(raw) = &options.runtime.security {
            let resolved =
                resolve_security_policy(Some(raw)).map_err(|e| Box::new(e) as ToolError)?;
            if resolved != inner.security {
                return Err(Box::new(invalid_binding("security policy mismatch")));
            }
        }
        if options.owner_id.is_empty() || inner.owners.lock().unwrap().contains(&options.owner_id) {
            // 早期 fast path — authoritative な重複判定は commit 時点の
            // `HashSet::insert` の bool(後述)が担う(TS でも `owners.add`
            // は `createToolRuntime` 成功後にだけ呼ぶ)。
            return Err(Box::new(invalid_binding("duplicate or empty owner")));
        }
        if let Some(lease) = options.runtime.parent_lease {
            inner
                .resources
                .parent(lease, &options.owner_id)
                .map_err(|e| Box::new(e) as ToolError)?;
        }
        // `{...options, artifacts: opts.artifacts, exec: opts.exec,
        // signal: composite, abortGraceMs: options.abortGraceMs ?? opts.abortGraceMs}`。
        let signal = match options.runtime.signal {
            Some(owner) => AbortSignal::any(&[inner.controller.signal(), owner]),
            None => inner.controller.signal(),
        };
        let runtime_options = ToolRuntimeOptions {
            resource_manifest: options.runtime.resource_manifest,
            write_scope: options.runtime.write_scope,
            read_strictness: options.runtime.read_strictness,
            artifacts: inner.artifacts.clone(),
            // `inner.exec` の再注入 — scope は常に invocation-resolved
            // の declared-shape unified exec policy を受け取る
            // (design §(k))。caller 指定の raw は上の parity check で
            // `inner.exec` との一致を検証済み、`None` もそのままでは
            // scope へ漏れない。
            exec: Some(
                serde_json::to_value(&inner.exec)
                    .expect("ResolvedExecPolicy は必ず serialize 可能"),
            ),
            // `inner.command` と同型の再注入 — scope は常に
            // invocation-resolved の security を受け取る(caller の
            // `None`/任意 raw に関わらず)。
            security: Some(
                serde_json::to_value(&inner.security)
                    .expect("SecurityPolicy は必ず serialize 可能"),
            ),
            root: options.runtime.root,
            symlink_policy: options.runtime.symlink_policy,
            names: options.runtime.names,
            degradable: options.runtime.degradable,
            max_tool_calls: options.runtime.max_tool_calls,
            on_limit: options.runtime.on_limit,
            journal: options.runtime.journal,
            denylist: options.runtime.denylist,
            parent_lease: options.runtime.parent_lease,
            signal: Some(signal),
            abort_grace_ms: options.runtime.abort_grace_ms.or(inner.abort_grace_ms),
            concurrency: options.runtime.concurrency,
            eval_spawn: options.runtime.eval_spawn,
            route_context: options.runtime.route_context,
            execution_correlation: options.runtime.execution_correlation,
        };
        let scope = create_tool_runtime(
            runtime_options,
            Some(OperationBinding {
                manager: inner.manager.clone(),
                scheduler: inner.scheduler.clone(),
                owner_id: options.owner_id.clone(),
                resources: inner.resources.clone(),
            }),
        )?;
        // owner は construction 成功の commit 時点で 1 lock の下に
        // insert する — check-then-insert の隙間で同時生成された同じ
        // owner をここで reject する(TOCTOU 対策。TS の
        // `owners.add(ownerId)` は `createToolRuntime` 直後の atomic
        // 相当)。
        if !inner.owners.lock().unwrap().insert(options.owner_id) {
            return Err(Box::new(invalid_binding("duplicate or empty owner")));
        }
        let key = scope.inner_key();
        inner.scopes.lock().unwrap().insert(key, scope.clone());
        // `scope.dispose` wrap — runtime dispose の完了時に `scopes` から
        // 自分を外す。
        let weak = Arc::downgrade(&self.inner);
        scope.set_dispose_hook(move || {
            if let Some(inner) = weak.upgrade() {
                inner.scopes.lock().unwrap().remove(&key);
            }
        });
        Ok(scope)
    }

    /// `operations(ownerId?)`。
    pub fn operations(&self, owner_id: Option<&str>) -> Vec<OperationRecord> {
        self.inner.manager.records(owner_id)
    }

    /// `executionMetrics(ownerId?)`。
    pub fn execution_metrics(&self, owner_id: Option<&str>) -> ExecutionMetrics {
        self.inner.manager.execution_metrics(owner_id)
    }

    /// `abort(reason?)`。
    pub fn abort(&self, reason: Option<AbortReason>) {
        self.inner
            .controller
            .abort(reason.unwrap_or_else(AbortReason::aborted));
    }

    /// `drain()` — abort + 全 scope の drain。`Promise.all` 相当だが scope
    /// 間に依存はない(owner 別 drain)ため逐次待機でも observable end
    /// state は同一 — 最初の error を返す。
    pub async fn drain(&self) -> Result<(), CheaperError> {
        self.abort(None);
        let scopes: Vec<ToolRuntime> = self
            .inner
            .scopes
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        let mut first_err = None;
        for scope in &scopes {
            if let Err(e) = scope.drain().await {
                first_err.get_or_insert(e);
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// `dispose()`(memoized)。
    pub async fn dispose(&self) {
        let inner = Arc::clone(&self.inner);
        let init_inner = Arc::clone(&inner);
        let _ = inner
            .disposed
            .get_or_init(move || async move { dispose_inner(init_inner).await })
            .await;
    }
}

async fn dispose_inner(inner: Arc<InvocationInner>) {
    inner.controller.abort(AbortReason::aborted());
    let scopes: Vec<ToolRuntime> = inner.scopes.lock().unwrap().values().cloned().collect();
    for scope in &scopes {
        scope.dispose().await;
    }
    // `finally` 相当 — scope dispose の成否に関わらず実行する。
    inner.manager.dispose().await;
    if let (Some(signal), Some(id)) = (&inner.external_signal, inner.external_listener) {
        signal.remove_listener(id);
    }
    inner.scopes.lock().unwrap().clear();
    // `acquireJobLease` の caller が所有する parent は late executor
    // finally まで保持する(TS 同様 release は caller 責務)。
}

/// `createInvocationToolRuntime(options)`。
pub fn create_invocation_tool_runtime(
    options: InvocationToolRuntimeOptions,
) -> Result<InvocationToolRuntime, ToolError> {
    // `security` は declared-shape + compiled 形の両方を保持する
    // (issue #164) — parity check が source string equality を見る
    // ため `SecurityPolicy` を、scope 構築・再 resolve で共有する
    // compiled predicate を `ExecDenyList` に持つ。
    let security =
        resolve_security_policy(options.security.as_ref()).map_err(|e| Box::new(e) as ToolError)?;
    let exec_deny = ExecDenyList::compile(&security.exec_deny_patterns)
        .map_err(|e| Box::new(e) as ToolError)?;
    // unified exec は declared-shape のみ resolve — pin は `names` を
    // 持つ `create_tool_runtime` 境界の per-scope 責務(design §(b)/(k))。
    let exec = resolve_exec_policy(options.exec.as_ref(), &exec_deny)
        .map_err(|e| Box::new(e) as ToolError)?;
    if let Some(grace) = options.abort_grace_ms {
        if !(grace.is_finite() && grace >= 0.0) {
            return Err(Box::new(invalid_binding(
                "abort grace must be a finite nonnegative number",
            )));
        }
    }
    let controller = AbortController::new();
    let manager = OperationManager::new(OperationManagerOptions {
        signal: Some(controller.signal()),
        abort_grace_ms: Some(options.abort_grace_ms.unwrap_or(5000.0)),
        clock: options.clock.clone(),
    })
    .map_err(|e| Box::new(e) as ToolError)?;
    let resources = ResourceManager::new(&options.root, options.symlink_policy);
    let scheduler = ToolScheduler::new(
        manager.clone(),
        options.concurrency.as_ref(),
        Some(resources.clone()),
    )
    .map_err(|e| Box::new(e) as ToolError)?;
    // external signal → `abort(opts.signal?.reason)`。
    let external_listener = options.signal.as_ref().map(|sig| {
        let controller = controller.clone();
        sig.on_abort(move |reason| {
            controller.abort(reason.clone());
        })
    });
    if let Some(reason) = options.signal.as_ref().and_then(AbortSignal::reason) {
        controller.abort(reason);
    }
    let inner = Arc::new(InvocationInner {
        controller,
        manager,
        resources,
        scheduler,
        artifacts: options.artifacts,
        exec,
        security,
        exec_deny,
        root: options.root,
        symlink_policy: options.symlink_policy,
        abort_grace_ms: options.abort_grace_ms,
        external_signal: options.signal,
        external_listener,
        scopes: Mutex::new(HashMap::new()),
        owners: Mutex::new(HashSet::new()),
        disposed: OnceCell::new(),
    });
    Ok(InvocationToolRuntime { inner })
}

// TS `invocation.ts` は `ResourceLease` を re-export するが、Rust では
// 呼出し側が `crate::resources::ResourceLease` を直接 import できるため
// 重複 export しない。
