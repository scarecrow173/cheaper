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

// ---------------------------------------------------------------------------
// `workflow run --grant` — invocation-scoped command grant(AC14、
// approved test-spec `docs/test-specs/invocation-scoped-command-grants.md`)。
// `execute_run_inner` → `build_invocation_layer` → `prepare_workflow_context`
// → `load_policy` の同一 merge 経路で `run` と parity が保たれる。
// `kind: tool` job は `cheaper.run.tool_scope` span を通らないため
// stderr の `grant: <names>` 行が workflow 経路の audit surface。
// ---------------------------------------------------------------------------

/// `command` grant が spawn する fixture child — `workflow run` の
/// worker/tool job がこの test binary 自身を再実行する。
/// `tests/workflow.rs` の `workflow_command_fixture_child` は別 test
/// binary のため `current_exe` 解決上再利用不可 — 本 file 内に同名の
/// self-reshim を持つ。`CHEAPER_PROC_CHILD_MODE` 未設定(通常の
/// `cargo test`)なら no-op。`probe` は `CHEAPER_PROC_CHILD_ARG` へ
/// marker file を書く。
#[test]
fn workflow_dev_seam_command_fixture_child() {
    let Ok(mode) = std::env::var("CHEAPER_PROC_CHILD_MODE") else {
        return;
    };
    match mode.as_str() {
        "probe" => {
            std::fs::write(std::env::var("CHEAPER_PROC_CHILD_ARG").unwrap(), "ran").unwrap();
        }
        other => panic!("unknown fixture mode: {other}"),
    }
}

/// `--grant` flag 値(JSON)— executable = この test binary 自身
/// (workspace 外のため workspace-resident 拒否に掛からない)、argv
/// literal pin、`env` で fixture mode + marker path を渡す。
fn workflow_command_grant_flag(marker: &str) -> String {
    serde_json::json!({
        "name": "self",
        "executable": std::env::current_exe().unwrap().to_string_lossy(),
        "argv": ["workflow_dev_seam_command_fixture_child", "--exact", "--nocapture"],
        "env": {
            "CHEAPER_PROC_CHILD_MODE": "probe",
            "CHEAPER_PROC_CHILD_ARG": marker,
        }
    })
    .to_string()
}

/// opt-in `command` worker — `cheaper.optionalTools` で `command` を
/// opt-in 宣言(grants 空なら縮退、grant があれば実登録)。
/// workflow の worker 解決は project/user/builtin dir のみを見る
/// (fixture `worker` 節 fallback は `run` 専用)ため、project workers
/// dir(`<ws>/.cheaper/workers/`)へ配置して参照する。
const WF_OPT_COMMAND_WORKER_MD: &str = r#"---
name: wf-opt-command-worker
description: opt-in command worker for workflow
tools: [read, command]
cheaper:
  optionalTools: [command]
  model:
    tier: low
  output:
    schema:
      inline:
        type: object
        required: [ok]
        properties:
          ok:
            type: boolean
---
Use command when granted.
"#;

/// `command` tool call を 1 件 emit してから正常終了する fixture —
/// grant name `self`・args は child fixture fn の self-reshim argv。
const FIXTURE_WORKFLOW_COMMAND_CALL: &str = r#"{
  "name": "cli-t7-grant-command",
  "sessions": [
    {"steps": [
      {"tool_calls": [{"name": "command", "input": {"name": "self", "args": ["workflow_dev_seam_command_fixture_child", "--exact", "--nocapture"]}}]},
      {"turn": {"tool_calls": true}},
      {"usage_delta": {"input": 100, "output": 20}},
      {"finish": {"text": "{\"ok\":true}"}}
    ]}
  ]
}"#;

/// AC14 — `workflow run --grant` が `kind: worker` job(opt-in worker)
/// の縮退を解消する。`load_policy` の同一 merge 経路で grant が
/// `RuntimePolicy.command` へ畳まれ、scripted `command` call が実
/// subprocess を spawn する。stderr `grant:` 行は invocation-sourced
/// grant の audit surface。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_grant_reaches_worker_job() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_WORKFLOW_COMMAND_CALL);
    let marker = f
        .ws_path()
        .join("wf-cmd-marker")
        .to_string_lossy()
        .into_owned();
    // project workers dir へ opt-in worker を配置。
    let workers = f.ws_path().join(".cheaper").join("workers");
    std::fs::create_dir_all(&workers).unwrap();
    std::fs::write(
        workers.join("wf-opt-command-worker.md"),
        WF_OPT_COMMAND_WORKER_MD,
    )
    .unwrap();
    f = f.with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "op": {
                "kind": "worker",
                "worker": "wf-opt-command-worker",
                "input": { "task": "t" }
            }
        },
        "outputs": {
            "ok": { "schema": { "type": "boolean" }, "value": { "$from": "op", "path": "/ok" } }
        }
    }));
    let grant = workflow_command_grant_flag(&marker);
    let (code, env, err) = f
        .run(&["workflow", "run", "workflow.json", "--grant", &grant])
        .await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    // `kind: worker` job でも stderr `grant:` 行が出る
    // (audit surface — span field は worker job の
    // `cheaper.run.tool_scope` にも record される)。
    assert!(err.contains("grant: self\n"), "stderr: {err}");
    assert_eq!(env["result"], json!({"ok": true}));
    let jobs = env["jobs"].as_array().unwrap();
    let job = jobs.iter().find(|j| j["id"] == json!("op")).unwrap();
    assert_eq!(job["status"], json!("succeeded"), "job: {job}");
    // 縮退解消 — command call が実 spawn(metering に計上)。
    assert_eq!(job["meta"]["toolCalls"], json!(1));
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
}

/// AC14 — `kind: tool` job の `command` call が `inner.command`
/// 再注入経由(invocation grant 込み resolved policy)で grant を受け
/// 実 subprocess を spawn する。`command: None` で parity check を
/// skip しても grant は tool job に届く。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_grant_reaches_tool_job() {
    let mut f = Fixture::new();
    let marker = f
        .ws_path()
        .join("wf-cmd-marker")
        .to_string_lossy()
        .into_owned();
    f = f.with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "cmd": {
                "kind": "tool",
                "tool": "command",
                "input": {
                    "name": "self",
                    "args": ["workflow_dev_seam_command_fixture_child", "--exact", "--nocapture"]
                }
            }
        },
        "outputs": {
            "state": { "schema": { "type": "string" }, "value": { "$from": "cmd", "path": "/details/state" } },
            "exit": { "schema": { "type": "number" }, "value": { "$from": "cmd", "path": "/details/exitCode" } }
        }
    }));
    let grant = workflow_command_grant_flag(&marker);
    let (code, env, err) = f
        .run(&["workflow", "run", "workflow.json", "--grant", &grant])
        .await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["result"], json!({"state": "completed", "exit": 0}));
    // `kind: tool` job は tool_scope span を通らない — stderr の
    // `grant:` 行が唯一の audit surface。
    assert!(err.contains("grant: self\n"), "stderr: {err}");
    let jobs = env["jobs"].as_array().unwrap();
    let job = jobs.iter().find(|j| j["id"] == json!("cmd")).unwrap();
    assert_eq!(job["status"], json!("succeeded"), "job: {job}");
    assert_eq!(job["meta"]["toolCalls"], json!(1));
    // 実 spawn — grant env(minimal env)経由で子が marker を書いた。
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
}
