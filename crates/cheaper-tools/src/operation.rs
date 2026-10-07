//! Port of `packages/tool-runtime/src/operation.ts` (T2-2).
//!
//! `OperationManager` — 六状態機械(queued/running/completed/failed/
//! cancelled/timed_out)・cancel の first-reason 保存・owner 単位 cancel・
//! `drain`(grace 経過で未 settle を `unsettled` 分類して detach)・
//! 注入可能な monotonic clock(ms `f64`)。
//!
//! ADR-034zT6bqTxOvK1nP0ELSyR / 共有 ADR D1/D3/D7。scheduler の責務は
//! 持たない。
//!
//! ## AbortSignal/AbortController 相当
//!
//! plan File Map の規則に従い、TS の `AbortSignal`/`AbortController`/
//! `AbortSignal.any` を `CancellationToken` + first-reason `OnceCell` +
//! 同期 listener として再実装する。listener は abort 発生スレッド上で
//! 同期起動される(TS の `"abort"` event listener 相当)ため、abort 直後の
//! `state === "cancelled"` 観測が deterministic に成立する。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};
use std::time::Duration;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::resources::types::ResourceClaim;
use cheaper_core::workspace::paths::WorkspaceEscapeError;
use serde::Serialize;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::execution_metrics::{collect_execution_metrics, ExecutionMetrics};
use crate::ToolError;

// ---------------------------------------------------------------------------
// AbortReason — TS `unknown` throw/abort 値の Rust 表現
// ---------------------------------------------------------------------------

/// `describeError` 済みの name/message/code と、reason が error 値だった
/// 場合の元 object(`Arc` 共有)を保持する。TS の `signal.reason` /
/// `catch (error)` 値の同一性は Arc で近似する。
#[derive(Debug)]
struct ReasonInner {
    name: String,
    message: String,
    code: Option<String>,
    error: Option<ToolError>,
}

/// TS の abort reason / thrown `unknown` 値。`CheaperError` は
/// name `"CheaperError"` + `code` を保持し、`TIMEOUT`/`TimeoutError` で
/// `timed_out` に分類される。
#[derive(Debug, Clone)]
pub struct AbortReason {
    inner: Arc<ReasonInner>,
}

impl AbortReason {
    /// `{name, message, code}` を明示して作る(generic payload /
    /// `DOMException("...", "TimeoutError")` 相当)。
    pub fn new(name: impl Into<String>, message: impl Into<String>, code: Option<String>) -> Self {
        Self {
            inner: Arc::new(ReasonInner {
                name: name.into(),
                message: message.into(),
                code,
                error: None,
            }),
        }
    }

    /// `new Error(message)` / 文字列 payload 相当(`name` は `"Error"`)。
    /// (`message` 名は getter が占有するため `msg` とする。)
    pub fn msg(message: impl Into<String>) -> Self {
        Self::new("Error", message, None)
    }

    /// `cancel()`/`cancelAll()`/`cancelOwner()` の既定 reason
    /// (`new Error("Operation cancelled")`)。
    pub fn cancelled() -> Self {
        Self::msg("Operation cancelled")
    }

    /// `controller.abort()` を reason なしで呼んだときの Node 既定値
    /// (`new DOMException("This operation was aborted", "AbortError")`)。
    pub fn aborted() -> Self {
        Self::new("AbortError", "This operation was aborted", None)
    }

    /// error 値を reason 化する。`CheaperError` は code/name を
    /// `describeError` と同じ形で取り出す。`AbortReason` を包んでいた場合は
    /// 二重包装せず中身をそのまま使う。
    pub fn from_error(error: ToolError) -> Self {
        let dyn_err: &(dyn std::error::Error + Send + Sync) = error.as_ref();
        if let Some(reason) = dyn_err.downcast_ref::<AbortReason>() {
            return reason.clone();
        }
        let (name, message, code) = describe_error_object(dyn_err);
        Self {
            inner: Arc::new(ReasonInner {
                name,
                message,
                code,
                error: Some(error),
            }),
        }
    }

    /// `describeError` の `{name}`。
    pub fn name(&self) -> &str {
        &self.inner.name
    }

    /// `describeError` の `{message}`。
    pub fn message(&self) -> &str {
        &self.inner.message
    }

    /// `describeError` の `{code}`。
    pub fn code(&self) -> Option<&str> {
        self.inner.code.as_deref()
    }

    /// reason が error 値だった場合の元 object(`signal.reason` を
    /// downcast で検査したい呼び出し側向け)。
    pub fn error_object(&self) -> Option<&ToolError> {
        self.inner.error.as_ref()
    }

    /// `describeError` 相当の `{name, message, code}`。
    pub fn describe(&self) -> OperationError {
        OperationError {
            name: self.inner.name.clone(),
            message: self.inner.message.clone(),
            code: self.inner.code.clone(),
        }
    }
}

impl std::fmt::Display for AbortReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.inner.message)
    }
}

