//! `usage_log.rs` の record schema / best-effort 契約 test。
//!
//! 後半は usage.jsonl の新旧双方向互換(T9):
//! - TS `buildUsageRecord` の現行 record shape を Rust `read_usage_records`
//!   / `build_usage_report` が読める(旧 reader → 新 writer 方向の逆)。
//! - Rust `build_usage_record` の出力が旧 reader 系(`isRecord` +
//!   `aggregate` が触る field 群)の前提を満たす additive-only record で
//!   あること。
//!
//! 実 TS reader/writer との spawn cross-check は TS 実装撤去(Phase 6)
//! に伴い廃止 — record schema の互換は上記 2 test と regression
//! snapshot(`cli-usage-*` case)が pin する。

use std::path::Path;

use cheaper_cli::usage_log::{
    append_usage_log, build_usage_record, ExecutionEcho, USAGE_LOG_FILENAME,
};
use cheaper_cli::usage_report::{build_usage_report, read_usage_records, UsageFilters};
use serde_json::{json, Value};

fn fixed(ms: f64) -> impl Fn() -> f64 {
    move || ms
}

// epoch ms → `toISOString` 互換
#[test]
fn iso8601_timestamps() {
    // count record を媒介に ts を観測する。
    let env = json!({
        "schemaVersion":1,"ok":true,"worker":null,
        "result":{"path":"a.txt","tokens":3,"bytes":9},
        "meta":{"durationMs":1},
    });
    // 2025-10-08T03:09:29.061Z = 1759892969061
    let r = build_usage_record(&env, "/w", None, &fixed(1_759_892_969_061.0)).unwrap();
    assert_eq!(r["ts"], "2025-10-08T03:09:29.061Z");
    // leap day 2024-02-29T12:00:00.000Z = 1709208000000
    let r = build_usage_record(&env, "/w", None, &fixed(1_709_208_000_000.0)).unwrap();
    assert_eq!(r["ts"], "2024-02-29T12:00:00.000Z");
    // epoch
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    assert_eq!(r["ts"], "1970-01-01T00:00:00.000Z");
    // 年末境界 2023-12-31T23:59:59.999Z
    let r = build_usage_record(&env, "/w", None, &fixed(1_704_067_199_999.0)).unwrap();
    assert_eq!(r["ts"], "2023-12-31T23:59:59.999Z");
}

#[test]
fn count_record_shape_and_key_order() {
    let env = json!({
        "schemaVersion":1,"ok":true,"worker":null,
        "result":{"path":"a.txt","tokens":3,"bytes":9},
        "meta":{"durationMs":5},
    });
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        "{\"schemaVersion\":1,\"ts\":\"1970-01-01T00:00:00.000Z\",\"root\":\"/w\",\"durationMs\":5,\"kind\":\"count\",\"worker\":null,\"ok\":true,\"result\":{\"path\":\"a.txt\",\"tokens\":3,\"bytes\":9}}"
    );
}

#[test]
fn run_record_shape() {
    let env = json!({
        "schemaVersion":1,"ok":true,"worker":"w",
        "result":{},
        "meta":{"durationMs":7,"model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":1,"output":2,"cacheRead":0,"cacheWrite":0,"totalTokens":3,"costUsd":0.0},
                "turns":1,"toolCalls":2,"savings":{"sourceTokens":9,"returnedTokens":1,"avoidedContextTokens":8,"directlyComparable":true}},
    });
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    assert_eq!(r["kind"], "run");
    assert_eq!(r["worker"], "w");
    assert_eq!(r["ok"], true);
    assert_eq!(r["model"]["id"], "m");
    assert_eq!(r["turns"], 1);
    assert_eq!(r["savings"]["avoidedContextTokens"], 8);
}

#[test]
fn error_record_only_when_usage_bearing() {
    // usage 無し error → 記録対象外
    let env = json!({
        "schemaVersion":1,"ok":false,"worker":null,
        "error":{"code":"INVALID_REQUEST","message":"x","exitCode":2,"retryable":false},
        "meta":{"durationMs":1,"model":null,"usage":null,"turns":null,"toolCalls":null,
                "rejectedToolCalls":null,"repairs":null,"transientRetries":null,
                "changes":null,"savings":null,"execution":null},
    });
    assert!(build_usage_record(&env, "/w", None, &fixed(0.0)).is_none());

    // usage-bearing error → kind:"run" + error.code
    let env = json!({
        "schemaVersion":1,"ok":false,"worker":"w",
        "error":{"code":"TOOL_EXECUTION_FAILURE","message":"x","exitCode":6,"retryable":false},
        "meta":{"durationMs":1,"model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":1,"output":0,"cacheRead":0,"cacheWrite":0,"totalTokens":1,"costUsd":0},
                "turns":1,"toolCalls":1,"rejectedToolCalls":null,"repairs":null,
                "transientRetries":null,"changes":null,"savings":null,"execution":null},
    });
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    assert_eq!(r["kind"], "run");
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["code"], "TOOL_EXECUTION_FAILURE");
    // null の turns/toolCalls/savings は載らない(TS `!== null` 条件)
    assert!(r.get("turns").is_some()); // turns=1 は number
    assert!(r.get("savings").is_none());
}

