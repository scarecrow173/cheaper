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
//! - declared/pin/spawn-time の各適用点 — declared executable 名の
//!   policy resolve 受理、pin の PATH 解決・spawn viability、pin 済み
//!   path の spawn 前再検証(symlink 差替・削除の fail-closed)
//! - model input `{name,args}` のみ(schema で余剰 key `INVALID_INPUT`)
//! - argv pattern match(metachar は literal argv として受理)・cwd 境界
//! - 独自 pool `"command"`(`command.maxConcurrent`)+ Exclusive claim の
//!   serialize

use std::collections::BTreeSet;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::exec::build_exec_input;
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::operation::{AbortController, AbortReason, OperationState};
use cheaper_tools::registry::{create_tool_runtime, ToolCallContext, ToolRuntimeOptions};
use cheaper_tools::tools::exec::RouteCommandContext;
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

/// authoring `command` section(`{"grants":…}` + section-level)→
/// unified exec-doc(`options.exec` の raw 入力 shape)への synthesis
/// seam(project task 名なし)。
fn exec_doc(command: Value) -> Value {
    Value::Object(build_exec_input(None, Some(&command), &BTreeSet::new()).expect("synthesis"))
}

/// `ToolRuntimeOptions`(`create_tool_runtime` 直結の最小形)。
/// `command` は authoring `command` section — unified exec-doc へ畳んで
/// `options.exec` に設定する。
fn options(root: &str, command: Value, names: Vec<ToolName>) -> ToolRuntimeOptions {
    ToolRuntimeOptions {
        resource_manifest: None,
        write_scope: None,
        read_strictness: None,
        artifacts: None,
        exec: Some(exec_doc(command)),
        security: None,
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
        route_context: None,
    }
}

/// この test binary を executable とする grant — `argv` は fixture test
/// 名 + `--exact --nocapture` の literal pin、`env` で fixture mode(+
/// mode 依存 path)を渡す。absolute path 宣言のため pin は ambient PATH
/// 解決に依存せず deterministic。
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

/// unified exec-doc(`{"grants":{…}}`)を `options.exec` へ直接注入する
/// seam — 無効値を authoring resolver(`build_exec_input` 内)で先に
/// reject させず、`resolve_exec_policy`(runtime 構築境界)の
/// validation へ到達させる。negative validation 系テスト用。
fn exec_runtime(
    root: &str,
    grants: Value,
) -> Result<cheaper_tools::registry::ToolRuntime, ToolError> {
    let mut o = options(root, json!({}), vec![ToolName::Command]);
    o.exec = Some(json!({"grants": grants}));
    create_tool_runtime(o, None)
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

#[cfg(windows)]
mod windows_process_handle {
    use std::ffi::c_void;
    use std::io;

    type Handle = *mut c_void;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_TERMINATE: u32 = 0x0001;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;
    const WAIT_FAILED: u32 = u32::MAX;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn TerminateProcess(handle: Handle, exit_code: u32) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    pub struct ProcessHandle(Handle);

    impl ProcessHandle {
        pub fn open(pid: u32) -> io::Result<Self> {
            // SAFETY: OpenProcess receives a PID from the fixture, requests only
            // synchronization/termination rights, and returns an owned handle.
            let handle = unsafe { OpenProcess(SYNCHRONIZE | PROCESS_TERMINATE, 0, pid) };
            if handle.is_null() {
                // SAFETY: GetLastError is thread-local and called immediately
                // after the failed Win32 API call.
                return Err(io::Error::from_raw_os_error(
                    unsafe { GetLastError() } as i32
                ));
            }
            Ok(Self(handle))
        }

        pub fn wait(&self, milliseconds: u32) -> u32 {
            // SAFETY: this guard owns a valid process handle until Drop.
            unsafe { WaitForSingleObject(self.0, milliseconds) }
        }

        pub fn terminate(&self, exit_code: u32) -> Result<(), u32> {
            // SAFETY: this guard owns a valid process handle with PROCESS_TERMINATE.
            if unsafe { TerminateProcess(self.0, exit_code) } != 0 {
                Ok(())
            } else {
                // SAFETY: GetLastError is thread-local and called immediately
                // after the failed Win32 API call.
                Err(unsafe { GetLastError() })
            }
        }

        pub fn last_error() -> u32 {
            // SAFETY: used immediately after a failed WaitForSingleObject call.
            unsafe { GetLastError() }
        }

        pub const WAIT_OBJECT_0: u32 = WAIT_OBJECT_0;
        pub const WAIT_TIMEOUT: u32 = WAIT_TIMEOUT;
        pub const WAIT_FAILED: u32 = WAIT_FAILED;
    }

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            // SAFETY: this guard uniquely owns the handle and closes it once.
            let _ = unsafe { CloseHandle(self.0) };
        }
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
// executable 受理 — declared 名・pin resolved basename・workspace 内 file
// ---------------------------------------------------------------------------

/// declared executable 名は内容審査されず policy resolve を通る
/// (design §(d) — interpreter・exec wrapper/loader・shell 系名を
/// 含む任意名を受理)。`resolve_exec_policy` は PATH 非依存で
/// deterministic(exec-doc `{"grants":…}` を直接受理 — caller は必ず
/// `build_exec_input` 経由のためここでも段 2 を通す)。
/// pin 層の受理は workspace 内 file の case で別途 pin。
#[test]
fn declared_executable_names_pass_policy_resolution() {
    use cheaper_core::policy::exec::resolve_exec_policy;

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
        "ssh",
        "tar",
        "sh",
        "bash",
        "cmd.exe",
        "powershell",
        "deploy.sh",
    ] {
        let doc = exec_doc(json!({
            "grants": {"g": {"executable": exe}}
        }));
        resolve_exec_policy(Some(&doc), &ExecDenyList::default())
            .unwrap_or_else(|e| panic!("{exe} should be accepted: {e}"));
    }
}

