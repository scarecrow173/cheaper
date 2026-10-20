//! Port of `packages/workflow/test/process.test.ts`。
//!
//! `node` executable が無い環境では skip する(TS 側は実 `node` を
//! spawn する policy task)。

mod common;

use std::collections::{HashMap, HashSet};

use cheaper_core::errors::ErrorCode;
use cheaper_core::nodepath;
use cheaper_core::resources::normalization::ScopeDeclaration as CoreScopeDeclaration;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::paths::{realpath, SymlinkPolicy};
use cheaper_tools::catalog::create_tool_catalog;
use cheaper_tools::invocation::{
    create_invocation_tool_runtime, InvocationToolRuntimeOptions, WorkerScopeOptions,
};
use cheaper_tools::registry::ToolRuntimeOptions;
use serde_json::json;

use cheaper_workflow::{
    JobKind, ProcessResourceDeclaration, ResourceInference, ResourceInferenceJob, WorkflowCompiler,
    WorkflowCompilerPorts, WorkflowJob, WorkspaceResourceInference,
};

use common::{fixture_worker, FixtureWorkers};

fn worker_scope_options(owner: &str, root: &str, names: Vec<ToolName>) -> WorkerScopeOptions {
    WorkerScopeOptions {
        owner_id: owner.to_string(),
        runtime: ToolRuntimeOptions {
            resource_manifest: None,
            write_scope: None,
            read_strictness: None,
            artifacts: None,
            process: None,
            command: None,
            root: root.to_string(),
            symlink_policy: SymlinkPolicy::Deny,
            names,
            degradable: Vec::new(),
            max_tool_calls: 10,
            on_limit: Box::new(|| {}),
            journal: None,
            denylist: None,
            parent_lease: None,
            signal: None,
            abort_grace_ms: None,
            concurrency: None,
            eval_spawn: None,
            route_context: None,
        },
    }
}

/// Rust runtime 固定ルール: bare `node` は `process.execPath` 相当の
/// host が無いため明示 `POLICY_VIOLATION`。PATH 上の実 `node` の
/// absolute path に解決して task command へ入れる。
fn resolve_node() -> Option<String> {
    let output = std::process::Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg("node")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let path = stdout.lines().next()?.trim().to_string();
    (nodepath::is_absolute(&path) && std::fs::metadata(&path).is_ok()).then_some(path)
}

#[tokio::test]
async fn compiles_policy_process_producer_and_executes_materialized_jobs() {
    let Some(node) = resolve_node() else {
        eprintln!("skipping: node executable not found");
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let root = realpath(&dir.path().to_string_lossy()).expect("realpath");
    std::fs::write(
        nodepath::join(&[&root, "child.mjs"]),
        "import {writeFileSync} from \"node:fs\"; writeFileSync(\"out.txt\", \"generated\");",
    )
    .expect("child.mjs");
    let process_policy = json!({
        "maxConcurrent": 1,
        "tasks": {
            "generate": {
                "command": [node, "child.mjs"],
                "inputs": ["child.mjs"],
                "outputs": ["out.txt"]
            }
        }
    });
    let tools = create_tool_catalog(Some(&process_policy)).expect("catalog");
    let resources = WorkspaceResourceInference {
        workspace_root: root.clone(),
        symlink_policy: SymlinkPolicy::Deny,
        tools: &tools,
    };
    let workers = FixtureWorkers::new(fixture_worker());
    let compiler = WorkflowCompiler::new(WorkflowCompilerPorts {
        tools: &tools,
        workers: &workers,
        resources: Some(&resources),
    });
    let runtime = create_invocation_tool_runtime(InvocationToolRuntimeOptions {
        artifacts: None,
        process: Some(process_policy.clone()),
        command: None,
        root: root.clone(),
        symlink_policy: SymlinkPolicy::Deny,
        signal: None,
        abort_grace_ms: None,
        concurrency: None,
        clock: None,
    })
    .expect("runtime");

    let mut raw = common::definition();
    raw["jobs"]["producer"] =
        json!({ "kind": "tool", "tool": "process", "input": { "task": "generate" } });
    raw["jobs"]["consumer"] =
        json!({ "kind": "tool", "tool": "read", "input": { "path": "out.txt" } });

    let compiled = compiler.compile(&raw, None).await.expect("compile");
    assert!(compiled.edges.contains(&cheaper_workflow::DependencyEdge {
        from: "producer".to_string(),
        to: "consumer".to_string()
    }));
    assert_eq!(
        compiled.order,
        vec!["producer".to_string(), "consumer".to_string()]
    );

    let mut results: HashMap<String, serde_json::Value> = HashMap::new();
    for id in compiled.order.clone() {
        let scope = runtime
            .create_worker_scope(worker_scope_options(
                &id,
                &root,
                vec![if id == "producer" {
                    ToolName::Process
                } else {
                    ToolName::Read
                }],
            ))
            .expect("worker scope");
        let input = compiled
            .materialize_job_input(&id, &results, &HashSet::new())
            .expect("materialize");
        let result = scope
            .execute(
                if id == "producer" { "process" } else { "read" },
                &input,
                None,
            )
            .await
            .expect("execute");
        results.insert(id, serde_json::to_value(&result).expect("result json"));
    }
    assert_eq!(
        std::fs::read_to_string(nodepath::join(&[&root, "out.txt"])).expect("out.txt"),
        "generated"
    );
    assert_eq!(
        results["consumer"]["content"][0]["text"],
        json!("generated")
    );

    for produces in [
        serde_json::json!([]),
        serde_json::json!([{ "path": "outside.txt", "scope": "exact" }]),
    ] {
        raw["jobs"]["producer"]["resources"] = json!({ "produces": produces });
        let err = common::unwrap_err(compiler.compile(&raw, None).await);
        assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));
    }
    raw["jobs"]["producer"]
        .as_object_mut()
        .unwrap()
        .remove("resources");

    let err = resources
        .infer(&[(
            "a".to_string(),
            ResourceInferenceJob {
                process: None,
                job: WorkflowJob {
                    kind: JobKind::Tool,
                    name: "process".to_string(),
                    input: json!({ "task": "unknown" }),
                    needs: None,
                    when: None,
                    resources: None,
                },
                worker: None,
            },
        )])
        .await
        .expect_err("unknown task");
    assert!(common::err_code(&err).is_some());
    let err = resources
        .infer(&[(
            "a".to_string(),
            ResourceInferenceJob {
                process: Some(ProcessResourceDeclaration {
                    task: "different".to_string(),
                    inputs: Vec::new(),
                    outputs: Vec::<CoreScopeDeclaration>::new(),
                }),
                job: WorkflowJob {
                    kind: JobKind::Tool,
                    name: "process".to_string(),
                    input: json!({ "task": "generate" }),
                    needs: None,
                    when: None,
                    resources: None,
                },
                worker: None,
            },
        )])
        .await
        .expect_err("task mismatch");
    assert_eq!(common::err_code(&err), Some(ErrorCode::InvalidRequest));

    runtime.dispose().await;
}