#[test]
fn workflow_envelope_not_recorded() {
    let env = json!({
        "schemaVersion":1,"workflow":true,"ok":true,"worker":null,
        "result":{},"jobs":[],"meta":{"durationMs":1},
    });
    assert!(build_usage_record(&env, "/w", None, &fixed(0.0)).is_none());
}

#[test]
fn append_writes_jsonl_line() {
    let dir = tempfile::tempdir().unwrap();
    let ucd = dir.path().join("cfg");
    let rec = json!({"schemaVersion":1,"ts":"1970-01-01T00:00:00.000Z","root":"/w","durationMs":1,"kind":"count","worker":null,"ok":true,"result":{"tokens":3}});
    append_usage_log(ucd.to_str().unwrap(), &rec).unwrap();
    append_usage_log(ucd.to_str().unwrap(), &rec).unwrap();
    let text = std::fs::read_to_string(ucd.join(USAGE_LOG_FILENAME)).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(text.ends_with('\n'));
}

// ---------------------------------------------------------------------------
// 新旧双方向互換(T9)
// ---------------------------------------------------------------------------

fn write_log_lines(dir: &Path, lines: &[&str]) {
    let mut text = lines.join("\n");
    text.push('\n');
    std::fs::write(dir.join(USAGE_LOG_FILENAME), text).unwrap();
}

/// `Date.parse` 互換の ts が filter で生きる程度に新しい値(NOW は
/// tests/usage_report.rs と同じ 2026-09-20)。
const NOW_MS: f64 = 1_789_862_400_000.0;

/// TS `buildUsageRecord`(現行 writer)が emit する literal shape —
/// field 順は TS object literal 構築順。
/// success run / usage-bearing error / count の 3 形 + 旧 schema の
/// 最小 record(turns/toolCalls/savings 無し)を混在させる。
const TS_RUN_OK: &str = "{\"schemaVersion\":1,\"ts\":\"2026-09-19T00:00:00.000Z\",\"root\":\"/ws\",\"durationMs\":10,\"kind\":\"run\",\"worker\":\"bulk-reader\",\"ok\":true,\"model\":{\"provider\":\"ollama\",\"id\":\"qwen3.5:9b\",\"tier\":\"low\"},\"usage\":{\"input\":100,\"output\":20,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":120,\"costUsd\":0.01},\"turns\":2,\"toolCalls\":3,\"savings\":{\"sourceTokens\":800,\"returnedTokens\":50,\"avoidedContextTokens\":750,\"directlyComparable\":true}}";
const TS_RUN_ERR: &str = "{\"schemaVersion\":1,\"ts\":\"2026-09-19T00:01:00.000Z\",\"root\":\"/ws\",\"durationMs\":20,\"kind\":\"run\",\"worker\":\"bulk-reader\",\"ok\":false,\"model\":{\"provider\":\"ollama\",\"id\":\"qwen3.5:9b\",\"tier\":\"low\"},\"usage\":{\"input\":50,\"output\":5,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":55,\"costUsd\":0.005},\"turns\":1,\"toolCalls\":1,\"error\":{\"code\":\"INVALID_OUTPUT\"}}";
const TS_COUNT: &str = "{\"schemaVersion\":1,\"ts\":\"2026-09-19T00:02:00.000Z\",\"root\":\"/ws\",\"durationMs\":3,\"kind\":\"count\",\"worker\":null,\"ok\":true,\"result\":{\"path\":\"a.txt\",\"tokens\":3,\"bytes\":9}}";
/// 旧 schema record — `turns`/`toolCalls`/`savings`/`error` を持たない
/// 最小 run record(issue #44 前の writer が出した形)。
const OLD_RUN_MINIMAL: &str = "{\"schemaVersion\":1,\"ts\":\"2026-09-19T00:03:00.000Z\",\"root\":\"/ws\",\"durationMs\":7,\"kind\":\"run\",\"worker\":\"bulk-reader\",\"ok\":true,\"model\":{\"provider\":\"ollama\",\"id\":\"qwen3.5:9b\",\"tier\":\"low\"},\"usage\":{\"input\":10,\"output\":2,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":12,\"costUsd\":0.001}}";
/// 旧 savings — `directlyComparable` flag を持たない形。
const OLD_RUN_SAVINGS_NO_FLAG: &str = "{\"schemaVersion\":1,\"ts\":\"2026-09-19T00:04:00.000Z\",\"root\":\"/ws\",\"durationMs\":9,\"kind\":\"run\",\"worker\":\"bulk-reader\",\"ok\":true,\"model\":{\"provider\":\"ollama\",\"id\":\"qwen3.5:9b\",\"tier\":\"low\"},\"usage\":{\"input\":30,\"output\":6,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":36,\"costUsd\":0.003},\"savings\":{\"sourceTokens\":400,\"returnedTokens\":20,\"avoidedContextTokens\":380}}";

