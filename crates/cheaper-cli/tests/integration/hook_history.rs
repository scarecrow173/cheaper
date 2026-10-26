//! Hook decision History contracts: typed reasons are captured at the
//! evaluator, native responses remain unchanged, and cache followers do not
//! create duplicate decision receipts.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::hook::{big_text, run_hook, tool_use_payload, windsurf_read, windsurf_shell, Fixture};

fn windsurf_range_read(file_path: &str, offset: u64, limit: u64) -> String {
    json!({
        "agent_action_name": "pre_read_code",
        "tool_info": {
            "file_path": file_path,
            "offset": offset,
            "limit": limit
        }
    })
    .to_string()
}

fn history_root(fx: &Fixture) -> PathBuf {
    Path::new(&fx.env["CHEAPER_CONFIG_DIR"]).join("history")
}

fn history_decisions(fx: &Fixture) -> Vec<(PathBuf, Value)> {
    let mut records = Vec::new();
    let Ok(entries) = std::fs::read_dir(history_root(fx)) else {
        return records;
    };
    for entry in entries.flatten() {
        let path = entry.path().join("decision.json");
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if let Ok(value) = serde_json::from_slice(&bytes) {
            records.push((path, value));
        }
    }
    records
}

fn only_decision(fx: &Fixture) -> Value {
    let records = history_decisions(fx);
    assert_eq!(records.len(), 1, "expected one Hook decision: {records:?}");
    records.into_iter().next().unwrap().1
}

