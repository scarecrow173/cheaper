//! `workflow run` の fake-stream fixture 経路 test(`dev-seam` cargo
//! feature 前提 — `CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test` seam が
//! feature 有効時のみ compile される)。`required-features =
//! ["dev-seam"]` で gate 済み。
//!
//! worker job は `CHEAPER_TEST_FAKE_STREAM` fixture で deterministic に
//! 実行する(実 provider 不要)。`CHEAPER_CONFIG_DIR` を temp dir へ
//! 向けて credential store / usage.jsonl を分離する。

mod common;
use common::Fixture;

use serde_json::json;

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