/// pin 時 resolved basename は審査されない — symlink 解決後の
/// basename が任意名でも pin される。resolved basename ≠ declared
/// basename を作れるのは symlink のみ(Windows symlink は権限依存)
/// のため unix gate。
#[cfg(unix)]
#[test]
fn pin_accepts_symlink_resolved_basename() {
    let dir = TempDir::new();
    // workspace 外の bin dir。
    let bin = TempDir::new();
    // resolved basename を `python3` とする file(内容は current_exe
    // 複製 — 内容・basename は判定対象外)。
    let real = bin.join("python3");
    std::fs::copy(std::env::current_exe().unwrap(), &real).unwrap();
    let link = bin.join("tool");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    command_runtime(
        &dir.root(),
        command_policy(json!({
            "g": {"executable": link.to_string_lossy()}
        })),
    )
    .expect("symlink-resolved executable pins successfully");
}

/// workspace 内の executable は relative・absolute どちらの
/// declared path でも pin・runtime 構築を通る(workspace 境界の
/// escape 検査のみが残る)。
#[test]
fn workspace_executables_are_pinned() {
    let dir = TempDir::new();
    let resident = dir.join(if cfg!(windows) { "tool.exe" } else { "tool" });
    std::fs::copy(std::env::current_exe().unwrap(), &resident).unwrap();

    // (a) relative declared path — workspace 内へ解決され pin される。
    let declared_rel = if cfg!(windows) {
        ".\\tool.exe"
    } else {
        "./tool"
    };
    command_runtime(
        &dir.root(),
        command_policy(json!({
            "g": {"executable": declared_rel}
        })),
    )
    .expect("workspace-relative executable pins");

    // (b) absolute declared path — workspace 内 file も pin される。
    command_runtime(
        &dir.root(),
        command_policy(json!({
            "g": {"executable": resident.to_string_lossy()}
        })),
    )
    .expect("absolute workspace executable pins");
}

