//! `usage_report.rs` 集計/`execute_usage` と `pricing.rs` 連携の test
//! (`packages/cli/test/usage-report.test.ts` の port + dispatch 経路)。

use std::collections::HashMap;
use std::path::Path;

use crate::common;
use common::{ambient_env, PipeStdin, SharedVecSink};

use cheaper_cli::cli::{run_cli, CliHooks, CliIo};
use cheaper_cli::usage_report::{
    build_usage_report, read_usage_records, UsageFilters, UsageGroupBy, PRICING_STALE_MS,
};
use cheaper_core::errors::ErrorCode;
use cheaper_core::model::classify::ModelInfo;
use cheaper_core::model::pricing::ModelCost;
use serde_json::{json, Map, Value};

/// `Date.parse("2026-09-20T00:00:00.000Z")` = 1789862400000。
const NOW_MS: f64 = 1_789_862_400_000.0;

fn now() -> f64 {
    NOW_MS
}

fn write_log(dir: &Path, records: &[Value]) {
    let mut text = records
        .iter()
        .map(|r| serde_json::to_string(r).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    std::fs::write(dir.join("usage.jsonl"), text).unwrap();
}

/// TS `run()` fixture 相当の base record(map として返し、test が field
/// を上書き/削除する)。
fn run_record() -> Map<String, Value> {
    json!({
        "schemaVersion": 1,
        "ts": "2026-09-19T00:00:00.000Z",
        "kind": "run",
        "worker": "bulk-reader",
        "root": "/ws",
        "ok": true,
        "durationMs": 10,
        "model": {"provider": "ollama", "id": "qwen3.5:9b", "tier": "low"},
        "usage": {"input": 100, "output": 20, "cacheRead": 0, "cacheWrite": 0,
                  "totalTokens": 120, "costUsd": 0.01},
        "savings": {"sourceTokens": 800, "returnedTokens": 50,
                    "avoidedContextTokens": 750, "directlyComparable": true},
    })
    .as_object()
    .unwrap()
    .clone()
}

fn count_record() -> Map<String, Value> {
    json!({
        "schemaVersion": 1,
        "ts": "2026-09-19T00:00:00.000Z",
        "kind": "count",
        "worker": null,
        "root": "/ws",
        "ok": true,
        "durationMs": 3,
        "result": {"path": "a.txt", "tokens": 3, "bytes": 9},
    })
    .as_object()
    .unwrap()
    .clone()
}

fn report(dir: &Path) -> Map<String, Value> {
    build_usage_report(dir, &UsageFilters::default(), None, None, None, &now).unwrap()
}

fn f64_at(v: &Value, path: &[&str]) -> f64 {
    let mut cur = v;
    for key in path {
        cur = &cur[*key];
    }
    cur.as_f64().expect("expected number")
}

// #184 contract fixtures。writer/History schema の test ではなく read-side 入力。
fn route_record() -> Map<String, Value> {
    json!({
        "schemaVersion": 1, "kind": "route", "ts": "2026-09-19T00:00:00.000Z",
        "route": "route-a", "logRef": "log-a", "ok": true,
        "originalTokens": 100, "returnedTokens": 25, "postprocessFailures": 0,
        "durationMs": {"exec": 10, "analyze": 2, "total": 12}, "worker": {}
    })
    .as_object()
    .unwrap()
    .clone()
}

fn refetch_record() -> Map<String, Value> {
    json!({
        "schemaVersion": 1, "kind": "route-refetch",
        "ts": "2026-09-19T00:00:00.000Z", "logRef": "log-a", "bytesRead": 9
    })
    .as_object()
    .unwrap()
    .clone()
}

fn run_error_record(code: &str) -> Map<String, Value> {
    let mut r = run_record();
    r.insert("ok".into(), json!(false));
    r.insert("error".into(), json!({"code": code}));
    r
}

fn grouped(dir: &Path, filters: &UsageFilters, group: UsageGroupBy) -> Value {
    Value::Object(build_usage_report(dir, filters, Some(group), None, None, &now).unwrap())
}

fn route_report(records: &[Value], filters: &UsageFilters) -> Value {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), records);
    grouped(dir.path(), filters, UsageGroupBy::Route)
}

fn error_report(records: &[Value]) -> Value {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), records);
    grouped(dir.path(), &UsageFilters::default(), UsageGroupBy::Error)
}

fn keys(v: &Value) -> Vec<&str> {
    v.as_object()
        .expect("contract object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn route_filters() -> UsageFilters {
    UsageFilters {
        route: Some("route-a".into()),
        ..UsageFilters::default()
    }
}

#[test]
fn route_bucket_applies_since() {
    let mut old_route = route_record();
    old_route.insert("ts".into(), json!("2020-01-01"));
    let mut old_refetch = refetch_record();
    old_refetch.insert("ts".into(), json!("2020-01-01"));
    let r = route_report(
        &[
            route_record().into(),
            old_route.into(),
            refetch_record().into(),
            old_refetch.into(),
        ],
        &UsageFilters {
            since_ms: Some(86_400_000.0),
            ..UsageFilters::default()
        },
    );
    assert_eq!(r["route"]["executions"], json!(1.0));
    assert_eq!(r["route"]["refetches"], json!(1.0));
    assert_eq!(r["byRoute"][0]["executions"], json!(1.0));
}

#[test]
fn route_window_invalid_timestamps() {
    let mut records = vec![
        Value::Object(route_record()),
        Value::Object(refetch_record()),
    ];
    for ts in [
        None,
        Some(json!("bad-date")),
        Some(json!(42)),
        Some(Value::Null),
    ] {
        for mut record in [route_record(), refetch_record()] {
            record.remove("ts");
            if let Some(ts) = &ts {
                record.insert("ts".into(), ts.clone());
            }
            records.push(record.into());
        }
    }
    let r = route_report(
        &records,
        &UsageFilters {
            since_ms: Some(86_400_000.0),
            ..UsageFilters::default()
        },
    );
    assert_eq!(r["route"]["executions"], json!(1.0));
    assert_eq!(r["route"]["refetches"], json!(1.0));
    let unwindowed = route_report(&records, &UsageFilters::default());
    assert_eq!(unwindowed["route"]["executions"], json!(5.0));
    assert_eq!(unwindowed["route"]["refetches"], json!(5.0));
}

#[test]
fn group_by_route_emits_per_route_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = route_record();
    b.insert("route".into(), json!("route-b"));
    b.insert("ok".into(), json!(false));
    b.insert("logRef".into(), json!("log-b"));
    b.insert("postprocessFailures".into(), json!(2));
    let mut a2 = route_record();
    a2.insert("originalTokens".into(), json!(300));
    a2.insert("returnedTokens".into(), json!(0));
    write_log(
        dir.path(),
        &[
            b.into(),
            route_record().into(),
            a2.into(),
            refetch_record().into(),
            run_record().into(),
        ],
    );
    let r = Value::Object(
        build_usage_report(
            dir.path(),
            &UsageFilters::default(),
            Some(UsageGroupBy::Route),
            Some("openai/gpt-5"),
            Some(catalog_model(2.0, 8.0)),
            &now,
        )
        .unwrap(),
    );
    assert_eq!(
        r["byRoute"],
        json!([
            {"route":"route-a","executions":2.0,"succeeded":2.0,"failed":0.0,
         "originalTokens":400.0,"returnedTokens":25.0,"reductionRatio":0.9375,
             "refetches":{"count":1.0,"bytes":9.0},"durationsMs":{"exec":20.0,"analyze":4.0,"total":24.0},
             "postprocessFailures":0.0,"unmeasurable":{"tokens":0.0,"durations":0.0}},
            {"route":"route-b","executions":1.0,"succeeded":0.0,"failed":1.0,
             "originalTokens":100.0,"returnedTokens":25.0,"reductionRatio":0.75,
             "refetches":{"count":0.0,"bytes":0.0},"durationsMs":{"exec":10.0,"analyze":2.0,"total":12.0},
             "postprocessFailures":2.0,"unmeasurable":{"tokens":0.0,"durations":0.0}}
        ])
    );
    assert_eq!(
        keys(&r["byRoute"][0]),
        [
            "route",
            "executions",
            "succeeded",
            "failed",
            "originalTokens",
            "returnedTokens",
            "reductionRatio",
            "refetches",
            "durationsMs",
            "postprocessFailures",
            "unmeasurable"
        ]
    );
    assert_eq!(
        keys(&r["route"]),
        [
            "executions",
            "failures",
            "refetches",
            "refetchBytes",
            "originalTokens",
            "returnedTokens",
            "postprocessFailures",
            "orphanRefetches",
            "unkeyedRecords"
        ]
    );
    assert!(!keys(&r).contains(&"byError"));
    let k = keys(&r);
    assert_eq!(
        k.iter().position(|&k| k == "byRoute"),
        k.iter().position(|&k| k == "route").map(|i| i + 1)
    );
    assert_eq!(
        k.iter().position(|&k| k == "estimated"),
        k.iter().position(|&k| k == "byRoute").map(|i| i + 1)
    );
}