impl std::error::Error for AbortReason {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner
            .error
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

impl From<CheaperError> for AbortReason {
    fn from(e: CheaperError) -> Self {
        Self::from_error(Box::new(e))
    }
}

/// `&CheaperError` → AbortReason。borrowed しか持てない callback 文脈
/// (process の `onTimeout(&reason)` 等)からの変換用。`cause` は Error
/// trait object のため複製できないが、code/message/details を保持した
/// rebuild で error object を残す(source chain から `CheaperError` が
/// downcast できる形 — `find_cheaper_error`/`code()` 判定 parity)。
impl From<&CheaperError> for AbortReason {
    fn from(e: &CheaperError) -> Self {
        let mut rebuilt = CheaperError::new(e.code(), e.message());
        if let Some(details) = e.details() {
            rebuilt = rebuilt.with_details(details.clone());
        }
        Self::from_error(Box::new(rebuilt))
    }
}

impl From<ToolError> for AbortReason {
    fn from(e: ToolError) -> Self {
        Self::from_error(e)
    }
}

impl From<&str> for AbortReason {
    fn from(s: &str) -> Self {
        Self::msg(s)
    }
}

impl From<String> for AbortReason {
    fn from(s: String) -> Self {
        Self::msg(s)
    }
}

/// JS `error.name`/`error.code`/`error.message` の近似。
/// `CheaperError` → `"CheaperError"` + `code`、`WorkspaceEscapeError` →
/// 同名、その他の error は `"Error"`(TS `Error` 基底 name)。
fn describe_error_object(
    e: &(dyn std::error::Error + Send + Sync + 'static),
) -> (String, String, Option<String>) {
    if let Some(ce) = e.downcast_ref::<CheaperError>() {
        return (
            "CheaperError".to_string(),
            ce.message().to_string(),
            Some(ce.code().as_str().to_string()),
        );
    }
    if let Some(we) = e.downcast_ref::<WorkspaceEscapeError>() {
        return ("WorkspaceEscapeError".to_string(), we.to_string(), None);
    }
    // `path.posix.matchesGlob` の over-long pattern — Node は
    // `TypeError` を投げるので name を揃える。
    if let Some(te) = e.downcast_ref::<crate::glob::PatternTooLongError>() {
        return ("TypeError".to_string(), te.to_string(), None);
    }
    // `normalizeResourceManifest` の `ResourceError` — inner の
    // CheaperError/WorkspaceEscapeError と同じ name/code を保つ。
    if let Some(re) = e.downcast_ref::<cheaper_core::resources::normalization::ResourceError>() {
        return match re {
            cheaper_core::resources::normalization::ResourceError::Invalid(ce) => (
                "CheaperError".to_string(),
                ce.message().to_string(),
                Some(ce.code().as_str().to_string()),
            ),
            cheaper_core::resources::normalization::ResourceError::Escape(we) => {
                ("WorkspaceEscapeError".to_string(), we.to_string(), None)
            }
        };
    }
    // Node fs `UVException` 相当 — `CountTokensError::Io` 等で enum wrap
    // される経路も拾うため source chain を走査する(TS では raw fs error
    // がそのまま投げられ `error.code` を持つ)。
    {
        let mut cur: Option<&(dyn std::error::Error + 'static)> =
            Some(e as &(dyn std::error::Error + 'static));
        while let Some(err) = cur {
            if let Some(node) = err.downcast_ref::<cheaper_core::errors::NodeIoError>() {
                return (
                    "Error".to_string(),
                    node.to_string(),
                    Some(node.code().to_string()),
                );
            }
            cur = err.source();
        }
    }
    ("Error".to_string(), e.to_string(), None)
}

// ---------------------------------------------------------------------------
// AbortSignal / AbortController
// ---------------------------------------------------------------------------

/// `signal.addEventListener("abort", …)` の登録 id(`{once:true}` 相当 —
/// abort 時に一度だけ呼ばれ、その後は自動除去される)。
pub type AbortListenerId = u64;

type AbortListener = Box<dyn FnOnce(&AbortReason) + Send + 'static>;

/// listener map + dispatch 済みフラグ。`drained` と map の insert/drain
/// は同一 mutex で serialize する: drain 後に登録された listener は
/// JS と同じく二度と呼ばれないため即 drop し、map に残して leak
/// させない(登録と abort drain の race 対策)。
struct Listeners {
    map: HashMap<AbortListenerId, AbortListener>,
    drained: bool,
}

struct SignalInner {
    token: CancellationToken,
    reason: OnceLock<AbortReason>,
    listeners: Mutex<Listeners>,
    next_listener: AtomicU64,
    /// `AbortSignal.any` の source 保持。JS では composite が source を
    /// 強参照する(GC が伝播させる)ため、Rust でも composite の生存中は
    /// source の inner を保持する。保持しないと `any([a, b])` の結果だけを
    /// 残して source handle を drop した場合、source→composite の listener
    /// 経路ごと消えて abort が伝播しない。listener 側は `Weak` のため
    /// ここで cycle は発生しない。
    sources: Vec<AbortSignal>,
}

impl std::fmt::Debug for SignalInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignalInner")
            .field("token", &self.token)
            // `sources` は read されない liveness keep(`any` composite の
            // GC 相当)のため、len だけを Debug に出して使用済みにする。
            .field("sources", &self.sources.len())
            .finish_non_exhaustive()
    }
}

impl SignalInner {
    fn abort(&self, reason: AbortReason) {
        if self.reason.set(reason.clone()).is_err() {
            return; // first reason wins
        }
        self.token.cancel();
        // listener 起動前に lock を離す: listener は manager/resource の
        // 状態更新や再入 abort(remove/no-op)を行い得るため。
        let listeners: Vec<AbortListener> = {
            let mut l = self.listeners.lock().unwrap();
            l.drained = true;
            l.map.drain().map(|(_, f)| f).collect()
        };
        for listener in listeners {
            listener(&reason);
        }
    }
}

/// TS `AbortSignal` 相当: 一度だけ abort され、first reason を保持し、
/// `{once:true}` listener を同期 dispatch する。
#[derive(Debug)]
pub struct AbortSignal {
    inner: Arc<SignalInner>,
}

impl Clone for AbortSignal {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl PartialEq for AbortSignal {
    /// record.signal === execute 引数 signal の参照比較(registry が
    /// active operation を特定する用途)を `Arc` identity で再現。
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for AbortSignal {}

impl AbortSignal {
    fn from_inner(inner: Arc<SignalInner>) -> Self {
        Self { inner }
    }

    /// `new AbortController().signal` 相当(abort 経路を持たない signal)。
    pub fn detached() -> Self {
        AbortController::new().signal()
    }

    /// `AbortSignal.any(signals)` 相当: どれかの source が abort したら
    /// その reason で composite を abort する(first reason wins)。
    /// source listener は `Weak` 経由で composite を参照するため、
    /// composite が drop されても listener は残留しない。逆方向は
    /// `SignalInner.sources` で強参照する(JS の GC 相当 — composite の
    /// 生存中は source も生存する)。
    pub fn any(signals: &[AbortSignal]) -> Self {
        let composite = AbortSignal {
            inner: Arc::new(SignalInner {
                token: CancellationToken::new(),
                reason: OnceLock::new(),
                listeners: Mutex::new(Listeners {
                    map: HashMap::new(),
                    drained: false,
                }),
                next_listener: AtomicU64::new(0),
                sources: signals.to_vec(),
            }),
        };
        for source in signals {
            let weak = Arc::downgrade(&composite.inner);
            source.on_abort(move |reason| {
                if let Some(inner) = weak.upgrade() {
                    inner.abort(reason.clone());
                }
            });
            // 登録後に abort 済み確認(登録とチェックの間の abort は
            // listener が拾い、OnceLock が first reason を守る)。
            if let Some(reason) = source.reason() {
                composite.inner.abort(reason);
            }
        }
        composite
    }

    /// `signal.aborted`。
    pub fn is_aborted(&self) -> bool {
        self.inner.token.is_cancelled()
    }

    /// `signal.reason`(未 abort なら `None`)。
    pub fn reason(&self) -> Option<AbortReason> {
        self.inner.reason.get().cloned()
    }

    /// abort まで解決しない future(async wait 用の token 経路)。
    pub async fn cancelled(&self) {
        self.inner.token.cancelled().await;
    }

    /// `addEventListener("abort", f, {once:true})`。既に abort 済みでも
    /// 呼ばれない(TS と同じく `aborted` チェックは呼び出し側の責務)。
    pub fn on_abort(
        &self,
        listener: impl FnOnce(&AbortReason) + Send + 'static,
    ) -> AbortListenerId {
        let id = self.inner.next_listener.fetch_add(1, Ordering::Relaxed);
        let mut l = self.inner.listeners.lock().unwrap();
        // dispatch 済みなら JS と同じく二度と呼ばれない。map に残さず
        // 即 drop して leak を防ぐ(insert と drain は同じ lock で
        // serialize されるため、ここで見えた `drained` は確定)。
        if !l.drained {
            l.map.insert(id, Box::new(listener));
        }
        id
    }

    /// `removeEventListener("abort", f)`。
    pub fn remove_listener(&self, id: AbortListenerId) {
        self.inner.listeners.lock().unwrap().map.remove(&id);
    }
}

/// TS `AbortController` 相当。`signal()` は clone 可能な handle を返す。
#[derive(Debug)]
pub struct AbortController {
    inner: Arc<SignalInner>,
}

impl AbortController {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SignalInner {
                token: CancellationToken::new(),
                reason: OnceLock::new(),
                listeners: Mutex::new(Listeners {
                    map: HashMap::new(),
                    drained: false,
                }),
                next_listener: AtomicU64::new(0),
                sources: Vec::new(),
            }),
        }
    }

    pub fn signal(&self) -> AbortSignal {
        AbortSignal::from_inner(Arc::clone(&self.inner))
    }

    /// `controller.abort(reason)`。first reason wins、listener は
    /// 同期 dispatch される。
    pub fn abort(&self, reason: AbortReason) {
        self.inner.abort(reason);
    }
}

impl Default for AbortController {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AbortController {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// ---------------------------------------------------------------------------
// 状態機械
// ---------------------------------------------------------------------------

/// `OperationState`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl OperationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::TimedOut => "timed_out",
        }
    }

    fn is_terminal(self) -> bool {
        !matches!(self, Self::Queued | Self::Running)
    }
}

/// `transitions` 表(変更禁止の契約表)。TS と同一形状で定義する。
fn allowed_transitions(from: OperationState) -> &'static [OperationState] {
    match from {
        OperationState::Queued => &[
            OperationState::Running,
            OperationState::Cancelled,
            OperationState::TimedOut,
        ],
        OperationState::Running => &[
            OperationState::Completed,
            OperationState::Failed,
            OperationState::Cancelled,
            OperationState::TimedOut,
        ],
        OperationState::Completed
        | OperationState::Failed
        | OperationState::Cancelled
        | OperationState::TimedOut => &[],
    }
}

