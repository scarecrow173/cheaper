//! Rust-only regression harness — `tests/differential/` の後継。
//!
//! `cases/*.json` の 1 case を Rust binary(`cheaper`)のみで replay し、
//! stdout(envelope または raw bytes)と exit code を `normalize` 適用後に
//! `expected/<case>.json` の固定 snapshot と比較する。snapshot は Phase 6
//! Commit C の TS 撤去直前に、最終 differential green 状態の Rust 側
//! normalized 出力から capture したもの(`CHEAPER_REGRESSION_CAPTURE=1` で
//! 再取得可能)。
//!
//! fixture 経路: `CHEAPER_TEST_FAKE_STREAM` → RunFixture JSON path
//! (`fake_stream.rs`)。TS adapter(`adapter.mjs`)は撤去済み。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use cheaper_runtime::dev::fixture::RunFixture;
use serde::Deserialize;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// case schema(`tests/regression/cases/*.json` — differential と同一 DSL)
// ---------------------------------------------------------------------------

/// 1 case の宣言。`fixture` 有無で run 系/argv 系を分ける。
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Case {
    /// 診断表示名(省略時は file stem)。
    pub name: Option<String>,
    /// 実行 argv(binary 名を除く)。`fixture` 有りの場合は
    /// `run <worker> --input - --model <explicit>` を自動構成し
    /// `extra_args` を末尾に足す。
    pub argv: Vec<String>,
    /// `run` argv 末尾に付ける追加 args(`--timeout` 等)。
    pub extra_args: Vec<String>,
    /// stdin payload。文字列はそのまま、それ以外の JSON 値は compact
    /// serialize して流す。省略時は空 stdin。
    pub stdin: Option<Value>,
    /// workspace file 群(`{"dir/x.txt": "…"}` or `{"repeat":[unit,n]}`)。
    pub files: BTreeMap<String, Value>,
    /// config dir seed(usage.jsonl / 任意 file)。
    pub config: ConfigSeed,
    /// RunFixture JSON — 存在すれば fake-stream run case。
    pub fixture: Option<Value>,
    /// stdout 比較モード。`"envelope"`(既定)は JSON normalize 比較、
    /// `"raw"` は byte 完全一致(hook/version/help 等の non-envelope 系)。
    pub mode: CaseMode,
    /// stderr も比較する(path scrub 適用後)。hook の deny 応答は
    /// stderr が contract の一部(windsurf deny = guidance + exit 2)の
    /// ため、hook case で opt-in する。既定は比較しない。
    pub compare_stderr: bool,
    /// TS oracle 時代の既知 divergence pin — Rust-only replay では意味を
    /// 持たないが、divergence の履歴 note として case file に残す。
    pub expect_divergence: Option<String>,
    /// 子プロセスの watchdog timeout(ms)。
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseMode {
    #[default]
    Envelope,
    Raw,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ConfigSeed {
    /// `<cfg>/usage.jsonl` へ書き込む JSONL 行(Value 列)。
    pub usage_jsonl: Vec<Value>,
    /// config dir 直下の任意 file(`credentials.json` 等)。
    pub files: BTreeMap<String, Value>,
}

// ---------------------------------------------------------------------------
// case layout / workspace materialization
// ---------------------------------------------------------------------------

/// 1 case 分の実行環境。
pub struct SideDir {
    pub ws: PathBuf,
    pub cfg: PathBuf,
    pub home: PathBuf,
}

pub struct CaseEnv {
    pub side: SideDir,
    /// fixture JSON の実 path(`expect`/`name` 等の不要 field は strip)。
    pub fixture_path: Option<PathBuf>,
    /// fixture `env` overlay — child env にそのまま流す。
    pub fixture_env: BTreeMap<String, String>,
    _tmp: tempfile::TempDir,
}

fn write_files(dir: &Path, files: &BTreeMap<String, Value>) {
    for (rel, content) in files {
        let p = dir.join(rel.split('/').collect::<PathBuf>());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        if let Some(repeat) = content.get("repeat").and_then(Value::as_array) {
            let unit = repeat[0].as_str().unwrap_or_default();
            let times = repeat[1].as_u64().unwrap_or(0) as usize;
            std::fs::write(&p, unit.repeat(times)).unwrap();
        } else {
            std::fs::write(&p, content.as_str().unwrap_or_default()).unwrap();
        }
    }
}

/// fixture `policy` 節 → project `.cheaper/policy.yaml`。differential 時代に
/// TS binary が読む policy file を作るために導入した経路 — Rust-only 化後も
/// 同一配置を維持し、fixture `policy.apply_to` の overlay と同値で重ねる。
fn fixture_policy_yaml(fixture: &RunFixture) -> String {
    let p = &fixture.policy;
    let mut out = String::from("version: 1\n");
    if p.timeout_ms.is_some() || p.max_turns.is_some() || p.max_tool_calls.is_some() {
        out.push_str("execution:\n");
        if let Some(t) = p.timeout_ms {
            out.push_str(&format!("  timeout: {t}ms\n"));
        }
        if let Some(n) = p.max_turns {
            out.push_str(&format!("  maxTurns: {n}\n"));
        }
        if let Some(n) = p.max_tool_calls {
            out.push_str(&format!("  maxToolCalls: {n}\n"));
        }
    }
    if let Some(n) = p.repair_attempts {
        out.push_str(&format!("output:\n  repairAttempts: {n}\n"));
    }
    if let Some(n) = p.transient_attempts {
        out.push_str(&format!("retry:\n  transientAttempts: {n}\n"));
    }
    out
}

fn materialize_side(dir: &Path, case: &Case, fixture: Option<&RunFixture>) -> SideDir {
    let side = SideDir {
        ws: dir.join("ws"),
        cfg: dir.join("cfg"),
        home: dir.join("home"),
    };
    std::fs::create_dir_all(&side.ws).unwrap();
    std::fs::create_dir_all(&side.cfg).unwrap();
    std::fs::create_dir_all(&side.home).unwrap();
    write_files(&side.ws, &case.files);
    write_files(&side.cfg, &case.config.files);
    if !case.config.usage_jsonl.is_empty() {
        let mut text = String::new();
        for line in &case.config.usage_jsonl {
            text.push_str(&serde_json::to_string(line).unwrap());
            text.push('\n');
        }
        std::fs::write(side.cfg.join("usage.jsonl"), text).unwrap();
    }
    if let Some(f) = fixture {
        // worker frontmatter を workspace の project workers dir へ —
        // `run <name>` が project → user → builtin の順で解決する。
        let md = f.worker_markdown();
        let workers = side.ws.join(".cheaper").join("workers");
        std::fs::create_dir_all(&workers).unwrap();
        std::fs::write(workers.join(format!("{}.md", f.worker.name)), md).unwrap();
        let yaml = fixture_policy_yaml(f);
        if yaml != "version: 1\n" {
            std::fs::write(side.ws.join(".cheaper").join("policy.yaml"), yaml).unwrap();
        }
    }
    side
}

/// case 用 temp dir を構築し、workspace + fixture JSON を配置する。
pub fn setup_case(name: &str, case: &Case) -> CaseEnv {
    let tmp = tempfile::Builder::new()
        .prefix(&format!("cheaper-regr-{}-", sanitize(name)))
        .tempdir()
        .unwrap();
    let fixture = case
        .fixture
        .as_ref()
        .map(|v| serde_json::from_value::<RunFixture>(v.clone()).expect("fixture parses"));
    let fixture_path = fixture.as_ref().map(|_| {
        let p = tmp.path().join("fixture.json");
        std::fs::write(
            &p,
            serde_json::to_string(case.fixture.as_ref().unwrap()).unwrap(),
        )
        .unwrap();
        p
    });
    let fixture_env = fixture.as_ref().map(|f| f.env.clone()).unwrap_or_default();
    CaseEnv {
        side: materialize_side(&tmp.path().join("run"), case, fixture.as_ref()),
        fixture_path,
        fixture_env,
        _tmp: tmp,
    }
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// run case の argv を fixture から構成する。
pub fn case_argv(case: &Case) -> Vec<String> {
    if let Some(fv) = &case.fixture {
        let f = serde_json::from_value::<RunFixture>(fv.clone()).expect("fixture parses");
        let mut argv = vec![
            "run".to_string(),
            f.worker.name.clone(),
            "--input".to_string(),
            "-".to_string(),
            "--model".to_string(),
            f.explicit_model(),
        ];
        argv.extend(case.extra_args.iter().cloned());
        argv
    } else {
        let mut argv = case.argv.clone();
        argv.extend(case.extra_args.iter().cloned());
        argv
    }
}

/// run case の stdin(fixture `input` → compact JSON)。
pub fn case_stdin(case: &Case) -> Vec<u8> {
    if let Some(stdin) = &case.stdin {
        return match stdin {
            Value::String(s) => s.clone().into_bytes(),
            other => serde_json::to_string(other).unwrap().into_bytes(),
        };
    }
    if let Some(fv) = &case.fixture {
        let f = serde_json::from_value::<RunFixture>(fv.clone()).expect("fixture parses");
        return serde_json::to_string(&f.input).unwrap().into_bytes();
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// process spawn
// ---------------------------------------------------------------------------

pub struct ProcOut {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

/// stdin を write して close、stdout/stderr を reader thread で回収し、
/// `timeout` 超過なら kill する子プロセス実行。
pub fn spawn_collect(
    program: &Path,
    args: &[String],
    cwd: &Path,
    env: &[(String, String)],
    stdin: &[u8],
    timeout: Duration,
) -> ProcOut {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child: Child = cmd.spawn().expect("spawn binary");

    // stdin write は別 thread(子が読まない場合の block 回避)。
    let mut child_stdin = child.stdin.take().unwrap();
    let stdin_bytes = stdin.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = child_stdin.write_all(&stdin_bytes);
        let _ = child_stdin.flush();
        drop(child_stdin);
    });

    let mut out_reader = child.stdout.take().unwrap();
    let mut err_reader = child.stderr.take().unwrap();
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut out_reader, &mut buf);
        buf
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut err_reader, &mut buf);
        buf
    });

    let start = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    timed_out = true;
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("wait failed: {e}"),
        }
    }
    let _ = writer.join();
    ProcOut {
        code: if timed_out {
            None
        } else {
            child.wait().ok().and_then(|s| s.code())
        },
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
        timed_out,
    }
}

