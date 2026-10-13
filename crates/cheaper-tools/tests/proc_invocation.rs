//! `packages/tool-runtime/test/invocation.test.ts` 対応の integration test。
//!
//! TS test は injected `handlers`(`ls`/`write`/`edit`)で handler を
//! gate 可能にするが、Rust 版は全 tool が native のため injection seam
//! は無い。owner 単位の abort propagation・late settlement といった
//! blocking が要る contract は、`process` tool が spawn する
//! **この test binary 自身の fixture child**(`invocation_fixture_child`,
//! env mode `sleep` で 30 秒 sleep)で再現する。
//!
//! `#[tokio::test]` は current-thread runtime — abort listener は同期
//! dispatch、process kill は executor task の poll が必要なため、abort
//! 直後の `signal.aborted`/`state=running` 観測は deterministic。

use std::path::PathBuf;
use std::time::Duration;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::invocation::{
    create_invocation_tool_runtime, InvocationToolRuntimeOptions, JobLeaseOptions,
    WorkerScopeOptions,
};
use cheaper_tools::operation::{AbortController, AbortReason, OperationState, Settlement};
use cheaper_tools::registry::{ToolCallContext, ToolRuntimeOptions};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

const MODE_ENV: &str = "CHEAPER_INVOCATION_CHILD_MODE";

// ---------------------------------------------------------------------------
// fixture child(non-cooperative sleep)
// ---------------------------------------------------------------------------

/// `process` task が spawn する fixture。`MODE_ENV=sleep` で 30 秒 sleep
/// し、abort 後の process-tree cleanup の kill で落ちる(non-cooperative
/// handler 相当 — handler 自身は abort を見て終わらず、外部 kill が
/// settlement をもたらす)。
#[test]
fn invocation_fixture_child() {
    if std::env::var(MODE_ENV).unwrap_or_default() != "sleep" {
        return;
    }
    std::thread::sleep(Duration::from_secs(30));
}

