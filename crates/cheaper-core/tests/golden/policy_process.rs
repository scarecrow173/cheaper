use std::collections::BTreeSet;

use cheaper_core::policy::exec::{
    build_exec_input, exec_grant_resources, resolve_exec_grant, resolve_exec_policy,
};
use cheaper_core::policy::security::ExecDenyList;

use crate::common::*;

/// 統一 exec model(issue #165)— fixture の `input`/`policy` field は
/// `process` authoring section 形のまま維持し、driver が
/// `build_exec_input` + `resolve_exec_policy` 経路へ変換する。
/// `resolveProcessTask`/`processTaskResources` は fixed surface の
/// `resolve_exec_grant`/`exec_grant_resources` へ写像する。
fn exec_policy_from_process_section(
    input: Option<&serde_json::Value>,
) -> Result<cheaper_core::policy::exec::ResolvedExecPolicy, cheaper_core::errors::CheaperError> {
    let doc = build_exec_input(input, None, &BTreeSet::new())?;
    resolve_exec_policy(
        Some(&serde_json::Value::Object(doc)),
        &ExecDenyList::default(),
    )
}

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
            "resolveProcessPolicy" => {
                match exec_policy_from_process_section(opt_input(case, "input")) {
                    Ok(p) => Ok(serde_json::to_value(&p).unwrap()),
                    Err(e) => Err(cheaper_err(&e, &[])),
                }
            }
            "resolveProcessTask" | "processTaskResources" => {
                let policy = exec_policy_from_process_section(opt_input(case, "policy"))
                    .unwrap_or_else(|e| panic!("{name}: fixture policy resolves: {e}"));
                let task_name = case["input"].as_str().unwrap();
                if op == "resolveProcessTask" {
                    match resolve_exec_grant(&policy, task_name, true) {
                        Ok(t) => Ok(serde_json::to_value(&t).unwrap()),
                        Err(e) => Err(cheaper_err(&e, &[])),
                    }
                } else {
                    match exec_grant_resources(&policy, task_name) {
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
