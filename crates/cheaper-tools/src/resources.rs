//! Port of `packages/tool-runtime/src/resources.ts` (T2-2).
//!
//! `ResourceManager` — invocation 内の job/operation lease request を一つの
//! 提出順(FIFO)queue で atomic に調停する。TS の async pump
//! (`queueMicrotask` + `pumping`/`deferred` flag)は、plan の設計判断通り
//! `Mutex` 保護の同期仲裁関数 + 再入点(prepare 完了・release・notify・
//! abort)として再現する。
//!
//! - 提出順は `request()` の同期 prefix(enqueue)で確定する。
//! - `pump` は `pumping`/`deferred` guard を持つ inner method。
//!   `deferred` は queued abort 中の admission を抑制し、abort 伝播完了後の
//!   1 回の repump(`tokio::spawn` ≒ `queueMicrotask`)で解放する
//!   (bounded-tool-scheduler 設計: "queue abort 後の pump は microtask に
//!   回し root cancel の伝播途中に他 record を start しない")。
//! - `prepare` が async な間も waiter は queue に残り、先行 waiter への
//!   fairness 判定は「未 prepared = scope 不明 → 先行扱い」を維持する。

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::code_unit_compare;
use cheaper_core::resources::predicates::{claim_contains, claims_conflict, resource_key};
use cheaper_core::resources::types::{ClaimMode, ResourceClaim};
use cheaper_core::workspace::paths::{realpath, SymlinkPolicy};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::operation::{AbortListenerId, AbortReason, AbortSignal};

/// `ReadStrictness`: `"record" | "reject"`(`access.ts` の型)。
/// T2-3 が `access.rs` を移植するまで、この module が所有する。
/// (`access.rs` は `crate::resources::ReadStrictness` を参照・再 export
/// することで表現ドリフトを防ぐ。)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadStrictness {
    Record,
    Reject,
}

/// `ResourceLease` — manager が発行した identity のみ有効。
/// 公開値に claim/owner の可変状態を載せない(TS の `Object.freeze({})`
/// token 相当)。field は private で外部から偽造できない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceLease(u64);

static NEXT_LEASE: AtomicU64 = AtomicU64::new(1);

impl ResourceLease {
    fn mint() -> Self {
        Self(NEXT_LEASE.fetch_add(1, Ordering::Relaxed))
    }
}

/// `"job" | "operation"`。job は operation capacity を消費しない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseKind {
    Job,
    Operation,
}

/// `LeasePreparation`。
#[derive(Debug, Clone)]
pub struct LeasePreparation {
    pub claims: Vec<ResourceClaim>,
    pub read_strictness: ReadStrictness,
}

struct LeaseEntry {
    claims: Vec<ResourceClaim>,
    read_strictness: ReadStrictness,
    owner_id: String,
    kind: LeaseKind,
    parent: Option<ResourceLease>,
    children: u32,
    closing: bool,
}

/// `prepare` の戻り値型(TS `LeasePreparation | Promise<LeasePreparation>`
/// 相当)。同期 prepare は `lease_ready` で包む。
pub type PrepareFuture =
    Pin<Box<dyn Future<Output = Result<LeasePreparation, AbortReason>> + Send + 'static>>;
pub type PrepareFn = Box<dyn FnOnce() -> PrepareFuture + Send>;

/// 同期 resolve する `prepare` の helper(TS で non-Promise 値を返す相当)。
pub fn prepared(prep: LeasePreparation) -> PrepareFuture {
    Box::pin(async move { Ok(prep) })
}

/// `parentLease?: ResourceLease | (() => ResourceLease | undefined)`。
/// 遅延評価 variant は調停の都度 resolve される(registry の scope 確立前
/// 提出を可能にする seam)。
pub enum ParentLeaseSource {
    Fixed(ResourceLease),
    Resolver(Box<dyn Fn() -> Option<ResourceLease> + Send + Sync>),
}

impl std::fmt::Debug for ParentLeaseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(lease) => f.debug_tuple("Fixed").field(lease).finish(),
            Self::Resolver(_) => f.write_str("Resolver(..)"),
        }
    }
}

impl ParentLeaseSource {
    fn resolve(&self) -> Option<ResourceLease> {
        match self {
            Self::Fixed(lease) => Some(*lease),
            Self::Resolver(f) => f(),
        }
    }
}

impl From<ResourceLease> for ParentLeaseSource {
    fn from(lease: ResourceLease) -> Self {
        Self::Fixed(lease)
    }
}

/// `ResourceRequest<T>`。`start` の戻り値 `T` がそのまま
/// `request()` の解決値になる(scheduler は `start` が future を返し、
/// caller がそれを await する — TS で promise が flatten されるのと同等)。
pub struct ResourceRequest<S> {
    pub owner_id: String,
    pub kind: LeaseKind,
    pub signal: Option<AbortSignal>,
    pub parent_lease: Option<ParentLeaseSource>,
    /// serialized admission の全域 gate。
    pub access: Option<ClaimMode>,
    pub prepare: PrepareFn,
    pub can_start: Option<Box<dyn Fn() -> bool + Send + Sync>>,
    pub start: S,
}

type CanStartFn = Box<dyn Fn() -> bool + Send + Sync>;
/// `waiter.resolve`/`waiter.reject` 相当。`Inner` mutex の外で呼ばれる
/// ため `&Arc<Mutex<Inner>>` を受け取り、必要な巻き戻し(start 失敗時の
/// `release` 等)は自身で再取得する。
type RespondFn = Box<dyn FnOnce(&Arc<Mutex<Inner>>, Result<ResourceLease, AbortReason>) + Send>;

