//! `packages/tool-runtime/test/process.test.ts` 対応の integration test。
//!
//! spawn 対象: TS test は `node` + `process-child.mjs` を spawn するが、
//! Rust 版は設計固定ルールにより `node` を PATH 解決しないため、
//! **この test binary 自身(`current_exe`)を fixture child として再利用**
//! する。fixture は `proc_fixture_child` test が
//! `CHEAPER_PROC_CHILD_MODE` env で切替える。libtest の filter 引数と
//! fixture mode を混ぜないため、mode/flag は env、path 引数だけを
//! argv(filters)で渡す。harness の stdout noise("running 1 test" 等)を
//! 避けるため structured 出力は `@@` sentinel 行に乗せる。
//!
//! POSIX 向けは `cfg(unix)` で構造化済み。

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::process::{resolve_process_policy, ResolvedProcessPolicy};
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::catalog::CanonicalToolResult;
use cheaper_tools::operation::{AbortController, AbortReason};
use cheaper_tools::tools::process::{
    create_process_tool, find_cheaper_error, ProcessTool, ProcessToolOptions,
};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

const FIXTURE: &str = "proc_fixture_child";
const MODE_ENV: &str = "CHEAPER_PROC_CHILD_MODE";
const ARG_ENV: &str = "CHEAPER_PROC_CHILD_ARG";
const SENTINEL: &str = "@@";

// ---------------------------------------------------------------------------
// Fixture child(re-exec of this test binary)
// ---------------------------------------------------------------------------

