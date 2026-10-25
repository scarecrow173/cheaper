//! record/aggregate schema v2 validation の contract test
//! (design §3.4 の 6 invariant + §3.5 store 規約)。
//!
//! - `schemaVersion === 2` 完全一致(v1 record reject)
//! - `parentUsage` status enum + status 別必須 key
//! - `metrics.usage.source === "scripted"` marker
//! - `environment.node` reject + `ambientRustc`/`measuredBinary.profile`
//!   必須
//! - comparisons `claim` enum + claim 別形状(unavailable 全値 null・
//!   estimated assumptions 必須・measured reason/assumptions null)
//! - `parentUsage` block 以外の数値 parent-usage field 不存在
//! - `armExpect` 照合の等値/null 期待/fallback 意味論
//! - `create_new` 非上書き書込み・v1 file の load reject

use cheaper_bench::aggregate;
use cheaper_bench::manifest::Scenario;
use cheaper_bench::measure::{self, Mode, Observation};
use cheaper_bench::record;
use cheaper_bench::report;
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// helpers — invariant を全て満たす最小の valid record
// ---------------------------------------------------------------------------

fn valid_environment() -> Value {
    json!({
        "os": "windows",
        "osRelease": "10.0.26200",
        "arch": "x86_64",
        "cpuCount": 16,
        "ambientRustc": "rustc 1.99.0 (000000000 2026-10-25)",
        "harnessProfile": "release",
    })
}

fn valid_observation() -> Value {
    json!({
        "schemaVersion": 2,
        "runId": "4f3f4db6-4c3c-4b5c-8b7c-9f4a2d3e4f5a",
        "scenarioId": "single-simple-tool-call",
        "scenarioTitle": "s",
        "arm": "workflow",
        "repetition": 1,
        "warmup": false,
        "mode": "workflow",
        "parentUsage": {
            "status": "unavailable",
            "reason": "no parent-agent telemetry is exposed to the benchmark runner",
        },
        "fixtureRevision": "seeded-v2",
        "environment": valid_environment(),
        "command": "target/release/cheaper.exe workflow run …",
        "wallTimeMs": 123.4,
        "success": true,
        "exitCode": 0,
        "envelopeOk": true,
        "errorCode": null,
        "directUsageRecordCount": 0,
        "directInvocationTurnsProxy": null,
        "workflowDefinitionBytes": 804,
        "parentVisibleInterfaceBytes": null,
        "interfaceDiscoveryWallTimeMs": null,
        "discoveryMode": "generated-definition",
        "definitionGenerationMs": 0.5,
        "route": null,
        "metrics": {
            "jobCount": 2,
            "usage": {
                "input": 10,
                "output": 5,
                "cachedInput": 0,
                "cachedOutput": 0,
                "total": 15,
                "source": "scripted",
            },
            "repairs": 0,
        },
    })
}

fn valid_comparison() -> Value {
    json!({
        "scenarioId": "single-simple-tool-call",
        "left": "workflow",
        "right": "direct-worker-tool-call",
        "claim": "measured",
        "deltas": {"wallTimeMs": 1.5, "usageInput": 3},
        "reason": null,
        "assumptions": null,
    })
}

fn valid_result() -> Value {
    json!({
        "schemaVersion": 2,
        "runId": "4f3f4db6-4c3c-4b5c-8b7c-9f4a2d3e4f5a",
        "createdAt": "2026-10-25T12:00:00.000Z",
        "fixtureRevision": "seeded-v2",
        "repositoryRevision": "deadbeef",
        "conditions": {
            "warmupCount": 2,
            "repetitionCount": 15,
            "selectedScenarios": ["single-simple-tool-call"],
            "ambientPassthrough": ["CHEAPER_LOG", "CHEAPER_LOG_FORMAT"],
        },
        "measuredBinary": {
            "path": "target/release/cheaper.exe",
            "profile": "release",
            "devSeamProbe": "passed",
        },
        "environment": valid_environment(),
        "pricing": null,
        "observations": [valid_observation()],
        "aggregate": {
            "arms": [],
            "comparisons": [valid_comparison()],
        },
    })
}

fn err_of(result: &Value) -> String {
    record::validate_result(result)
        .unwrap_err()
        .message()
        .to_string()
}

// ---------------------------------------------------------------------------
// invariant 1 — schemaVersion === 2(v1 reject)
// ---------------------------------------------------------------------------

#[test]
fn valid_result_passes() {
    record::validate_result(&valid_result()).expect("valid v2 result must pass");
}

