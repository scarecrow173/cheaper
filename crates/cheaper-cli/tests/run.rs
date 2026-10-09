//! `cheaper run`(T3)の in-process 結合 test — worker 解決・input・
//! policy/`--timeout`・fake-stream fixture・signal 中継・envelope・
//! usage.jsonl を `run_cli` 経路で検証する。
//!
//! provider 解決は fixture(`CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test`)
//! で行うため実 credential/network 不要。`CHEAPER_CONFIG_DIR` を temp
//! dir へ向けて credential store / usage.jsonl を分離する。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cheaper_cli::cli::{run_cli, CliHooks, CliIo, CliStdin, CliWrite};
use serde_json::{json, Value};

struct VecSink(Vec<u8>);
impl CliWrite for VecSink {
    fn write_str(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }
}

struct PipeStdin {
    data: Vec<u8>,
    tty: bool,
}
impl CliStdin for PipeStdin {
    fn is_tty(&self) -> bool {
        self.tty
    }
    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        Ok(std::mem::take(&mut self.data))
    }
}

/// test 用組立一式。`env` map は `io.env` 経由で lookup され、
/// 未設定名は process env へ fallback する。
struct Fixture {
    ws: tempfile::TempDir,
    cfg: tempfile::TempDir,
    env_map: BTreeMap<String, String>,
    stdin: PipeStdin,
    out: VecSink,
    err: VecSink,
}

impl Fixture {
    fn new() -> Self {
        let ws = tempfile::tempdir().unwrap();
        let cfg = tempfile::tempdir().unwrap();
        let mut env_map = BTreeMap::new();
        env_map.insert(
            "CHEAPER_CONFIG_DIR".to_string(),
            cfg.path().to_string_lossy().into_owned(),
        );
        Self {
            ws,
            cfg,
            env_map,
            stdin: PipeStdin {
                data: Vec::new(),
                tty: false,
            },
            out: VecSink(Vec::new()),
            err: VecSink(Vec::new()),
        }
    }

    fn with_fake_stream(mut self, fixture_json: &str) -> Self {
        let path = self.ws.path().join("fixture.json");
        std::fs::write(&path, fixture_json).unwrap();
        self.env_map
            .insert("NODE_ENV".to_string(), "test".to_string());
        self.env_map.insert(
            "CHEAPER_TEST_FAKE_STREAM".to_string(),
            path.to_string_lossy().into_owned(),
        );
        self
    }

    fn with_input(mut self, input_json: &str) -> Self {
        self.stdin.data = input_json.as_bytes().to_vec();
        self
    }

    fn usage_log(&self) -> Option<String> {
        std::fs::read_to_string(self.cfg.path().join("usage.jsonl")).ok()
    }

    /// `providers/devin/catalog.json` に malformed snapshot を配置する
    /// — `Runtime::create` の snapshot load error が catalog errors に
    /// 記録され `warning:` 行として stderr に出る fixture。
    fn write_malformed_devin_catalog(&self) {
        let provider_dir = self.cfg.path().join("providers").join("devin");
        std::fs::create_dir_all(&provider_dir).unwrap();
        std::fs::write(provider_dir.join("catalog.json"), "{ not json").unwrap();
    }

    fn ws_path(&self) -> PathBuf {
        self.ws.path().to_path_buf()
    }

    async fn run(&mut self, argv: &[&str]) -> (i32, Value, String) {
        let env_map = &self.env_map;
        let env_lookup = move |name: &str| {
            env_map
                .get(name)
                .cloned()
                .or_else(|| std::env::var(name).ok())
        };
        let cwd = self.ws.path().to_string_lossy().into_owned();
        let mut io = CliIo {
            stdin: &mut self.stdin,
            stdout: &mut self.out,
            stderr: &mut self.err,
            env: &env_lookup,
            cwd,
        };
        let hooks = CliHooks {
            // embedded materialize を避け、crate 内の bundled resource dir
            // を使う(workers/ + policy.yaml が同一 layout で存在する)。
            builtin_dir: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources")),
            ..Default::default()
        };
        let code = run_cli(
            &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &mut io,
            &hooks,
        )
        .await;
        drop(io);
        let out_str = String::from_utf8_lossy(&self.out.0).into_owned();
        let err_str = String::from_utf8_lossy(&self.err.0).into_owned();
        (code, serde_json::from_str(&out_str).unwrap(), err_str)
    }
}

