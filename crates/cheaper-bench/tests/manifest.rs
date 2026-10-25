//! manifest v2 fail-fast validation の contract test(design §2.6)。
//!
//! `Manifest::parse` が enforce する invariant を violation 単位で検証する
//! — missing/unexpected scenario・armBase 連鎖・route `*`/self-match/
//! argv 不一致・未知 policy section・comparisons/armExpect の不正参照。
//! exact coverage 前提のため、全 test は「12 required scenario を揃えた
//! 最小 base」を組み立ててから 1 箇所だけ壊す方式を取る。

use cheaper_bench::manifest::{Manifest, REQUIRED_SCENARIO_IDS};
use serde_json::{json, Value};

/// 12 required scenario を全て含む最小 manifest — 各 scenario は
/// 宣言内容の最小形(workflow 系 arm を持たない・route 無しの安全形)。
/// per-scenario の上書きは `patch` 側が担う。
fn base_manifest() -> Value {
    let scenarios: Vec<Value> = REQUIRED_SCENARIO_IDS
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "title": format!("scenario {id}"),
                "arms": ["direct-filesystem"],
                "manual": {"path": "seed.txt"},
                "files": {"seed.txt": "seed\n"},
            })
        })
        .collect();
    json!({
        "schemaVersion": 2,
        "fixtureRevision": "seeded-v2",
        "scenarios": scenarios,
    })
}

/// `id` の scenario object へ patch(同 key 上書き merge)を適用した
/// manifest 文字列を返す。
fn manifest_with(scenario_id: &str, patch: Value) -> String {
    let mut manifest = base_manifest();
    let patch_map = patch.as_object().expect("patch is an object");
    for scenario in manifest["scenarios"].as_array_mut().unwrap() {
        if scenario["id"].as_str() == Some(scenario_id) {
            let target = scenario.as_object_mut().unwrap();
            for (key, value) in patch_map {
                target.insert(key.clone(), value.clone());
            }
        }
    }
    serde_json::to_string(&manifest).unwrap()
}

/// root 構造を直接書き換える variant(scenarios 自体の改変用)。
fn manifest_raw(manifest: Value) -> String {
    serde_json::to_string(&manifest).unwrap()
}

fn err_of(text: &str) -> String {
    Manifest::parse(text).unwrap_err().message().to_string()
}

#[test]
fn base_manifest_is_valid() {
    let text = manifest_raw(base_manifest());
    Manifest::parse(&text).unwrap_or_else(|e| panic!("base manifest should be valid: {e}"));
}

#[test]
fn rejects_schema_version_1() {
    let e = Manifest::parse(r#"{"schemaVersion":1,"fixtureRevision":"seeded-v1","scenarios":[]}"#)
        .unwrap_err();
    assert!(e.message().contains("schemaVersion"));
}

#[test]
fn rejects_missing_required_scenario() {
    let mut manifest = base_manifest();
    manifest["scenarios"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["id"].as_str() != Some("route-routed-exec"));
    let e = err_of(&manifest_raw(manifest));
    assert!(e.contains("missing required scenarios"), "{e}");
    assert!(e.contains("route-routed-exec"), "{e}");
}

#[test]
fn rejects_unexpected_scenario() {
    let mut manifest = base_manifest();
    manifest["scenarios"].as_array_mut().unwrap().push(json!({
        "id": "unexpected-extra",
        "title": "t",
        "arms": ["direct-filesystem"],
    }));
    let e = err_of(&manifest_raw(manifest));
    assert!(e.contains("unexpected scenarios"), "{e}");
    assert!(e.contains("unexpected-extra"), "{e}");
}

#[test]
fn rejects_duplicate_scenario_id() {
    let mut manifest = base_manifest();
    let dup = manifest["scenarios"][0].clone();
    manifest["scenarios"].as_array_mut().unwrap().push(dup);
    let e = err_of(&manifest_raw(manifest));
    assert!(e.contains("duplicate scenario id"), "{e}");
}

#[test]
fn rejects_empty_title_and_arms() {
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({"title": "  "}),
    ));
    assert!(e.contains("title must not be empty"), "{e}");
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({"arms": []}),
    ));
    assert!(e.contains("arms must not be empty"), "{e}");
}