fn assert_hook_decision(record: &Value, reason: &str, evaluator: &str, native: &str) {
    assert_eq!(record["kind"], "hook-decision");
    assert_eq!(record["event"], "decision");
    assert_eq!(record["status"], "completed");
    assert_eq!(record["reasonCode"], reason);
    assert_eq!(record["evaluatorVerdict"], evaluator);
    assert_eq!(record["nativeVerdict"], native);
    assert!(
        record.get("verdict").is_none(),
        "Hook uses its paired verdict fields"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_records_read_advice_as_a_decision_without_a_worker_run() {
    let fx = Fixture::new();
    fx.write_big("private-big.txt");
    let response = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("private-big.txt"),
        &[],
    )
    .await;

    assert_eq!(response.code, 2);
    assert!(response.stdout.is_empty());
    assert!(response.stderr.contains("Do not read it directly."));
    let record = only_decision(&fx);
    assert_hook_decision(
        &record,
        "read.threshold_exceeded_delegate_advised",
        "deny-delegation-advised",
        "deny",
    );
    assert_eq!(record["fileBytes"], 108_000);
    assert_eq!(record["threshold"], 8192);
    assert!(record["measuredTokens"].as_u64().unwrap() > 8192);
    assert_eq!(record["hookCacheProvenance"], "leader-evaluated");
    assert!(!serde_json::to_string(&record)
        .unwrap()
        .contains("private-big.txt"));
    assert!(!Path::new(&fx.env["CHEAPER_CONFIG_DIR"])
        .join("usage.jsonl")
        .exists());
    assert_eq!(
        history_decisions(&fx).len(),
        1,
        "Deny is not a Worker start"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_records_byte_fast_path_with_unmeasured_tokens_as_null() {
    let fx = Fixture::new();
    fx.write("small.txt", b"hello\n");
    let response = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("small.txt"),
        &[],
    )
    .await;

    assert_eq!(response.code, 0);
    assert!(response.stdout.is_empty());
    assert!(response.stderr.is_empty());
    let record = only_decision(&fx);
    assert_hook_decision(&record, "read.byte_fast_path", "allow", "allow");
    assert_eq!(record["fileBytes"], 6);
    assert_eq!(record["threshold"], 8192);
    assert!(record["measuredTokens"].is_null());
    assert_eq!(record["hookCacheProvenance"], "bypass");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_uses_inherited_carriers_only_as_unverified_hints() {
    let mut fx = Fixture::new();
    fx.write("small.txt", b"hello\n");
    fx.env.insert(
        "CHEAPER_TRACE_ID".to_string(),
        "123e4567-e89b-42d3-a456-426614174000".to_string(),
    );
    fx.env.insert(
        "CHEAPER_PARENT_EXECUTION_ID".to_string(),
        "123e4567-e89b-42d3-a456-426614174001".to_string(),
    );
    let response = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("small.txt"),
        &[],
    )
    .await;
    assert_eq!(response.code, 0);

    let record = only_decision(&fx);
    assert_eq!(record["traceId"], "123e4567-e89b-42d3-a456-426614174000");
    assert_eq!(
        record["parentExecutionHintId"],
        "123e4567-e89b-42d3-a456-426614174001"
    );
    assert_eq!(record["contextProvenance"], "inherited-unverified");
    assert!(record.get("parentExecutionId").is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_records_measured_read_below_threshold_and_measurement_limit() {
    let measured = Fixture::new();
    let contents = "word ".repeat(1_639);
    assert!(contents.len() > 8192);
    measured.write("medium.txt", contents.as_bytes());
    let response = run_hook(
        &measured,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("medium.txt"),
        &[],
    )
    .await;
    assert_eq!(response.code, 0);
    let record = only_decision(&measured);
    assert_hook_decision(&record, "read.below_threshold", "allow", "allow");
    assert_eq!(record["fileBytes"], contents.len());
    assert!(record["measuredTokens"].as_u64().unwrap() < 8192);

    let limited = Fixture::new();
    limited.write_big("big.txt");
    let response = run_hook(
        &limited,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[("CHEAPER_COUNT_MAX_BYTES", "10")],
    )
    .await;
    assert_eq!(response.code, 2);
    let record = only_decision(&limited);
    assert_hook_decision(
        &record,
        "read.measurement_limited",
        "deny-delegation-advised",
        "deny",
    );
    assert_eq!(record["measuredTokens"], Value::Null);
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_maps_empty_range_and_byte_bounded_window_from_source_evidence() {
    let contents = "line\n".repeat(3_000);
    assert!(contents.len() > 8192);

    let empty = Fixture::new();
    empty.write("lines.txt", contents.as_bytes());
    let response = run_hook(
        &empty,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_range_read("lines.txt", 9_999, 1),
        &[],
    )
    .await;
    assert_eq!(response.code, 0);
    let record = only_decision(&empty);
    assert_hook_decision(&record, "read.no_candidate", "allow", "allow");
    assert_eq!(record["fileBytes"], contents.len());
    assert!(record["measuredTokens"].is_null());

    let bounded = Fixture::new();
    bounded.write("lines.txt", contents.as_bytes());
    let response = run_hook(
        &bounded,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_range_read("lines.txt", 1, 1),
        &[],
    )
    .await;
    assert_eq!(response.code, 0);
    let record = only_decision(&bounded);
    assert_hook_decision(&record, "read.byte_fast_path", "allow", "allow");
    assert_eq!(record["fileBytes"], contents.len());
    assert!(record["measuredTokens"].is_null());
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_cache_leader_records_once_and_cache_hit_adds_no_receipt() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let first = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;
    let second = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;

    assert_eq!(first.code, second.code);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(history_decisions(&fx).len(), 1);
    assert_eq!(
        only_decision(&fx)["hookCacheProvenance"],
        "leader-evaluated"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_fallback_and_store_failure_preserve_native_response() {
    let baseline = Fixture::new();
    baseline.write_big("big.txt");
    let baseline_response = run_hook(
        &baseline,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;

    let fallback = Fixture::new();
    fallback.write_big("big.txt");
    let fallback_config = Path::new(&fallback.env["CHEAPER_CONFIG_DIR"]);
    std::fs::create_dir_all(fallback_config).unwrap();
    std::fs::write(
        fallback_config.join("hook-eval"),
        b"unwritable dedup directory",
    )
    .unwrap();
    let fallback_response = run_hook(
        &fallback,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;
    assert_eq!(fallback_response.code, baseline_response.code);
    assert_eq!(fallback_response.stdout, baseline_response.stdout);
    assert_eq!(fallback_response.stderr, baseline_response.stderr);
    assert_eq!(only_decision(&fallback)["hookCacheProvenance"], "fallback");

    let history_failure = Fixture::new();
    history_failure.write_big("big.txt");
    std::fs::create_dir_all(&history_failure.env["CHEAPER_CONFIG_DIR"]).unwrap();
    let history_path = history_root(&history_failure);
    std::fs::write(&history_path, b"history store unavailable").unwrap();
    let failed_write_response = run_hook(
        &history_failure,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;
    assert_eq!(failed_write_response.code, baseline_response.code);
    assert_eq!(failed_write_response.stdout, baseline_response.stdout);
    assert_eq!(failed_write_response.stderr, baseline_response.stderr);
    assert!(history_path.is_file());
    assert!(history_decisions(&history_failure).is_empty());
}

struct SearchReasonCase<'a> {
    path: &'a str,
    contents: &'a [u8],
    pattern: &'a str,
    extra_env: &'a [(&'a str, &'a str)],
    expected_reason: &'a str,
    expect_threshold: bool,
    expected_evaluator: &'a str,
    expected_native: &'a str,
}

async fn assert_search_reason(case: SearchReasonCase<'_>) {
    let SearchReasonCase {
        path,
        contents,
        pattern,
        extra_env,
        expected_reason,
        expect_threshold,
        expected_evaluator,
        expected_native,
    } = case;
    let fx = Fixture::new();
    fx.write("search.txt", contents);
    let tool_input = json!({
        "path": path,
        "pattern": pattern,
        "max_results": 1
    })
    .to_string();
    let response = run_hook(
        &fx,
        &["pre-tool-use", "--target", "pretooluse"],
        &tool_use_payload("grep", &tool_input),
        extra_env,
    )
    .await;
    assert_eq!(response.code, 0);
    let record = only_decision(&fx);
    assert_hook_decision(
        &record,
        expected_reason,
        expected_evaluator,
        expected_native,
    );
    if expect_threshold {
        assert_eq!(record["threshold"], 8192);
    } else {
        assert!(record.get("threshold").is_none());
    }
    let serialized = serde_json::to_string(&record).unwrap();
    assert!(!serialized.contains(&serde_json::to_string(path).unwrap()));
    if !pattern.is_empty() {
        assert!(!serialized.contains(&serde_json::to_string(pattern).unwrap()));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_search_reason_mapping_covers_evaluator_branches() {
    assert_search_reason(SearchReasonCase {
        path: "search.txt",
        contents: b"ordinary text\n",
        pattern: "no-match",
        extra_env: &[],
        expected_reason: "search.no_candidate",
        expect_threshold: true,
        expected_evaluator: "allow",
        expected_native: "allow",
    })
    .await;
    assert_search_reason(SearchReasonCase {
        path: "search.txt",
        contents: b"ordinary text\n",
        pattern: "",
        extra_env: &[],
        expected_reason: "search.no_candidate",
        expect_threshold: false,
        expected_evaluator: "allow",
        expected_native: "allow",
    })
    .await;
    assert_search_reason(SearchReasonCase {
        path: "../outside.txt",
        contents: b"ordinary text\n",
        pattern: "hit",
        extra_env: &[],
        expected_reason: "search.no_candidate",
        expect_threshold: false,
        expected_evaluator: "allow",
        expected_native: "allow",
    })
    .await;
    assert_search_reason(SearchReasonCase {
        path: "search.txt",
        contents: b"small hit\n",
        pattern: "hit",
        extra_env: &[],
        expected_reason: "search.below_threshold",
        expect_threshold: true,
        expected_evaluator: "allow",
        expected_native: "allow",
    })
    .await;
    let measured_below = "word ".repeat(1_800);
    assert!(measured_below.len() > 8192);
    assert_search_reason(SearchReasonCase {
        path: "search.txt",
        contents: measured_below.as_bytes(),
        pattern: "word",
        extra_env: &[],
        expected_reason: "search.below_threshold",
        expect_threshold: true,
        expected_evaluator: "allow",
        expected_native: "allow",
    })
    .await;
    let big = big_text();
    assert_search_reason(SearchReasonCase {
        path: "search.txt",
        contents: big.as_bytes(),
        pattern: "lorem",
        extra_env: &[],
        expected_reason: "search.threshold_exceeded_delegate_advised",
        expect_threshold: true,
        expected_evaluator: "deny-delegation-advised",
        expected_native: "deny",
    })
    .await;
    assert_search_reason(SearchReasonCase {
        path: "search.txt",
        contents: b"a line larger than a one byte scan cap\n",
        pattern: "line",
        extra_env: &[("CHEAPER_COUNT_MAX_BYTES", "1")],
        expected_reason: "search.scan_limited",
        expect_threshold: true,
        expected_evaluator: "deny-delegation-advised",
        expected_native: "deny",
    })
    .await;
    assert_search_reason(SearchReasonCase {
        path: "search.txt",
        contents: b"search data\n",
        pattern: "[",
        extra_env: &[],
        expected_reason: "search.measurement_unavailable",
        expect_threshold: true,
        expected_evaluator: "allow",
        expected_native: "allow",
    })
    .await;
}

fn install_cat_route(fx: &Fixture, with_grant: bool) {
    let cfg = Path::new(&fx.env["CHEAPER_CONFIG_DIR"]);
    std::fs::create_dir_all(cfg).unwrap();
    std::fs::write(
        cfg.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: cat-route\n    match: {command: [cat]}\n    target: {kind: workflow, path: w.json}\n    grant: g\n",
    )
    .unwrap();
    std::fs::write(cfg.join("w.json"), r#"{"version":1,"jobs":{}}"#).unwrap();
    let policy = if with_grant {
        let executable = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        format!(
            "version: 1\ncommand:\n  grants:\n    g:\n      executable: \"{executable}\"\n      argv: [\"*\"]\n"
        )
    } else {
        "version: 1\ncommand:\n  grants: {}\n".to_string()
    };
    std::fs::write(cfg.join("policy.yaml"), policy).unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_rewrite_and_unsupported_deny_fallback_are_decision_only() {
    let fx = Fixture::new();
    install_cat_route(&fx, true);
    fx.write_big("big.txt");
    let response = run_hook(
        &fx,
        &["pre-run-command", "--target", "windsurf"],
        &windsurf_shell("cat big.txt"),
        &[],
    )
    .await;

    assert_eq!(response.code, 2);
    assert!(response
        .stderr
        .contains("blocked shell command 'cat big.txt'."));
    let record = only_decision(&fx);
    assert_hook_decision(&record, "route.rewrite_emitted", "rewrite-emitted", "deny");
    assert_eq!(
        record["fallbackReason"],
        "route.rewrite_unsupported_fallback_deny"
    );
    assert_eq!(record["routeId"], "cat-route");
    assert_eq!(record["viability"], "runnable");
    assert_eq!(record["hookCacheProvenance"], "leader-evaluated");
    assert_eq!(record["fileBytes"], 108_000);
    assert!(!Path::new(&fx.env["CHEAPER_CONFIG_DIR"])
        .join("usage.jsonl")
        .exists());
    assert_eq!(
        history_decisions(&fx).len(),
        1,
        "Rewrite is not a Route run"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_supported_rewrite_is_not_an_actual_route_run() {
    let fx = Fixture::new();
    install_cat_route(&fx, true);
    fx.write_big("big.txt");
    let response = run_hook(
        &fx,
        &["pre-tool-use", "--target", "pretooluse"],
        &tool_use_payload("bash", "{\"command\":\"cat big.txt\"}"),
        &[],
    )
    .await;

    assert_eq!(response.code, 0);
    let stdout: Value = serde_json::from_str(&response.stdout).unwrap();
    assert!(stdout
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(Value::as_str)
        .is_some_and(|command| command.contains("route run --payload")));
    let record = only_decision(&fx);
    assert_hook_decision(
        &record,
        "route.rewrite_emitted",
        "rewrite-emitted",
        "rewrite",
    );
    assert!(record.get("fallbackReason").is_none());
    assert_eq!(record["routeId"], "cat-route");
    assert_eq!(record["viability"], "runnable");
    assert_eq!(record["hookCacheProvenance"], "leader-evaluated");
    assert_eq!(history_decisions(&fx).len(), 1, "Hook only emits advice");
    assert!(!Path::new(&fx.env["CHEAPER_CONFIG_DIR"])
        .join("usage.jsonl")
        .exists());
}

#[tokio::test(flavor = "current_thread")]
async fn hook_history_route_no_match_and_no_grant_remain_typed_decisions() {
    let no_match = Fixture::new();
    install_cat_route(&no_match, true);
    let response = run_hook(
        &no_match,
        &["pre-run-command", "--target", "windsurf"],
        &windsurf_shell("echo plain"),
        &[],
    )
    .await;
    assert_eq!(response.code, 0);
    let record = only_decision(&no_match);
    assert_hook_decision(&record, "route.no_match", "allow", "allow");
    assert!(record.get("viability").is_none());

    let no_grant = Fixture::new();
    install_cat_route(&no_grant, false);
    no_grant.write("small.txt", b"small\n");
    let response = run_hook(
        &no_grant,
        &["pre-run-command", "--target", "windsurf"],
        &windsurf_shell("cat small.txt"),
        &[],
    )
    .await;
    assert_eq!(response.code, 0);
    let record = only_decision(&no_grant);
    assert_hook_decision(&record, "route.no_grant", "allow", "allow");
    assert_eq!(record["viability"], "no-grant");
    assert_eq!(record["routeId"], "cat-route");
}