#[test]
fn rejects_v1_schema_version() {
    let mut result = valid_result();
    result["schemaVersion"] = json!(1);
    let e = err_of(&result);
    assert!(e.contains("schemaVersion"), "{e}");
    // 2.0(float 形)も `=== 2` 完全一致ではない。
    let mut result = valid_result();
    result["schemaVersion"] = json!(2.0);
    assert!(err_of(&result).contains("schemaVersion"));
}

// ---------------------------------------------------------------------------
// invariant 2 — parentUsage status enum + status 別必須 key
// ---------------------------------------------------------------------------

#[test]
fn parent_usage_status_enum_and_required_keys() {
    // unavailable + reason 欠落 → reject。
    let mut result = valid_result();
    result["observations"][0]["parentUsage"] = json!({"status": "unavailable"});
    let e = err_of(&result);
    assert!(e.contains("reason"), "{e}");
    // observed + source 欠落 → reject。
    let mut result = valid_result();
    result["observations"][0]["parentUsage"] = json!({"status": "observed"});
    let e = err_of(&result);
    assert!(e.contains("source"), "{e}");
    // enum 外 status → reject。
    let mut result = valid_result();
    result["observations"][0]["parentUsage"] = json!({"status": "guessed"});
    let e = err_of(&result);
    assert!(e.contains("observed|unavailable"), "{e}");
    // parentUsage block 自体の欠落 → reject。
    let mut result = valid_result();
    result["observations"][0]
        .as_object_mut()
        .unwrap()
        .remove("parentUsage");
    assert!(err_of(&result).contains("parentUsage"));
    // observed + source は受理される(将来配線の contract)。
    let mut result = valid_result();
    result["observations"][0]["parentUsage"] =
        json!({"status": "observed", "source": "parent-telemetry"});
    record::validate_result(&result).expect("observed+source is valid");
}

// ---------------------------------------------------------------------------
// invariant 3 — metrics.usage の scripted marker
// ---------------------------------------------------------------------------

#[test]
fn metrics_usage_requires_scripted_source() {
    // source 欠落 → reject。
    let mut result = valid_result();
    result["observations"][0]["metrics"]["usage"]
        .as_object_mut()
        .unwrap()
        .remove("source");
    let e = err_of(&result);
    assert!(e.contains("scripted"), "{e}");
    // 非 scripted 値 → reject。
    let mut result = valid_result();
    result["observations"][0]["metrics"]["usage"]["source"] = json!("measured");
    assert!(err_of(&result).contains("scripted"));
    // usage null(未計測)は許容。
    let mut result = valid_result();
    result["observations"][0]["metrics"]["usage"] = Value::Null;
    record::validate_result(&result).expect("usage:null is valid");
}

// ---------------------------------------------------------------------------
// invariant 4 — environment.node reject + ambientRustc/profile 必須
// ---------------------------------------------------------------------------

#[test]
fn environment_node_is_rejected() {
    // top-level environment。
    let mut result = valid_result();
    result["environment"]["node"] = json!("v22.0.0");
    let e = err_of(&result);
    assert!(e.contains("node"), "{e}");
    // observation の environment も同じ規則。
    let mut result = valid_result();
    result["observations"][0]["environment"]["node"] = json!("v22.0.0");
    assert!(err_of(&result).contains("node"));
}

#[test]
fn ambient_rustc_and_profile_are_required() {
    let mut result = valid_result();
    result["environment"]
        .as_object_mut()
        .unwrap()
        .remove("ambientRustc");
    let e = err_of(&result);
    assert!(e.contains("ambientRustc"), "{e}");
    // null も必須違反(採取不能は emit 前に harness が fail する設計)。
    let mut result = valid_result();
    result["environment"]["ambientRustc"] = Value::Null;
    assert!(err_of(&result).contains("ambientRustc"));
    let mut result = valid_result();
    result["measuredBinary"]
        .as_object_mut()
        .unwrap()
        .remove("profile");
    let e = err_of(&result);
    assert!(e.contains("profile"), "{e}");
    let mut result = valid_result();
    result["measuredBinary"]["profile"] = Value::Null;
    assert!(err_of(&result).contains("profile"));
}

// ---------------------------------------------------------------------------
// invariant 5 — comparisons claim enum + claim 別形状
// ---------------------------------------------------------------------------

fn result_with_comparison(entry: Value) -> Value {
    let mut result = valid_result();
    result["aggregate"]["comparisons"] = json!([entry]);
    result
}

