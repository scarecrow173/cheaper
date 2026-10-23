use cheaper_core::policy::schema::{
    normalize_policy, resolve_concurrency_policy, validate_policy_schema, PolicySchemaMode,
};

use crate::common::*;

#[test]
fn policy_schema_cases() {
    let f = fixture("policy/schema.json");
    for case in cases(&f) {
        let name = case_name(case);
        let op = case["op"].as_str().unwrap();
        let produced = match op {
            "validatePolicySchema" => {
                let mode = match case["mode"].as_str() {
                    Some("partial") => PolicySchemaMode::Partial,
                    Some("project-partial") => PolicySchemaMode::ProjectPartial,
                    _ => PolicySchemaMode::Complete,
                };
                match validate_policy_schema(&case["input"], mode) {
                    Ok(policy) => Ok(serde_json::json!({"policy": policy})),
                    Err(e) => Err(cheaper_err(&e, &[])),
                }
            }
            "normalizePolicy" => match normalize_policy(
                case["input"].as_object().unwrap(),
                &std::collections::BTreeSet::new(),
            ) {
                Ok(p) => Ok(serde_json::json!({
                    "policy": serde_json::to_value(&p).unwrap_or(serde_json::Value::Null)
                })),
                Err(e) => Err(cheaper_err(&e, &[])),
            },
            "resolveConcurrencyPolicy" => {
                match resolve_concurrency_policy(opt_input(case, "input")) {
                    Ok(c) => Ok(serde_json::to_value(c).unwrap()),
                    Err(e) => Err(cheaper_err(&e, &[])),
                }
            }
            other => panic!("unknown op {other} in {name}"),
        };
        check(&name, &case["expected"], produced);
    }
}
