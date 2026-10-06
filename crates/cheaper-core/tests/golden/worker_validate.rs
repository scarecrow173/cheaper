use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::parse::parse_worker_file;
use cheaper_core::worker::validate::{
    is_mutating_tool, validate_worker_definition, validate_worker_input, WorkerOrigin,
};
use cheaper_core::worker::write_scope::extract_write_scope;

use crate::common::*;

fn origin(v: &serde_json::Value) -> WorkerOrigin {
    match v.as_str() {
        Some("user") => WorkerOrigin::User,
        Some("builtin") => WorkerOrigin::Builtin,
        Some("explicit") => WorkerOrigin::Explicit,
        _ => WorkerOrigin::Project,
    }
}

#[test]
fn validate_cases() {
    let f = fixture("worker/validate.json");
    for case in cases(&f) {
        let name = case_name(case);
        let op = case["op"].as_str().unwrap_or("validate");
        if op == "isMutatingTool" {
            let got = serde_json::json!({
                "read": is_mutating_tool(ToolName::Read),
                "write": is_mutating_tool(ToolName::Write),
                "edit": is_mutating_tool(ToolName::Edit),
                "process": is_mutating_tool(ToolName::Process),
                "grep": is_mutating_tool(ToolName::Grep),
            });
            check(&name, &case["expected"], Ok(got));
            continue;
        }
        let Some(c) = case_dir(case) else {
            panic!("{name}: case dir setup failed");
        };
        let source_path = crate::common::join(
            &c.dir.to_string_lossy(),
            case["sourcePath"].as_str().unwrap(),
        );
        let produced: Result<serde_json::Value, serde_json::Value> = (|| {
            let parsed = match parse_worker_file(case["content"].as_str().unwrap(), &source_path) {
                Ok(p) => p,
                Err(e) => return Err(cheaper_err(&e, &c.needles)),
            };
            let def =
                match validate_worker_definition(&parsed, &source_path, origin(&case["origin"])) {
                    Ok(d) => d,
                    Err(e) => return Err(cheaper_err(&e, &c.needles)),
                };
            match op {
                "validate" => Ok(serde_json::json!({"def": dump_worker_def(&def, &c.needles)})),
                "validateWorkerInput" => match validate_worker_input(&def, &case["input"]) {
                    Ok(()) => Ok(serde_json::json!({"inputOk": true})),
                    // The dump records inner errors as `{ok: {error}}`.
                    Err(e) => Ok(serde_json::json!({"error": cheaper_err(&e, &c.needles)})),
                },
                "extractWriteScope" => match extract_write_scope(&def, &case["input"]) {
                    // `None` == TS `undefined` (no writeScope configured or
                    // the field is absent); `Some` yields the normalized set.
                    Ok(None) => Ok(serde_json::json!({"scope": null})),
                    Ok(Some(scope)) => {
                        let mut list: Vec<String> = scope.into_iter().collect();
                        list.sort();
                        Ok(serde_json::json!({"scope": list}))
                    }
                    Err(e) => Ok(serde_json::json!({"error": cheaper_err(&e, &c.needles)})),
                },
                other => panic!("unknown op {other} in {name}"),
            }
        })();
        // `extractWriteScope` returns a set — compare order-insensitively.
        let mut expected = case["expected"].clone();
        if op == "extractWriteScope" {
            if let Some(arr) = expected
                .get_mut("ok")
                .and_then(|ok| ok.get_mut("scope"))
                .and_then(|s| s.as_array_mut())
            {
                arr.sort_by(|a, b| a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or("")));
            }
        }
        check(&name, &expected, produced);
    }
}