/// ambient PATH に存在しない executable は pin 不能 → `POLICY_VIOLATION`
/// (fail closed)。`\0` を含む declared executable も同じ — realpath
/// 失敗で pin 層のまま縮退せず失敗する(spawn 層へは到達しない)。
#[test]
fn unpinnable_executable_fails_closed() {
    let dir = TempDir::new();
    for exe in ["definitely-not-a-real-command-xyz", "a\0b"] {
        let err = command_runtime(
            &dir.root(),
            command_policy(json!({
                "ghost": {"executable": exe}
            })),
        )
        .unwrap_err();
        let e = cheaper_err(&err);
        assert_eq!(e.code(), ErrorCode::PolicyViolation, "{exe:?}");
        assert!(
            e.message()
                .contains("could not be resolved to a spawnable executable"),
            "{exe:?}: {}",
            e.message()
        );
    }
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
// authorization — unknown grant / argv pattern / argv passthrough
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

/// argv token の内容は審査されない — (a) metachar(`|&;<>`・`` ` ``・
/// `$(`・改行)は pattern 適合なら literal argv として子 process へ
/// 届く(fixture `inspect` の sentinel args で観測)。(b) `\0` を含む
/// token は shell-free spawn が引き受けず `TOOL_EXECUTION_FAILURE` +
/// `spawnCode="SPAWN_ERROR"` へ縮退する(観測点 pin)。
#[tokio::test]
async fn metachar_args_pass_through_and_nul_degrades_to_spawn_error() {
    let dir = TempDir::new();
    let mut grant = self_grant("inspect", None);
    // rest marker — 任意個数の args を受理する形。
    grant
        .as_object_mut()
        .unwrap()
        .insert("argv".to_string(), json!([FIXTURE, "*"]));
    let rt = command_runtime(&dir.root(), command_policy(json!({"self": grant}))).unwrap();

    // (a) metachar token は literal のまま子 argv へ届く。
    let metachars = [
        "a|b", "a;b", "a>b", "a<b", "a&b", "$(id)", "`id`", "a\ntail",
    ];
    let mut args = vec![json!(FIXTURE), json!("--exact"), json!("--nocapture")];
    args.extend(metachars.iter().map(|t| json!(t)));
    let result = rt
        .execute("command", &json!({"name": "self", "args": args}), None)
        .await
        .unwrap();
    let report = sentinel_json(
        result.details.as_ref().unwrap()["stdout"]["text"]
            .as_str()
            .unwrap(),
    );
    let mut expected = vec![json!("--nocapture")];
    expected.extend(metachars.iter().map(|t| json!(t)));
    assert_eq!(report["args"], json!(expected));

    // (b) `\0` は policy 層を通り spawn で `SPAWN_ERROR` へ縮退する。
    let err = rt
        .execute(
            "command",
            &json!({"name": "self", "args": [FIXTURE, "a\0b"]}),
            None,
        )
        .await
        .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::ToolExecutionFailure, "{err}");
    assert_eq!(
        e.details().unwrap()["spawnCode"],
        json!("SPAWN_ERROR"),
        "{err}"
    );
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
        let err = exec_runtime(&dir.root(), json!({"self": grant})).unwrap_err();
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
        let err = exec_runtime(&dir.root(), json!({"self": grant})).unwrap_err();
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
    let mut pending = tokio::spawn({
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
    let pid: u32 = std::fs::read_to_string(&pid_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    #[cfg(windows)]
    let process_handle = {
        let handle = windows_process_handle::ProcessHandle::open(pid)
            .unwrap_or_else(|error| panic!("OpenProcess failed for fixture PID {pid}: {error}"));
        let initial = handle.wait(0);
        assert_eq!(
            initial,
            windows_process_handle::ProcessHandle::WAIT_TIMEOUT,
            "fixture PID {pid} was not live on the captured process handle before abort (wait result={initial})"
        );
        handle
    };
    let abort_started = Instant::now();
    call.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Aborted,
        "stop managed command",
    )));
    let err = match tokio::time::timeout(Duration::from_secs(20), &mut pending).await {
        Ok(result) => result.unwrap().unwrap_err(),
        Err(_) => {
            let elapsed = abort_started.elapsed();
            #[cfg(windows)]
            {
                pending.abort();
                let terminate_result = process_handle.terminate(1);
                let cleanup_wait = process_handle.wait(5_000);
                let cleanup_wait_error = (cleanup_wait
                    == windows_process_handle::ProcessHandle::WAIT_FAILED)
                    .then(windows_process_handle::ProcessHandle::last_error);
                panic!(
                    "operation=command phase=await-abort-result elapsed={elapsed:?} allowed_budget=20s pid={pid}; fixture cleanup separate: TerminateProcess={terminate_result:?}, WaitForSingleObject(5s)={cleanup_wait}, wait_error={cleanup_wait_error:?}"
                );
            }
            #[cfg(not(windows))]
            {
                pending.abort();
                panic!("operation=command phase=await-abort-result elapsed={elapsed:?} allowed_budget=20s pid={pid}");
            }
        }
    };
    assert_eq!(err.to_string(), "stop managed command");
    #[cfg(windows)]
    {
        let wait = process_handle.wait(20_000);
        let wait_error = (wait == windows_process_handle::ProcessHandle::WAIT_FAILED)
            .then(windows_process_handle::ProcessHandle::last_error);
        assert_eq!(
            wait,
            windows_process_handle::ProcessHandle::WAIT_OBJECT_0,
            "operation=command phase=await-process-termination elapsed={:?} allowed_budget=20s pid={pid}; WaitForSingleObject={wait}, error={wait_error:?}",
            abort_started.elapsed()
        );
    }
    #[cfg(not(windows))]
    {
        wait_for(|| !alive(pid), Duration::from_secs(20), "pid gone").await;
        assert!(!alive(pid));
    }
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

/// `log_tee` 全量 sink(R26) — `maxOutputBytes` cap を超えた分も含め
/// stdout/stderr 全 byte が sink へ届き、bounded result は不変。
#[tokio::test]
async fn log_tee_captures_full_output_beyond_cap() {
    let dir = TempDir::new();
    let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = {
        let captured = Arc::clone(&captured);
        move |chunk: &[u8]| captured.lock().unwrap().extend_from_slice(chunk)
    };
    let mut opts = options(
        &dir.root(),
        command_policy(json!({
            "self": grant_with("output", Some("5000"), json!({"maxOutputBytes": 256}))
        })),
        vec![ToolName::Command],
    );
    opts.route_context = Some(RouteCommandContext {
        log_tee: Some(Arc::new(sink)),
        ..RouteCommandContext::default()
    });
    let rt = create_tool_runtime(opts, None).unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    let tee = captured.lock().unwrap();
    // tee には bounded cap を超えた全量(5000B stdout + 3B stderr)
    // が届く — interleaved 順序は不定だが各 stream の byte は含む。
    let x_bytes = tee.iter().filter(|b| **b == b'x').count();
    assert!(x_bytes >= 5000, "tee x bytes: {x_bytes}");
    assert!(
        tee.windows(3).any(|w| w == b"err"),
        "tee missing stderr bytes"
    );
    // bounded result は従来どおり cap 適用。
    let details = result.details.as_ref().expect("details");
    let bounded = details["stdout"]["capturedBytes"].as_u64().unwrap()
        + details["stderr"]["capturedBytes"].as_u64().unwrap();
    assert_eq!(bounded, 256);
}

/// tee sink の write 失敗は closure 自身が記録し command 結果を
/// 壊さない(postprocessFailure 縮退、R29)。
#[tokio::test]
async fn log_tee_sink_failure_does_not_break_result() {
    let dir = TempDir::new();
    // sink 側が write 失敗を記録する形(実 sink は RouteLogWriter)。
    let failed = Arc::new(AtomicBool::new(false));
    let sink = {
        let failed = Arc::clone(&failed);
        move |_: &[u8]| {
            let write: io::Result<()> = Err(io::Error::other("disk full"));
            if write.is_err() {
                failed.store(true, Ordering::SeqCst);
            }
        }
    };
    let mut opts = options(
        &dir.root(),
        command_policy(json!({"self": self_grant("output", Some("64"))})),
        vec![ToolName::Command],
    );
    opts.route_context = Some(RouteCommandContext {
        log_tee: Some(Arc::new(sink)),
        ..RouteCommandContext::default()
    });
    let rt = create_tool_runtime(opts, None).unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    assert!(failed.load(Ordering::SeqCst));
    assert_eq!(
        result.details.as_ref().unwrap()["state"],
        json!("completed")
    );
}

/// route context `extra_env` — `grant.env` への override として
/// 子 process env へ畳まれる(A3)。grant env `MODE=fail` を
/// `MODE=inspect` で上書きし sentinel 出力が返ることを検証する。
#[tokio::test]
async fn route_context_extra_env_overrides_grant_env() {
    let dir = TempDir::new();
    let mut opts = options(
        &dir.root(),
        command_policy(json!({"self": self_grant("fail", None)})),
        vec![ToolName::Command],
    );
    let mut extra_env = Map::new();
    extra_env.insert(MODE_ENV.to_string(), json!("inspect"));
    opts.route_context = Some(RouteCommandContext {
        extra_env,
        ..RouteCommandContext::default()
    });
    let rt = create_tool_runtime(opts, None).unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    let stdout_text = result.details.as_ref().unwrap()["stdout"]["text"]
        .as_str()
        .unwrap()
        .to_string();
    let inspected = sentinel_json(&stdout_text);
    assert_eq!(inspected["mode"], json!("inspect"));
}

/// route context `exec_cwd` — grant.cwd 経路を bypass し、workspace
/// containment 無しで payload cwd を復元する(R18)。`inspect`
/// fixture が報告する実 cwd が override dir と一致するか検証。
#[tokio::test]
async fn route_context_exec_cwd_overrides_grant_cwd() {
    let dir = TempDir::new();
    let outside = TempDir::new(); // workspace 外の実行 dir
    let mut opts = options(
        &dir.root(),
        command_policy(json!({"self": self_grant("inspect", None)})),
        vec![ToolName::Command],
    );
    opts.route_context = Some(RouteCommandContext {
        exec_cwd: Some(outside.0.to_string_lossy().into_owned()),
        ..RouteCommandContext::default()
    });
    let rt = create_tool_runtime(opts, None).unwrap();
    let result = rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .unwrap();
    let stdout_text = result.details.as_ref().unwrap()["stdout"]["text"]
        .as_str()
        .unwrap()
        .to_string();
    let inspected = sentinel_json(&stdout_text);
    // Windows verbatim prefix・大小文字・separator 差を正規化して比較。
    let norm = |p: &str| {
        p.trim_start_matches(r"\\?\")
            .replace('/', "\\")
            .to_lowercase()
    };
    assert_eq!(
        norm(inspected["cwd"].as_str().unwrap()),
        norm(&outside.root())
    );
}

/// route context `accept_any_exit_code` — declared grant の
/// `exitCodes` を上書きして非 0 exit を result 化する(C2)。
#[tokio::test]
async fn route_context_accept_any_exit_code_mirrors_exit() {
    let dir = TempDir::new();
    // baseline: accept_any 無しは exit 7 が ToolExecutionFailure。
    let baseline = command_runtime(
        &dir.root(),
        command_policy(json!({"self": self_grant("fail", None)})),
    )
    .unwrap();
    assert!(baseline
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
        .is_err());
    // accept_any_exit_code で同じ grant が exitCode 7 result を返す。
    let mut opts = options(
        &dir.root(),
        command_policy(json!({"self": self_grant("fail", None)})),
        vec![ToolName::Command],
    );
    opts.route_context = Some(RouteCommandContext {
        accept_any_exit_code: true,
        ..RouteCommandContext::default()
    });
    let rt = create_tool_runtime(opts, None).unwrap();
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

/// pin 後に pinned path が差し替えられた場合、spawn 前再検証が
/// `POLICY_VIOLATION` で fail-closed する — 観測対象は path の差替
/// (symlink retarget・削除)で、同一 path の内容差替は審査しない。
#[tokio::test]
async fn spawn_time_revalidation_detects_path_changes() {
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

    // 同一 path の内容差替は検出対象外 — 再検証を通り spawn 層へ
    // 進む(unix では shebang 付き text が実行されて completed に
    // なり得る。win32 では spawn 層の `TOOL_EXECUTION_FAILURE`)。
    std::fs::write(&exe, b"#!/bin/sh\necho swapped\n").unwrap();
    match rt
        .execute(
            "command",
            &json!({"name": "self", "args": self_args()}),
            None,
        )
        .await
    {
        Ok(result) => {
            assert_eq!(
                result.details.as_ref().unwrap()["state"],
                json!("completed")
            );
        }
        Err(err) => {
            // policy 層ではなく spawn 層の失敗であることを pin。
            assert_eq!(code(&err), Some(ErrorCode::ToolExecutionFailure), "{err}");
        }
    }

    // pinned file の削除 → `"no longer resolves"`(全 platform)。
    std::fs::remove_file(&exe).unwrap();
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
        e.message().contains("no longer resolves"),
        "{}",
        e.message()
    );

    // pinned file を symlink へ置換 → realpath 不一致で
    // `"path changed since pin"`(windows symlink は権限依存のため
    // unix のみ)。
    #[cfg(unix)]
    {
        let real = bin.join("real-target");
        std::fs::copy(std::env::current_exe().unwrap(), &real).unwrap();
        std::os::unix::fs::symlink(&real, &exe).unwrap();
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
            e.message().contains("path changed since pin"),
            "{}",
            e.message()
        );
    }
}

/// `check_exec_execution` — route resolver が使う spawn 無し
/// preflight seam(design §viability・B#3)。`command::execute` と
/// 同一 predicate であることを、受理・拒否の双方で確認する。
#[test]
fn check_command_execution_shares_execute_predicate() {
    use cheaper_core::policy::exec::resolve_exec_policy;
    use cheaper_core::policy::security::ExecDenyList;
    use cheaper_tools::tools::exec::{check_exec_execution, pin_exec_grant, ExecToolOptions};

    let dir = TempDir::new();
    let root = dir.root();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let policy = resolve_exec_policy(
        Some(&exec_doc(json!({
            "grants": {"t": {"executable": exe, "argv": ["test", "*"]}}
        }))),
        &ExecDenyList::default(),
    )
    .unwrap();
    let options = ExecToolOptions {
        root: root.clone(),
        symlink_policy: SymlinkPolicy::Deny,
        exec: policy.clone(),
        pins: Default::default(),
        on_timeout: None,
        denylist: None,
        exec_deny: ExecDenyList::default(),
        write_scope: None,
        route_context: None,
    };
    // pin map 不在は fail-closed(spawn 層へは到達しない)。
    let e = check_exec_execution(&options, "t", &["test".into()])
        .map(|_| ())
        .unwrap_err();
    assert_eq!(cheaper_err(&e).code(), ErrorCode::PolicyViolation);
    let pinned = pin_exec_grant(
        "t",
        policy.grants.get("t").unwrap(),
        &root,
        SymlinkPolicy::Deny,
        None,
        &ExecDenyList::default(),
    )
    .unwrap();
    let options = ExecToolOptions {
        pins: std::collections::BTreeMap::from([("t".to_string(), pinned)]),
        ..options
    };
    // 受理 — grant 名・argv pattern・executable pin 解決・cwd 全通過。
    let checked = check_exec_execution(&options, "t", &["test".into(), "--x".into()]).unwrap();
    assert!(!checked.executable.is_empty());
    assert_eq!(checked.cwd, root);
    // argv pattern 不一致 → execute と同じ POLICY_VIOLATION。
    let e = check_exec_execution(&options, "t", &["rm".into()])
        .map(|_| ())
        .unwrap_err();
    assert_eq!(cheaper_err(&e).code(), ErrorCode::PolicyViolation);
    // metachar token も pattern(`"test","*"`)適合なら受理される —
    // argv 内容は審査対象外。
    let checked = check_exec_execution(&options, "t", &["test".into(), "a;rm".into()]).unwrap();
    assert!(!checked.executable.is_empty());
    // 未宣言 grant → `unknown or unauthorized command grant`
    // (resolve_exec_grant の fail-closed — resolver の no-grant
    // viability が拾う分類)。
    assert!(check_exec_execution(&options, "absent", &[]).is_err());
}

// ---------------------------------------------------------------------------
// security.execDenyPatterns(issue #164)— options.security の raw JSON が
// scope 構築の compile → 適用点 1/2/3 に届くことの統合 pin。
// ---------------------------------------------------------------------------

/// `ToolRuntimeOptions.security` の raw JSON deny — declared
/// executable が pattern に match する grant は scope 構築の
/// 適用点 1(`resolve_exec_policy`)で `InvalidRuntimePolicy` に
/// fail-closed する。空/`None` security は no-op。
#[test]
fn security_exec_deny_applies_at_scope_construction() {
    let dir = TempDir::new();
    let root = dir.root();
    let command = command_policy(json!({"self": self_grant("inspect", None)}));
    // `.*` deny-all — declared stem に無条件 match する opt-in。
    let mut opts = options(&root, command.clone(), vec![ToolName::Command]);
    opts.security = Some(json!({"execDenyPatterns": [".*"]}));
    let err = create_tool_runtime(opts, None).unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    assert!(
        e.message().contains("denied by security.execDenyPatterns"),
        "{e}"
    );
    assert_eq!(e.details().unwrap()["grant"], json!("self"));
    // 空集合と `None` は no-op — 同じ command で runtime が構築できる。
    let mut opts = options(&root, command.clone(), vec![ToolName::Command]);
    opts.security = Some(json!({"execDenyPatterns": []}));
    create_tool_runtime(opts, None).expect("empty deny is no-op");
    create_tool_runtime(options(&root, command, vec![ToolName::Command]), None)
        .expect("absent security is no-op");
}

/// 適用点 2 — resolved basename が deny に match する grant を含む
/// runtime 構築は `pin_exec_grants` 全体で `PolicyViolation`
/// (blast radius pin — `safe` grant が先に pin 済みでも fail)。
/// resolved basename ≠ declared basename は symlink のみで作れる
/// (Windows symlink は権限依存)ため unix gate。
#[cfg(unix)]
#[test]
fn security_exec_deny_pin_blast_radius() {
    let dir = TempDir::new();
    let root = dir.root();
    // 実体 `deny-me` ← symlink `helper`(declared)。内容は
    // current_exe 複製(basename のみが判定対象)。
    let target = dir.join("deny-me");
    std::fs::copy(std::env::current_exe().unwrap(), &target).unwrap();
    std::os::unix::fs::symlink(&target, dir.join("helper")).unwrap();
    let command = command_policy(json!({
        "helper": {"executable": "./helper"},
    }));
    let mut opts = options(&root, command, vec![ToolName::Command]);
    // The symlink is the intended way to make the resolved basename differ
    // from the declared basename. Its target remains inside this workspace.
    opts.symlink_policy = SymlinkPolicy::WithinWorkspace;
    opts.security = Some(json!({"execDenyPatterns": ["deny-me"]}));
    let err = create_tool_runtime(opts, None).unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("resolved executable is denied by security.execDenyPatterns"),
        "{e}"
    );
    assert_eq!(e.details().unwrap()["grant"], json!("helper"));
}