/// `assertOperationTransition`。
pub fn assert_operation_transition(
    from: OperationState,
    to: OperationState,
) -> Result<(), CheaperError> {
    if !allowed_transitions(from).contains(&to) {
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            format!(
                "invalid operation transition: {} -> {}",
                from.as_str(),
                to.as_str()
            ),
        ));
    }
    Ok(())
}

/// `OperationError`(`describeError` の結果形)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationError {
    pub name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// `cancellation` ブロック。`state` は `"cancelled" | "timed_out"` の
/// みを取る(`OperationState` の subset)。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperationCancellation {
    #[serde(rename = "requestedAt")]
    pub requested_at: f64,
    pub state: OperationState,
    pub reason: OperationError,
}

/// `settlement`(`"pending" | "settled" | "unsettled"`)。`unsettled` は
/// grace 超過時の明示分類で、実停止や terminal の意味を持たない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Settlement {
    Pending,
    Settled,
    Unsettled,
}

/// `OperationRecord`。`get()`/`records()` は内部 record の clone を返す
/// (TS の defensive snapshot 相当。`signal` は同一 inner を指す clone)。
#[derive(Debug, Clone, Serialize)]
pub struct OperationRecord {
    pub id: String,
    #[serde(rename = "ownerId")]
    pub owner_id: String,
    pub kind: String,
    #[serde(rename = "toolName", skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub state: OperationState,
    #[serde(rename = "createdAt")]
    pub created_at: f64,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub started_at: Option<f64>,
    #[serde(rename = "finishedAt", skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<f64>,
    #[serde(skip)]
    pub signal: AbortSignal,
    #[serde(rename = "resourceClaims", skip_serializing_if = "Option::is_none")]
    pub resource_claims: Option<Vec<ResourceClaim>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancellation: Option<OperationCancellation>,
    pub settlement: Settlement,
    pub owned: bool,
    #[serde(rename = "detachedAt", skip_serializing_if = "Option::is_none")]
    pub detached_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
}

/// `CreateOperationOptions`。`signal` は owner scope と call-context の
/// composite を runtime 側から渡す seam。
#[derive(Default)]
pub struct CreateOperationOptions {
    pub owner_id: String,
    pub kind: String,
    pub tool_name: Option<String>,
    pub resource_claims: Option<Vec<ResourceClaim>>,
    pub signal: Option<AbortSignal>,
}

/// monotonic clock(ms)。`performance.now()` 相当の `f64` を返す。
pub type Clock = Arc<dyn Fn() -> f64 + Send + Sync>;

/// `OperationManagerOptions`。
#[derive(Default)]
pub struct OperationManagerOptions {
    pub signal: Option<AbortSignal>,
    pub abort_grace_ms: Option<f64>,
    /// timestamp は monotonic。同じ clock を invocation の全 owner が共有する。
    pub clock: Option<Clock>,
}

struct Entry {
    record: OperationRecord,
    controller: AbortController,
    /// `record.signal` へ登録した abort listener(cleanup で除去)。
    listener: Option<AbortListenerId>,
}

#[derive(Default)]
struct OwnerConcurrency {
    running: u64,
    maximum: u64,
}

#[derive(Default)]
struct ManagerState {
    /// insertion order を保つため Vec + id index(TS Map 相当)。
    entries: Vec<Entry>,
    index: HashMap<String, usize>,
    closed_owners: HashSet<String>,
    closed: bool,
    next_id: u64,
    running: u64,
    max_concurrency: u64,
    owner_concurrency: HashMap<String, OwnerConcurrency>,
    root_listener: Option<AbortListenerId>,
}

struct ManagerInner {
    state: Mutex<ManagerState>,
    clock: Clock,
    grace_ms: f64,
    root_signal: Option<AbortSignal>,
    /// finish 毎に drain 待機側を起こす(`entry.settled` promise 相当の
    /// manager 集約版。drain は開始時の pending snapshot を追跡する)。
    settle_notify: Notify,
}

impl ManagerInner {
    fn lock(&self) -> MutexGuard<'_, ManagerState> {
        self.state.lock().unwrap()
    }

    fn entry_index(&self, state: &ManagerState, id: &str) -> Result<usize, CheaperError> {
        state.index.get(id).copied().ok_or_else(|| {
            CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("unknown operation: {id}"),
            )
        })
    }

    /// `finish` — state lock 下で record/state/counter/listener を更新する。
    /// `settle_notify` の発火は呼び出し側が lock を離してから行う。
    fn finish_locked(&self, state: &mut ManagerState, index: usize, to: OperationState) {
        let entry = &mut state.entries[index];
        debug_assert!(allowed_transitions(entry.record.state).contains(&to));
        if entry.record.state == OperationState::Running {
            state.running -= 1;
            if let Some(owner) = state.owner_concurrency.get_mut(&entry.record.owner_id) {
                owner.running -= 1;
            }
        }
        entry.record.state = to;
        entry.record.finished_at = Some((self.clock)());
        entry.record.owned = false;
        entry.record.settlement = Settlement::Settled;
        if let Some(listener) = entry.listener.take() {
            entry.record.signal.remove_listener(listener);
        }
    }

    /// `requestCancellation` — first reason を記録し、queued なら即座に
    /// terminal へ遷移させ、最後に controller.abort で signal listener を
    /// 起こす。listener 起動は必ず lock 解放後(reentrancy 許容)。
    fn request_cancellation(&self, id: &str, reason: AbortReason) {
        let (finished, controller) = {
            let mut state = self.lock();
            let Some(index) = state.index.get(id).copied() else {
                return;
            };
            let entry = &mut state.entries[index];
            // TS `onAbort` と同じく terminal record には触れない
            // (finish 済みなら listener は cleanup で除去済みのはずだが、
            // composite signal の登録順によっては残り得るため防御)。
            if entry.record.state.is_terminal() || entry.record.cancellation.is_some() {
                return;
            }
            let cancellation_state = cancellation_state(&reason);
            entry.record.cancellation = Some(OperationCancellation {
                requested_at: (self.clock)(),
                state: cancellation_state,
                reason: reason.describe(),
            });
            let finish = entry.record.state == OperationState::Queued;
            let controller = entry.controller.clone();
            if finish {
                self.finish_locked(&mut state, index, cancellation_state);
            }
            (finish, controller)
        };
        if finished {
            self.settle_notify.notify_one();
        }
        controller.abort(reason);
    }
}

