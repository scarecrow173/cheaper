//! Port of `packages/tool-runtime/src/scheduler.ts` (T2-2).
//!
//! `ToolScheduler` — `OperationManager` と `ConcurrencyPolicy` を受け、
//! `run(options, declaration, execute)` で FIFO admission を行う。
//! claims 待機には capacity を与えず、実際の settle まで capacity を保持する
//! ("claims 待機に capacity を与えず、実際の settle まで両方を保持する")。
//!
//! ## TS との構造差
//!
//! - `start` callback が返す async 実行本体は `request()` の解決値として
//!   返され、caller(`run`)が同じ task で await する。TS では
//!   `Promise<T>` の thenable flatten で同じ結果になる。
//! - `readiness` promise の then 相当は runtime 上の `tokio::spawn` で
//!   駆動する。
//! - `structuredClone(declaration.resources)` は `serde_json::Value` の
//!   clone で再現する。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::schema::{resolve_concurrency_policy, ConcurrencyPolicy, MutationsMode};
use cheaper_core::resources::normalization::normalize_resource_manifest;
use cheaper_core::resources::types::{ClaimMode, Resource, ResourceClaim, ResourceScope};
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde::Serialize;

use crate::operation::{
    AbortReason, AbortSignal, CreateOperationOptions, OperationManager, OperationState,
};
use crate::resources::{
    LeaseKind, LeasePreparation, ParentLeaseSource, PrepareFn, PrepareFuture, ResourceLease,
    ResourceManager, ResourceRequest,
};

/// `OperationDeclaration` — 既知 tool は registry、process 等は
/// explicit resources 宣言 seam を使う。
pub struct OperationDeclaration {
    /// `"shared" | "exclusive"`。
    pub access: ClaimMode,
    pub pool: Option<OperationPool>,
    /// `ResourceManifestInput` — untyped JSON(TS `structuredClone` 相当に
    /// `run` 内で clone される)。`normalize_resource_manifest` に渡す。
    pub resources: Option<serde_json::Value>,
    pub resource_scoped: bool,
    pub parent_lease: Option<ParentLeaseSource>,
    pub prepare: Option<PrepareFn>,
    /// `readiness` — resolve で ready、reject で queued cancel。
    pub readiness: Option<ReadinessFuture>,
}

pub type ReadinessFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AbortReason>> + Send>>;

impl OperationDeclaration {
    pub fn new(access: ClaimMode) -> Self {
        Self {
            access,
            pool: None,
            resources: None,
            resource_scoped: false,
            parent_lease: None,
            prepare: None,
            readiness: None,
        }
    }
}

impl Default for OperationDeclaration {
    /// `access: "shared"` の既定(TS では必須 field)。
    fn default() -> Self {
        Self::new(ClaimMode::Shared)
    }
}

impl std::fmt::Debug for OperationDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperationDeclaration")
            .field("access", &self.access)
            .field("pool", &self.pool)
            .field("resources", &self.resources)
            .field("resource_scoped", &self.resource_scoped)
            .field("parent_lease", &self.parent_lease)
            .field("prepare", &self.prepare.as_ref().map(|_| ".."))
            .field("readiness", &self.readiness.as_ref().map(|_| ".."))
            .finish()
    }
}

/// `pool?: { name: string; maxConcurrent: number }`。
/// `max_concurrent` は `Number.isSafeInteger && >= 1` の検証を受けるため
/// `i64` で保持する。
#[derive(Debug, Clone)]
pub struct OperationPool {
    pub name: String,
    pub max_concurrent: i64,
}

struct Pool {
    cap: u64,
    running: u64,
}

#[derive(Default)]
struct SchedulerState {
    running: u64,
    exclusive: bool,
    pools: HashMap<String, Pool>,
}

/// claims 待機に capacity を与えず、実際の settle まで両方を保持する。
/// `Clone` は同一内部 state を共有する。
pub struct ToolScheduler {
    manager: OperationManager,
    policy: ConcurrencyPolicy,
    state: Arc<Mutex<SchedulerState>>,
    /// `resources` は TS 同様 public readonly。
    pub resources: ResourceManager,
}

impl Clone for ToolScheduler {
    fn clone(&self) -> Self {
        Self {
            manager: self.manager.clone(),
            policy: self.policy,
            state: Arc::clone(&self.state),
            resources: self.resources.clone(),
        }
    }
}

