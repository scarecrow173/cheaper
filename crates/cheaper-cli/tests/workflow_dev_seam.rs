//! `workflow run` の fake-stream fixture 経路 test(`dev-seam` cargo
//! feature 前提 — `CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test` seam が
//! feature 有効時のみ compile される)。`required-features =
//! ["dev-seam"]` で gate 済み。
//!
//! worker job は `CHEAPER_TEST_FAKE_STREAM` fixture で deterministic に
//! 実行する(実 provider 不要)。`CHEAPER_CONFIG_DIR` を temp dir へ
//! 向けて credential store / usage.jsonl を分離する。

use std::collections::BTreeMap;
use std::path::Path;

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

/// test 用組立一式(tests/workflow.rs と同一パターン)。`env` map は
/// `io.env` 経由で lookup され、未設定名は process env へ fallback
/// する(`CHEAPER_LOG*` 除く)。
struct Fixture {
    ws: tempfile::TempDir,
    /// `CHEAPER_CONFIG_DIR` 先の temp dir。`env_map` からのみ参照
    /// されるため field 自体は read されない(drop まで生存必須)。
    _cfg: tempfile::TempDir,
    env_map: BTreeMap<String, String>,
    stdin: PipeStdin,
    out: VecSink,
    err: VecSink,
    hooks: CliHooks,
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
            hooks: CliHooks {
                // embedded materialize を避け、crate 内の bundled resource
                // dir を使う(workers/ + policy.yaml が同一 layout で存在)。
                builtin_dir: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources")),
                ..Default::default()
            },
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

    fn with_workflow(self, definition: Value) -> Self {
        let path = self.ws.path().join("workflow.json");
        std::fs::write(&path, serde_json::to_string(&definition).unwrap()).unwrap();
        self
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
        let code = run_cli(
            &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            &mut io,
            &self.hooks,
        )
        .await;
        drop(io);
        let out_str = String::from_utf8_lossy(&self.out.0).into_owned();
        let err_str = String::from_utf8_lossy(&self.err.0).into_owned();
        (code, serde_json::from_str(&out_str).unwrap(), err_str)
    }
}

// ---------------------------------------------------------------------------
// worker job — fake-stream fixture
// ---------------------------------------------------------------------------

/// bulk-reader の output schema に適合する 2 session fixture
/// (`{"summary",findings[],relevantFiles[],suggestedReads[],
/// openQuestions[]}` 必須)。
const FIXTURE_TWO_WORKERS: &str = r#"{
  "name": "cli-t7-two-workers",
  "sessions": [
    {"steps": [
      {"turn": {}},
      {"finish": {"text": "{\"summary\":\"first\",\"findings\":[],\"relevantFiles\":[],\"suggestedReads\":[],\"openQuestions\":[]}"}}
    ]},
    {"steps": [
      {"turn": {}},
      {"finish": {"text": "{\"summary\":\"second\",\"findings\":[],\"relevantFiles\":[],\"suggestedReads\":[],\"openQuestions\":[]}"}}
    ]}
  ]
}"#;

/// TS test: "shares one lazy model runtime across Worker jobs" 相当。
/// `needs` edge 付きの 2 worker job が fixture session を順に消費する。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_two_worker_jobs_with_needs() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_TWO_WORKERS).with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "a": { "kind": "worker", "worker": "bulk-reader", "input": { "task": "first" } },
            "b": {
                "kind": "worker",
                "worker": "bulk-reader",
                "input": { "task": "second" },
                "needs": ["a"]
            }
        },
        "outputs": {
            "answer": { "schema": { "type": "string" }, "value": { "$from": "a", "path": "/summary" } },
            "next": { "schema": { "type": "string" }, "value": { "$from": "b", "path": "/summary" } }
        }
    }));
    let (code, env, err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    // session は job 順に消費される(a → session 0、b → session 1)。
    assert_eq!(env["result"], json!({"answer": "first", "next": "second"}));
    let jobs = env["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 2);
    for id in ["a", "b"] {
        let job = jobs.iter().find(|j| j["id"] == json!(id)).unwrap();
        assert_eq!(job["status"], json!("succeeded"));
        // `selectWorkerMeta` — model/usage/turns 等が載る。
        // `execution`/`durationMs` は engine が job meta に merge する
        // (TS `{...executorMeta, durationMs, execution}`)。
        assert_eq!(job["meta"]["model"]["provider"], json!("cheaper-fake"));
        assert!(job["meta"]["durationMs"].is_number());
        assert!(job["meta"]["execution"]["operations"].is_number());
    }
}
