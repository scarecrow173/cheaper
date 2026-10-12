//! Port of `packages/workflow/src/types.ts` (+ `engine.ts` の port 型の
//! うち compile/inference 側)。serde 上の JSON 表現は TS と同一 key 名・
//! 同一 wire shape に揃える。

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cheaper_core::resources::types::{ResourceClaim, ResourceManifest, ResourceScope};
use cheaper_core::worker::schema::CompiledSchema;
use cheaper_core::worker::validate::WorkerDefinition;
use cheaper_tools::ToolError;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::JsonValue;

/// `JsonSchema` = `Record<string, unknown>`。
pub type JsonSchema = Map<String, Value>;

/// `{ $from: string; path: string }`。
#[derive(Debug, Clone, PartialEq)]
pub struct FromBinding {
    pub from: String,
    pub path: String,
}

/// `Binding` union。`{ $literal }` は値をそのまま保持する。
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    Input(String),
    From(FromBinding),
    Literal(Value),
}

/// `ScopeDeclaration["path"]` = `string | InputBinding`。
#[derive(Debug, Clone, PartialEq)]
pub enum ScopePath {
    Literal(String),
    Input(String),
}

/// workflow `resources` の宣言 bucket 要素。
#[derive(Debug, Clone, PartialEq)]
pub struct ScopeDeclaration {
    pub path: ScopePath,
    pub scope: ResourceScope,
}

/// `WorkflowResources`。bucket の `undefined`/`[]` 区別は Option で保持
/// する(推論の underclaim/omitted 判定が区別に依存する)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkflowResources {
    pub reads: Option<Vec<ScopeDeclaration>>,
    pub writes: Option<Vec<ScopeDeclaration>>,
    pub produces: Option<Vec<ScopeDeclaration>>,
}

/// `WorkflowJob["kind"]`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    Tool,
    Worker,
}

impl JobKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Worker => "worker",
        }
    }
}

/// `WorkflowJob` の型付き view。definition object の `{kind, tool|worker,
/// input, needs?, resources?}` を解釈済みの形。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowJob {
    pub kind: JobKind,
    /// `kind == Tool` なら tool 名、`Worker` なら worker 名。
    pub name: String,
    pub input: Value,
    pub needs: Option<Vec<String>>,
    pub resources: Option<WorkflowResources>,
}

/// `WorkflowOutput`。`value` は binding object を保持した raw JSON。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowOutput {
    pub schema: JsonSchema,
    pub value: Value,
}

/// `WorkflowDefinition` — `validateDefinition` 済みの snapshot(JSON)。
/// cloneJson 済みのため全 object key は code-unit 順。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct WorkflowDefinition {
    raw: Value,
}

impl WorkflowDefinition {
    /// 検証済み object snapshot を包む。`raw` は object であること
    /// (`validate_definition` が保証)。
    pub(crate) fn new(raw: Value) -> Self {
        Self { raw }
    }

    pub fn raw(&self) -> &Value {
        &self.raw
    }

    /// `definition.inputs`(object)。
    pub fn inputs(&self) -> &Map<String, Value> {
        self.raw["inputs"].as_object().expect("validated inputs")
    }

    /// `definition.jobs`(object、key 順 = definition 記述順)。
    pub fn jobs(&self) -> &Map<String, Value> {
        self.raw["jobs"].as_object().expect("validated jobs")
    }

    /// `definition.outputs`(object)。
    pub fn outputs(&self) -> &Map<String, Value> {
        self.raw["outputs"].as_object().expect("validated outputs")
    }

    /// `definition.jobs[id]` の型付き view。
    pub fn job(&self, id: &str) -> Option<WorkflowJob> {
        self.jobs().get(id).map(parse_job)
    }
}

/// definition object の `jobs.*` entry を `WorkflowJob` へ写す。
/// caller は schema 検証済みであることを前提にする(`panic` しない —
/// 検証前の直接利用には到達しない)。
pub(crate) fn parse_job(value: &Value) -> WorkflowJob {
    let obj = value.as_object().expect("validated job object");
    let kind = match obj["kind"].as_str() {
        Some("tool") => JobKind::Tool,
        _ => JobKind::Worker,
    };
    let name_key = kind.as_str();
    let resources = obj.get("resources").map(parse_resources);
    WorkflowJob {
        kind,
        name: obj[name_key].as_str().unwrap_or_default().to_string(),
        input: obj.get("input").cloned().unwrap_or(Value::Null),
        needs: obj.get("needs").map(|v| {
            v.as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| item.as_str().unwrap_or_default().to_string())
                        .collect()
                })
                .unwrap_or_default()
        }),
        resources,
    }
}

