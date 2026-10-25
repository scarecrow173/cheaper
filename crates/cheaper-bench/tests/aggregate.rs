//! aggregate 純粋関数の boundary case test(design §4 — v1
//! `benchmarks/lib/aggregate.mjs` parity の pin)。
//!
//! - `median`: 昇順 sort → 奇数 `sorted[floor(n/2)]`・偶数は
//!   `sorted[n/2-1]` と `sorted[n/2]` の平均
//! - `p95`(`percentile(xs,0.95)`): nearest-rank
//!   `sorted[max(0,ceil(n*f)-1)]` — 補間しない
//! - `stableValue`: 全 obs 全同一なら値、変動・欠測・非数混在なら
//!   null(bool/string/null dim は JSON 等値への拡張 — §4.1)
//! - `successRate`・warmup 除外・manifest comparisons 駆動・
//!   reduction・estimated layer(`--pricing` fail-fast)の emit 規則

use cheaper_bench::aggregate::{self, Pricing};
use cheaper_bench::manifest::Scenario;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// median / percentile(v1 parity algorithm)
// ---------------------------------------------------------------------------

#[test]
fn median_boundary_cases() {
    // 単一要素。
    assert_eq!(aggregate::median(&[7.5]), Some(7.5));
    // 奇数 — 昇順 sort 後の `sorted[floor(n/2)]`。
    assert_eq!(aggregate::median(&[3.0, 1.0, 2.0]), Some(2.0));
    assert_eq!(aggregate::median(&[5.0, 1.0, 9.0, 3.0, 7.0]), Some(5.0));
    // 偶数 — `sorted[n/2-1]` と `sorted[n/2]` の平均。
    assert_eq!(aggregate::median(&[1.0, 3.0]), Some(2.0));
    assert_eq!(aggregate::median(&[4.0, 1.0, 10.0, 2.0]), Some(3.0));
    // 空列 → None。
    assert_eq!(aggregate::median(&[]), None);
}

#[test]
fn p95_nearest_rank_no_interpolation() {
    // 単一要素 → その要素。
    assert_eq!(aggregate::p95(&[42.0]), Some(42.0));
    // n=2 → ceil(1.9)=2 → sorted[1](補間せず最大値)。
    assert_eq!(aggregate::p95(&[100.0, 1.0]), Some(100.0));
    // n=20 → ceil(19)=19 → sorted[18](19 番目の実測値)。
    let xs: Vec<f64> = (1..=20).map(|i| i as f64).collect();
    assert_eq!(aggregate::p95(&xs), Some(19.0));
    // n=21 → ceil(19.95)=20 → sorted[19](補間しない — 20.15 等の
    // 補間値ではなく実測値そのもの)。
    let xs: Vec<f64> = (1..=21).map(|i| i as f64).collect();
    assert_eq!(aggregate::p95(&xs), Some(20.0));
    // n=100 → ceil(95)=95 → sorted[94]。
    let xs: Vec<f64> = (1..=100).map(|i| i as f64).collect();
    assert_eq!(aggregate::p95(&xs), Some(95.0));
    // 未整列入力でも昇順 sort 後の nearest-rank。
    assert_eq!(aggregate::p95(&[9.0, 1.0, 5.0]), Some(9.0));
    assert_eq!(aggregate::p95(&[]), None);
}

// ---------------------------------------------------------------------------
// stableValue(全同一規則 + null/非数 semantics)
// ---------------------------------------------------------------------------

#[test]
fn stable_value_boundary_cases() {
    // 全同一数値 → その値。
    assert_eq!(aggregate::stable_value(&[json!(4), json!(4)]), json!(4));
    // 整数/浮動小数の表記差は `as_f64` 等値(v1 JS number parity)。
    assert_eq!(aggregate::stable_value(&[json!(4), json!(4.0)]), json!(4));
    // 変動 → null。
    assert_eq!(aggregate::stable_value(&[json!(1), json!(2)]), Value::Null);
    // 欠測(Null)混在 → null(全 null は「全同一 null」— §4.1 の
    // 決定値として null を返す。emit 上は区別が付かないのが意図)。
    assert_eq!(
        aggregate::stable_value(&[json!(1), Value::Null]),
        Value::Null
    );
    assert_eq!(
        aggregate::stable_value(&[Value::Null, Value::Null]),
        Value::Null
    );
    // 非数混在 → null。全同一 bool/string/null dim(errorCode・
    // condenseFired・runId 等)は JSON 等値の決定値になる(§4.1)。
    assert_eq!(
        aggregate::stable_value(&[json!("TIMEOUT"), json!("TIMEOUT")]),
        json!("TIMEOUT")
    );
    assert_eq!(
        aggregate::stable_value(&[json!("TIMEOUT"), json!("OTHER")]),
        Value::Null
    );
    assert_eq!(
        aggregate::stable_value(&[json!(true), json!(true)]),
        json!(true)
    );
    assert_eq!(
        aggregate::stable_value(&[json!("1"), json!(1)]),
        Value::Null
    );
    // 空列 → null。
    assert_eq!(aggregate::stable_value(&[]), Value::Null);
}

