//! Observability foundation contract test — dev-seam 経路
//! (`required-features = ["dev-seam"]` で gate)。
//!
//! `CHEAPER_TEST_FAKE_STREAM` fixture で dev-seam binary を
//! `CHEAPER_LOG=debug` subprocess として spawn し、以下を assert する:
//!
//! - secrets 非記録(credential・prompt 本文・tool input/output 本体・
//!   forbidden field key・workspace 絶対 path)が file/stderr 双方で
//!   grep 0 件
//! - run pipeline stage timing — `cheaper.run` span tree の close
//!   event(`time.busy`)と stage span 名が file に出る
//! - tool internals(`cheaper.tool` span + `cheaper.tool.result` /
//!   `cheaper.tool.process` event)が file に出る
//!
//! subprocess isolation は `tests/observability.rs` と同一方針
//! (`CARGO_BIN_EXE_cheaper` + `env_clear` spawn)。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

// ---------------------------------------------------------------------------
// subprocess harness(`tests/observability.rs` と同一最小構成)
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

fn spawn_collect(
    args: &[String],
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
    // dev-seam の fake stream は `NODE_ENV === "test"` のときのみ有効。
    env.push(("NODE_ENV".to_string(), "test".to_string()));
    env.push((
        "CHEAPER_CONFIG_DIR".to_string(),
        cfg.to_string_lossy().into_owned(),
    ));
    env
}

fn log_text(ws: &Path) -> String {
    let dir = ws.join(".cheaper").join("logs");
    let mut text = String::new();
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
            text.push_str(&std::fs::read_to_string(&p).unwrap());
            text.push('\n');
        }
    }
    text
}

/// marker 文字列 — payload/credential が field に載ると必ず検出される
/// 一意の語彙(通常の log 行には出ない)。
const SECRET_MARKER: &str = "OBS-SECRET-9f8e7d6c5b4a";
const PROMPT_MARKER: &str = "OBS-PROMPTBODY-5a4b3c2d1e0f";
const FILE_BODY_MARKER: &str = "OBS-FILEBODY-1f2e3d4c5b6a";
const INPUT_MARKER: &str = "OBS-INPUTBODY-6c5d4e3f2a1b";

/// tool call(read + policy-defined process task)を含む deterministic
/// fixture run。`auth_logins` で credential store への永続化経路
/// (`cheaper.auth`/`cheaper.auth.store` event)も通す。
fn fixture_json() -> String {
    serde_json::json!({
        "name": "observability-contract",
        "worker": {
            "name": "obs-worker",
            "tools": ["read", "process"],
            "prompt": format!("Summarize files. {PROMPT_MARKER}")
        },
        "auth_logins": { "cheaper-fake": SECRET_MARKER },
        "input": { "files": ["a.txt"], "task": format!("read a.txt {INPUT_MARKER}") },
        "sessions": [
            { "steps": [
                { "tool_calls": [
                    { "name": "read", "input": { "path": "a.txt" } },
                    { "name": "process", "input": { "task": "emit" } }
                ] },
                { "turn": { "tool_calls": true } },
                { "usage_delta": { "input": 100, "output": 20 } },
                { "turn": {} },
                { "finish": { "text": "{\"ok\":true}" } }
            ] }
        ]
    })
    .to_string()
}

/// `.cheaper/policy.yaml` — process task `emit` は cheaper binary 自身を
/// `--version` で起動する(portable・deterministic・shell 不可分離文字を
/// 含まない argv)。
fn policy_yaml() -> String {
    serde_json::json!({
        "version": 1,
        "process": {
            "tasks": {
                "emit": {
                    "command": [cheaper_bin().to_string_lossy(), "--version"],
                    "inputs": [],
                    "outputs": []
                }
            }
        }
    })
    .to_string()
}

struct Run {
    _tmp: tempfile::TempDir,
    ws: PathBuf,
    cfg: PathBuf,
    out: ProcOut,
}

fn spawn_logged_run() -> Run {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("ws");
    let cfg = tmp.path().join("cfg");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&ws).unwrap();
    std::fs::create_dir_all(&cfg).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    std::fs::write(
        ws.join("a.txt"),
        format!("alpha {FILE_BODY_MARKER} omega\n"),
    )
    .unwrap();
    let cheaper_dir = ws.join(".cheaper");
    std::fs::create_dir_all(&cheaper_dir).unwrap();
    std::fs::write(cheaper_dir.join("policy.yaml"), policy_yaml()).unwrap();

    let fixture_path = tmp.path().join("fixture.json");
    std::fs::write(&fixture_path, fixture_json()).unwrap();

    let mut env = base_env(&home, &cfg);
    env.push((
        "CHEAPER_TEST_FAKE_STREAM".to_string(),
        fixture_path.to_string_lossy().into_owned(),
    ));
    env.push(("CHEAPER_LOG".to_string(), "debug".to_string()));
    env.push(("CHEAPER_LOG_FORMAT".to_string(), "json".to_string()));

    let argv: Vec<String> = ["run", "obs-worker"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let stdin = br#"{"files":["a.txt"],"task":"read a.txt"}"#;
    let out = spawn_collect(&argv, &ws, &env, stdin, Duration::from_secs(120));
    Run {
        _tmp: tmp,
        ws,
        cfg,
        out,
    }
}

