//! `cheaper.optionalTools` opt-in 縮退機構(design §(e))の registration
//! gate test — `create_tool_runtime` 直結。
//!
//! content-gate(nonempty `tasks`/`grants`)不成立時と pin 失敗時に
//! `options.degradable` membership の gated tool は `POLICY_VIOLATION`
//! ではなく未登録へ縮退し、それ以外の gate(resolve error・
//! writeScope × command)は opt-in tool に対しても hard のままである
//! ことを固定する。pin-fail の opt-in 縮退は両 surface 対称の
//! uniform 規則(issue #165 R11-(e) — Command 側の現行無条件 hard
//! fail からの意図的拡張 + task executable pin 化による Process 側の
//! 新規規則)。非 opt-in worker の fail-closed 意味論は不変
//! (後方互換・test-spec)。

use std::collections::BTreeSet;
use std::path::PathBuf;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::exec::build_exec_input;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::registry::{create_tool_runtime, ToolRuntimeOptions};
use cheaper_tools::ToolError;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-degrade-it-{}-{}",
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

fn options(root: &str, names: Vec<ToolName>, degradable: Vec<ToolName>) -> ToolRuntimeOptions {
    ToolRuntimeOptions {
        resource_manifest: None,
        write_scope: None,
        read_strictness: None,
        artifacts: None,
        exec: None,
        security: None,
        root: root.to_string(),
        symlink_policy: SymlinkPolicy::Deny,
        names,
        degradable,
        max_tool_calls: 20,
        on_limit: Box::new(|| {}),
        journal: None,
        denylist: None,
        parent_lease: None,
        signal: None,
        abort_grace_ms: None,
        concurrency: None,
        eval_spawn: None,
        route_context: None,
        execution_correlation: None,
    }
}

/// `err` / cause chain から `CheaperError` を探す。
fn find_cheaper_error(err: &ToolError) -> Option<&CheaperError> {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err.as_ref());
    while let Some(e) = current {
        if let Some(cheaper) = e.downcast_ref::<CheaperError>() {
            return Some(cheaper);
        }
        current = e.source();
    }
    None
}

fn cheaper_err(err: &ToolError) -> &CheaperError {
    find_cheaper_error(err).unwrap_or_else(|| panic!("expected CheaperError, got {err}"))
}

fn code(err: &ToolError) -> Option<ErrorCode> {
    find_cheaper_error(err).map(|e| e.code())
}

fn definition_names(runtime: &cheaper_tools::registry::ToolRuntime) -> Vec<&str> {
    runtime
        .definitions()
        .iter()
        .map(|d| d.name.as_str())
        .collect()
}

fn degraded_names(runtime: &cheaper_tools::registry::ToolRuntime) -> Vec<&str> {
    runtime
        .degraded_names()
        .iter()
        .map(|n| n.as_str())
        .collect()
}

/// task 1 件の process authoring section — tasks 非空の gate 通過側。
fn process_with_task() -> Value {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    json!({"tasks": {"self": {"command": [exe], "inputs": [], "outputs": [], "env": {}}}})
}

/// grant 1 件の command authoring section — grants 非空の gate 通過側。
fn command_with_grant(executable: &str) -> Value {
    json!({"grants": {"self": {"executable": executable, "argv": [], "env": {}}}})
}

/// authoring 2 section → unified exec-doc(`options.exec` の raw 入力
/// shape)への synthesis seam(project task 名なし)。
fn exec_doc(process: Option<Value>, command: Option<Value>) -> Value {
    Value::Object(
        build_exec_input(process.as_ref(), command.as_ref(), &BTreeSet::new()).expect("synthesis"),
    )
}

// ---------------------------------------------------------------------------
// degrade path — opt-in gated tool の empty content-gate 縮退
// ---------------------------------------------------------------------------

/// opt-in した `process` / `command` は empty tasks/grants で
/// `POLICY_VIOLATION` ではなく未登録へ縮退する。`degraded_names()` は
/// gate 分岐の push 順ではなく `options.names` 宣言順で報告する。
#[tokio::test]
async fn opted_in_gated_tools_degrade_on_empty_content_gates() {
    let dir = TempDir::new();
    // 宣言順を gate 順(process → command)と逆にして、degraded の整列が
    // 宣言順であることを検証する。
    let runtime = create_tool_runtime(
        options(
            &dir.root(),
            vec![ToolName::Read, ToolName::Command, ToolName::Process],
            vec![ToolName::Process, ToolName::Command],
        ),
        None,
    )
    .expect("opted-in gated tools degrade instead of hard-failing");
    // 実登録集合は縮退分を除く有効 names のみ。
    assert_eq!(definition_names(&runtime), ["read"]);
    assert_eq!(degraded_names(&runtime), ["command", "process"]);

    // 縮退した tool は実行経路を持たない — 名前での call は
    // UNKNOWN_TOOL で安全に失敗する。
    let err = runtime
        .execute("command", &json!({"name": "self", "args": []}), None)
        .await
        .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::UnknownTool), "{err}");
    runtime.dispose().await;
}