/// 調停で決まった応答・評価を `Inner` lock の外へ送る deferred queue
/// の要素。TS では `start`/`canStart` は `pump` 内で同期実行されるが、
/// Rust 側で mutex 保持中に user callback を呼ぶと (a) callback からの
/// `notify`/`release`/`request` 再入で自己 deadlock、(b) panic が
/// guard を越えて mutex を poison する。そのため lock 内では
/// 決定(admission 記録 + waiter 除去)のみ行い、callback 実行は
/// `drain_pending` が lock 解放後に処理する。
enum Pending {
    /// `(waiter.respond)(outcome)` — `resolve(start(token))`/`reject(e)`。
    Respond(RespondFn, Result<ResourceLease, AbortReason>),
    /// `options.canStart()` の deferred 評価。admission の他条件を
    /// lock 内で再検証してから verdict を適用する。
    CanStart { id: u64, f: CanStartFn },
}

struct Waiter {
    id: u64,
    owner_id: String,
    kind: LeaseKind,
    signal: Option<AbortSignal>,
    parent_lease: Option<ParentLeaseSource>,
    access: Option<ClaimMode>,
    can_start: Option<CanStartFn>,
    /// `canStart` の deferred 評価が in-flight。二重 enqueue 防止。
    can_start_pending: bool,
    prepared: Option<LeasePreparation>,
    respond: RespondFn,
    abort_listener: Option<AbortListenerId>,
}

impl Waiter {
    /// `waiter.cleanup()` — abort listener の除去。
    fn cleanup(&mut self) {
        if let (Some(signal), Some(id)) = (self.signal.as_ref(), self.abort_listener.take()) {
            signal.remove_listener(id);
        }
    }
}

#[derive(Default)]
struct Inner {
    leases: HashMap<ResourceLease, LeaseEntry>,
    queue: Vec<Waiter>,
    /// pump が積んだ lock-外 callback(`drain_pending` が消化する)。
    pending: VecDeque<Pending>,
    pumping: bool,
    deferred: bool,
    next_waiter: u64,
}

impl Inner {
    fn waiter_index(&self, id: u64) -> Option<usize> {
        self.queue.iter().position(|w| w.id == id)
    }

    fn remove_waiter(&mut self, id: u64) -> Option<Waiter> {
        let index = self.waiter_index(id)?;
        let mut waiter = self.queue.remove(index);
        waiter.cleanup();
        Some(waiter)
    }

    /// `release` — closing 印を立て、children が 0 なら除去して
    /// parent chain を辿り、最後に pump する。
    fn release_locked(&mut self, lease: ResourceLease) {
        let mut current = lease;
        while let Some(entry) = self.leases.get_mut(&current) {
            entry.closing = true;
            if entry.children > 0 {
                break;
            }
            let entry = self.leases.remove(&current).unwrap();
            match entry.parent {
                Some(parent) => {
                    let recurse = match self.leases.get_mut(&parent) {
                        Some(p) => {
                            p.children -= 1;
                            p.closing
                        }
                        None => false,
                    };
                    if !recurse {
                        break;
                    }
                    current = parent;
                }
                None => break,
            }
        }
        self.pump();
    }

    /// `pump` — queue snapshot を走査し、各 waiter を順に裁く。
    /// `pumping`/`deferred` 中は no-op(TS と同じ)。
    fn pump(&mut self) {
        if self.pumping || self.deferred {
            return;
        }
        self.pumping = true;
        let snapshot: Vec<u64> = self.queue.iter().map(|w| w.id).collect();
        for id in snapshot {
            self.pump_waiter(id);
        }
        self.pumping = false;
    }

    fn pump_waiter(&mut self, id: u64) {
        let Some(index) = self.waiter_index(id) else {
            return;
        };
        let waiter = &self.queue[index];
        let Some(prepared) = waiter.prepared.clone() else {
            return;
        };
        let parent_lease = waiter.parent_lease.as_ref().and_then(|p| p.resolve());
        let owner_id = waiter.owner_id.clone();
        let kind = waiter.kind;
        let access = waiter.access;
        if let Some(signal) = &waiter.signal {
            if let Some(reason) = signal.reason() {
                let waiter = self.remove_waiter(id).unwrap();
                self.pending
                    .push_back(Pending::Respond(waiter.respond, Err(reason)));
                return;
            }
        }
        // parent borrow の検証。失敗は waiter の reject(TS `try/catch`)。
        let parent = match parent_lease {
            Some(lease) => match self.parent_locked(lease, &owner_id) {
                Ok(parent) => Some(parent),
                Err(error) => {
                    let waiter = self.remove_waiter(id).unwrap();
                    self.pending.push_back(Pending::Respond(
                        waiter.respond,
                        Err(AbortReason::from(error)),
                    ));
                    return;
                }
            },
            None => None,
        };
        if let Some(parent) = &parent {
            if !covers(&parent.claims, &prepared.claims) {
                return self.reject_waiter(id, invalid_resource("child claims are outside parent"));
            }
            if parent.read_strictness != prepared.read_strictness {
                return self.reject_waiter(id, invalid_resource("parent read strictness mismatch"));
            }
        }
        // active leases(parent token 自身は除く)との conflict を一度に走査。
        let mut foreign_job_conflict = false;
        let mut any_conflict = false;
        for (token, entry) in &self.leases {
            if Some(*token) == parent_lease {
                continue;
            }
            if conflict(&entry.claims, &prepared.claims) {
                any_conflict = true;
                if entry.kind == LeaseKind::Job {
                    foreign_job_conflict = true;
                }
            }
        }
        if parent.is_some() && foreign_job_conflict {
            // 外部 binding seam が二つ目の job token を作った状態でも
            // 子を queue で待たせず fail-fast する。
            return self.reject_waiter(
                id,
                invalid_resource("borrow conflicts with foreign job token"),
            );
        }
        if any_conflict {
            return; // active lease と conflict → 待機
        }
        // FIFO fairness: 先行 waiter がこの waiter を塞ぐか。
        let blocked = self.queue[..index].iter().any(|previous| {
            let Some(previous_prepared) = &previous.prepared else {
                // 未正規化の先行要求は scope 不明なので先行する。
                return true;
            };
            let previous_parent = previous.parent_lease.as_ref().and_then(|p| p.resolve());
            if let Some(parent) = &parent {
                if previous_parent != parent_lease
                    && conflict(&parent.claims, &previous_prepared.claims)
                {
                    return false; // blockedByParent: 同じ parent 下の sibling は塞がない
                }
            }
            let global_conflict = access.is_some()
                && previous.access.is_some()
                && (access == Some(ClaimMode::Exclusive)
                    || previous.access == Some(ClaimMode::Exclusive));
            global_conflict || conflict(&previous_prepared.claims, &prepared.claims)
        });
        if blocked {
            return;
        }
        let index = self.waiter_index(id).unwrap();
        if self.queue[index].can_start_pending {
            // `canStart` 評価が in-flight — drain 後の repump で再評価。
            return;
        }
        if let Some(f) = self.queue[index].can_start.take() {
            // `options.canStart()` は user callback なので lock の外で
            // 評価する(`drain_pending` で verdict を適用 — 真なら
            // 同じ調停サイクルの repump が admission へ進める)。
            self.queue[index].can_start_pending = true;
            self.pending.push_back(Pending::CanStart { id, f });
            return;
        }
        let token = ResourceLease::mint();
        self.leases.insert(
            token,
            LeaseEntry {
                claims: prepared.claims.clone(),
                read_strictness: prepared.read_strictness,
                owner_id,
                kind,
                parent: parent_lease,
                children: 0,
                closing: false,
            },
        );
        if let Some(parent_lease) = parent_lease {
            if let Some(parent) = self.leases.get_mut(&parent_lease) {
                parent.children += 1;
            }
        }
        let waiter = self.remove_waiter(id).unwrap();
        // `resolve(start(token))` — `start` は lock の外で呼ばれ、
        // 失敗/panic は respond 内で `release` + reject へ写像される
        // (TS `try { resolve(start(token)) } catch { release; reject }`)。
        self.pending
            .push_back(Pending::Respond(waiter.respond, Ok(token)));
    }

