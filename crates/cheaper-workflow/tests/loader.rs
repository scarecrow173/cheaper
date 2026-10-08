//! Port of `packages/workflow/test/loader.test.ts`。
//!
//! `symlink(..., "junction")` — Windows では特権不要の junction を
//! `mklink /J` で作る(junction target は absolute 必須)。

mod common;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::workspace::deny_paths::create_path_denylist;
use cheaper_core::workspace::paths::{realpath, SymlinkPolicy};
use cheaper_tools::ToolError;
use serde_json::{json, Value};

use cheaper_workflow::{load_workflow_file, WorkflowFileOptions};

use common::{definition, FixtureSet};

struct Workspace {
    _dir: tempfile::TempDir,
    root: String,
}

fn workspace() -> Workspace {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = realpath(&dir.path().to_string_lossy()).expect("realpath");
    Workspace { _dir: dir, root }
}

fn options(root: &str, file: &str, denied: &[&str]) -> WorkflowFileOptions {
    WorkflowFileOptions {
        root: root.to_string(),
        path: file.to_string(),
        symlink_policy: SymlinkPolicy::WithinWorkspace,
        denylist: create_path_denylist(denied),
    }
}

fn save(root: &str, raw: &Value) {
    std::fs::write(
        nodepath::join(&[root, "workflow.json"]),
        serde_json::to_string(raw).expect("serialize"),
    )
    .expect("write workflow.json");
}

/// `symlink(target, link, "junction"|"dir")` 相当。
fn symlink_dir(target: &str, link: &str) {
    #[cfg(windows)]
    {
        // junction は absolute target が必須(`root` は realpath 済み)。
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J", link, target])
            .output()
            .expect("cmd mklink");
        assert!(status.status.success(), "mklink /J failed");
    }
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

#[tokio::test]
async fn loads_reusable_definition_and_compiles_through_same_compiler() {
    let ws = workspace();
    let raw = json!({
        "schemaVersion": 1,
        "inputs": {
            "target": { "type": "string" },
            "config": { "type": "object", "default": { "limit": 3 } }
        },
        "jobs": {
            "scan": {
                "kind": "tool",
                "tool": "read",
                "input": { "path": { "$input": "target" } }
            }
        },
        "outputs": {
            "result": { "schema": {}, "value": { "$from": "scan", "path": "" } }
        }
    });
    save(&ws.root, &raw);
    let fixtures = FixtureSet::new();
    let compiler = fixtures.compiler();
    let loaded = load_workflow_file(
        &options(&ws.root, "workflow.json", &[]),
        |_raw| async move { Ok::<_, ToolError>(compiler) },
    )
    .await
    .expect("load");
    assert_eq!(loaded.definition.raw(), &raw);
    assert!(
        loaded
            .compiler
            .project_interface(loaded.definition.raw())
            .expect("interface")
            .inputs
            .iter()
            .find(|(name, _)| name == "target")
            .expect("target")
            .1
            .required
    );
    let err = common::unwrap_err(loaded.compiler.compile(loaded.definition.raw(), None).await);
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidInput));
    let inputs = json!({ "target": "src" });
    let from_file = loaded
        .compiler
        .compile(loaded.definition.raw(), Some(&inputs))
        .await
        .expect("from file compiles");
    let dynamic = fixtures
        .compiler()
        .compile(&raw, Some(&inputs))
        .await
        .expect("dynamic compiles");
    assert_eq!(from_file.inputs, dynamic.inputs);
    assert_eq!(from_file.jobs, dynamic.jobs);
    assert_eq!(from_file.order, dynamic.order);
    assert_eq!(from_file.edges, dynamic.edges);
}