// ---------------------------------------------------------------------------
// env / scrub roots
// ---------------------------------------------------------------------------

/// repository root(`crates/cheaper-cli` → 2 段上)。
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

/// 最小 env(ambient `CHEAPER_*`/`PI_*`/credential 系は全て落とし、
/// isolated workspace/config/home で実行する)。
fn base_env(side: &SideDir) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    for key in [
        "PATH",
        "SYSTEMROOT",
        "WINDIR",
        "TEMP",
        "TMP",
        "PATHEXT",
        "COMSPEC",
        "NUMBER_OF_PROCESSORS",
    ] {
        if let Some(v) = std::env::var_os(key) {
            env.push((key.to_string(), v.to_string_lossy().into_owned()));
        }
    }
    env.push(("HOME".to_string(), side.home.to_string_lossy().into_owned()));
    env.push((
        "USERPROFILE".to_string(),
        side.home.to_string_lossy().into_owned(),
    ));
    env.push(("NODE_ENV".to_string(), "test".to_string()));
    env.push(("PI_OFFLINE".to_string(), "1".to_string()));
    env.push((
        "CHEAPER_CONFIG_DIR".to_string(),
        side.cfg.to_string_lossy().into_owned(),
    ));
    env
}

/// child env(`CHEAPER_TEST_FAKE_STREAM` = fixture JSON path)。
pub fn run_env(case_env: &CaseEnv) -> Vec<(String, String)> {
    let mut env = base_env(&case_env.side);
    for (k, v) in &case_env.fixture_env {
        env.push((k.clone(), v.clone()));
    }
    if let Some(p) = &case_env.fixture_path {
        env.push((
            "CHEAPER_TEST_FAKE_STREAM".to_string(),
            p.to_string_lossy().into_owned(),
        ));
    }
    env
}

