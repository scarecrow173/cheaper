//! 読み取り専用 History CLI 契約。

use crate::common::{self, Fixture};
use cheaper_cli::cli::{run_cli, CliIo};
use cheaper_cli::history::identity::ExecutionIdentity;
use cheaper_cli::history::schema::{ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent};
use cheaper_cli::history::store::HistoryStore;
use serde_json::{json, Value};

const RUN_USAGE_FIXTURE: &str = r#"{
  "name": "history-run-usage",
  "sessions": [{"steps": [
    {"usage_delta": {"input": 2, "output": 1}},
    {"turn": {}},
    {"finish": {"text": "```json\n{\"ok\":true}\n```"}}
  ]}]
}"#;

fn write_started(
    config_dir: &std::path::Path,
    worker_id: &str,
) -> (ExecutionIdentity, HistoryEvent) {
    let identity = ExecutionIdentity::root();
    let event = HistoryEvent::new(
        &identity,
        ExecutionKind::Worker,
        ExecutionEvent::Started,
        ExecutionStatus::Running,
    )
    .with_worker_id(worker_id);
    assert!(HistoryStore::new(config_dir).write_started(&event));
    (identity, event)
}

fn write_terminal(config_dir: &std::path::Path, identity: &ExecutionIdentity, worker_id: &str) {
    let event = HistoryEvent::new(
        identity,
        ExecutionKind::Worker,
        ExecutionEvent::Terminal,
        ExecutionStatus::Completed,
    )
    .with_worker_id(worker_id);
    assert!(HistoryStore::new(config_dir).write_terminal(&event));
}

async fn run_raw(fixture: &mut Fixture, argv: &[&str]) -> (i32, String, String) {
    fixture.out.0.clear();
    fixture.err.0.clear();
    let env_lookup = common::env_lookup(&fixture.env_map);
    let cwd = fixture.ws.path().to_string_lossy().into_owned();
    let mut io = CliIo {
        stdin: &mut fixture.stdin,
        stdout: &mut fixture.out,
        stderr: &mut fixture.err,
        env: &env_lookup,
        cwd,
    };
    let code = run_cli(
        &argv
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>(),
        &mut io,
        &fixture.hooks,
    )
    .await;
    drop(io);
    (
        code,
        String::from_utf8(fixture.out.0.clone()).unwrap(),
        String::from_utf8(fixture.err.0.clone()).unwrap(),
    )
}

fn result(output: &Value) -> &Value {
    &output["result"]
}