impl ToolScheduler {
    pub fn new(
        manager: OperationManager,
        concurrency: Option<&serde_json::Value>,
        resources: Option<ResourceManager>,
    ) -> Result<Self, CheaperError> {
        let policy = resolve_concurrency_policy(concurrency)?;
        Ok(Self {
            manager,
            policy,
            state: Arc::new(Mutex::new(SchedulerState::default())),
            resources: resources
                .unwrap_or_else(|| ResourceManager::new("/workspace", SymlinkPolicy::Deny)),
        })
    }

    /// 内部の `OperationManager`(TS `private` — registry 等の同一 crate
    /// 利用向け accessor)。
    pub fn operations(&self) -> &OperationManager {
        &self.manager
    }

    fn lock(&self) -> MutexGuard<'_, SchedulerState> {
        self.state.lock().unwrap()
    }

    /// `run(options, declaration, execute)`。
    ///
    /// `execute` は abort-aware handler(`signal` は record の composite
    /// signal)。`E` は `Into<AbortReason>` を実装する error 型。
    pub async fn run<T, E, F, Fut>(
        &self,
        options: CreateOperationOptions,
        declaration: OperationDeclaration,
        execute: F,
    ) -> Result<T, AbortReason>
    where
        T: Serialize + Send + 'static,
        E: Into<AbortReason> + Send + 'static,
        F: FnOnce(AbortSignal) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send + 'static,
    {
        let mut pool_name: Option<String> = None;
        if let Some(pool_decl) = &declaration.pool {
            // `!name || !Number.isSafeInteger(maxConcurrent) || < 1`。
            if pool_decl.name.is_empty() || pool_decl.max_concurrent < 1 {
                return Err(AbortReason::from(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "invalid operation pool declaration",
                )));
            }
            let cap = pool_decl.max_concurrent as u64;
            let mut state = self.lock();
            match state.pools.get(&pool_decl.name) {
                Some(pool) if pool.cap != cap => {
                    return Err(AbortReason::from(CheaperError::new(
                        ErrorCode::InvalidRequest,
                        "operation pool cap mismatch",
                    )));
                }
                Some(_) => {}
                None => {
                    state
                        .pools
                        .insert(pool_decl.name.clone(), Pool { cap, running: 0 });
                }
            }
            pool_name = Some(pool_decl.name.clone());
        }

        let owner_id = options.owner_id.clone();
        let operation = self.manager.create(options).map_err(AbortReason::from)?;
        // `manager.create` は既 terminal record を返し得る(pre-aborted
        // composite)。enqueue 前に `queued` と `aborted` を同期確認する。
        if let Some(reason) = operation.signal.reason() {
            return Err(reason);
        }
        if self
            .manager
            .get(&operation.id)
            .map_err(AbortReason::from)?
            .state
            != OperationState::Queued
        {
            return Err(operation
                .signal
                .reason()
                .unwrap_or_else(AbortReason::cancelled));
        }

        let op_id = operation.id.clone();
        let op_signal = operation.signal.clone();
        // `structuredClone(declaration.resources)`。
        let snapshot = declaration.resources.clone();
        let access = if self.policy.mutations == MutationsMode::PerResource
            && (declaration.resource_scoped || snapshot.is_some())
        {
            ClaimMode::Shared
        } else {
            declaration.access
        };

        // `readiness` — resolve で ready+notify、reject で queued cancel。
        let ready = Arc::new(AtomicBool::new(declaration.readiness.is_none()));
        if let Some(readiness) = declaration.readiness {
            let ready = Arc::clone(&ready);
            let resources = self.resources.clone();
            let manager = self.manager.clone();
            let op_id = op_id.clone();
            tokio::spawn(async move {
                match readiness.await {
                    Ok(()) => {
                        ready.store(true, Ordering::SeqCst);
                        resources.notify();
                    }
                    Err(error) => {
                        let queued = manager
                            .get(&op_id)
                            .map(|r| r.state == OperationState::Queued)
                            .unwrap_or(false);
                        if queued {
                            let _ = manager.cancel(&op_id, error);
                        }
                    }
                }
            });
        }

        // `prepare` — declaration.prepare > resources manifest normalize >
        // 既定の workspace fallback claim(access mode)。
        let prepare: PrepareFn = {
            let manager = self.manager.clone();
            let resources = self.resources.clone();
            let op_id = op_id.clone();
            let access = declaration.access;
            Box::new(move || -> PrepareFuture {
                let manager = manager.clone();
                let resources = resources.clone();
                let op_id = op_id.clone();
                let declaration_prepare = declaration.prepare;
                let snapshot = snapshot.clone();
                Box::pin(async move {
                    let value: LeasePreparation = match declaration_prepare {
                        Some(prepare) => prepare().await?,
                        None => match snapshot {
                            Some(input) => {
                                let manifest = normalize_resource_manifest(
                                    &resources.root,
                                    Some(&input),
                                    resources.symlink_policy,
                                )
                                .map_err(|e| AbortReason::from_error(Box::new(e)))?;
                                LeasePreparation {
                                    claims: manifest
                                        .reads
                                        .into_iter()
                                        .chain(manifest.writes)
                                        .chain(manifest.produces)
                                        .collect(),
                                    read_strictness: crate::resources::ReadStrictness::Record,
                                }
                            }
                            None => LeasePreparation {
                                claims: vec![ResourceClaim {
                                    resource: Resource::workspace(
                                        resources.root.clone(),
                                        ResourceScope::Subtree,
                                    ),
                                    mode: access,
                                }],
                                read_strictness: crate::resources::ReadStrictness::Record,
                            },
                        },
                    };
                    // `record(prepared)` — まだ queued なら claims を record に
                    // 書き戻す。
                    let queued = manager
                        .get(&op_id)
                        .map(|r| r.state == OperationState::Queued)
                        .unwrap_or(false);
                    if queued {
                        let _ = manager.set_resource_claims(&op_id, &value.claims);
                    }
                    Ok(value)
                })
            })
        };

        let can_start = {
            let state = Arc::clone(&self.state);
            let ready = Arc::clone(&ready);
            let max_operations = self.policy.max_operations;
            let pool_name = pool_name.clone();
            move || {
                if !ready.load(Ordering::SeqCst) {
                    return false;
                }
                let state = state.lock().unwrap();
                if state.exclusive || state.running >= max_operations {
                    return false;
                }
                if access == ClaimMode::Exclusive && state.running != 0 {
                    return false;
                }
                if let Some(name) = &pool_name {
                    if let Some(pool) = state.pools.get(name) {
                        if pool.running >= pool.cap {
                            return false;
                        }
                    }
                }
                true
            }
        };

        let start = {
            let manager = self.manager.clone();
            let state = Arc::clone(&self.state);
            let resources = self.resources.clone();
            let op_id = op_id.clone();
            let op_signal = op_signal.clone();
            let pool_name = pool_name.clone();
            move |lease: ResourceLease| -> Result<_, AbortReason> {
                // `manager.start` — queued→running の transition assert が
                // arbitration と admission の間の cancel を捉える。
                manager.start(&op_id).map_err(AbortReason::from)?;
                {
                    let mut state = state.lock().unwrap();
                    state.running += 1;
                    if let Some(name) = &pool_name {
                        if let Some(pool) = state.pools.get_mut(name) {
                            pool.running += 1;
                        }
                    }
                    if access == ClaimMode::Exclusive {
                        state.exclusive = true;
                    }
                }
                // start が返す実行 future(TS の thenable resolve 相当)。
                let future = async move {
                    let outcome = match execute(op_signal).await {
                        Ok(result) => {
                            let value =
                                serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
                            match manager.complete(&op_id, value) {
                                Ok(_) => Ok(result),
                                // `complete` の throw は catch→fail→throw
                                // と同じ経路を辿る。
                                Err(error) => {
                                    let reason = AbortReason::from(error);
                                    let _ = manager.fail(&op_id, reason.clone());
                                    Err(reason)
                                }
                            }
                        }
                        Err(error) => {
                            let reason = error.into();
                            let _ = manager.fail(&op_id, reason.clone());
                            Err(reason)
                        }
                    };
                    // capacity release は実際の settle(late settle)まで
                    // 保持する。scheduler lock は resources 呼び出し前に離す
                    // (lock order: resources > scheduler)。
                    {
                        let mut state = state.lock().unwrap();
                        state.running -= 1;
                        if let Some(name) = &pool_name {
                            if let Some(pool) = state.pools.get_mut(name) {
                                pool.running -= 1;
                            }
                        }
                        if access == ClaimMode::Exclusive {
                            state.exclusive = false;
                        }
                    }
                    resources.release(lease);
                    resources.notify();
                    outcome
                };
                Ok(Box::pin(future)
                    as std::pin::Pin<
                        Box<dyn std::future::Future<Output = Result<T, AbortReason>> + Send>,
                    >)
            }
        };

        let request_result = self
            .resources
            .request(ResourceRequest {
                owner_id,
                kind: LeaseKind::Operation,
                signal: Some(op_signal),
                parent_lease: declaration.parent_lease,
                access: Some(access),
                prepare,
                can_start: Some(Box::new(can_start)),
                start,
            })
            .await;
        let future = match request_result {
            Ok(future) => future,
            Err(error) => {
                // preparation の失敗は queued→cancelled。六状態の契約を
                // 拡張しない。
                let queued = self
                    .manager
                    .get(&op_id)
                    .map(|r| r.state == OperationState::Queued)
                    .unwrap_or(false);
                if queued {
                    let _ = self.manager.cancel(&op_id, error.clone());
                }
                return Err(error);
            }
        };
        future.await
    }
}

