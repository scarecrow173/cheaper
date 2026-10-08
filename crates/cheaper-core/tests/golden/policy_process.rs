use cheaper_core::policy::process::{
    is_shell_process_executable, process_task_resources, resolve_process_policy,
    resolve_process_task,
};

use crate::common::*;

#[test]
fn process_cases() {
    let f = fixture("policy/process.json");
    for case in cases(&f) {
        let name = case_name(case);
        if !axis_ok(case) {
            continue;
        }
        let op = case["op"].as_str().unwrap();
        let produced = match op {
            "resolveProcessPolicy" => match resolve_process_policy(opt_input(case, "input")) {
                Ok(p) => Ok(serde_json::to_value(&p).unwrap()),
                Err(e) => Err(cheaper_err(&e, &[])),
            },
            "isShellProcessExecutable" => Ok(serde_json::json!(is_shell_process_executable(
                case["input"].as_str().unwrap()
            ))),
            "resolveProcessTask" | "processTaskResources" => {
                let policy = resolve_process_policy(opt_input(case, "policy"))
                    .unwrap_or_else(|e| panic!("{name}: fixture policy resolves: {e}"));
                let task_name = case["input"].as_str().unwrap();
                if op == "resolveProcessTask" {
                    match resolve_process_task(&policy, task_name) {
                        Ok(t) => Ok(serde_json::to_value(&t).unwrap()),
                        Err(e) => Err(cheaper_err(&e, &[])),
                    }
                } else {
                    match process_task_resources(&policy, task_name) {
                        Ok(m) => Ok(serde_json::json!({
                            "reads": scopes(m.reads),
                            "writes": scopes(m.writes),
                            "produces": scopes(m.produces),
                        })),
                        Err(e) => Err(cheaper_err(&e, &[])),
                    }
                }
            }
            other => panic!("unknown op {other} in {name}"),
        };
        check(&name, &case["expected"], produced);
    }
}

fn scopes(
    v: Option<Vec<cheaper_core::resources::normalization::ScopeDeclaration>>,
) -> serde_json::Value {
    serde_json::json!(v
        .unwrap_or_default()
        .iter()
        .map(|s| serde_json::json!({"path": s.path, "scope": s.scope.as_str()}))
        .collect::<Vec<_>>())
}
