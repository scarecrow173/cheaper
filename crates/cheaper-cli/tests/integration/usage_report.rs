//! `usage_report.rs` 集計/`execute_usage` と `pricing.rs` 連携の test
//! (`packages/cli/test/usage-report.test.ts` の port + dispatch 経路)。

use std::collections::HashMap;
use std::path::Path;

use crate::common;
use common::{ambient_env, PipeStdin, SharedVecSink};

use cheaper_cli::cli::{run_cli, CliHooks, CliIo};
use cheaper_cli::usage_report::{
    build_usage_report, read_usage_records, UsageFilters, PRICING_STALE_MS,
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
    build_usage_report(dir, &UsageFilters::default(), None, None, &now).unwrap()
}

fn f64_at(v: &Value, path: &[&str]) -> f64 {
    let mut cur = v;
    for key in path {
        cur = &cur[*key];
    }
    cur.as_f64().expect("expected number")
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
    let err =
        build_usage_report(dir.path(), &UsageFilters::default(), None, None, &now).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
}

#[test]
fn explicit_null_savings_fails_like_js_type_error() {
    let dir = tempfile::tempdir().unwrap();
    let mut r = run_record();
    r.insert("savings".to_string(), Value::Null);
    write_log(dir.path(), &[Value::Object(r)]);
    let err =
        build_usage_report(dir.path(), &UsageFilters::default(), None, None, &now).unwrap_err();
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
    let r = build_usage_report(dir.path(), &filters, None, None, &now).unwrap();
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
    let r = build_usage_report(dir.path(), &by_provider, None, None, &now).unwrap();
    assert_eq!(f64_at(&Value::Object(r), &["records"]), 1.0);
    let by_model = UsageFilters {
        model: Some("openai/gpt-5-mini".to_string()),
        ..UsageFilters::default()
    };
    let r = build_usage_report(dir.path(), &by_model, None, None, &now).unwrap();
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
    let r = build_usage_report(dir.path(), &filters, None, None, &now).unwrap();
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