// ---------------------------------------------------------------------------
// arm 集計(warmup 除外・successRate・parentUsage 集約・stableMetrics)
// ---------------------------------------------------------------------------

fn obs(scenario: &str, arm: &str, wall: f64, success: bool, metrics: Value) -> Value {
    json!({
        "schemaVersion": 2,
        "runId": "r",
        "scenarioId": scenario,
        "scenarioTitle": "t",
        "arm": arm,
        "repetition": 1,
        "warmup": false,
        "mode": "workflow",
        "parentUsage": {
            "status": "unavailable",
            "reason": "no parent-agent telemetry is exposed to the benchmark runner",
        },
        "fixtureRevision": "seeded-v2",
        "command": "c",
        "wallTimeMs": wall,
        "success": success,
        "exitCode": 0,
        "envelopeOk": true,
        "errorCode": null,
        "directUsageRecordCount": 0,
        "metrics": metrics,
    })
}

fn scenario(id: &str, arms: &[&str], comparisons: Value) -> Scenario {
    serde_json::from_value(json!({
        "id": id,
        "title": format!("scenario {id}"),
        "arms": arms,
        "comparisons": comparisons,
    }))
    .unwrap()
}

fn usage(input: f64, output: f64, cached_input: f64) -> Value {
    json!({
        "input": input,
        "output": output,
        "cachedInput": cached_input,
        "cachedOutput": 0,
        "total": input + output + cached_input,
        "source": "scripted",
    })
}

#[test]
fn arm_grouping_excludes_warmup_and_computes_fields() {
    let records = vec![
        // warmup obs は集計除外 — wall-time の median/p95 母数にも
        // 入らない。
        {
            let mut warm = obs("s", "a", 999.0, false, json!({"jobCount": 99}));
            warm["warmup"] = json!(true);
            warm
        },
        obs(
            "s",
            "a",
            10.0,
            true,
            json!({"jobCount": 2, "usage": usage(10.0, 5.0, 0.0)}),
        ),
        obs(
            "s",
            "a",
            20.0,
            false,
            json!({"jobCount": 2, "usage": usage(10.0, 5.0, 0.0)}),
        ),
        obs(
            "s",
            "a",
            30.0,
            true,
            json!({"jobCount": 2, "usage": usage(10.0, 5.0, 0.0)}),
        ),
    ];
    let scenarios = vec![scenario("s", &["a"], json!([]))];
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    let arm = &agg["arms"][0];
    assert_eq!(arm["observationCount"], json!(3));
    assert_eq!(arm["medianWallTimeMs"], json!(20.0));
    // n=3 → ceil(2.85)=3 → sorted[2] = 30.0。
    assert_eq!(arm["p95WallTimeMs"], json!(30.0));
    // successRate = success===true の obs / 全 obs。
    assert_eq!(arm["successRate"], json!(2.0 / 3.0));
    // stableMetrics — 全 obs 同一値は値、変動・欠測は null。
    assert_eq!(arm["stableMetrics"]["jobCount"], json!(2));
    assert_eq!(arm["stableMetrics"]["usageInput"], json!(10.0));
    assert_eq!(arm["stableMetrics"]["exitCode"], json!(0));
    assert!(arm["stableMetrics"]["errorCode"].is_null());
    // parentUsage 集約 — 全 obs unavailable → unavailable + reason。
    assert_eq!(arm["parentUsage"]["status"], json!("unavailable"));
    assert_eq!(
        arm["parentUsage"]["reason"],
        json!("no parent-agent telemetry is exposed to the benchmark runner")
    );
}