fn default_clock() -> f64 {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f64()
        * 1000.0
}

/// `cancellationState(reason)`: code `TIMEOUT`/`TimeoutError` または
/// name `TimeoutError` で `timed_out`、それ以外は `cancelled`。
fn cancellation_state(reason: &AbortReason) -> OperationState {
    match reason.code() {
        Some("TIMEOUT") | Some("TimeoutError") => OperationState::TimedOut,
        _ => {
            if reason.name() == "TimeoutError" {
                OperationState::TimedOut
            } else {
                OperationState::Cancelled
            }
        }
    }
}

fn invalid_request(message: &str) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidRequest, message)
}

/// ADR-034zT6bqTxOvK1nP0ELSyR / 共有 ADR D1/D3/D7。
/// `Clone` は同一内部 state を共有する(TS の同一 instance 参照相当)。
pub struct OperationManager {
    inner: Arc<ManagerInner>,
}

impl Clone for OperationManager {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl OperationManager {
    pub fn new(options: OperationManagerOptions) -> Result<Self, CheaperError> {
        let grace_ms = validate_grace(options.abort_grace_ms.unwrap_or(5000.0))?;
        let inner = Arc::new(ManagerInner {
            state: Mutex::new(ManagerState::default()),
            clock: options.clock.unwrap_or_else(|| Arc::new(default_clock)),
            grace_ms,
            root_signal: options.signal.clone(),
            settle_notify: Notify::new(),
        });
        if let Some(signal) = &options.signal {
            let weak: Weak<ManagerInner> = Arc::downgrade(&inner);
            let listener_id = signal.on_abort(move |reason| {
                if let Some(inner) = weak.upgrade() {
                    let reason = reason.clone();
                    inner.lock().closed = true;
                    cancel_all_locked(&inner, reason);
                }
            });
            inner.lock().root_listener = Some(listener_id);
            // pre-aborted root: TS の `if (this.rootSignal?.aborted)`。
            if let Some(reason) = signal.reason() {
                inner.lock().closed = true;
                cancel_all_locked(&inner, reason);
            }
        }
        Ok(Self { inner })
    }

    /// `create`。queued record を作り、composite signal を張る。
    pub fn create(&self, options: CreateOperationOptions) -> Result<OperationRecord, CheaperError> {
        {
            let state = self.inner.lock();
            if state.closed || state.closed_owners.contains(&options.owner_id) {
                return Err(invalid_request("Operation aborted: admission closed"));
            }
            if options.owner_id.is_empty() || options.kind.is_empty() {
                return Err(invalid_request("operation owner and kind must be nonempty"));
            }
        }
        let controller = AbortController::new();
        let controller_signal = controller.signal();
        let signal = match &options.signal {
            Some(external) => AbortSignal::any(&[controller_signal.clone(), external.clone()]),
            None => controller_signal,
        };
        let id = {
            let mut state = self.inner.lock();
            state.next_id += 1;
            format!("op-{}", state.next_id)
        };
        let record = OperationRecord {
            id: id.clone(),
            owner_id: options.owner_id.clone(),
            kind: options.kind.clone(),
            tool_name: options.tool_name,
            state: OperationState::Queued,
            created_at: (self.inner.clock)(),
            started_at: None,
            finished_at: None,
            signal: signal.clone(),
            resource_claims: options.resource_claims.map(|c| copy_claims(&c)),
            cancellation: None,
            settlement: Settlement::Pending,
            owned: true,
            detached_at: None,
            result: None,
            error: None,
        };
        {
            let mut state = self.inner.lock();
            let position = state.entries.len();
            state.index.insert(id.clone(), position);
            state.entries.push(Entry {
                record,
                controller,
                listener: None,
            });
        }
        // signal abort → requestCancellation({once:true})。composite の
        // reason がそのまま渡される。
        let weak = Arc::downgrade(&self.inner);
        let entry_id = id.clone();
        let listener_id = signal.on_abort(move |reason| {
            if let Some(inner) = weak.upgrade() {
                inner.request_cancellation(&entry_id, reason.clone());
            }
        });
        {
            let mut state = self.inner.lock();
            let index = state.index[&id];
            state.entries[index].listener = Some(listener_id);
        }
        // 既 abort(composite 化の前後問わず)を取りこぼさない。
        if let Some(reason) = signal.reason() {
            self.inner.request_cancellation(&id, reason);
        }
        self.get(&id)
    }

    /// `queue`(`create` の別名)。
    pub fn queue(&self, options: CreateOperationOptions) -> Result<OperationRecord, CheaperError> {
        self.create(options)
    }

    /// `setResourceClaims` — resource preparation は queued 状態でのみ許可。
    pub fn set_resource_claims(
        &self,
        id: &str,
        claims: &[ResourceClaim],
    ) -> Result<(), CheaperError> {
        let mut state = self.inner.lock();
        let index = self.inner.entry_index(&state, id)?;
        let entry = &mut state.entries[index];
        if entry.record.state != OperationState::Queued {
            return Err(invalid_request(
                "resource preparation requires queued operation",
            ));
        }
        entry.record.resource_claims = Some(copy_claims(claims));
        Ok(())
    }

    /// `start` — queued → running。`startedAt` を stamp し concurrency
    /// high-water を更新する。
    pub fn start(&self, id: &str) -> Result<OperationRecord, CheaperError> {
        let mut state = self.inner.lock();
        let index = self.inner.entry_index(&state, id)?;
        let owner_id = state.entries[index].record.owner_id.clone();
        assert_operation_transition(state.entries[index].record.state, OperationState::Running)?;
        {
            let entry = &mut state.entries[index];
            entry.record.state = OperationState::Running;
            entry.record.started_at = Some((self.inner.clock)());
        }
        state.running += 1;
        state.max_concurrency = state.max_concurrency.max(state.running);
        let owner = state.owner_concurrency.entry(owner_id).or_default();
        owner.running += 1;
        owner.maximum = owner.maximum.max(owner.running);
        drop(state);
        self.get(id)
    }