#[cfg(unix)]
static GRACEFUL_TERM: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn graceful_handler(_sig: i32) {
    GRACEFUL_TERM.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// `process-child.mjs` 相当の fixture。通常の `cargo test` では
/// `CHEAPER_PROC_CHILD_MODE` が未設定で即 return(fixture mode のみ動作)。
/// argv は常に `[exe, "proc_fixture_child", "--nocapture", ...payload]`
/// (`--nocapture` で libtest の per-test output capture を bypass し、
/// `println!` が real stdout に届くようにする)。
#[test]
fn proc_fixture_child() {
    let Ok(mode) = std::env::var(MODE_ENV) else {
        return;
    };
    let arg_env = std::env::var(ARG_ENV).unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(3).collect();
    match mode.as_str() {
        "inspect" => {
            let out = json!({
                "cwd": std::env::current_dir().unwrap().to_string_lossy(),
                "args": args,
                "explicit": std::env::var("ONLY_THIS").ok(),
                "secret": std::env::var("CHEAPER_PROCESS_TEST_SECRET").ok(),
                "path": std::env::var("PATH").ok(),
                "stdin": read_stdin(),
            });
            println!("{SENTINEL}{out}");
        }
        "output" => {
            let mut stdout = std::io::stdout().lock();
            let mut stderr = std::io::stderr().lock();
            stdout.write_all(&vec![0xffu8; 131_072]).unwrap();
            stdout.flush().unwrap();
            stderr.write_all(&vec![0x51u8; 131_072]).unwrap();
            stderr.flush().unwrap();
        }
        "fail" => {
            println!("before failure");
            eprintln!("failure detail");
            std::process::exit(7);
        }
        "hold" => {
            let gate = args.first().expect("hold gate");
            while !Path::new(gate).exists() {
                std::thread::sleep(Duration::from_millis(20));
            }
            println!("released");
        }
        "touch" => {
            let target = PathBuf::from(args.first().expect("touch target"));
            if let Some(parent) = target.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).unwrap();
                }
            }
            std::fs::write(&target, "generated").unwrap();
            println!("generated");
        }
        "stubborn" => {
            write_pid(args.first().expect("pid path"));
            park_forever();
        }
        "graceful" => {
            write_pid(args.first().expect("pid path"));
            #[cfg(unix)]
            {
                unsafe {
                    libc::signal(libc::SIGTERM, graceful_handler as usize);
                }
                let target = args.first().unwrap().clone();
                loop {
                    if GRACEFUL_TERM.load(std::sync::atomic::Ordering::SeqCst) {
                        std::fs::write(format!("{target}.terminated"), "graceful").unwrap();
                        std::process::exit(0);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
            #[cfg(not(unix))]
            park_forever();
        }
        "child" => {
            if arg_env == "stubborn" {
                ignore_sigterm();
            }
            println!("ready");
            std::io::stdout().flush().unwrap();
            park_forever();
        }
        "branch" => {
            if arg_env == "stubborn" {
                ignore_sigterm();
            }
            let leaf_flag = (!arg_env.is_empty()).then_some(arg_env.as_str());
            let (leaf, leaf_out) = spawn_fixture("child", leaf_flag);
            let mut reader = BufReader::new(leaf_out);
            wait_line(&mut reader, "ready");
            println!(
                "{SENTINEL}{}",
                json!({"child": std::process::id(), "grandchild": leaf.id()})
            );
            std::io::stdout().flush().unwrap();
            park_forever();
        }
        "tree" => {
            let target = args.first().expect("tree target").clone();
            let (child, child_out) = spawn_fixture("child", None);
            let mut reader = BufReader::new(child_out);
            wait_line(&mut reader, "ready");
            std::fs::write(
                &target,
                json!({"parent": std::process::id(), "child": child.id()}).to_string(),
            )
            .unwrap();
            park_forever();
        }
        "exit-tree" | "fail-tree" | "deep-tree" | "deep-stubborn" => {
            let stubborn = mode.as_str() == "deep-stubborn";
            if stubborn {
                ignore_sigterm();
            }
            let target = args.first().expect("deep target").clone();
            let flag = stubborn.then_some("stubborn");
            let (_branch, branch_out) = spawn_fixture("branch", flag);
            let mut reader = BufReader::new(branch_out);
            let line = wait_line(&mut reader, SENTINEL);
            let ids: Value = serde_json::from_str(&line[SENTINEL.len()..]).unwrap();
            std::fs::write(
                &target,
                json!({
                    "parent": std::process::id(),
                    "child": ids["child"],
                    "grandchild": ids["grandchild"],
                })
                .to_string(),
            )
            .unwrap();
            if stubborn || mode.as_str() == "deep-tree" {
                park_forever();
            } else {
                std::process::exit(if mode.as_str() == "fail-tree" { 7 } else { 0 });
            }
        }
        other => panic!("unknown fixture mode: {other}"),
    }
}

fn read_stdin() -> String {
    // stdin は tool 側で ignore — 内容が来ないことを確認するだけ。
    let mut buf = String::new();
    let _ = std::io::stdin().lock().read_to_string(&mut buf);
    buf
}

fn write_pid(path: &str) {
    std::fs::write(path, std::process::id().to_string()).unwrap();
}

fn park_forever() -> ! {
    loop {
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn ignore_sigterm() {
    unsafe {
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn ignore_sigterm() {}

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

/// fixture を `child`/`branch` mode で spawn し piped stdout を返す
/// (`windowsHide` 相当の `CREATE_NO_WINDOW` 付き)。
fn spawn_fixture(
    mode: &str,
    arg_env: Option<&str>,
) -> (std::process::Child, std::process::ChildStdout) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([FIXTURE, "--nocapture"])
        .env(MODE_ENV, mode)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    if let Some(flag) = arg_env {
        command.env(ARG_ENV, flag);
    }
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
            "cheaper-proc-it-{}-{}",
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

/// `task(mode, args, outputs)` 相当。command[0] は absolute native
/// executable(test binary)で PATH 解決に依存しない。
fn fixture_task(mode: &str, args: &[&str], outputs: &[&str]) -> Value {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut command = vec![json!(exe), json!(FIXTURE), json!("--nocapture")];
    command.extend(args.iter().map(|a| json!(a)));
    let mut env = Map::new();
    env.insert(MODE_ENV.to_string(), json!(mode));
    json!({
        "command": command,
        "inputs": [],
        "outputs": outputs,
        "env": Value::Object(env),
    })
}

fn policy(tasks: Value, extra: Value) -> ResolvedProcessPolicy {
    let mut raw = Map::new();
    raw.insert("tasks".to_string(), tasks);
    if let Value::Object(map) = extra {
        raw.extend(map);
    }
    resolve_process_policy(Some(&Value::Object(raw))).unwrap()
}

fn tool_with(root: &str, tasks: Value, extra: Value) -> ProcessTool {
    create_process_tool(ProcessToolOptions {
        root: root.to_string(),
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(tasks, extra),
        on_timeout: None,
        denylist: None,
        write_scope: None,
    })
}

fn code(err: &ToolError) -> Option<ErrorCode> {
    find_cheaper_error(err.as_ref()).map(|e| e.code())
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

/// `alive(pid)`(`process.kill(pid, 0)` + /proc zombie 判定相当)。
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
// canonical managed process
// ---------------------------------------------------------------------------

#[tokio::test]
async fn executes_literal_argv_in_validated_cwd_with_minimal_env_and_ignored_stdin() {
    let dir = TempDir::new();
    let root = dir.root();
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::env::set_var("CHEAPER_PROCESS_TEST_SECRET", "do-not-inherit");
    let mut task = fixture_task("inspect", &["literal space", "*.txt"], &[]);
    task["cwd"] = json!("sub");
    task["env"]["ONLY_THIS"] = json!("yes");
    let tool = tool_with(&root, json!({"inspect": task}), json!({}));
    let result: CanonicalToolResult = tool
        .execute(&json!({"task": "inspect"}), None)
        .await
        .unwrap();
    let details = result.details.as_ref().unwrap();
    assert_eq!(details["state"], json!("completed"));
    assert_eq!(details["exitCode"], json!(0));
    assert_eq!(details["stderr"]["bytes"], json!(0));
    let report = sentinel_json(details["stdout"]["text"].as_str().unwrap());
    let expected_cwd = canonicalize_workspace_root(dir.join("sub").to_str().unwrap()).unwrap();
    assert_eq!(report["cwd"], json!(expected_cwd));
    assert_eq!(report["args"], json!(["literal space", "*.txt"]));
    assert_eq!(report["explicit"], json!("yes"));
    assert_eq!(report["secret"], Value::Null);
    // Windows は required env 空値遮断で PATH=""、POSIX は未設定(null)。
    let expected_path: Value = if cfg!(windows) {
        json!("")
    } else {
        Value::Null
    };
    assert_eq!(report["path"], expected_path);
    assert_eq!(report["stdin"], json!(""));
    // 表示 fold text。
    assert!(result.content[0]
        .text
        .starts_with("process inspect: completed, exitCode=0\nstdout:"));
}

#[tokio::test]
async fn node_task_is_an_explicit_policy_violation() {
    let dir = TempDir::new();
    let tool = tool_with(
        &dir.root(),
        json!({"script": {
            "command": ["node", "worker.js"],
            "inputs": [],
            "outputs": [],
        }}),
        json!({}),
    );
    let err = tool
        .execute(&json!({"task": "script"}), None)
        .await
        .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::PolicyViolation), "{err}");
    assert!(err.to_string().contains("\"node\""));
}

#[tokio::test]
async fn fails_nonzero_exit_with_bounded_structured_diagnostics() {
    let dir = TempDir::new();
    let tool = tool_with(
        &dir.root(),
        json!({"fail": fixture_task("fail", &[], &[])}),
        json!({}),
    );
    let err = tool
        .execute(&json!({"task": "fail"}), None)
        .await
        .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::ToolExecutionFailure));
    let cheaper = find_cheaper_error(err.as_ref()).unwrap();
    let details = cheaper.details().unwrap();
    assert_eq!(details["task"], json!("fail"));
    assert_eq!(details["state"], json!("failed"));
    assert_eq!(details["exitCode"], json!(7));
    assert!(details["stdout"]["text"]
        .as_str()
        .unwrap()
        .contains("before failure"));
    assert!(details["stderr"]["text"]
        .as_str()
        .unwrap()
        .contains("failure detail"));
}

#[tokio::test]
async fn caps_combined_output_and_drains_all_bytes() {
    for cap in [31u64, 65_536, 98_304] {
        let dir = TempDir::new();
        let tool = tool_with(
            &dir.root(),
            json!({"output": fixture_task("output", &[], &[])}),
            json!({"maxOutputBytes": cap}),
        );
        let result = tool
            .execute(&json!({"task": "output"}), None)
            .await
            .unwrap();
        let details = result.details.as_ref().unwrap();
        let stdout = &details["stdout"];
        let stderr = &details["stderr"];
        // fixture 128KiB + libtest harness noise。combined cap が効く。
        assert!(stdout["bytes"].as_u64().unwrap() >= 131_072);
        assert!(stderr["bytes"].as_u64().unwrap() >= 131_072);
        let captured =
            stdout["capturedBytes"].as_u64().unwrap() + stderr["capturedBytes"].as_u64().unwrap();
        assert_eq!(captured, cap, "cap={cap}");
        let text_bytes =
            stdout["text"].as_str().unwrap().len() + stderr["text"].as_str().unwrap().len();
        assert!(text_bytes <= cap as usize, "cap={cap}");
        assert!(stdout["truncated"] == json!(true) || stderr["truncated"] == json!(true));
    }
}

#[tokio::test]
async fn times_out_native_process_and_cleans_tree() {
    let dir = TempDir::new();
    let pids_path = dir.join("pid").to_string_lossy().into_owned();
    let tool = tool_with(
        &dir.root(),
        json!({"stubborn": fixture_task("stubborn", &[&pids_path], &["pid"])}),
        json!({"timeout": "500ms", "killGrace": "100ms"}),
    );
    let err = tool
        .execute(&json!({"task": "stubborn"}), None)
        .await
        .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::Timeout));
    let pid: u32 = std::fs::read_to_string(&pids_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    wait_for(|| !alive(pid), Duration::from_secs(20), "pid gone").await;
    assert!(!alive(pid));
}

#[tokio::test]
async fn abort_kills_managed_parent_and_descendant_and_preserves_reason() {
    let dir = TempDir::new();
    let pids_path = dir.join("pids.json").to_string_lossy().into_owned();
    let tool = tool_with(
        &dir.root(),
        json!({"tree": fixture_task("tree", &[&pids_path], &["pids.json"])}),
        json!({"killGrace": "200ms"}),
    );
    let controller = AbortController::new();
    let input = json!({"task": "tree"});
    let pending = tool.execute(&input, Some(controller.signal()));
    tokio::pin!(pending);
    tokio::select! {
        _ = wait_for(
            || Path::new(&pids_path).exists(),
            Duration::from_secs(15),
            "pids.json",
        ) => {}
        r = &mut pending => panic!("tree fixture died early: {r:?}"),
    }
    let pids: Value = serde_json::from_str(&std::fs::read_to_string(&pids_path).unwrap()).unwrap();
    controller.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Aborted,
        "stop managed tree",
    )));
    let err = pending.await.unwrap_err();
    assert_eq!(err.to_string(), "stop managed tree");
    for key in ["parent", "child"] {
        let pid = pids[key].as_u64().unwrap() as u32;
        wait_for(|| !alive(pid), Duration::from_secs(20), "{key} gone").await;
        assert!(!alive(pid), "{key} should be dead");
    }
}