#[test]
fn arm_grouping_route_and_observed_parent_usage() {
    let mut observed = obs("s", "a", 1.0, true, json!({}));
    observed["parentUsage"] = json!({"status": "observed", "source": "telemetry-a"});
    let mut observed2 = obs("s", "a", 1.0, true, json!({}));
    observed2["parentUsage"] = json!({"status": "observed", "source": "telemetry-b"});
    let mut route_obs = obs("s", "route-run", 5.0, true, json!({"logRefetches": 1}));
    route_obs["route"] = json!({
        "condenseFired": true,
        "childOutputBytes": 4096,
        "runId": "rep-specific-id",
    });
    let records = vec![observed, observed2, route_obs];
    let scenarios = vec![scenario("s", &["a", "route-run"], json!([]))];
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    // observed 混在 → sources 重複排除。
    let arm_a = &agg["arms"][0];
    assert_eq!(arm_a["parentUsage"]["status"], json!("observed"));
    assert_eq!(
        arm_a["parentUsage"]["sources"],
        json!(["telemetry-a", "telemetry-b"])
    );
    // route dims の stableValue(route.* からの参照 — §4.2)。
    let arm_route = &agg["arms"][1];
    assert_eq!(arm_route["stableMetrics"]["condenseFired"], json!(true));
    assert_eq!(arm_route["stableMetrics"]["childOutputBytes"], json!(4096));
    // runId は単一 obs で全同一扱いになる(reps では実質 null —
    // 形式 pin は armExpect 側)。
    assert_eq!(arm_route["stableMetrics"]["logRefetches"], json!(1));
}

// ---------------------------------------------------------------------------
// comparisons(manifest 宣言駆動 — measured/unavailable)
// ---------------------------------------------------------------------------

#[test]
fn comparison_absent_arm_folds_to_unavailable() {
    let records = vec![obs("s", "a", 10.0, true, json!({}))];
    let scenarios = vec![scenario(
        "s",
        &["a"],
        json!([{"left": "a", "right": "ghost"}]),
    )];
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    let entry = &agg["comparisons"][0];
    // v1 `status:"incomplete"` → claim:"unavailable" + reason へ fold。
    assert_eq!(entry["claim"], json!("unavailable"));
    assert_eq!(
        entry["reason"],
        json!("one or both comparison arms are absent")
    );
    assert!(entry["assumptions"].is_null());
    // deltas 全値 null — 数値主張を置かない。
    let deltas = entry["deltas"].as_object().unwrap();
    assert!(!deltas.is_empty());
    assert!(deltas.values().all(Value::is_null));
    // reason/assumptions key は常に emit(fixed presence)。
    assert!(entry.as_object().unwrap().contains_key("reason"));
    assert!(entry.as_object().unwrap().contains_key("assumptions"));
}

#[test]
fn comparison_measured_deltas_cover_median_and_p95() {
    let metrics_l = json!({"jobCount": 2, "retainedBytes": 100, "usage": usage(10.0, 5.0, 0.0)});
    let metrics_r = json!({"jobCount": 1, "retainedBytes": 40, "usage": usage(4.0, 2.0, 0.0)});
    let records = vec![
        obs("s", "left", 10.0, true, metrics_l.clone()),
        obs("s", "left", 30.0, true, metrics_l.clone()),
        obs("s", "right", 5.0, true, metrics_r.clone()),
        obs("s", "right", 15.0, true, metrics_r.clone()),
    ];
    let scenarios = vec![scenario(
        "s",
        &["left", "right"],
        json!([{"left": "left", "right": "right"}]),
    )];
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    let entry = &agg["comparisons"][0];
    assert_eq!(entry["claim"], json!("measured"));
    assert!(entry["reason"].is_null());
    assert!(entry["assumptions"].is_null());
    let deltas = &entry["deltas"];
    // wallTimeMs delta は median と p95 の双方(§4.1 — v1 は median
    // のみだった拡張点)。
    assert_eq!(deltas["wallTimeMs"], json!(20.0 - 10.0));
    assert_eq!(deltas["p95WallTimeMs"], json!(30.0 - 15.0));
    // stable dims の delta。
    assert_eq!(deltas["jobCount"], json!(1.0));
    assert_eq!(deltas["retainedBytes"], json!(60.0));
    assert_eq!(deltas["usageInput"], json!(6.0));
    // 一方が null(欠測/変動)なら delta も null。
    assert!(deltas["materializedPromptBytes"].is_null());
}