/// `WorkflowResources` object を型付きに写す。bucket は `reads`・
/// `writes`・`produces` のみ(schema で additionalProperties: false)。
fn parse_resources(value: &Value) -> WorkflowResources {
    let obj = value.as_object().expect("validated resources object");
    let bucket = |key: &str| -> Option<Vec<ScopeDeclaration>> {
        obj.get(key).map(|v| {
            v.as_array()
                .map(|items| items.iter().map(parse_scope_declaration).collect())
                .unwrap_or_default()
        })
    };
    WorkflowResources {
        reads: bucket("reads"),
        writes: bucket("writes"),
        produces: bucket("produces"),
    }
}

fn parse_scope_declaration(value: &Value) -> ScopeDeclaration {
    let obj = value.as_object().expect("validated scope declaration");
    let path = match obj.get("path") {
        Some(Value::String(s)) => ScopePath::Literal(s.clone()),
        // `{ $input: name }` — schema で shape は保証済み。
        Some(Value::Object(binding)) => {
            ScopePath::Input(binding["$input"].as_str().unwrap_or_default().to_string())
        }
        _ => ScopePath::Literal(String::new()),
    };
    let scope = match obj["scope"].as_str() {
        Some("subtree") => ResourceScope::Subtree,
        _ => ResourceScope::Exact,
    };
    ScopeDeclaration { path, scope }
}

/// `DependencyEdge`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
}

/// `WorkerCatalog` port。`get` は loader 済み metadata を返し、worker を
/// 実行しない。T7 の CLI adapter は worker 解決失敗を Err で再現する
/// (TS 版は `failures.get(name)` を throw する)。
pub trait WorkerCatalog: Send + Sync {
    fn get(&self, name: &str) -> Result<Option<&WorkerDefinition>, ToolError>;
}

/// `ToolCatalog` port(TS interface の `get` のみ — workflow が使う
/// surface)。`cheaper_tools::catalog::ToolCatalog` には adapter impl を
/// 提供する。
pub trait ToolCatalog: Send + Sync {
    fn get(&self, name: &str) -> Option<ToolMetadata>;
}

/// `ToolMetadata`。`resources` は省略可能(TS `resources?`)。
pub struct ToolMetadata {
    pub name: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub resources: Option<crate::ResourceDeclarationFn>,
}

/// `resources?(input)` callback 型(`cheaper-tools` の
/// `ResourcesDeclaration` と同一形)。
pub type ResourceDeclarationFn = cheaper_tools::catalog::ResourcesDeclaration;

impl ToolCatalog for cheaper_tools::catalog::ToolCatalog {
    fn get(&self, name: &str) -> Option<ToolMetadata> {
        let meta = cheaper_tools::catalog::ToolCatalog::get(self, name)?;
        Some(ToolMetadata {
            name: meta.name.as_str().to_string(),
            input_schema: meta.input_schema,
            output_schema: meta.output_schema,
            resources: Some(meta.resources),
        })
    }
}

/// `ProcessResourceDeclaration`(process job の policy-aware 宣言)。
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessResourceDeclaration {
    pub task: String,
    pub inputs: Vec<cheaper_core::resources::normalization::ScopeDeclaration>,
    pub outputs: Vec<cheaper_core::resources::normalization::ScopeDeclaration>,
}

/// `ResourceConflict`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceConflict {
    pub jobs: [String; 2],
    pub claims: [ResourceClaim; 2],
}

/// `ResourceInferenceDiagnostic` union。
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceInferenceDiagnostic {
    Fallback { job: String, reason: String },
    Dependency(ResourceConflict),
    Conflict(ResourceConflict),
}