#[tokio::test]
async fn cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout() {
    for (mode, expected) in [
        ("exit-tree", "ok"),
        ("fail-tree", "fail"),
        ("deep-tree", "abort"),
        ("deep-stubborn", "timeout"),
    ] {
        let dir = TempDir::new();
        let pids_path = dir.join("pids.json").to_string_lossy().into_owned();
        let tool = tool_with(
            &dir.root(),
            json!({"tree": fixture_task(mode, &[&pids_path], &["pids.json"])}),
            json!({"timeout": "2s", "killGrace": "100ms"}),
        );
        let controller = AbortController::new();
        let input = json!({"task": "tree"});
        let pending = tool.execute(&input, Some(controller.signal()));
        tokio::pin!(pending);
        tokio::select! {
            _ = wait_for(
                || Path::new(&pids_path).exists(),
                Duration::from_secs(15),
                "pids.json",
            ) => {}
            r = &mut pending => panic!("{mode} fixture died early: {r:?}"),
        }
        let pids: Value =
            serde_json::from_str(&std::fs::read_to_string(&pids_path).unwrap()).unwrap();
        if expected == "abort" {
            controller.abort(AbortReason::from(CheaperError::new(
                ErrorCode::Aborted,
                "outer tree cancellation",
            )));
        }
        match pending.await {
            Ok(_) => assert_eq!(expected, "ok"),
            Err(err) => match expected {
                "fail" => {
                    assert_eq!(code(&err), Some(ErrorCode::ToolExecutionFailure));
                    let details = find_cheaper_error(err.as_ref()).unwrap().details().unwrap();
                    assert_eq!(details["exitCode"], json!(7));
                }
                "abort" => assert_eq!(err.to_string(), "outer tree cancellation"),
                "timeout" => assert_eq!(code(&err), Some(ErrorCode::Timeout)),
                other => panic!("{other} expected success"),
            },
        }
        for key in ["parent", "child", "grandchild"] {
            let pid = pids[key].as_u64().unwrap() as u32;
            wait_for(|| !alive(pid), Duration::from_secs(20), "{key} gone").await;
            assert!(!alive(pid), "{mode}:{key} should be dead");
        }
    }
}