    /// `complete` — running → completed(queued cancel が先行していれば
    /// cancellation.state の terminal 分類を採る)。
    pub fn complete(
        &self,
        id: &str,
        result: serde_json::Value,
    ) -> Result<OperationRecord, CheaperError> {
        let mut state = self.inner.lock();
        let index = self.inner.entry_index(&state, id)?;
        let entry = &mut state.entries[index];
        assert_operation_transition(entry.record.state, OperationState::Completed)?;
        entry.record.result = Some(result);
        let terminal = entry
            .record
            .cancellation
            .as_ref()
            .map(|c| c.state)
            .unwrap_or(OperationState::Completed);
        self.inner.finish_locked(&mut state, index, terminal);
        drop(state);
        self.inner.settle_notify.notify_one();
        self.get(id)
    }

    /// `fail` — running → failed(cancel 先行時は同上)。
    pub fn fail(
        &self,
        id: &str,
        error: impl Into<AbortReason>,
    ) -> Result<OperationRecord, CheaperError> {
        let reason = error.into();
        let mut state = self.inner.lock();
        let index = self.inner.entry_index(&state, id)?;
        let entry = &mut state.entries[index];
        assert_operation_transition(entry.record.state, OperationState::Failed)?;
        entry.record.error = Some(reason.describe());
        let terminal = entry
            .record
            .cancellation
            .as_ref()
            .map(|c| c.state)
            .unwrap_or(OperationState::Failed);
        self.inner.finish_locked(&mut state, index, terminal);
        drop(state);
        self.inner.settle_notify.notify_one();
        self.get(id)
    }

    /// `cancel` — reason を first-cause として記録する。
    pub fn cancel(
        &self,
        id: &str,
        reason: impl Into<AbortReason>,
    ) -> Result<OperationRecord, CheaperError> {
        let reason = reason.into();
        {
            let state = self.inner.lock();
            let index = self.inner.entry_index(&state, id)?;
            let entry = &state.entries[index];
            assert_operation_transition(entry.record.state, cancellation_state(&reason))?;
        }
        self.inner.request_cancellation(id, reason);
        self.get(id)
    }

    /// `cancelOwner` — owner の admission を閉じ、未 terminal の全 record を
    /// 同一 reason で cancel する。listener が再入しても安全
    /// (id 収集と cancel を分離)。
    pub fn cancel_owner(&self, owner_id: &str, reason: impl Into<AbortReason>) {
        let reason = reason.into();
        let ids = {
            let mut state = self.inner.lock();
            state.closed_owners.insert(owner_id.to_string());
            state
                .entries
                .iter()
                .filter(|e| e.record.owner_id == owner_id && !e.record.state.is_terminal())
                .map(|e| e.record.id.clone())
                .collect::<Vec<_>>()
        };
        for id in ids {
            self.inner.request_cancellation(&id, reason.clone());
        }
    }

    /// `cancelAll` — invocation 全体の admission を閉じる。
    pub fn cancel_all(&self, reason: impl Into<AbortReason>) {
        let reason = reason.into();
        self.inner.lock().closed = true;
        cancel_all_locked(&self.inner, reason);
    }

    /// lifecycle の低水準 API。admission bound は持たず、tool 実行には
    /// `ToolScheduler` を使う。
    pub async fn run<T, E, F, Fut>(
        &self,
        options: CreateOperationOptions,
        execute: F,
    ) -> Result<T, AbortReason>
    where
        F: FnOnce(AbortSignal) -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        T: Serialize,
        E: Into<AbortReason>,
    {
        let operation = self.create(options).map_err(AbortReason::from)?;
        if let Some(reason) = operation.signal.reason() {
            return Err(reason);
        }
        self.start(&operation.id).map_err(AbortReason::from)?;
        match execute(operation.signal.clone()).await {
            Ok(result) => {
                self.complete(
                    &operation.id,
                    serde_json::to_value(&result).unwrap_or(serde_json::Value::Null),
                )
                .map_err(AbortReason::from)?;
                Ok(result)
            }
            Err(error) => {
                let reason = error.into();
                let _ = self.fail(&operation.id, reason.clone());
                Err(reason)
            }
        }
    }

    /// `get` — snapshot clone(claims/cancellation/error は deep clone)。
    pub fn get(&self, id: &str) -> Result<OperationRecord, CheaperError> {
        let state = self.inner.lock();
        let index = self.inner.entry_index(&state, id)?;
        Ok(state.entries[index].record.clone())
    }

    /// `records` — insertion order(TS Map values order)を保つ。
    pub fn records(&self, owner_id: Option<&str>) -> Vec<OperationRecord> {
        let state = self.inner.lock();
        state
            .entries
            .iter()
            .filter(|e| owner_id.is_none_or(|o| e.record.owner_id == o))
            .map(|e| e.record.clone())
            .collect()
    }

    /// D3: owner-local または invocation 全体の集合から同じ定義で算出する。
    pub fn execution_metrics(&self, owner_id: Option<&str>) -> ExecutionMetrics {
        let snapshot_at = (self.inner.clock)();
        let state = self.inner.lock();
        let maximum = match owner_id {
            None => state.max_concurrency,
            Some(owner) => state
                .owner_concurrency
                .get(owner)
                .map(|o| o.maximum)
                .unwrap_or(0),
        };
        let records: Vec<OperationRecord> = state
            .entries
            .iter()
            .filter(|e| owner_id.is_none_or(|o| e.record.owner_id == o))
            .map(|e| e.record.clone())
            .collect();
        drop(state);
        collect_execution_metrics(&records, snapshot_at, maximum)
    }