impl Serialize for ResourceInferenceDiagnostic {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Fallback { job, reason } => {
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("kind", "fallback")?;
                m.serialize_entry("job", job)?;
                m.serialize_entry("reason", reason)?;
                m.end()
            }
            Self::Dependency(c) | Self::Conflict(c) => {
                let kind = if matches!(self, Self::Dependency(_)) {
                    "dependency"
                } else {
                    "conflict"
                };
                let mut m = serializer.serialize_map(Some(3))?;
                m.serialize_entry("kind", kind)?;
                m.serialize_entry("jobs", &c.jobs)?;
                m.serialize_entry("claims", &c.claims)?;
                m.end()
            }
        }
    }
}

/// `ResourceInferenceJob`。
#[derive(Clone)]
pub struct ResourceInferenceJob {
    pub process: Option<ProcessResourceDeclaration>,
    pub job: WorkflowJob,
    pub worker: Option<Arc<WorkerDefinition>>,
}

/// `ResourceInferenceResult`。`conflicts`/`diagnostics` は port が省略
/// できる(TS optional)。`manifests` は job 順を保持する pair 列。
#[derive(Debug)]
pub struct ResourceInferenceResult {
    pub manifests: Vec<(String, ResourceManifest)>,
    pub edges: Vec<DependencyEdge>,
    pub conflicts: Option<Vec<ResourceConflict>>,
    pub diagnostics: Option<Vec<ResourceInferenceDiagnostic>>,
}

/// `ResourceInference` port。`infer` は失敗をそのまま伝播する
/// (未実装を空辺の成功に変換しない — `ToolError` = TS throw 相当)。
pub trait ResourceInference: Send + Sync {
    fn infer<'a>(
        &'a self,
        jobs: &'a [(String, ResourceInferenceJob)],
    ) -> Pin<Box<dyn Future<Output = Result<ResourceInferenceResult, ToolError>> + Send + 'a>>;
}

/// `JobValidation`(schema 検証済み job metadata)。
pub struct JobValidation {
    pub input: Option<Arc<CompiledSchema>>,
    pub output: Arc<CompiledSchema>,
    /// `workerSnapshot` — validation port の schema を `Arc` 共有した
    /// snapshot(`CompiledSchema` は immutable で mutation を遮断できる)。
    pub worker: Option<Arc<WorkerDefinition>>,
}

/// `CompiledJob`(`WorkflowJob` + `manifest?`)。
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledJob {
    pub kind: JobKind,
    pub name: String,
    pub input: Value,
    pub needs: Option<Vec<String>>,
    pub resources: Option<WorkflowResources>,
    pub manifest: Option<ResourceManifest>,
}

impl Serialize for CompiledJob {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // `{kind, tool|worker, input, needs?, resources?, manifest?}`。
        let mut len = 3;
        if self.needs.is_some() {
            len += 1;
        }
        if self.resources.is_some() {
            len += 1;
        }
        if self.manifest.is_some() {
            len += 1;
        }
        let mut m = serializer.serialize_map(Some(len))?;
        m.serialize_entry("kind", self.kind.as_str())?;
        m.serialize_entry(self.kind.as_str(), &self.name)?;
        m.serialize_entry("input", &self.input)?;
        if let Some(needs) = &self.needs {
            m.serialize_entry("needs", needs)?;
        }
        if let Some(resources) = &self.resources {
            m.serialize_entry("resources", &SerializeWorkflowResources(resources))?;
        }
        if let Some(manifest) = &self.manifest {
            m.serialize_entry("manifest", manifest)?;
        }
        m.end()
    }
}

/// `WorkflowResources` の JSON 表現(`{reads?, writes?, produces?}`、
/// materialize 済み path は `{path: string, scope}`、未解決 input
/// binding は `{path: {$input}, scope}`)。
pub(crate) struct SerializeWorkflowResources<'a>(pub &'a WorkflowResources);

