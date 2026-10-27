//! `packages/workflow/test/fixtures.ts` の Rust 版。
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use cheaper_core::model::tier::Tier;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::schema::{compile_schema, SchemaKind, SchemaRef};
use cheaper_core::worker::validate::{
    WorkerCheaper, WorkerDefinition, WorkerIo, WorkerModel, WorkerOrigin,
};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use cheaper_workflow::{
    JobExecutor, JobLeaseRequest, ToolCatalog as WorkflowToolCatalog, ToolMetadata, WorkerCatalog,
    WorkflowCompiler, WorkflowCompilerPorts, WorkflowEnginePorts, WorkflowExecutionContext,
    WorkflowExecutorResult,
};

pub type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'static>>;

pub fn canonical_schema() -> Value {
    json!({
        "type": "object",
        "required": ["content"],
        "additionalProperties": false,
        "properties": {
            "content": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["type", "text"],
                    "additionalProperties": false,
                    "properties": {
                        "type": { "const": "text" },
                        "text": { "type": "string" }
                    }
                }
            },
            "details": { "type": "object", "additionalProperties": true }
        }
    })
}

/// fixture tools(`read`/`grep`/`count_tokens`)。`resources` callback は
/// 持たない(TS fixture と同じ)。
pub struct FixtureTools {
    metas: Vec<ToolMetadata>,
}

impl FixtureTools {
    pub fn new() -> Self {
        Self {
            metas: vec![
                ToolMetadata {
                    name: "read".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "required": ["path"],
                        "additionalProperties": false,
                        "properties": { "path": { "type": "string" } }
                    }),
                    output_schema: canonical_schema(),
                    resources: None,
                },
                ToolMetadata {
                    name: "grep".to_string(),
                    input_schema: json!({}),
                    output_schema: canonical_schema(),
                    resources: None,
                },
                ToolMetadata {
                    name: "count_tokens".to_string(),
                    input_schema: json!({
                        "type": "object",
                        "required": ["path"],
                        "additionalProperties": false,
                        "properties": { "path": { "type": "string" } }
                    }),
                    output_schema: canonical_schema(),
                    resources: None,
                },
            ],
        }
    }
}

impl WorkflowToolCatalog for FixtureTools {
    fn get(&self, name: &str) -> Option<ToolMetadata> {
        self.metas
            .iter()
            .find(|m| m.name == name)
            .map(|m| ToolMetadata {
                name: m.name.clone(),
                input_schema: m.input_schema.clone(),
                output_schema: m.output_schema.clone(),
                resources: None,
            })
    }
}

/// `bulk-reader` fixture worker。
pub fn fixture_worker() -> WorkerDefinition {
    WorkerDefinition {
        name: "bulk-reader".to_string(),
        description: "fake validated metadata".to_string(),
        tools: vec![ToolName::Read],
        cheaper: WorkerCheaper {
            model: WorkerModel {
                tier: Tier::Low,
                workload: None,
            },
            input: None,
            output: WorkerIo {
                schema: SchemaRef::Inline(Map::new()),
            },
            write_scope: None,
            optional_tools: Vec::new(),
        },
        instructions: "PRIVATE PROMPT".to_string(),
        source_path: "fake/bulk-reader.md".to_string(),
        origin: WorkerOrigin::Builtin,
        input_schema: Some(Arc::new(
            compile_schema(
                json!({
                    "type": "object",
                    "required": ["task"],
                    "additionalProperties": false,
                    "properties": { "task": { "type": "string" } }
                })
                .as_object()
                .unwrap()
                .clone(),
                SchemaKind::Input,
            )
            .expect("input schema compiles"),
        )),
        output_schema: Arc::new(
            compile_schema(Map::new(), SchemaKind::Output).expect("output schema compiles"),
        ),
    }
}

pub struct FixtureWorkers {
    worker: WorkerDefinition,
}

impl FixtureWorkers {
    pub fn new(worker: WorkerDefinition) -> Self {
        Self { worker }
    }
}

impl WorkerCatalog for FixtureWorkers {
    fn get(&self, name: &str) -> Result<Option<&WorkerDefinition>, ToolError> {
        Ok((name == self.worker.name).then_some(&self.worker))
    }
}

/// `fixtures(overrides)` 相当。返す struct が catalog ownership を持ち、
/// `compiler()` で借用 ports を組み立てる。TS の `worker` mutation は
/// `worker_mut()` で catalog 側 instance へ反映する。
pub struct FixtureSet<'a> {
    pub tools: FixtureTools,
    workers: Arc<FixtureWorkers>,
    pub resources: Option<&'a dyn cheaper_workflow::ResourceInference>,
}

