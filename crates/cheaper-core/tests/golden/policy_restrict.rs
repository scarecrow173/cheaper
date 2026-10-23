use cheaper_core::policy::capability::check_capabilities;
use cheaper_core::policy::restrict::{restrict_policy, PolicyRestrictions};
use cheaper_core::policy::schema::{normalize_policy, validate_policy_schema, PolicySchemaMode};
use cheaper_core::worker::parse::parse_worker_file;
use cheaper_core::worker::validate::{validate_worker_definition, WorkerOrigin};

use crate::common::*;

fn normalize_base(raw: &serde_json::Value) -> cheaper_core::policy::schema::RuntimePolicy {
    let validated =
        validate_policy_schema(raw, PolicySchemaMode::Complete).expect("fixture policy validates");
    normalize_policy(&validated, &std::collections::BTreeSet::new())
        .expect("fixture policy normalizes")
}

#[test]
fn restrict_cases() {
    let f = fixture("policy/restrict.json");
    for case in cases(&f) {
        let name = case_name(case);
        let op = case["op"].as_str().unwrap_or("restrict");
        let policy = normalize_base(&case["policy"]);
        let produced = match op {
            "restrict" => {
                let r = &case["restrictions"];
                let restrictions = PolicyRestrictions {
                    timeout_ms: r.get("timeoutMs").and_then(|v| v.as_u64()),
                    // `get` returns Some only when the key is present —
                    // `concurrency: null` stays distinguishable from
                    // `concurrency` absent (the `!== undefined` gate).
                    concurrency: r.get("concurrency").cloned(),
                };
                match restrict_policy(&policy, &restrictions) {
                    Ok(p) => Ok(serde_json::json!({
                        "policy": serde_json::to_value(&p).unwrap_or(serde_json::Value::Null)
                    })),
                    Err(e) => Err(cheaper_err(&e, &[])),
                }
            }
            "checkCapabilities" => {
                let Some(c) = case_dir(case) else {
                    panic!("{name}: case dir setup failed");
                };
                let sp = join(&c.dir.to_string_lossy(), "w.md");
                let content = std::fs::read_to_string(&sp).unwrap();
                let parsed = parse_worker_file(&content, &sp).unwrap();
                let def = validate_worker_definition(&parsed, &sp, WorkerOrigin::Project).unwrap();
                match check_capabilities(&def, &policy) {
                    Ok(tools) => Ok(serde_json::json!({
                        "tools": tools.iter().map(|t| t.as_str()).collect::<Vec<_>>()
                    })),
                    Err(e) => Err(cheaper_err(&e, &c.needles)),
                }
            }
            other => panic!("unknown op {other}"),
        };
        check(&name, &case["expected"], produced);
    }
}