#[test]
fn reduction_emits_bytes_and_ratio() {
    // `definitionReadBytes` は metrics.*、`parentVisibleInterfaceBytes`
    // は record top-level(§3.3 の field 配置 — dim 参照規則と一致)。
    let mut right = obs("s", "iface", 1.0, true, json!({}));
    right["parentVisibleInterfaceBytes"] = json!(200);
    let records = vec![
        obs("s", "full", 1.0, true, json!({"definitionReadBytes": 800})),
        right,
    ];
    let scenarios = vec![scenario(
        "s",
        &["full", "iface"],
        json!([{
            "left": "full",
            "right": "iface",
            "reduction": {"left": "definitionReadBytes", "right": "parentVisibleInterfaceBytes"},
        }]),
    )];
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    let reduction = &agg["comparisons"][0]["reduction"];
    // reductionBytes = leftStable − rightStable / ratio = (l−r)/l。
    assert_eq!(reduction["reductionBytes"], json!(600.0));
    assert_eq!(reduction["reductionRatio"], json!(0.75));
    assert_eq!(reduction["left"], json!("definitionReadBytes"));
    assert_eq!(reduction["right"], json!("parentVisibleInterfaceBytes"));
}

#[test]
fn reduction_null_when_side_missing_or_left_zero() {
    // right 側の dim が欠測(全 obs null)→ 両値 null。
    let records = vec![
        obs("s", "full", 1.0, true, json!({"definitionReadBytes": 800})),
        obs("s", "iface", 1.0, true, json!({})),
    ];
    let scenarios = vec![scenario(
        "s",
        &["full", "iface"],
        json!([{
            "left": "full", "right": "iface",
            "reduction": {"left": "definitionReadBytes", "right": "parentVisibleInterfaceBytes"},
        }]),
    )];
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    let reduction = &agg["comparisons"][0]["reduction"];
    assert!(reduction["reductionBytes"].is_null());
    assert!(reduction["reductionRatio"].is_null());
    // left==0 → ratio は null(bytes は出る)。
    let mut right = obs("s", "b", 1.0, true, json!({}));
    right["parentVisibleInterfaceBytes"] = json!(5);
    let records = vec![
        obs("s", "a", 1.0, true, json!({"definitionReadBytes": 0})),
        right,
    ];
    let scenarios = vec![scenario(
        "s",
        &["a", "b"],
        json!([{
            "left": "a", "right": "b",
            "reduction": {"left": "definitionReadBytes", "right": "parentVisibleInterfaceBytes"},
        }]),
    )];
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    let reduction = &agg["comparisons"][0]["reduction"];
    assert_eq!(reduction["reductionBytes"], json!(-5.0));
    assert!(reduction["reductionRatio"].is_null());
}

// ---------------------------------------------------------------------------
// estimated layer(§4.3 — `--pricing` 有無の emit 差異 + fail-fast load)
// ---------------------------------------------------------------------------

#[test]
fn estimated_layer_emits_only_with_pricing() {
    let records = vec![
        obs(
            "s",
            "a",
            10.0,
            true,
            json!({"usage": usage(10.0, 5.0, 4.0)}),
        ),
        obs("s", "b", 8.0, true, json!({"usage": usage(4.0, 2.0, 0.0)})),
    ];
    let scenarios = vec![scenario(
        "s",
        &["a", "b"],
        json!([{"left": "a", "right": "b"}]),
    )];
    // `--pricing` 無し — estimated entry を一切 emit しない。
    let agg = aggregate::aggregate(&records, &scenarios, None).unwrap();
    assert_eq!(agg["comparisons"].as_array().unwrap().len(), 1);
    assert_eq!(agg["comparisons"][0]["claim"], json!("measured"));
    // `--pricing` 有り — measured entry の sibling として estimated。
    let pricing = Pricing::parse(
        r#"{"model":"fake-model","catalogRevision":"2026-10-01","source":"bundled",
            "inputUsdPerMTok":1.0,"outputUsdPerMTok":2.0,"cachedInputUsdPerMTok":0.5}"#,
    )
    .unwrap();
    let agg = aggregate::aggregate(&records, &scenarios, Some(&pricing)).unwrap();
    let entries = agg["comparisons"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    let estimated = &entries[1];
    assert_eq!(estimated["claim"], json!("estimated"));
    assert!(estimated["reason"].is_null());
    // assumptions echo — model・catalogRevision を必ず含む。
    assert_eq!(estimated["assumptions"]["tokenizer"], json!("o200k_base"));
    assert_eq!(
        estimated["assumptions"]["boundary"],
        json!("scripted-usage")
    );
    assert_eq!(
        estimated["assumptions"]["pricing"]["model"],
        json!("fake-model")
    );
    assert_eq!(
        estimated["assumptions"]["pricing"]["catalogRevision"],
        json!("2026-10-01")
    );
    // cost = (input×1 + output×2 + cached×0.5)/1e6。
    // left=(10+10+2)/1e6=22e-6・right=(4+4+0)/1e6=8e-6 → delta=14e-6。
    let delta = estimated["deltas"]["estimatedCostUsd"].as_f64().unwrap();
    assert!((delta - 14e-6).abs() < 1e-12, "delta={delta}");
}