#[test]
fn route_filter_scopes_route_sections_only() {
    let mut b = route_record();
    b.insert("route".into(), json!("route-b"));
    let records = [
        route_record().into(),
        b.into(),
        run_record().into(),
        run_error_record("X").into(),
    ];
    let all = route_report(&records, &UsageFilters::default());
    let scoped = route_report(&records, &route_filters());
    assert_eq!(scoped["route"]["executions"], json!(1.0));
    assert_eq!(scoped["byRoute"].as_array().unwrap().len(), 1);
    for key in ["records", "ok", "errors", "usage", "measured"] {
        assert_eq!(scoped[key], all[key], "{key} must not be route-scoped");
    }
}

#[test]
fn route_group_ignores_worker_filter() {
    let records = [
        route_record().into(),
        refetch_record().into(),
        run_record().into(),
    ];
    for filters in [
        UsageFilters {
            worker: Some("absent".into()),
            ..UsageFilters::default()
        },
        UsageFilters {
            provider: Some("absent".into()),
            ..UsageFilters::default()
        },
        UsageFilters {
            model: Some("absent/model".into()),
            ..UsageFilters::default()
        },
    ] {
        let r = route_report(&records, &filters);
        assert_eq!(r["records"], json!(0.0));
        assert_eq!(r["byRoute"][0]["executions"], json!(1.0));
        assert_eq!(r["byRoute"][0]["refetches"]["count"], json!(1.0));
    }
}

#[test]
fn route_filter_without_group_by() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = route_record();
    b.insert("route".into(), json!("route-b"));
    write_log(
        dir.path(),
        &[route_record().into(), b.into(), refetch_record().into()],
    );
    for (name, expected) in [("route-a", 1.0), ("missing", 0.0), ("", 0.0)] {
        let r = Value::Object(
            build_usage_report(
                dir.path(),
                &UsageFilters {
                    route: Some(name.into()),
                    ..UsageFilters::default()
                },
                None,
                None,
                None,
                &now,
            )
            .unwrap(),
        );
        assert_eq!(r["route"]["executions"], json!(expected));
        assert_eq!(r["route"]["refetches"], json!(expected));
        assert!(r.get("byRoute").is_none());
        assert!(r.get("byError").is_none());
        assert!(r["route"].get("orphanRefetches").is_none());
        assert!(r["route"].get("unkeyedRecords").is_none());
    }
}