#[tokio::test]
async fn rejects_invalid_file_definitions_with_canonical_semantics() {
    let ws = workspace();
    for raw in [
        json!({ "schemaVersion": 2, "inputs": {}, "jobs": {}, "outputs": {} }),
        json!({
            "schemaVersion": 1,
            "inputs": { "limit": { "type": "integer", "default": "bad" } },
            "jobs": {},
            "outputs": {}
        }),
        json!({
            "schemaVersion": 1,
            "inputs": {},
            "jobs": { "a": { "kind": "tool", "tool": "read", "input": { "path": "x" }, "needs": ["a"] } },
            "outputs": {}
        }),
        json!({
            "schemaVersion": 1,
            "inputs": {},
            "jobs": {},
            "outputs": { "x": { "schema": {}, "value": { "$input": "missing" } } }
        }),
    ] {
        save(&ws.root, &raw);
        let fixtures = FixtureSet::new();
        let direct = fixtures
            .compiler()
            .validate_definition(&raw)
            .expect_err("invalid definition");
        assert_eq!(
            common::err_code(&direct),
            Some(ErrorCode::InvalidRequest),
            "direct rejection for {raw}"
        );
        let compiler = fixtures.compiler();
        let err = common::unwrap_err(
            load_workflow_file(
                &options(&ws.root, "workflow.json", &[]),
                |_raw| async move { Ok::<_, ToolError>(compiler) },
            )
            .await,
        );
        // `rejects.toEqual(directError)` — 同じ error code/message で拒否。
        let (direct, err) = (
            direct.downcast_ref::<CheaperError>().expect("cheaper"),
            err.downcast_ref::<CheaperError>().expect("cheaper"),
        );
        assert_eq!(err.code(), direct.code());
        assert_eq!(err.message(), direct.message());
    }
}

#[tokio::test]
async fn maps_read_non_file_and_parse_errors_before_compiler() {
    let ws = workspace();
    for path in ["missing.json", "."] {
        let fixtures = FixtureSet::new();
        let mut called = false;
        let compiler = fixtures.compiler();
        let err = common::unwrap_err(
            load_workflow_file(&options(&ws.root, path, &[]), |_raw| {
                called = true;
                async move { Ok::<_, ToolError>(compiler) }
            })
            .await,
        );
        assert_eq!(
            common::err_code(&err),
            Some(ErrorCode::InvalidRequest),
            "{path}"
        );
        let cheaper = err.downcast_ref::<CheaperError>().unwrap();
        assert_eq!(cheaper.exit_code(), 2, "{path}");
        assert!(!called);
    }
    std::fs::write(
        nodepath::join(&[&ws.root, "workflow.json"]),
        "PRIVATE malformed JSON",
    )
    .expect("write malformed");
    let fixtures = FixtureSet::new();
    let mut called = false;
    let compiler = fixtures.compiler();
    let err = common::unwrap_err(
        load_workflow_file(&options(&ws.root, "workflow.json", &[]), |_raw| {
            called = true;
            async move { Ok::<_, ToolError>(compiler) }
        })
        .await,
    );
    let cheaper = err.downcast_ref::<CheaperError>().expect("cheaper");
    assert_eq!(cheaper.code(), ErrorCode::InvalidInput);
    assert_eq!(cheaper.exit_code(), 2);
    assert!(!called);
}

#[tokio::test]
async fn rejects_escape_builtin_deny_and_denied_ancestors() {
    let ws = workspace();
    save(&ws.root, &definition());
    std::fs::create_dir(nodepath::join(&[&ws.root, "blocked"])).expect("mkdir blocked");
    std::fs::write(
        nodepath::join(&[&ws.root, "blocked", "workflow.json"]),
        serde_json::to_string(&definition()).unwrap(),
    )
    .expect("write blocked/workflow.json");
    for (file, denied) in [
        ("../workflow.json", Vec::<&str>::new()),
        (".env", Vec::new()),
        ("blocked/workflow.json", vec!["blocked"]),
    ] {
        let fixtures = FixtureSet::new();
        let mut called = false;
        let compiler = fixtures.compiler();
        let err = common::unwrap_err(
            load_workflow_file(&options(&ws.root, file, &denied), |_raw| {
                called = true;
                async move { Ok::<_, ToolError>(compiler) }
            })
            .await,
        );
        assert_eq!(
            common::err_code(&err),
            Some(ErrorCode::InvalidRequest),
            "{file}"
        );
        assert!(!called);
    }
}