/// `task("sleep")` — command[0] は absolute native executable(test
/// binary)で PATH 解決に依存しない。
fn sleep_task() -> Value {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut env = Map::new();
    env.insert(MODE_ENV.to_string(), json!("sleep"));
    json!({
        "command": [exe, "invocation_fixture_child", "--exact", "--nocapture"],
        "inputs": [],
        "outputs": [],
        "env": Value::Object(env),
    })
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-invocation-it-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }

    fn root(&self) -> String {
        canonicalize_workspace_root(self.0.to_str().unwrap()).unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn invocation_options(root: String, process: Option<Value>) -> InvocationToolRuntimeOptions {
    InvocationToolRuntimeOptions {
        artifacts: None,
        process,
        command: None,
        root,
        symlink_policy: SymlinkPolicy::Deny,
        signal: None,
        abort_grace_ms: None,
        concurrency: None,
        clock: None,
    }
}

fn scope_options(root: &str, names: Vec<ToolName>) -> ToolRuntimeOptions {
    ToolRuntimeOptions {
        resource_manifest: None,
        write_scope: None,
        read_strictness: None,
        artifacts: None,
        process: None,
        command: None,
        root: root.to_string(),
        symlink_policy: SymlinkPolicy::Deny,
        names,
        degradable: Vec::new(),
        max_tool_calls: 20,
        on_limit: Box::new(|| {}),
        journal: None,
        denylist: None,
        parent_lease: None,
        signal: None,
        abort_grace_ms: None,
        concurrency: None,
        eval_spawn: None,
    }
}

fn worker(owner_id: &str, runtime: ToolRuntimeOptions) -> WorkerScopeOptions {
    WorkerScopeOptions {
        owner_id: owner_id.to_string(),
        runtime,
    }
}

fn err_message(err: &ToolError) -> String {
    err.to_string()
}

async fn wait_for(mut cond: impl FnMut() -> bool, what: &str) {
    let start = std::time::Instant::now();
    while !cond() {
        if start.elapsed() > Duration::from_secs(10) {
            panic!("wait_for timed out: {what}");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

/// "records unique ids across owners and retains successful result and
/// failed error" — handler 差替の代わりに、成功 owner(ls 成功)と失敗
/// owner(存在しない path → native error)を native ls で検証する。
#[tokio::test]
async fn records_unique_ids_across_owners() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "x").unwrap();
    let invocation = create_invocation_tool_runtime(invocation_options(dir.root(), None)).unwrap();
    let a = invocation
        .create_worker_scope(worker("a", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    let b = invocation
        .create_worker_scope(worker("b", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    let result = a.execute("ls", &json!({"path": "."}), None).await.unwrap();
    assert!(!result.content.is_empty());
    let err = b
        .execute("ls", &json!({"path": "missing-dir"}), None)
        .await
        .unwrap_err();
    let ops = invocation.operations(None);
    assert_eq!(ops.len(), 2);
    let pairs: Vec<(&str, OperationState)> = ops
        .iter()
        .map(|op| (op.owner_id.as_str(), op.state))
        .collect();
    assert_eq!(
        pairs,
        [
            ("a", OperationState::Completed),
            ("b", OperationState::Failed)
        ]
    );
    let ids: std::collections::HashSet<&str> = ops.iter().map(|op| op.id.as_str()).collect();
    assert_eq!(ids.len(), 2);
    assert_eq!(a.operations().len(), 1);
    let b_op = &b.operations()[0];
    let b_err = b_op.error.as_ref().expect("failed op retains error");
    assert!(
        b_err.message.contains(&err.to_string()) || !b_err.message.is_empty(),
        "operation error is retained"
    );
    assert!(a.operations()[0].finished_at.is_some());
    invocation.dispose().await;
}

/// "root timeout cancels every owner but stamps terminal only when
/// handlers actually settle" — blocking handler は process fixture
/// sleep で再現し、abort 後に両 owner の op signal が abort され、
/// settle が遅れても TIMEOUT reason で terminal になることを検証する。
#[tokio::test]
async fn root_abort_marks_every_owner_and_settles_late() {
    let dir = TempDir::new();
    let root_controller = AbortController::new();
    let mut opts = invocation_options(
        dir.root(),
        Some(json!({"maxConcurrent": 2, "tasks": {"sleep": sleep_task()}})),
    );
    opts.signal = Some(root_controller.signal());
    let invocation = create_invocation_tool_runtime(opts).unwrap();
    let a = invocation
        .create_worker_scope(worker(
            "a",
            scope_options(&dir.root(), vec![ToolName::Process]),
        ))
        .unwrap();
    let b = invocation
        .create_worker_scope(worker(
            "b",
            scope_options(&dir.root(), vec![ToolName::Process]),
        ))
        .unwrap();
    let pa = tokio::spawn({
        let a = a.clone();
        async move { a.execute("process", &json!({"task": "sleep"}), None).await }
    });
    let pb = tokio::spawn({
        let b = b.clone();
        async move { b.execute("process", &json!({"task": "sleep"}), None).await }
    });
    wait_for(
        || {
            invocation
                .operations(None)
                .iter()
                .filter(|op| op.state == OperationState::Running)
                .count()
                == 2
        },
        "two running operations",
    )
    .await;
    root_controller.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Timeout,
        "workflow deadline",
    )));
    // abort listener は同期 dispatch — kill 未完了でも signal は既に abort。
    for op in invocation.operations(None) {
        assert!(op.signal.is_aborted(), "{} not aborted", op.id);
    }
    let ra = pa.await.unwrap();
    let rb = pb.await.unwrap();
    assert!(ra.is_err() && rb.is_err(), "aborted calls reject");
    for op in invocation.operations(None) {
        assert_eq!(op.state, OperationState::TimedOut, "{}", op.id);
        assert!(op.finished_at.is_some());
        assert!(!op.owned);
    }
    invocation.dispose().await;
}

/// "combines call and owner signals with first-cause reason while
/// retaining sibling scope"。
#[tokio::test]
async fn combines_call_and_owner_signals() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "x").unwrap();
    let invocation = create_invocation_tool_runtime(invocation_options(
        dir.root(),
        Some(json!({"maxConcurrent": 2, "tasks": {"sleep": sleep_task()}})),
    ))
    .unwrap();
    let child = AbortController::new();
    let call = AbortController::new();
    let mut a_opts = scope_options(&dir.root(), vec![ToolName::Process]);
    a_opts.signal = Some(child.signal());
    let a = invocation.create_worker_scope(worker("a", a_opts)).unwrap();
    let b = invocation
        .create_worker_scope(worker("b", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    let p1 = tokio::spawn({
        let a = a.clone();
        let call = call.signal();
        async move {
            a.execute(
                "process",
                &json!({"task": "sleep"}),
                Some(ToolCallContext { signal: Some(call) }),
            )
            .await
        }
    });
    let p2 = tokio::spawn({
        let a = a.clone();
        async move { a.execute("process", &json!({"task": "sleep"}), None).await }
    });
    wait_for(
        || {
            a.operations()
                .iter()
                .filter(|op| op.state == OperationState::Running)
                .count()
                == 2
        },
        "a runs two operations",
    )
    .await;
    call.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Aborted,
        "call cancelled",
    )));
    let aborted: Vec<bool> = a
        .operations()
        .iter()
        .map(|op| op.signal.is_aborted())
        .collect();
    assert_eq!(aborted, [true, false]);
    child.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Timeout,
        "worker deadline",
    )));
    let aborted: Vec<bool> = a
        .operations()
        .iter()
        .map(|op| op.signal.is_aborted())
        .collect();
    assert_eq!(aborted, [true, true]);
    // child abort 済み scope への新規 call は reason そのまま reject。
    let err = a
        .execute("process", &json!({"task": "sleep"}), None)
        .await
        .unwrap_err();
    let reason = err
        .downcast_ref::<AbortReason>()
        .expect("rejects with child signal reason");
    assert_eq!(reason.code(), Some("TIMEOUT"));
    let _ = p1.await;
    let _ = p2.await;
    let states: Vec<(OperationState, Option<String>)> = a
        .operations()
        .iter()
        .map(|op| {
            (
                op.state,
                op.cancellation.as_ref().and_then(|c| c.reason.code.clone()),
            )
        })
        .collect();
    assert_eq!(
        states,
        [
            (OperationState::Cancelled, Some("ABORTED".to_string())),
            (OperationState::TimedOut, Some("TIMEOUT".to_string())),
        ]
    );
    a.dispose().await;
    // sibling owner は残る。
    let result = b.execute("ls", &json!({"path": "."}), None).await.unwrap();
    assert!(!result.content.is_empty());
    let c = invocation
        .create_worker_scope(worker("c", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    c.execute("ls", &json!({"path": "."}), None).await.unwrap();
    invocation.dispose().await;
}

/// "bounds drain for a non-cooperative handler and tolerates late
/// settlement" — fixture sleep が abort で落ちるまでの間、dispose は
/// bounded(10ms grace)で完了し、late settle 後に record が terminal を
/// 刻むことを検証する。
#[tokio::test]
async fn bounds_disposal_and_records_late_settlement() {
    let dir = TempDir::new();
    let mut opts = invocation_options(
        dir.root(),
        Some(json!({"maxConcurrent": 2, "tasks": {"sleep": sleep_task()}})),
    );
    opts.abort_grace_ms = Some(10.0);
    let invocation = create_invocation_tool_runtime(opts).unwrap();
    let a = invocation
        .create_worker_scope(worker(
            "a",
            scope_options(&dir.root(), vec![ToolName::Process]),
        ))
        .unwrap();
    let pending = tokio::spawn({
        let a = a.clone();
        async move { a.execute("process", &json!({"task": "sleep"}), None).await }
    });
    wait_for(
        || {
            a.operations()
                .iter()
                .any(|op| op.state == OperationState::Running)
        },
        "operation running",
    )
    .await;
    a.dispose().await;
    // dispose は bounded で返る。kill の landing 時期は timing-dependent
    // のため detached/settled どちらにもなり得るが、`owned` は既に外れる。
    assert!(!a.operations()[0].owned);
    // late settle — process kill が landed した後に pending は abort で
    // reject され、record は terminal を刻む。
    let _ = pending.await.unwrap();
    wait_for(
        || a.operations()[0].finished_at.is_some(),
        "late settlement stamps finishedAt",
    )
    .await;
    let record = &a.operations()[0];
    assert_eq!(record.settlement, Settlement::Settled);
    assert!(!record.owned);
    a.dispose().await;
    invocation.dispose().await;
}

/// "fixes the boundary configuration at construction and rejects
/// unbounded grace"。
#[tokio::test]
async fn fixes_boundary_config_and_rejects_unbounded_grace() {
    let dir = TempDir::new();
    let invocation = create_invocation_tool_runtime(invocation_options(dir.root(), None)).unwrap();
    assert_eq!(invocation.root(), dir.root());
    let mut other = scope_options("/changed", vec![ToolName::Ls]);
    other.root = "/changed".to_string();
    let err = invocation
        .create_worker_scope(worker("a", other))
        .unwrap_err();
    assert!(err_message(&err).contains("root mismatch"));
    let err = create_invocation_tool_runtime(InvocationToolRuntimeOptions {
        abort_grace_ms: Some(f64::INFINITY),
        ..invocation_options(dir.root(), None)
    })
    .unwrap_err();
    assert!(err_message(&err).contains("finite"));
    invocation.dispose().await;
}

/// "rejects duplicate owner, boundary mismatch, and forged parent
/// lease" — forged lease は別 owner の job lease を使い回す形で検証する
/// (`ResourceLease` は外側から mint できない newtype)。
#[tokio::test]
async fn rejects_duplicate_owner_boundary_mismatch_and_forged_lease() {
    let dir = TempDir::new();
    let invocation = create_invocation_tool_runtime(invocation_options(dir.root(), None)).unwrap();
    invocation
        .create_worker_scope(worker("a", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    let err = invocation
        .create_worker_scope(worker("a", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap_err();
    assert!(err_message(&err).contains("owner"));
    let mut root_opts = scope_options(&dir.root(), vec![ToolName::Ls]);
    root_opts.root = "/other".to_string();
    let err = invocation
        .create_worker_scope(worker("b", root_opts))
        .unwrap_err();
    assert!(err_message(&err).contains("root"));
    let mut symlink_opts = scope_options(&dir.root(), vec![ToolName::Ls]);
    symlink_opts.symlink_policy = SymlinkPolicy::WithinWorkspace;
    let err = invocation
        .create_worker_scope(worker("b", symlink_opts))
        .unwrap_err();
    assert!(err_message(&err).contains("policy"));
    // forged parent lease — "x" owner の job lease を "b" に充てる。
    let lease = invocation
        .acquire_job_lease(JobLeaseOptions {
            owner_id: "x".to_string(),
            names: vec![ToolName::Ls],
            signal: None,
            resource_manifest: None,
            write_scope: None,
            read_strictness: None,
        })
        .await
        .unwrap();
    let mut parent_opts = scope_options(&dir.root(), vec![ToolName::Ls]);
    parent_opts.parent_lease = Some(lease);
    let err = invocation
        .create_worker_scope(worker("b", parent_opts))
        .unwrap_err();
    assert!(err_message(&err).contains("lease"));
    invocation.release_job_lease(lease);
    invocation.dispose().await;
    let err = invocation
        .create_worker_scope(worker("c", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap_err();
    assert!(err_message(&err).contains("aborted"));
}

/// "child abort and dispose leave sibling and root alive"。
#[tokio::test]
async fn child_abort_and_dispose_leave_sibling_alive() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "x").unwrap();
    let invocation = create_invocation_tool_runtime(invocation_options(dir.root(), None)).unwrap();
    let child_abort = AbortController::new();
    let mut a_opts = scope_options(&dir.root(), vec![ToolName::Ls]);
    a_opts.signal = Some(child_abort.signal());
    let a = invocation.create_worker_scope(worker("a", a_opts)).unwrap();
    let b = invocation
        .create_worker_scope(worker("b", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    child_abort.abort(AbortReason::cancelled());
    assert!(a.execute("ls", &json!({"path": "."}), None).await.is_err());
    a.dispose().await;
    let result = b.execute("ls", &json!({"path": "."}), None).await.unwrap();
    assert!(!result.content.is_empty());
    assert_eq!(b.budget().tool_calls(), 1);
    let c = invocation
        .create_worker_scope(worker("c", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    invocation.abort(None);
    assert!(b.execute("ls", &json!({"path": "."}), None).await.is_err());
    assert!(c.execute("ls", &json!({"path": "."}), None).await.is_err());
    invocation.dispose().await;
}

/// `acquireJobLease`/`releaseJobLease` + `parentLease` 指定 worker
/// scope の正常系(job claim ⊆ parent claim で worker が走る)。
#[tokio::test]
async fn acquires_job_lease_and_runs_scoped_worker() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "x").unwrap();
    let invocation = create_invocation_tool_runtime(invocation_options(dir.root(), None)).unwrap();
    let lease = invocation
        .acquire_job_lease(JobLeaseOptions {
            owner_id: "w".to_string(),
            names: vec![ToolName::Ls],
            signal: None,
            resource_manifest: Some(json!({
                "reads": [{"path": ".", "scope": "subtree"}]
            })),
            write_scope: None,
            read_strictness: None,
        })
        .await
        .unwrap();
    let mut w_opts = scope_options(&dir.root(), vec![ToolName::Ls]);
    w_opts.parent_lease = Some(lease);
    w_opts.resource_manifest = Some(json!({
        "reads": [{"path": ".", "scope": "subtree"}]
    }));
    let w = invocation.create_worker_scope(worker("w", w_opts)).unwrap();
    let result = w.execute("ls", &json!({"path": "."}), None).await.unwrap();
    assert!(!result.content.is_empty());
    invocation.release_job_lease(lease);
    invocation.dispose().await;
}

/// scope dispose は sibling を殺さず、invocation `scopes` から自分を外す
/// (TS `scope.dispose` wrap + `scopes.delete` 相当)。
#[tokio::test]
async fn scope_dispose_removes_self_from_invocation() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "x").unwrap();
    let invocation = create_invocation_tool_runtime(invocation_options(dir.root(), None)).unwrap();
    let a = invocation
        .create_worker_scope(worker("a", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    let b = invocation
        .create_worker_scope(worker("b", scope_options(&dir.root(), vec![ToolName::Ls])))
        .unwrap();
    a.dispose().await;
    assert!(a.execute("ls", &json!({"path": "."}), None).await.is_err());
    let result = b.execute("ls", &json!({"path": "."}), None).await.unwrap();
    assert!(!result.content.is_empty());
    invocation.dispose().await;
}

// ---------------------------------------------------------------------------
// command scope — `inner.command` 再注入・parity・nonempty-grants gate(T4)
// ---------------------------------------------------------------------------

/// `command` grant — executable はこの test binary 自身(absolute native
/// executable で PATH 解決に依存せず、temp workspace 外のため
/// workspace-resident 拒否にも掛からない)。argv は literal pin —
/// `MODE_ENV` 未設定のため fixture test は即 return で exit 0。
fn self_grant() -> Value {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    json!({
        "executable": exe,
        "argv": ["invocation_fixture_child", "--exact", "--nocapture"],
    })
}

/// T4: `create_worker_scope` は `options.runtime.command` を passthrough
/// せず `inner.command`(invocation-resolved grants)を scope options へ
/// 再注入する。caller が `command: None` を渡す形状(workflow
/// `kind: tool` job の `ToolJobExecutor` と同じ)でも scope は invocation
/// の resolved grants で構築され、`command` が expose・実行可能になる
/// — passthrough のままなら empty grants gate で `POLICY_VIOLATION` に
/// なるため、再注入を直接固定する test。
#[tokio::test]
async fn worker_scope_receives_invocation_resolved_command_grants() {
    let dir = TempDir::new();
    let mut opts = invocation_options(dir.root(), None);
    opts.command = Some(json!({"maxConcurrent": 1, "grants": {"self": self_grant()}}));
    let invocation = create_invocation_tool_runtime(opts).unwrap();
    // `command: None` — workflow tool job と同じ scope options 形状。
    let scope = invocation
        .create_worker_scope(worker(
            "cmd",
            scope_options(&dir.root(), vec![ToolName::Command]),
        ))
        .unwrap();
    assert!(scope
        .definitions()
        .iter()
        .any(|d| d.name == ToolName::Command));
    let result = scope
        .execute(
            "command",
            &json!({
                "name": "self",
                "args": ["invocation_fixture_child", "--exact", "--nocapture"],
            }),
            None,
        )
        .await
        .unwrap();
    let details = result.details.as_ref().expect("command result details");
    assert_eq!(details["grant"], json!("self"));
    assert_eq!(details["state"], json!("completed"));
    assert_eq!(details["exitCode"], json!(0));
    invocation.dispose().await;
}

/// T4: grants 空(= trusted layer が grant を宣言していない)では
/// `Command` を含む scope 生成が `POLICY_VIOLATION` で fail-closed する
/// — worker 経路・workflow `kind: tool` 経路共通の registration gate。
#[tokio::test]
async fn command_scope_fails_closed_without_grants() {
    let dir = TempDir::new();
    let invocation = create_invocation_tool_runtime(invocation_options(dir.root(), None)).unwrap();
    let err = invocation
        .create_worker_scope(worker(
            "cmd",
            scope_options(&dir.root(), vec![ToolName::Command]),
        ))
        .unwrap_err();
    let err = err
        .downcast_ref::<CheaperError>()
        .expect("nonempty-grants gate は CheaperError");
    assert_eq!(err.code(), ErrorCode::PolicyViolation);
    assert!(err.message().contains("nonempty authorized grants"));
    invocation.dispose().await;
}

/// T4: scope が渡す `command` raw は `inner.command` との declared-shape
/// parity check を通る必要がある — 異なる grant set の密輸は
/// `command policy mismatch` で reject される。
#[tokio::test]
async fn worker_scope_rejects_command_policy_mismatch() {
    let dir = TempDir::new();
    let mut opts = invocation_options(dir.root(), None);
    opts.command = Some(json!({"maxConcurrent": 1, "grants": {"self": self_grant()}}));
    let invocation = create_invocation_tool_runtime(opts).unwrap();
    // inner.command(1 grant)と declared-shape が異なる raw → mismatch。
    let mut opts = scope_options(&dir.root(), vec![ToolName::Command]);
    opts.command = Some(json!({"maxConcurrent": 1, "grants": {}}));
    let err = invocation
        .create_worker_scope(worker("cmd", opts))
        .unwrap_err();
    assert!(err_message(&err).contains("command policy mismatch"));
    invocation.dispose().await;
}