#[tokio::test(flavor = "current_thread")]
async fn history_list_missing_store_is_empty_and_does_not_write() {
    let mut fixture = Fixture::new();
    let history_dir = fixture.cfg.path().join("history");
    assert!(!history_dir.exists());

    let (code, output, _) = fixture.run(&["history", "list", "--format", "json"]).await;

    assert_eq!(code, 0, "{output:#}");
    assert_eq!(output["ok"], json!(true));
    assert_eq!(result(&output)["records"], json!([]));
    assert_eq!(result(&output)["scanComplete"], json!(true));
    assert_eq!(result(&output)["nextCursor"], Value::Null);

    let (show_code, missing, _) = fixture
        .run(&[
            "history",
            "show",
            "2f1f7fd9-4c5b-4dc3-b225-1f207a133215",
            "--format",
            "json",
        ])
        .await;
    assert_eq!(show_code, 0, "{missing:#}");
    assert_eq!(result(&missing)["record"], Value::Null);
    assert_eq!(result(&missing)["recordState"], "not-found");
    assert!(!history_dir.exists());
    assert!(fixture.usage_log().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn history_queries_filter_page_trace_and_show_without_writes() {
    let mut fixture = Fixture::new();
    let (identity_a, started_a) = write_started(fixture.cfg.path(), "worker-a");
    write_terminal(fixture.cfg.path(), &identity_a, "worker-a");
    let (_identity_b, _started_b) = write_started(fixture.cfg.path(), "worker-b");

    let (code, filtered, _) = fixture
        .run(&[
            "history",
            "list",
            "--since",
            "1h",
            "--worker",
            "worker-a",
            "--kind",
            "worker",
            "--status",
            "completed",
            "--format",
            "json",
        ])
        .await;
    assert_eq!(code, 0, "{filtered:#}");
    assert_eq!(result(&filtered)["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        result(&filtered)["records"][0]["event"]["workerId"],
        "worker-a"
    );
    assert_eq!(result(&filtered)["records"][0]["status"], "completed");

    let (code, first_page, _) = fixture
        .run(&["history", "list", "--limit", "1", "--format", "json"])
        .await;
    assert_eq!(code, 0, "{first_page:#}");
    assert_eq!(result(&first_page)["records"].as_array().unwrap().len(), 1);
    assert_eq!(result(&first_page)["scanComplete"], true);
    let cursor = result(&first_page)["nextCursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let first_id = result(&first_page)["records"][0]["event"]["executionId"]
        .as_str()
        .unwrap()
        .to_owned();

    let (code, second_page, _) = fixture
        .run(&[
            "history", "list", "--limit", "1", "--cursor", &cursor, "--format", "json",
        ])
        .await;
    assert_eq!(code, 0, "{second_page:#}");
    assert_eq!(result(&second_page)["records"].as_array().unwrap().len(), 1);
    assert_ne!(
        result(&second_page)["records"][0]["event"]["executionId"],
        first_id
    );
    assert_eq!(result(&second_page)["nextCursor"], Value::Null);

    let trace_id = identity_a.trace_id().to_string();
    let (code, trace, _) = fixture
        .run(&["history", "trace", &trace_id, "--format", "json"])
        .await;
    assert_eq!(code, 0, "{trace:#}");
    assert_eq!(result(&trace)["traceId"], trace_id);
    assert_eq!(result(&trace)["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        result(&trace)["records"][0]["event"]["executionId"],
        started_a.execution_id()
    );

    let execution_id = started_a.execution_id().to_owned();
    let (code, detail, _) = fixture
        .run(&["history", "show", &execution_id, "--format", "json"])
        .await;
    assert_eq!(code, 0, "{detail:#}");
    assert_eq!(result(&detail)["recordState"], "complete");
    assert_eq!(result(&detail)["record"]["started"]["event"], "started");
    assert_eq!(result(&detail)["record"]["terminal"]["status"], "completed");

    let execution_dirs = std::fs::read_dir(fixture.cfg.path().join("history"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .count();
    assert_eq!(execution_dirs, 2);
    assert!(fixture.usage_log().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn history_records_standalone_worker_resolution_failure_with_inherited_hints() {
    let mut fixture = Fixture::new();
    let trace_id = "123e4567-e89b-42d3-a456-426614174000";
    let parent_id = "123e4567-e89b-42d3-a456-426614174001";
    fixture
        .env_map
        .insert("CHEAPER_TRACE_ID".into(), trace_id.into());
    fixture
        .env_map
        .insert("CHEAPER_PARENT_EXECUTION_ID".into(), parent_id.into());

    let (code, failure, _) = fixture
        .run(&["run", "unknown-worker", "--format", "json"])
        .await;
    assert_eq!(code, 3, "{failure:#}");
    assert_eq!(failure["error"]["code"], "WORKER_NOT_FOUND");

    let (code, listing, _) = fixture.run(&["history", "list", "--format", "json"]).await;
    assert_eq!(code, 0, "{listing:#}");
    let records = result(&listing)["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    let event = &records[0]["event"];
    assert_eq!(event["kind"], "worker");
    assert_eq!(event["event"], "started");
    assert_eq!(event["traceId"], trace_id);
    assert_eq!(event["parentExecutionHintId"], parent_id);
    assert_eq!(event["contextProvenance"], "inherited-unverified");
    let execution_id = event["executionId"].as_str().unwrap();
    let (code, detail, _) = fixture
        .run(&["history", "show", execution_id, "--format", "json"])
        .await;
    assert_eq!(code, 0, "{detail:#}");
    assert_eq!(result(&detail)["record"]["terminal"]["status"], "failed");
    assert_eq!(
        result(&detail)["record"]["terminal"]["errorCode"],
        "WORKER_NOT_FOUND"
    );
    assert!(fixture.usage_log().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn standalone_run_and_usage_share_one_execution_identity() {
    let mut fixture = Fixture::new()
        .with_fake_stream(RUN_USAGE_FIXTURE)
        .with_input(r#"{"files":["a.txt"]}"#);
    let inherited_trace = "123e4567-e89b-42d3-a456-426614174002";
    let inherited_parent = "123e4567-e89b-42d3-a456-426614174003";
    fixture
        .env_map
        .insert("CHEAPER_TRACE_ID".into(), inherited_trace.into());
    fixture.env_map.insert(
        "CHEAPER_PARENT_EXECUTION_ID".into(),
        inherited_parent.into(),
    );
    let (code, output, stderr) = fixture.run(&["run", "fixture-worker"]).await;
    assert_eq!(code, 0, "{output:#}\n{stderr}");

    let usage = fixture.usage_log().expect("run Usage row");
    let rows: Vec<Value> = usage
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 1, "usage log: {usage}");
    assert_eq!(rows[0]["kind"], "run");
    let execution_id = rows[0]["executionId"].as_str().expect("executionId");
    let trace_id = rows[0]["traceId"].as_str().expect("traceId");

    let (code, listing, _) = fixture.run(&["history", "list", "--format", "json"]).await;
    assert_eq!(code, 0, "{listing:#}");
    let records = result(&listing)["records"].as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["event"]["executionId"], execution_id);
    assert_eq!(records[0]["event"]["traceId"], trace_id);
    assert_eq!(trace_id, inherited_trace);
    assert_eq!(
        records[0]["event"]["parentExecutionHintId"],
        inherited_parent
    );
    assert_eq!(
        records[0]["event"]["contextProvenance"],
        "inherited-unverified"
    );
    assert_eq!(records[0]["status"], "completed");
    assert!(rows[0].get("parentExecutionId").is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn history_text_defaults_to_text_and_marks_incomplete_records() {
    let mut fixture = Fixture::new();
    let (identity, started) = write_started(fixture.cfg.path(), "worker-a");
    let store = HistoryStore::new(fixture.cfg.path());
    let terminal_path = store
        .root_dir()
        .join(identity.execution_id().to_string())
        .join("terminal.json");
    std::fs::write(&terminal_path, b"{truncated").unwrap();

    let (code, output, _) = run_raw(&mut fixture, &["history", "list"]).await;
    assert_eq!(code, 0, "{output}");
    assert!(output.starts_with("ok history\n"), "{output}");
    assert!(output.contains("incomplete"), "{output}");
    assert!(fixture.usage_log().is_none());

    let incomplete_page = cheaper_cli::text_format::format_text(&json!({
        "ok": true,
        "worker": null,
        "result": {
            "records": [],
            "scanComplete": false,
            "nextCursor": null,
        },
    }));
    assert!(
        incomplete_page.contains("incomplete scan; narrow filters or restart query"),
        "{incomplete_page}"
    );

    let execution_id = started.execution_id().to_owned();
    let (code, detail, _) = run_raw(&mut fixture, &["history", "show", &execution_id]).await;
    assert_eq!(code, 0, "{detail}");
    assert!(detail.contains("recordState: incomplete"), "{detail}");
    assert!(
        !detail.contains("{truncated"),
        "raw event bytes leaked: {detail}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn history_rejects_bad_cursor_and_grammar_before_store_access() {
    let mut fixture = Fixture::new();
    let history_dir = fixture.cfg.path().join("history");

    for args in [
        vec!["history", "list", "--cursor", "bad", "--format", "json"],
        vec!["history", "list", "--limit", "0", "--format", "json"],
        vec!["history", "show", "not-a-uuid", "--format", "json"],
        vec![
            "history",
            "show",
            "2f1f7fd9-4c5b-4dc3-b225-1f207a133215",
            "--limit",
            "2",
            "--format",
            "json",
        ],
        vec!["history", "list", "--since", "2d", "--format", "json"],
    ] {
        let (code, output, _) = fixture.run(&args).await;
        assert_eq!(code, 2, "args={args:?}, output={output:#}");
        assert_eq!(output["error"]["code"], "INVALID_REQUEST");
        assert_ne!(
            output["error"]["message"], "unknown command: history",
            "history grammar must be recognized: args={args:?}"
        );
        assert!(!history_dir.exists(), "args={args:?}");
        assert!(fixture.usage_log().is_none(), "args={args:?}");
    }
}
