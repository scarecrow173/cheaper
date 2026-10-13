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

// ---------------------------------------------------------------------------
// worker job — opt-in 縮退(test-spec Rule: workflow `kind: worker` の
// 縮退は run と同一挙動 — `kind: tool` job は frontmatter を持たず
// opt-in 対象外)
// ---------------------------------------------------------------------------

/// builtin `tool-operator` の output schema 適合 result を返す
/// fake-stream fixture — sessions のみ(worker 節不要 — job は builtin
/// `tool-operator.md` を解決し `cheaper.optionalTools` opt-in 宣言を
/// frontmatter から読む)。
const FIXTURE_TOOL_OPERATOR: &str = r#"{
  "name": "cli-t7-tool-operator",
  "sessions": [
    {"steps": [
      {"turn": {}},
      {"finish": {"text": "{\"status\":\"blocked\",\"summary\":\"command unavailable under current policy\",\"findings\":[{\"title\":\"t\",\"detail\":\"d\",\"evidence\":[{\"kind\":\"note\",\"detail\":\"e\"}]}],\"unresolved\":[\"command unavailable\"],\"nextActions\":[\"grant command policy\"]}"}}
    ]}
  ]
}"#;

/// `kind: worker` job が builtin `tool-operator`(`optionalTools:
/// [process, command]` opt-in)を実行する経路 — builtin default
/// policy(tasks/grants 空)では opt-in tool が `run` と同一の registry
/// 縮退を受け、`POLICY_VIOLATION` で job が死なず model session まで
/// 到達して `status: "blocked"` の worker result を返す。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_tool_operator_degrades_like_run() {
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_TOOL_OPERATOR)
        .with_workflow(json!({
            "schemaVersion": 1,
            "inputs": {},
            "jobs": {
                "op": {
                    "kind": "worker",
                    "worker": "tool-operator",
                    "input": { "task": "inspect the workspace" }
                }
            },
            "outputs": {
                "status": {
                    "schema": { "type": "string" },
                    "value": { "$from": "op", "path": "/status" }
                }
            }
        }));
    let (code, env, err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    // 縮退下でも run は完走 — `result.status` は worker result であり
    // run-level error ではない。
    assert_eq!(env["result"], json!({"status": "blocked"}));
    let jobs = env["jobs"].as_array().unwrap();
    let job = jobs.iter().find(|j| j["id"] == json!("op")).unwrap();
    assert_eq!(job["status"], json!("succeeded"));
    // degraded `process`/`command` は実行経路を持たない。
    assert_eq!(job["meta"]["toolCalls"], json!(0));
    assert_eq!(job["meta"]["savings"]["directlyComparable"], json!(false));
}