    /// `drain` — cancel 後、pending(owned)record の settle を grace まで
    /// 待ち、残りを `unsettled`/`detached` に分類する。
    pub async fn drain(
        &self,
        owner_id: Option<&str>,
        abort_grace_ms: Option<f64>,
    ) -> Result<(), CheaperError> {
        let grace = validate_grace(abort_grace_ms.unwrap_or(self.inner.grace_ms))?;
        match owner_id {
            None => self.cancel_all(AbortReason::cancelled()),
            Some(owner) => self.cancel_owner(owner, AbortReason::cancelled()),
        }
        let pending: Vec<String> = {
            let state = self.inner.lock();
            state
                .entries
                .iter()
                .filter(|e| e.record.owned && owner_id.is_none_or(|o| e.record.owner_id == o))
                .map(|e| e.record.id.clone())
                .collect()
        };
        if pending.is_empty() {
            return Ok(());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs_f64(grace / 1000.0);
        loop {
            // notified を先に登録してから条件を確認する(missed wake 回避)。
            let notified = self.inner.settle_notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let done = {
                let state = self.inner.lock();
                pending.iter().all(|id| match state.index.get(id) {
                    Some(&i) => state.entries[i].record.settlement == Settlement::Settled,
                    None => true,
                })
            };
            if done {
                break;
            }
            tokio::select! {
                _ = notified => {}
                _ = tokio::time::sleep_until(deadline) => break,
            }
        }
        let mut state = self.inner.lock();
        for id in pending {
            let Some(&index) = state.index.get(&id) else {
                continue;
            };
            let entry = &mut state.entries[index];
            if !entry.record.owned {
                continue;
            }
            entry.record.owned = false;
            entry.record.settlement = Settlement::Unsettled;
            entry.record.detached_at = Some((self.inner.clock)());
            if let Some(listener) = entry.listener.take() {
                entry.record.signal.remove_listener(listener);
            }
        }
        Ok(())
    }

    /// `dispose` — drain + root listener の解除。TS は promise を
    /// memoize するが、drain は冪等なため再実行で同等になる。
    pub async fn dispose(&self) {
        let _ = self.drain(None, None).await;
        let listener = self.inner.lock().root_listener.take();
        if let (Some(signal), Some(id)) = (&self.inner.root_signal, listener) {
            signal.remove_listener(id);
        }
    }
}

/// `cancelAll` の state 部分(closed 設定は呼び出し側/コンストラクタが行う)。
fn cancel_all_locked(inner: &ManagerInner, reason: AbortReason) {
    let ids: Vec<String> = inner
        .lock()
        .entries
        .iter()
        .filter(|e| !e.record.state.is_terminal())
        .map(|e| e.record.id.clone())
        .collect();
    for id in ids {
        inner.request_cancellation(&id, reason.clone());
    }
}

/// `copyClaims` — `{...claim, resource: {...claim.resource}}` の浅い clone。
fn copy_claims(claims: &[ResourceClaim]) -> Vec<ResourceClaim> {
    claims.to_vec()
}

fn validate_grace(grace: f64) -> Result<f64, CheaperError> {
    if !grace.is_finite() || grace < 0.0 {
        return Err(invalid_request(
            "abort grace must be a finite nonnegative number",
        ));
    }
    Ok(grace)
}

#[cfg(test)]
mod tests {
    //! `operation.test.ts` 対応。fake clock(ManualClock)と deferred
    //! handler(`run` に渡す pending future)で deterministically に再現する。
    //! `setTimeout` 相当の grace だけは実時間(tokio::time)で小さく使う。

    use std::sync::{Arc, Mutex};

    use super::*;

    struct ManualClock(Arc<Mutex<f64>>);

    impl ManualClock {
        fn new(at: f64) -> Self {
            Self(Arc::new(Mutex::new(at)))
        }
        fn set(&self, v: f64) {
            *self.0.lock().unwrap() = v;
        }
        fn clock(&self) -> Clock {
            let inner = Arc::clone(&self.0);
            Arc::new(move || *inner.lock().unwrap())
        }
    }

    fn input() -> CreateOperationOptions {
        CreateOperationOptions {
            owner_id: "a".to_string(),
            kind: "tool".to_string(),
            tool_name: Some("write".to_string()),
            ..Default::default()
        }
    }

    fn manager(options: OperationManagerOptions) -> OperationManager {
        OperationManager::new(options).unwrap()
    }

    const STATES: [OperationState; 6] = [
        OperationState::Queued,
        OperationState::Running,
        OperationState::Completed,
        OperationState::Failed,
        OperationState::Cancelled,
        OperationState::TimedOut,
    ];

    /// `operation.test.ts` の手書き期待行列(実装の transition 表は再利用しない)。
    const ALLOWED: [[bool; 6]; 6] = [
        [false, true, false, false, true, true],
        [false, false, true, true, true, true],
        [false; 6],
        [false; 6],
        [false; 6],
        [false; 6],
    ];

    #[test]
    fn operation_state_contract_transition_table() {
        for (row, from) in STATES.iter().enumerate() {
            for (column, to) in STATES.iter().enumerate() {
                match (
                    ALLOWED[row][column],
                    assert_operation_transition(*from, *to),
                ) {
                    (true, Ok(())) => {}
                    (false, Err(error)) => assert!(
                        error.message().contains("transition"),
                        "{from:?} -> {to:?} should be invalid"
                    ),
                    (expected, result) => {
                        panic!("{from:?} -> {to:?}: expected {expected}, got {result:?}")
                    }
                }
            }
        }
    }

    #[test]
    fn stamps_times_and_preserves_result_and_failure() {
        let clock = ManualClock::new(10.0);
        let m = manager(OperationManagerOptions {
            clock: Some(clock.clock()),
            ..Default::default()
        });
        let first = m.create(input()).unwrap();
        assert_eq!(first.state, OperationState::Queued);
        assert_eq!(first.created_at, 10.0);
        assert!(first.owned);
        assert!(first.started_at.is_none());
        clock.set(20.0);
        let started = m.start(&first.id).unwrap();
        assert_eq!(started.state, OperationState::Running);
        assert_eq!(started.started_at, Some(20.0));
        clock.set(40.0);
        let completed = m
            .complete(&first.id, serde_json::json!({ "answer": 1 }))
            .unwrap();
        assert_eq!(completed.state, OperationState::Completed);
        assert_eq!(completed.finished_at, Some(40.0));
        assert_eq!(completed.result, Some(serde_json::json!({ "answer": 1 })));
        assert!(!completed.owned);
        assert_eq!(completed.settlement, Settlement::Settled);

        let second = m.queue(input()).unwrap();
        assert_ne!(second.id, first.id);
        m.start(&second.id).unwrap();
        let failed = m
            .fail(
                &second.id,
                AbortReason::from(CheaperError::new(ErrorCode::ToolExecutionFailure, "broken")),
            )
            .unwrap();
        assert_eq!(
            failed.error,
            Some(OperationError {
                name: "CheaperError".to_string(),
                message: "broken".to_string(),
                code: Some("TOOL_EXECUTION_FAILURE".to_string()),
            })
        );
        assert_eq!(failed.state, OperationState::Failed);
        assert_eq!(failed.finished_at, Some(40.0));
    }

    #[test]
    fn rejects_lifecycle_methods_from_wrong_state_and_keeps_terminal_metadata() {
        let m = manager(OperationManagerOptions::default());
        let queued = m.create(input()).unwrap();
        assert!(m.complete(&queued.id, serde_json::Value::Null).is_err());
        assert!(m.fail(&queued.id, AbortReason::msg("no")).is_err());
        for terminal in [
            OperationState::Completed,
            OperationState::Failed,
            OperationState::Cancelled,
            OperationState::TimedOut,
        ] {
            let op = m.create(input()).unwrap();
            match terminal {
                OperationState::Completed | OperationState::Failed => {
                    m.start(&op.id).unwrap();
                    match terminal {
                        OperationState::Completed => {
                            m.complete(&op.id, serde_json::json!(1)).unwrap();
                        }
                        _ => {
                            m.fail(&op.id, AbortReason::msg("failure")).unwrap();
                        }
                    }
                }
                _ => {
                    let reason = if terminal == OperationState::TimedOut {
                        AbortReason::new("TimeoutError", "deadline", None)
                    } else {
                        AbortReason::from("stop")
                    };
                    m.cancel(&op.id, reason).unwrap();
                }
            }
            let before = m.get(&op.id).unwrap();
            assert!(m.start(&op.id).is_err());
            assert!(m.complete(&op.id, serde_json::json!(2)).is_err());
            assert!(m.fail(&op.id, AbortReason::from("changed")).is_err());
            assert!(m.cancel(&op.id, AbortReason::from("changed")).is_err());
            let after = m.get(&op.id).unwrap();
            assert_eq!(after.state, before.state);
            assert_eq!(after.finished_at, before.finished_at);
        }
        let error = m.start("unknown").unwrap_err();
        assert!(error.message().contains("unknown operation"));
    }

