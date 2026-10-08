//! `usage_log.rs` の record schema / best-effort 契約 test。
//!
//! 後半は usage.jsonl の新旧双方向互換(T9):
//! - TS `buildUsageRecord` の現行 record shape を Rust `read_usage_records`
//!   / `build_usage_report` が読める(旧 reader → 新 writer 方向の逆)。
//! - Rust `build_usage_record` の出力が TS reader(`isRecord` +
//!   `aggregate` が触る field 群)の前提を満たす additive-only record で
//!   あること。
//! - node + `packages/cli/dist` があれば実 TS reader/writer との
//!   cross-check を spawn で行う(differential harness と同じ skip 規約)。

use std::path::{Path, PathBuf};

use cheaper_cli::usage_log::{append_usage_log, build_usage_record, USAGE_LOG_FILENAME};
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
    let report =
        build_usage_report(dir.path(), &UsageFilters::default(), None, None, &|| NOW_MS).unwrap();
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
    let run = build_usage_record(&run_env, "/w", &fixed(0.0)).unwrap();
    let err = build_usage_record(&err_env, "/w", &fixed(0.0)).unwrap();
    let count = build_usage_record(&count_env, "/w", &fixed(0.0)).unwrap();

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

// --- 実 TS reader/writer との cross-check(node + dist があれば) -------------

/// repository root(`crates/cheaper-cli` → 2 段上)。canonicalize は
/// しない — Windows では `\\?\` UNC 接頭辞が付き file URL 化を壊す。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

/// differential harness(`tests/differential/harness.rs::node_binary`)
/// と同じ解決規約の最小版: `CHEAPER_DIFF_NODE` → `mise where node` →
/// PATH の非 shim node。
fn node_binary() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CHEAPER_DIFF_NODE") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let repo = repo_root();
    if let Ok(out) = std::process::Command::new("mise")
        .args(["where", "node"])
        .current_dir(&repo)
        .output()
    {
        if out.status.success() {
            let dir = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string());
            for cand in [dir.join("node.exe"), dir.join("node")] {
                if cand.exists() {
                    return Some(cand);
                }
            }
        }
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for cand in [dir.join("node.exe"), dir.join("node")] {
                if cand.exists()
                    && !dir
                        .to_string_lossy()
                        .replace('\\', "/")
                        .contains("mise/shims")
                {
                    return Some(cand);
                }
            }
        }
    }
    None
}

fn file_url(p: &Path) -> String {
    format!("file:///{}", p.to_string_lossy().replace('\\', "/"))
}

