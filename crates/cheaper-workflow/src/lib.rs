//! `cheaper-workflow`: port of `packages/workflow`.
//!
//! Rig-independent workflow engine: definition schema validation,
//! `$input`/`$from`/`$literal` binding materialization, deterministic DAG
//! compile, `run_workflow` pump/admit/execute scheduler (tokio),
//! resource inference, run-scoped artifact store, and the JSON file
//! loader. Error/cancellation types come from `cheaper-core` and
//! `cheaper-tools` (`ToolError`, `AbortSignal`, `ResourceLease`,
//! `ExecutionMetrics`, artifact ports). Module layout mirrors
//! `packages/workflow/src/`.

pub mod artifacts;
pub mod bindings;
pub mod compiler;
pub mod dag;
pub mod engine;
pub mod interface;
pub mod json;
pub mod loader;
pub mod resource_inference;
pub mod schema;
pub mod types;

pub use artifacts::{
    project_artifact_metadata, with_artifact_store, ArtifactProjectionOptions,
    ArtifactStoreOptions, ArtifactStoreStats, RunArtifactStore,
};
pub use bindings::{
    materialize_bindings, parse_binding, pointer_segments, resolve_pointer, scan_bindings,
    validate_pointer_schema, MaterializeOptions, ScannedBindings,
};
pub use compiler::{WorkflowCompiler, WorkflowCompilerPorts};
pub use dag::{compile_dag, DagResult};
pub use engine::{
    run_workflow, JobExecutor, JobLeaseRequest, JobOutcomeMeta, JobStatus, WorkflowDiagnostic,
    WorkflowEnginePorts, WorkflowExecutionContext, WorkflowExecutorError, WorkflowExecutorResult,
    WorkflowJobOutcome, WorkflowOutcome, WorkflowOutcomeError, WorkflowOutcomeMeta,
    WorkflowOutputError, WorkflowRunOptions,
};
pub use interface::project_workflow_interface;
pub use json::{clone_json, has_own, is_object};
pub use loader::{load_workflow_document, load_workflow_file, LoadedWorkflow, WorkflowFileOptions};
pub use resource_inference::WorkspaceResourceInference;
pub use schema::{validate_definition, ValidatedDefinition, WORKFLOW_JSON_SCHEMA};
pub use types::*;

/// `packages/tool-runtime` の `JsonValue` re-export(`types.ts` と同じく
/// `serde_json::Value`)。
pub type JsonValue = serde_json::Value;