#[test]
fn ts_writer_shape_records_parse_and_aggregate() {
    // TS writer の現行 shape + 旧 schema record の混在 file を
    // Rust reader が読み、集計が新旧双方を合算できる。
    let dir = tempfile::tempdir().unwrap();
    write_log_lines(
        dir.path(),
        &[
            TS_RUN_OK,
            TS_RUN_ERR,
            TS_COUNT,
            OLD_RUN_MINIMAL,
            OLD_RUN_SAVINGS_NO_FLAG,
        ],
    );
    let records = read_usage_records(dir.path());
    assert_eq!(records.len(), 5);
    let report = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        None,
        None,
        &|| NOW_MS,
    )
    .unwrap();
    let r = Value::Object(report);
    // count record は run 集計から外れる → run 4 件(ok 3 + error 1)。
    // report の数値は f64 系(Number::Float)なので float literal で比較。
    assert_eq!(r["records"], json!(4.0));
    assert_eq!(r["ok"], json!(3.0));
    assert_eq!(r["errors"], json!(1.0));
    assert_eq!(r["usage"]["input"], json!(190.0));
    assert_eq!(r["usage"]["costUsd"], json!(0.019));
    // 旧 record の turns/toolCalls は `?? 0` で 0 加算。
    assert_eq!(r["turns"], json!(3.0));
    assert_eq!(r["toolCalls"], json!(4.0));
    assert_eq!(r["durationMs"], json!(46.0));
    // savings 持ち 2 件が measured 母数、flag 持ちは新 record のみ。
    assert_eq!(r["measured"]["runs"], json!(2.0));
    assert_eq!(r["measured"]["sourceTokens"], json!(1200.0));
    assert_eq!(r["measured"]["comparable"]["runs"], json!(1.0));
    assert_eq!(r["measured"]["comparable"]["costUsd"], json!(0.01));
}

#[test]
fn rust_written_records_satisfy_ts_reader_field_contract() {
    // Rust writer の出力が TS reader(`isRecord` + `filterRunRecords` +
    // `aggregate` + `estimate`)が触る field を全て正しい JSON 型で
    // 持つことを assert する(additive-only schema contract)。
    let run_env = json!({
        "schemaVersion":1,"ok":true,"worker":"bulk-reader","result":{},
        "meta":{"durationMs":7,"model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":1,"output":2,"cacheRead":0,"cacheWrite":0,"totalTokens":3,"costUsd":0.5},
                "turns":1,"toolCalls":2,"savings":{"sourceTokens":9,"returnedTokens":1,"avoidedContextTokens":8,"directlyComparable":true}},
    });
    let err_env = json!({
        "schemaVersion":1,"ok":false,"worker":"bulk-reader",
        "error":{"code":"INVALID_OUTPUT","message":"x","exitCode":6,"retryable":false},
        "meta":{"durationMs":1,"model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":1,"output":0,"cacheRead":0,"cacheWrite":0,"totalTokens":1,"costUsd":0},
                "turns":1,"toolCalls":1,"rejectedToolCalls":null,"repairs":null,
                "transientRetries":null,"changes":null,"savings":null,"execution":null},
    });
    let count_env = json!({
        "schemaVersion":1,"ok":true,"worker":null,
        "result":{"path":"a.txt","tokens":3,"bytes":9},
        "meta":{"durationMs":5},
    });
    let run = build_usage_record(&run_env, "/w", None, &fixed(0.0)).unwrap();
    let err = build_usage_record(&err_env, "/w", None, &fixed(0.0)).unwrap();
    let count = build_usage_record(&count_env, "/w", None, &fixed(0.0)).unwrap();

    let is_num = |v: &Value, k: &str| v.get(k).is_some_and(Value::is_number);
    for rec in [&run, &err, &count] {
        // isRecord の前提: object + kind ∈ {"run","count"}。
        assert!(rec.is_object());
        assert!(matches!(
            rec.get("kind").and_then(Value::as_str),
            Some("run") | Some("count")
        ));
        // filterRunRecords / aggregate が読む共通 field。
        assert!(rec.get("schemaVersion").is_some_and(|v| v == &json!(1)));
        assert!(rec
            .get("ts")
            .and_then(Value::as_str)
            .is_some_and(|s| { s.ends_with('Z') && s.len() == "1970-01-01T00:00:00.000Z".len() }));
        assert!(rec.get("ok").is_some_and(Value::is_boolean));
        assert!(rec
            .get("worker")
            .is_some_and(|v| v.is_string() || v.is_null()));
        assert!(rec
            .get("durationMs")
            .is_some_and(|v| v.is_number() || v.is_null()));
    }
    for rec in [&run, &err] {
        let model = rec.get("model").unwrap();
        for k in ["provider", "id", "tier"] {
            assert!(model.get(k).is_some_and(Value::is_string), "model.{k}");
        }
        let usage = rec.get("usage").unwrap();
        for k in [
            "input",
            "output",
            "cacheRead",
            "cacheWrite",
            "totalTokens",
            "costUsd",
        ] {
            assert!(is_num(usage, k), "usage.{k}");
        }
        // aggregate の `?? 0` 対象 — 存在するなら number。
        for k in ["turns", "toolCalls"] {
            assert!(is_num(rec, k), "{k}");
        }
    }
    // savings(run success のみ): measured/comparable 集計が読む field。
    let savings = run.get("savings").unwrap();
    for k in ["sourceTokens", "returnedTokens", "avoidedContextTokens"] {
        assert!(is_num(savings, k), "savings.{k}");
    }
    assert_eq!(savings["directlyComparable"], true);
    // error record は `error.code` を持つ(reader は集計しないが
    // schema 上の記録 field)。
    assert_eq!(err["error"]["code"], "INVALID_OUTPUT");
    // count record: reader は kind のみ見るが result shape は writer
    // contract として固定。
    assert!(is_num(&count["result"], "tokens"));
    assert!(is_num(&count["result"], "bytes"));
    assert!(count["result"]["path"].is_string());

    // Rust writer の serialize 出力を file に流して Rust reader で
    // round-trip できること(additive-only: 未知 field も保持)。
    let dir = tempfile::tempdir().unwrap();
    for rec in [&run, &err, &count] {
        append_usage_log(dir.path().to_str().unwrap(), rec).unwrap();
    }
    let parsed = read_usage_records(dir.path());
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0], run);
    assert_eq!(parsed[1], err);
    assert_eq!(parsed[2], count);
}