#[cfg(test)]
mod tests {
    //! `scheduler.test.ts` 対応 — invocation facade(T2-7)未整備のため
    //! `ToolScheduler`/`OperationManager`/`ResourceManager` primitive で
    //! 同じ構成を組み立てる。`flush()` 相当は `yield_now` ループ。

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use cheaper_core::errors::ErrorCode;
    use cheaper_core::resources::types::ClaimMode;
    use cheaper_core::workspace::paths::SymlinkPolicy;
    use serde_json::json;
    use tokio::sync::{oneshot, Notify};
    use tokio::task::JoinHandle;

    use super::*;
    use crate::operation::{AbortController, OperationManager, OperationManagerOptions};

    fn manager() -> OperationManager {
        OperationManager::new(OperationManagerOptions {
            abort_grace_ms: Some(0.0),
            ..Default::default()
        })
        .unwrap()
    }

    fn scheduler_with(
        manager: &OperationManager,
        concurrency: Option<serde_json::Value>,
    ) -> ToolScheduler {
        ToolScheduler::new(
            manager.clone(),
            concurrency.as_ref(),
            Some(ResourceManager::new("/workspace", SymlinkPolicy::Deny)),
        )
        .unwrap()
    }

    fn options(owner: &str) -> CreateOperationOptions {
        CreateOperationOptions {
            owner_id: owner.to_string(),
            kind: "tool".to_string(),
            ..Default::default()
        }
    }