/// mixed policy — `tasks` 非空 × `grants` 空では failed-gate の
/// `command` のみ縮退し、`process` は gate 通過込みで登録される
/// (縮退は per-tool、test-spec Rule)。
#[test]
fn only_failed_gate_tool_degrades_in_mixed_policy() {
    let dir = TempDir::new();
    let mut opts = options(
        &dir.root(),
        vec![ToolName::Read, ToolName::Process, ToolName::Command],
        vec![ToolName::Process, ToolName::Command],
    );
    // tasks 非空 × grants 空の unified doc — fixed grant のみ存在し
    // `!fixed` 0 件の Command gate が縮退対象になる。
    opts.exec = Some(exec_doc(
        Some(process_with_task()),
        Some(json!({"grants": {}})),
    ));
    let runtime = create_tool_runtime(opts, None).expect("process gate passes");
    assert_eq!(definition_names(&runtime), ["read", "process"]);
    assert_eq!(degraded_names(&runtime), ["command"]);
}

// ---------------------------------------------------------------------------
// hard のまま維持 — 非 opt-in / pin 失敗 / writeScope × command /
// resolve error / zero-effective-names guard
// ---------------------------------------------------------------------------

/// `degradable` に含まれない gated tool は従来通り `POLICY_VIOLATION`
/// で fail-closed(非 opt-in worker の意味論不変 — 後方互換)。
#[test]
fn non_opted_empty_gates_still_fail_closed() {
    let dir = TempDir::new();
    // `degradable` 空。
    let err = create_tool_runtime(
        options(
            &dir.root(),
            vec![ToolName::Read, ToolName::Process],
            Vec::new(),
        ),
        None,
    )
    .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(e.message().contains("nonempty authorized tasks"));

    let err = create_tool_runtime(
        options(
            &dir.root(),
            vec![ToolName::Read, ToolName::Command],
            Vec::new(),
        ),
        None,
    )
    .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(e.message().contains("nonempty authorized grants"));

    // `degradable` が他 tool のみ宣言していても縮退しない。
    let err = create_tool_runtime(
        options(
            &dir.root(),
            vec![ToolName::Read, ToolName::Command],
            vec![ToolName::Process],
        ),
        None,
    )
    .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::PolicyViolation));
}

/// pin 失敗は失敗 grant の surface に対応する tool が `degradable`
/// 宣言されていれば当該 tool のみ縮退する — 両 surface 対称の
/// uniform 規則(R11-(e) 確定の意図的拡張。Command 側は現行の無条件
/// hard fail からの差分、Process 側は task executable が本 issue で
/// 初めて pin 対象になった D9 由来の新規規則)。
#[tokio::test]
async fn pin_failure_degrades_opted_in_surface() {
    let dir = TempDir::new();
    // Command 側 — opt-in 宣言時は pin 不能で command のみ縮退する。
    let mut opts = options(
        &dir.root(),
        vec![ToolName::Read, ToolName::Command],
        vec![ToolName::Command],
    );
    opts.exec = Some(exec_doc(
        None,
        Some(command_with_grant("definitely-not-a-real-command-xyz")),
    ));
    let runtime = create_tool_runtime(opts, None).expect("command degrades on pin failure");
    assert_eq!(definition_names(&runtime), ["read"]);
    assert_eq!(degraded_names(&runtime), ["command"]);
    runtime.dispose().await;

    // Process 側 — task executable の pin 不能で process のみ縮退する
    // (fixed grant pin は本 issue が初 — 旧 lazy resolve 経路では
    // scope 構築を fail させなかった)。
    let mut opts = options(
        &dir.root(),
        vec![ToolName::Read, ToolName::Process],
        vec![ToolName::Process],
    );
    opts.exec = Some(exec_doc(
        Some(json!({"tasks": {"self": {
            "command": ["definitely-not-a-real-command-xyz"],
            "inputs": [], "outputs": [],
        }}})),
        None,
    ));
    let runtime = create_tool_runtime(opts, None).expect("process degrades on pin failure");
    assert_eq!(definition_names(&runtime), ["read"]);
    assert_eq!(degraded_names(&runtime), ["process"]);
    runtime.dispose().await;
}

