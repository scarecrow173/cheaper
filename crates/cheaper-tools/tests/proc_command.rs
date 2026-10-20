//! `command` tool の integration test — policy-gated dynamic subprocess
//! execution(T5)の挙動固定。
//!
//! `proc_process.rs` と同じく **この test binary 自身の fixture child**
//! (`command_fixture_child`)を grant executable として re-exec し、
//! managed subprocess の全 lifecycle(成功・非零終了・timeout・abort・
//! bounded output・process tree cleanup)を検証する。fixture child は
//! `CHEAPER_PROC_CHILD_MODE`(必須)+ `CHEAPER_PROC_CHILD_ARG`(mode 依存
//! path)を grant `env` 経由で受け取る — `command` の spawn env は
//! `minimalEnv(grant.env)` のため親 env は継承されない。
//!
//! 対象(design §(a)-(g)・test-spec):
//! - registration gate: nonempty grants・writeScope 排他・pin fail-closed・
//!   dormant runtime
//! - declared/pin/spawn-time の 3 適用点 deny(shell/shim・interpreter/
//!   exec-wrapper/loader・workspace-resident)
//! - model input `{name,args}` のみ(schema で余剰 key `INVALID_INPUT`)
//! - argv pattern match・metachar 拒否・cwd 境界
//! - 独自 pool `"command"`(`command.maxConcurrent`)+ Exclusive claim の
//!   serialize

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::operation::{AbortController, AbortReason, OperationState};
use cheaper_tools::registry::{create_tool_runtime, ToolCallContext, ToolRuntimeOptions};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

const FIXTURE: &str = "command_fixture_child";
const MODE_ENV: &str = "CHEAPER_PROC_CHILD_MODE";
const ARG_ENV: &str = "CHEAPER_PROC_CHILD_ARG";
const SENTINEL: &str = "@@";

// ---------------------------------------------------------------------------
// Fixture child(re-exec of this test binary)
// ---------------------------------------------------------------------------