#[test]
fn estimated_layer_missing_stable_usage_yields_null_cost() {
    // usage が欠測(未計測)の arm — 推計不能は null で表す
    // (assumptions 無しの unavailable fold ではない — 推計を試みた
    // fact として claim:"estimated" + null delta)。
    let records = vec![
        obs(
            "s",
            "a",
            10.0,
            true,
            json!({"usage": usage(10.0, 5.0, 0.0)}),
        ),
        obs("s", "b", 8.0, true, json!({})),
    ];
    let scenarios = vec![scenario(
        "s",
        &["a", "b"],
        json!([{"left": "a", "right": "b"}]),
    )];
    let pricing =
        Pricing::parse(r#"{"model":"m","source":"s","inputUsdPerMTok":1,"outputUsdPerMTok":1}"#)
            .unwrap();
    let agg = aggregate::aggregate(&records, &scenarios, Some(&pricing)).unwrap();
    let estimated = &agg["comparisons"][1];
    assert_eq!(estimated["claim"], json!("estimated"));
    assert!(estimated["deltas"]["estimatedCostUsd"].is_null());
}

#[test]
fn pricing_file_load_is_fail_fast() {
    // malformed JSON。
    assert!(Pricing::parse("not json").is_err());
    // 必須 key 欠落。
    for text in [
        r#"{"source":"s","inputUsdPerMTok":1,"outputUsdPerMTok":1}"#, // model
        r#"{"model":"m","inputUsdPerMTok":1,"outputUsdPerMTok":1}"#,  // source
        r#"{"model":"m","source":"s","outputUsdPerMTok":1}"#,         // input
        r#"{"model":"m","source":"s","inputUsdPerMTok":1}"#,          // output
    ] {
        assert!(Pricing::parse(text).is_err(), "{text}");
    }
    // 非数値・負数。
    assert!(Pricing::parse(
        r#"{"model":"m","source":"s","inputUsdPerMTok":"1","outputUsdPerMTok":1}"#
    )
    .is_err());
    assert!(Pricing::parse(
        r#"{"model":"m","source":"s","inputUsdPerMTok":-1,"outputUsdPerMTok":1}"#
    )
    .is_err());
    // 未知 key。
    assert!(Pricing::parse(
        r#"{"model":"m","source":"s","inputUsdPerMTok":1,"outputUsdPerMTok":1,"bogus":1}"#
    )
    .is_err());
    // catalogRevision/cachedInputUsdPerMTok は optional。
    let p =
        Pricing::parse(r#"{"model":"m","source":"s","inputUsdPerMTok":1,"outputUsdPerMTok":2}"#)
            .unwrap();
    assert!(p.catalog_revision.is_none());
    assert!(p.cached_input_usd_per_mtok.is_none());
    // load path — file の read/parse も同規則。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pricing.json");
    std::fs::write(
        &path,
        r#"{"model":"m","source":"s","inputUsdPerMTok":1,"outputUsdPerMTok":2}"#,
    )
    .unwrap();
    assert!(Pricing::load(&path).is_ok());
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, r#"{"model":"m"}"#).unwrap();
    assert!(Pricing::load(&bad).is_err());
}
