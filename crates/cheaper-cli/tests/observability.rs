//! Observability foundation contract test(non-seam)—
//! TSPEC-034DWcP8v4SUuuui8hvhnT / DESIGN-034DV9ypzmxdAobGSJ6Cm6 の
//! test 戦略表に従い、file sink を assert する検証は全て
//! `CARGO_BIN_EXE_cheaper` + `env_clear` の subprocess isolation で実装
//! する(`tests/regression/harness.rs::spawn_collect` と同方式 —
//! `set_global_default` は process 当たり一度しか成功しないため、
//! in-process の `run_cli` 複数呼出しは repeat-init contract に乗り
//! test 毎の独立 subscriber を取れない)。
//!
//! in-process の `run_cli` は repeat-init contract 確認の 1 test のみ。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

mod common;
use common::{PipeStdin, VecSink};

use cheaper_cli::cli::{run_cli, CliHooks, CliIo};
use serde_json::Value;

// ---------------------------------------------------------------------------
// subprocess harness(`tests/regression/harness.rs` と同方式の最小版)
// ---------------------------------------------------------------------------

struct ProcOut {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn cheaper_bin() -> PathBuf {
    std::env::var("CARGO_BIN_EXE_cheaper")
        .or_else(|_| std::env::var("CHEAPER_REGRESSION_BIN"))
        .map(PathBuf::from)
        .expect("cheaper binary path (CARGO_BIN_EXE_cheaper)")
}

/// ambient `CHEAPER_*`/credential 系を全て落とした最小 env +
/// 注入 env(harness.rs::base_env/run_env と同一方針)。
fn base_env(home: &Path, cfg: &Path) -> Vec<(String, String)> {
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
    env.push(("HOME".to_string(), home.to_string_lossy().into_owned()));
    env.push((
        "USERPROFILE".to_string(),
        home.to_string_lossy().into_owned(),
    ));
    env.push(("PI_OFFLINE".to_string(), "1".to_string()));
    env.push((
        "CHEAPER_CONFIG_DIR".to_string(),
        cfg.to_string_lossy().into_owned(),
    ));
    env
}

fn spawn_collect(
    args: &[&str],
    cwd: &Path,
    env: &[(String, String)],
    stdin: &[u8],
    timeout: Duration,
) -> ProcOut {
    let mut cmd = Command::new(cheaper_bin());
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.env_clear();
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("spawn cheaper binary");

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
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                assert!(
                    start.elapsed() <= timeout,
                    "cheaper {args:?} timed out after {timeout:?}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => panic!("wait failed: {e}"),
        }
    }
    let _ = writer.join();
    ProcOut {
        code: child.wait().ok().and_then(|s| s.code()),
        stdout: out_thread.join().unwrap_or_default(),
        stderr: err_thread.join().unwrap_or_default(),
    }
}

/// 1 test 分の隔離環境 — workspace・config dir・home は全て tempdir。
struct Side {
    _tmp: tempfile::TempDir,
    ws: PathBuf,
    cfg: PathBuf,
    home: PathBuf,
}

impl Side {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("ws");
        let cfg = tmp.path().join("cfg");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        Self {
            _tmp: tmp,
            ws,
            cfg,
            home,
        }
    }

    /// child env(base + 注入 overlay)。
    fn env(&self, overlay: &[(&str, &str)]) -> Vec<(String, String)> {
        let mut env = base_env(&self.home, &self.cfg);
        for (k, v) in overlay {
            env.push(((*k).to_string(), (*v).to_string()));
        }
        env
    }

    fn spawn(&self, args: &[&str], overlay: &[(&str, &str)], stdin: &[u8]) -> ProcOut {
        spawn_collect(
            args,
            &self.ws,
            &self.env(overlay),
            stdin,
            Duration::from_secs(60),
        )
    }

    fn log_dir(&self) -> PathBuf {
        self.ws.join(".cheaper").join("logs")
    }

    /// `.cheaper/logs/cheaper.log.*` の file 名一覧(排序済み)。
    fn log_file_names(&self) -> Vec<String> {
        let dir = self.log_dir();
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| n.starts_with("cheaper.log"))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    /// 全 log file の行を連結して返す(file 無しなら空文字列)。
    fn log_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let dir = self.log_dir();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            let mut paths: Vec<PathBuf> = rd
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("cheaper.log"))
                        .unwrap_or(false)
                })
                .collect();
            paths.sort();
            for p in paths {
                let text = std::fs::read_to_string(&p).unwrap();
                lines.extend(text.lines().map(|l| l.to_string()));
            }
        }
        lines
    }
}

