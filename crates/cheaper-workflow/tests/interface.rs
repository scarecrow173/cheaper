//! Port of `packages/workflow/test/interface.test.ts`。
//!
//! TS の `vi.spyOn(list)` 相当: workflow `ToolCatalog` port は `get` のみ
//! を持つため、`list` が呼ばれないことは trait surface で保証される
//! (Rust では呼び出し不能)。`resourceInference` 未呼出しは flag で検査。

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_tools::ToolError;
use serde_json::{json, Value};

use cheaper_workflow::{
    JobKind, ResourceInference, ResourceInferenceJob, ResourceInferenceResult, ToolCatalog,
    ToolMetadata, WorkerCatalog, WorkflowCompiler, WorkflowCompilerPorts, WorkflowInterface,
};

use common::{definition, fixture_worker, FixtureTools, FixtureWorkers};

/// `get` call を記録する tool catalog spy。
struct SpyTools {
    inner: FixtureTools,
    calls: Mutex<Vec<String>>,
}

impl ToolCatalog for SpyTools {
    fn get(&self, name: &str) -> Option<ToolMetadata> {
        self.calls.lock().unwrap().push(name.to_string());
        self.inner.get(name)
    }
}

struct SpyWorkers {
    inner: FixtureWorkers,
    calls: Mutex<Vec<String>>,
}

impl WorkerCatalog for SpyWorkers {
    fn get(
        &self,
        name: &str,
    ) -> Result<Option<&cheaper_core::worker::validate::WorkerDefinition>, ToolError> {
        self.calls.lock().unwrap().push(name.to_string());
        self.inner.get(name)
    }
}

/// `infer` が呼ばれたら flag を立てる fail-fast inference。
struct FailInference {
    called: AtomicBool,
}

impl ResourceInference for FailInference {
    fn infer<'a>(
        &'a self,
        _jobs: &'a [(String, ResourceInferenceJob)],
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<ResourceInferenceResult, ToolError>>
                + Send
                + 'a,
        >,
    > {
        self.called.store(true, Ordering::SeqCst);
        Box::pin(async {
            Err(Box::new(CheaperError::new(
                ErrorCode::InternalError,
                "must not execute",
            )) as ToolError)
        })
    }
}

fn interface_value(interface: &WorkflowInterface) -> Value {
    serde_json::to_value(interface).expect("interface serializes")
}

#[test]
fn projects_public_schema_defaults_requiredness_only() {
    let inference = FailInference {
        called: AtomicBool::new(false),
    };
    let tools = SpyTools {
        inner: FixtureTools::new(),
        calls: Mutex::new(Vec::new()),
    };
    let workers = SpyWorkers {
        inner: FixtureWorkers::new(fixture_worker()),
        calls: Mutex::new(Vec::new()),
    };
    let ports = WorkflowCompilerPorts {
        tools: &tools,
        workers: &workers,
        resources: Some(&inference),
    };
    let compiler = WorkflowCompiler::new(ports);

    let mut raw = definition();
    raw["inputs"]["required"] = json!({ "type": "string" });
    raw["inputs"]["optional"] = json!({ "type": ["object", "null"], "default": null });
    raw["jobs"]["secret"] = json!({
        "kind": "worker",
        "worker": "bulk-reader",
        "input": { "task": { "$input": "required" } },
        "resources": { "reads": [] }
    });
    raw["outputs"]["public"] = json!({
        "schema": { "type": "string" },
        "value": { "$literal": "PRIVATE BINDING" }
    });

    let projection = compiler.project_interface(&raw).expect("projection");
    assert_eq!(
        interface_value(&projection),
        json!({
            "schemaVersion": 1,
            "inputs": {
                "optional": { "schema": { "type": ["object", "null"], "default": null }, "required": false },
                "required": { "schema": { "type": "string" }, "required": true }
            },
            "outputs": {
                "public": { "schema": { "type": "string" } }
            }
        })
    );
    let serialized = serde_json::to_string(&projection).unwrap();
    for secret in [
        "jobs",
        "bindings",
        "resources",
        "PRIVATE PROMPT",
        "PRIVATE BINDING",
        "$input",
        "$literal",
    ] {
        assert!(!serialized.contains(secret), "leaked: {secret}");
    }
    assert!(!inference.called.load(Ordering::SeqCst));
    assert!(tools.calls.lock().unwrap().contains(&"read".to_string()));
    assert!(workers
        .calls
        .lock()
        .unwrap()
        .contains(&"bulk-reader".to_string()));

    // projection の mutation は raw に波及しない(clone 済み)。
    let mut projected = projection;
    let required = projected
        .inputs
        .iter_mut()
        .find(|(name, _)| name == "required")
        .map(|(_, input)| input)
        .expect("required input");
    required.schema.insert("type".to_string(), json!("number"));
    assert_eq!(raw["inputs"]["required"]["type"], json!("string"));
}

#[test]
fn rejects_definition_errors_without_invocation() {
    let fixtures = common::FixtureSet::new();
    let mut raw = definition();
    raw["jobs"]["a"] = json!({
        "kind": "tool",
        "tool": "grep",
        "input": { "$input": "unknown" }
    });
    let err = fixtures
        .compiler()
        .project_interface(&raw)
        .expect_err("unknown input binding rejected");
    assert_eq!(
        err.downcast_ref::<CheaperError>().map(|e| e.code()),
        Some(ErrorCode::InvalidRequest)
    );
}

/// `process` job を catalog metadata のみで compile できる(TS test は
/// `execute` を stub するが Rust metadata には execute seam が無く、
/// 不実行は型で保証される)。
struct ProcessTools {
    inner: FixtureTools,
}

impl ToolCatalog for ProcessTools {
    fn get(&self, name: &str) -> Option<ToolMetadata> {
        if name == "process" {
            return Some(ToolMetadata {
                name: "process".to_string(),
                input_schema: json!({}),
                output_schema: common::canonical_schema(),
                resources: None,
            });
        }
        self.inner.get(name)
    }
}

#[tokio::test]
async fn compiles_process_jobs_from_tool_metadata_without_executing() {
    let tools = ProcessTools {
        inner: FixtureTools::new(),
    };
    let workers = FixtureWorkers::new(fixture_worker());
    let compiler = WorkflowCompiler::new(WorkflowCompilerPorts {
        tools: &tools,
        workers: &workers,
        resources: None,
    });
    let mut raw = definition();
    raw["jobs"]["a"] = json!({
        "kind": "tool",
        "tool": "process",
        "input": { "path": "named-task" }
    });
    let interface = compiler.project_interface(&raw).expect("projection");
    assert_eq!(
        interface_value(&interface),
        json!({ "schemaVersion": 1, "inputs": {}, "outputs": {} })
    );
    let compiled = compiler.compile(&raw, None).await.expect("compile");
    assert_eq!(compiled.job("a").expect("job a").kind, JobKind::Tool);
    let mut invalid = raw.clone();
    invalid["jobs"]["a"] = json!({ "kind": "process", "input": {} });
    let err = compiler
        .validate_definition(&invalid)
        .expect_err("kind=process rejected");
    assert_eq!(
        err.downcast_ref::<CheaperError>().map(|e| e.code()),
        Some(ErrorCode::InvalidRequest)
    );
}