/// `command` grant が spawn する fixture。`MODE_ENV` 未設定なら即 return
/// (通常の `cargo test` では何もしない)。`process` fixture と違い、
/// `command` の model input は `{name,args}` のみで grant argv は
/// policy 側 pin のため、path 引数は argv ではなく `ARG_ENV` で渡す
/// (argv literal pin を維持するため)。
///
/// modes:
/// - `inspect`: `{SENTINEL}{cwd, args, mode}` を stdout へ。
/// - `output`: stdout へ `ARG_ENV` bytes、stderr へ "err"。
/// - `fail`: "before failure"/"failure detail" を出して exit 7。
/// - `hold`: `ARG_ENV`(gate file)が現れるまで待つ — pool cap test の
///   release gate。
/// - `probe`: `ARG_ENV` へ marker file を書く。
/// - `stubborn`: `ARG_ENV` へ pid を書いて park — timeout/abort kill 対象。
/// - `tree`: `child` fixture を spawn し `ARG_ENV` へ `{parent,child}` pid
///   JSON を書いて park — process tree cleanup 対象。
/// - `child`: "ready" を出力して park(tree の子)。
#[test]
fn command_fixture_child() {
    let Ok(mode) = std::env::var(MODE_ENV) else {
        return;
    };
    let arg = std::env::var(ARG_ENV).unwrap_or_default();
    match mode.as_str() {
        "inspect" => {
            let out = json!({
                "cwd": std::env::current_dir().unwrap().to_string_lossy(),
                "args": std::env::args().skip(3).collect::<Vec<String>>(),
                "mode": "inspect",
            });
            println!("{SENTINEL}{out}");
        }
        "output" => {
            let n: usize = arg.parse().unwrap_or(0);
            let mut stdout = std::io::stdout().lock();
            let mut stderr = std::io::stderr().lock();
            stdout.write_all(&vec![b'x'; n]).unwrap();
            stdout.flush().unwrap();
            stderr.write_all(b"err").unwrap();
            stderr.flush().unwrap();
        }
        "fail" => {
            println!("before failure");
            eprintln!("failure detail");
            std::process::exit(7);
        }
        "hold" => {
            let gate = PathBuf::from(&arg);
            let start = Instant::now();
            while !gate.exists() {
                assert!(
                    start.elapsed() < Duration::from_secs(20),
                    "hold gate never released"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
            println!("released");
        }
        "probe" => {
            std::fs::write(&arg, "ran").unwrap();
            println!("probed");
        }
        "stubborn" => {
            std::fs::write(&arg, std::process::id().to_string()).unwrap();
            park_forever();
        }
        "tree" => {
            let (child, child_out) = spawn_fixture("child");
            let mut reader = BufReader::new(child_out);
            wait_line(&mut reader, "ready");
            std::fs::write(
                &arg,
                json!({"parent": std::process::id(), "child": child.id()}).to_string(),
            )
            .unwrap();
            park_forever();
        }
        "child" => {
            println!("ready");
            std::io::stdout().flush().unwrap();
            park_forever();
        }
        other => panic!("unknown fixture mode: {other}"),
    }
}

fn park_forever() -> ! {
    loop {
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// harness noise を読み飛ばし、`marker` を含む最初の行を返す。
fn wait_line<R: BufRead>(reader: &mut R, marker: &str) -> String {
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).unwrap();
        assert!(n > 0, "fixture stdout closed before marker {marker:?}");
        if line.contains(marker) {
            return line;
        }
    }
}

/// fixture child からさらに fixture を spawn する(tree mode 用)。
/// `windowsHide` 相当の `CREATE_NO_WINDOW` 付き。
fn spawn_fixture(mode: &str) -> (std::process::Child, std::process::ChildStdout) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([FIXTURE, "--exact", "--nocapture"])
        .env(MODE_ENV, mode)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().unwrap();
    let stdout = child.stdout.take().unwrap();
    (child, stdout)
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-command-it-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }

    fn root(&self) -> String {
        canonicalize_workspace_root(self.0.to_str().unwrap()).unwrap()
    }

    fn join(&self, p: &str) -> PathBuf {
        self.0.join(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `ToolRuntimeOptions`(`create_tool_runtime` 直結の最小形)。
fn options(root: &str, command: Value, names: Vec<ToolName>) -> ToolRuntimeOptions {
    ToolRuntimeOptions {
        resource_manifest: None,
        write_scope: None,
        read_strictness: None,
        artifacts: None,
        process: None,
        command: Some(command),
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

/// この test binary を executable とする grant — `argv` は fixture test
/// 名 + `--exact --nocapture` の literal pin、`env` で fixture mode(+
/// mode 依存 path)を渡す。PATH 解決に依存せず、temp workspace 外のため
/// workspace-resident 拒否にも掛からない。
fn self_grant(mode: &str, arg: Option<&str>) -> Value {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut env = Map::new();
    env.insert(MODE_ENV.to_string(), json!(mode));
    if let Some(arg) = arg {
        env.insert(ARG_ENV.to_string(), json!(arg));
    }
    json!({
        "executable": exe,
        "argv": SELF_ARGV,
        "env": Value::Object(env),
    })
}

/// `self_grant` に lifecycle/cwd override を畳み込んだ形。
fn grant_with(mode: &str, arg: Option<&str>, extra: Value) -> Value {
    let mut grant = self_grant(mode, arg);
    for (k, v) in extra.as_object().expect("extra object") {
        grant.as_object_mut().unwrap().insert(k.clone(), v.clone());
    }
    grant
}

const SELF_ARGV: &[&str] = &[FIXTURE, "--exact", "--nocapture"];

fn self_args() -> Value {
    json!(SELF_ARGV)
}

fn command_policy(grants: Value) -> Value {
    json!({"maxConcurrent": 1, "grants": grants})
}

fn command_runtime(
    root: &str,
    command: Value,
) -> Result<cheaper_tools::registry::ToolRuntime, ToolError> {
    create_tool_runtime(options(root, command, vec![ToolName::Command]), None)
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

fn sentinel_json(stdout_text: &str) -> Value {
    let line = stdout_text
        .lines()
        .find(|l| l.starts_with(SENTINEL))
        .unwrap_or_else(|| panic!("no {SENTINEL} line in stdout: {stdout_text:?}"));
    serde_json::from_str(&line[SENTINEL.len()..]).unwrap()
}

async fn wait_for(mut cond: impl FnMut() -> bool, timeout: Duration, what: &str) {
    let start = Instant::now();
    while !cond() {
        if start.elapsed() > timeout {
            panic!("wait_for timed out: {what}");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// `alive(pid)`(`kill(pid, 0)` + /proc zombie 判定相当) — proc_process
/// と同一 helper。
fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        text.contains(&format!(" {pid} "))
    }
    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(pid as i32, 0) };
        if rc != 0 {
            return std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH);
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                if let Some(close) = stat.rfind(')') {
                    let state = stat[close + 2..].split(' ').next();
                    if state == Some("Z") || state == Some("X") {
                        return false;
                    }
                }
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// registration gate — nonempty grants / dormant runtime / writeScope
// ---------------------------------------------------------------------------

/// grants 空の `command` runtime は `POLICY_VIOLATION` で fail-closed、
/// `names` に `Command` を含まない runtime は dormant として許容される
/// (pin も試行されないため、pin 不能な grant があっても構築できる)。
#[test]
fn nonempty_grants_gate_and_dormant_runtime() {
    let dir = TempDir::new();
    let empty = command_policy(json!({}));
    let err = command_runtime(&dir.root(), empty.clone()).unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(e.message().contains("nonempty authorized grants"));

    // dormant — `Command` を含まない runtime は grants 空でも構築される。
    create_tool_runtime(options(&dir.root(), empty, vec![ToolName::Ls]), None)
        .expect("dormant runtime with empty grants");

    // dormant — pin 不能な grant(ambient PATH に存在しない executable)も
    // `Command` 非含有なら pin を試行せず構築される。
    let unpinnable = command_policy(json!({
        "ghost": {"executable": "definitely-not-a-real-command-xyz"}
    }));
    create_tool_runtime(
        options(&dir.root(), unpinnable.clone(), vec![ToolName::Ls]),
        None,
    )
    .expect("dormant runtime skips pin");
    let err = command_runtime(&dir.root(), unpinnable).unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::PolicyViolation));
}

/// `command` × `writeScope` は scope 生成時 `POLICY_VIOLATION`
/// (design §(c) — 子副作用を resource 層が観測不能のため併存不可)。
#[test]
fn write_scope_rejects_command() {
    let dir = TempDir::new();
    let mut opts = options(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
        vec![ToolName::Command],
    );
    opts.write_scope = Some(vec!["files".to_string()]);
    let err = create_tool_runtime(opts, None).unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(e.message().contains("writeScope"));
}

// ---------------------------------------------------------------------------
// deny — declared basename / pin resolved basename / workspace-resident
// ---------------------------------------------------------------------------

/// declared basename の deny(design §(d) 適用点 1) — interpreter・
/// exec wrapper/loader は policy resolve(`InvalidRuntimePolicy`)で
/// fail-fast し、shell/shim は baseline `is_shell_process_executable`
/// で拒否される。
#[test]
fn declared_denied_executables_fail_policy_resolution() {
    let dir = TempDir::new();
    for exe in [
        "python",
        "python3.11",
        "node",
        "perl",
        "ruby",
        "env",
        "xargs",
        "sudo",
        "docker",
        "ld-linux.so.2",
    ] {
        let err = command_runtime(
            &dir.root(),
            command_policy(json!({"g": {"executable": exe}})),
        )
        .unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{exe}");
        assert!(
            e.message().contains("interpreter or exec-capable"),
            "{exe}: {}",
            e.message()
        );
    }
    for exe in ["sh", "bash", "cmd.exe", "powershell"] {
        let err = command_runtime(
            &dir.root(),
            command_policy(json!({"g": {"executable": exe}})),
        )
        .unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{exe}");
        assert!(
            e.message().contains("shell or script shim"),
            "{exe}: {}",
            e.message()
        );
    }
}

/// pin 時 resolved basename の deny 再検査(design §(d) 適用点 2) —
/// declared name が deny 外でも、symlink 解決後の basename が
/// interpreter 名なら `POLICY_VIOLATION`。resolved basename ≠ declared
/// basename を作れるのは symlink のみ(Windows symlink は権限依存)のため
/// unix gate — deny check 自体の適用点は declared basename test で
/// 両 platform 共通に固定済み。
#[cfg(unix)]
#[test]
fn pinned_resolved_basename_deny_rejects() {
    let dir = TempDir::new();
    // workspace 外の bin dir(workspace-resident 拒否を避ける)。
    let bin = TempDir::new();
    let real = bin.join("python3");
    std::fs::copy(std::env::current_exe().unwrap(), &real).unwrap();
    let link = bin.join("tool");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let err = command_runtime(
        &dir.root(),
        command_policy(json!({
            "g": {"executable": link.to_string_lossy()}
        })),
    )
    .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("resolved executable is interpreter or exec-capable"),
        "{}",
        e.message()
    );
}

/// workspace-resident executable は二段で拒否 — (a) separator 付き
/// relative declared path(委譲前 pre-filter)、(b) realpath 後の
/// canonical path が workspace 内。両方 `POLICY_VIOLATION`。
#[test]
fn workspace_resident_executables_are_rejected() {
    let dir = TempDir::new();
    let resident = dir.join(if cfg!(windows) { "tool.exe" } else { "tool" });
    std::fs::copy(std::env::current_exe().unwrap(), &resident).unwrap();

    // (a) relative declared path。
    let err = command_runtime(
        &dir.root(),
        command_policy(json!({
            "g": {"executable": "./tool"}
        })),
    )
    .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("workspace-resident executables are not grantable"),
        "{}",
        e.message()
    );

    // (b) absolute declared path — realpath 後の inside-workspace 検査。
    let err = command_runtime(
        &dir.root(),
        command_policy(json!({
            "g": {"executable": resident.to_string_lossy()}
        })),
    )
    .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("workspace-resident executables are not grantable"),
        "{}",
        e.message()
    );
}

/// ambient PATH に存在しない executable は pin 不能 → `POLICY_VIOLATION`
/// (fail closed)。
#[test]
fn unpinnable_executable_fails_closed() {
    let dir = TempDir::new();
    let err = command_runtime(
        &dir.root(),
        command_policy(json!({
            "ghost": {"executable": "definitely-not-a-real-command-xyz"}
        })),
    )
    .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("requires a supported native executable"),
        "{}",
        e.message()
    );
}

// ---------------------------------------------------------------------------
// model input — `{name, args}` のみ(schema 層 `INVALID_INPUT`)
// ---------------------------------------------------------------------------

/// `parameters` schema は `{name, args}` のみを公開する — `executable`・
/// `cwd`・`env` は model input 経路に存在しない(test-spec Rule)。
#[test]
fn command_schema_exposes_only_name_and_args() {
    let dir = TempDir::new();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
    )
    .unwrap();
    let def = rt
        .definitions()
        .iter()
        .find(|d| d.name == ToolName::Command)
        .expect("command definition");
    assert_eq!(def.parameters["required"], json!(["name", "args"]));
    assert_eq!(def.parameters["additionalProperties"], json!(false));
    let props = def.parameters["properties"].as_object().unwrap();
    assert!(props.contains_key("name") && props.contains_key("args"));
    for key in ["executable", "cwd", "env"] {
        assert!(!props.contains_key(key), "schema leaks {key}");
    }
}