/// 非 opt-in(`degradable` 未宣言)経路の pin fail は hard
/// `POLICY_VIOLATION` のまま不変 — 両 surface で同一文言。
#[test]
fn pin_failure_stays_hard_when_not_opted_in() {
    let dir = TempDir::new();
    let mut opts = options(
        &dir.root(),
        vec![ToolName::Read, ToolName::Command],
        Vec::new(),
    );
    opts.exec = Some(exec_doc(
        None,
        Some(command_with_grant("definitely-not-a-real-command-xyz")),
    ));
    let err = create_tool_runtime(opts, None).unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("could not be resolved to a spawnable executable"),
        "{}",
        e.message()
    );

    // fixed surface も同じ pin 失敗文言で fail-closed。
    let mut opts = options(
        &dir.root(),
        vec![ToolName::Read, ToolName::Process],
        Vec::new(),
    );
    opts.exec = Some(exec_doc(
        Some(json!({"tasks": {"self": {
            "command": ["definitely-not-a-real-command-xyz"],
            "inputs": [], "outputs": [],
        }}})),
        None,
    ));
    let err = create_tool_runtime(opts, None).unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("could not be resolved to a spawnable executable"),
        "{}",
        e.message()
    );
}

/// opt-in しても grants 非空時の writeScope × `command` 不併存は
/// hard `POLICY_VIOLATION` のまま(grants 空での縮退時のみ不発)。
#[test]
fn write_scope_rejects_command_even_when_opted_in() {
    let dir = TempDir::new();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut opts = options(
        &dir.root(),
        vec![ToolName::Read, ToolName::Command],
        vec![ToolName::Command],
    );
    opts.exec = Some(exec_doc(None, Some(command_with_grant(&exe))));
    opts.write_scope = Some(vec!["files".to_string()]);
    let err = create_tool_runtime(opts, None).unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(e.message().contains("writeScope"));
}

/// `resolve_*_policy` の error は opt-in tool に対しても hard のまま
/// 伝播する(縮退対象は empty tasks/grants の content-gate と
/// pin-fail 縮退のみ — resolve error は両者の外)。
#[test]
fn resolve_policy_errors_stay_hard_even_when_opted_in() {
    let dir = TempDir::new();
    // wildcard 構造違反の grant spec は policy resolve 段階で
    // `InvalidRuntimePolicy` — degradable 宣言で soft 化されない
    // (PATH 非依存で deterministic)。exec-doc を直接注入し
    // `resolve_exec_policy` の `ArgPattern::compile` で検査を
    // 発火させる(authoring `build_exec_input` 内 `resolve_command_
    // policy` でも同じ構造検査が発火する — code 同一)。
    let mut opts = options(
        &dir.root(),
        vec![ToolName::Read, ToolName::Command],
        vec![ToolName::Command],
    );
    opts.exec =
        Some(json!({"grants": {"self": {"executable": "node", "argv": ["a*b"], "env": {}}}}));
    let err = create_tool_runtime(opts, None).unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::InvalidRuntimePolicy), "{err}");
}

/// degradation が全 tool を縮退した scope は tool 0 個の runtime を
/// 生成せず `POLICY_VIOLATION` で fail-closed(直接 caller 向け guard —
/// worker 経路では validate の not-proper-subset が防ぐ)。
#[test]
fn zero_effective_names_guard_fails_closed() {
    let dir = TempDir::new();
    let err = create_tool_runtime(
        options(
            &dir.root(),
            vec![ToolName::Command],
            vec![ToolName::Command],
        ),
        None,
    )
    .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message().contains("no tools to register"),
        "{}",
        e.message()
    );
}

/// `degradable` の非 gated 名は `is_scope_gated_tool` clamp で除外
/// される(直接 caller 向け防御) — 非 gated tool は content-gate を
/// 持たず縮退対象にならない。
#[test]
fn non_gated_degradable_entries_are_clamped() {
    let dir = TempDir::new();
    let runtime = create_tool_runtime(
        options(&dir.root(), vec![ToolName::Read], vec![ToolName::Read]),
        None,
    )
    .expect("non-gated degradable entry is ignored");
    assert_eq!(definition_names(&runtime), ["read"]);
    assert!(runtime.degraded_names().is_empty());
}

/// 非 opt-in の gated tool を含まない通常 runtime は縮退なし —
/// `degraded_names()` は常に空。
#[test]
fn degraded_names_empty_without_degradation() {
    let dir = TempDir::new();
    let runtime = create_tool_runtime(options(&dir.root(), vec![ToolName::Read], Vec::new()), None)
        .expect("read-only runtime");
    assert_eq!(definition_names(&runtime), ["read"]);
    assert!(runtime.degraded_names().is_empty());
}