    fn reject_waiter(&mut self, id: u64, error: CheaperError) {
        if let Some(waiter) = self.remove_waiter(id) {
            self.pending.push_back(Pending::Respond(
                waiter.respond,
                Err(AbortReason::from(error)),
            ));
        }
    }

    /// `parent` — job lease の claim/readStrictness を借用する。
    fn parent_locked(
        &self,
        lease: ResourceLease,
        owner_id: &str,
    ) -> Result<LeasePreparation, CheaperError> {
        let entry = self.leases.get(&lease);
        match entry {
            Some(entry) if entry.kind == LeaseKind::Job && !entry.closing => {
                if entry.owner_id != owner_id {
                    Err(invalid_resource("parent owner mismatch"))
                } else {
                    Ok(LeasePreparation {
                        claims: copy_claims(&entry.claims),
                        read_strictness: entry.read_strictness,
                    })
                }
            }
            _ => Err(invalid_resource("foreign, released or closing parent")),
        }
    }
}

/// `invalidResource`。
pub fn invalid_resource(message: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRequest,
        format!("invalid resource lease: {message}"),
    )
}

fn conflict(a: &[ResourceClaim], b: &[ResourceClaim]) -> bool {
    a.iter().any(|x| b.iter().any(|y| claims_conflict(x, y)))
}

fn covers(a: &[ResourceClaim], b: &[ResourceClaim]) -> bool {
    b.iter().all(|x| a.iter().any(|y| claim_contains(y, x)))
}

/// `copyClaims` — 並べ替え/重複除去と immutable 内部 snapshot を同じ
/// 境界で行う。dedupe key は `JSON.stringify([resourceKey, mode])`、
/// sort は JS relational(UTF-16 code unit)順。
fn copy_claims(claims: &[ResourceClaim]) -> Vec<ResourceClaim> {
    let mut unique: HashMap<String, ResourceClaim> = HashMap::new();
    for claim in claims {
        let key =
            serde_json::json!([resource_key(&claim.resource), claim.mode.as_str()]).to_string();
        unique.entry(key).or_insert_with(|| claim.clone());
    }
    let mut pairs: Vec<(String, ResourceClaim)> = unique.into_iter().collect();
    pairs.sort_by(|a, b| code_unit_compare(&a.0, &b.0));
    pairs.into_iter().map(|(_, claim)| claim).collect()
}

/// poison-tolerant lock。`respond`/`can_start` callback の panic が
/// `Mutex` poisoning で後続操作を連鎖失敗させないよう、poison 時も
/// inner を取り出す(TS には mutex poison の概念が無い)。
fn lock_inner(inner: &Arc<Mutex<Inner>>) -> MutexGuard<'_, Inner> {
    inner.lock().unwrap_or_else(|e| e.into_inner())
}

/// `Inner.pending` に積まれた deferred callback を lock の外で
/// 順に実行する。callback が reentrant に `release`/`notify`/`request`
/// して pending を追加した場合も drain を続ける。
///
/// `CanStart` の verdict は `Inner` lock 内で waiter の存否を再検証して
/// から適用する — 真なら `pump()` で admission へ進め、その pump が
/// 積んだ `Respond` を**次の `CanStart` 評価より先に**再帰 drain
/// する(TS では `start` が pump 内で同期実行されるため、後続 waiter
/// の `canStart` はその副作用を観測する)。`canStart` が false /
/// throw した場合は waiter を queue に残し次回 pump で再評価する
/// (TS の pump caller への伝播と同じく、throw は drain caller へ
/// `resume_unwind` で伝える)。
fn drain_pending(inner: &Arc<Mutex<Inner>>) {
    loop {
        // `while let` の scrutinee temporary は body 実行中も生き続ける
        // (pre-2024 edition)— lock を保持したまま callback を呼ぶと
        // respond 内の `release` 再取得で自己 deadlock するため、
        // pop と callback 実行を別 statement に分ける。
        let Some(item) = lock_inner(inner).pending.pop_front() else {
            return;
        };
        match item {
            Pending::Respond(respond, outcome) => respond(inner, outcome),
            Pending::CanStart { id, f } => {
                let verdict = std::panic::catch_unwind(std::panic::AssertUnwindSafe(&f));
                {
                    let mut inner = lock_inner(inner);
                    if let Some(index) = inner.waiter_index(id) {
                        let waiter = &mut inner.queue[index];
                        waiter.can_start_pending = false;
                        match &verdict {
                            Ok(true) => inner.pump(),
                            Ok(false) | Err(_) => waiter.can_start = Some(f),
                        }
                    }
                }
                if let Err(payload) = verdict {
                    std::panic::resume_unwind(payload);
                }
                drain_pending(inner);
            }
        }
    }
}