/// normalize 用の path→token 対。
pub fn scrub_roots(side: &SideDir, case_env: &CaseEnv) -> Vec<(String, String)> {
    let mut roots = vec![
        ("<WS>".to_string(), side.ws.to_string_lossy().into_owned()),
        ("<CFG>".to_string(), side.cfg.to_string_lossy().into_owned()),
        (
            "<HOME>".to_string(),
            side.home.to_string_lossy().into_owned(),
        ),
    ];
    if let Some(p) = &case_env.fixture_path {
        roots.push(("<FIXTURE>".to_string(), p.to_string_lossy().into_owned()));
        if let Some(parent) = p.parent() {
            roots.push(("<TMP>".to_string(), parent.to_string_lossy().into_owned()));
        }
    }
    // builtin worker dir(Rust は temp dir 展開 `cheaper-builtin-*`)+ repo
    // root + OS temp root — `details.searched[]` 等の実装依存 path を
    // token 化する。
    roots.push((
        "<REPO>".to_string(),
        repo_root().to_string_lossy().into_owned(),
    ));
    roots.push((
        "<OSTEMP>".to_string(),
        std::env::temp_dir().to_string_lossy().into_owned(),
    ));
    roots
}

/// test report 用の compact JSON 行(実行結果の診断保存用)。
pub fn describe(out: &ProcOut) -> Value {
    json!({
        "code": out.code,
        "timedOut": out.timed_out,
        "stdout": String::from_utf8_lossy(&out.stdout),
        "stderr": String::from_utf8_lossy(&out.stderr),
    })
}

/// case fixture への `expect` 混入回避 — fixture JSON の `expect` 節は
/// runtime fixture DSL 向けの field で、replay 比較は全 field を対象と
/// するため不要。混入した場合はここで取り除く。
pub fn strip_fixture_expect(v: &mut Value) {
    if let Some(o) = v.as_object_mut() {
        o.remove("expect");
    }
}
