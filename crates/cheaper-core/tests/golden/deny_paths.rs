use cheaper_core::workspace::deny_paths::{
    create_path_denylist_for, deny_pattern_to_regex_for, DEFAULT_DENY_PATHS,
};

use crate::common::*;

/// Both platform axes replay on every platform: the `caseInsensitive`
/// flag recorded in the fixture is the platform-derived value the dump
/// observed, and the Rust `_for` variants take it explicitly.
#[test]
fn deny_cases() {
    let f = fixture("workspace/deny.json");
    for case in cases(&f) {
        let name = case_name(case);
        let ci = case["caseInsensitive"].as_bool().unwrap_or(cfg!(windows));
        let op = case["op"].as_str().unwrap();
        let produced: Result<serde_json::Value, serde_json::Value> = Ok(match op {
            "denyPattern" => serde_json::json!(deny_pattern_to_regex_for(
                case["pattern"].as_str().unwrap(),
                ci
            )
            .is_match(case["display"].as_str().unwrap())),
            "denylist" => {
                let extra: Vec<&str> = case["extra"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();
                serde_json::json!(create_path_denylist_for(&extra, ci)
                    .is_denied(case["display"].as_str().unwrap()))
            }
            "defaults" => serde_json::json!({
                "patterns": DEFAULT_DENY_PATHS
            }),
            other => panic!("unknown op {other} in {name}"),
        });
        check(&name, &case["expected"], produced);
    }
}