/// `{name,args}` 以外の key(`executable`/`cwd`/`env` — model が grant
/// override を密輸しようとする形)は schema 層で `INVALID_INPUT`。
#[tokio::test]
async fn extra_model_input_keys_are_invalid_input() {
    let dir = TempDir::new();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
    )
    .unwrap();
    for key in ["executable", "cwd", "env"] {
        let mut input = Map::new();
        input.insert("name".to_string(), json!("self"));
        input.insert("args".to_string(), self_args());
        input.insert(key.to_string(), json!("smuggled"));
        let err = rt
            .execute("command", &Value::Object(input), None)
            .await
            .unwrap_err();
        assert_eq!(
            code(&err),
            Some(ErrorCode::InvalidInput),
            "extra key {key}: {err}"
        );
    }
    // 非 string name / 非 array args / 超過 args も schema 層で拒否。
    for input in [
        json!({"name": 1, "args": []}),
        json!({"name": "self", "args": "x"}),
        json!({"name": "self", "args": [1]}),
        json!({"name": "self", "args": vec!["x"; 257]}),
    ] {
        let err = rt.execute("command", &input, None).await.unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::InvalidInput), "input {input}");
    }
}

/// `name` の policy pattern(`^[a-z][a-z0-9_-]{0,63}$`)違反は unknown
/// grant 判定に到達する前の schema 層 `INVALID_INPUT`。
#[tokio::test]
async fn invalid_grant_name_pattern_is_invalid_input() {
    let dir = TempDir::new();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
    )
    .unwrap();
    for name in ["Self", "-bad", "bad name", "a;b"] {
        let err = rt
            .execute("command", &json!({"name": name, "args": []}), None)
            .await
            .unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::InvalidInput), "name {name}");
    }
}