/// stdout の envelope exclusivity — 1 行の JSON object(`schemaVersion:1`)
/// で末尾改行のみ、logging 有効化でも 0 byte の追加。
fn assert_single_envelope_line(stdout: &[u8]) -> Value {
    let text = String::from_utf8_lossy(stdout);
    assert!(
        text.ends_with('\n'),
        "stdout lacks trailing newline: {text:?}"
    );
    let mut iter = text.lines();
    let first = iter.next().unwrap_or_default();
    assert!(iter.next().is_none(), "multi-line stdout: {text:?}");
    let env: Value = serde_json::from_str(first)
        .unwrap_or_else(|e| panic!("stdout is not one JSON envelope line: {e} ({text:?})"));
    assert_eq!(env["schemaVersion"], 1);
    env
}

// ---------------------------------------------------------------------------
// activation matrix: OFF / dual sink / format contract
// ---------------------------------------------------------------------------

#[test]
fn logging_off_creates_no_log_dir() {
    // Rule: activation matrix — 既定は OFF(subscriber 未 install・
    // `.cheaper/logs/` 未作成)。
    let side = Side::new();
    let out = side.spawn(&["usage"], &[], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    assert!(
        !side.log_dir().exists(),
        "default OFF must not create .cheaper/logs"
    );
    assert_single_envelope_line(&out.stdout);
}

#[test]
fn file_sink_and_stderr_dual_output() {
    // Rule: dual sink — 同一 event が file と stderr の両方に出る。
    let side = Side::new();
    let out = side.spawn(&["usage"], &[("CHEAPER_LOG", "debug")], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    assert_single_envelope_line(&out.stdout);

    let lines = side.log_lines();
    assert!(
        lines.iter().any(|l| l.contains("cheaper.cli.dispatch")),
        "file sink must contain cheaper.cli.dispatch span events: {lines:?}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cheaper.cli.dispatch"),
        "stderr sink must contain the same tracing events: {stderr:?}"
    );
}

#[test]
fn json_format_lines_parse() {
    // Rule: format contract — json は file/stderr の各行が parse 可能な
    // JSON object。span/level/target field が揃う。
    let side = Side::new();
    let out = side.spawn(
        &["usage"],
        &[("CHEAPER_LOG", "debug"), ("CHEAPER_LOG_FORMAT", "json")],
        b"",
    );
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    assert_single_envelope_line(&out.stdout);

    let lines = side.log_lines();
    assert!(!lines.is_empty(), "file sink must emit events");
    let mut saw_dispatch = false;
    for line in &lines {
        let v: Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("log line must parse as JSON: {e} ({line:?})"));
        assert!(v.get("level").is_some(), "JSON event needs level: {line:?}");
        assert!(
            v.get("target").is_some(),
            "JSON event needs target: {line:?}"
        );
        let span_names: Vec<&str> = v
            .get("spans")
            .and_then(Value::as_array)
            .map(|spans| spans.iter().filter_map(|s| s["name"].as_str()).collect())
            .unwrap_or_default();
        let close_name = v.get("span").and_then(|s| s["name"].as_str());
        if close_name == Some("cheaper.cli.dispatch")
            || span_names.contains(&"cheaper.cli.dispatch")
        {
            saw_dispatch = true;
        }
    }
    assert!(
        saw_dispatch,
        "cheaper.cli.dispatch must appear in JSON spans"
    );

    // stderr sink も同一 format。
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.trim().is_empty(), "stderr sink must emit events");
    for line in stderr.lines() {
        serde_json::from_str::<Value>(line)
            .unwrap_or_else(|e| panic!("stderr tracing line must parse as JSON: {e} ({line:?})"));
    }
}

