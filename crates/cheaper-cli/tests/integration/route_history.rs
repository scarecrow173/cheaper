//! Route/refetch execution History contracts.

use std::collections::BTreeMap;
use std::io::Write;

use crate::common::{self, Fixture};
use cheaper_cli::cli::{run_cli, CliIo};
use cheaper_cli::route::log_store::RouteLogStore;
use cheaper_cli::route::payload::{encode, EncodeInput};
use serde_json::Value;

fn history_events(f: &Fixture) -> Vec<Value> {
    std::fs::read_dir(f.cfg.path().join("history"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read(entry.path().join("started.json")).ok())
        .chain(
            std::fs::read_dir(f.cfg.path().join("history"))
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter_map(|entry| std::fs::read(entry.path().join("decision.json")).ok()),
        )
        .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
        .collect()
}

fn history_terminal(f: &Fixture, execution_id: &str) -> Option<Value> {
    std::fs::read(
        f.cfg
            .path()
            .join("history")
            .join(execution_id)
            .join("terminal.json"),
    )
    .ok()
    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn usage_records(f: &Fixture) -> Vec<Value> {
    f.usage_log()
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

async fn run_raw(f: &mut Fixture, argv: &[&str]) -> (i32, String) {
    f.out.0.clear();
    f.err.0.clear();
    let env = common::env_lookup(&f.env_map);
    let cwd = f.ws.path().to_string_lossy().into_owned();
    let mut io = CliIo {
        stdin: &mut f.stdin,
        stdout: &mut f.out,
        stderr: &mut f.err,
        env: &env,
        cwd,
    };
    let args = argv
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    let code = run_cli(&args, &mut io, &f.hooks).await;
    drop(io);
    (code, String::from_utf8(f.out.0.clone()).unwrap())
}

fn make_log(f: &Fixture, body: &[u8]) -> String {
    let store = RouteLogStore::new(f.cfg.path());
    let mut writer = store.begin().unwrap();
    let id = writer.run_id().to_owned();
    writer.write_all(body).unwrap();
    writer.finish().unwrap();
    id
}

fn route_token(f: &Fixture, command: String, argv: Vec<String>) -> String {
    encode(EncodeInput {
        executable: argv.first().cloned().unwrap_or_default(),
        args: argv.iter().skip(1).cloned().collect(),
        argv,
        command,
        cwd: f.ws.path().to_string_lossy().into_owned(),
        ambient_env: BTreeMap::new(),
        passthrough_env: Vec::new(),
    })
    .unwrap()
}

fn history_routed_fixture_for_child(
    child_test: &str,
    include_ignored: bool,
) -> (Fixture, String, Vec<String>) {
    let f = Fixture::new();
    let executable = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut argv = vec![
        executable.clone(),
        format!("route_history::{child_test}"),
        "--exact".into(),
        "--nocapture".into(),
    ];
    if include_ignored {
        argv.push("--include-ignored".into());
    }
    let command = format!("\"{}\" {}", executable, argv[1..].join(" "));
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        format!(
            "version: 1\nroutes:\n  - name: history-route\n    match: {{command: ['{}']}}\n    target: {{kind: workflow, path: history-route.workflow.json}}\n    grant: history-route\n",
            executable.replace('\'', "''")
        ),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("history-route.workflow.json"),
        r#"{"schemaVersion":1,"inputs":{"grant":{"type":"string"},"executable":{"type":"string"},"argv":{"type":"array","items":{"type":"string"}},"args":{"type":"array","items":{"type":"string"}},"execArgs":{"type":"array","items":{"type":"string"}},"cwd":{"type":"string"},"tokenThreshold":{"type":"number"},"runId":{"type":"string"}},"jobs":{"exec":{"kind":"tool","tool":"command","input":{"name":{"$input":"grant"},"args":{"$input":"execArgs"}}}},"outputs":{"execResult":{"schema":{"type":"object"},"value":{"$from":"exec","path":""}}}}"#,
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    history-route:\n      executable: '{}'\n      argv: [{}]\n      timeout: '60s'\n",
            executable.replace('\'', "''"),
            argv[1..]
                .iter()
                .map(|arg| format!("'{}'", arg.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
    .unwrap();
    (f, command, argv)
}

fn history_routed_fixture() -> (Fixture, String, Vec<String>) {
    history_routed_fixture_for_child("command_fixture", false)
}

#[test]
fn command_fixture() {
    print!("ROUTE-HISTORY-CHILD");
}

#[test]
#[ignore]
fn command_fixture_nonzero() {
    std::process::exit(7);
}

#[cfg(windows)]
#[test]
#[ignore]
fn command_fixture_i32_min_exit_code() {
    std::process::exit(i32::MIN);
}

#[cfg(windows)]
#[test]
#[ignore]
fn command_fixture_i32_max_exit_code() {
    std::process::exit(i32::MAX);
}

#[tokio::test(flavor = "current_thread")]
async fn refetch_has_its_own_history_and_usage_identity_joined_by_log_ref() {
    let mut f = Fixture::new();
    let log_ref = make_log(&f, b"route output");

    let (code, output, _) = f.run(&["route", "logs", &log_ref]).await;

    assert_eq!(code, 0, "{output:#}");
    let started = history_events(&f);
    assert_eq!(started.len(), 1);
    let event = &started[0];
    assert_eq!(event["kind"], "route-refetch");
    assert_eq!(event["event"], "started");
    assert_eq!(event["logRef"], log_ref);
    let execution_id = event["executionId"].as_str().unwrap();
    assert_ne!(execution_id, log_ref);
    assert_eq!(
        history_terminal(&f, execution_id).as_ref().unwrap()["status"],
        "completed"
    );

    let records = usage_records(&f);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["kind"], "route-refetch");
    assert_eq!(records[0]["executionId"], execution_id);
    assert_eq!(records[0]["logRef"], log_ref);
}

#[tokio::test(flavor = "current_thread")]
async fn failed_refetch_closes_history_before_returning_without_usage() {
    let mut f = Fixture::new();
    let missing = make_log(&f, b"remove before fetching");
    std::fs::remove_file(
        RouteLogStore::new(f.cfg.path())
            .dir()
            .join(format!("{missing}.log")),
    )
    .unwrap();

    let (code, _, _) = f.run(&["route", "logs", &missing]).await;

    assert_ne!(code, 0);
    let started = history_events(&f);
    assert_eq!(started.len(), 1);
    assert_eq!(started[0]["kind"], "route-refetch");
    assert_eq!(started[0]["logRef"], missing);
    let execution_id = started[0]["executionId"].as_str().unwrap();
    assert_eq!(
        history_terminal(&f, execution_id).as_ref().unwrap()["status"],
        "failed"
    );
    assert!(usage_records(&f).is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn route_resolve_is_read_only_for_history_and_usage() {
    let mut f = Fixture::new();

    let (code, output, _) = f
        .run(&["route", "resolve", "--command", "git status"])
        .await;

    assert_eq!(code, 0, "{output:#}");
    assert!(!f.cfg.path().join("history").exists());
    assert!(f.usage_log().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn route_run_assigns_distinct_execution_and_workflow_parent_ids() {
    let (mut f, command, argv) = history_routed_fixture();
    let token = route_token(&f, command, argv);

    let (code, output) = run_raw(&mut f, &["route", "run", "--payload", &token]).await;

    assert_eq!(code, 0, "{output}");
    assert!(output.contains("[cheaper route log:"), "{output}");
    let events = history_events(&f);
    let route = events
        .iter()
        .find(|event| event["kind"] == "route")
        .unwrap();
    let workflow = events
        .iter()
        .find(|event| event["kind"] == "workflow")
        .unwrap();
    assert_eq!(route["routeId"], "history-route");
    assert_eq!(workflow["parentExecutionId"], route["executionId"]);
    assert_ne!(workflow["executionId"], route["executionId"]);

    let route_id = route["executionId"].as_str().unwrap();
    let route_terminal = history_terminal(&f, route_id).unwrap();
    assert_eq!(route_terminal["event"], "terminal");
    assert_eq!(route_terminal["status"], "completed");
    assert_eq!(route_terminal["exitCode"], 0);
    let records = usage_records(&f);
    let route_usage = records
        .iter()
        .find(|record| record["kind"] == "route")
        .unwrap();
    assert_eq!(route_usage["executionId"], route_id);
    assert_eq!(route_usage["traceId"], route["traceId"]);
}

#[tokio::test(flavor = "current_thread")]
async fn route_run_projects_nonzero_exit_code_to_terminal_history() {
    let (mut f, command, argv) = history_routed_fixture_for_child("command_fixture_nonzero", true);
    let token = route_token(&f, command, argv);

    let (code, output) = run_raw(&mut f, &["route", "run", "--payload", &token]).await;

    assert_eq!(code, 7, "{output}");
    let route = history_events(&f)
        .into_iter()
        .find(|event| event["kind"] == "route")
        .unwrap();
    let route_id = route["executionId"].as_str().unwrap();
    let terminal = history_terminal(&f, route_id).unwrap();
    assert_eq!(terminal["status"], "failed");
    assert_eq!(terminal["exitCode"], 7);
}

#[cfg(windows)]
#[tokio::test(flavor = "current_thread")]
async fn route_run_projects_signed_exit_code_boundaries_to_terminal_history() {
    for (child_test, expected) in [
        ("command_fixture_i32_min_exit_code", i32::MIN),
        ("command_fixture_i32_max_exit_code", i32::MAX),
    ] {
        let (mut f, command, argv) = history_routed_fixture_for_child(child_test, true);
        let token = route_token(&f, command, argv);

        let (code, output) = run_raw(&mut f, &["route", "run", "--payload", &token]).await;

        assert_ne!(code, 0, "{output}");
        let route = history_events(&f)
            .into_iter()
            .find(|event| event["kind"] == "route")
            .unwrap();
        let route_id = route["executionId"].as_str().unwrap();
        let terminal = history_terminal(&f, route_id).unwrap();
        assert_eq!(terminal["exitCode"], expected);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn route_run_error_terminal_omits_unobserved_exit_code() {
    let (mut f, command, argv) = history_routed_fixture();
    std::fs::write(
        f.cfg.path().join("history-route.workflow.json"),
        "not valid workflow JSON",
    )
    .unwrap();
    let token = route_token(&f, command, argv);

    let (code, output) = run_raw(&mut f, &["route", "run", "--payload", &token]).await;

    assert_ne!(code, 0, "{output}");
    let route = history_events(&f)
        .into_iter()
        .find(|event| event["kind"] == "route")
        .unwrap();
    let route_id = route["executionId"].as_str().unwrap();
    let terminal = history_terminal(&f, route_id).unwrap();
    assert_eq!(terminal["status"], "failed");
    assert!(terminal.get("exitCode").is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn rejected_route_run_records_only_a_decision() {
    let mut f = Fixture::new();
    let command = "git status".to_string();
    let token = route_token(&f, command, vec!["git".into(), "status".into()]);

    let (code, _, _) = f.run(&["route", "run", "--payload", &token]).await;

    assert_ne!(code, 0);
    let events = history_events(&f);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "route-decision");
    assert_eq!(events[0]["event"], "decision");
    assert_eq!(events[0]["reasonCode"], "route.no_match");
    let execution_id = events[0]["executionId"].as_str().unwrap();
    assert!(!f
        .cfg
        .path()
        .join("history")
        .join(execution_id)
        .join("started.json")
        .exists());
    assert!(usage_records(&f).is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn history_write_failure_does_not_change_route_result_or_usage() {
    let (mut f, command, argv) = history_routed_fixture();
    std::fs::write(f.cfg.path().join("history"), "history unavailable").unwrap();
    let token = route_token(&f, command, argv);

    let (code, output) = run_raw(&mut f, &["route", "run", "--payload", &token]).await;

    assert_eq!(code, 0, "{output}");
    assert!(output.contains("[cheaper route log:"), "{output}");
    assert!(f.cfg.path().join("history").is_file());
    let records = usage_records(&f);
    let route_usage = records
        .iter()
        .find(|record| record["kind"] == "route")
        .expect("Route Usage remains available when History writes fail");
    assert!(route_usage["executionId"].is_string());
    assert!(route_usage["traceId"].is_string());
}