// ---------------------------------------------------------------------------
// authorization — unknown grant / argv pattern / metachar
// ---------------------------------------------------------------------------

/// 未知 grant は spawn 前に `POLICY_VIOLATION "unknown or unauthorized
/// command grant"`(details `{grant}`)。
#[tokio::test]
async fn unknown_grant_is_policy_violation() {
    let dir = TempDir::new();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
    )
    .unwrap();
    let err = rt
        .execute("command", &json!({"name": "ghost", "args": []}), None)
        .await
        .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(e
        .message()
        .contains("unknown or unauthorized command grant"));
    assert_eq!(e.details().unwrap()["grant"], json!("ghost"));
}

/// argv pattern 不一致は `POLICY_VIOLATION`(schema は通るが認可
/// mismatch)— literal pin は個数・値の完全一致を要求する。
#[tokio::test]
async fn argv_pattern_mismatch_is_policy_violation() {
    let dir = TempDir::new();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
    )
    .unwrap();
    for args in [
        json!([FIXTURE]),
        json!([FIXTURE, "--exact", "--nocapture", "--other"]),
        json!([FIXTURE, "--list", "--nocapture"]),
        json!([]),
    ] {
        let err = rt
            .execute("command", &json!({"name": "self", "args": args}), None)
            .await
            .unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(e.code(), ErrorCode::PolicyViolation, "args {args}");
        assert!(
            e.message()
                .contains("arguments do not match the grant argv pattern"),
            "{}",
            e.message()
        );
        assert_eq!(e.details().unwrap()["grant"], json!("self"));
    }
}