#[test]
fn text_format_lines_are_not_json() {
    // Rule: format contract — text format は JSON parse 不能な
    // human-readable 行を両 sink に出す(format 未指定 = text)。
    let side = Side::new();
    let out = side.spawn(&["usage"], &[("CHEAPER_LOG", "info")], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);

    let lines = side.log_lines();
    assert!(!lines.is_empty(), "file sink must emit events");
    assert!(
        lines
            .iter()
            .all(|l| serde_json::from_str::<Value>(l).is_err()),
        "text format lines must not parse as JSON: {lines:?}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cheaper.cli.dispatch"),
        "stderr: {stderr:?}"
    );
    assert!(stderr
        .lines()
        .all(|l| serde_json::from_str::<Value>(l).is_err()));
}

#[test]
fn flag_overrides_env_level() {
    // Rule: activation matrix — flag > env(level)。
    // `CHEAPER_LOG=warn` + `--log debug` で debug event が出る。
    // `usage` は info-level span のみ emit するため、debug event
    // (`cheaper.catalog` merge 等)を持つ `models` で検証する。
    let side = Side::new();
    let out = side.spawn(
        &["models", "--log", "debug"],
        &[("CHEAPER_LOG", "warn")],
        b"",
    );
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    let lines = side.log_lines();
    assert!(
        lines.iter().any(|l| l.contains("DEBUG")),
        "--log debug must override CHEAPER_LOG=warn: {lines:?}"
    );
}

#[test]
fn flag_overrides_env_format() {
    // Rule: activation matrix — flag > env(format)。
    // `CHEAPER_LOG_FORMAT=json` + `--log-format text` で text 行。
    let side = Side::new();
    let out = side.spawn(
        &["usage", "--log-format", "text"],
        &[("CHEAPER_LOG", "debug"), ("CHEAPER_LOG_FORMAT", "json")],
        b"",
    );
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    let lines = side.log_lines();
    assert!(!lines.is_empty());
    assert!(
        lines
            .iter()
            .all(|l| serde_json::from_str::<Value>(l).is_err()),
        "--log-format text must override CHEAPER_LOG_FORMAT=json: {lines:?}"
    );
}

#[test]
fn invalid_level_fails_open() {
    // Rule: activation matrix — 不正 level は fail-open(warning + OFF、
    // command は logging 都合で失敗しない)。
    let side = Side::new();
    let out = side.spawn(&["usage"], &[("CHEAPER_LOG", "bogus")], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: invalid CHEAPER_LOG value \"bogus\""),
        "init warning must reach stderr: {stderr:?}"
    );
    assert!(
        !side.log_dir().exists(),
        "invalid level resolves OFF — no log dir"
    );
    assert_single_envelope_line(&out.stdout);
}

#[test]
fn invalid_format_fails_open() {
    // 不正 format は warning + text 扱いで logging は有効のまま。
    let side = Side::new();
    let out = side.spawn(
        &["usage"],
        &[("CHEAPER_LOG", "debug"), ("CHEAPER_LOG_FORMAT", "bogus")],
        b"",
    );
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: invalid CHEAPER_LOG_FORMAT value \"bogus\""),
        "init warning must reach stderr: {stderr:?}"
    );
    let lines = side.log_lines();
    assert!(!lines.is_empty(), "logging stays enabled (text format)");
    assert!(lines
        .iter()
        .all(|l| serde_json::from_str::<Value>(l).is_err()));
}

#[test]
fn quiet_suppresses_init_warning_but_keeps_logging() {
    // Rule: `--quiet` — diagnostics(今回は init `warning:` 行)は抑制
    // するが flag/env による明示的 log 有効化は抑制しない。
    let side = Side::new();
    let out = side.spawn(
        &["usage", "--quiet"],
        &[("CHEAPER_LOG", "debug"), ("CHEAPER_LOG_FORMAT", "bogus")],
        b"",
    );
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("warning:"),
        "--quiet must suppress the init warning: {stderr:?}"
    );
    // stderr sink 自体は diagnostics ではなく `--quiet` の対象外 —
    // tracing 行は出てよい。
    let lines = side.log_lines();
    assert!(
        lines.iter().any(|l| l.contains("cheaper.cli.dispatch")),
        "explicitly-enabled logging must not be suppressed by --quiet: {lines:?}"
    );
}

// ---------------------------------------------------------------------------
// hook 経路(非対称 contract: stderr sink 構造的無効)
// ---------------------------------------------------------------------------