/// 最小 success fixture(01-success-basic 相当 — fenced `{"ok":true}`)。
const FIXTURE_BASIC: &str = r#"{
  "name": "cli-t3-basic",
  "sessions": [
    {"steps": [
      {"usage_delta": {"input": 1000, "output": 2000}},
      {"turn": {}},
      {"finish": {"text": "```json\n{\"ok\":true}\n```"}}
    ]}
  ]
}"#;

/// signal abort fixture(08-signal-abort 相当 — `hang_until_abort`)。
const FIXTURE_ABORT: &str = r#"{
  "name": "cli-t3-abort",
  "abort": {"after_ms": 10, "message": "user abort"},
  "sessions": [
    {"steps": [
      {"usage_delta": {"input": 10, "output": 1}},
      {"turn": {}},
      "hang_until_abort"
    ]}
  ]
}"#;

// ---------------------------------------------------------------------------
// worker / input / policy 解決経路
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn run_unknown_worker_worker_not_found() {
    let mut f = Fixture::new();
    let (code, env, _err) = f.run(&["run", "no-such-worker"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("WORKER_NOT_FOUND"));
    // `details.searched` — project → user → builtin の 3 path。
    let searched = env["error"]["details"]["searched"].as_array().unwrap();
    assert_eq!(searched.len(), 3);
    assert!(searched[0].as_str().unwrap().contains("workers"));
    assert_ne!(code, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn run_stdin_tty_without_input_errors() {
    let mut f = Fixture::new();
    f.stdin.tty = true;
    let (code, env, _err) = f.run(&["run", "bulk-reader"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_REQUEST"));
    assert!(env["error"]["message"]
        .as_str()
        .unwrap()
        .contains("stdin is a TTY"));
    assert_ne!(code, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn run_invalid_timeout_invalid_request() {
    let mut f = Fixture::new().with_input(r#"{"task":"t"}"#);
    let (code, env, _err) = f.run(&["run", "bulk-reader", "--timeout", "bogus"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_REQUEST"));
    assert_eq!(
        env["error"]["message"],
        json!("invalid --timeout duration: bogus")
    );
    assert_eq!(env["error"]["details"]["timeout"], json!("bogus"));
    assert_ne!(code, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn run_timeout_over_policy_policy_violation() {
    // builtin policy の execution.timeoutMs(既定 120s)を超える
    // `--timeout` は narrow ではなく違反。
    let mut f = Fixture::new().with_input(r#"{"task":"t"}"#);
    let (code, env, _err) = f.run(&["run", "bulk-reader", "--timeout", "9999s"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("POLICY_VIOLATION"));
    assert_ne!(code, 0);
}

// ---------------------------------------------------------------------------
// fake-stream fixture 経路
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_fixture_worker_fallback_success() {
    // `fixture-worker` は builtin/user/project いずれにも存在しない
    // → fixture `worker` 節へ fallback し scripted stream を実行する。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    let (code, env, err) = f.run(&["run", "fixture-worker"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], json!("fixture-worker"));
    assert_eq!(env["result"], json!({"ok": true}));
    assert_eq!(env["meta"]["model"]["provider"], json!("cheaper-fake"));
    assert_eq!(env["meta"]["model"]["id"], json!("fake-model"));
    // stderr の model line(TS `model: <prov>/<id> (tier <t>)`)。
    assert!(err.contains("model: cheaper-fake/fake-model (tier "));
    // usage.jsonl に run record が追記される。
    let log = f.usage_log().expect("usage.jsonl must be written");
    assert!(log.contains("\"kind\":\"run\""), "usage log: {log}");
}

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_quiet_suppresses_model_line() {
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    let (code, _env, err) = f.run(&["run", "fixture-worker", "--quiet"]).await;
    assert_eq!(code, 0);
    assert_eq!(err, "");
}

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_signal_abort() {
    // fixture `abort.after_ms` → signal 側 abort → ABORTED 分類。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_ABORT)
        .with_input(r#"{"files":["a.txt"]}"#);
    let (code, env, _err) = f.run(&["run", "fixture-worker"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("ABORTED"));
    assert_eq!(env["error"]["exitCode"], json!(8));
    assert_eq!(env["error"]["details"]["reason"], json!("signal"));
    assert_eq!(code, 8);
}

#[tokio::test(flavor = "current_thread")]
async fn run_worker_file_with_fake_stream() {
    // `--worker-file` — 明示 path の worker を fixture stream で実行。
    let worker_md = r#"---
name: file-worker
description: explicit worker file
tools: [read]
cheaper:
  model: {tier: low}
  output:
    schema:
      inline: {"type":"object","required":["ok"],"properties":{"ok":{"type":"boolean"}}}
---
Do it.
"#;
    let mut f = Fixture::new().with_fake_stream(FIXTURE_BASIC);
    let wpath = f.ws_path().join("file-worker.md");
    std::fs::write(&wpath, worker_md).unwrap();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let warg = wpath.to_string_lossy().into_owned();
    let (code, env, err) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], json!("file-worker"));
    assert_eq!(env["result"], json!({"ok": true}));
}

/// main + repair 用の 2 session fixture(どちらも `{"ok":true}` を
/// 返す — bulk-reader の output schema には合わず INVALID_OUTPUT
/// まで到達する)。
const FIXTURE_TWO_BAD: &str = r#"{
  "name": "cli-t3-two-bad",
  "sessions": [
    {"steps": [{"turn": {}}, {"finish": {"text": "{\"ok\":true}"}}]},
    {"steps": [{"finish": {"text": "{\"ok\":true}"}}]}
  ]
}"#;

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_named_worker_takes_precedence_over_fallback() {
    // 実在する builtin worker(`bulk-reader`)が resolve される限り
    // fixture `worker` fallback は発火しない — bulk-reader の output
    // schema(`summary` 等必須)は `{"ok":true}` で失敗し、repair も
    // 同じ応答で exhausted → INVALID_OUTPUT に到達すれば named
    // resolution 優先が確認できる。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_TWO_BAD)
        .with_input(r#"{"task":"t"}"#);
    let (code, env, _err) = f.run(&["run", "bulk-reader"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["worker"], json!("bulk-reader"));
    assert_eq!(env["error"]["code"], json!("INVALID_OUTPUT"));
    assert_ne!(code, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_disabled_without_node_env_test() {
    // `NODE_ENV` 未設定 → fixture 無効 → fake provider 未登録のため
    // `fixture-worker` は named resolution で WORKER_NOT_FOUND。
    let mut f = Fixture::new();
    let path = f.ws_path().join("fixture.json");
    std::fs::write(&path, FIXTURE_BASIC).unwrap();
    f.env_map.insert(
        "CHEAPER_TEST_FAKE_STREAM".to_string(),
        path.to_string_lossy().into_owned(),
    );
    let (code, env, _err) = f.run(&["run", "fixture-worker"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("WORKER_NOT_FOUND"));
    assert_ne!(code, 0);
}

// ---------------------------------------------------------------------------
// catalog error の stderr surfacing(spec Req 4)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn run_emits_catalog_warnings_for_malformed_plugin_catalog() {
    // spec Req 4 — `run` も catalog error を `warning:` 行として stderr
    // へ出す(emit は `run_worker` より先)。envelope・exit code 不変。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    f.write_malformed_devin_catalog();
    let (code, env, err) = f.run(&["run", "fixture-worker"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["result"], json!({"ok": true}));
    assert!(
        err.contains("warning: Provider \"devin\":"),
        "stderr: {err}"
    );
    assert!(err.contains("plugin catalog"), "stderr: {err}");
    // 既存の stderr model line は変わらない。
    assert!(
        err.contains("model: cheaper-fake/fake-model"),
        "stderr: {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn run_quiet_suppresses_catalog_warnings() {
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    f.write_malformed_devin_catalog();
    let (code, env, err) = f.run(&["run", "fixture-worker", "--quiet"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    // warning・model line ともに抑制される。
    assert!(!err.contains("warning:"), "stderr: {err}");
    assert_eq!(err, "");
}