/// 適用点 3 — `ExecToolOptions.exec_deny` が `check_exec_execution`
/// (spawn 前再検証 seam)に届き、pinned executable の basename match で
/// `PolicyViolation` + `details.grant`。
#[test]
fn check_command_execution_applies_exec_deny_at_verify() {
    use cheaper_core::policy::exec::resolve_exec_policy;
    use cheaper_core::policy::security::executable_stem;
    use cheaper_tools::tools::exec::{check_exec_execution, pin_exec_grant, ExecToolOptions};

    let dir = TempDir::new();
    let root = dir.root();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let policy = resolve_exec_policy(
        Some(&exec_doc(json!({
            "grants": {"t": {"executable": exe, "argv": ["test", "*"]}}
        }))),
        &ExecDenyList::default(),
    )
    .unwrap();
    let pinned = pin_exec_grant(
        "t",
        policy.grants.get("t").unwrap(),
        &root,
        SymlinkPolicy::Deny,
        None,
        &ExecDenyList::default(),
    )
    .unwrap();
    // pinned basename の stem で deny を構成 → spawn 前再検証が
    // pin-equality 通過後に deny で fail-closed。
    let deny = ExecDenyList::compile(&[executable_stem(&pinned)]).unwrap();
    let options = ExecToolOptions {
        root: root.clone(),
        symlink_policy: SymlinkPolicy::Deny,
        exec: policy,
        pins: std::collections::BTreeMap::from([("t".to_string(), pinned)]),
        on_timeout: None,
        denylist: None,
        exec_deny: deny,
        write_scope: None,
        route_context: None,
    };
    let err = check_exec_execution(&options, "t", &["test".into(), "--x".into()])
        .map(|_| ())
        .unwrap_err();
    let e = cheaper_err(&err);
    assert_eq!(e.code(), ErrorCode::PolicyViolation);
    assert!(
        e.message()
            .contains("resolved executable is denied by security.execDenyPatterns"),
        "{e}"
    );
    assert_eq!(e.details().unwrap()["grant"], json!("t"));
}