    /// `block(key)` 相当: start を `started` に記録し、`Notify` が鳴るまで
    /// 結果を保留する。
    struct Gates {
        started: Arc<Mutex<Vec<String>>>,
        gates: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
    }

    impl Gates {
        fn new() -> Self {
            Self {
                started: Arc::new(Mutex::new(Vec::new())),
                gates: Arc::new(Mutex::new(HashMap::new())),
            }
        }
        fn gate(&self, key: &str) -> Arc<Notify> {
            self.gates
                .lock()
                .unwrap()
                .entry(key.to_string())
                .or_insert_with(|| Arc::new(Notify::new()))
                .clone()
        }
        fn release(&self, key: &str) {
            self.gate(key).notify_waiters();
        }
        fn started(&self) -> Vec<String> {
            self.started.lock().unwrap().clone()
        }
        /// `handler` — `{owner}:{key}` を記録して gate を待つ。
        fn execute(
            &self,
            owner: &str,
            key: &str,
        ) -> impl FnOnce(
            AbortSignal,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value, AbortReason>> + Send>,
        > + Send {
            let started = Arc::clone(&self.started);
            let gate = self.gate(key);
            let label = format!("{owner}:{key}");
            move |_| {
                Box::pin(async move {
                    started.lock().unwrap().push(label);
                    gate.notified().await;
                    Ok(json!({ "content": [] }))
                })
            }
        }
    }

    async fn flush() {
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
    }

    type RunHandle = JoinHandle<Result<serde_json::Value, AbortReason>>;