#[test]
fn fixture_run_stage_timing_and_tool_events() {
    let run = spawn_logged_run();
    assert_eq!(
        run.out.code,
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&run.out.stderr)
    );
    // stdout は引き続き envelope 1 行のみ。
    let stdout = String::from_utf8_lossy(&run.out.stdout);
    let mut lines = stdout.lines();
    let env: Value = serde_json::from_str(lines.next().unwrap_or_default())
        .expect("stdout must be one JSON envelope line");
    assert!(lines.next().is_none(), "multi-line stdout: {stdout:?}");
    assert_eq!(env["schemaVersion"], 1);
    assert_eq!(env["ok"], true);

    let text = log_text(&run.ws);
    assert!(!text.is_empty(), "file sink must contain events");

    // 全行 JSON parse 可能。
    let mut events = Vec::new();
    for line in text.lines().filter(|l| !l.is_empty()) {
        events.push(
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|e| panic!("log line must parse as JSON: {e} ({line:?})")),
        );
    }

    // stage timing — `cheaper.run` + stage 子 span の close event が
    // `time.busy`/`time.idle` を持つ。design-named span は JSON output
    // の `span.name`(close event)/`spans[].name` から検索する。
    // NOTE: `cheaper.run.meta` は success path のみ emit される既知の
    // 実装 note — 常時発火する core stage に絞って assert する。
    let mut span_names = std::collections::BTreeSet::new();
    let mut saw_timed_close = false;
    for ev in &events {
        if let Some(name) = ev.get("span").and_then(|s| s["name"].as_str()) {
            span_names.insert(name.to_string());
            if ev["fields"].get("time.busy").is_some() {
                saw_timed_close = true;
            }
        }
        if let Some(spans) = ev.get("spans").and_then(Value::as_array) {
            for s in spans {
                if let Some(name) = s["name"].as_str() {
                    span_names.insert(name.to_string());
                }
            }
        }
    }
    for expected in [
        "cheaper.run",
        "cheaper.run.tool_runtime",
        "cheaper.run.model_resolve",
        "cheaper.run.tool_scope",
        "cheaper.run.session",
        "cheaper.run.prompt",
        "cheaper.run.extract",
    ] {
        assert!(
            span_names.contains(expected),
            "stage span {expected} missing; saw: {span_names:?}"
        );
    }
    assert!(
        saw_timed_close,
        "span close events must carry time.busy (FmtSpan::CLOSE)"
    );

    // design-named event/span は target(T4/T5)または span name(T3)
    // のどちらかで検出できる — target 必須とはしない。
    let targets: std::collections::BTreeSet<String> = events
        .iter()
        .filter_map(|ev| ev["target"].as_str().map(String::from))
        .collect();
    let named = |name: &str| span_names.contains(name) || targets.contains(name);
    assert!(named("cheaper.cli.dispatch"), "{span_names:?}");
    assert!(named("cheaper.cli.command"), "{span_names:?}");
    assert!(named("cheaper.tool"), "{span_names:?}");
    assert!(named("cheaper.tool.result"), "{targets:?}");
    assert!(
        named("cheaper.tool.process"),
        "process tool call must emit cheaper.tool.process events: {targets:?}"
    );
    // NOTE: `cheaper.adapter.*` events live in cheaper-rig-adapter's real
    // session path — the fake-stream fixture substitutes the session
    // factory, so adapter events are legitimately absent here (the
    // contract is asserted on the real path, not the seam).
    assert!(named("cheaper.auth"), "{span_names:?}");
    assert!(named("cheaper.auth.store"), "{targets:?}");
    assert!(named("cheaper.catalog"), "{targets:?}");
}

#[test]
fn secrets_and_payloads_are_not_recorded() {
    let run = spawn_logged_run();
    assert_eq!(
        run.out.code,
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&run.out.stderr)
    );

    let file_text = log_text(&run.ws);
    assert!(!file_text.is_empty());
    let stderr = String::from_utf8_lossy(&run.out.stderr);
    // file sink + stderr sink 双方を scan 対象にする。
    let combined = format!("{file_text}\n{stderr}");

    // forbidden values — credential・prompt 本文・tool input/output
    // 本体・run input 本文はどこにも出ない。
    for marker in [SECRET_MARKER, PROMPT_MARKER, FILE_BODY_MARKER, INPUT_MARKER] {
        assert!(
            !combined.contains(marker),
            "payload/secret marker {marker} leaked into logs"
        );
    }

    // forbidden field keys — metadata-only allowlist 外の key が
    // emitted event に現れない(JSON format で機械検査)。`"k":` 形で
    // key を絞る — `"api_key"` は `auth.type` の分類値として合法に
    // 出るため value 側は区別しない。
    for forbidden in [
        "\"api_key\":",
        "\"apiKey\":",
        "\"password\":",
        "\"authorization\":",
        "\"credential\":",
        "\"system_prompt\":",
        "\"systemPrompt\":",
        "\"prompt\":",
        "\"url\":",
        "\"base_url\":",
        "\"path\":",
        "\"token\":",
    ] {
        assert!(
            !combined.contains(forbidden),
            "forbidden field key {forbidden} found in logs"
        );
    }

    // absolute paths — log は basename 参照(`file.name`)のみを許容し、
    // workspace/config dir の絶対 path を出さない(OS username を含み
    // 得る識別情報)。JSON 内では Windows の `\` が `\\` に escape され
    // るため両形を scan する。
    for abs in [&run.ws, &run.cfg] {
        let s_fwd = abs.to_string_lossy().into_owned();
        let s_escaped = s_fwd.replace('\\', "\\\\");
        assert!(
            !combined.contains(&s_fwd) && !combined.contains(s_escaped.as_str()),
            "absolute path {abs:?} leaked into the log sinks"
        );
    }
}