#[test]
fn claim_enum_is_closed() {
    let mut entry = valid_comparison();
    entry["claim"] = json!("incomplete");
    let e = err_of(&result_with_comparison(entry));
    assert!(e.contains("measured|estimated|unavailable"), "{e}");
    // claim 欠落も reject。
    let mut entry = valid_comparison();
    entry.as_object_mut().unwrap().remove("claim");
    assert!(err_of(&result_with_comparison(entry)).contains("claim"));
}

#[test]
fn measured_requires_null_reason_and_assumptions() {
    // key presence は必須(claim 毎に shape が変わる可変 key 構成に
    // しない — §3.4)。
    let mut entry = valid_comparison();
    entry.as_object_mut().unwrap().remove("assumptions");
    let e = err_of(&result_with_comparison(entry));
    assert!(e.contains("reason/assumptions"), "{e}");
    // measured + 非 null reason → mislabel。
    let mut entry = valid_comparison();
    entry["reason"] = json!("because");
    let e = err_of(&result_with_comparison(entry));
    assert!(e.contains("measured"), "{e}");
    // measured + 非 null assumptions → mislabel。
    let mut entry = valid_comparison();
    entry["assumptions"] = json!({"pricing": {}});
    assert!(err_of(&result_with_comparison(entry)).contains("measured"));
}

#[test]
fn unavailable_requires_null_deltas_and_reason() {
    // 正しい unavailable 形(v1 `status:"incomplete"` の fold)。
    let entry = json!({
        "scenarioId": "s", "left": "a", "right": "b",
        "claim": "unavailable",
        "deltas": {"wallTimeMs": null, "usageInput": null},
        "reason": "one or both comparison arms are absent",
        "assumptions": null,
    });
    record::validate_result(&result_with_comparison(entry))
        .expect("canonical unavailable shape is valid");
    // deltas に数値 → reject(unavailable 上の数値主張を置かない)。
    let mut entry = valid_comparison();
    entry["claim"] = json!("unavailable");
    entry["reason"] = json!("one or both comparison arms are absent");
    let e = err_of(&result_with_comparison(entry));
    assert!(e.contains("must be null"), "{e}");
    // reason 欠落/null → reject。
    let mut entry = valid_comparison();
    entry["claim"] = json!("unavailable");
    entry["deltas"] = json!({"wallTimeMs": null});
    let e = err_of(&result_with_comparison(entry));
    assert!(e.contains("reason"), "{e}");
}

#[test]
fn estimated_requires_assumptions_record() {
    let assumptions = json!({
        "pricing": {
            "model": "fake-model",
            "catalogRevision": "2026-10-01",
            "source": "bundled-snapshot",
            "inputUsdPerMTok": 1.0,
            "outputUsdPerMTok": 2.0,
            "cachedInputUsdPerMTok": 0.5,
        },
        "tokenizer": "o200k_base",
        "boundary": "scripted-usage",
    });
    // 正しい estimated 形。
    let entry = json!({
        "scenarioId": "s", "left": "a", "right": "b",
        "claim": "estimated",
        "deltas": {"estimatedCostUsd": 0.001},
        "reason": null,
        "assumptions": assumptions,
    });
    record::validate_result(&result_with_comparison(entry))
        .expect("canonical estimated shape is valid");
    // assumptions null → reject。
    let mut entry = valid_comparison();
    entry["claim"] = json!("estimated");
    let e = err_of(&result_with_comparison(entry));
    assert!(e.contains("assumptions"), "{e}");
    // tokenizer/boundary 不一致 → reject。
    for (key, bad) in [
        ("tokenizer", json!("cl100k")),
        ("boundary", json!("real-billing")),
    ] {
        let mut wrong = assumptions.clone();
        wrong[key] = bad;
        let entry = json!({
            "scenarioId": "s", "left": "a", "right": "b",
            "claim": "estimated", "deltas": {"estimatedCostUsd": 1.0},
            "reason": null, "assumptions": wrong,
        });
        let e = err_of(&result_with_comparison(entry));
        assert!(e.contains(key), "{key}: {e}");
    }
    // pricing.model 欠落 → reject。
    let mut wrong = assumptions.clone();
    wrong["pricing"].as_object_mut().unwrap().remove("model");
    let entry = json!({
        "scenarioId": "s", "left": "a", "right": "b",
        "claim": "estimated", "deltas": {"estimatedCostUsd": 1.0},
        "reason": null, "assumptions": wrong,
    });
    let e = err_of(&result_with_comparison(entry));
    assert!(e.contains("model"), "{e}");
}

// ---------------------------------------------------------------------------
// invariant 6 — parentUsage block 以外の数値 parent-usage field 不存在
// ---------------------------------------------------------------------------