    /// `run` を runtime 上に spawn し JoinHandle を返す。
    fn spawn_run(
        scheduler: &ToolScheduler,
        options: CreateOperationOptions,
        declaration: OperationDeclaration,
        execute: impl FnOnce(
                AbortSignal,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<serde_json::Value, AbortReason>> + Send,
                >,
            > + Send
            + 'static,
    ) -> RunHandle {
        let scheduler = scheduler.clone();
        tokio::spawn(async move { scheduler.run(options, declaration, execute).await })
    }

    /// "overlaps reads to the configured invocation bound across owners"。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_overlaps_shared_to_bound_and_exposes_wait() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, Some(json!({ "maxOperations": 2 })));
        let gates = Gates::new();
        let p1 = spawn_run(
            &scheduler,
            options("a"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("a", "one"),
        );
        let p2 = spawn_run(
            &scheduler,
            options("b"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("b", "two"),
        );
        let p3 = spawn_run(
            &scheduler,
            options("a"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("a", "three"),
        );
        flush().await;
        assert_eq!(gates.started(), ["a:one", "b:two"]);
        let states: Vec<_> = manager.records(None).iter().map(|o| o.state).collect();
        assert_eq!(
            states,
            [
                OperationState::Running,
                OperationState::Running,
                OperationState::Queued
            ]
        );
        assert!(manager.records(None)[2].started_at.is_none());
        gates.release("one");
        p1.await.unwrap().unwrap();
        flush().await;
        assert_eq!(gates.started(), ["a:one", "b:two", "a:three"]);
        let last = &manager.records(None)[2];
        assert_eq!(last.state, OperationState::Running);
        assert!(last.started_at.unwrap() >= last.created_at);
        gates.release("two");
        gates.release("three");
        p2.await.unwrap().unwrap();
        p3.await.unwrap().unwrap();
    }

    /// "defaults standalone runtime to eight simultaneous shared operations"。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_defaults_to_eight_simultaneous_shared() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, None);
        let gates = Gates::new();
        let mut pending = Vec::new();
        for index in 0..9 {
            let key = index.to_string();
            pending.push(spawn_run(
                &scheduler,
                options("a"),
                OperationDeclaration::new(ClaimMode::Shared),
                gates.execute("a", &key),
            ));
        }
        flush().await;
        assert_eq!(gates.started().len(), 8);
        assert_eq!(
            manager
                .records(None)
                .iter()
                .filter(|o| o.state == OperationState::Queued)
                .count(),
            1
        );
        gates.release("0");
        pending.remove(0).await.unwrap().unwrap();
        flush().await;
        assert_eq!(gates.started().len(), 9);
        for key in 1..9 {
            gates.release(&key.to_string());
        }
        for p in pending {
            p.await.unwrap().unwrap();
        }
    }

    /// "serializes mutations and prevents read/writer overlap"
    /// (serialized 相当 — exclusive access が全 running を排他する)。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_exclusive_serializes_against_shared() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, Some(json!({ "maxOperations": 4 })));
        let gates = Gates::new();
        let mut pending = Vec::new();
        for (owner, key, access) in [
            ("a", "read", ClaimMode::Shared),
            ("b", "A", ClaimMode::Exclusive),
            ("a", "B", ClaimMode::Exclusive),
            ("b", "C", ClaimMode::Exclusive),
            ("a", "tail", ClaimMode::Shared),
        ] {
            pending.push((
                key,
                spawn_run(
                    &scheduler,
                    options(owner),
                    OperationDeclaration::new(access),
                    gates.execute(owner, key),
                ),
            ));
        }
        flush().await;
        assert_eq!(gates.started(), ["a:read"]);
        let expected = ["b:A", "a:B", "b:C", "a:tail"];
        for (index, (key, _)) in pending.iter().enumerate().take(4) {
            gates.release(key);
            flush().await;
            assert_eq!(gates.started().last().unwrap(), expected[index]);
            assert_eq!(
                manager
                    .records(None)
                    .iter()
                    .filter(|o| o.state == OperationState::Running)
                    .count(),
                1
            );
        }
        gates.release("tail");
        flush().await;
        for (_, p) in pending {
            p.await.unwrap().unwrap();
        }
        assert_eq!(gates.started(), ["a:read", "b:A", "a:B", "b:C", "a:tail"]);
    }

    /// "cancels a waiting writer without blocking following shared work or
    /// losing first cause"。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_queued_cancel_preserves_first_cause_and_unblocks() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, Some(json!({ "maxOperations": 2 })));
        let gates = Gates::new();
        let _read = spawn_run(
            &scheduler,
            options("a"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("a", "read"),
        );
        let call = AbortController::new();
        let queued = spawn_run(
            &scheduler,
            CreateOperationOptions {
                signal: Some(call.signal()),
                ..options("b")
            },
            OperationDeclaration::new(ClaimMode::Exclusive),
            gates.execute("b", "writer"),
        );
        let tail = spawn_run(
            &scheduler,
            options("a"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("a", "tail"),
        );
        flush().await;
        let b_record = manager.records(Some("b")).remove(0);
        assert_eq!(b_record.state, OperationState::Queued);
        let first = AbortReason::from(CheaperError::new(ErrorCode::Aborted, "caller cancelled"));
        call.abort(first.clone());
        // owner cancel(later timeout)は first cause を上書きしない。
        manager.cancel_owner(
            "b",
            AbortReason::from(CheaperError::new(ErrorCode::Timeout, "later timeout")),
        );
        let error = queued.await.unwrap().unwrap_err();
        assert_eq!(error.code(), Some("ABORTED"));
        assert_eq!(error.message(), "caller cancelled");
        flush().await;
        assert_eq!(gates.started(), ["a:read", "a:tail"]);
        let record = &manager.records(Some("b"))[0];
        assert_eq!(record.state, OperationState::Cancelled);
        assert_eq!(
            record.cancellation.as_ref().unwrap().reason.code.as_deref(),
            Some("ABORTED")
        );
        assert!(record.started_at.is_none());
        gates.release("read");
        gates.release("tail");
        tail.await.unwrap().unwrap();
    }

    /// "defers re-entrant enqueue admission until owner cancellation has
    /// reached every queued record" — queued abort 中は deferred で admission
    /// せず、propagation 完了後の repump でのみ start する。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_reentrant_enqueue_waits_for_queued_abort_propagation() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, Some(json!({ "maxOperations": 2 })));
        let gates = Gates::new();
        let _running = spawn_run(
            &scheduler,
            options("a"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("a", "running"),
        );
        let writer = spawn_run(
            &scheduler,
            options("b"),
            OperationDeclaration::new(ClaimMode::Exclusive),
            gates.execute("b", "writer"),
        );
        let cancelled_read = spawn_run(
            &scheduler,
            options("b"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("b", "cancelled-read"),
        );
        flush().await;
        assert_eq!(gates.started(), ["a:running"]);
        let reentrant: Arc<Mutex<Option<RunHandle>>> = Arc::new(Mutex::new(None));
        {
            let record = manager.records(Some("b")).remove(0);
            let reentrant = Arc::clone(&reentrant);
            let scheduler = scheduler.clone();
            let gates_started = Arc::clone(&gates.started);
            record.signal.on_abort(move |_| {
                // abort listener 内からの再提出(deferred repump 経路を検証)。
                let scheduler = scheduler.clone();
                *reentrant.lock().unwrap() = Some(tokio::spawn(async move {
                    let started = gates_started;
                    scheduler
                        .run(
                            options("a"),
                            OperationDeclaration::new(ClaimMode::Shared),
                            move |_| {
                                Box::pin(async move {
                                    started.lock().unwrap().push("a:reentrant".to_string());
                                    Ok::<_, AbortReason>(json!({ "content": [] }))
                                })
                            },
                        )
                        .await
                }));
            });
        }
        let reason = AbortReason::from(CheaperError::new(ErrorCode::Aborted, "owner cancelled"));
        manager.cancel_owner("b", reason.clone());
        // listener は同期起動 → spawn は発行済みだが admission は deferred。
        assert!(reentrant.lock().unwrap().is_some());
        assert_eq!(gates.started(), ["a:running"]);
        flush().await;
        assert_eq!(gates.started(), ["a:running", "a:reentrant"]);
        let error = writer.await.unwrap().unwrap_err();
        assert_eq!(error.message(), "owner cancelled");
        let error = cancelled_read.await.unwrap().unwrap_err();
        assert_eq!(error.message(), "owner cancelled");
        for record in manager.records(Some("b")) {
            assert_eq!(record.state, OperationState::Cancelled);
            assert!(record.started_at.is_none());
        }
        let handle = reentrant.lock().unwrap().take().unwrap();
        handle.await.unwrap().unwrap();
    }

    /// "holds capacity after non-cooperative owner detach until actual late
    /// settlement"。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_holds_capacity_until_late_settlement() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, Some(json!({ "maxOperations": 1 })));
        let gates = Gates::new();
        let late = spawn_run(
            &scheduler,
            options("a"),
            OperationDeclaration::new(ClaimMode::Exclusive),
            gates.execute("a", "late"),
        );
        flush().await;
        assert_eq!(gates.started(), ["a:late"]);
        // owner "a" の drain(grace=0) → detach しても capacity は保持される。
        manager.drain(Some("a"), Some(0.0)).await.unwrap();
        {
            let record = &manager.records(Some("a"))[0];
            assert_eq!(record.state, OperationState::Running);
            assert!(!record.owned);
            assert_eq!(record.settlement, crate::operation::Settlement::Unsettled);
        }
        let next = spawn_run(
            &scheduler,
            options("b"),
            OperationDeclaration::new(ClaimMode::Shared),
            gates.execute("b", "next"),
        );
        flush().await;
        assert_eq!(gates.started(), ["a:late"]);
        assert_eq!(manager.records(Some("b"))[0].state, OperationState::Queued);
        gates.release("late");
        late.await.unwrap().unwrap();
        flush().await;
        // late settle で capacity が返り "next" が start。
        assert_eq!(gates.started(), ["a:late", "b:next"]);
        assert_eq!(
            manager.records(Some("a"))[0].state,
            OperationState::Cancelled
        );
        gates.release("next");
        next.await.unwrap().unwrap();
    }

    /// "preserves pre-aborted composite first cause and terminal state
    /// without enqueue/start"。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_pre_aborted_preserves_reason_and_skips_work() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, Some(json!({ "maxOperations": 1 })));
        let call = AbortController::new();
        let reason = AbortReason::from(CheaperError::new(ErrorCode::Timeout, "already expired"));
        call.abort(reason);
        let executed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let executed2 = Arc::clone(&executed);
        let error = scheduler
            .run(
                CreateOperationOptions {
                    signal: Some(call.signal()),
                    ..options("a")
                },
                OperationDeclaration::new(ClaimMode::Shared),
                move |_| {
                    let executed = Arc::clone(&executed2);
                    Box::pin(async move {
                        executed.store(true, Ordering::SeqCst);
                        Ok::<_, AbortReason>(json!({ "content": [] }))
                    })
                },
            )
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some("TIMEOUT"));
        assert_eq!(error.message(), "already expired");
        let record = &manager.records(None)[0];
        assert_eq!(record.state, OperationState::TimedOut);
        assert_eq!(
            record.cancellation.as_ref().unwrap().reason.code.as_deref(),
            Some("TIMEOUT")
        );
        assert!(record.started_at.is_none());
        assert!(!executed.load(Ordering::SeqCst));
    }

