//! process 固有 glue(task 解決・prepare 検査・declaration・tool surface)
//! の unit test。shared lifecycle 機構の test 群(`capture_finish_*`・
//! `display_*`・`minimal_env_*`・`spawn_code_*`/`spawn_failure_*`)は
//! `tools::proc::tests` へ移設済み(design §(e))。

use std::collections::HashSet;

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Value};

use crate::operation::{AbortController, AbortReason};

use super::*;

#[test]
fn operation_declaration_derives_access_from_outputs() {
    let policy = cheaper_core::policy::process::resolve_process_policy(Some(&json!({
        "maxConcurrent": 3,
        "tasks": {
            "ro": {"command": ["cc"], "inputs": [], "outputs": []},
            "wr": {"command": ["cc"], "inputs": [], "outputs": ["out"]},
        },
    })))
    .unwrap();
    let ro = process_operation_declaration(&policy, "ro").unwrap();
    assert_eq!(ro.access, cheaper_core::resources::types::ClaimMode::Shared);
    let pool = ro.pool.as_ref().expect("process pool");
    assert_eq!(pool.name, "process");
    assert_eq!(pool.max_concurrent, 3);
    let wr = process_operation_declaration(&policy, "wr").unwrap();
    assert_eq!(
        wr.access,
        cheaper_core::resources::types::ClaimMode::Exclusive
    );
    let err = process_operation_declaration(&policy, "nope").unwrap_err();
    assert_eq!(err.code(), ErrorCode::PolicyViolation);
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-proc-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }

    fn root(&self) -> String {
        cheaper_core::workspace::paths::canonicalize_workspace_root(self.0.to_str().unwrap())
            .unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn policy(tasks: Value) -> ResolvedProcessPolicy {
    cheaper_core::policy::process::resolve_process_policy(Some(&json!({
        "tasks": tasks,
    })))
    .unwrap()
}

fn tool(root: String, tasks: Value) -> ProcessTool {
    create_process_tool(ProcessToolOptions {
        root,
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(tasks),
        on_timeout: None,
        denylist: None,
        write_scope: None,
    })
}

fn cheaper_code(err: &ToolError) -> Option<ErrorCode> {
    find_cheaper_error(err.as_ref()).map(|e| e.code())
}

#[tokio::test]
async fn unknown_task_is_policy_violation_before_spawn() {
    let dir = TempDir::new();
    let tool = tool(
        dir.root(),
        json!({"check": {"command": ["cc"], "inputs": [], "outputs": []}}),
    );
    let err = tool
        .execute(&json!({"task": "missing"}), None)
        .await
        .unwrap_err();
    assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
}

#[tokio::test]
async fn prepare_rejects_missing_cwd() {
    let dir = TempDir::new();
    let tool = tool(
        dir.root(),
        json!({"check": {
            "command": ["cc"], "inputs": [], "outputs": [], "cwd": "missing",
        }}),
    );
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    // TS parity: `stat` の ENOENT は CheaperError で wrap せず raw
    // error で伝播する(isDirectory=false のみ INVALID_REQUEST)。
    assert_eq!(cheaper_code(&err), None);
    assert!(err.is::<std::io::Error>() || err.source().is_some());
}

#[tokio::test]
async fn prepare_rejects_denied_output_and_write_scope_excess() {
    let dir = TempDir::new();
    let root = dir.root();
    // declared output が denylist に一致 → deny。
    let denylist = cheaper_core::workspace::deny_paths::create_path_denylist(&[".cheaper/**"]);
    let tool = create_process_tool(ProcessToolOptions {
        root: root.clone(),
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(json!({"check": {
            "command": ["cc"], "inputs": [], "outputs": [".cheaper/out"],
        }})),
        on_timeout: None,
        denylist: Some(denylist),
        write_scope: None,
    });
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
    // writeScope 外の output → POLICY_VIOLATION。
    let tool = create_process_tool(ProcessToolOptions {
        root,
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(json!({"check": {
            "command": ["cc"], "inputs": [], "outputs": ["out"],
        }})),
        on_timeout: None,
        denylist: None,
        write_scope: Some(HashSet::from(["allowed".to_string()])),
    });
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
}

/// magic check(MZ)を通すが exec 不可能な executable。
/// `ERROR_EXE_MACHINE_TYPE_MISMATCH`(216)→ libuv 写像 `"UNKNOWN"`、
/// Node は非 ENOENT error を sync throw するため
/// `"managed process spawn failed"` + `spawnCode` + cause 相当。
/// (unix 側は `execvpe` の ENOEXEC→`/bin/sh` fallback が干渉するため
/// execute-level 検証は win32 のみ — mapping/shape 規則は unit test
/// で別途担保する。)
#[cfg(windows)]
#[tokio::test]
async fn spawn_errno_failure_reports_libuv_style_code() {
    let dir = TempDir::new();
    let bogus = dir.0.join("bogus.exe");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&bogus).unwrap();
        f.write_all(b"MZnot-a-real-pe").unwrap();
    }
    let tool = tool(
        dir.root(),
        json!({"check": {"command": ["./bogus.exe"], "inputs": [], "outputs": []}}),
    );
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    let ce = find_cheaper_error(err.as_ref()).expect("CheaperError");
    assert_eq!(ce.code(), ErrorCode::ToolExecutionFailure);
    let details = ce.details().expect("details");
    assert_eq!(ce.message(), "managed process spawn failed");
    // 216(machine type mismatch)は libuv の unmapped → "UNKNOWN"。
    assert_eq!(details["spawnCode"], json!("UNKNOWN"));
    assert_eq!(details["task"], json!("check"));
    assert_eq!(details["state"], json!("failed"));
}

#[tokio::test]
async fn pre_spawn_abort_rejects_with_outer_reason() {
    let dir = TempDir::new();
    let tool = tool(
        dir.root(),
        json!({"check": {"command": ["cc"], "inputs": [], "outputs": []}}),
    );
    let controller = AbortController::new();
    controller.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Aborted,
        "outer wins",
    )));
    let err = tool
        .execute(&json!({"task": "check"}), Some(controller.signal()))
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "outer wins");
    assert_eq!(cheaper_code(&err), Some(ErrorCode::Aborted));
}