// --- route / route-refetch records (B4) ---------------------------------------

#[test]
fn route_record_shape() {
    use cheaper_cli::usage_log::{build_route_record, RouteRecordInput};
    let input = RouteRecordInput {
        route: "cargo-test".to_string(),
        command: "SECRET_TOKEN=x cargo test".to_string(),
        executed_grant: "cargo-test".to_string(),
        executed_argv: vec!["test".to_string(), "--workspace".to_string()],
        original_tokens: 1000.0,
        returned_tokens: 250.0,
        worker: json!({"model": {"provider":"ollama","id":"q"}, "usage": {"input":1}}),
        duration_ms: json!({"exec": 10, "analyze": 5, "total": 15}),
        log_ref: "1700000000000-1-abcd".to_string(),
        refetches: 0.0,
        postprocess_failures: 0.0,
        suppressed: None,
        exit_code: 0,
        ok: true,
    };
    let r = build_route_record(&input, "/w", &fixed(0.0));
    assert_eq!(r["kind"], "route");
    assert_eq!(r["route"], "cargo-test");
    assert_eq!(r["logRef"], "1700000000000-1-abcd");
    assert_eq!(r["exitCode"], 0);
    assert_eq!(r["originalTokens"], 1000.0);
    assert_eq!(r["returnedTokens"], 250.0);
    assert_eq!(r["reductionRatio"], 0.75);
    assert_eq!(r["ok"], true);
    // `executed` — #166 additive field、commandHash 直後の位置に
    // 実実行 `{grant, argv}` を記録する。
    assert_eq!(r["executed"]["grant"], "cargo-test");
    assert_eq!(r["executed"]["argv"], json!(["test", "--workspace"]));
    let keys: Vec<&String> = r.as_object().unwrap().keys().collect();
    let hash_pos = keys
        .iter()
        .position(|k| k.as_str() == "commandHash")
        .unwrap();
    assert_eq!(keys[hash_pos + 1].as_str(), "executed", "keys: {keys:?}");
    // command 本文(env 値を含み得る)は `command` には載らず
    // `commandHash` のみ(executed.argv は実行 argv で env-prefix を
    // 含まない — `SECRET_TOKEN=` prefix は argv ではなく env-prefix
    // 経路に置かれるため record のどこにも出ない)。
    let text = serde_json::to_string(&r).unwrap();
    assert!(!text.contains("SECRET_TOKEN"), "{text}");
    assert!(r["commandHash"].as_str().unwrap().len() == 16);
    // deterministic — 同入力は同 hash。
    let r2 = build_route_record(&input, "/w", &fixed(0.0));
    assert_eq!(r["commandHash"], r2["commandHash"]);
    assert!(r.get("suppressed").is_none(), "None で wire 不変");
    let input = RouteRecordInput {
        suppressed: Some("project-shadow".to_string()),
        ..input
    };
    let r = build_route_record(&input, "/w", &fixed(0.0));
    assert_eq!(r["suppressed"], "project-shadow");
}