    /// "records exact queue wait and releases failed execution capacity"。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_queue_wait_and_failed_execution_releases_capacity() {
        let now = Arc::new(Mutex::new(10.0));
        let now2 = Arc::clone(&now);
        let manager = OperationManager::new(OperationManagerOptions {
            clock: Some(Arc::new(move || *now2.lock().unwrap())),
            abort_grace_ms: Some(0.0),
            ..Default::default()
        })
        .unwrap();
        let scheduler = scheduler_with(&manager, Some(json!({ "maxOperations": 1 })));
        let (fail_tx, fail_rx) = oneshot::channel::<&'static str>();
        let first = spawn_run(
            &scheduler,
            options("a"),
            OperationDeclaration::new(ClaimMode::Shared),
            move |_| {
                Box::pin(async move {
                    let message = fail_rx.await.unwrap();
                    Err::<serde_json::Value, AbortReason>(AbortReason::msg(message))
                })
            },
        );
        flush().await;
        *now.lock().unwrap() = 20.0;
        let second = spawn_run(
            &scheduler,
            options("b"),
            OperationDeclaration::new(ClaimMode::Shared),
            |_| Box::pin(async move { Ok::<_, AbortReason>(json!({ "content": [] })) }),
        );
        flush().await;
        {
            let record = &manager.records(None)[1];
            assert_eq!(record.state, OperationState::Queued);
            assert_eq!(record.created_at, 20.0);
        }
        *now.lock().unwrap() = 50.0;
        fail_tx.send("handler failed").unwrap();
        let error = first.await.unwrap().unwrap_err();
        assert_eq!(error.message(), "handler failed");
        second.await.unwrap().unwrap();
        let states: Vec<_> = manager.records(None).iter().map(|o| o.state).collect();
        assert_eq!(states, [OperationState::Failed, OperationState::Completed]);
        let record = &manager.records(None)[1];
        assert_eq!(record.created_at, 20.0);
        assert_eq!(record.started_at, Some(50.0));
        assert_eq!(record.finished_at, Some(50.0));
    }

