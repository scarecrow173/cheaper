//! `usage_log.rs` の record schema / best-effort 契約 test。

use cheaper_cli::usage_log::{append_usage_log, build_usage_record, USAGE_LOG_FILENAME};
use serde_json::json;

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
    let r = build_usage_record(&env, "/w", &fixed(1_759_892_969_061.0)).unwrap();
    assert_eq!(r["ts"], "2025-10-08T03:09:29.061Z");
    // leap day 2024-02-29T12:00:00.000Z = 1709208000000
    let r = build_usage_record(&env, "/w", &fixed(1_709_208_000_000.0)).unwrap();
    assert_eq!(r["ts"], "2024-02-29T12:00:00.000Z");
    // epoch
    let r = build_usage_record(&env, "/w", &fixed(0.0)).unwrap();
    assert_eq!(r["ts"], "1970-01-01T00:00:00.000Z");
    // 年末境界 2023-12-31T23:59:59.999Z
    let r = build_usage_record(&env, "/w", &fixed(1_704_067_199_999.0)).unwrap();
    assert_eq!(r["ts"], "2023-12-31T23:59:59.999Z");
}

#[test]
fn count_record_shape_and_key_order() {
    let env = json!({
        "schemaVersion":1,"ok":true,"worker":null,
        "result":{"path":"a.txt","tokens":3,"bytes":9},
        "meta":{"durationMs":5},
    });
    let r = build_usage_record(&env, "/w", &fixed(0.0)).unwrap();
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
    let r = build_usage_record(&env, "/w", &fixed(0.0)).unwrap();
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
    assert!(build_usage_record(&env, "/w", &fixed(0.0)).is_none());

    // usage-bearing error → kind:"run" + error.code
    let env = json!({
        "schemaVersion":1,"ok":false,"worker":"w",
        "error":{"code":"TOOL_EXECUTION_FAILURE","message":"x","exitCode":6,"retryable":false},
        "meta":{"durationMs":1,"model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":1,"output":0,"cacheRead":0,"cacheWrite":0,"totalTokens":1,"costUsd":0},
                "turns":1,"toolCalls":1,"rejectedToolCalls":null,"repairs":null,
                "transientRetries":null,"changes":null,"savings":null,"execution":null},
    });
    let r = build_usage_record(&env, "/w", &fixed(0.0)).unwrap();
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
    assert!(build_usage_record(&env, "/w", &fixed(0.0)).is_none());
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