#[test]
fn rejects_undeclared_arm_keys() {
    for (field, value) in [
        ("armBase", json!({"ghost": "workflow"})),
        ("armEnv", json!({"ghost": {"X": "1"}})),
        ("armExpect", json!({"ghost": {"envelopeOk": true}})),
        ("armFakeStream", json!({"ghost": "auto"})),
    ] {
        let e = err_of(&manifest_with(
            "single-simple-tool-call",
            json!({field: value}),
        ));
        assert!(e.contains("not a declared arm"), "{field}: {e}");
    }
}

#[test]
fn rejects_arm_base_alias_chain() {
    // `a → b` で `b` も宣言 arm(alias 連鎖 — `b` の armBase は
    // 再度解決されないため `a` の resolved は impl 語彙外)。
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({
            "arms": ["a", "b", "direct-filesystem"],
            "armBase": {"a": "b", "b": "workflow"},
            "workflow": {"jobs": {"j": {"kind": "tool", "tool": "read", "input": {"path": "seed.txt"}}}},
        }),
    ));
    assert!(e.contains("resolves to unknown impl"), "{e}");
}

#[test]
fn rejects_unknown_impl_resolution() {
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({
            "arms": ["odd-arm"],
            "armBase": {"odd-arm": "no-such-impl"},
        }),
    ));
    assert!(
        e.contains("resolves to neither") || e.contains("unknown impl"),
        "{e}"
    );
}

#[test]
fn workflow_arm_requires_nonempty_jobs() {
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({
            "arms": ["workflow"],
            "workflow": {"jobs": {}},
        }),
    ));
    assert!(e.contains("workflow.jobs"), "{e}");
}

#[test]
fn saved_workflow_requires_file() {
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({
            "arms": ["saved-workflow"],
            "workflow": {"jobs": {"j": {"kind": "worker", "worker": "bulk-reader", "input": {"task": "t"}}}},
        }),
    ));
    assert!(e.contains("savedWorkflowFile"), "{e}");
}

#[test]
fn route_block_requires_route_arm_and_vice_versa() {
    // route block 有り・route-* arm 無し。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        json!({
            "route": {
                "name": "r", "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child emit",
                "grant": "g", "childMode": "emit",
            },
        }),
    ));
    assert!(e.contains("without any route-* arm"), "{e}");
    // route-* arm 有り・route block 無し。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        json!({"arms": ["route-run"]}),
    ));
    assert!(e.contains("without a route block"), "{e}");
}

fn route_scenario_patch(overrides: Value) -> Value {
    let mut route = json!({
        "name": "r",
        "match": {"command": ["benchrun"]},
        "command": "benchrun --fixture-child emit",
        "grant": "g",
        "childMode": "emit",
    });
    route
        .as_object_mut()
        .unwrap()
        .extend(overrides.as_object().unwrap().clone());
    json!({
        "arms": ["route-run"],
        "route": route,
    })
}

#[test]
fn rejects_wildcard_and_self_match() {
    let e = err_of(&manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"match": {"command": ["bench*"]}})),
    ));
    assert!(e.contains("literal"), "{e}");
    let e = err_of(&manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"match": {"command": ["Cheaper"]}})),
    ));
    assert!(e.contains("self-bypass"), "{e}");
}