/// `start` callback の panic payload を reject reason の error object
/// へ写す(TS `catch (error) { this.release(token); reject(error) }`
/// で thrown value がそのまま rejection になる相当)。
fn panic_payload_error(payload: Box<dyn std::any::Any + Send>) -> crate::ToolError {
    let msg = payload
        .downcast_ref::<&'static str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".to_string());
    Box::new(std::io::Error::other(format!(
        "start callback panicked: {msg}"
    )))
}

/// invocation 内の job/operation を一つの提出順で atomic に調停する。
/// `Clone` は同一内部 state を共有する。
pub struct ResourceManager {
    inner: Arc<Mutex<Inner>>,
    /// Test-only one-shot checkpoint between the successful signal pre-check
    /// and enqueue. Clones share it because they are the same manager instance.
    #[cfg(test)]
    request_phase_hook: Arc<Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>>,
    /// canonical realpath(実在しない injected root は入力のまま)。
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
}

impl Clone for ResourceManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            #[cfg(test)]
            request_phase_hook: Arc::clone(&self.request_phase_hook),
            root: self.root.clone(),
            symlink_policy: self.symlink_policy,
        }
    }
}

impl ResourceManager {
    pub fn new(root: &str, symlink_policy: SymlinkPolicy) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            #[cfg(test)]
            request_phase_hook: Arc::new(Mutex::new(None)),
            // 実在 root の identity を同期に確定。injected fake-root の
            // 従来 admission は維持する。
            root: realpath(root).unwrap_or_else(|_| root.to_string()),
            symlink_policy,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        lock_inner(&self.inner)
    }

    #[cfg(test)]
    fn set_request_phase_hook(&self, hook: impl FnOnce() + Send + 'static) {
        *self.request_phase_hook.lock().unwrap() = Some(Box::new(hook));
    }

    /// `inner` lock 内で `f` を実行し、積まれた deferred callback
    /// (`respond`/`can_start` — user callback で再入・panic し得る)を
    /// lock 解放後に drain する。
    fn transact<R>(&self, f: impl FnOnce(&mut Inner) -> R) -> R {
        let result = f(&mut self.lock());
        drain_pending(&self.inner);
        result
    }

    /// `parent` — job lease から preparation を借用する。
    pub fn parent(
        &self,
        lease: ResourceLease,
        owner_id: &str,
    ) -> Result<LeasePreparation, CheaperError> {
        self.lock().parent_locked(lease, owner_id)
    }

    /// `request` — submission 順を同期に確定し、prepare 完了・abort・
    /// capacity 変化の各再入点から単一の pump が調停する。
    pub async fn request<T, S>(&self, options: ResourceRequest<S>) -> Result<T, AbortReason>
    where
        S: FnOnce(ResourceLease) -> Result<T, AbortReason> + Send + 'static,
        T: Send + 'static,
    {
        if let Some(signal) = &options.signal {
            // `options.signal?.aborted` pre-check(reason そのまま reject)。
            if let Some(reason) = signal.reason() {
                return Err(reason);
            }
        }
        #[cfg(test)]
        if let Some(hook) = self.request_phase_hook.lock().unwrap().take() {
            hook();
        }
        let (tx, rx) = oneshot::channel::<Result<T, AbortReason>>();
        let ResourceRequest {
            owner_id,
            kind,
            signal,
            parent_lease,
            access,
            prepare,
            can_start,
            start,
        } = options;
        let waiter_id = {
            let mut inner = self.lock();
            inner.next_waiter += 1;
            let id = inner.next_waiter;
            inner.queue.push(Waiter {
                id,
                owner_id,
                kind,
                signal: signal.clone(),
                parent_lease,
                access,
                can_start,
                can_start_pending: false,
                prepared: None,
                respond: Box::new(move |inner, outcome| {
                    let result = outcome.and_then(|lease| {
                        // `start` は lock 外で呼ばれる(deferred respond)。
                        // Err return と panic の双方を `release(token)` +
                        // reject へ写す(TS `try/catch { release; reject }`
                        // 相当 — panic が mutex poisoning で他 waiter を
                        // 巻き込まない)。
                        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            start(lease)
                        })) {
                            Ok(Ok(value)) => Ok(value),
                            Ok(Err(error)) => {
                                // `start` 失敗は lease を即座に巻き戻す
                                // (TS: `catch { this.release(token); reject }`)。
                                lock_inner(inner).release_locked(lease);
                                Err(error)
                            }
                            Err(payload) => {
                                lock_inner(inner).release_locked(lease);
                                Err(AbortReason::from_error(panic_payload_error(payload)))
                            }
                        }
                    });
                    let _ = tx.send(result);
                }),
                abort_listener: None,
            });
            id
        };
        // queue abort listener — wait 中の entry を除去し original
        // composite signal.reason で reject し、pump は microtask 相当に
        // 回す(deferred)。
        if let Some(signal) = &signal {
            let inner = Arc::clone(&self.inner);
            let handle = tokio::runtime::Handle::current();
            let listener_id = signal.on_abort(move |reason| {
                let inner_arc = Arc::clone(&inner);
                {
                    let mut inner = lock_inner(&inner);
                    let Some(waiter) = inner.remove_waiter(waiter_id) else {
                        return;
                    };
                    inner
                        .pending
                        .push_back(Pending::Respond(waiter.respond, Err(reason.clone())));
                    inner.deferred = true;
                }
                drain_pending(&inner_arc);
                handle.spawn(async move {
                    {
                        let mut inner = lock_inner(&inner_arc);
                        inner.deferred = false;
                        inner.pump();
                    }
                    drain_pending(&inner_arc);
                });
            });
            let mut inner = self.lock();
            if let Some(index) = inner.waiter_index(waiter_id) {
                inner.queue[index].abort_listener = Some(listener_id);
                // `on_abort` 登録前に signal が abort 済みだった場合、
                // listener は post-drain で drop され二度と発火しない
                // (operation.rs の `drained` 分岐)。同じ inner lock の
                // 下で `signal.reason()` を再確認し、listener 相当の
                // teardown(remove → reject → deferred repump)を行う。
                // listener が既に発火済みなら waiter は queue に無く
                // `remove_waiter` は no-op(idempotent)。
                if let Some(reason) = signal.reason() {
                    if let Some(waiter) = inner.remove_waiter(waiter_id) {
                        inner
                            .pending
                            .push_back(Pending::Respond(waiter.respond, Err(reason)));
                        inner.deferred = true;
                        drop(inner);
                        drain_pending(&self.inner);
                        let inner_arc = Arc::clone(&self.inner);
                        tokio::spawn(async move {
                            {
                                let mut inner = lock_inner(&inner_arc);
                                inner.deferred = false;
                                inner.pump();
                            }
                            drain_pending(&inner_arc);
                        });
                    }
                }
            } else {
                drop(inner);
                signal.remove_listener(listener_id);
            }
        }
        let mut guard = WaiterGuard {
            manager: self,
            waiter_id,
            armed: true,
        };
        // `prepare()` — aborted 中は結果を破棄して abort で抜ける。
        let prepare = prepare();
        let prepared = match &signal {
            Some(signal) => {
                tokio::select! {
                    biased;
                    _ = signal.cancelled() => PrepareOutcome::Aborted,
                    result = prepare => PrepareOutcome::Done(result),
                }
            }
            None => PrepareOutcome::Done(prepare.await),
        };
        match prepared {
            PrepareOutcome::Aborted => {
                // listener が waiter 除去 + reject 済み(非同期 abort の
                // 場合は rx が resolve するまで待つ)。
            }
            PrepareOutcome::Done(Ok(prep)) => {
                // `prepared(value)`: queue に残っていれば正規化して pump。
                let pumped = self.transact(|inner| {
                    if let Some(index) = inner.waiter_index(waiter_id) {
                        inner.queue[index].prepared = Some(LeasePreparation {
                            claims: copy_claims(&prep.claims),
                            read_strictness: prep.read_strictness,
                        });
                        inner.pump();
                        true
                    } else {
                        false
                    }
                });
                if pumped {
                    // `queueMicrotask(() => this.pump())` 相当。
                    let inner = Arc::clone(&self.inner);
                    tokio::spawn(async move {
                        {
                            lock_inner(&inner).pump();
                        }
                        drain_pending(&inner);
                    });
                }
            }
            PrepareOutcome::Done(Err(error)) => {
                let removed = self.transact(|inner| {
                    if let Some(waiter) = inner.remove_waiter(waiter_id) {
                        inner
                            .pending
                            .push_back(Pending::Respond(waiter.respond, Err(error)));
                        inner.pump();
                        true
                    } else {
                        false
                    }
                });
                if removed {
                    let inner = Arc::clone(&self.inner);
                    tokio::spawn(async move {
                        {
                            lock_inner(&inner).pump();
                        }
                        drain_pending(&inner);
                    });
                }
            }
        }
        let result = match rx.await {
            Ok(result) => result,
            // waiter が manager drop で消失した場合のみ発生し得る。
            Err(_) => Err(AbortReason::msg("resource request dropped")),
        };
        guard.armed = false;
        result
    }

    /// `release` — capacity の解放と同じ仲裁点への通知を兼ねる。
    pub fn release(&self, lease: ResourceLease) {
        self.transact(|inner| inner.release_locked(lease));
    }

    /// `notify` — capacity の解放も同じ仲裁点へ通知する。
    pub fn notify(&self) {
        self.transact(|inner| inner.pump());
    }

    /// `releaseJobs`。
    pub fn release_jobs(&self) {
        self.transact(|inner| {
            let leases: Vec<ResourceLease> = inner
                .leases
                .iter()
                .filter(|(_, entry)| entry.kind == LeaseKind::Job)
                .map(|(token, _)| *token)
                .collect();
            for lease in leases {
                inner.release_locked(lease);
            }
        });
    }
}

