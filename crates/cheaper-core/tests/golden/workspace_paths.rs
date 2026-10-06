use cheaper_core::workspace::paths::{
    canonicalize_workspace_root, is_inside_workspace_root, resolve_workspace_path,
};

use crate::common::*;

#[test]
fn path_cases() {
    let f = fixture("workspace/paths.json");
    for case in cases(&f) {
        let name = case_name(case);
        let op = case["op"].as_str().unwrap();
        if op == "inside" {
            let got = is_inside_workspace_root(
                &native_path(case["root"].as_str().unwrap()),
                &native_path(case["candidate"].as_str().unwrap()),
                case["caseInsensitive"].as_bool().unwrap(),
            );
            check(&name, &case["expected"], Ok(serde_json::json!(got)));
            continue;
        }
        let Some(c) = case_dir(case) else {
            continue; // symlinks unavailable → skip (mirrors the dump)
        };
        let produced = match op {
            "resolve" => {
                let input = unscrub(&case["input"], &c.needles);
                match resolve_workspace_path(
                    &c.root,
                    input.as_str().unwrap(),
                    symlink_policy(&case["policy"]),
                ) {
                    Ok(p) => Ok(serde_json::json!({
                        "path": {"display": p.display, "real": scrub(&serde_json::json!(p.real), &c.needles)}
                    })),
                    Err(e) => Err(escape_err(&e, &c.needles)),
                }
            }
            "canonicalize" => {
                let cwd = join(
                    &c.dir.to_string_lossy(),
                    case["cwdRel"].as_str().unwrap_or("."),
                );
                match canonicalize_workspace_root(&cwd) {
                    Ok(root) => Ok(serde_json::json!({
                        "root": scrub(&serde_json::json!(root), &c.needles)
                    })),
                    Err(e) => Err(cheaper_err(&e, &c.needles)),
                }
            }
            other => panic!("unknown op {other} in {name}"),
        };
        check(&name, &case["expected"], produced);
    }
}