#[test]
fn unkeyed_route_records_counted() {
    let mut absent = route_record();
    absent.remove("route");
    absent.insert("logRef".into(), json!("unkeyed"));
    let mut invalid = route_record();
    invalid.insert("route".into(), json!(5));
    let mut refetch = refetch_record();
    refetch.insert("logRef".into(), json!("unkeyed"));
    let records = [
        route_record().into(),
        absent.into(),
        invalid.into(),
        refetch.into(),
    ];
    for filters in [UsageFilters::default(), route_filters()] {
        let r = route_report(&records, &filters);
        assert_eq!(r["route"]["unkeyedRecords"], json!(2.0));
        assert_eq!(
            r["route"]["orphanRefetches"],
            json!({"count":1.0,"bytes":9.0})
        );
        assert_eq!(
            r["route"]["executions"],
            json!(if filters.route.is_some() { 1.0 } else { 3.0 })
        );
        assert_eq!(r["byRoute"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn orphan_and_joined_refetches() {
    let mut records = vec![Value::Object(route_record())];
    // 同一 logRef の所有者は file 順 first-wins。
    let mut second = route_record();
    second.insert("route".into(), json!("route-b"));
    records.push(second.into());
    for log_ref in [
        Some(json!("log-a")),
        Some(json!("absent")),
        Some(json!("")),
        None,
        Some(json!(7)),
    ] {
        let mut f = refetch_record();
        f.remove("logRef");
        if let Some(log_ref) = log_ref {
            f.insert("logRef".into(), log_ref);
        }
        records.push(f.into());
    }
    let mut empty_owner = route_record();
    empty_owner.insert("logRef".into(), json!(""));
    records.push(empty_owner.into());
    for filters in [
        UsageFilters::default(),
        route_filters(),
        UsageFilters {
            route: Some("route-b".into()),
            ..UsageFilters::default()
        },
    ] {
        let r = route_report(&records, &filters);
        assert_eq!(
            r["route"]["orphanRefetches"],
            json!({"count":4.0,"bytes":36.0})
        );
        if filters.route.as_deref() == Some("route-b") {
            assert_eq!(r["route"]["refetches"], json!(0.0));
            assert_eq!(
                r["byRoute"][0]["refetches"],
                json!({"count":0.0,"bytes":0.0})
            );
        } else {
            assert_eq!(
                r["byRoute"][0]["refetches"],
                json!({"count":1.0,"bytes":9.0})
            );
            assert_eq!(
                r["route"]["refetches"],
                json!(if filters.route.is_some() { 1.0 } else { 5.0 })
            );
        }
    }
}

#[test]
fn unmeasurable_metric_families() {
    let mut tokens = route_record();
    tokens.remove("originalTokens");
    tokens.insert("returnedTokens".into(), json!("bad"));
    let mut durations = route_record();
    durations.insert("durationMs".into(), json!({"exec":3,"analyze":null}));
    let r = route_report(
        &[route_record().into(), tokens.into(), durations.into()],
        &UsageFilters::default(),
    );
    let row = &r["byRoute"][0];
    assert_eq!(row["unmeasurable"], json!({"tokens":1.0,"durations":1.0}));
    assert_eq!(row["originalTokens"], json!(200.0));
    assert_eq!(row["returnedTokens"], json!(50.0));
    assert_eq!(
        row["durationsMs"],
        json!({"exec":23.0,"analyze":4.0,"total":24.0})
    );
}

#[test]
fn no_double_counting() {
    let mut route = route_record();
    route.insert(
        "worker".into(),
        json!({"usage":{"input":99999,"totalTokens":99999}}),
    );
    route.insert("refetches".into(), json!({"count":999,"bytes":999}));
    let r = route_report(
        &[run_record().into(), route.into(), refetch_record().into()],
        &UsageFilters::default(),
    );
    assert_eq!(r["records"], json!(1.0));
    assert_eq!(r["usage"]["totalTokens"], json!(120.0));
    assert_eq!(r["measured"]["runs"], json!(1.0));
    assert_eq!(r["measured"]["comparable"]["runs"], json!(1.0));
    assert_eq!(r["route"]["refetches"], json!(1.0));
    assert_eq!(
        r["byRoute"][0]["refetches"],
        json!({"count":1.0,"bytes":9.0})
    );
}

#[test]
fn refetch_only_log() {
    let r = route_report(&[refetch_record().into()], &UsageFilters::default());
    assert_eq!(r["records"], json!(0.0));
    assert_eq!(r["route"]["executions"], json!(0.0));
    assert_eq!(r["route"]["refetches"], json!(1.0));
    assert_eq!(
        r["route"]["orphanRefetches"],
        json!({"count":1.0,"bytes":9.0})
    );
    assert_eq!(r["byRoute"], json!([]));
}

#[test]
fn fake_records_aggregate() {
    let dir = tempfile::tempdir().unwrap();
    let mut fake = route_record();
    fake.insert("schemaVersion".into(), json!(999));
    fake.insert("route".into(), json!("invented"));
    write_log(
        dir.path(),
        &[
            fake.into(),
            refetch_record().into(),
            json!(3),
            json!({"kind":"future"}),
        ],
    );
    use std::io::Write;
    writeln!(
        std::fs::OpenOptions::new()
            .append(true)
            .open(dir.path().join("usage.jsonl"))
            .unwrap(),
        "{{broken"
    )
    .unwrap();
    assert_eq!(read_usage_records(dir.path()).len(), 2);
    let r = grouped(dir.path(), &UsageFilters::default(), UsageGroupBy::Route);
    assert_eq!(r["byRoute"][0]["route"], "invented");
    assert_eq!(r["byRoute"][0]["refetches"]["count"], json!(1.0));
}

#[test]
fn legacy_records_use_defaults() {
    let r = route_report(
        &[
            json!({"kind":"route","route":"legacy","ok":true}),
            json!({"kind":"route-refetch"}),
        ],
        &UsageFilters::default(),
    );
    let row = &r["byRoute"][0];
    assert_eq!(row["originalTokens"], json!(0.0));
    assert_eq!(row["returnedTokens"], json!(0.0));
    assert_eq!(row["reductionRatio"], json!(0.0));
    assert_eq!(row["postprocessFailures"], json!(0.0));
    assert_eq!(
        row["durationsMs"],
        json!({"exec":0.0,"analyze":0.0,"total":0.0})
    );
    assert_eq!(row["unmeasurable"], json!({"tokens":1.0,"durations":1.0}));
    assert_eq!(
        r["route"]["orphanRefetches"],
        json!({"count":1.0,"bytes":0.0})
    );
}

#[test]
fn group_by_error_emits_code_rows() {
    let mut missing = run_error_record("discard");
    missing.remove("error");
    missing.remove("worker");
    missing.remove("model");
    let mut a = run_error_record("A");
    a.insert("worker".into(), json!("z-worker"));
    a.insert("error".into(), json!({"code":"A","reason":"timeout"}));
    let r = error_report(&[
        run_record().into(),
        run_error_record("B").into(),
        a.into(),
        missing.into(),
        route_record().into(),
        refetch_record().into(),
    ]);
    let rows = r["byError"].as_array().expect("byError emitted");
    assert_eq!(
        rows.iter()
            .map(|r| r["code"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["A", "B", "unknown"]
    );
    assert_eq!(
        rows[0],
        json!({"code":"A","count":1.0,"shareOfRuns":0.25,"shareOfFailures":1.0/3.0,
        "workers":{"z-worker":1.0},"models":{"ollama/qwen3.5:9b":1.0},"durationMs":10.0,
        "reasons":{"timeout":1.0},"execution":{"timeoutMs":{},"maxTurns":{},"maxToolCalls":{}}})
    );
    assert_eq!(rows[2]["workers"], json!({}));
    assert_eq!(rows[2]["models"], json!({}));
    assert_eq!(
        keys(&rows[0]),
        [
            "code",
            "count",
            "shareOfRuns",
            "shareOfFailures",
            "workers",
            "models",
            "durationMs",
            "reasons",
            "execution"
        ]
    );
    assert!(r.get("byRoute").is_none());
    let k = keys(&r);
    assert_eq!(
        k.iter().position(|&k| k == "byError"),
        k.iter().position(|&k| k == "route").map(|i| i + 1)
    );
    let mut records = vec![Value::Object(run_error_record("B")); 2];
    records.push(run_error_record("A").into());
    let r = error_report(&records);
    assert_eq!(r["byError"][0]["code"], "B");
    assert_eq!(r["byError"][0]["count"], json!(2.0));

    let mut z = run_error_record("X");
    z.insert("worker".into(), json!("z"));
    z.insert("model".into(), json!({"provider":"z","id":"m"}));
    z.insert("error".into(), json!({"code":"X","reason":"z"}));
    let mut a = run_error_record("X");
    a.insert("worker".into(), json!("a"));
    a.insert("model".into(), json!({"provider":"a","id":"m"}));
    a.insert("error".into(), json!({"code":"X","reason":"a"}));
    let mut invalid = run_error_record("X");
    invalid.insert("worker".into(), json!(5));
    invalid.insert("model".into(), json!({"provider":"bad","id":null}));
    invalid.insert("error".into(), json!({"code":"X","reason":false}));
    let dir = tempfile::tempdir().unwrap();
    write_log(
        dir.path(),
        &[z.into(), a.into(), invalid.into(), run_record().into()],
    );
    let r = grouped(dir.path(), &UsageFilters::default(), UsageGroupBy::Error);
    assert_eq!(r["byError"][0]["count"], json!(3.0));
    for (key, expected) in [
        ("workers", vec!["a", "z"]),
        ("models", vec!["a/m", "z/m"]),
        ("reasons", vec!["a", "z"]),
    ] {
        assert_eq!(keys(&r["byError"][0][key]), expected);
    }
    // share は filter 後の recorded run/error 集合で計算する。
    let r = grouped(
        dir.path(),
        &UsageFilters {
            worker: Some("z".into()),
            ..UsageFilters::default()
        },
        UsageGroupBy::Error,
    );
    assert_eq!(r["records"], json!(1.0));
    assert_eq!(r["byError"][0]["count"], json!(1.0));
    assert_eq!(r["byError"][0]["shareOfRuns"], json!(1.0));
    assert_eq!(r["byError"][0]["shareOfFailures"], json!(1.0));
}

#[test]
fn empty_error_code_is_preserved() {
    let mut records = vec![Value::Object(run_error_record(""))];
    for error in [
        None,
        Some(json!({})),
        Some(json!({"code":null})),
        Some(json!({"code":42})),
    ] {
        let mut r = run_error_record("X");
        r.remove("error");
        if let Some(error) = error {
            r.insert("error".into(), error);
        }
        records.push(r.into());
    }
    let r = error_report(&records);
    assert_eq!(r["byError"][0]["code"], "unknown");
    assert_eq!(r["byError"][0]["count"], json!(4.0));
    assert_eq!(r["byError"][1]["code"], "");
    assert_eq!(r["byError"][1]["count"], json!(1.0));
}

#[test]
fn execution_number_keys_use_js_canonical_format() {
    let mut records = Vec::new();
    for value in [
        json!(3),
        json!(3.0),
        Value::Null,
        json!(true),
        json!(false),
        json!("string"),
        json!([]),
        json!({}),
    ] {
        let mut r = run_error_record("X");
        r.insert(
            "execution".into(),
            json!({"timeoutMs":value,"maxTurns":value,"maxToolCalls":value}),
        );
        records.push(r.into());
    }
    let r = error_report(&records);
    for key in ["timeoutMs", "maxTurns", "maxToolCalls"] {
        let map = &r["byError"][0]["execution"][key];
        assert_eq!(
            map,
            &json!({"3":2.0,"false":1.0,"null":1.0,"string":1.0,"true":1.0})
        );
        assert_eq!(keys(map), ["3", "false", "null", "string", "true"]);
    }
}

#[test]
fn nonfinite_aggregates_serialize_as_null() {
    let mut route = route_record();
    route.insert("originalTokens".into(), json!(1e308));
    route.insert("returnedTokens".into(), json!(1e308));
    route.insert(
        "durationMs".into(),
        json!({"exec":1e308,"analyze":1e308,"total":1e308}),
    );
    let r = route_report(
        &[route.clone().into(), route.into()],
        &UsageFilters::default(),
    );
    let row = &r["byRoute"][0];
    assert!(row.is_object(), "grouped row must exist");
    for key in ["originalTokens", "returnedTokens", "reductionRatio"] {
        assert_eq!(row[key], Value::Null);
    }
    assert_eq!(
        row["durationsMs"],
        json!({"exec":null,"analyze":null,"total":null})
    );
    let mut error = run_error_record("X");
    error.insert("durationMs".into(), json!(1e308));
    let e = error_report(&[error.clone().into(), error.into()]);
    assert!(e["byError"][0].is_object(), "error row must exist");
    assert_eq!(e["byError"][0]["durationMs"], Value::Null);
    for v in [&r, &e] {
        let text = cheaper_cli::text_format::format_usage_report(v);
        let json = serde_json::to_string(v).unwrap();
        for forbidden in ["NaN", "Infinity"] {
            assert!(!text.contains(forbidden));
            assert!(!json.contains(forbidden));
        }
    }
}

#[test]
fn failure_only_log_emits_by_error_rows() {
    let r = error_report(&[run_error_record("X").into(), run_error_record("X").into()]);
    assert_eq!(r["byError"][0]["count"], json!(2.0));
    assert_eq!(r["byError"][0]["shareOfRuns"], json!(1.0));
    assert_eq!(r["byError"][0]["shareOfFailures"], json!(1.0));
}

#[test]
fn empty_log_emits_empty_grouped_sections() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        grouped(dir.path(), &UsageFilters::default(), UsageGroupBy::Route)["byRoute"],
        json!([])
    );
    assert_eq!(
        grouped(dir.path(), &UsageFilters::default(), UsageGroupBy::Error)["byError"],
        json!([])
    );
}

#[test]
fn fake_run_error_records_aggregate() {
    let r = error_report(&[
        json!({"kind":"run","ok":0,"error":{"code":"invented"},"durationMs":-3}),
        json!({"kind":"run","ok":"truthy","error":{"code":"not-a-failure"}}),
    ]);
    assert_eq!(r["byError"].as_array().unwrap().len(), 1);
    assert_eq!(r["byError"][0]["code"], "invented");
    assert_eq!(r["byError"][0]["shareOfRuns"], json!(0.5));
    assert_eq!(r["byError"][0]["durationMs"], json!(-3.0));
}

#[test]
fn argv_and_error_detail_not_leaked() {
    let mut route = route_record();
    route.insert(
        "executed".into(),
        json!({"argv":["argv-secret"],"command":"command-secret"}),
    );
    let mut error = run_error_record("X");
    error.insert("root".into(), json!("root-secret"));
    error.insert("error".into(), json!({"code":"X","message":"message-secret","schemaErrors":["schema-secret"],
        "checkpoint":{"path":"checkpoint-secret"},"errorMessage":"error-secret","prompt":"prompt-secret"}));
    for group in [UsageGroupBy::Route, UsageGroupBy::Error] {
        let dir = tempfile::tempdir().unwrap();
        write_log(dir.path(), &[route.clone().into(), error.clone().into()]);
        let r = grouped(dir.path(), &UsageFilters::default(), group);
        assert!(r[if group == UsageGroupBy::Route {
            "byRoute"
        } else {
            "byError"
        }]
        .is_array());
        for output in [
            serde_json::to_string(&r).unwrap(),
            cheaper_cli::text_format::format_usage_report(&r),
        ] {
            for secret in [
                "argv-secret",
                "command-secret",
                "root-secret",
                "message-secret",
                "schema-secret",
                "checkpoint-secret",
                "error-secret",
                "prompt-secret",
            ] {
                assert!(!output.contains(secret), "{output}");
            }
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_group_by_value_rejected_at_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    for value in ["bogus", "route,error", ""] {
        let (code, out, _) = run_usage(dir.path(), &["usage", "--group-by", value]).await;
        assert_eq!(code, 2);
        let r: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(r["error"]["code"], "INVALID_REQUEST");
        assert_eq!(
            r["error"]["message"],
            format!("invalid --group-by: {value} (expected route|error)")
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn route_filter_leaves_by_error_unscoped() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = route_record();
    b.insert("route".into(), json!("route-b"));
    write_log(
        dir.path(),
        &[
            route_record().into(),
            b.into(),
            run_error_record("X").into(),
            run_record().into(),
        ],
    );
    let (code, out, _) = run_usage(
        dir.path(),
        &["usage", "--route", "route-a", "--group-by", "error"],
    )
    .await;
    assert_eq!(code, 0, "{out}");
    let r: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(r["result"]["route"]["executions"], json!(1.0));
    assert_eq!(r["result"]["byError"][0]["shareOfRuns"], json!(0.5));
    assert_eq!(r["result"]["byError"][0]["count"], json!(1.0));
    assert!(r["result"].get("byRoute").is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn route_flag_scopes_route_at_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let mut b = route_record();
    b.insert("route".into(), json!("route-b"));
    write_log(dir.path(), &[route_record().into(), b.into()]);
    let (code, out, _) = run_usage(dir.path(), &["usage", "--route", "route-a"]).await;
    assert_eq!(code, 0, "{out}");
    let r: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(r["result"]["route"]["executions"], json!(1.0));
}

#[tokio::test(flavor = "current_thread")]
async fn group_by_last_wins_emits_by_error_only() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[run_error_record("X").into()]);
    let (code, out, _) = run_usage(
        dir.path(),
        &["usage", "--group-by", "route", "--group-by", "error"],
    )
    .await;
    assert_eq!(code, 0, "{out}");
    let r: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(r["result"]["byError"][0]["code"], "X");
    assert!(r["result"].get("byRoute").is_none());
}

#[test]
fn format_text_parity() {
    use cheaper_cli::text_format::format_usage_report;
    let r = route_report(
        &[route_record().into(), refetch_record().into()],
        &UsageFilters::default(),
    );
    let text = format_usage_report(&r);
    for line in [
        "route:         executions=1 failures=0 refetches=1 refetchBytes=9",
        "route tokens:  original=100 returned=25 postprocessFailures=0",
        "route scope:   orphanRefetches=0 orphanRefetchBytes=0 unkeyedRecords=0",
        "byRoute:",
        "  route-a: executions=1 ok=1 failed=0 tokens=100->25 reduction=0.75 postprocessFailures=0",
        "    refetches=1 bytes=9 exec/analyze/total=10/2/12ms unmeasurable=0/0",
    ] {
        assert!(
            text.lines().any(|s| s == line),
            "missing {line:?} in {text}"
        );
    }
    let e = error_report(&[run_error_record("X").into()]);
    let text = format_usage_report(&e);
    for line in [
        "byError:",
        "  X: count=1 shareOfRuns=1 shareOfFailures=1 durationMs=10",
        "    workers: bulk-reader=1",
        "    models: ollama/qwen3.5:9b=1",
    ] {
        assert!(text.lines().any(|s| s == line), "{text}");
    }
    assert!(!text.contains("    reasons:"));
    assert!(!text.contains("    execution:"));
    let mut detail = run_error_record("X");
    detail.insert("error".into(), json!({"code":"X","reason":"timeout"}));
    detail.insert(
        "execution".into(),
        json!({"timeoutMs":3,"maxTurns":null,"maxToolCalls":8}),
    );
    let text = format_usage_report(&error_report(&[detail.into()]));
    assert!(
        text.lines().any(|s| s == "    reasons: timeout=1"),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|s| s == "    execution: timeoutMs={3:1} maxTurns={null:1} maxToolCalls={8:1}"),
        "{text}"
    );
    let dir = tempfile::tempdir().unwrap();
    let plain = Value::Object(report(dir.path()));
    let text = format_usage_report(&plain);
    assert!(text.contains("route:         executions=0 failures=0 refetches=0 refetchBytes=0"));
    assert!(!text.contains("byRoute:"));
    assert!(!text.contains("byError:"));
    assert!(!text.contains("route scope:"));
    for (group, header) in [
        (UsageGroupBy::Route, "byRoute:"),
        (UsageGroupBy::Error, "byError:"),
    ] {
        assert!(
            format_usage_report(&grouped(dir.path(), &UsageFilters::default(), group))
                .contains(header)
        );
    }
}

// #192 consumer tolerance: ID の形式/生成/伝搬をここでは規定しない。
#[test]
fn usage_optional_ids_preserve_report() {
    let dir = tempfile::tempdir().unwrap();
    let records = vec![
        Value::Object(run_record()),
        run_error_record("X").into(),
        route_record().into(),
        refetch_record().into(),
    ];
    let mut with_ids = records.clone();
    for (i, r) in with_ids.iter_mut().enumerate() {
        r["executionId"] = json!(format!("opaque-{i}"));
        r["parentExecutionId"] = json!("opaque-parent");
        r["traceId"] = json!("opaque-trace");
    }
    for filters in [
        UsageFilters::default(),
        UsageFilters {
            since_ms: Some(86_400_000.0),
            worker: Some("bulk-reader".into()),
            provider: Some("ollama".into()),
            model: Some("ollama/qwen3.5:9b".into()),
            route: Some("route-a".into()),
        },
        UsageFilters {
            worker: Some("absent".into()),
            ..UsageFilters::default()
        },
    ] {
        for group in [None, Some(UsageGroupBy::Route), Some(UsageGroupBy::Error)] {
            write_log(dir.path(), &records);
            let before = Value::Object(
                build_usage_report(dir.path(), &filters, group, None, None, &now).unwrap(),
            );
            write_log(dir.path(), &with_ids);
            let after = Value::Object(
                build_usage_report(dir.path(), &filters, group, None, None, &now).unwrap(),
            );
            assert_eq!(
                serde_json::to_string(&before).unwrap(),
                serde_json::to_string(&after).unwrap()
            );
            assert_eq!(
                cheaper_cli::text_format::format_usage_report(&before),
                cheaper_cli::text_format::format_usage_report(&after)
            );
        }
    }
}

#[test]
fn usage_same_trace_distinct_runs_counted() {
    let mut ok = Value::Object(run_record());
    let mut error = Value::Object(run_error_record("X"));
    ok["executionId"] = json!("first");
    error["executionId"] = json!("second");
    ok["traceId"] = json!("shared");
    error["traceId"] = json!("shared");
    let r = error_report(&[ok, error]);
    assert_eq!(r["records"], json!(2.0));
    assert_eq!(r["ok"], json!(1.0));
    assert_eq!(r["errors"], json!(1.0));
    assert_eq!(r["byError"][0]["count"], json!(1.0));
    assert_eq!(r["byError"][0]["shareOfRuns"], json!(0.5));
    assert_eq!(r["byError"][0]["shareOfFailures"], json!(1.0));
}

#[test]
fn usage_refetch_distinct_execution_id_joins_log_ref() {
    let mut owner = Value::Object(route_record());
    let mut refetch = Value::Object(refetch_record());
    owner["executionId"] = json!("route-execution");
    refetch["executionId"] = json!("refetch-execution");
    // Only logRef may establish this join; execution/parent IDs are opaque
    // and deliberately do not point at the route owner.
    refetch["parentExecutionId"] = json!("unrelated-parent");
    let r = route_report(
        &[owner, refetch, run_record().into()],
        &UsageFilters {
            since_ms: Some(86_400_000.0),
            ..UsageFilters::default()
        },
    );
    assert_eq!(r["records"], json!(1.0));
    assert_eq!(r["route"]["executions"], json!(1.0));
    assert_eq!(r["route"]["refetches"], json!(1.0));
    assert_eq!(
        r["byRoute"][0]["refetches"],
        json!({"count":1.0,"bytes":9.0})
    );
    assert_eq!(
        r["route"]["orphanRefetches"],
        json!({"count":0.0,"bytes":0.0})
    );
}

#[test]
fn usage_window_orphan_not_backfilled_from_history() {
    let dir = tempfile::tempdir().unwrap();
    let mut old = route_record();
    old.insert("ts".into(), json!("2020-01-01"));
    write_log(dir.path(), &[old.into(), refetch_record().into()]);
    let filters = UsageFilters {
        since_ms: Some(86_400_000.0),
        ..UsageFilters::default()
    };
    let before = grouped(dir.path(), &filters, UsageGroupBy::Route);
    // 将来の History path/schema を仮定しない inert な外部情報。
    std::fs::write(
        dir.path().join("unrelated-sentinel.txt"),
        "owner log-a exists outside usage window",
    )
    .unwrap();
    let after = grouped(dir.path(), &filters, UsageGroupBy::Route);
    assert_eq!(before, after);
    assert_eq!(after["route"]["executions"], json!(0.0));
    assert_eq!(
        after["route"]["orphanRefetches"],
        json!({"count":1.0,"bytes":9.0})
    );
    assert_eq!(after["byRoute"], json!([]));
}

#[test]
fn usage_report_history_independent() {
    let dir = tempfile::tempdir().unwrap();
    write_log(
        dir.path(),
        &[run_record().into(), run_error_record("X").into()],
    );
    for group in [None, Some(UsageGroupBy::Route), Some(UsageGroupBy::Error)] {
        let before = build_usage_report(
            dir.path(),
            &UsageFilters::default(),
            group,
            None,
            None,
            &now,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("unrelated-sentinel.txt"),
            "unrecorded failure / skipped execution",
        )
        .unwrap();
        let after = build_usage_report(
            dir.path(),
            &UsageFilters::default(),
            group,
            None,
            None,
            &now,
        )
        .unwrap();
        assert_eq!(before, after);
        assert_eq!(after["records"], json!(2.0));
        assert_eq!(after["errors"], json!(1.0));
        std::fs::remove_file(dir.path().join("unrelated-sentinel.txt")).unwrap();
    }
}

// ---------------------------------------------------------------------------
// splitModelRef (usage 側では --model/--compare-model の検証に利用)
// ---------------------------------------------------------------------------

#[test]
fn split_model_ref_semantics() {
    use cheaper_core::model::model_ref::parse_model_ref;
    let r = parse_model_ref("openai/gpt-5").unwrap();
    assert_eq!((r.provider.as_str(), r.id.as_str()), ("openai", "gpt-5"));
    let r = parse_model_ref("ollama/qwen3.5:9b").unwrap();
    assert_eq!(
        (r.provider.as_str(), r.id.as_str()),
        ("ollama", "qwen3.5:9b")
    );
    assert!(parse_model_ref("noslash").is_none());
    assert!(parse_model_ref("/id").is_none());
    assert!(parse_model_ref("p/").is_none());
}

// ---------------------------------------------------------------------------
// aggregate / build_usage_report
// ---------------------------------------------------------------------------

/// Node `readFile(path, "utf8")` parity — 不正 UTF-8 byte を含む行が
/// あっても lossy decode で残りの正常行を読む(`read_to_string` では
/// file 全体が捨てられた)。不正 byte は U+FFFD に置き換わり、decode
/// 後に parse 可能な行は record として残る。
#[test]
fn read_usage_records_decodes_lossy_utf8() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(serde_json::to_string(&run_record()).unwrap().as_bytes());
    bytes.push(b'\n');
    // string 内の不正 byte(0xFF)→ U+FFFD で decode 後も parse 可能。
    bytes.extend_from_slice(b"{\"kind\":\"run\",\"ts\":\"\xFFoops\"}\n");
    // JSON 自体が壊れた行は skip。
    bytes.extend_from_slice(b"{\"kind\":\"run\",broken\n");
    bytes.extend_from_slice(serde_json::to_string(&count_record()).unwrap().as_bytes());
    bytes.push(b'\n');
    std::fs::write(dir.path().join("usage.jsonl"), &bytes).unwrap();
    let records = read_usage_records(dir.path());
    assert_eq!(records.len(), 3, "records: {records:?}");
    assert_eq!(records[0]["kind"], json!("run"));
    assert_eq!(records[1]["ts"], json!("\u{FFFD}oops"));
    assert_eq!(records[2]["kind"], json!("count"));
    // report 経路では kind:"run" の 2 件だけが集計対象になる
    // (`count` record は run 集計に含まれない)。
    let r = Value::Object(report(dir.path()));
    assert_eq!(f64_at(&r, &["records"]), 2.0);
}

#[test]
fn aggregates_actual_usage_only_when_no_savings() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = run_record();
    r.remove("savings");
    write_log(dir.path(), &[Value::Object(r)]);
    let report = report(dir.path());
    assert_eq!(f64_at(&Value::Object(report.clone()), &["records"]), 1.0);
    let measured = &report["measured"];
    assert_eq!(
        f64_at(measured, &["runs"]),
        0.0,
        "measured.runs should be 0"
    );
    assert_eq!(f64_at(measured, &["sourceTokens"]), 0.0);
    assert_eq!(f64_at(measured, &["comparable", "runs"]), 0.0);
    assert!(report.get("estimated").is_none());
}

#[test]
fn counts_measured_offload_including_failures() {
    let dir = tempfile::tempdir().unwrap();
    let mut err = run_record();
    err.insert("ok".to_string(), Value::Bool(false));
    err.insert(
        "savings".to_string(),
        json!({"sourceTokens":100,"returnedTokens":0,
               "avoidedContextTokens":100,"directlyComparable":true}),
    );
    write_log(
        dir.path(),
        &[Value::Object(run_record()), Value::Object(err)],
    );
    let r = Value::Object(report(dir.path()));
    assert_eq!(f64_at(&r, &["records"]), 2.0);
    assert_eq!(f64_at(&r, &["ok"]), 1.0);
    assert_eq!(f64_at(&r, &["errors"]), 1.0);
    assert_eq!(f64_at(&r, &["measured", "runs"]), 2.0);
    assert_eq!(f64_at(&r, &["measured", "sourceTokens"]), 900.0);
    assert_eq!(f64_at(&r, &["measured", "returnedTokens"]), 50.0);
    assert_eq!(f64_at(&r, &["measured", "avoidedContextTokens"]), 850.0);
    assert_eq!(f64_at(&r, &["measured", "comparable", "runs"]), 2.0);
    assert_eq!(f64_at(&r, &["measured", "comparable", "costUsd"]), 0.02);
}

#[test]
fn non_comparable_workers_stay_in_measured_not_comparable() {
    let dir = tempfile::tempdir().unwrap();
    let mut w = run_record();
    w.insert("worker".to_string(), json!("code-writer"));
    w.insert(
        "savings".to_string(),
        json!({"sourceTokens":400,"returnedTokens":10,
               "avoidedContextTokens":390,"directlyComparable":false}),
    );
    write_log(dir.path(), &[Value::Object(run_record()), Value::Object(w)]);
    let r = Value::Object(report(dir.path()));
    assert_eq!(f64_at(&r, &["measured", "runs"]), 2.0);
    assert_eq!(f64_at(&r, &["measured", "sourceTokens"]), 1200.0);
    assert_eq!(f64_at(&r, &["measured", "comparable", "runs"]), 1.0);
    assert_eq!(
        f64_at(&r, &["measured", "comparable", "sourceTokens"]),
        800.0
    );
    assert_eq!(
        f64_at(&r, &["measured", "comparable", "returnedTokens"]),
        50.0
    );
    assert_eq!(f64_at(&r, &["measured", "comparable", "costUsd"]), 0.01);
}

#[test]
fn legacy_savings_without_flag_is_not_comparable() {
    // 旧 schema record は directlyComparable を持たない(非 comparable)。
    let dir = tempfile::tempdir().unwrap();
    let mut r = run_record();
    r.insert(
        "savings".to_string(),
        json!({"sourceTokens":800,"returnedTokens":50,"avoidedContextTokens":750}),
    );
    write_log(dir.path(), &[Value::Object(r)]);
    let r = Value::Object(report(dir.path()));
    assert_eq!(f64_at(&r, &["measured", "runs"]), 1.0);
    assert_eq!(f64_at(&r, &["measured", "comparable", "runs"]), 0.0);
}

#[test]
fn count_kind_records_are_parsed_but_excluded_from_aggregation() {
    // 新旧互換: `kind:"count"` record は読めるが run 集計には入らない。
    let dir = tempfile::tempdir().unwrap();
    write_log(
        dir.path(),
        &[Value::Object(run_record()), Value::Object(count_record())],
    );
    assert_eq!(read_usage_records(dir.path()).len(), 2);
    let r = Value::Object(report(dir.path()));
    assert_eq!(f64_at(&r, &["records"]), 1.0);
}

#[test]
fn old_records_missing_turns_toolcalls_still_aggregate() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = run_record();
    r.remove("turns");
    r.remove("toolCalls");
    write_log(dir.path(), &[Value::Object(r)]);
    let r = Value::Object(report(dir.path()));
    assert_eq!(f64_at(&r, &["turns"]), 0.0);
    assert_eq!(f64_at(&r, &["toolCalls"]), 0.0);
    assert_eq!(f64_at(&r, &["durationMs"]), 10.0);
}

#[test]
fn explicit_null_usage_fails_like_js_type_error() {
    // JS `r.usage.input` on null → TypeError → INTERNAL_ERROR envelope。
    let dir = tempfile::tempdir().unwrap();
    let mut r = run_record();
    r.insert("usage".to_string(), Value::Null);
    write_log(dir.path(), &[Value::Object(r)]);
    let err = build_usage_report(dir.path(), &UsageFilters::default(), None, None, None, &now)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
}

#[test]
fn explicit_null_savings_fails_like_js_type_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = run_record();
    r.insert("savings".to_string(), Value::Null);
    write_log(dir.path(), &[Value::Object(r)]);
    let err = build_usage_report(dir.path(), &UsageFilters::default(), None, None, None, &now)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
}

// ---------------------------------------------------------------------------
// estimated (--compare-model)
// ---------------------------------------------------------------------------

fn catalog_model(input: f64, output: f64) -> ModelInfo {
    ModelInfo {
        provider: "openai".to_string(),
        id: "gpt-5".to_string(),
        cost: Some(ModelCost {
            input,
            output,
            cache_read: None,
            cache_write: None,
        }),
        max_tokens: None,
    }
}

fn write_pricing(dir: &Path, models: Value, updated_at: &str) {
    let body = json!({"version":"test","updatedAt":updated_at,"models":models});
    std::fs::write(
        dir.join("pricing.json"),
        serde_json::to_string(&body).unwrap(),
    )
    .unwrap();
}

#[test]
fn no_estimated_without_compare_model() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let r = report(dir.path());
    assert!(r.get("estimated").is_none());
}

#[test]
fn no_estimated_when_compare_model_has_no_pricing() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("openai/unknown-xyz-99"),
        None,
        &now,
    )
    .unwrap();
    assert!(r.get("estimated").is_none());
}

