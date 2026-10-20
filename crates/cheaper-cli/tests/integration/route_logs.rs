use std::io::Write;

use crate::common::Fixture;
use cheaper_cli::route::log_store::RouteLogStore;
use serde_json::Value;

fn make_log(f: &Fixture, body: &[u8]) -> String {
    let store = RouteLogStore::new(f.cfg.path());
    let mut writer = store.begin().unwrap();
    let id = writer.run_id().to_string();
    writer.write_all(body).unwrap();
    writer.finish().unwrap();
    id
}

fn records(f: &Fixture) -> Vec<Value> {
    f.usage_log()
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect()
}

fn refetch_records(f: &Fixture) -> Vec<Value> {
    records(f)
        .into_iter()
        .filter(|r| r["kind"] == "route-refetch")
        .collect()
}

#[tokio::test(flavor = "current_thread")]
async fn logs_reads_paged_and_eof() {
    let mut f = Fixture::new();
    let id = make_log(&f, b"0123456789");
    let (code, out, _) = f
        .run(&["route", "logs", &id, "--offset", "4", "--max-bytes", "3"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["data"], "456");
    assert_eq!(out["result"]["offset"], 4);
    assert_eq!(out["result"]["nextOffset"], 7);
    assert_eq!(out["result"]["eof"], false);
    let (code, out, _) = f
        .run(&["route", "logs", &id, "--offset", "7", "--max-bytes", "100"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["data"], "789");
    assert_eq!(out["result"]["eof"], true);
    let (code, out, _) = f
        .run(&["route", "logs", &id, "--offset", "10", "--max-bytes", "8"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["data"], "");
    assert_eq!(out["result"]["offset"], 10);
    assert_eq!(out["result"]["eof"], true);
    let fetched = refetch_records(&f);
    assert_eq!(fetched.len(), 3);
    assert_eq!(fetched[0]["logRef"], id);
    assert_eq!(fetched[0]["bytesRead"], 3);
    assert_eq!(fetched[2]["bytesRead"], 0);
    assert!(records(&f)
        .iter()
        .all(|r| r["kind"] != "run" && r["kind"] != "count" && r["kind"] != "route"));
}

#[tokio::test(flavor = "current_thread")]
async fn logs_rejects_bad_offsets_and_ids() {
    let mut f = Fixture::new();
    let id = make_log(&f, b"0123456789");
    for argv in [
        vec!["route", "logs", &id, "--offset", "11", "--max-bytes", "8"],
        vec!["route", "logs", "1-2-abc"],
        vec!["route", "logs", "../x"],
        vec!["route", "logs", "a/b"],
        vec!["route", "logs", "a\\b"],
        vec!["route", "logs", "1-2-a.log"],
        vec!["route", "logs", "1-2-ABC"],
        vec!["route", "logs", ""],
    ] {
        let (code, _, _) = f.run(&argv).await;
        assert_ne!(code, 0, "must reject: {argv:?}");
    }
    assert!(refetch_records(&f).is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn logs_default_max_obeys_policy_cap() {
    let mut f = Fixture::new();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        "version: 1\nworkflow:\n  artifacts:\n    maxReadBytes: 4\n",
    )
    .unwrap();
    let id = make_log(&f, b"0123456789");
    let (code, out, _) = f.run(&["route", "logs", &id]).await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["data"], "0123");
    assert_eq!(out["result"]["nextOffset"], 4);
    let (code, _, _) = f.run(&["route", "logs", &id, "--max-bytes", "5"]).await;
    assert_ne!(code, 0, "request above cap must reject");
    assert_eq!(refetch_records(&f).len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn logs_rejects_malformed_numeric_flags() {
    let mut f = Fixture::new();
    let id = make_log(&f, b"0123456789");
    for extra in [
        vec!["--max-bytes", "0"],
        vec!["--offset", "-1"],
        vec!["--offset", "+1"],
        vec!["--offset", "1.5"],
        vec!["--offset", "1e3"],
        vec!["--offset", ""],
        vec!["--offset", "9007199254740992"],
        vec!["--max-bytes", "18446744073709551616"],
    ] {
        let mut argv = vec!["route", "logs", &id];
        argv.extend(extra);
        let (code, _, _) = f.run(&argv).await;
        assert_ne!(code, 0, "must reject: {argv:?}");
    }
    assert!(refetch_records(&f).is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn logs_numeric_flags_rejected_on_other_commands() {
    let mut f = Fixture::new();
    for argv in [
        vec!["models", "--offset", "1"],
        vec!["count-tokens", "x", "--max-bytes", "4"],
        vec!["run", "--offset", "1"],
    ] {
        let (code, _, _) = f.run(&argv).await;
        assert_ne!(code, 0, "must reject: {argv:?}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn logs_usage_append_failure_keeps_success() {
    let mut f = Fixture::new();
    std::fs::create_dir_all(f.cfg.path().join("usage.jsonl")).unwrap();
    let id = make_log(&f, b"abc");
    let (code, out, err) = f.run(&["route", "logs", &id]).await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["data"], "abc");
    assert!(err.contains("usage-log"), "diagnostic expected: {err}");
    let mut f2 = Fixture::new();
    std::fs::create_dir_all(f2.cfg.path().join("usage.jsonl")).unwrap();
    let id2 = make_log(&f2, b"def");
    let (code, out, err) = f2.run(&["route", "logs", &id2, "--quiet"]).await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["data"], "def");
    assert!(!err.contains("usage-log"), "quiet must suppress: {err}");
}

#[tokio::test(flavor = "current_thread")]
async fn logs_read_works_with_malformed_routes() {
    let mut f = Fixture::new();
    std::fs::write(f.cfg.path().join("routes.yaml"), "!!! [broken").unwrap();
    let id = make_log(&f, b"survives");
    let (code, out, _) = f.run(&["route", "logs", &id]).await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["data"], "survives");
}

#[tokio::test(flavor = "current_thread")]
async fn logs_offsets_are_bytes_not_chars() {
    let mut f = Fixture::new();
    let id = make_log(&f, "あいう".as_bytes());
    let (code, out, _) = f
        .run(&["route", "logs", &id, "--offset", "3", "--max-bytes", "3"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(out["result"]["offset"], 3);
    assert_eq!(out["result"]["nextOffset"], 6);
    assert_eq!(out["result"]["eof"], false);
    assert_eq!(refetch_records(&f)[0]["bytesRead"], 3);
}
