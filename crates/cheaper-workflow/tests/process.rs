//! Port of `packages/workflow/test/process.test.ts`。
//!
//! workflow producer fixture はこの test binary を子プロセスとして再実行し、
//! ambient executable に依存せず declared output を生成する。

mod common;

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use cheaper_core::errors::ErrorCode;
use cheaper_core::nodepath;
use cheaper_core::policy::exec::build_exec_input;
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
            exec: None,
            security: None,
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
            execution_correlation: None,
        },
    }
}

const WORKFLOW_FIXTURE_ENV: &str = "CHEAPER_WORKFLOW_PROCESS_FIXTURE";

/// `process` task から同じ integration test binary を再実行する fixture。
/// 通常の test harness 実行では marker が無いため何もしない。
#[test]
fn workflow_fixture_child() {
    match std::env::var(WORKFLOW_FIXTURE_ENV) {
        Err(std::env::VarError::NotPresent) => return,
        Ok(mode) => assert_eq!(mode, "generate", "unknown workflow fixture mode"),
        Err(error) => panic!("invalid workflow fixture marker: {error}"),
    }

    std::fs::write("out.txt", "generated").expect("write declared workflow output");
}

#[tokio::test]
async fn compiles_policy_process_producer_and_executes_materialized_jobs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = realpath(&dir.path().to_string_lossy()).expect("realpath");
    let fixture_exe = std::env::current_exe()
        .expect("current test executable")
        .to_string_lossy()
        .into_owned();
    let process_policy = json!({
        "maxConcurrent": 1,
        "tasks": {
            "generate": {
                "command": [fixture_exe, "--exact", "workflow_fixture_child", "--nocapture", "--", "out.txt"],
                "env": {"CHEAPER_WORKFLOW_PROCESS_FIXTURE": "generate"},
                "inputs": [],
                "outputs": ["out.txt"]
            }
        }
    });
    // authoring `process` section → unified exec-doc へ synthesis
    // し、catalog/invocation 双方へ同じ resolved shape を渡す。
    let exec_doc = serde_json::Value::Object(
        build_exec_input(Some(&process_policy), None, &BTreeSet::new()).expect("synthesis"),
    );
    let tools = create_tool_catalog(Some(&exec_doc)).expect("catalog");
    let resources = WorkspaceResourceInference {
        workspace_root: root.clone(),
        symlink_policy: SymlinkPolicy::Deny,
        tools: &tools,
    };
    let workers = Arc::new(FixtureWorkers::new(fixture_worker()));
    let compiler = WorkflowCompiler::new(WorkflowCompilerPorts {
        tools: &tools,
        workers,
        resources: Some(&resources),
    });
    let runtime = create_invocation_tool_runtime(InvocationToolRuntimeOptions {
        artifacts: None,
        exec: Some(exec_doc.clone()),
        security: None,
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