#[test]
fn runtime_catalog_pricing_wins() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("openai/gpt-5"),
        Some(catalog_model(2.0, 8.0)),
        &now,
    )
    .unwrap();
    let e = &r["estimated"];
    assert_eq!(e["pricing"]["source"], "runtime-catalog");
    // parentEquiv = (800*2 + 50*8)/1e6 = 0.002 ; savings = 0.002 - 0.01
    let pe = f64_at(e, &["estimatedParentEquivalentCostUsd"]);
    assert!((pe - 0.002).abs() < 1e-9, "parentEquiv={pe}");
    let s = f64_at(e, &["estimatedSavingsUsd"]);
    assert!((s - -0.008).abs() < 1e-9, "savings={s}");
    let pct = f64_at(e, &["estimatedSavingsPercent"]);
    assert!((pct - -400.0).abs() < 1e-9, "pct={pct}");
    assert_eq!(e["pricing"]["stale"], false);
    assert!(e["pricing"].get("updatedAt").is_none());
}

#[test]
fn user_pricing_json_fallback_reports_pricing_json_source() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    write_pricing(
        dir.path(),
        json!({"test-prov/test-model":{"input":2,"output":8}}),
        "2026-09-18",
    );
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("test-prov/test-model"),
        None,
        &now,
    )
    .unwrap();
    let e = &r["estimated"];
    assert_eq!(e["pricing"]["source"], "pricing.json");
    assert_eq!(e["pricing"]["updatedAt"], "2026-09-18");
    assert_eq!(e["pricing"]["snapshotVersion"], "test");
}

