use cheaper_core::config::dirs::ConfigDirs;
use cheaper_core::worker::resolve::{resolve_worker, resolve_worker_file, ResolveWorkerError};

use crate::common::*;

/// `cap` shape for a `ResolveWorkerError`: `CheaperError` → the cheaper
/// shape; a raw `WorkerReadError` mirrors the TS `ErrnoException`, which
/// `cap` collapses to `{kind: "io"}` (it carries a string `code`).
fn resolve_err(e: &ResolveWorkerError, needles: &[(String, String)]) -> serde_json::Value {
    match e {
        ResolveWorkerError::Cheaper(e) => cheaper_err(e, needles),
        ResolveWorkerError::Read(_) => serde_json::json!({"kind": "io"}),
    }
}

#[test]
fn resolve_cases() {
    let f = fixture("worker/resolve.json");
    for case in cases(&f) {
        let name = case_name(case);
        let Some(c) = case_dir(case) else {
            panic!("{name}: case dir setup failed");
        };
        let op = case["op"].as_str().unwrap();
        let produced = match op {
            "resolveWorker" => {
                let dirs = ConfigDirs {
                    user_config_dir: c.dir.join(crate::common::rel_path_pub(
                        case["dirs"]["userConfigDir"].as_str().unwrap(),
                    )),
                    project_dir: c.dir.join(crate::common::rel_path_pub(
                        case["dirs"]["projectDir"].as_str().unwrap(),
                    )),
                    builtin_dir: c.dir.join(crate::common::rel_path_pub(
                        case["dirs"]["builtinDir"].as_str().unwrap(),
                    )),
                };
                match resolve_worker(case["workerName"].as_str().unwrap(), &dirs) {
                    Ok(def) => Ok(serde_json::json!({"def": dump_worker_def(&def, &c.needles)})),
                    Err(e) => Err(resolve_err(&e, &c.needles)),
                }
            }
            "resolveWorkerFile" => {
                let cwd = crate::common::join(
                    &c.dir.to_string_lossy(),
                    case["cwdRel"].as_str().unwrap_or("."),
                );
                match resolve_worker_file(case["filePath"].as_str().unwrap(), &cwd) {
                    Ok(def) => Ok(serde_json::json!({"def": dump_worker_def(&def, &c.needles)})),
                    Err(e) => Err(resolve_err(&e, &c.needles)),
                }
            }
            other => panic!("unknown op {other}"),
        };
        check(&name, &case["expected"], produced);
    }
}