    #[test]
    fn classifies_cancel_reasons_with_queued_terminal_timestamp() {
        // (name, code) → timed_out | cancelled。
        for (reason, want) in [
            (
                AbortReason::new("Error", "deadline", Some("TIMEOUT".to_string())),
                OperationState::TimedOut,
            ),
            (
                AbortReason::new("Error", "deadline", Some("TimeoutError".to_string())),
                OperationState::TimedOut,
            ),
            (
                AbortReason::new("TimeoutError", "deadline", None),
                OperationState::TimedOut,
            ),
            (
                AbortReason::from(CheaperError::new(ErrorCode::Timeout, "deadline")),
                OperationState::TimedOut,
            ),
            (
                AbortReason::new("Error", "budget", Some("MAX_TURNS_EXCEEDED".to_string())),
                OperationState::Cancelled,
            ),
        ] {
            let clock = ManualClock::new(20.0);
            let m = manager(OperationManagerOptions {
                clock: Some(clock.clock()),
                ..Default::default()
            });
            let op = m.create(input()).unwrap();
            let cancelled = m.cancel(&op.id, reason).unwrap();
            assert_eq!(cancelled.state, want);
            assert_eq!(cancelled.finished_at, Some(20.0));
            assert_eq!(cancelled.settlement, Settlement::Settled);
            assert!(!cancelled.owned);
            assert!(m.get(&op.id).unwrap().signal.is_aborted());
        }
    }