impl<'a> FixtureSet<'a> {
    pub fn new() -> Self {
        Self {
            tools: FixtureTools::new(),
            workers: Arc::new(FixtureWorkers {
                worker: fixture_worker(),
            }),
            resources: None,
        }
    }

    pub fn with_resources(
        mut self,
        resources: &'a dyn cheaper_workflow::ResourceInference,
    ) -> Self {
        self.resources = Some(resources);
        self
    }

    /// fixture worker metadata(catalog と同一 instance)。
    pub fn worker(&self) -> &WorkerDefinition {
        &self.workers.worker
    }

    /// TS `worker.tools = [...]` 等の mutation seam。
    pub fn worker_mut(&mut self) -> &mut WorkerDefinition {
        &mut Arc::get_mut(&mut self.workers)
            .expect("worker mutation before compiler creation")
            .worker
    }

    /// `new WorkflowCompiler(ports)`。
    pub fn compiler(&self) -> WorkflowCompiler<'_> {
        WorkflowCompiler::new(WorkflowCompilerPorts {
            tools: &self.tools,
            workers: self.workers.clone(),
            resources: self.resources,
        })
    }
}

impl Default for FixtureSet<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// `definition()` — `{ schemaVersion: 1, inputs: {}, jobs: {}, outputs: {} }`。
pub fn definition() -> Value {
    json!({ "schemaVersion": 1, "inputs": {}, "jobs": {}, "outputs": {} })
}

// ---------------------------------------------------------------------------
// engine harness
// ---------------------------------------------------------------------------

/// `toolResult(text)`。
pub fn tool_result(text: &str) -> WorkflowExecutorResult {
    WorkflowExecutorResult {
        result: json!({ "content": [{ "type": "text", "text": text }] }),
        meta: None,
    }
}

/// closure を `JobExecutor` に写す adapter。
pub struct FnExecutor<F>(pub F);

impl<F, Fut> JobExecutor for FnExecutor<F>
where
    F: Fn(WorkflowExecutionContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = Result<WorkflowExecutorResult, ToolError>> + Send + 'static,
{
    fn execute(
        &self,
        context: WorkflowExecutionContext,
    ) -> BoxFuture<Result<WorkflowExecutorResult, ToolError>> {
        Box::pin((self.0)(context))
    }
}

/// `deferred()` — engine test の one-shot latch。
pub struct Deferred<T> {
    tx: Mutex<Option<tokio::sync::oneshot::Sender<T>>>,
    rx: Mutex<Option<tokio::sync::oneshot::Receiver<T>>>,
}

impl<T> Deferred<T> {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::oneshot::channel();
        Self {
            tx: Mutex::new(Some(tx)),
            rx: Mutex::new(Some(rx)),
        }
    }

    pub fn resolve(&self, value: T) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            let _ = tx.send(value);
        }
    }

    /// TS の late rejection 相当 — 送信側を破棄して `wait()` を
    /// `Err` で settle させる(`Deferred` instance 自体の drop では
    /// executor 側が持つ Arc clone が残るため効かない)。
    pub fn reject(&self) {
        let _ = self.tx.lock().unwrap().take();
    }

    /// deferred の drop は TS の promise reject 相当(late rejection)。
    pub async fn wait(&self) -> Result<T, ToolError> {
        let rx = self.rx.lock().unwrap().take().expect("waited once");
        rx.await
            .map_err(|_| Box::new(std::io::Error::other("deferred dropped")) as ToolError)
    }
}

impl<T> Default for Deferred<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// engine ports harness(`harness()`)。`ResourceManager` 経由の real
/// lease + worker/tool mock executor + call counters。
pub struct EngineHarness {
    pub ports: WorkflowEnginePorts,
    pub resources: cheaper_tools::resources::ResourceManager,
    pub acquire_calls: Arc<Mutex<Vec<String>>>,
    pub abort_calls: Arc<Mutex<Vec<cheaper_tools::operation::AbortReason>>>,
    pub release_calls: Arc<Mutex<Vec<cheaper_tools::resources::ResourceLease>>>,
}

