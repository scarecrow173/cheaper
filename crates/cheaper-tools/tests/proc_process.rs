//! `packages/tool-runtime/test/process.test.ts` 対応の integration test。
//!
//! spawn 対象: TS test は `node` + `process-child.mjs` を spawn するが、
//! Rust 版は **この test binary 自身(`current_exe`)を fixture child
//! として再利用** する(環境非依存の deterministic fixture)。
//! fixture は `proc_fixture_child` test が
//! `CHEAPER_PROC_CHILD_MODE` env で切替える。libtest の filter 引数と
//! fixture mode を混ぜないため、mode/flag は env、path 引数だけを
//! argv(filters)で渡す。harness の stdout noise("running 1 test" 等)を
//! 避けるため structured 出力は `@@` sentinel 行に乗せる。
//!
//! tree-family mode(stubborn/graceful/tree/deep)の pid 通知は
//! `pids.d/{parent,child,grandchild}` の per-key atomic emit(tmp+rename)
//! で行い、`CHEAPER_PROC_PID_DIR` env が唯一の伝達路(issue #180 —
//! 並列負荷下 flake の determinism 再設計、design D1-D8)。
//!
//! POSIX 向けは `cfg(unix)` で構造化済み。

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::policy::exec::{build_exec_input, resolve_exec_policy, ResolvedExecPolicy};
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::catalog::CanonicalToolResult;
use cheaper_tools::operation::{AbortController, AbortReason};
use cheaper_tools::tools::exec::{pin_exec_grants, ExecToolOptions};
use cheaper_tools::tools::process::{
    create_process_tool, find_cheaper_error, process_tool_definition, ProcessTool,
};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

const FIXTURE: &str = "proc_fixture_child";
const MODE_ENV: &str = "CHEAPER_PROC_CHILD_MODE";
const ARG_ENV: &str = "CHEAPER_PROC_CHILD_ARG";
const SENTINEL: &str = "@@";
/// fixture → test の emit channel。pids.d dir を渡す唯一の経路。
const PID_DIR_ENV: &str = "CHEAPER_PROC_PID_DIR";
const PATH_REEXEC_ENV: &str = "CHEAPER_NODE_PATH_REEXEC";
const DIFF_CHILD_BIN: &str = env!("CARGO_BIN_EXE_diff_fixture_child");

/// emit 観測対象の全 key(deep mode の 3 段 + tree/stubborn/graceful の subset)。
const ALL_KEYS: &[&str] = &["parent", "child", "grandchild"];

