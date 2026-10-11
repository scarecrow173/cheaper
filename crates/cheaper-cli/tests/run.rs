//! `cheaper run`(T3)の in-process 結合 test — worker 解決・input・
//! policy/`--timeout`・envelope の `run_cli` 経路を検証する。
//!
//! `dev-seam` cargo feature に依存しない case のみ置く。
//! fake-stream fixture(`CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test`)
//! 経路の test は `run_dev_seam.rs`(`required-features = ["dev-seam"]`
//! で gate)へ分離済み。
//!
//! `CHEAPER_CONFIG_DIR` を temp dir へ向けて credential store /
//! usage.jsonl を分離する。

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

/// ambient env fallback — `CHEAPER_LOG*` は絶対に継承しない
/// (in-process test binary で global subscriber を install すると
/// 後続 test が "subscriber already installed" になり stray
/// .cheaper/logs を残す)。
fn ambient_env(name: &str) -> Option<String> {
    if name.starts_with("CHEAPER_LOG") {
        return None;
    }
    std::env::var(name).ok()
}

/// test 用組立一式。`env` map は `io.env` 経由で lookup され、
/// 未設定名は process env へ fallback する(`CHEAPER_LOG*` 除く)。
struct Fixture {
    ws: tempfile::TempDir,
    /// `CHEAPER_CONFIG_DIR` 先の temp dir。`env_map` からのみ参照
    /// されるため field 自体は read されない(drop まで生存必須)。
    _cfg: tempfile::TempDir,
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
            _cfg: cfg,
            env_map,
            stdin: PipeStdin {
                data: Vec::new(),
                tty: false,
            },
            out: VecSink(Vec::new()),
            err: VecSink(Vec::new()),
        }
    }

    fn with_input(mut self, input_json: &str) -> Self {
        self.stdin.data = input_json.as_bytes().to_vec();
        self
    }

    fn ws_path(&self) -> PathBuf {
        self.ws.path().to_path_buf()
    }

    async fn run(&mut self, argv: &[&str]) -> (i32, Value, String) {
        let env_map = &self.env_map;
        let env_lookup = move |name: &str| env_map.get(name).cloned().or_else(|| ambient_env(name));
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
/// `CHEAPER_TEST_FAKE_STREAM` を env map へ直接流す用(feature off では
/// fixture は絶対に読まれない — seam-invariance 確認用)。
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
// seam-invariance — `CHEAPER_TEST_FAKE_STREAM` があっても `NODE_ENV=test`
// が無ければ(feature off では seam 自体が compile 対象外のため常に)
// fake 経路は起動しない
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_disabled_without_node_env_test() {
    // `NODE_ENV` 未設定 → fixture 無効 → fake provider 未登録のため
    // `fixture-worker` は named resolution で WORKER_NOT_FOUND。
    // `dev-seam` feature off でも同じ不成立条件を通る(seam 非含有の
    // compile-time gate と同じ assertion)。
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