pub fn harness(worker: Arc<dyn JobExecutor>, tool: Arc<dyn JobExecutor>) -> EngineHarness {
    let resources = cheaper_tools::resources::ResourceManager::new(
        &std::env::current_dir().unwrap().to_string_lossy(),
        cheaper_core::workspace::paths::SymlinkPolicy::Deny,
    );
    let acquire_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let abort_calls = Arc::new(Mutex::new(Vec::new()));
    let release_calls = Arc::new(Mutex::new(Vec::new()));

    let rm = resources.clone();
    let acquired = Arc::clone(&acquire_calls);
    let acquire_job_lease = Arc::new(move |request: JobLeaseRequest| -> BoxFuture<Result<cheaper_tools::resources::ResourceLease, ToolError>> {
        let rm = rm.clone();
        let acquired = Arc::clone(&acquired);
        Box::pin(async move {
            acquired.lock().unwrap().push(request.job_id.clone());
            let claims = request
                .job
                .manifest
                .clone()
                .map(|m| {
                    let mut c = m.reads;
                    c.extend(m.writes);
                    c.extend(m.produces);
                    c
                })
                .unwrap_or_default();
            rm.request(cheaper_tools::resources::ResourceRequest {
                owner_id: request.job_id,
                kind: cheaper_tools::resources::LeaseKind::Job,
                signal: Some(request.signal),
                parent_lease: None,
                access: None,
                prepare: Box::new(move || {
                    cheaper_tools::resources::prepared(
                        cheaper_tools::resources::LeasePreparation {
                            claims,
                            read_strictness: cheaper_tools::resources::ReadStrictness::Record,
                        },
                    )
                }),
                can_start: None,
                start: |lease| Ok(lease),
            })
            .await
            .map_err(|reason| -> ToolError {
                Box::new(reason)
            })
        })
    });

    let rm2 = resources.clone();
    let released = Arc::clone(&release_calls);
    let release_job_lease = Arc::new(move |lease: cheaper_tools::resources::ResourceLease| {
        released.lock().unwrap().push(lease);
        rm2.release(lease);
    });

    let aborts = Arc::clone(&abort_calls);
    let abort = Arc::new(move |reason: &cheaper_tools::operation::AbortReason| {
        aborts.lock().unwrap().push(reason.clone());
    });

    EngineHarness {
        ports: WorkflowEnginePorts {
            worker,
            tool,
            acquire_job_lease,
            release_job_lease,
            abort,
            execution_metrics: Arc::new(|_| None),
        },
        resources,
        acquire_calls,
        abort_calls,
        release_calls,
    }
}

/// default harness — worker は `"worker"` を返し、tool は jobId text。
pub fn default_harness() -> EngineHarness {
    harness(
        Arc::new(FnExecutor(|_ctx: WorkflowExecutionContext| async {
            Ok(WorkflowExecutorResult {
                result: json!("worker"),
                meta: None,
            })
        })),
        Arc::new(FnExecutor(|ctx: WorkflowExecutionContext| async move {
            Ok(tool_result(&ctx.job_id))
        })),
    )
}

/// `independent(...ids)`。
pub async fn independent(ids: &[&str]) -> Arc<cheaper_workflow::CompiledWorkflow> {
    let mut raw = definition();
    for id in ids {
        raw["jobs"][*id] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    }
    let fixtures = FixtureSet::new();
    fixtures
        .compiler()
        .compile(&raw, None)
        .await
        .expect("compiles")
}

/// `expect_err` 相当 — ただし `T: Debug` を要求しない
/// (`CompiledWorkflow`/`LoadedWorkflow`/`Arc<RunArtifactStore>` は Debug
/// を持たない)。
pub fn unwrap_err<T, E>(result: Result<T, E>) -> E {
    match result {
        Err(e) => e,
        Ok(_) => panic!("expected error, got Ok"),
    }
}

/// `toMatchObject({code})` 相当 — `ToolError` から canonical `ErrorCode`
/// を取り出す。scheduler/executor 経路の `AbortReason` wrap や
/// `ResourceError`(core)wrap を `source()` chain で透過する。
pub fn err_code(e: &ToolError) -> Option<cheaper_core::errors::ErrorCode> {
    let mut current: &(dyn std::error::Error + 'static) = e.as_ref();
    let mut reason_code: Option<String> = None;
    loop {
        if let Some(c) = current.downcast_ref::<cheaper_core::errors::CheaperError>() {
            return Some(c.code());
        }
        if reason_code.is_none() {
            if let Some(r) = current.downcast_ref::<cheaper_tools::operation::AbortReason>() {
                reason_code = r.code().map(|s| s.to_string());
            }
        }
        match current.source() {
            Some(source) => current = source,
            None => break,
        }
    }
    reason_code.and_then(|code| {
        serde_json::from_value::<cheaper_core::errors::ErrorCode>(serde_json::Value::String(code))
            .ok()
    })
}

/// `flush()` — spawned task に microtask 相当の実行機会を与える。
pub async fn flush() {
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
}

/// paused clock を `ms` 進めて yield(fake timer + microtask flush)。
#[allow(dead_code)]
pub async fn advance(ms: u64) {
    tokio::time::advance(std::time::Duration::from_millis(ms)).await;
    flush().await;
}