#[tokio::test]
async fn touch_with_matching_write_scope_succeeds() {
    let dir = TempDir::new();
    let root = dir.root();
    let marker = dir.join("out").to_string_lossy().into_owned();
    let tool = create_process_tool(ProcessToolOptions {
        root,
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(
            json!({"check": fixture_task("touch", &[&marker], &["out"])}),
            json!({}),
        ),
        on_timeout: None,
        denylist: None,
        write_scope: Some(std::collections::HashSet::from(["out".to_string()])),
    });
    tool.execute(&json!({"task": "check"}), None).await.unwrap();
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "generated");
}

#[cfg(unix)]
#[tokio::test]
async fn allows_sigterm_exit_within_grace_before_force_kill() {
    let dir = TempDir::new();
    let pids_path = dir.join("pid").to_string_lossy().into_owned();
    let tool = tool_with(
        &dir.root(),
        json!({"graceful": fixture_task("graceful", &[&pids_path], &["pid"])}),
        json!({"timeout": "500ms", "killGrace": "1s"}),
    );
    let err = tool
        .execute(&json!({"task": "graceful"}), None)
        .await
        .unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::Timeout));
    wait_for(
        || Path::new(&format!("{pids_path}.terminated")).exists(),
        Duration::from_secs(20),
        "terminated marker",
    )
    .await;
    assert_eq!(
        std::fs::read_to_string(format!("{pids_path}.terminated")).unwrap(),
        "graceful"
    );
}