#[test]
fn route_refetch_record_shape() {
    use cheaper_cli::usage_log::build_route_refetch_record;
    let r = build_route_refetch_record("rid-1", 4096, "/w", &fixed(0.0));
    assert_eq!(r["kind"], "route-refetch");
    assert_eq!(r["logRef"], "rid-1");
    assert_eq!(r["bytesRead"], 4096);
    assert_eq!(r["root"], "/w");
}

#[test]
fn route_and_refetch_records_accept_additive_execution_correlation() {
    use cheaper_cli::usage_log::{
        build_route_record_with_correlation, build_route_refetch_record_with_correlation,
        RouteRecordInput, UsageCorrelation,
    };
    let input = RouteRecordInput {
        route: "cargo-test".to_string(),
        command: "cargo test".to_string(),
        executed_grant: "cargo-test".to_string(),
        executed_argv: vec!["test".to_string()],
        original_tokens: 10.0,
        returned_tokens: 5.0,
        worker: json!({}),
        duration_ms: json!({"exec": 1, "analyze": 1, "total": 2}),
        log_ref: "rid-1".to_string(),
        refetches: 0.0,
        postprocess_failures: 0.0,
        suppressed: None,
        exit_code: 0,
        ok: true,
    };
    let correlation = UsageCorrelation {
        execution_id: Some("route-execution".to_string()),
        parent_execution_id: Some("workflow-execution".to_string()),
        trace_id: Some("trace-root".to_string()),
    };

    let route = build_route_record_with_correlation(&input, Some(&correlation), "/w", &fixed(0.0));
    assert_eq!(route["executionId"], "route-execution");
    assert_eq!(route["parentExecutionId"], "workflow-execution");
    assert_eq!(route["traceId"], "trace-root");

    let refetch = build_route_refetch_record_with_correlation(
        "rid-1",
        4096,
        Some(&correlation),
        "/w",
        &fixed(0.0),
    );
    assert_eq!(refetch["kind"], "route-refetch");
    assert_eq!(refetch["logRef"], "rid-1");
    assert_eq!(refetch["executionId"], "route-execution");
    assert_eq!(refetch["parentExecutionId"], "workflow-execution");
    assert_eq!(refetch["traceId"], "trace-root");
}

// ---------------------------------------------------------------------------
// diagnostic fields(issue #141 — TSPEC-034Kr7KMznzk97mL1fbS1b)
// `version`/`repairs`/`execution` echo・error allowlist・cap・fail-soft。
// ---------------------------------------------------------------------------

/// usage-bearing error envelope — `error`・`meta.repairs` は test ごとに
/// 差し替える。`meta.usage`/`meta.model` が非 null なので常に記録対象。
fn error_env(error: Value, repairs: Value) -> Value {
    json!({
        "schemaVersion":1,"ok":false,"worker":"w",
        "error":error,
        "meta":{"durationMs":1,
                "model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":1,"output":0,"cacheRead":0,"cacheWrite":0,"totalTokens":1,"costUsd":0},
                "turns":1,"toolCalls":1,"repairs":repairs},
    })
}

/// success run envelope — `meta.repairs` のみ差し替え。
fn ok_env(repairs: Value) -> Value {
    json!({
        "schemaVersion":1,"ok":true,"worker":"w","result":{},
        "meta":{"durationMs":7,
                "model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":1,"output":2,"cacheRead":0,"cacheWrite":0,"totalTokens":3,"costUsd":0.0},
                "turns":1,"toolCalls":2,"repairs":repairs},
    })
}