// budget 階層(design §6): task timeout が fixture の観測可能な全
// lifetime と同じ clock で競合しないよう、wait 系は別 budget で bound する。
/// 全 key emit 待ち(abort mode の abort 前)。
const EMIT_WAIT_BUDGET: Duration = Duration::from_secs(30);
/// resolution 後の straggler emit 回収。
const POST_RESOLVE_EMIT_GRACE: Duration = Duration::from_secs(5);
/// emit 済み pid 集合の消滅待ち(集合 budget)。
const DEAD_BUDGET: Duration = Duration::from_secs(30);
/// `settle()` の wall-clock bound(task timeout 60s + cleanup worst ~90s + slack)。
const SETTLE_BUDGET: Duration = Duration::from_secs(180);
/// give-up 時の abort→cleanup 完走待ち(Windows terminate worst ~90s + slack)。
const DRAIN_BUDGET: Duration = Duration::from_secs(120);
/// file emit/marker 系の poll cadence。
const FILE_POLL: Duration = Duration::from_millis(20);
/// pid-death 系の poll cadence(R5 self-load 削減 — ~10 tasklist/s 以下)。
const PID_POLL: Duration = Duration::from_millis(100);

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
            let dir = pid_dir(&mode);
            emit_pid(&dir, "parent", std::process::id());
            park_forever();
        }
        "graceful" => {
            let dir = pid_dir(&mode);
            emit_pid(&dir, "parent", std::process::id());
            #[cfg(unix)]
            {
                unsafe {
                    libc::signal(libc::SIGTERM, graceful_handler as *const () as usize);
                }
                // handler install 直後の armed marker(kill-before-arming と
                // armed-but-no-terminated を test 側で区別するため)。
                emit_marker(&dir.join("parent.armed"), "armed");
                let terminated = dir.join("parent.terminated");
                loop {
                    if GRACEFUL_TERM.load(std::sync::atomic::Ordering::SeqCst) {
                        emit_marker(&terminated, "graceful");
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
            let dir = pid_dir(&mode);
            emit_pid(&dir, "child", std::process::id());
            if arg_env == "stubborn" {
                ignore_sigterm();
            }
            let leaf_flag = (!arg_env.is_empty()).then_some(arg_env.as_str());
            let (leaf, leaf_out) = spawn_fixture("child", leaf_flag);
            // OS pid は spawn 返却時点で確定 — `ready` 待ちより先に emit。
            emit_pid(&dir, "grandchild", leaf.id());
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
            let dir = pid_dir(&mode);
            emit_pid(&dir, "parent", std::process::id());
            let (child, child_out) = spawn_fixture("child", None);
            emit_pid(&dir, "child", child.id());
            let mut reader = BufReader::new(child_out);
            wait_line(&mut reader, "ready");
            park_forever();
        }
        "exit-tree" | "fail-tree" | "deep-tree" | "deep-stubborn" => {
            let dir = pid_dir(&mode);
            // arm 先頭で emit(ignore_sigterm より前)し pre-emit window を
            // 構造的に最小化する。
            emit_pid(&dir, "parent", std::process::id());
            let stubborn = mode.as_str() == "deep-stubborn";
            if stubborn {
                ignore_sigterm();
            }
            let flag = stubborn.then_some("stubborn");
            let (_branch, branch_out) = spawn_fixture("branch", flag);
            let mut reader = BufReader::new(branch_out);
            // `@@` 行は「chain 形成済み」の内部同期として維持(pid 転送
            // 目的は pids.d emit が担う)。
            let _ = wait_line(&mut reader, SENTINEL);
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

/// emit 責務を持つ mode が pids.d dir を得る唯一の経路
/// (`env::var(PID_DIR_ENV)`)。未設定・空値は fail-fast の mode 名入り panic。
fn pid_dir(mode: &str) -> PathBuf {
    match std::env::var(PID_DIR_ENV) {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!("fixture mode {mode:?} requires {PID_DIR_ENV}"),
    }
}

/// `{dir}/{key}.tmp` → `fs::rename` の atomic per-key emit。各 key は
/// owner process が 1 回だけ書く。`dir` は初回 emit 時に `create_dir_all`。
fn emit_pid(dir: &Path, key: &str, pid: u32) {
    std::fs::create_dir_all(dir)
        .unwrap_or_else(|e| panic!("emit {key}: create_dir_all {}: {e}", dir.display()));
    let tmp = dir.join(format!("{key}.tmp"));
    std::fs::write(&tmp, pid.to_string())
        .unwrap_or_else(|e| panic!("emit {key}: write {}: {e}", tmp.display()));
    std::fs::rename(&tmp, dir.join(key))
        .unwrap_or_else(|e| panic!("emit {key}: rename {}: {e}", dir.join(key).display()));
}

/// `{path}.tmp-<pid>` → `fs::rename` の atomic marker write
/// (`.armed`/`.terminated` 用 — unix `graceful` mode のみが使う。
/// tmp 名を per-process にして衝突を避ける)。
#[cfg(unix)]
fn emit_marker(path: &Path, text: &str) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = PathBuf::from(format!("{}.tmp-{}", path.display(), std::process::id()));
    std::fs::write(&tmp, text)
        .unwrap_or_else(|e| panic!("emit marker {}: write: {e}", path.display()));
    std::fs::rename(&tmp, path)
        .unwrap_or_else(|e| panic!("emit marker {}: rename: {e}", path.display()));
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

/// fixture 内で spawn した direct child の kill-on-drop guard。
/// panic(unwind)経路でのみ発火し(正常 `process::exit` / `park_forever`
/// では drop 自体が走らない)、`try_wait()` が未終了を示す場合だけ kill
/// する — 既 exit の pid が再利用済みでも誤 kill しない。
/// Windows は absolute taskkill の `/PID <pid> /T /F`(subtree)+
/// `child.kill()` belt、unix は direct `child.kill()`(transitive は
/// parent 死亡が誘発する production `kill(-pgid)` cleanup が処理する)。
struct SpawnedChild(std::process::Child);

impl SpawnedChild {
    fn id(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for SpawnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(None)) {
            return;
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new(taskkill_path())
                .args([
                    "/PID".to_string(),
                    self.0.id().to_string(),
                    "/T".to_string(),
                    "/F".to_string(),
                ])
                .output();
            let _ = self.0.kill();
        }
        #[cfg(unix)]
        {
            let _ = self.0.kill();
        }
    }
}

/// `{SystemRoot}\System32\taskkill.exe` の absolute path(tree.rs:407 と
/// 同じ convention — ambient PATH に依存しない)。
#[cfg(windows)]
fn taskkill_path() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    PathBuf::from(root).join("System32").join("taskkill.exe")
}

/// fixture を `child`/`branch` mode で spawn し piped stdout を返す
/// (`windowsHide` 相当の `CREATE_NO_WINDOW` 付き)。stderr は
/// `Stdio::inherit()` — descendant panic が managed parent の capture
/// stderr → `details.stderr.text` → `dump_failure` へ乗るようにする。
fn spawn_fixture(mode: &str, arg_env: Option<&str>) -> (SpawnedChild, std::process::ChildStdout) {
    let mut command = std::process::Command::new(std::env::current_exe().expect("current exe"));
    command
        .args([FIXTURE, "--nocapture"])
        .env(MODE_ENV, mode)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());
    if let Some(flag) = arg_env {
        command.env(ARG_ENV, flag);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let child = command
        .spawn()
        .unwrap_or_else(|e| panic!("spawn_fixture({mode}): {e}"));
    let mut child = SpawnedChild(child);
    let stdout = child.0.stdout.take().expect("fixture stdout piped");
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
/// `pids_dir` を渡す caller(tree-family mode)のみ `env[PID_DIR_ENV]` を
/// 宣言し、outputs は `["pids.d/**"]` で統一される(`**` 末尾 →
/// `ResourceScope::Subtree`)。emit 伝達路を持たない task には pids.d
/// scope を宣言しない(touch/inspect/fail/output は `None`)。
fn fixture_task(mode: &str, args: &[&str], outputs: &[&str], pids_dir: Option<&Path>) -> Value {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut command = vec![json!(exe), json!(FIXTURE), json!("--nocapture")];
    command.extend(args.iter().map(|a| json!(a)));
    let mut env = Map::new();
    env.insert(MODE_ENV.to_string(), json!(mode));
    let outputs: Vec<Value> = match pids_dir {
        Some(dir) => {
            env.insert(PID_DIR_ENV.to_string(), json!(dir.to_string_lossy()));
            vec![json!("pids.d/**")]
        }
        None => outputs.iter().map(|o| json!(o)).collect(),
    };
    json!({
        "command": command,
        "inputs": [],
        "outputs": outputs,
        "env": Value::Object(env),
    })
}

/// authoring `process` section(`{tasks}` + section-level extra)→
/// unified resolved policy への seam 変換(段 2 synthesis → 段 3
/// resolve)。task executable の pin は `tool_with`/`tool_options` で行う。
fn policy(tasks: Value, extra: Value) -> ResolvedExecPolicy {
    let mut raw = Map::new();
    raw.insert("tasks".to_string(), tasks);
    if let Value::Object(map) = extra {
        raw.extend(map);
    }
    let doc = build_exec_input(Some(&Value::Object(raw)), None, &BTreeSet::new()).unwrap();
    resolve_exec_policy(Some(&Value::Object(doc)), &ExecDenyList::default()).unwrap()
}

/// fixed grant を ambient PATH で pin した `ExecToolOptions` を構築
/// する(D9 — task executable の pin は scope 構築責務。registry を
/// bypass する本経路では test 側が同 seam で pin する)。
fn tool_options(root: &str, tasks: Value, extra: Value) -> ExecToolOptions {
    let exec = policy(tasks, extra);
    let pins = pin_exec_grants(
        &exec,
        true,
        root,
        SymlinkPolicy::Deny,
        None,
        &ExecDenyList::default(),
    )
    .unwrap();
    ExecToolOptions {
        root: root.to_string(),
        symlink_policy: SymlinkPolicy::Deny,
        exec,
        pins,
        on_timeout: None,
        denylist: None,
        exec_deny: ExecDenyList::default(),
        write_scope: None,
        route_context: None,
    }
}

fn tool_with(root: &str, tasks: Value, extra: Value) -> ProcessTool {
    create_process_tool(tool_options(root, tasks, extra))
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

// ---------------------------------------------------------------------------
// Deterministic wait/verify + failure diagnostics(issue #180、design D4-D7)
// ---------------------------------------------------------------------------

/// wait/verify 系の failure context。`wait_until` の budget/poll と
/// `dump_failure`/`diag_fail` が採取する mode・elapsed・emit 状態を保持する。
struct WaitCtx {
    mode: &'static str,
    what: String,
    budget: Duration,
    poll: Duration,
    started: Instant,
    pids_dir: Option<PathBuf>,
    keys: &'static [&'static str],
}

/// 非 panic 版 wait — 満了時 `false` を返すだけで panic しない(panic 版
/// `wait_for`/`wait_emitted` は作らない — design D5)。失敗は caller が
/// `dump_failure` → `diag_fail` へ送る。`select!` 内外を問わず全 wait が
/// 同じ primitive を使う。
async fn wait_until(ctx: &WaitCtx, mut cond: impl FnMut() -> bool) -> bool {
    while !cond() {
        if ctx.started.elapsed() > ctx.budget {
            return false;
        }
        tokio::time::sleep(ctx.poll).await;
    }
    true
}

/// `pids.d/{key}` の存在かつ parseable な `{key: pid}` 集合。
/// atomic emit のため exists→parse の TOCTOU は構造的に存在しない
/// (未完備 file は rename 前の `.tmp` で観測されない)。
fn emitted_pids(dir: &Path, keys: &[&'static str]) -> BTreeMap<&'static str, u32> {
    let mut emitted = BTreeMap::new();
    for &key in keys {
        if let Ok(text) = std::fs::read_to_string(dir.join(key)) {
            if let Ok(pid) = text.trim().parse::<u32>() {
                emitted.insert(key, pid);
            }
        }
    }
    emitted
}

/// 全 key parseable か `grace` 経過まで `wait_until` する straggler 回収。
/// 満了しても panic せず、その時点で parseable な集合を返す
/// (emit floor の判定は caller)。
async fn collect_emitted(
    dir: &Path,
    keys: &'static [&'static str],
    grace: Duration,
) -> BTreeMap<&'static str, u32> {
    let ctx = WaitCtx {
        mode: "collect",
        what: format!("collect emitted {keys:?}"),
        budget: grace,
        poll: FILE_POLL,
        started: Instant::now(),
        pids_dir: Some(dir.to_path_buf()),
        keys,
    };
    let _ = wait_until(&ctx, || emitted_pids(dir, keys).len() == keys.len()).await;
    emitted_pids(dir, keys)
}

/// unfiltered `tasklist /NH /FO CSV` を 1 回だけ spawn した pid→image
/// snapshot。filter 併記は AND 評価で常に空を返すため使わない。
/// parse 規則: 各行は `"` prefix を要求して `","` で split、field index 1
/// が bare PID(field 0 = image name — `kill_emitted` の image 照合に
/// 共用。memory 列の `"12,345 K"` は `","` を含まないため split は安全)。
/// nonconforming output・helper spawn 失敗は `None`(callers が
/// conservative に全員 alive 扱い — wait budget で bound され
/// vacuous-pass にはならない)。
#[cfg(windows)]
fn tasklist_snapshot() -> Option<BTreeMap<u32, String>> {
    let out = std::process::Command::new("tasklist")
        .args(["/NH", "/FO", "CSV"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut snap = BTreeMap::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with('"') {
            return None;
        }
        let fields: Vec<&str> = line.split("\",\"").collect();
        if fields.len() < 2 {
            return None;
        }
        let Ok(pid) = fields[1].trim_matches('"').parse::<u32>() else {
            return None;
        };
        snap.insert(pid, fields[0].trim_start_matches('"').to_string());
    }
    Some(snap)
}

/// `process.kill(pid, 0)` + `/proc` zombie 判定相当の unix liveness。
#[cfg(unix)]
fn unix_live(pid: u32) -> bool {
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

/// `want` 集合のうち現在稼働中の pid を返す。
/// Windows は poll ごとに 1 回だけ snapshot を取り client 側で照合する
/// (R5 self-load 削減)。unix は per-pid `kill(pid,0)` + zombie を畳む。
fn live_pids(want: &BTreeSet<u32>) -> BTreeSet<u32> {
    #[cfg(windows)]
    {
        match tasklist_snapshot() {
            Some(snap) => want
                .iter()
                .copied()
                .filter(|p| snap.contains_key(p))
                .collect(),
            // helper 失敗・nonconforming output は conservative に全員 alive。
            None => want.clone(),
        }
    }
    #[cfg(unix)]
    {
        want.iter().copied().filter(|&pid| unix_live(pid)).collect()
    }
}

/// `emitted` 集合の全 pid 消滅を `wait_until` で待つ(集合 budget
/// `ctx.budget`・poll 100ms 系)。残存なら live pid 一覧付きで `diag_fail`。
/// emit 0 件は no-op(0-emit 受理は per-mode emit floor で制限済み)。
async fn verify_emitted_dead(ctx: &WaitCtx, emitted: &BTreeMap<&'static str, u32>) {
    let want: BTreeSet<u32> = emitted.values().copied().collect();
    if want.is_empty() {
        return;
    }
    if !wait_until(ctx, || live_pids(&want).is_empty()).await {
        eprintln!("[{}] live pids remaining: {:?}", ctx.mode, live_pids(&want));
        dump_failure(None, ctx);
        diag_fail(ctx);
    }
}

/// test binary 自身(`current_exe`)の filename — emit 済み pid の image
/// 照合に使う(PID reuse で無関係 process に化けた pid を殺さない)。
fn self_image_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

/// unix kill 対象の image 照合。linux は `/proc/<pid>/exe` link filename で
/// `current_exe` と照合、oracle が無い(他 unix・read 失敗)ときは false
/// で fail-closed(group kill の誤爆を防ぐ)。
#[cfg(unix)]
fn unix_image_matches(pid: u32, image: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_link(format!("/proc/{pid}/exe"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .is_some_and(|name| name == image)
    }
    #[cfg(not(target_os = "linux"))]
    {
        // `/proc` が無い unix には安価な image oracle が無い —
        // emitted ∧ live に限定した上で spec residual (c) として受容する。
        let _ = (pid, image);
        true
    }
}

/// emit 済み pid の best-effort 同期 kill(panic/give-up 経路の最終防衛)。
/// pids.d 不存在・parse 不能は tolerate し対象 0 件で return。kill 対象は
/// live ∩ emitted ∧ `current_exe` filename image 一致に限定する。
/// Windows は per-pid `taskkill /PID <pid> /T /F`(subtree)、unix は
/// `parent` key があれば `kill(-(pid), SIGKILL)` の group kill(managed
/// parent は `process_group(0)` の leader — spawn.rs:163-168)+ 他 emitted
/// pid への `kill(pid, SIGKILL)` belt。
fn kill_emitted(dir: &Path) {
    let emitted = emitted_pids(dir, ALL_KEYS);
    if emitted.is_empty() {
        return;
    }
    let image = self_image_name();
    #[cfg(windows)]
    {
        let Some(snap) = tasklist_snapshot() else {
            return;
        };
        for &pid in emitted.values() {
            let Some(img) = snap.get(&pid) else {
                continue;
            };
            if !img.eq_ignore_ascii_case(&image) {
                continue;
            }
            let _ = std::process::Command::new(taskkill_path())
                .args([
                    "/PID".to_string(),
                    pid.to_string(),
                    "/T".to_string(),
                    "/F".to_string(),
                ])
                .output();
        }
    }
    #[cfg(unix)]
    {
        let targets: Vec<u32> = emitted
            .values()
            .copied()
            .filter(|&pid| unix_live(pid) && unix_image_matches(pid, &image))
            .collect();
        if let Some(&parent) = emitted.get("parent") {
            if targets.contains(&parent) {
                unsafe {
                    libc::kill(-(parent as i32), libc::SIGKILL);
                }
            }
        }
        for pid in targets {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }
}

/// failure diagnostics の採取(D7)。error の `source()` chain・
/// `find_cheaper_error` の details pretty JSON・ctx 項目(mode/what/
/// budget/poll/elapsed/pids.d 状態)を stderr へ出す。
/// error を持たない pure-wait failure は `err: None` で同じ経路を通る。
fn dump_failure(err: Option<&ToolError>, ctx: &WaitCtx) {
    eprintln!("=== proc_process failure diagnostics ===");
    eprintln!("mode: {}", ctx.mode);
    eprintln!("what: {}", ctx.what);
    eprintln!(
        "budget: {:?} poll: {:?} elapsed: {:?}",
        ctx.budget,
        ctx.poll,
        ctx.started.elapsed()
    );
    if let Some(dir) = &ctx.pids_dir {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let names: Vec<String> = entries
                .filter_map(|e| e.ok().map(|x| x.file_name().to_string_lossy().into_owned()))
                .collect();
            eprintln!("pids.d entries: {names:?}");
        }
        eprintln!("emitted: {:?}", emitted_pids(dir, ctx.keys));
    }
    if let Some(err) = err {
        eprintln!("error: {err}");
        let mut depth = 0;
        let mut source = err.source();
        while let Some(e) = source {
            eprintln!("  source[{depth}]: {e}");
            source = e.source();
            depth += 1;
        }
        if let Some(cheaper) = find_cheaper_error(err.as_ref()) {
            eprintln!("cheaper code: {}", cheaper.code().as_str());
            if let Some(details) = cheaper.details() {
                match serde_json::to_string_pretty(&Value::Object(details.clone())) {
                    Ok(text) => eprintln!("details: {text}"),
                    Err(_) => eprintln!("details: {details:?}"),
                }
            }
        }
    }
    eprintln!("=== end diagnostics ===");
}

/// 全 failure 出口の panic(`-> !`)。panic には必ず `dump_failure` の
/// stderr 採取が先行する。
fn diag_fail(ctx: &WaitCtx) -> ! {
    panic!(
        "[{}] {}: timed out after {:?} (poll {:?}); elapsed {:?}",
        ctx.mode,
        ctx.what,
        ctx.budget,
        ctx.poll,
        ctx.started.elapsed()
    );
}

/// `process_tool_definition(options)` の `(def.execute)(input, signal)` が
/// 返す 'static future を `tokio::spawn` した JoinHandle +
/// `AbortController` + pids.d 観測 context の guard(registry が使う
/// canonical `CanonicalExecute` seam — process/mod.rs:166-181 — と同一
/// 経路。`tool.execute(&input, ..)` の `&self`/`&Value` borrow は
/// `'static` を満たさず spawn 不能)。
/// `armed` のまま drop されると(panic/unwind・give-up 未処理)
/// `controller.abort(teardown)` + 同期 `kill_emitted` を行う。
/// `handle.abort()` は **呼ばない** — JoinHandle abort は future を次
/// yield で drop するだけで production cleanup は走らず leak を悪化させる。
/// `TempDir` は `TaskRun` より先に宣言すること(`kill_emitted` が pids.d
/// を読めるうちに guard を発火させる宣言順ルール)。
struct TaskRun {
    mode: &'static str,
    pids_dir: PathBuf,
    keys: &'static [&'static str],
    controller: AbortController,
    handle: tokio::task::JoinHandle<Result<CanonicalToolResult, ToolError>>,
    armed: bool,
}

impl TaskRun {
    fn spawn(
        mode: &'static str,
        options: ExecToolOptions,
        input: Value,
        pids_dir: &Path,
        keys: &'static [&'static str],
    ) -> Self {
        let def = process_tool_definition(options);
        let controller = AbortController::new();
        let handle = tokio::spawn((def.execute)(input, Some(controller.signal())));
        Self {
            mode,
            pids_dir: pids_dir.to_path_buf(),
            keys,
            controller,
            handle,
            armed: true,
        }
    }

    fn ctx(&self, what: impl Into<String>, budget: Duration, poll: Duration) -> WaitCtx {
        WaitCtx {
            mode: self.mode,
            what: what.into(),
            budget,
            poll,
            started: Instant::now(),
            pids_dir: Some(self.pids_dir.clone()),
            keys: self.keys,
        }
    }

    /// `controller.abort(AbortReason::from(CheaperError(Aborted, reason)))`。
    fn abort(&self, reason: &str) {
        self.controller.abort(AbortReason::from(CheaperError::new(
            ErrorCode::Aborted,
            reason,
        )));
    }

    /// `timeout(SETTLE_BUDGET, &mut handle)` — task timeout + cleanup 連鎖を
    /// 上回る wall-clock bound。JoinError は `ToolError` ではないため
    /// `err: None` で diag。超過時は `abort_and_drain` → `diag_fail`。
    async fn settle(&mut self) -> Result<CanonicalToolResult, ToolError> {
        let ctx = self.ctx("settle", SETTLE_BUDGET, FILE_POLL);
        match tokio::time::timeout(SETTLE_BUDGET, &mut self.handle).await {
            Ok(Ok(result)) => result,
            Ok(Err(join)) => {
                eprintln!("[{}] join error: {join}", self.mode);
                dump_failure(None, &ctx);
                diag_fail(&ctx);
            }
            Err(_) => {
                self.abort_and_drain("settle budget").await;
                dump_failure(None, &ctx);
                diag_fail(&ctx);
            }
        }
    }

    /// 正式な give-up 経路 — `controller.abort` → `timeout(DRAIN_BUDGET,
    /// &mut handle)` で runtime 生存中に production の abort→cleanup
    /// (tree kill)を完走させる。DRAIN_BUDGET 超過の give-up branch は
    /// `kill_emitted` + stderr note + `disarm`(guard の二重 kill 防止)の
    /// 上で `diag_fail`。
    async fn abort_and_drain(&mut self, why: &str) {
        let ctx = self.ctx(format!("abort_and_drain({why})"), DRAIN_BUDGET, FILE_POLL);
        self.abort(why);
        if tokio::time::timeout(DRAIN_BUDGET, &mut self.handle)
            .await
            .is_err()
        {
            kill_emitted(&self.pids_dir);
            eprintln!(
                "[{}] abort_and_drain({why}): drain budget exceeded; emitted pids killed",
                self.mode
            );
            self.disarm();
            dump_failure(None, &ctx);
            diag_fail(&ctx);
        }
    }

    /// `armed = false` — `abort_and_drain` give-up branch と test の
    /// post-resolution verification pass 後に呼び、完走済み経路での
    /// 二重 kill を防ぐ。
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TaskRun {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.controller.abort(AbortReason::from(CheaperError::new(
            ErrorCode::Aborted,
            "test teardown",
        )));
        kill_emitted(&self.pids_dir);
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
    let mut task = fixture_task("inspect", &["literal space", "*.txt"], &[], None);
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

/// PATH を fixture binary のディレクトリだけにした nested test process
/// で bare-name → grant pin → spawn を検証する。親 test process の PATH
/// は変更しない。
#[tokio::test]
async fn node_task_resolves_via_path() {
    let fixture_dir = Path::new(DIFF_CHILD_BIN)
        .parent()
        .expect("diff fixture binary has a parent directory");
    match std::env::var(PATH_REEXEC_ENV) {
        Ok(marker) => {
            assert_eq!(marker, "1", "invalid controlled-PATH re-exec marker");
            assert_eq!(
                std::env::var_os("PATH").as_deref(),
                Some(fixture_dir.as_os_str()),
                "re-executed test must use only the fixture binary directory on PATH",
            );
        }
        Err(std::env::VarError::NotPresent) => {
            let output = std::process::Command::new(
                std::env::current_exe().expect("current test executable"),
            )
            .args(["--exact", "node_task_resolves_via_path", "--nocapture"])
            .env(PATH_REEXEC_ENV, "1")
            .env("PATH", fixture_dir)
            .output()
            .expect("re-execute node_task_resolves_via_path with controlled PATH");
            assert!(
                output.status.success(),
                "controlled-PATH child failed: status={}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            return;
        }
        Err(error) => panic!("invalid controlled-PATH re-exec marker: {error}"),
    }

    let dir = TempDir::new();
    let tool = tool_with(
        &dir.root(),
        json!({"script": {
            "command": ["diff_fixture_child"],
            "inputs": [],
            "outputs": [],
            "env": {"CHEAPER_DIFF_CHILD_MODE": "bare"},
        }}),
        json!({}),
    );
    let result = tool
        .execute(&json!({"task": "script"}), None)
        .await
        .expect("bare diff_fixture_child resolves, pins, and spawns");
    let details = result.details.expect("process result details");
    assert_eq!(details["state"], json!("completed"));
    assert_eq!(details["exitCode"], json!(0));
    assert_eq!(details["stdout"]["text"], json!("bare node\n"));
}

#[tokio::test]
async fn fails_nonzero_exit_with_bounded_structured_diagnostics() {
    let dir = TempDir::new();
    let tool = tool_with(
        &dir.root(),
        json!({"fail": fixture_task("fail", &[], &[], None)}),
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
            json!({"output": fixture_task("output", &[], &[], None)}),
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
    // TempDir は TaskRun より先に宣言する。
    let dir = TempDir::new();
    let pids_dir = dir.join("pids.d");
    let options = tool_options(
        &dir.root(),
        json!({"stubborn": fixture_task("stubborn", &[], &[], Some(&pids_dir))}),
        json!({"timeout": "500ms", "killGrace": "100ms"}),
    );
    let mut run = TaskRun::spawn(
        "stubborn",
        options,
        json!({"task": "stubborn"}),
        &pids_dir,
        &["parent"],
    );
    let result = run.settle().await;
    // 受理 resolution は `Err` code=Timeout のみ(他は diag_fail)。
    match &result {
        Err(err) if code(err) == Some(ErrorCode::Timeout) => {}
        other => {
            eprintln!("[stubborn] unexpected resolution: {other:?}");
            let ctx = run.ctx("timeout resolution", SETTLE_BUDGET, FILE_POLL);
            dump_failure(other.as_ref().err(), &ctx);
            diag_fail(&ctx);
        }
    }
    // 500ms mode は genuine pre-emit kill window があるため 0-emit も受理
    // — emit 済み分のみ消滅を検証する。
    let emitted = collect_emitted(&pids_dir, &["parent"], POST_RESOLVE_EMIT_GRACE).await;
    let dead_ctx = run.ctx("emitted pids dead", DEAD_BUDGET, PID_POLL);
    verify_emitted_dead(&dead_ctx, &emitted).await;
    run.disarm();
}

#[tokio::test]
async fn abort_kills_managed_parent_and_descendant_and_preserves_reason() {
    let dir = TempDir::new();
    let pids_dir = dir.join("pids.d");
    let options = tool_options(
        &dir.root(),
        json!({"tree": fixture_task("tree", &[], &[], Some(&pids_dir))}),
        json!({"killGrace": "200ms"}),
    );
    let mut run = TaskRun::spawn(
        "tree",
        options,
        json!({"task": "tree"}),
        &pids_dir,
        &["parent", "child"],
    );
    // `pids.d/{parent,child}` 全件必須の emit 待ちを handle と select! する
    // (select! 内では非 panic の `wait_until` のみ使う)。
    let emit_ctx = run.ctx("emit wait parent+child", EMIT_WAIT_BUDGET, FILE_POLL);
    let mut early = None;
    let mut stalled = false;
    tokio::select! {
        done = wait_until(&emit_ctx, || {
            emitted_pids(&pids_dir, &["parent", "child"]).len() == 2
        }) => {
            if !done {
                stalled = true;
            }
        }
        r = &mut run.handle => {
            early = Some(r);
        }
    }
    if let Some(early) = early {
        // abort test に timeout は armed されていない — 早期解決は異常。
        eprintln!("[tree] early resolution before abort: {early:?}");
        let err = match &early {
            Ok(Err(err)) => Some(err),
            _ => None,
        };
        dump_failure(err, &emit_ctx);
        diag_fail(&emit_ctx);
    }
    if stalled {
        run.abort_and_drain("emit stall").await;
        dump_failure(None, &emit_ctx);
        diag_fail(&emit_ctx);
    }
    run.abort("stop managed tree");
    let result = run.settle().await;
    match &result {
        Err(err) if err.to_string() == "stop managed tree" => {}
        other => {
            eprintln!("[tree] unexpected resolution: {other:?}");
            let ctx = run.ctx("abort resolution", SETTLE_BUDGET, FILE_POLL);
            dump_failure(other.as_ref().err(), &ctx);
            diag_fail(&ctx);
        }
    }
    let emitted = collect_emitted(&pids_dir, &["parent", "child"], POST_RESOLVE_EMIT_GRACE).await;
    if emitted.len() != 2 {
        let ctx = run.ctx("parent+child emitted", POST_RESOLVE_EMIT_GRACE, FILE_POLL);
        dump_failure(result.as_ref().err(), &ctx);
        diag_fail(&ctx);
    }
    let dead_ctx = run.ctx("emitted pids dead", DEAD_BUDGET, PID_POLL);
    verify_emitted_dead(&dead_ctx, &emitted).await;
    run.disarm();
}

#[tokio::test]
async fn cleans_live_descendants_on_exit_and_failure_and_abort_and_timeout() {
    for (mode, expected) in [
        ("exit-tree", "ok"),
        ("fail-tree", "fail"),
        ("deep-tree", "abort"),
        ("deep-stubborn", "timeout"),
    ] {
        // TempDir は TaskRun より先に宣言する(宣言順ルール — TaskRun が
        // 先に drop され、pids.d が読める間に kill_emitted を発火させる)。
        let dir = TempDir::new();
        let pids_dir = dir.join("pids.d");
        // timeout 期待 mode(deep-stubborn)のみ kill 検証に必要な最小値
        // "2s"。他は fixture の観測可能な全 lifetime(3 段 spawn chain +
        // emit + exit/park + 解決)を上回る "60s" — timeout が lifetime と
        // 同じ clock で競合しない(R1 の構造 race を消す)。
        let timeout = if mode == "deep-stubborn" { "2s" } else { "60s" };
        let options = tool_options(
            &dir.root(),
            json!({"tree": fixture_task(mode, &[], &[], Some(&pids_dir))}),
            json!({"timeout": timeout, "killGrace": "100ms"}),
        );
        let mut run = TaskRun::spawn(mode, options, json!({"task": "tree"}), &pids_dir, ALL_KEYS);
        let result = if expected == "abort" {
            // emit 全件(parent+child+grandchild、EMIT_WAIT_BUDGET)と
            // handle を select! — select! 内では非 panic の `wait_until`
            // のみ使う。stall は `abort_and_drain` の正式 give-up 経路。
            let emit_ctx = run.ctx(
                "emit wait parent+child+grandchild",
                EMIT_WAIT_BUDGET,
                FILE_POLL,
            );
            let mut early = None;
            let mut stalled = false;
            tokio::select! {
                done = wait_until(&emit_ctx, || {
                    emitted_pids(&pids_dir, ALL_KEYS).len() == ALL_KEYS.len()
                }) => {
                    if !done {
                        stalled = true;
                    }
                }
                r = &mut run.handle => {
                    early = Some(r);
                }
            }
            if let Some(early) = early {
                match early {
                    Ok(result) => result,
                    Err(join) => {
                        eprintln!("[{mode}] join error before abort: {join}");
                        dump_failure(None, &emit_ctx);
                        diag_fail(&emit_ctx);
                    }
                }
            } else if stalled {
                run.abort_and_drain("emit stall").await;
                dump_failure(None, &emit_ctx);
                diag_fail(&emit_ctx);
            } else {
                run.abort("outer tree cancellation");
                run.settle().await
            }
        } else {
            run.settle().await
        };
        // outcome 分類表(D3): 「期待 outcome OR code=Timeout」の二択。
        // それ以外の Err・JoinError・不整合 details は diag_fail。
        // fail-tree は cleanup-failed arm(monitor.rs:319-327)が同一
        // details を運ぶため message=="managed process failed" 照合が必須。
        let class_ctx = run.ctx("resolution classification", SETTLE_BUDGET, FILE_POLL);
        let natural = match (expected, &result) {
            ("ok", Ok(res)) => res
                .details
                .as_ref()
                .is_some_and(|d| d["state"] == json!("completed") && d["exitCode"] == json!(0)),
            ("fail", Err(err)) => find_cheaper_error(err.as_ref()).is_some_and(|ce| {
                ce.code() == ErrorCode::ToolExecutionFailure
                    && ce.message() == "managed process failed"
                    && ce.details().is_some_and(|d| d["exitCode"] == json!(7))
            }),
            ("abort", Err(err)) => err.to_string() == "outer tree cancellation",
            _ => false,
        };
        let timeout_resolution =
            matches!(&result, Err(err) if code(err) == Some(ErrorCode::Timeout));
        let natural_resolution = if natural {
            true
        } else if timeout_resolution {
            false
        } else {
            eprintln!("[{mode}] unexpected resolution: {result:?}");
            dump_failure(result.as_ref().err(), &class_ctx);
            diag_fail(&class_ctx);
        };
        // per-mode emit floor: 自然解決(Ok/ToolExecutionFailure/abort
        // reason 一致)なら全 expected key、60s-armed mode(exit/fail/
        // deep-tree)の Timeout 解決は emit ≥1、0-emit Timeout の受理は
        // deep-stubborn(2s — genuine pre-emit kill window)のみ。
        let emitted = collect_emitted(&pids_dir, ALL_KEYS, POST_RESOLVE_EMIT_GRACE).await;
        if natural_resolution && emitted.len() != ALL_KEYS.len() {
            let ctx = run.ctx(
                "natural resolution requires all keys emitted",
                POST_RESOLVE_EMIT_GRACE,
                FILE_POLL,
            );
            dump_failure(result.as_ref().err(), &ctx);
            diag_fail(&ctx);
        }
        if !natural_resolution && mode != "deep-stubborn" && emitted.is_empty() {
            let ctx = run.ctx(
                "60s-armed timeout requires at least one emit",
                POST_RESOLVE_EMIT_GRACE,
                FILE_POLL,
            );
            dump_failure(result.as_ref().err(), &ctx);
            diag_fail(&ctx);
        }
        // 全 resolution 経路で emit 済み全 pid の消滅を検証し、pass 後に
        // guard を disarm する。
        let dead_ctx = run.ctx("emitted pids dead", DEAD_BUDGET, PID_POLL);
        verify_emitted_dead(&dead_ctx, &emitted).await;
        run.disarm();
    }
}

#[tokio::test]
async fn touch_with_matching_write_scope_succeeds() {
    let dir = TempDir::new();
    let root = dir.root();
    let marker = dir.join("out").to_string_lossy().into_owned();
    let exec = policy(
        json!({"check": fixture_task("touch", &[&marker], &["out"], None)}),
        json!({}),
    );
    let pins = pin_exec_grants(
        &exec,
        true,
        &root,
        SymlinkPolicy::Deny,
        None,
        &ExecDenyList::default(),
    )
    .unwrap();
    let tool = create_process_tool(ExecToolOptions {
        root,
        symlink_policy: SymlinkPolicy::Deny,
        exec,
        pins,
        on_timeout: None,
        denylist: None,
        exec_deny: ExecDenyList::default(),
        write_scope: Some(std::collections::HashSet::from(["out".to_string()])),
        route_context: None,
    });
    tool.execute(&json!({"task": "check"}), None).await.unwrap();
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "generated");
}

#[cfg(unix)]
#[tokio::test]
async fn allows_sigterm_exit_within_grace_before_force_kill() {
    let dir = TempDir::new();
    let pids_dir = dir.join("pids.d");
    let options = tool_options(
        &dir.root(),
        json!({"graceful": fixture_task("graceful", &[], &[], Some(&pids_dir))}),
        json!({"timeout": "500ms", "killGrace": "10s"}),
    );
    let mut run = TaskRun::spawn(
        "graceful",
        options,
        json!({"task": "graceful"}),
        &pids_dir,
        &["parent"],
    );
    let result = run.settle().await;
    // 受理 resolution は `Err` code=Timeout のみ。
    match &result {
        Err(err) if code(err) == Some(ErrorCode::Timeout) => {}
        other => {
            eprintln!("[graceful] unexpected resolution: {other:?}");
            let ctx = run.ctx("timeout resolution", SETTLE_BUDGET, FILE_POLL);
            dump_failure(other.as_ref().err(), &ctx);
            diag_fail(&ctx);
        }
    }
    let emitted = collect_emitted(&pids_dir, &["parent"], POST_RESOLVE_EMIT_GRACE).await;
    if pids_dir.join("parent.armed").exists() {
        // armed なのに `.terminated` が出ない = real failure。
        let marker_ctx = run.ctx("parent.terminated marker", DEAD_BUDGET, FILE_POLL);
        if !wait_until(&marker_ctx, || pids_dir.join("parent.terminated").exists()).await {
            dump_failure(result.as_ref().err(), &marker_ctx);
            diag_fail(&marker_ctx);
        }
        let terminated = std::fs::read_to_string(pids_dir.join("parent.terminated"))
            .expect("parent.terminated readable after marker wait");
        if terminated.trim() != "graceful" {
            let ctx = run.ctx("parent.terminated content", DEAD_BUDGET, FILE_POLL);
            dump_failure(result.as_ref().err(), &ctx);
            diag_fail(&ctx);
        }
    } else {
        // `.armed` 無 — kill-before-arming は acceptable early outcome。
        eprintln!("[graceful] killed before signal arming — acceptable early outcome");
    }
    let dead_ctx = run.ctx("emitted pids dead", DEAD_BUDGET, PID_POLL);
    verify_emitted_dead(&dead_ctx, &emitted).await;
    run.disarm();
}