#[test]
fn rejects_route_command_argv_mismatch() {
    // match.command prefix と argv 不一致。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"command": "other --fixture-child emit"})),
    ));
    assert!(e.contains("literal prefix"), "{e}");
    // childMode 末尾不一致。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"childMode": "fail"})),
    ));
    assert!(e.contains("argv tail"), "{e}");
    // 末尾余分 token。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"command": "benchrun --fixture-child emit extra"})),
    ));
    assert!(e.contains("argv tail"), "{e}");
    // 未知 childMode。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"childMode": "bogus"})),
    ));
    assert!(e.contains("not a known mode"), "{e}");
    // 不正 duration。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"timeout": "0s"})),
    ));
    assert!(e.contains("route.timeout"), "{e}");
    // env-prefix 付きの合法形は受理される(env-prefix strip 規約)。
    let text = manifest_with(
        "route-routed-exec",
        route_scenario_patch(json!({"command": "FOO=1 benchrun --fixture-child emit"})),
    );
    Manifest::parse(&text).unwrap_or_else(|e| panic!("env-prefix command should parse: {e}"));
}

#[test]
fn rejects_unknown_policy_section() {
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({"policy": {"typo-section": {"x": 1}}}),
    ));
    assert!(e.contains("unknown section"), "{e}");
}

#[test]
fn rejects_comparison_arm_and_field_violations() {
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({"comparisons": [{"left": "ghost", "right": "direct-filesystem"}]}),
    ));
    assert!(e.contains("not a declared arm"), "{e}");
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({
            "arms": ["a", "b"],
            "armBase": {"a": "direct-filesystem", "b": "direct-filesystem"},
            "comparisons": [{"left": "a", "right": "b", "reduction": {"left": "bogus", "right": "retainedBytes"}}],
        }),
    ));
    assert!(e.contains("not a known metric field"), "{e}");
}

#[test]
fn rejects_unknown_expect_keys() {
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({"armExpect": {"direct-filesystem": {"nope": 1}}}),
    ));
    assert!(e.contains("unknown key"), "{e}");
    // envelope 語彙 key を非 envelope arm へ非 null で pin → reject。
    let e = err_of(&manifest_with(
        "single-simple-tool-call",
        json!({"armExpect": {"direct-filesystem": {"envelopeOk": true}}}),
    ));
    assert!(e.contains("only meaningful on envelope arms"), "{e}");
    // route block の未知 field。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        json!({
            "arms": ["route-run"],
            "route": {
                "name": "r", "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child emit",
                "grant": "g", "childMode": "emit",
            },
            "armExpect": {"route-run": {"route": {"bogus": 1}}},
        }),
    ));
    assert!(e.contains("route contains unknown key"), "{e}");
    // metrics block の未知 metric。
    let e = err_of(&manifest_with(
        "route-routed-exec",
        json!({
            "arms": ["route-run"],
            "route": {
                "name": "r", "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child emit",
                "grant": "g", "childMode": "emit",
            },
            "armExpect": {"route-run": {"metrics": {"bogus": 1}}},
        }),
    ));
    assert!(e.contains("metrics contains unknown metric"), "{e}");
}

#[test]
fn loads_the_committed_scenario_manifest() {
    // repo の本 manifest — validation 全規則を pass し 12 required
    // coverage を持つこと(task の ordering pin: flip 直後に
    // `cargo test -p cheaper-bench` で担保する)。
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../benchmarks/scenarios/scenarios.json");
    let manifest = Manifest::load(&path)
        .unwrap_or_else(|e| panic!("committed scenarios.json must validate: {e}"));
    assert_eq!(manifest.fixture_revision, "seeded-v2");
    assert_eq!(manifest.scenarios.len(), REQUIRED_SCENARIO_IDS.len());
    for required in REQUIRED_SCENARIO_IDS {
        assert!(
            manifest.scenarios.iter().any(|s| s.id == *required),
            "missing {required}"
        );
    }
    // fixture JSON 生成も検証側で round-trip 済みであることを smoke
    // 確認(失敗系 `{fixture:}` DSL が RunFixture::parse を通る)。
    let condense = manifest
        .scenarios
        .iter()
        .find(|s| s.id == "route-condense-and-refetch")
        .expect("route-condense-and-refetch");
    let spec = condense.fake_stream.as_ref().expect("condense fakeStream");
    cheaper_bench::fixture::fixture_document(Some(spec))
        .expect("condense fixture must pass RunFixture::parse");
}
