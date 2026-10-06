use cheaper_core::policy::merge::{merge_policy_layers, MergeMode};

use crate::common::*;

#[test]
fn merge_cases() {
    let f = fixture("policy/merge.json");
    for case in cases(&f) {
        let name = case_name(case);
        let mode = match case["mode"].as_str() {
            Some("restrict-concurrency") => MergeMode::RestrictConcurrency,
            _ => MergeMode::Replace,
        };
        let base = case["base"].as_object().unwrap().clone();
        let over = case["override"].as_object().unwrap().clone();
        let produced = match merge_policy_layers(&base, &over, mode) {
            Ok(merged) => Ok(serde_json::json!({
                "merged": merged,
                "baseAfter": base,
                "overrideAfter": over,
            })),
            // TS propagates the `parseDuration` RangeError raw.
            Err(e) => Err(serde_json::json!({"kind": "range", "message": e.to_string()})),
        };
        check(&name, &case["expected"], produced);
    }
}