#[test]
fn runtime_catalog_preferred_over_pricing_json() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    write_pricing(
        dir.path(),
        json!({"openai/gpt-5":{"input":100,"output":100}}),
        "2026-09-18",
    );
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("openai/gpt-5"),
        Some(catalog_model(2.0, 8.0)),
        &now,
    )
    .unwrap();
    let e = &r["estimated"];
    assert_eq!(e["pricing"]["source"], "runtime-catalog");
    let pe = f64_at(e, &["estimatedParentEquivalentCostUsd"]);
    assert!((pe - 0.002).abs() < 1e-9);
}

#[test]
fn stale_snapshot_pricing_emits_no_estimated() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    write_pricing(
        dir.path(),
        json!({"test-prov/test-model":{"input":2,"output":8}}),
        "2020-01-01",
    );
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("test-prov/test-model"),
        None,
        &now,
    )
    .unwrap();
    assert!(r.get("estimated").is_none());
}

#[test]
fn exactly_stale_boundary_is_not_stale() {
    // `now - updatedAt > PRICING_STALE_MS` のみ stale(境界値は fresh)。
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let boundary_ms = NOW_MS - PRICING_STALE_MS;
    // boundary_ms を ISO-8601 に戻す必要はない — date-only で
    // ずらす代わりに updatedAt を経由 stale 判定単体で検証するため、
    // `2026-08-21` (30 日前 = boundary 日) を使う。
    let _ = boundary_ms;
    write_pricing(
        dir.path(),
        json!({"test-prov/test-model":{"input":2,"output":8}}),
        "2026-08-21",
    );
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("test-prov/test-model"),
        None,
        &now,
    )
    .unwrap();
    // 2026-08-21T00:00Z → 2026-09-20T00:00Z はちょうど 30 日 = stale ではない。
    assert!(r.get("estimated").is_some());
}