#[test]
fn numeric_parent_usage_fields_are_rejected() {
    // observation top-level。
    let mut result = valid_result();
    result["observations"][0]["parentTokensSaved"] = json!(1200);
    let e = err_of(&result);
    assert!(e.contains("parent-usage"), "{e}");
    // metrics 内への潜ませも reject。
    let mut result = valid_result();
    result["observations"][0]["metrics"]["parentCostUsd"] = json!(0.5);
    let e = err_of(&result);
    assert!(e.contains("parent-usage"), "{e}");
    // result top-level も対象。
    let mut result = valid_result();
    result["parentUsageTokens"] = json!(42);
    let e = err_of(&result);
    assert!(e.contains("parent-usage"), "{e}");
    // 宣言語彙は誤検しない(parentVisible*・usageInput・estimatedCostUsd)。
    record::validate_result(&valid_result()).expect("declared vocabulary must not trip");
}

// ---------------------------------------------------------------------------
// observation record 投影(record.rs × measure.rs 接続)
// ---------------------------------------------------------------------------

fn test_scenario() -> Scenario {
    serde_json::from_value(json!({
        "id": "s", "title": "t", "arms": ["workflow"]
    }))
    .unwrap()
}

fn run_context() -> record::RunContext {
    record::RunContext {
        run_id: "run-1".to_string(),
        fixture_revision: "seeded-v2".to_string(),
        environment: valid_environment().as_object().unwrap().clone(),
    }
}

#[test]
fn observation_record_projects_run_level_fields() {
    let mut obs = Observation::new(&test_scenario(), "workflow", 3, false, Mode::Workflow, "x");
    obs.wall_time_ms = 12.5;
    obs.exit_code = Some(0);
    obs.envelope_ok = Some(true);
    let rec = record::observation_record(&obs, &run_context());
    assert_eq!(rec["schemaVersion"], json!(2));
    assert_eq!(rec["runId"], json!("run-1"));
    assert_eq!(rec["fixtureRevision"], json!("seeded-v2"));
    assert_eq!(rec["parentUsage"]["status"], json!("unavailable"));
    assert!(rec["parentUsage"]["reason"].is_string());
    assert_eq!(
        rec["environment"]["ambientRustc"],
        json!("rustc 1.99.0 (000000000 2026-10-25)")
    );
    assert!(rec["environment"].get("node").is_none());
    // v1 mode 名を継承(direct-worker-tool-call arm の mode は
    // `single-direct-worker-with-one-tool-call` — rename しない)。
    let obs = Observation::new(
        &test_scenario(),
        "direct-worker-tool-call",
        1,
        false,
        Mode::SingleDirectWorkerWithOneToolCall,
        "x",
    );
    let rec = record::observation_record(&obs, &run_context());
    assert_eq!(
        rec["mode"],
        json!("single-direct-worker-with-one-tool-call")
    );
}

#[test]
fn capture_environment_has_no_node_key_and_required_fields() {
    // ambient `rustc --version` は cargo test 実行環境では常に
    // 利用可能(cargo が rustc を spawn する PATH 上にある)。
    let env = record::capture_environment().expect("capture must succeed under cargo");
    assert!(env.get("node").is_none());
    for key in [
        "os",
        "osRelease",
        "arch",
        "cpuCount",
        "ambientRustc",
        "harnessProfile",
    ] {
        assert!(env.contains_key(key), "missing {key}");
    }
    assert!(env["ambientRustc"].as_str().unwrap().starts_with("rustc"));
    assert!(["debug", "release"].contains(&env["harnessProfile"].as_str().unwrap()));
}

// ---------------------------------------------------------------------------
// armExpect 照合(§2.3/§3.3 success 意味論 — measure.rs 経由)
// ---------------------------------------------------------------------------

#[test]
fn arm_expect_equality_covers_value_variants() {
    let mut obs = Observation::new(&test_scenario(), "workflow", 1, false, Mode::Workflow, "x");
    obs.exit_code = Some(0);
    obs.envelope_ok = Some(true);
    obs.wall_time_ms = 5.0;
    // 数値・bool・文字列・null の等値比較 — 全一致。
    let expect = json!({
        "exitCode": 0, "envelopeOk": true,
        "errorCode": null, "metrics": {"jobCount": 2},
    });
    obs.metrics.insert("jobCount".to_string(), json!(2));
    assert!(measure::evaluate_expect(&expect, &obs).is_empty());
    // 型違いの等値(0 vs 0.0)— `as_f64` 比較ではなく JSON 等値。
    // v1 の `===` parity として数値どうしなら一致する(serde_json
    // Number は整数/浮動小数を区別しない比較ではない — expect 側は
    // observation 投影と同じ JSON 表記を宣言する前提)。
    let expect = json!({"envelopeOk": false});
    assert_eq!(measure::evaluate_expect(&expect, &obs).len(), 1);
}