#[test]
fn hook_stderr_clean_and_file_sink_active() {
    // Rule: dual sink — hook 経路は stderr sink を構造的に無効化。
    // invalid `CHEAPER_LOG_FORMAT` を注入して init 警告が stderr に
    // 漏れないこと(install 済み subscriber へ畳まれ file にのみ残る)と、
    // file に `cheaper.hook` event が出ることを確認する。
    let side = Side::new();
    std::fs::write(side.ws.join("small.txt"), "tiny\n").unwrap();
    let stdin = br#"{"agent_action_name":"pre_read_code","tool_info":{"file_path":"small.txt"}}"#;
    let out = side.spawn(
        &["hook", "pre-read-code", "--target", "windsurf"],
        &[("CHEAPER_LOG", "debug"), ("CHEAPER_LOG_FORMAT", "bogus")],
        stdin,
    );
    // windsurf allow は silent(stdout/stderr ともに空・exit 0)。
    assert_eq!(out.code, Some(0));
    assert!(
        out.stdout.is_empty(),
        "stdout: {:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    // zero tracing bytes + init `warning:` 行 — target native stderr
    // contract を汚さない。
    assert_eq!(
        out.stderr,
        b"",
        "hook stderr must stay clean (no tracing lines, no warning): {:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = side.log_lines().join("\n");
    assert!(
        text.contains("cheaper.hook"),
        "file sink must contain cheaper.hook span events: {text:?}"
    );
    assert!(
        text.contains("invalid CHEAPER_LOG_FORMAT"),
        "init warnings fold into the file sink via warn!: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// log root anchoring と fail-open
// ---------------------------------------------------------------------------

#[test]
fn cwd_flag_anchors_log_root() {
    // Rule: log root anchoring — `models` 等 `--cwd` が command 意味を
    // 持つ経路は argv `--cwd` が `.cheaper/logs` の anchor になる。
    let side = Side::new();
    let other = side._tmp.path().join("other-ws");
    std::fs::create_dir_all(&other).unwrap();
    let out = side.spawn(
        &["models", "--cwd", &other.to_string_lossy()],
        &[("CHEAPER_LOG", "debug")],
        b"",
    );
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    let anchored = other.join(".cheaper").join("logs");
    assert!(
        std::fs::read_dir(&anchored)
            .map(|rd| rd
                .flatten()
                .any(|e| e.file_name().to_string_lossy().starts_with("cheaper.log")))
            .unwrap_or(false),
        "log root must anchor to --cwd ({anchored:?} must hold cheaper.log.*)"
    );
    assert!(
        !side.log_dir().exists(),
        "process cwd must not receive .cheaper/logs when --cwd is given"
    );
}

#[test]
fn unwritable_log_root_fails_open_to_stderr_only() {
    // Rule: log root anchoring と fail-open — dir 作成/書込み不可は
    // `warning:` + stderr-only に降格し、command は logging 都合で
    // 失敗しない。`.cheaper` を regular file として作ることで
    // `.cheaper/logs` の dir 作成を portable に失敗させる(read-only
    // attribute や ACL は platform・権限依存で flaky になり得る)。
    let side = Side::new();
    std::fs::write(side.ws.join(".cheaper"), "not a dir").unwrap();
    let out = side.spawn(&["usage"], &[("CHEAPER_LOG", "debug")], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: cannot initialize file log sink"),
        "file sink failure must surface as a warning line: {stderr:?}"
    );
    // stderr-only 降格 — subscriber は install され tracing 行は
    // stderr sink に届き続ける。
    assert!(
        stderr.contains("cheaper.cli.dispatch"),
        "stderr sink keeps receiving events: {stderr:?}"
    );
    assert_single_envelope_line(&out.stdout);
}

// ---------------------------------------------------------------------------
// spec AC5 — run pipeline 以外の計測点(auth/catalog I/O boundary)
// ---------------------------------------------------------------------------

#[test]
fn auth_status_emits_auth_boundary_events() {
    let side = Side::new();
    let out = side.spawn(&["auth", "status"], &[("CHEAPER_LOG", "debug")], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    assert_single_envelope_line(&out.stdout);
    let text = side.log_lines().join("\n");
    assert!(
        text.contains("cheaper.auth"),
        "auth status must emit cheaper.auth span/store events: {text:?}"
    );
}

#[test]
fn models_emits_catalog_events() {
    let side = Side::new();
    let out = side.spawn(&["models"], &[("CHEAPER_LOG", "debug")], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);
    assert_single_envelope_line(&out.stdout);
    let text = side.log_lines().join("\n");
    assert!(
        text.contains("cheaper.catalog"),
        "models must emit cheaper.catalog merge events: {text:?}"
    );
}

// ---------------------------------------------------------------------------
// retention — `max_log_files(7)` の m+1 semantics 実測
// ---------------------------------------------------------------------------

#[test]
fn retention_keeps_most_recent_files() {
    // Rule: retention — tracing-appender 0.2.x は appender build 時に
    // `max_files - 1` 個だけ既存 file を残し(これから新規 file を 1 つ
    // 作るため)、prefix 一致の file のみを削除対象にする。
    // 実測: 9 個の既存世代を置いて spawn すると 6 + 当日 = 計 7 file。
    let side = Side::new();
    let dir = side.log_dir();
    std::fs::create_dir_all(&dir).unwrap();
    for day in 1..=9 {
        std::fs::write(dir.join(format!("cheaper.log.2024-01-{day:02}")), "stale\n").unwrap();
        // retention の新旧判定は file の作成時刻 — 逐次作成で順序を
        // 確定させる(同一 timestamp の tie は削除対象を不定にする)。
        std::thread::sleep(Duration::from_millis(30));
    }
    // prefix 非一致の file は retention 対象外。
    std::fs::write(dir.join("other.log.2024-01-01"), "keep\n").unwrap();

    let out = side.spawn(&["usage"], &[("CHEAPER_LOG", "debug")], b"");
    assert_eq!(out.code, Some(0), "stderr: {:?}", out.stderr);

    let names = side.log_file_names();
    assert_eq!(
        names.len(),
        7,
        "max_log_files(7) keeps the 6 newest pre-existing files plus today's: {names:?}"
    );
    assert!(
        !names.contains(&"cheaper.log.2024-01-01".to_string())
            && names.contains(&"cheaper.log.2024-01-09".to_string()),
        "oldest generations are pruned, newest retained: {names:?}"
    );
    assert!(
        dir.join("other.log.2024-01-01").exists(),
        "files not matching the cheaper.log prefix must never be pruned"
    );
}

// ---------------------------------------------------------------------------
// repeat-init(in-process — file 内容は assert しない唯一の経路)
// ---------------------------------------------------------------------------

/// Rule: repeat-init — install 済みでの再呼出しは first subscriber
/// wins。同一 test binary 内で `run_cli` を複数回呼ぶ経路がこの
/// contract に依存するため、in-process では本 test のみが logging を
/// 有効化する(他の in-process test が subscriber を先取りしないよう
/// 本 file の他 test は全て subprocess)。
#[tokio::test(flavor = "current_thread")]
async fn repeat_init_warns_and_continues() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("ws");
    let cfg = tmp.path().join("cfg");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::create_dir_all(&cfg).unwrap();

    let env_map: BTreeMap<String, String> = BTreeMap::from([
        (
            "CHEAPER_CONFIG_DIR".to_string(),
            cfg.to_string_lossy().into_owned(),
        ),
        ("CHEAPER_LOG".to_string(), "debug".to_string()),
    ]);

    for attempt in 0..2 {
        let env_lookup = |name: &str| env_map.get(name).cloned();
        let mut stdin = PipeStdin::empty(false);
        let mut out = VecSink(Vec::new());
        let mut err = VecSink(Vec::new());
        let mut io = CliIo {
            stdin: &mut stdin,
            stdout: &mut out,
            stderr: &mut err,
            env: &env_lookup,
            cwd: ws.to_string_lossy().into_owned(),
        };
        let hooks = CliHooks::default();
        let code = run_cli(&["usage".to_string()], &mut io, &hooks).await;
        assert_eq!(code, 0, "attempt {attempt} exit code");
        let err = String::from_utf8(err.0).unwrap();
        if attempt == 0 {
            assert!(
                !err.contains("subscriber already installed"),
                "first init installs cleanly: {err:?}"
            );
        } else {
            assert!(
                err.contains("warning: subscriber already installed"),
                "second init must report first-subscriber-wins: {err:?}"
            );
        }
        // envelope exclusivity — init 有無で stdout は変わらない。
        let out = String::from_utf8(out.0).unwrap();
        let env: Value = serde_json::from_str(out.trim_end()).unwrap();
        assert_eq!(env["schemaVersion"], 1);
    }
}