#[test]
fn unparseable_updated_at_is_stale() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    write_pricing(
        dir.path(),
        json!({"test-prov/test-model":{"input":2,"output":8}}),
        "not-a-date",
    );
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("test-prov/test-model"),
        None,
        &now,
    )
    .unwrap();
    assert!(r.get("estimated").is_none());
}

#[test]
fn no_estimated_when_only_non_comparable_workers() {
    let dir = tempfile::tempdir().unwrap();
    let mut w = run_record();
    w.insert("worker".to_string(), json!("code-writer"));
    w.insert(
        "savings".to_string(),
        json!({"sourceTokens":800,"returnedTokens":50,
               "avoidedContextTokens":750,"directlyComparable":false}),
    );
    write_log(dir.path(), &[Value::Object(w)]);
    write_pricing(
        dir.path(),
        json!({"test-prov/test-model":{"input":2,"output":8}}),
        "2026-09-18",
    );
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("test-prov/test-model"),
        None,
        &now,
    )
    .unwrap();
    assert_eq!(
        f64_at(&Value::Object(r.clone()), &["measured", "runs"]),
        1.0
    );
    assert!(r.get("estimated").is_none());
}

#[test]
fn estimated_uses_comparable_subset_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut w = run_record();
    w.insert("worker".to_string(), json!("code-writer"));
    w.insert(
        "usage".to_string(),
        json!({"input":1,"output":1,"cacheRead":0,"cacheWrite":0,
               "totalTokens":2,"costUsd":5}),
    );
    w.insert(
        "savings".to_string(),
        json!({"sourceTokens":9000,"returnedTokens":10,
               "avoidedContextTokens":8990,"directlyComparable":false}),
    );
    write_log(dir.path(), &[Value::Object(run_record()), Value::Object(w)]);
    write_pricing(
        dir.path(),
        json!({"test-prov/test-model":{"input":2,"output":8}}),
        "2026-09-18",
    );
    let r = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        Some("test-prov/test-model"),
        None,
        &now,
    )
    .unwrap();
    let e = &r["estimated"];
    // comparable subset のみ: parentEquiv=(800*2+50*8)/1e6=0.002, cost=0.01。
    let pe = f64_at(e, &["estimatedParentEquivalentCostUsd"]);
    assert!((pe - 0.002).abs() < 1e-9);
    let s = f64_at(e, &["estimatedSavingsUsd"]);
    assert!((s - -0.008).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// filters
// ---------------------------------------------------------------------------

#[test]
fn applies_since_worker_filters_before_aggregating() {
    let dir = tempfile::tempdir().unwrap();
    let mut other = run_record();
    other.insert("worker".to_string(), json!("doc-writer"));
    other.remove("savings");
    let mut old = run_record();
    old.insert("ts".to_string(), json!("2020-01-01T00:00:00.000Z"));
    write_log(
        dir.path(),
        &[
            Value::Object(run_record()),
            Value::Object(other),
            Value::Object(old),
        ],
    );
    let filters = UsageFilters {
        since_ms: Some(7.0 * 24.0 * 3_600_000.0),
        worker: Some("bulk-reader".to_string()),
        ..UsageFilters::default()
    };
    let r = build_usage_report(dir.path(), &filters, None, None, None, &now).unwrap();
    assert_eq!(f64_at(&Value::Object(r), &["records"]), 1.0);
}

#[test]
fn applies_provider_and_model_filters() {
    let dir = tempfile::tempdir().unwrap();
    let mut other = run_record();
    other.insert(
        "model".to_string(),
        json!({"provider":"openai","id":"gpt-5-mini","tier":"low"}),
    );
    write_log(
        dir.path(),
        &[Value::Object(run_record()), Value::Object(other)],
    );
    let by_provider = UsageFilters {
        provider: Some("ollama".to_string()),
        ..UsageFilters::default()
    };
    let r = build_usage_report(dir.path(), &by_provider, None, None, None, &now).unwrap();
    assert_eq!(f64_at(&Value::Object(r), &["records"]), 1.0);
    let by_model = UsageFilters {
        model: Some("openai/gpt-5-mini".to_string()),
        ..UsageFilters::default()
    };
    let r = build_usage_report(dir.path(), &by_model, None, None, None, &now).unwrap();
    assert_eq!(f64_at(&Value::Object(r), &["records"]), 1.0);
}

#[test]
fn unparseable_model_filter_is_ignored() {
    // buildUsageReport 単体では invalid model ref は filter 不適用
    // (cli 層では先に INVALID_REQUEST)。
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let filters = UsageFilters {
        model: Some("noslash".to_string()),
        ..UsageFilters::default()
    };
    let r = build_usage_report(dir.path(), &filters, None, None, None, &now).unwrap();
    assert_eq!(f64_at(&Value::Object(r), &["records"]), 1.0);
}

#[test]
fn skips_malformed_jsonl_lines_and_missing_log_file() {
    let dir = tempfile::tempdir().unwrap();
    let valid = serde_json::to_string(&Value::Object(run_record())).unwrap();
    std::fs::write(
        dir.path().join("usage.jsonl"),
        format!("{{broken\n{valid}\n"),
    )
    .unwrap();
    let r = report(dir.path());
    assert_eq!(f64_at(&Value::Object(r), &["records"]), 1.0);

    let empty = tempfile::tempdir().unwrap();
    let r = report(empty.path());
    assert_eq!(f64_at(&Value::Object(r), &["records"]), 0.0);
}

#[test]
fn unknown_kind_lines_are_skipped() {
    // 前方互換: 未知 kind record は is_record で除外。
    let dir = tempfile::tempdir().unwrap();
    let mut r = run_record();
    r.insert("kind".to_string(), json!("future-kind"));
    write_log(dir.path(), &[Value::Object(r), Value::Object(run_record())]);
    assert_eq!(read_usage_records(dir.path()).len(), 1);
}

// ---------------------------------------------------------------------------
// dispatch 経路(run_cli)
// ---------------------------------------------------------------------------

/// `CHEAPER_CONFIG_DIR` を tempdir に向け、他は process env に fallthrough。
async fn run_usage(config_dir: &Path, argv: &[&str]) -> (i32, String, String) {
    let injected: HashMap<String, String> = HashMap::from_iter([(
        "CHEAPER_CONFIG_DIR".to_string(),
        config_dir.to_string_lossy().into_owned(),
    )]);
    let env_lookup = |name: &str| injected.get(name).cloned().or_else(|| ambient_env(name));
    let mut stdin = PipeStdin::empty(true);
    let out = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let err = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut io = CliIo {
        stdin: &mut stdin,
        stdout: &mut SharedVecSink(out.clone()),
        stderr: &mut SharedVecSink(err.clone()),
        env: &env_lookup,
        cwd: std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    };
    let hooks = CliHooks {
        now: Some(std::sync::Arc::new(|| NOW_MS)),
        ..CliHooks::default()
    };
    let code = run_cli(
        &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        &mut io,
        &hooks,
    )
    .await;
    let stdout = String::from_utf8(out.borrow().clone()).unwrap();
    let stderr = String::from_utf8(err.borrow().clone()).unwrap();
    (code, stdout, stderr)
}

#[tokio::test(flavor = "current_thread")]
async fn usage_command_writes_utility_envelope() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let (code, out, _err) = run_usage(dir.path(), &["usage"]).await;
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["schemaVersion"], 1);
    assert_eq!(v["ok"], true);
    assert_eq!(v["worker"], Value::Null);
    assert_eq!(f64_at(&v, &["result", "records"]), 1.0);
    assert_eq!(f64_at(&v, &["result", "usage", "input"]), 100.0);
    // hooks.now 固定で durationMs = 0。
    assert_eq!(f64_at(&v, &["meta", "durationMs"]), 0.0);
    // utility envelope の key 順: schemaVersion, ok, worker, result, meta。
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(keys, ["schemaVersion", "ok", "worker", "result", "meta"]);
}