#[test]
fn fallback_success_semantics() {
    // `armExpect` 未宣言の補助 arm — exitCode==0(envelope arm は
    // envelopeOk===true が加わる)。
    assert!(measure::fallback_success(
        Mode::Workflow,
        Some(0),
        Some(true)
    ));
    assert!(!measure::fallback_success(
        Mode::Workflow,
        Some(0),
        Some(false)
    ));
    assert!(!measure::fallback_success(
        Mode::Workflow,
        Some(1),
        Some(true)
    ));
    // 非 envelope arm — envelopeOk は無関係。
    assert!(measure::fallback_success(Mode::RouteRun, Some(0), None));
    assert!(!measure::fallback_success(
        Mode::DirectChildExec,
        Some(7),
        None
    ));
}

// ---------------------------------------------------------------------------
// §3.5 store — create_new 非上書き・v1 load reject
// ---------------------------------------------------------------------------

#[test]
fn write_result_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("result.json");
    report::write_result(&path, &valid_result()).expect("first write");
    // 同 path への二度目は拒否(create_new — v1 artifact 保護と同じ
    // 非上書き契約)。
    let e = report::write_result(&path, &valid_result()).unwrap_err();
    assert!(e.message().contains("immutable"), "{}", e.message());
    // 既存 v1 file と同じ内容でも拒否される(byte-for-byte 不変を確認)。
    let before = std::fs::read(&path).unwrap();
    assert!(report::write_result(&path, &json!({"schemaVersion": 1})).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn load_result_rejects_v1_and_accepts_v2() {
    let dir = tempfile::tempdir().unwrap();
    // v1 file — load は schemaVersion で reject(混在 run を作らない)。
    let v1 = dir.path().join("v1.json");
    std::fs::write(&v1, r#"{"schemaVersion":1,"observations":[]}"#).unwrap();
    let e = report::load_result(&v1).unwrap_err();
    assert!(e.message().contains("schemaVersion"), "{}", e.message());
    // v1 file は改変しない(immutable artifact)。
    assert_eq!(
        std::fs::read_to_string(&v1).unwrap(),
        r#"{"schemaVersion":1,"observations":[]}"#
    );
    // v2 file — validate を通して load できる。
    let v2 = dir.path().join("v2.json");
    report::write_result(&v2, &valid_result()).unwrap();
    let loaded = report::load_result(&v2).expect("v2 result must load");
    assert_eq!(loaded["schemaVersion"], json!(2));
}

// ---------------------------------------------------------------------------
// end-to-end — observation → record → aggregate → validate → write
// ---------------------------------------------------------------------------

#[test]
fn emitted_result_passes_validation() {
    let scenario: Scenario = serde_json::from_value(json!({
        "id": "s",
        "title": "t",
        "arms": ["workflow"],
        "comparisons": [{"left": "workflow", "right": "ghost-arm"}],
    }))
    .unwrap();
    let run = run_context();
    let mut obs = Observation::new(&scenario, "workflow", 1, false, Mode::Workflow, "x");
    obs.wall_time_ms = 10.0;
    obs.exit_code = Some(0);
    obs.envelope_ok = Some(true);
    obs.success = true;
    let records = vec![record::observation_record(&obs, &run)];
    let aggregated = aggregate::aggregate(&records, &[scenario], None).unwrap();
    let result = report::assemble_result(
        &report::RunMeta {
            run_id: &run.run_id,
            created_at: "2026-10-25T12:00:00.000Z",
            fixture_revision: &run.fixture_revision,
            repository_revision: None,
            conditions: Map::new(),
            measured_binary: record::measured_binary_block(
                std::path::Path::new("target/release/cheaper.exe"),
                std::path::Path::new("."),
                "passed",
            ),
            environment: run.environment.clone(),
            pricing: None,
        },
        records,
        aggregated,
    );
    record::validate_result(&result).expect("emitted result must satisfy all invariants");
    // 片側 arm 欠落の comparison は claim:unavailable + reason へ fold
    // される(v1 status:incomplete の後続形)。
    let entry = &result["aggregate"]["comparisons"][0];
    assert_eq!(entry["claim"], json!("unavailable"));
    assert_eq!(
        entry["reason"],
        json!("one or both comparison arms are absent")
    );
    assert!(entry["assumptions"].is_null());
    assert!(entry["deltas"]
        .as_object()
        .unwrap()
        .values()
        .all(Value::is_null));
}