/// JSON.stringify 由来の整数(PosInt)と Rust f64 系(Float)の表現差を
/// 潰す正規化 — 全 number を f64 へ揃えてから比較する(JS number は
/// 区別しない)。
fn canon_numbers(v: &Value) -> Value {
    match v {
        Value::Number(n) => Value::from(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(canon_numbers).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), canon_numbers(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Rust writer が書いた usage.jsonl を実 TS `readUsageRecords` /
/// `buildUsageReport` が読み、逆に TS `buildUsageRecord` の record を
/// Rust reader が読む — 双方向 cross-check。
/// node か `packages/cli/dist` が無ければ skip(differential 規約)。
#[test]
fn ts_reader_reads_rust_written_log_and_vice_versa() {
    let repo = repo_root();
    let usage_report_js = repo.join("packages/cli/dist/usage-report.js");
    let usage_log_js = repo.join("packages/cli/dist/usage-log.js");
    let Some(node) = node_binary() else {
        eprintln!("[usage_log] node unavailable — TS cross-check skipped");
        return;
    };
    if !usage_report_js.exists() || !usage_log_js.exists() {
        eprintln!("[usage_log] packages/cli/dist missing — TS cross-check skipped");
        return;
    }

    // 1. Rust writer が record を build して JSONL へ append する。
    let dir = tempfile::tempdir().unwrap();
    let ucd = dir.path().join("cfg");
    let run_env = json!({
        "schemaVersion":1,"ok":true,"worker":"bulk-reader","result":{},
        "meta":{"durationMs":7,"model":{"provider":"p","id":"m","tier":"low"},
                "usage":{"input":100,"output":20,"cacheRead":0,"cacheWrite":0,"totalTokens":120,"costUsd":0.01},
                "turns":2,"toolCalls":3,"savings":{"sourceTokens":800,"returnedTokens":50,"avoidedContextTokens":750,"directlyComparable":true}},
    });
    let count_env = json!({
        "schemaVersion":1,"ok":true,"worker":null,
        "result":{"path":"a.txt","tokens":3,"bytes":9},
        "meta":{"durationMs":5},
    });
    for env in [&run_env, &count_env] {
        let rec = build_usage_record(env, "/ws", &fixed(1_789_862_400_000.0)).unwrap();
        append_usage_log(ucd.to_str().unwrap(), &rec).unwrap();
    }

    // 2. TS reader で同じ file を集計する + TS writer で record を 1 件
    //    生成して JSON として返してもらう。
    let script = dir.path().join("check.mjs");
    std::fs::write(
        &script,
        format!(
            concat!(
                "import {{ readUsageRecords, buildUsageReport }} from {:?};\n",
                "import {{ buildUsageRecord }} from {:?};\n",
                "const dir = process.argv[2];\n",
                "const report = buildUsageReport({{ userConfigDir: dir, filters: {{}}, now: () => 1789862400000 }});\n",
                "const tsRecord = buildUsageRecord({{\n",
                "  schemaVersion: 1, ok: true, worker: \"ts-writer\",\n",
                "  result: {{}},\n",
                "  meta: {{ durationMs: 11, model: {{ provider: \"p\", id: \"m\", tier: \"low\" }},\n",
                "          usage: {{ input: 7, output: 3, cacheRead: 0, cacheWrite: 0, totalTokens: 10, costUsd: 0.001 }},\n",
                "          turns: 1, toolCalls: 0, savings: null }},\n",
                "}}, {{ root: \"/ts\", now: () => new Date(1789862400000) }});\n",
                "process.stdout.write(JSON.stringify({{ n: readUsageRecords(dir).length, report, tsRecord }}) + \"\\n\");\n",
            ),
            file_url(&usage_report_js),
            file_url(&usage_log_js),
        ),
    )
    .unwrap();
    let out = std::process::Command::new(node)
        .arg(&script)
        .arg(&ucd)
        .output()
        .expect("spawn node");
    assert!(
        out.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ts: Value = serde_json::from_slice(&out.stdout).unwrap();

    // Rust writer の file を TS reader が全行読み、aggregate 結果が
    // Rust `build_usage_report` と一致する。
    assert_eq!(ts["n"], 2);
    let rust_report = Value::Object(
        build_usage_report(&ucd, &UsageFilters::default(), None, None, &|| NOW_MS).unwrap(),
    );
    assert_eq!(
        canon_numbers(&ts["report"]),
        canon_numbers(&rust_report),
        "TS buildUsageReport != Rust build_usage_report"
    );

    // TS writer の record を Rust reader が読める(新 writer → 旧
    // reader 相当の方向 — additive-only schema 内)。
    let ts_dir = tempfile::tempdir().unwrap();
    let line = serde_json::to_string(&ts["tsRecord"]).unwrap();
    assert!(
        ts["tsRecord"].is_object(),
        "TS buildUsageRecord returned null"
    );
    write_log_lines(ts_dir.path(), &[&line]);
    let parsed = read_usage_records(ts_dir.path());
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["kind"], "run");
    assert_eq!(parsed[0]["worker"], "ts-writer");
    let report = build_usage_report(ts_dir.path(), &UsageFilters::default(), None, None, &|| {
        NOW_MS
    })
    .unwrap();
    assert_eq!(report["records"], json!(1.0));
    assert_eq!(report["usage"]["input"], json!(7.0));
}