enum PrepareOutcome {
    Aborted,
    Done(Result<LeasePreparation, AbortReason>),
}

/// request future が resolve 前に drop された場合、queue に waiter が
/// 残って FIFO を塞がないように除去する(TS の promise には drop 相当の
/// 概念がないため、Rust 側の追加ガード)。
struct WaiterGuard<'a> {
    manager: &'a ResourceManager,
    waiter_id: u64,
    armed: bool,
}

impl Drop for WaiterGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.manager.transact(|inner| {
                // queue 先頭の未 prepared waiter が FIFO を塞いでいる
                // 可能性があるため、除去後に pump する。
                if inner.remove_waiter(self.waiter_id).is_some() {
                    inner.pump();
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cheaper_core::resources::types::{ClaimMode, Resource, ResourceClaim, ResourceScope};
    use cheaper_core::workspace::paths::SymlinkPolicy;

    use super::*;
    use crate::operation::AbortController;

    fn claim(path: &str, scope: ResourceScope, mode: ClaimMode) -> ResourceClaim {
        ResourceClaim {
            resource: Resource::workspace(path, scope),
            mode,
        }
    }

    fn prep(paths: &[&str], mode: ClaimMode) -> LeasePreparation {
        LeasePreparation {
            claims: paths
                .iter()
                .map(|p| claim(p, ResourceScope::Exact, mode))
                .collect(),
            read_strictness: ReadStrictness::Reject,
        }
    }

    fn request_options<S>(
        owner_id: &str,
        kind: LeaseKind,
        preparation: LeasePreparation,
        start: S,
    ) -> ResourceRequest<S> {
        ResourceRequest {
            owner_id: owner_id.to_string(),
            kind,
            signal: None,
            parent_lease: None,
            access: None,
            prepare: Box::new(move || prepared(prep_for(&preparation))),
            can_start: None,
            start,
        }
    }

    fn prep_for(p: &LeasePreparation) -> LeasePreparation {
        p.clone()
    }

    fn lease_start() -> impl FnOnce(ResourceLease) -> Result<ResourceLease, AbortReason> + Send {
        |lease| Ok(lease)
    }

    /// flush: spawned task / repump task が走り切るまで yield。
    async fn flush() {
        for _ in 0..32 {
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn conflicting_exclusive_requests_are_fifo() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let first = manager
            .request(request_options(
                "a",
                LeaseKind::Operation,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            ))
            .await
            .unwrap();
        // same claim → conflict で second は待つ。
        let manager2 = manager.clone();
        let second = tokio::spawn(async move {
            manager2
                .request(request_options(
                    "b",
                    LeaseKind::Operation,
                    prep(&["/workspace/a"], ClaimMode::Exclusive),
                    lease_start(),
                ))
                .await
        });
        flush().await;
        assert!(!second.is_finished());
        manager.release(first);
        let second_lease = second.await.unwrap().unwrap();
        manager.release(second_lease);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn independent_claim_bypasses_earlier_blocked_waiter() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let held = manager
            .request(request_options(
                "a",
                LeaseKind::Operation,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            ))
            .await
            .unwrap();
        let manager2 = manager.clone();
        let blocked = tokio::spawn(async move {
            manager2
                .request(request_options(
                    "b",
                    LeaseKind::Operation,
                    prep(&["/workspace/a"], ClaimMode::Exclusive),
                    lease_start(),
                ))
                .await
        });
        let manager3 = manager.clone();
        let independent = tokio::spawn(async move {
            manager3
                .request(request_options(
                    "c",
                    LeaseKind::Operation,
                    prep(&["/workspace/c"], ClaimMode::Exclusive),
                    lease_start(),
                ))
                .await
        });
        flush().await;
        assert!(!blocked.is_finished());
        // `c` は blocked waiter と conflict しないので先行 admission。
        let c = independent.await.unwrap().unwrap();
        manager.release(c);
        manager.release(held);
        let b = blocked.await.unwrap().unwrap();
        manager.release(b);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn job_lease_parent_borrow_and_children_retain_closing_parent() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let parent = manager
            .request(request_options(
                "holder",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            ))
            .await
            .unwrap();
        // borrow(子 operation)。同じ owner + 包含 claim → 許可。
        let child = manager
            .request(ResourceRequest {
                parent_lease: Some(parent.into()),
                ..request_options(
                    "holder",
                    LeaseKind::Operation,
                    prep(&["/workspace/a"], ClaimMode::Exclusive),
                    lease_start(),
                )
            })
            .await
            .unwrap();
        // parent release しても子が生きている間は closing のみで残る。
        manager.release(parent);
        assert!(manager.parent(parent, "holder").is_err()); // closing → borrow 不可
                                                            // child release で parent chain も解放される。
        manager.release(child);
        // contender が同じ claim で入れる。
        let contender = manager
            .request(request_options(
                "contender",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            ))
            .await
            .unwrap();
        manager.release(contender);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn parent_lease_validates_owner_and_child_claims_containment() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let parent = manager
            .request(request_options(
                "holder",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            ))
            .await
            .unwrap();
        // owner mismatch → parent() が拒否。
        assert!(manager.parent(parent, "other").is_err());
        // child claims が parent の外側 → request が reject。
        let result = manager
            .request(ResourceRequest {
                parent_lease: Some(parent.into()),
                ..request_options(
                    "holder",
                    LeaseKind::Operation,
                    prep(&["/workspace/b"], ClaimMode::Exclusive),
                    lease_start(),
                )
            })
            .await;
        let error = result.unwrap_err();
        assert_eq!(error.code(), Some("INVALID_REQUEST"));
        assert!(error.message().contains("outside parent"));
        manager.release(parent);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn foreign_job_token_conflict_fails_fast_for_borrowed_operation() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let preparation = prep(&["/workspace/a"], ClaimMode::Exclusive);
        let parent = manager
            .request(request_options(
                "owner",
                LeaseKind::Job,
                preparation.clone(),
                lease_start(),
            ))
            .await
            .unwrap();
        // 外部 binding seam が二つ目の job token を作った状態。
        let other_job = manager
            .request(ResourceRequest {
                parent_lease: Some(parent.into()),
                ..request_options("owner", LeaseKind::Job, preparation.clone(), lease_start())
            })
            .await
            .unwrap();
        let error = manager
            .request(ResourceRequest {
                parent_lease: Some(parent.into()),
                ..request_options("owner", LeaseKind::Operation, preparation, |_| {
                    Err::<ResourceLease, AbortReason>(AbortReason::msg("must not start"))
                })
            })
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some("INVALID_REQUEST"));
        assert!(error.message().contains("foreign job token"));
        manager.release(other_job);
        manager.release(parent);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn reverse_multi_claim_jobs_are_all_or_none_and_restore_fifo() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let options = |owner: &str, paths: &[&str]| {
            request_options(
                owner,
                LeaseKind::Job,
                LeasePreparation {
                    claims: paths
                        .iter()
                        .map(|p| claim(p, ResourceScope::Exact, ClaimMode::Exclusive))
                        .collect(),
                    read_strictness: ReadStrictness::Reject,
                },
                lease_start(),
            )
        };
        let parent = manager
            .request(options("first", &["/workspace/a"]))
            .await
            .unwrap();
        let order = Arc::new(Mutex::new(Vec::<String>::new()));
        let manager2 = manager.clone();
        let order2 = order.clone();
        let second = tokio::spawn(async move {
            let lease = manager2
                .request(options("second", &["/workspace/b", "/workspace/a"]))
                .await
                .unwrap();
            order2.lock().unwrap().push("second".to_string());
            lease
        });
        let manager3 = manager.clone();
        let order3 = order.clone();
        let third = tokio::spawn(async move {
            let lease = manager3
                .request(options("third", &["/workspace/a", "/workspace/b"]))
                .await
                .unwrap();
            order3.lock().unwrap().push("third".to_string());
            lease
        });
        flush().await;
        // second は b を部分取得しない。独立した c は先行する。
        let independent = manager
            .request(options("other", &["/workspace/c"]))
            .await
            .unwrap();
        assert!(order.lock().unwrap().is_empty());
        manager.release(independent);
        manager.release(parent);
        let lease = second.await.unwrap();
        assert_eq!(order.lock().unwrap().as_slice(), ["second"]);
        manager.release(lease);
        let last = third.await.unwrap();
        assert_eq!(order.lock().unwrap().as_slice(), ["second", "third"]);
        manager.release(last);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn queued_abort_removes_waiter_and_preserves_reason() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let held = manager
            .request(request_options(
                "a",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            ))
            .await
            .unwrap();
        let controller = AbortController::new();
        let signal = controller.signal();
        let manager2 = manager.clone();
        let waiting = tokio::spawn(async move {
            manager2
                .request(ResourceRequest {
                    signal: Some(signal),
                    ..request_options(
                        "b",
                        LeaseKind::Job,
                        prep(&["/workspace/a"], ClaimMode::Exclusive),
                        lease_start(),
                    )
                })
                .await
        });
        flush().await;
        let reason = AbortReason::msg("stop waiting");
        controller.abort(reason.clone());
        let error = waiting.await.unwrap().unwrap_err();
        assert_eq!(error.message(), "stop waiting");
        manager.release(held);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn abort_during_prepare_resolves_with_reason() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let controller = AbortController::new();
        let signal = controller.signal();
        let (tx, rx) = oneshot::channel::<()>();
        let manager2 = manager.clone();
        let waiting = tokio::spawn(async move {
            manager2
                .request(ResourceRequest {
                    signal: Some(signal),
                    prepare: Box::new(move || {
                        Box::pin(async move {
                            let _ = rx.await;
                            Ok(prep(&["/workspace/a"], ClaimMode::Exclusive))
                        })
                    }),
                    ..request_options(
                        "b",
                        LeaseKind::Job,
                        prep(&[], ClaimMode::Shared),
                        lease_start(),
                    )
                })
                .await
        });
        flush().await;
        controller.abort(AbortReason::msg("mid-prepare"));
        let error = waiting.await.unwrap().unwrap_err();
        assert_eq!(error.message(), "mid-prepare");
        let _ = tx.send(());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn prepare_failure_rejects_and_unblocks_queue() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let held = manager
            .request(request_options(
                "a",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            ))
            .await
            .unwrap();
        let failure = AbortReason::msg("bad prep");
        let manager2 = manager.clone();
        let failed = tokio::spawn(async move {
            manager2
                .request(ResourceRequest {
                    prepare: Box::new(move || {
                        Box::pin(async move { Err::<LeasePreparation, _>(failure) })
                    }),
                    ..request_options(
                        "b",
                        LeaseKind::Job,
                        prep(&["/workspace/a"], ClaimMode::Exclusive),
                        lease_start(),
                    )
                })
                .await
        });
        // failure は FIFO を塞がない(直ちに waiter 除去)。
        let error = failed.await.unwrap().unwrap_err();
        assert_eq!(error.message(), "bad prep");
        manager.release(held);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn copy_claims_dedupes_and_sorts_by_resource_key() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let claims = vec![
            claim("/workspace/b", ResourceScope::Exact, ClaimMode::Shared),
            claim("/workspace/a", ResourceScope::Exact, ClaimMode::Shared),
            claim("/workspace/a", ResourceScope::Exact, ClaimMode::Shared),
        ];
        let lease = manager
            .request(request_options(
                "a",
                LeaseKind::Job,
                LeasePreparation {
                    claims,
                    read_strictness: ReadStrictness::Record,
                },
                lease_start(),
            ))
            .await
            .unwrap();
        let borrowed = manager.parent(lease, "a").unwrap();
        let paths: Vec<&str> = borrowed.claims.iter().map(|c| c.resource.path()).collect();
        // sort は ["workspace","<path>","exact"] + mode の順。
        let expected = vec!["/workspace/a", "/workspace/b"];
        assert_eq!(paths, expected);
        manager.release(lease);
    }

    #[test]
    fn parent_rejects_foreign_or_released_lease() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async {
            let lease = manager
                .request(request_options(
                    "holder",
                    LeaseKind::Job,
                    prep(&["/workspace/a"], ClaimMode::Exclusive),
                    lease_start(),
                ))
                .await
                .unwrap();
            let other = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
            // forged / foreign lease id は reject される
            // (test module は private field を直接構築できる)。
            let forged = ResourceLease(u64::MAX);
            assert!(manager.parent(forged, "holder").is_err());
            assert!(other.parent(lease, "holder").is_err());
            manager.release(lease);
            assert!(manager.parent(lease, "holder").is_err());
        });
    }

    /// pre-check 通過後・enqueue 前に abort する schedule gap を実際の
    /// one-shot checkpoint で固定し、real enqueue/listener/recheck 経路を
    /// 通ることを bounded に検証する。request 側の blocking lock を test
    /// executor と分離するため std::thread 上の current_thread runtime で
    /// 駆動する(rt-multi-thread feature は workspace で無効)。
    #[test]
    fn abort_landing_before_listener_registration_still_rejects() {
        const PHASE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
        const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
        const WORKER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
        const RESULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(13);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let held = rt
            .block_on(manager.request(request_options(
                "a",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            )))
            .unwrap();
        let controller = AbortController::new();
        let signal = controller.signal();
        let manager2 = manager.clone();
        let held_for_worker = held;
        let (phase_tx, phase_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        manager.set_request_phase_hook(move || {
            phase_tx
                .send(())
                .expect("test must observe the successful-pre-check phase");
            release_rx
                .recv_timeout(PHASE_TIMEOUT)
                .expect("test must release the one-shot phase hook within budget");
        });
        let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result = rt.block_on(async {
                tokio::time::timeout(
                    REQUEST_TIMEOUT,
                    manager2.request(ResourceRequest {
                        signal: Some(signal),
                        ..request_options(
                            "b",
                            LeaseKind::Job,
                            prep(&["/workspace/a"], ClaimMode::Exclusive),
                            lease_start(),
                        )
                    }),
                )
                .await
            });
            let later = match &result {
                Ok(Err(_)) => {
                    manager2.release(held_for_worker);
                    Some(rt.block_on(async {
                        tokio::time::timeout(
                            REQUEST_TIMEOUT,
                            manager2.request(request_options(
                                "c",
                                LeaseKind::Job,
                                prep(&["/workspace/b"], ClaimMode::Exclusive),
                                lease_start(),
                            )),
                        )
                        .await
                    }))
                }
                _ => None,
            };
            let _ = result_tx.send((result, later));
        });
        let phase_started = std::time::Instant::now();
        phase_rx.recv_timeout(PHASE_TIMEOUT).unwrap_or_else(|error| {
            panic!(
                "phase=post-precheck-pre-enqueue elapsed={:?} allowed_budget={PHASE_TIMEOUT:?}: {error}",
                phase_started.elapsed()
            )
        });
        controller.abort(AbortReason::msg("abort in registration gap"));
        release_tx
            .send(())
            .expect("worker must be waiting at the phase checkpoint");
        let result_started = std::time::Instant::now();
        let (result, later) = result_rx.recv_timeout(RESULT_TIMEOUT).unwrap_or_else(|error| {
            panic!(
                "phase=request-and-manager-reuse-result elapsed={:?} allowed_budget={RESULT_TIMEOUT:?}: {error}",
                result_started.elapsed()
            )
        });
        let error = match result {
            Ok(Err(error)) => error,
            Err(error) => panic!(
                "phase=abort-request elapsed>={REQUEST_TIMEOUT:?} allowed_budget={REQUEST_TIMEOUT:?}: {error}"
            ),
            Ok(Ok(_)) => panic!("phase=abort-request expected rejection but request was admitted"),
        };
        assert_eq!(error.message(), "abort in registration gap");

        let worker_started = std::time::Instant::now();
        let worker_deadline = worker_started + WORKER_TIMEOUT;
        while !worker.is_finished() && std::time::Instant::now() < worker_deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            worker.is_finished(),
            "phase=worker-completion elapsed={:?} allowed_budget={WORKER_TIMEOUT:?}",
            worker_started.elapsed()
        );
        worker.join().expect("completed worker must join cleanly");

        let later = later
            .expect("manager reuse request must be attempted after abort rejection")
            .expect("manager must remain usable after abort rejection")
            .expect("later independent request must be admitted");
        rt.block_on(async {
            manager.release(later);
        });
    }

    /// `start`/`can_start` callback は `Inner` mutex の外で実行されるため
    ///、callback からの `notify`/`release`/`request` 再入は deadlock
    /// しない(TS では pump 内同期呼出しだが JS は single thread で
    /// reentrancy-guard で済む — Rust は lock 外実行で対応)。
    #[tokio::test(flavor = "current_thread")]
    async fn start_and_can_start_callbacks_may_reenter_manager() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let manager2 = manager.clone();
        let manager3 = manager.clone();
        let lease = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            manager.request(ResourceRequest {
                can_start: Some(Box::new(move || {
                    // lock 保持中に呼ばれていればここで自己 deadlock。
                    manager2.notify();
                    true
                })),
                ..request_options(
                    "a",
                    LeaseKind::Job,
                    prep(&["/workspace/a"], ClaimMode::Exclusive),
                    move |lease| {
                        manager3.notify();
                        Ok(lease)
                    },
                )
            }),
        )
        .await
        .expect("reentrant callbacks must not deadlock")
        .unwrap();
        manager.release(lease);
    }

    /// `start` callback の panic は `catch_unwind` で捕捉し lease 巻き戻し
    /// + reject へ写す(TS `catch { release; reject }` 相当)。mutex が
    /// poison しないため後続の request が通る(以前は panic → poison →
    /// guard drop の re-unwrap で process abort していた)。
    #[tokio::test(flavor = "current_thread")]
    async fn start_panic_rejects_and_keeps_manager_healthy() {
        let manager = ResourceManager::new("/workspace", SymlinkPolicy::Deny);
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            manager.request(request_options(
                "a",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                |_| -> Result<ResourceLease, AbortReason> { panic!("start boom") },
            )),
        )
        .await
        .expect("panicking start must not abort the process");
        let error = result.unwrap_err();
        assert!(error.message().contains("start boom"), "{error}");
        // lease は巻き戻し済みで mutex も poison していない → 後続が通る。
        let lease = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            manager.request(request_options(
                "b",
                LeaseKind::Job,
                prep(&["/workspace/a"], ClaimMode::Exclusive),
                lease_start(),
            )),
        )
        .await
        .expect("manager must remain usable after start panic")
        .unwrap();
        manager.release(lease);
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn canonicalizes_symlink_root_before_conflicts() {
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir().join(format!(
            "cheaper-res-{}-{}",
            std::process::id(),
            NEXT_LEASE.load(Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&base).unwrap();
        let alias = base.join("alias");
        symlink(&base, &alias).unwrap();
        let manager = ResourceManager::new(alias.to_str().unwrap(), SymlinkPolicy::Deny);
        assert_eq!(manager.root, realpath(base.to_str().unwrap()).unwrap());
        let _ = std::fs::remove_dir_all(&base);
    }
}