#[tokio::test(flavor = "current_thread")]
async fn usage_command_format_text_writes_report() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let (code, out, _err) = run_usage(dir.path(), &["usage", "--format", "text"]).await;
    assert_eq!(code, 0);
    let expected = concat!(
        "ok\n",
        "runs:          1 (ok=1 errors=0)\n",
        "worker tokens: in=100 out=20 cacheRead=0 cacheWrite=0 total=120\n",
        "worker cost:   $0.010000\n",
        "turns:         0  toolCalls=0  duration=10ms\n",
        "avoided parent context: source=800 returned=50 avoided=750 (1 runs measured)\n",
    );
    assert_eq!(out, expected);
}

#[tokio::test(flavor = "current_thread")]
async fn usage_command_invalid_since_is_invalid_request() {
    let dir = tempfile::tempdir().unwrap();
    let (code, out, err) = run_usage(dir.path(), &["usage", "--since", "bogus"]).await;
    assert_eq!(code, 2);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert_eq!(v["error"]["message"], "invalid --since duration: bogus");
    assert_eq!(v["error"]["details"]["since"], "bogus");
    assert!(err.starts_with("error INVALID_REQUEST:"));
}

#[tokio::test(flavor = "current_thread")]
async fn usage_command_invalid_model_ref_is_invalid_request() {
    let dir = tempfile::tempdir().unwrap();
    let (code, out, _err) = run_usage(dir.path(), &["usage", "--model", "bad"]).await;
    assert_eq!(code, 2);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert_eq!(
        v["error"]["message"],
        "invalid --model: bad (expected <provider>/<id>)"
    );
    let (_c, out, _e) = run_usage(dir.path(), &["usage", "--compare-model", "bad"]).await;
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["error"]["details"]["compareModel"], "bad");
}