/// metachar(`|&;<>`・`` ` ``・`$(`・NUL/改行)は pattern match より先の
/// 独立 check で全 token に適用される — rest marker `*` が適合しても
/// `POLICY_VIOLATION`。
#[tokio::test]
async fn shell_metacharacters_are_rejected() {
    let dir = TempDir::new();
    let mut grant = self_grant("inspect", None);
    // rest marker — metachar 無しなら任意個数の args を受理する形。
    grant
        .as_object_mut()
        .unwrap()
        .insert("argv".to_string(), json!([FIXTURE, "*"]));
    let rt = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap();
    for bad in [
        "a|b", "a;b", "a>b", "a<b", "a&b", "$(id)", "`id`", "a\ntail",
    ] {
        let err = rt
            .execute(
                "command",
                &json!({"name": "self", "args": [FIXTURE, bad]}),
                None,
            )
            .await
            .unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(e.code(), ErrorCode::PolicyViolation, "arg {bad:?}");
        assert!(
            e.message().contains("forbidden shell metacharacters"),
            "{bad:?}: {}",
            e.message()
        );
    }
}

/// prefix(`--ex*`)と trailing rest(`*`)の argv pattern は適合 args で
/// 実行が通る — literal pin 以外の pattern 形の実動作確認。
#[tokio::test]
async fn argv_prefix_and_rest_patterns_execute() {
    let dir = TempDir::new();
    let mut prefix = self_grant("inspect", None);
    prefix
        .as_object_mut()
        .unwrap()
        .insert("argv".to_string(), json!([FIXTURE, "--ex*", "--nocapture"]));
    let mut rest = self_grant("inspect", None);
    rest.as_object_mut()
        .unwrap()
        .insert("argv".to_string(), json!([FIXTURE, "*"]));
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"prefix": prefix, "rest": rest})),
    )
    .unwrap();
    for name in ["prefix", "rest"] {
        let result = rt
            .execute("command", &json!({"name": name, "args": self_args()}), None)
            .await
            .unwrap();
        let details = result.details.as_ref().expect("details");
        assert_eq!(details["state"], json!("completed"), "{name}");
        assert_eq!(details["exitCode"], json!(0), "{name}");
    }
}

// ---------------------------------------------------------------------------
// execution — details / env / cwd
// ---------------------------------------------------------------------------

/// happy path — grant 実行が `{grant, argv, state, exitCode, signal,
/// durationMs, stdout, stderr}` を報告し、fixture child が workspace
/// root で実行されたことを `{SENTINEL}` payload で確認する。
#[tokio::test]
async fn grant_executes_and_reports_details() {
    let dir = TempDir::new();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
    )
    .unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    let details = result.details.as_ref().expect("details");
    assert_eq!(details["grant"], json!("self"));
    assert_eq!(details["argv"], self_args());
    assert_eq!(details["state"], json!("completed"));
    assert_eq!(details["exitCode"], json!(0));
    assert_eq!(details["signal"], Value::Null);
    // `durationMs` は `command` details schema の spec-mandated 差分
    // (`process` details には無い)。
    assert!(details["durationMs"].is_number(), "details: {details:?}");
    assert_eq!(details["stderr"]["bytes"], json!(0));
    let report = sentinel_json(details["stdout"]["text"].as_str().unwrap());
    // grant env が minimal env 経由で子へ届く(MODE_ENV が無いと fixture
    // は何も出力しない — mode が入っている時点で env 到達が証明済み)。
    assert_eq!(report["mode"], json!("inspect"));
    // grant argv literal pin の tail(`--nocapture` — exe/name/`--exact`
    // を除いた残余)がそのまま子 argv として届く。
    assert_eq!(report["args"], json!(["--nocapture"]));
    assert_eq!(report["cwd"], json!(dir.root()));
    // 表示 fold text — `command {grant}` の display prefix。
    assert!(result.content[0]
        .text
        .starts_with("command self: completed, exitCode=0\nstdout:"));
}

/// grant `exitCodes` 宣言内の非 0 exit は error ではなく completed
/// result として返り、`state`/`exitCode`/表示 text が実 exit code を
/// 報告する(design A1)。
#[tokio::test]
async fn declared_exitcodes_report_completed_result() {
    let dir = TempDir::new();
    let grant = grant_with("fail", None, json!({"exitCodes": [0, 7]}));
    let rt = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    let details = result.details.as_ref().expect("details");
    assert_eq!(details["state"], json!("completed"));
    assert_eq!(details["exitCode"], json!(7));
    assert!(details["stdout"]["text"]
        .as_str()
        .unwrap()
        .contains("before failure"));
    assert!(result.content[0]
        .text
        .starts_with("command self: completed, exitCode=7\nstdout:"));
}