    #[test]
    fn retains_first_cancellation_cause_and_ends_running_on_actual_settlement() {
        let clock = ManualClock::new(10.0);
        let m = manager(OperationManagerOptions {
            clock: Some(clock.clock()),
            ..Default::default()
        });
        let op = m.create(input()).unwrap();
        clock.set(20.0);
        m.start(&op.id).unwrap();
        clock.set(30.0);
        m.cancel(
            &op.id,
            AbortReason::from(CheaperError::new(ErrorCode::Timeout, "first")),
        )
        .unwrap();
        m.cancel(&op.id, AbortReason::msg("second")).unwrap();
        let record = m.get(&op.id).unwrap();
        assert_eq!(record.state, OperationState::Running);
        let cancellation = record.cancellation.as_ref().unwrap();
        assert_eq!(cancellation.requested_at, 30.0);
        assert_eq!(cancellation.reason.code.as_deref(), Some("TIMEOUT"));
        assert_eq!(cancellation.reason.message, "first");
        assert!(record.finished_at.is_none());
        clock.set(50.0);
        let completed = m
            .complete(&op.id, serde_json::json!("late result"))
            .unwrap();
        assert_eq!(completed.state, OperationState::TimedOut);
        assert_eq!(completed.finished_at, Some(50.0));
        assert_eq!(completed.result, Some(serde_json::json!("late result")));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn root_signal_cancels_queued_and_running_and_closes_admission() {
        let root = AbortController::new();
        let m = manager(OperationManagerOptions {
            signal: Some(root.signal()),
            abort_grace_ms: Some(0.0),
            ..Default::default()
        });
        let queued = m.create(input()).unwrap();
        let running = m
            .create(CreateOperationOptions {
                owner_id: "b".to_string(),
                ..input()
            })
            .unwrap();
        m.start(&running.id).unwrap();
        root.abort(AbortReason::from(CheaperError::new(
            ErrorCode::Aborted,
            "workflow cancellation",
        )));
        let record = m.get(&queued.id).unwrap();
        assert_eq!(record.state, OperationState::Cancelled);
        assert_eq!(
            record.cancellation.unwrap().reason.message,
            "workflow cancellation"
        );
        assert!(m.get(&running.id).unwrap().signal.is_aborted());
        let error = m
            .create(CreateOperationOptions {
                owner_id: "c".to_string(),
                ..input()
            })
            .unwrap_err();
        assert!(error.message().contains("aborted") || error.message().contains("admission"));
        m.dispose().await;
        assert_eq!(m.records(None).iter().filter(|o| o.owned).count(), 0);
        // late settle は cancelled を採る(detach 済み record への完了)。
        m.fail(&running.id, AbortReason::msg("settled rejection"))
            .unwrap();
        let record = m.get(&running.id).unwrap();
        assert_eq!(record.state, OperationState::Cancelled);
        assert_eq!(record.error.unwrap().message, "settled rejection");
        m.dispose().await;
    }

    #[test]
    fn owner_cancellation_closes_only_its_admission() {
        let m = manager(OperationManagerOptions::default());
        let call = AbortController::new();
        let a = m
            .create(CreateOperationOptions {
                signal: Some(call.signal()),
                ..input()
            })
            .unwrap();
        let b = m
            .create(CreateOperationOptions {
                owner_id: "b".to_string(),
                ..input()
            })
            .unwrap();
        call.abort(AbortReason::from("call stop"));
        assert_eq!(m.get(&a.id).unwrap().state, OperationState::Cancelled);
        let same_owner = m.create(input()).unwrap();
        m.cancel_owner("a", AbortReason::msg("owner stop"));
        assert_eq!(
            m.get(&same_owner.id).unwrap().state,
            OperationState::Cancelled
        );
        assert!(m.create(input()).is_err());
        m.start(&b.id).unwrap();
        assert_eq!(
            m.complete(&b.id, serde_json::json!("sibling"))
                .unwrap()
                .state,
            OperationState::Completed
        );
        m.cancel_all(AbortReason::from("root stop"));
        assert!(m
            .queue(CreateOperationOptions {
                owner_id: "c".to_string(),
                ..input()
            })
            .is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pre_aborted_root_and_call_without_executing_work() {
        let root = AbortController::new();
        root.abort(AbortReason::from("root stop"));
        let m = manager(OperationManagerOptions {
            signal: Some(root.signal()),
            ..Default::default()
        });
        assert!(m.create(input()).is_err());

        let call = AbortController::new();
        call.abort(AbortReason::from("call stop"));
        let n = manager(OperationManagerOptions::default());
        let executed = Arc::new(AtomicU64::new(0));
        let executed2 = executed.clone();
        let error = n
            .run(
                CreateOperationOptions {
                    signal: Some(call.signal()),
                    ..input()
                },
                move |_| {
                    let executed = executed2.clone();
                    async move {
                        executed.fetch_add(1, Ordering::Relaxed);
                        Ok::<serde_json::Value, AbortReason>(serde_json::json!(1))
                    }
                },
            )
            .await
            .unwrap_err();
        assert_eq!(error.message(), "call stop");
        assert_eq!(executed.load(Ordering::Relaxed), 0);
        let record = &n.records(None)[0];
        assert_eq!(record.state, OperationState::Cancelled);
        assert!(record.finished_at.is_some());
        assert!(record.started_at.is_none());
    }

    /// `bounded drain classifies unsettled work and safely observes late
    /// settle`(resolve/reject 両 outcome)。
    #[tokio::test(flavor = "current_thread")]
    async fn bounded_drain_detaches_and_observes_late_settle() {
        for reject in [false, true] {
            let m = manager(OperationManagerOptions {
                abort_grace_ms: Some(30.0),
                ..Default::default()
            });
            let (tx, rx) = tokio::sync::oneshot::channel::<Result<&'static str, &'static str>>();
            let pending = {
                let m = m.clone();
                tokio::spawn(async move {
                    m.run(input(), move |_| async move {
                        match rx.await {
                            Ok(Ok(v)) => Ok(serde_json::json!(v)),
                            Ok(Err(message)) => Err(AbortReason::msg(message)),
                            Err(_) => Err(AbortReason::msg("channel closed")),
                        }
                    })
                    .await
                })
            };
            // run が start に達するまで回す。
            for _ in 0..8 {
                tokio::task::yield_now().await;
            }
            let disposal = {
                let m = m.clone();
                tokio::spawn(async move { m.dispose().await })
            };
            for _ in 0..8 {
                tokio::task::yield_now().await;
            }
            {
                let record = &m.records(None)[0];
                assert_eq!(record.state, OperationState::Running);
                assert!(record.signal.is_aborted());
            }
            disposal.await.unwrap();
            {
                let record = &m.records(None)[0];
                assert_eq!(record.state, OperationState::Running);
                assert!(!record.owned);
                assert_eq!(record.settlement, Settlement::Unsettled);
                assert!(record.detached_at.is_some());
                assert!(record.finished_at.is_none());
            }
            if reject {
                let _ = tx.send(Err("late failure"));
            } else {
                let _ = tx.send(Ok("late"));
            }
            let outcome = pending.await.unwrap();
            if reject {
                assert_eq!(outcome.unwrap_err().message(), "late failure");
            } else {
                assert_eq!(outcome.unwrap(), serde_json::json!("late"));
            }
            let record = &m.records(None)[0];
            // late settle は first-reason(drain 時の default cancel)で分類。
            assert_eq!(record.state, OperationState::Cancelled);
            assert!(!record.owned);
            assert_eq!(record.settlement, Settlement::Settled);
            assert!(record.finished_at.is_some());
            m.dispose().await;
        }
    }

    #[test]
    fn rejects_nonfinite_or_negative_grace() {
        for bad in [f64::NAN, f64::INFINITY, -1.0] {
            let result = OperationManager::new(OperationManagerOptions {
                abort_grace_ms: Some(bad),
                ..Default::default()
            });
            assert!(result.is_err(), "grace {bad} should be rejected");
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn drain_waits_for_grace_then_detaches() {
        let m = manager(OperationManagerOptions {
            abort_grace_ms: Some(60.0),
            ..Default::default()
        });
        let op = m.create(input()).unwrap();
        m.start(&op.id).unwrap();
        let m2 = m.clone();
        let pending = tokio::spawn(async move {
            m2.dispose().await;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!pending.is_finished());
        pending.await.unwrap();
        assert!(m.get(&op.id).unwrap().detached_at.is_some());
    }

    #[test]
    fn snapshots_cannot_rewrite_lifecycle_state_or_resource_claims() {
        let m = manager(OperationManagerOptions::default());
        let claim = ResourceClaim {
            resource: cheaper_core::resources::types::Resource::workspace(
                "/workspace/a",
                cheaper_core::resources::types::ResourceScope::Exact,
            ),
            mode: cheaper_core::resources::types::ClaimMode::Shared,
        };
        let op = m
            .create(CreateOperationOptions {
                resource_claims: Some(vec![claim]),
                ..input()
            })
            .unwrap();
        // snapshot の clone をいじっても内部 record は不変
        // (Rust では返却値は deep clone)。
        let mut snapshot = m.get(&op.id).unwrap();
        snapshot.state = OperationState::Failed;
        snapshot.resource_claims.as_mut().unwrap()[0].resource =
            cheaper_core::resources::types::Resource::workspace(
                "/changed-snapshot",
                cheaper_core::resources::types::ResourceScope::Exact,
            );
        let record = m.get(&op.id).unwrap();
        assert_eq!(
            record.resource_claims.unwrap()[0].resource.path(),
            "/workspace/a"
        );
        assert_eq!(record.state, OperationState::Queued);
    }

    #[test]
    fn abort_signal_any_propagates_first_reason() {
        let a = AbortController::new();
        let b = AbortController::new();
        let composite = AbortSignal::any(&[a.signal(), b.signal()]);
        assert!(!composite.is_aborted());
        a.abort(AbortReason::from("first"));
        assert!(composite.is_aborted());
        assert_eq!(composite.reason().unwrap().message(), "first");
        // second abort は first reason を上書きしない。
        b.abort(AbortReason::from("second"));
        assert_eq!(composite.reason().unwrap().message(), "first");
    }

    #[test]
    fn abort_signal_any_inherits_pre_aborted_reason() {
        let a = AbortController::new();
        a.abort(AbortReason::from("already"));
        let composite = AbortSignal::any(&[a.signal(), AbortSignal::detached()]);
        assert_eq!(composite.reason().unwrap().message(), "already");
    }

    #[test]
    fn abort_signal_fires_listener_once_and_supports_removal() {
        let controller = AbortController::new();
        let signal = controller.signal();
        let fired = Arc::new(AtomicU64::new(0));
        let f = fired.clone();
        let _keep = signal.on_abort(move |r| {
            assert_eq!(r.message(), "why");
            f.fetch_add(1, Ordering::Relaxed);
        });
        let removable = signal.on_abort(|_| unreachable!("removed listener"));
        signal.remove_listener(removable);
        controller.abort(AbortReason::from("why"));
        controller.abort(AbortReason::from("again"));
        assert_eq!(fired.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn abort_signal_listener_on_aborted_signal_is_dropped() {
        // dispatch 済み signal への登録は JS と同じく呼ばれない。
        // map に残さず即 drop する(leak しない)ことも確認する。
        let controller = AbortController::new();
        let signal = controller.signal();
        controller.abort(AbortReason::from("early"));
        let _late = signal.on_abort(|_| unreachable!("post-abort listener"));
        assert!(signal.inner.listeners.lock().unwrap().map.is_empty());
    }

    #[test]
    fn abort_signal_register_abort_race_leaves_no_stored_listeners() {
        // `on_abort` と `abort` の race: 各 listener は「発火する」か
        // 「登録されず drop される」かのどちらかで、drain 後に map に
        // 取り残されることはない。bounded な試行回数で回帰確認する。
        for _ in 0..200 {
            let controller = AbortController::new();
            let signal = controller.signal();
            let fired = Arc::new(AtomicU64::new(0));
            std::thread::scope(|scope| {
                let s2 = signal.clone();
                let f2 = fired.clone();
                scope.spawn(move || {
                    for _ in 0..50 {
                        let f3 = f2.clone();
                        s2.on_abort(move |_| {
                            f3.fetch_add(1, Ordering::Relaxed);
                        });
                    }
                });
                scope.spawn(move || {
                    controller.abort(AbortReason::from("raced"));
                });
            });
            assert!(
                signal.inner.listeners.lock().unwrap().map.is_empty(),
                "listener が drain 済み map に取り残された"
            );
        }
    }
}