#[tokio::test(flavor = "current_thread")]
async fn usage_compare_model_uses_runtime_catalog() {
    // bundled catalog の `openai/gpt-5`(cost input 1.25 / output 10)が
    // runtime-catalog source として観測される。
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let (code, out, _err) =
        run_usage(dir.path(), &["usage", "--compare-model", "openai/gpt-5"]).await;
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    let e = &v["result"]["estimated"];
    assert_eq!(e["compareModel"], "openai/gpt-5");
    assert_eq!(e["pricing"]["source"], "runtime-catalog");
    // parentEquiv = (800*1.25 + 50*10)/1e6 = 0.0015; savings = 0.0015-0.01
    let pe = f64_at(e, &["estimatedParentEquivalentCostUsd"]);
    assert!((pe - 0.0015).abs() < 1e-9, "parentEquiv={pe}");
}

#[tokio::test(flavor = "current_thread")]
async fn usage_compare_model_falls_back_to_pricing_json() {
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    write_pricing(
        dir.path(),
        json!({"test-prov/test-model":{"input":2,"output":8}}),
        "2026-09-18",
    );
    let (code, out, _err) = run_usage(
        dir.path(),
        &["usage", "--compare-model", "test-prov/test-model"],
    )
    .await;
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    let e = &v["result"]["estimated"];
    assert_eq!(e["pricing"]["source"], "pricing.json");
    assert_eq!(e["pricing"]["updatedAt"], "2026-09-18");
}

#[tokio::test(flavor = "current_thread")]
async fn usage_command_does_not_record_itself() {
    // usage command は記録対象外 — usage.jsonl の行数が変わらないこと。
    let dir = tempfile::tempdir().unwrap();
    write_log(dir.path(), &[Value::Object(run_record())]);
    let before = std::fs::read_to_string(dir.path().join("usage.jsonl")).unwrap();
    let (_c, _o, _e) = run_usage(dir.path(), &["usage"]).await;
    let after = std::fs::read_to_string(dir.path().join("usage.jsonl")).unwrap();
    assert_eq!(before, after);
}

#[tokio::test(flavor = "current_thread")]
async fn usage_missing_log_reports_zero_records() {
    let dir = tempfile::tempdir().unwrap();
    let (code, out, _err) = run_usage(dir.path(), &["usage"]).await;
    assert_eq!(code, 0);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(f64_at(&v, &["result", "records"]), 0.0);
    assert_eq!(f64_at(&v, &["result", "ok"]), 0.0);
}

// --- route bucket (B4) ---------------------------------------------------------

#[test]
fn route_records_aggregate_into_independent_bucket() {
    let dir = tempfile::tempdir().unwrap();
    let route_rec = json!({
        "schemaVersion": 1, "ts": "2026-09-19T00:00:00.000Z",
        "kind": "route", "route": "cargo-test", "root": "/ws",
        "originalTokens": 1000, "returnedTokens": 250,
        "logRef": "rid-1", "exitCode": 0, "ok": true,
        "postprocessFailures": 0,
    });
    let route_fail = json!({
        "schemaVersion": 1, "ts": "2026-09-19T00:00:00.000Z",
        "kind": "route", "route": "cargo-test", "root": "/ws",
        "originalTokens": 500, "returnedTokens": 500,
        "logRef": "rid-2", "exitCode": 3, "ok": false,
        "postprocessFailures": 1,
    });
    let refetch = json!({
        "schemaVersion": 1, "ts": "2026-09-19T00:00:00.000Z",
        "kind": "route-refetch", "logRef": "rid-1",
        "bytesRead": 4096, "root": "/ws",
    });
    write_log(
        dir.path(),
        &[run_record().into(), route_rec, route_fail, refetch],
    );
    // route kind は is_record gate を通過して読まれる。
    let records = read_usage_records(dir.path());
    assert_eq!(records.len(), 4);
    let report =
        build_usage_report(dir.path(), &UsageFilters::default(), None, None, None, &now).unwrap();
    // run 集計は route record を含まない(records=1, run record のみ)。
    assert_eq!(report["records"], 1.0);
    assert_eq!(report["ok"], 1.0);
    assert_eq!(report["errors"], 0.0);
    // route bucket は独立 join。
    let route = &report["route"];
    assert_eq!(route["executions"], 2.0);
    assert_eq!(route["failures"], 1.0);
    assert_eq!(route["refetches"], 1.0);
    assert_eq!(route["refetchBytes"], 4096.0);
    assert_eq!(route["originalTokens"], 1500.0);
    assert_eq!(route["returnedTokens"], 750.0);
    assert_eq!(route["postprocessFailures"], 1.0);
}
