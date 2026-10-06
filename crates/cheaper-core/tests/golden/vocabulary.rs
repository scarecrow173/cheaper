use cheaper_core::tools::vocabulary::{normalize_tool_names, TOOL_NAMES};

use crate::common::*;

#[test]
fn vocabulary_cases() {
    let f = fixture("vocabulary.json");
    // Canonical name set + alias table.
    let names: Vec<&str> = f["toolNames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(TOOL_NAMES.to_vec(), names);
    for case in cases(&f) {
        let name = case_name(case);
        let raw = case
            .get("input")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let produced = match normalize_tool_names(&raw) {
            Ok(tools) => Ok(serde_json::to_value(tools).unwrap()),
            Err(e) => Err(cheaper_err(&e, &[])),
        };
        check(&name, &case["expected"], produced);
    }
}