/// `exitCodes` 未宣言(default `[0]`)の非 0 exit は従来通り
/// `TOOL_EXECUTION_FAILURE` error — 宣言 exitCodes に含まれない
/// code も同じ。
#[tokio::test]
async fn undeclared_nonzero_exit_remains_failure() {
    let dir = TempDir::new();
    for grant in [
        self_grant("fail", None),
        grant_with("fail", None, json!({"exitCodes": [0, 3]})),
    ] {
        let rt = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap();
        let err = rt
            .execute(
                "command",
                &json!({"name": "self", "args": self_args()}),
                None,
            )
            .await
            .unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(e.code(), ErrorCode::ToolExecutionFailure);
        assert_eq!(e.details().unwrap()["exitCode"], json!(7));
        assert_eq!(e.details().unwrap()["state"], json!("failed"));
    }
}

/// `exitCodes` の非 integer 要素・非 array は policy resolve で
/// `INVALID_RUNTIME_POLICY`。
#[test]
fn invalid_exitcodes_rejected_at_policy_resolution() {
    let dir = TempDir::new();
    for exit_codes in [json!("0"), json!([0, "1"]), json!([0.5]), json!([-1])] {
        let grant = grant_with("inspect", None, json!({"exitCodes": exit_codes}));
        let err = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(
            e.code(),
            ErrorCode::InvalidRuntimePolicy,
            "exitCodes {exit_codes}"
        );
    }
}

/// grant `cwd` は workspace 内へ解決される — `cwd: "sub"` の grant は
/// `root/sub` で実行される。
#[tokio::test]
async fn grant_cwd_runs_inside_workspace_subdir() {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let grant = grant_with("inspect", None, json!({"cwd": "sub"}));
    let rt = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    let report = sentinel_json(
        result.details.as_ref().unwrap()["stdout"]["text"]
            .as_str()
            .unwrap(),
    );
    let expected = canonicalize_workspace_root(dir.join("sub").to_str().unwrap()).unwrap();
    assert_eq!(report["cwd"], json!(expected));
}

/// grant `cwd` の workspace escape(`..`・absolute)は policy resolve で
/// `INVALID_RUNTIME_POLICY` — execute に到達しない。
#[test]
fn cwd_escape_rejected_at_policy_resolution() {
    let dir = TempDir::new();
    for cwd in ["..", "../outside", "/tmp", "C:\\Windows"] {
        let grant = grant_with("inspect", None, json!({"cwd": cwd}));
        let err = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "cwd {cwd}");
    }
}

// ---------------------------------------------------------------------------
// lifecycle — nonzero / timeout / abort / output cap / tree cleanup
// ---------------------------------------------------------------------------

/// 非零終了は `TOOL_EXECUTION_FAILURE "managed process failed"` + details
/// `{grant, argv, state: "failed", exitCode, signal, durationMs, stdout,
/// stderr}`(process tool の shared lifecycle と同じ shape)。
#[tokio::test]
async fn nonzero_exit_reports_failed_details() {
    let dir = TempDir::new();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("fail", None)})),
    )
    .unwrap();
    let err = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::ToolExecutionFailure));
    let details = cheaper_err(&err).details().unwrap();
    assert_eq!(details["grant"], json!("self"));
    assert_eq!(details["argv"], self_args());
    assert_eq!(details["state"], json!("failed"));
    assert_eq!(details["exitCode"], json!(7));
    assert!(details["durationMs"].is_number());
    assert!(details["stdout"]["text"]
        .as_str()
        .unwrap()
        .contains("before failure"));
    assert!(details["stderr"]["text"]
        .as_str()
        .unwrap()
        .contains("failure detail"));
}

/// per-grant `timeout` 超過は kill → `TIMEOUT`(details に grant・
/// durationMs)+ 子 process の実 kill を pid marker で検証する。
#[tokio::test]
async fn timeout_kills_and_reports() {
    let dir = TempDir::new();
    let pid_path = dir.join("pid").to_string_lossy().into_owned();
    let grant = grant_with(
        "stubborn",
        Some(&pid_path),
        json!({"timeout": "500ms", "killGrace": "200ms"}),
    );
    let rt = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap();
    let err = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::Timeout));
    let e = cheaper_err(&err);
    assert!(e.message().contains("timed out"), "{}", e.message());
    assert_eq!(e.details().unwrap()["grant"], json!("self"));
    assert!(e.details().unwrap()["durationMs"].is_number());
    let pid: u32 = std::fs::read_to_string(&pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    wait_for(|| !alive(pid), Duration::from_secs(20), "pid gone").await;
    assert!(!alive(pid));
}