fn top_keys(r: &Value) -> Vec<String> {
    r.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn run_record_version_is_binary_version_right_after_ok() {
    // success・error 両分岐で `version` が `env!("CARGO_PKG_VERSION")`
    // (この binary の product version)かつ `ok` 直後の固定位置。
    for env in [
        ok_env(json!(null)),
        error_env(json!({"code":"ABORTED","message":"x"}), json!(null)),
    ] {
        let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
        assert_eq!(r["version"], env!("CARGO_PKG_VERSION"));
        let keys = top_keys(&r);
        let i = keys.iter().position(|k| k == "ok").unwrap();
        assert_eq!(keys[i + 1], "version");
    }
}

#[test]
fn run_record_repairs_recorded_only_when_non_null() {
    for env in [
        ok_env(json!(2)),
        error_env(json!({"code":"INVALID_OUTPUT","message":"x"}), json!(2)),
    ] {
        let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
        assert_eq!(r["repairs"], 2);
    }
    for env in [
        ok_env(json!(null)),
        error_env(json!({"code":"INVALID_OUTPUT","message":"x"}), json!(null)),
    ] {
        let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
        assert!(r.get("repairs").is_none(), "null repairs must be omitted");
    }
}

#[test]
fn run_record_execution_echo_shape_and_position() {
    let echo = ExecutionEcho {
        timeout_ms: 45_000,
        max_turns: Some(9),
        max_tool_calls: 5,
    };
    for env in [
        ok_env(json!(1)),
        error_env(json!({"code":"TIMEOUT","message":"x"}), json!(1)),
    ] {
        let r = build_usage_record(&env, "/w", Some(&echo), &fixed(0.0)).unwrap();
        assert_eq!(
            r["execution"],
            json!({"timeoutMs":45000,"maxTurns":9,"maxToolCalls":5})
        );
        // `repairs` 直後の固定位置。
        let keys = top_keys(&r);
        let i = keys.iter().position(|k| k == "repairs").unwrap();
        assert_eq!(keys[i + 1], "execution");
    }
    // `max_turns: None` → `null` として記録(key 有り — echo 未供給の
    // key 欠落とは区別される)。
    let echo = ExecutionEcho {
        timeout_ms: 45_000,
        max_turns: None,
        max_tool_calls: 5,
    };
    let r = build_usage_record(&ok_env(json!(null)), "/w", Some(&echo), &fixed(0.0)).unwrap();
    assert!(r["execution"].get("maxTurns").is_some_and(Value::is_null));
    // `echo: None` → `execution` key 自体が無い(fail-soft)。
    for env in [ok_env(json!(1)), error_env(json!({"code":"X"}), json!(1))] {
        let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
        assert!(r.get("execution").is_none());
    }
}

#[test]
fn execution_echo_from_policy_projects_only_three_fields() {
    // `From<&ExecutionPolicy>` は 3 field 固定射影 — `concurrency` 等の
    // 他 field は record shape に漏れない。
    let policy = cheaper_core::policy::schema::ExecutionPolicy {
        timeout_ms: 300_000,
        max_turns: None,
        max_tool_calls: 64,
        concurrency: None,
    };
    let echo = ExecutionEcho::from(&policy);
    assert_eq!(echo.timeout_ms, 300_000);
    assert_eq!(echo.max_turns, None);
    assert_eq!(echo.max_tool_calls, 64);
    let r = build_usage_record(&ok_env(json!(null)), "/w", Some(&echo), &fixed(0.0)).unwrap();
    assert_eq!(
        r["execution"],
        json!({"timeoutMs":300000,"maxTurns":null,"maxToolCalls":64})
    );
}

#[test]
fn error_record_projects_allowlist_fields_in_fixed_order() {
    let env = error_env(
        json!({
            "code":"MAX_TURNS_EXCEEDED","message":"worker run aborted: maxTurns",
            "details":{
                "reason":"maxTurns",
                "errors":[{"instancePath":"/a","message":"required"},{"instancePath":"/b"}],
                "errorMessage":"GetChatMessage HTTP 500: upstream",
                "checkpoint":{"path":"/cfg/checkpoints/x.json","observationsCount":3,"extra":"drop"},
            },
        }),
        json!(2),
    );
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    let e = &r["error"];
    assert_eq!(e["code"], "MAX_TURNS_EXCEEDED");
    assert_eq!(e["reason"], "maxTurns");
    assert_eq!(e["message"], "worker run aborted: maxTurns");
    // entry の欠落 key はその key を省略(message 無し entry)。
    assert_eq!(
        e["schemaErrors"],
        json!([{"instancePath":"/a","message":"required"},{"instancePath":"/b"}])
    );
    assert_eq!(e["errorMessage"], "GetChatMessage HTTP 500: upstream");
    // checkpoint は {path, observationsCount} のみ — `extra` は漏らさない。
    assert_eq!(
        e["checkpoint"],
        json!({"path":"/cfg/checkpoints/x.json","observationsCount":3})
    );
    // 挿入順: code → reason → message → schemaErrors → errorMessage → checkpoint。
    let keys: Vec<&str> = e.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "code",
            "reason",
            "message",
            "schemaErrors",
            "errorMessage",
            "checkpoint"
        ]
    );
}