impl Serialize for SerializeWorkflowResources<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let r = self.0;
        let len = r.reads.is_some() as usize
            + r.writes.is_some() as usize
            + r.produces.is_some() as usize;
        let mut m = serializer.serialize_map(Some(len))?;
        let mut bucket =
            |key: &str, decls: &Option<Vec<ScopeDeclaration>>| -> Result<(), S::Error> {
                if let Some(decls) = decls {
                    let values: Vec<Value> = decls
                        .iter()
                        .map(|d| {
                            let path = match &d.path {
                                ScopePath::Literal(p) => Value::String(p.clone()),
                                ScopePath::Input(name) => {
                                    serde_json::json!({ "$input": name })
                                }
                            };
                            serde_json::json!({ "path": path, "scope": d.scope })
                        })
                        .collect();
                    m.serialize_entry(key, &values)?;
                }
                Ok(())
            };
        bucket("reads", &r.reads)?;
        bucket("writes", &r.writes)?;
        bucket("produces", &r.produces)?;
        m.end()
    }
}

/// `WorkflowJobState`。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowJobState {
    pub terminal: bool,
    /// `result !== undefined` のみ `Some`(`null` result は区別される)。
    pub result: Option<Value>,
}

/// `CompiledWorkflow`。`order` が job 実行順の正本。
pub struct CompiledWorkflow {
    /// `cloneJson(inputs)` — sorted key の公開 snapshot。
    pub inputs: Map<String, Value>,
    /// compiled jobs(`jobs` Record、key は code-unit sort 済み挿入順)。
    pub jobs: Vec<(String, CompiledJob)>,
    /// `cloneJson(definition.outputs)` — `{schema, value}` 形の raw JSON。
    pub outputs: Map<String, Value>,
    pub order: Vec<String>,
    pub edges: Vec<DependencyEdge>,
    pub conflicts: Option<Vec<ResourceConflict>>,
    pub diagnostics: Option<Vec<ResourceInferenceDiagnostic>>,
    /// compiler が保持する validation 側(internal)。
    pub(crate) definition: WorkflowDefinition,
    pub(crate) validated_outputs: Vec<(String, CompiledSchema)>,
    pub(crate) validated_jobs: Vec<(String, JobValidation)>,
    /// compile 時点の materialized inputs(private snapshot)。
    pub(crate) materialized_inputs: Map<String, Value>,
}

impl CompiledWorkflow {
    pub fn schema_version(&self) -> u32 {
        1
    }

    /// `jobs[id]`。
    pub fn job(&self, id: &str) -> Option<&CompiledJob> {
        self.jobs.iter().find(|(k, _)| k == id).map(|(_, v)| v)
    }

    /// `jobs` の mutable access(test が manifest を注入する seam)。
    pub fn job_mut(&mut self, id: &str) -> Option<&mut CompiledJob> {
        self.jobs.iter_mut().find(|(k, _)| k == id).map(|(_, v)| v)
    }

    pub(crate) fn job_validation(&self, id: &str) -> Option<&JobValidation> {
        self.validated_jobs
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| v)
    }

    pub(crate) fn output_validation(&self, name: &str) -> Option<&CompiledSchema> {
        self.validated_outputs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    }
}

/// `WorkflowInterface` — `inputs`/`outputs` は key 名 sort 済み pair 列
/// (code-unit 順。`Object.keys().sort()` と同じ)。serialize は object。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowInterface {
    pub inputs: Vec<(String, InterfaceInput)>,
    pub outputs: Vec<(String, InterfaceOutput)>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InterfaceInput {
    pub schema: JsonSchema,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InterfaceOutput {
    pub schema: JsonSchema,
}

impl Serialize for WorkflowInterface {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut m = serializer.serialize_map(Some(3))?;
        m.serialize_entry("schemaVersion", &1)?;
        m.serialize_entry(
            "inputs",
            &OrderedPairs(self.inputs.iter().map(|(k, v)| (k.as_str(), v)).collect()),
        )?;
        m.serialize_entry(
            "outputs",
            &OrderedPairs(self.outputs.iter().map(|(k, v)| (k.as_str(), v)).collect()),
        )?;
        m.end()
    }
}

struct OrderedPairs<'a, T: Serialize>(Vec<(&'a str, &'a T)>);

impl<T: Serialize> Serialize for OrderedPairs<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut m = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            m.serialize_entry(k, *v)?;
        }
        m.end()
    }
}

/// `materializeJobInput` 用の results map 型(TS `ReadonlyMap`)。
pub type JobResults<'a> = &'a HashMap<String, JsonValue>;

/// `materializeOutput(s)` 用の states map 型。
pub type JobStates<'a> = &'a HashMap<String, WorkflowJobState>;