/// call-scope signal abort は実行中の `command` を kill する — reason
/// message がそのまま伝播し、non-cooperative fixture は外部 kill で
/// settle される。
#[tokio::test]
async fn abort_kills_and_preserves_reason() {
    let dir = TempDir::new();
    let pid_path = dir.join("pid").to_string_lossy().into_owned();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("stubborn", Some(&pid_path))})),
    )
    .unwrap();
    let call = AbortController::new();
    let pending = tokio::spawn({
        let rt = rt.clone();
        let signal = call.signal();
        async move {
            rt.execute(
                "command",
                &json!({"name": "self", "args": self_args()}),
                Some(ToolCallContext {
                    signal: Some(signal),
                }),
            )
            .await
        }
    });
    wait_for(
        || Path::new(&pid_path).exists(),
        Duration::from_secs(15),
        "pid",
    )
    .await;
    call.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Aborted,
        "stop managed command",
    )));
    let err = pending.await.unwrap().unwrap_err();
    assert_eq!(err.to_string(), "stop managed command");
    let pid: u32 = std::fs::read_to_string(&pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    wait_for(|| !alive(pid), Duration::from_secs(20), "pid gone").await;
    assert!(!alive(pid));
}

/// grant `maxOutputBytes` の combined capture cap — `bytes` は実長、
/// `capturedBytes` は cap 合計、`truncated` flag が立つ。
#[tokio::test]
async fn output_cap_truncates_streams() {
    let dir = TempDir::new();
    let grant = grant_with("output", Some("5000"), json!({"maxOutputBytes": 256}));
    let rt = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    let details = result.details.as_ref().expect("details");
    let stdout = &details["stdout"];
    let stderr = &details["stderr"];
    // `bytes` は実長(fixture 5000B + libtest harness noise)。
    assert!(stdout["bytes"].as_u64().unwrap() >= 5000);
    assert!(stderr["bytes"].as_u64().unwrap() >= 3);
    // combined cap — 両 stream の capturedBytes 合計が cap に収まる。
    let captured =
        stdout["capturedBytes"].as_u64().unwrap() + stderr["capturedBytes"].as_u64().unwrap();
    assert_eq!(captured, 256);
    assert!(stdout["truncated"] == json!(true) || stderr["truncated"] == json!(true));
}

/// `command` は shared process-tree cleanup を使う — abort で direct 子
/// まで含めて kill されることを pid marker で検証する。
#[tokio::test]
async fn abort_kills_managed_parent_and_descendant() {
    let dir = TempDir::new();
    let pids_path = dir.join("pids.json").to_string_lossy().into_owned();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({
            "self": grant_with("tree", Some(&pids_path), json!({"killGrace": "200ms"}))
        })),
    )
    .unwrap();
    let call = AbortController::new();
    let pending = tokio::spawn({
        let rt = rt.clone();
        let signal = call.signal();
        async move {
            rt.execute(
                "command",
                &json!({"name": "self", "args": self_args()}),
                Some(ToolCallContext {
                    signal: Some(signal),
                }),
            )
            .await
        }
    });
    wait_for(
        || Path::new(&pids_path).exists(),
        Duration::from_secs(15),
        "pids.json",
    )
    .await;
    let pids: Value = serde_json::from_str(&std::fs::read_to_string(&pids_path).unwrap()).unwrap();
    call.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Aborted,
        "stop managed tree",
    )));
    let err = pending.await.unwrap().unwrap_err();
    assert_eq!(err.to_string(), "stop managed tree");
    for key in ["parent", "child"] {
        let pid = pids[key].as_u64().unwrap() as u32;
        wait_for(|| !alive(pid), Duration::from_secs(20), "{key} gone").await;
        assert!(!alive(pid), "{key} should be dead");
    }
}

// ---------------------------------------------------------------------------
// pool cap / exclusive claim — 独自 pool `"command"` + Exclusive claim
// ---------------------------------------------------------------------------