#[test]
fn error_record_excludes_non_allowlist_keys() {
    // `rawOutput`/`rawOutputPath`/`exitCode`/`retryable`/`cause` と
    // details 残余 key(`changes`/`missing`/`grant`/`layer`)は record に
    // 載らない(ADR Option A 固定 allowlist)。
    let env = error_env(
        json!({
            "code":"INVALID_OUTPUT","message":"x","exitCode":6,"retryable":false,
            "cause":{"code":"ABORTED","message":"inner"},
            "details":{
                "rawOutput":"raw model output",
                "rawOutputPath":"/cfg/invalid-outputs/run-1.txt",
                "changes":{"files":[]},"missing":["a"],"grant":"cmd","layer":"user",
            },
        }),
        json!(null),
    );
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    let e = r["error"].as_object().unwrap();
    let keys: Vec<&str> = e.keys().map(String::as_str).collect();
    assert_eq!(keys, ["code", "message"]);
    let serialized = serde_json::to_string(&r).unwrap();
    for banned in [
        "rawOutput",
        "rawOutputPath",
        "exitCode",
        "retryable",
        "cause",
        "changes",
        "missing",
        "grant",
        "layer",
    ] {
        assert!(
            !serialized.contains(banned),
            "{banned} must not leak: {serialized}"
        );
    }
}

#[test]
fn error_record_caps_truncate_char_based() {
    // multi-byte 文字を含む入力で char 基準に切り詰め(byte 境界で
    // 文字を壊さない)。
    let long_message = "あ".repeat(400); // 400 chars / 1200 bytes
    let long_error_message = "x".repeat(400);
    let long_path = "イ".repeat(250);
    let long_entry_message = "e".repeat(150);
    let mut errors: Vec<Value> = (0..7)
        .map(|i| json!({"instancePath":format!("/{i}"),"message":"m"}))
        .collect();
    errors[0] = json!({"instancePath":long_path,"message":long_entry_message});
    let env = error_env(
        json!({
            "code":"INVALID_OUTPUT","message":long_message,
            "details":{"errors":errors,"errorMessage":long_error_message},
        }),
        json!(null),
    );
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    let e = &r["error"];
    let m = e["message"].as_str().unwrap();
    assert_eq!(m.chars().count(), 300);
    assert!(m.ends_with('あ'), "char-based truncate keeps scalar intact");
    assert_eq!(e["errorMessage"].as_str().unwrap().chars().count(), 300);
    let se = e["schemaErrors"].as_array().unwrap();
    assert_eq!(se.len(), 5, "errors array capped at 5");
    assert_eq!(se[0]["instancePath"].as_str().unwrap().chars().count(), 200);
    assert_eq!(se[0]["message"].as_str().unwrap().chars().count(), 120);
}

#[test]
fn error_record_omits_fields_conditionally() {
    // abort 語彙外の `details.reason`・両 key 欠落の `checkpoint` は
    // 非記録。non-object errors entry は射影対象外。
    let env = error_env(
        json!({
            "code":"TOOL_EXECUTION_FAILURE","message":"x",
            "details":{
                "reason":"user",
                "errors":[{"instancePath":"/a"},{"message":"m"},"non-object",{"instancePath":"/b","message":null}],
                "checkpoint":{"other":1},
            },
        }),
        json!(null),
    );
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    let e = &r["error"];
    assert!(
        e.get("reason").is_none(),
        "non-abort reason must be omitted"
    );
    assert!(e.get("checkpoint").is_none());
    assert_eq!(
        e["schemaErrors"],
        json!([{"instancePath":"/a"},{"message":"m"},{"instancePath":"/b"}])
    );
    // PARTIAL_MUTATION 相当 — `reason` が cause 側に畳まれた record は
    // checkpoint のみを記録する構造的 edge を pin。
    let env = error_env(
        json!({
            "code":"PARTIAL_MUTATION","message":"x",
            "cause":{"code":"MAX_TOOL_CALLS_EXCEEDED","message":"inner"},
            "details":{"checkpoint":{"path":"/cfg/checkpoints/p.json","observationsCount":7}},
        }),
        json!(null),
    );
    let r = build_usage_record(&env, "/w", None, &fixed(0.0)).unwrap();
    assert!(r["error"].get("reason").is_none());
    assert_eq!(
        r["error"]["checkpoint"],
        json!({"path":"/cfg/checkpoints/p.json","observationsCount":7})
    );
}

#[test]
fn error_record_fail_soft_on_missing_or_type_mismatch() {
    // 新 field の欠落・型不一致でも `build_usage_record` が `Some` を
    // 返し panic しない(record 書き込み自体を落とさない)。
    for error in [
        json!({"code":"ABORTED"}), // message/details 欠落
        json!({"code":"X","message":42,"details":"not-an-object"}),
        json!({"code":"X","details":{"reason":123,"errors":"not-array","errorMessage":{},"checkpoint":"s"}}),
        json!({"details":{"errors":[{"instancePath":5,"message":true}]}}), // code 欠落
    ] {
        let r = build_usage_record(&error_env(error, json!(null)), "/w", None, &fixed(0.0))
            .expect("record must still be produced");
        assert_eq!(r["kind"], "run");
        assert!(r["error"].is_object());
    }
    // `code` 欠落は従来規約どおり `null` を記録する。
    let r = build_usage_record(
        &error_env(
            json!({"details":{"errors":[{"instancePath":5,"message":true}]}}),
            json!(null),
        ),
        "/w",
        None,
        &fixed(0.0),
    )
    .unwrap();
    assert!(r["error"]["code"].is_null());
    // 型不一致 entry の欠落 key は省略された空 object entry になる。
    assert_eq!(r["error"]["schemaErrors"], json!([{}]));
}