#[tokio::test]
async fn applies_symlink_policy_and_checks_logical_and_canonical_denies() {
    let ws = workspace();
    std::fs::create_dir(nodepath::join(&[&ws.root, "target"])).expect("mkdir target");
    std::fs::write(
        nodepath::join(&[&ws.root, "target", "workflow.json"]),
        serde_json::to_string(&definition()).unwrap(),
    )
    .expect("write target/workflow.json");
    symlink_dir(
        &nodepath::join(&[&ws.root, "target"]),
        &nodepath::join(&[&ws.root, "alias"]),
    );
    // `resolves` — junction 経由でも within-workspace では canonical target
    // が workspace 内なので受理される。
    let fixtures = FixtureSet::new();
    let loaded = load_workflow_file(&options(&ws.root, "alias/workflow.json", &[]), |_raw| {
        let compiler = fixtures.compiler();
        async move { Ok::<_, ToolError>(compiler) }
    })
    .await
    .expect("alias load");
    assert_eq!(loaded.definition.raw(), &definition());
    let mut denied = options(&ws.root, "alias/workflow.json", &[]);
    denied.symlink_policy = SymlinkPolicy::Deny;
    let compiler = fixtures.compiler();
    let err = common::unwrap_err(
        load_workflow_file(&denied, |_raw| async move { Ok::<_, ToolError>(compiler) }).await,
    );
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
    for prefix in ["alias", "target"] {
        let compiler = fixtures.compiler();
        let err = common::unwrap_err(
            load_workflow_file(
                &options(&ws.root, "alias/workflow.json", &[prefix]),
                |_raw| async move { Ok::<_, ToolError>(compiler) },
            )
            .await,
        );
        assert_eq!(
            common::err_code(&err),
            Some(ErrorCode::InvalidRequest),
            "{prefix}"
        );
    }
    symlink_dir(
        &nodepath::dirname(&ws.root),
        &nodepath::join(&[&ws.root, "outside"]),
    );
    let compiler = fixtures.compiler();
    let err = common::unwrap_err(
        load_workflow_file(
            &options(&ws.root, "outside/workflow.json", &[]),
            |_raw| async move { Ok::<_, ToolError>(compiler) },
        )
        .await,
    );
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
}

#[tokio::test]
async fn exposes_default_and_input_validation_without_compiling_jobs() {
    let fixtures = FixtureSet::new();
    let compiler = fixtures.compiler();
    let raw = json!({
        "schemaVersion": 1,
        "inputs": {
            "target": { "type": "string" },
            "config": { "type": "object", "default": { "limit": 3 } }
        },
        "jobs": {},
        "outputs": {}
    });
    let inputs = compiler
        .validate_inputs(&raw, Some(&json!({ "target": "src" })))
        .expect("validate");
    assert_eq!(
        Value::Object(inputs.clone()),
        json!({ "target": "src", "config": { "limit": 3 } })
    );
    // 返却値は caller 所有 — mutate しても compiler 側へ波及しない。
    let mut mutated = inputs.clone();
    mutated["config"]["limit"] = json!(99);
    let revalidated = compiler
        .validate_inputs(&raw, Some(&json!({ "target": "src" })))
        .expect("revalidate");
    let compiled = compiler
        .compile(&raw, Some(&json!({ "target": "src" })))
        .await
        .expect("compile");
    assert_eq!(revalidated, compiled.inputs);
    for provided in [
        json!({}),
        json!({ "target": 1 }),
        json!({ "target": "x", "unknown": true }),
        json!([]),
    ] {
        let err = compiler
            .validate_inputs(&raw, Some(&provided))
            .expect_err("validate rejects");
        assert_eq!(
            common::err_code(&err),
            Some(ErrorCode::InvalidInput),
            "{provided}"
        );
        let err = common::unwrap_err(compiler.compile(&raw, Some(&provided)).await);
        assert_eq!(
            common::err_code(&err),
            Some(ErrorCode::InvalidInput),
            "{provided}"
        );
    }
}

#[cfg(windows)]
#[tokio::test]
async fn rejects_case_aliases_of_denied_files_on_windows() {
    let ws = workspace();
    std::fs::create_dir(nodepath::join(&[&ws.root, "blocked"])).expect("mkdir blocked");
    std::fs::write(
        nodepath::join(&[&ws.root, "blocked", "workflow.json"]),
        serde_json::to_string(&definition()).unwrap(),
    )
    .expect("write blocked/workflow.json");
    std::fs::write(
        nodepath::join(&[&ws.root, ".env"]),
        serde_json::to_string(&definition()).unwrap(),
    )
    .expect("write .env");
    let fixtures = FixtureSet::new();
    for (file, denied) in [
        ("Blocked/workflow.json", vec!["blocked"]),
        ("blocked/workflow.json", vec!["BLOCKED"]),
        (".ENV", Vec::<&str>::new()),
    ] {
        let mut called = false;
        let compiler = fixtures.compiler();
        let err = common::unwrap_err(
            load_workflow_file(&options(&ws.root, file, &denied), |_raw| {
                called = true;
                async move { Ok::<_, ToolError>(compiler) }
            })
            .await,
        );
        assert_eq!(
            common::err_code(&err),
            Some(ErrorCode::InvalidRequest),
            "{file}"
        );
        assert!(!called);
    }
}