/// `command.maxConcurrent: 1` の独立 pool + Exclusive root claim は
/// 2 個目の `command` op を sibling settle まで `Queued` に留める。
/// `hold` gate file で deterministic に検証する(probe marker は hold
/// 実行中には現れない)。
#[tokio::test]
async fn command_pool_serializes_concurrent_grants() {
    let dir = TempDir::new();
    let gate = dir.join("gate").to_string_lossy().into_owned();
    let marker = dir.join("probe_ran").to_string_lossy().into_owned();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({
            "hold": self_grant("hold", Some(&gate)),
            "probe": self_grant("probe", Some(&marker)),
        })),
    )
    .unwrap();
    let hold = tokio::spawn({
        let rt = rt.clone();
        async move {
            rt.execute(
                "command",
                &json!({"name": "hold", "args": self_args()}),
                None,
            )
            .await
        }
    });
    wait_for(
        || {
            rt.operations()
                .iter()
                .any(|op| op.state == OperationState::Running)
        },
        Duration::from_secs(15),
        "hold op running",
    )
    .await;
    let probe = tokio::spawn({
        let rt = rt.clone();
        async move {
            rt.execute(
                "command",
                &json!({"name": "probe", "args": self_args()}),
                None,
            )
            .await
        }
    });
    // pool cap — probe は Queued のまま(marker はまだ書かれない)。
    wait_for(
        || {
            rt.operations()
                .iter()
                .any(|op| op.state == OperationState::Queued)
        },
        Duration::from_secs(15),
        "probe op queued behind pool cap",
    )
    .await;
    assert!(
        !Path::new(&marker).exists(),
        "probe ran while hold was still running"
    );
    std::fs::write(&gate, "1").unwrap();
    let hold_result = hold.await.unwrap().unwrap();
    let probe_result = probe.await.unwrap().unwrap();
    assert_eq!(
        hold_result.details.as_ref().unwrap()["state"],
        json!("completed")
    );
    assert_eq!(
        probe_result.details.as_ref().unwrap()["state"],
        json!("completed")
    );
    assert!(Path::new(&marker).exists());
}

/// `command` の Exclusive claim(workspace root subtree reads+writes)は
/// 同 scope の mutating sibling(`write`)を settle まで `Queued` に
/// 留める — `command_operation_declaration` の保守的 manifest が
/// scheduler 経由で効くことの挙動固定(design §(f))。
#[tokio::test]
async fn command_exclusive_claim_blocks_mutating_siblings() {
    let dir = TempDir::new();
    let gate = dir.join("gate").to_string_lossy().into_owned();
    let mut opts = options(
        &dir.root(),
        command_policy(json!({"hold": self_grant("hold", Some(&gate))})),
        vec![ToolName::Command, ToolName::Write],
    );
    opts.max_tool_calls = 20;
    let rt = create_tool_runtime(opts, None).unwrap();
    let hold = tokio::spawn({
        let rt = rt.clone();
        async move {
            rt.execute(
                "command",
                &json!({"name": "hold", "args": self_args()}),
                None,
            )
            .await
        }
    });
    wait_for(
        || {
            rt.operations()
                .iter()
                .any(|op| op.state == OperationState::Running)
        },
        Duration::from_secs(15),
        "command op running",
    )
    .await;
    let write = tokio::spawn({
        let rt = rt.clone();
        async move {
            rt.execute(
                "write",
                &json!({"path": "sibling.txt", "content": "x"}),
                None,
            )
            .await
        }
    });
    wait_for(
        || {
            rt.operations()
                .iter()
                .any(|op| op.state == OperationState::Queued)
        },
        Duration::from_secs(15),
        "write op queued behind command claim",
    )
    .await;
    assert!(
        !dir.join("sibling.txt").exists(),
        "write ran while command held the exclusive claim"
    );
    std::fs::write(&gate, "1").unwrap();
    hold.await.unwrap().unwrap();
    write.await.unwrap().unwrap();
    assert!(dir.join("sibling.txt").exists());
}

// ---------------------------------------------------------------------------
// spawn-time re-verification(design §(b) 適用点 3)
// ---------------------------------------------------------------------------

/// pin 後に executable が差し替えられた場合、spawn 前再検証が
/// `POLICY_VIOLATION` で fail-closed する(内容差替 → `native()` 再検査
/// 不成立)。
#[tokio::test]
async fn spawn_time_revalidation_detects_swapped_executable() {
    let dir = TempDir::new();
    let bin = TempDir::new(); // workspace 外
    let exe = bin.join(if cfg!(windows) { "tool.exe" } else { "tool" });
    std::fs::copy(std::env::current_exe().unwrap(), &exe).unwrap();
    let rt = command_runtime(
        &dir.root(),
        command_policy(json!({
            "self": {
                "executable": exe.to_string_lossy(),
                "argv": SELF_ARGV,
            }
        })),
    )
    .unwrap();
    // pin 後に pinned path の内容を非 native へ差し替える。
    std::fs::write(&exe, b"#!/bin/sh\necho swapped\n").unwrap();
    let err = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("no longer resolves to a supported native executable"),
        "{}",
        e.message()
    );
}