// ---------------------------------------------------------------------------
// `cheaper usage` 非回帰 — 新 field(version/execution/repairs/error.*)を
// 持つ record が混在する file を従来どおり集計できる(reader は未知
// field を無視する構造保証)。
// ---------------------------------------------------------------------------

/// 新 writer の error record(version/execution/repairs・error の
/// reason/message/schemaErrors/errorMessage/checkpoint を含む形)。
const NEW_RUN_ERR_DIAG: &str = "{\"schemaVersion\":1,\"ts\":\"2026-09-19T00:05:00.000Z\",\"root\":\"/ws\",\"durationMs\":20,\"kind\":\"run\",\"worker\":\"bulk-reader\",\"ok\":false,\"version\":\"1.0.0\",\"model\":{\"provider\":\"ollama\",\"id\":\"qwen3.5:9b\",\"tier\":\"low\"},\"usage\":{\"input\":50,\"output\":5,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":55,\"costUsd\":0.005},\"turns\":1,\"toolCalls\":1,\"repairs\":2,\"execution\":{\"timeoutMs\":300000,\"maxTurns\":null,\"maxToolCalls\":64},\"error\":{\"code\":\"MAX_TURNS_EXCEEDED\",\"reason\":\"maxTurns\",\"message\":\"worker run aborted: maxTurns\",\"schemaErrors\":[{\"instancePath\":\"/a\",\"message\":\"required\"}],\"errorMessage\":\"m\",\"checkpoint\":{\"path\":\"/cfg/checkpoints/x.json\",\"observationsCount\":3}}}";

#[test]
fn usage_report_aggregates_records_with_diagnostic_fields() {
    let dir = tempfile::tempdir().unwrap();
    write_log_lines(
        dir.path(),
        &[TS_RUN_OK, NEW_RUN_ERR_DIAG, TS_COUNT, OLD_RUN_MINIMAL],
    );
    // reader は record を全 field 保持で parse する(unknown field 無視
    // ではなく保持 — serialize した record の round-trip)。
    let records = read_usage_records(dir.path());
    assert_eq!(records.len(), 4);
    assert_eq!(records[1]["version"], "1.0.0");
    assert_eq!(records[1]["execution"]["maxToolCalls"], 64);
    assert_eq!(records[1]["error"]["reason"], "maxTurns");
    // 集計は既知 field のみ — 新旧混在で従来どおりの結果。
    let report = build_usage_report(
        dir.path(),
        &UsageFilters::default(),
        None,
        None,
        None,
        &|| NOW_MS,
    )
    .unwrap();
    let r = Value::Object(report);
    assert_eq!(r["records"], json!(3.0));
    assert_eq!(r["ok"], json!(2.0));
    assert_eq!(r["errors"], json!(1.0));
    assert_eq!(r["usage"]["input"], json!(160.0));
    assert_eq!(r["usage"]["costUsd"], json!(0.016));
    assert_eq!(r["turns"], json!(3.0));
    assert_eq!(r["toolCalls"], json!(4.0));
    assert_eq!(r["durationMs"], json!(37.0));
    assert_eq!(r["measured"]["runs"], json!(1.0));
    assert_eq!(r["measured"]["comparable"]["runs"], json!(1.0));
}

#[test]
fn usage_log_report_callers() {
    // この module の既存 report caller 2 種（旧TS、新diagnostic record）。
    let dir = tempfile::tempdir().unwrap();
    for error_line in [TS_RUN_ERR, NEW_RUN_ERR_DIAG] {
        write_log_lines(
            dir.path(),
            &[TS_RUN_OK, error_line, TS_COUNT, OLD_RUN_MINIMAL],
        );
        let report = build_usage_report(
            dir.path(),
            &UsageFilters::default(),
            None,
            None,
            None,
            &|| NOW_MS,
        )
        .unwrap();
        assert_eq!(report["records"], json!(3.0));
        assert_eq!(report["ok"], json!(2.0));
        assert_eq!(report["errors"], json!(1.0));
        assert_eq!(report["usage"]["input"], json!(160.0));
        assert_eq!(report["measured"]["runs"], json!(1.0));
        assert!(!report.contains_key("byRoute"));
        assert!(!report.contains_key("byError"));
    }
}