    /// pool cap — 同一 pool の second は first 完了まで queued。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_pool_cap_bounds_same_pool_operations() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, None);
        let gates = Gates::new();
        let declaration = |access| {
            let mut d = OperationDeclaration::new(access);
            d.pool = Some(OperationPool {
                name: "pool".to_string(),
                max_concurrent: 1,
            });
            d
        };
        let first = spawn_run(
            &scheduler,
            options("a"),
            declaration(ClaimMode::Shared),
            gates.execute("a", "one"),
        );
        let second = spawn_run(
            &scheduler,
            options("b"),
            declaration(ClaimMode::Shared),
            gates.execute("b", "two"),
        );
        flush().await;
        assert_eq!(gates.started(), ["a:one"]);
        assert_eq!(manager.records(Some("b"))[0].state, OperationState::Queued);
        // cap mismatch / invalid decl の検証。
        let mut mismatched = OperationDeclaration::new(ClaimMode::Shared);
        mismatched.pool = Some(OperationPool {
            name: "pool".to_string(),
            max_concurrent: 2,
        });
        let error = scheduler
            .run(options("c"), mismatched, |_| {
                Box::pin(async move { Ok::<_, AbortReason>(json!({})) })
            })
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some("INVALID_REQUEST"));
        assert_eq!(error.message(), "operation pool cap mismatch");
        let mut invalid = OperationDeclaration::new(ClaimMode::Shared);
        invalid.pool = Some(OperationPool {
            name: "other".to_string(),
            max_concurrent: 0,
        });
        let error = scheduler
            .run(options("c"), invalid, |_| {
                Box::pin(async move { Ok::<_, AbortReason>(json!({})) })
            })
            .await
            .unwrap_err();
        assert_eq!(error.message(), "invalid operation pool declaration");
        gates.release("one");
        first.await.unwrap().unwrap();
        flush().await;
        assert_eq!(gates.started(), ["a:one", "b:two"]);
        gates.release("two");
        second.await.unwrap().unwrap();
    }

    /// readiness — resolve で ready、reject で queued cancel。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_readiness_gates_start_and_rejection_cancels() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, None);
        let gates = Gates::new();
        // resolve ケース
        let (ready_tx, ready_rx) = oneshot::channel::<()>();
        let mut declaration = OperationDeclaration::new(ClaimMode::Shared);
        declaration.readiness = Some(Box::pin(async move {
            ready_rx.await.map_err(|_| AbortReason::msg("closed"))
        }));
        let pending = spawn_run(
            &scheduler,
            options("a"),
            declaration,
            gates.execute("a", "gated"),
        );
        flush().await;
        assert_eq!(gates.started(), Vec::<String>::new());
        assert_eq!(manager.records(Some("a"))[0].state, OperationState::Queued);
        ready_tx.send(()).unwrap();
        flush().await;
        assert_eq!(gates.started(), ["a:gated"]);
        gates.release("gated");
        pending.await.unwrap().unwrap();
        // reject ケース
        let mut declaration = OperationDeclaration::new(ClaimMode::Shared);
        declaration.readiness = Some(Box::pin(async move {
            Err::<(), AbortReason>(AbortReason::msg("not ready"))
        }));
        let executed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let executed2 = Arc::clone(&executed);
        let error = scheduler
            .run(options("b"), declaration, move |_| {
                let executed = Arc::clone(&executed2);
                Box::pin(async move {
                    executed.store(true, Ordering::SeqCst);
                    Ok::<_, AbortReason>(json!({}))
                })
            })
            .await
            .unwrap_err();
        assert_eq!(error.message(), "not ready");
        assert_eq!(
            manager.records(Some("b"))[0].state,
            OperationState::Cancelled
        );
        assert!(!executed.load(Ordering::SeqCst));
    }

    /// `prepare` 失敗は queued→cancelled で reason を保持する。
    #[tokio::test(flavor = "current_thread")]
    async fn sched_prepare_failure_cancels_queued_with_reason() {
        let manager = manager();
        let scheduler = scheduler_with(&manager, None);
        let mut declaration = OperationDeclaration::new(ClaimMode::Shared);
        declaration.prepare = Some(Box::new(|| {
            Box::pin(async move { Err(AbortReason::msg("prep broke")) })
        }));
        let error = scheduler
            .run(options("a"), declaration, |_| {
                Box::pin(async move { Ok::<_, AbortReason>(json!({})) })
            })
            .await
            .unwrap_err();
        assert_eq!(error.message(), "prep broke");
        assert_eq!(
            manager.records(Some("a"))[0].state,
            OperationState::Cancelled
        );
    }
}
