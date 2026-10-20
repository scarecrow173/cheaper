//! Port of `packages/workflow/src/schema.ts`.

use std::sync::{Arc, LazyLock};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::worker::schema::{compile_schema, CompiledSchema, SchemaKind};
use cheaper_core::worker::validate::WorkerDefinition;
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use crate::bindings::{parse_binding, scan_bindings, validate_pointer_schema};
use crate::dag::compile_dag;
use crate::json::{clone_json, has_own};
use crate::types::{
    parse_job, DependencyEdge, JobValidation, ScopePath, WorkflowDefinition, WorkflowJob,
};
use crate::WorkflowCompilerPorts;

fn named_string() -> Value {
    json!({ "type": "string", "minLength": 1 })
}

fn input_binding() -> Value {
    json!({
        "type": "object",
        "required": ["$input"],
        "additionalProperties": false,
        "properties": { "$input": named_string() },
    })
}

fn from_binding() -> Value {
    json!({
        "type": "object",
        "required": ["$from", "path"],
        "additionalProperties": false,
        "properties": { "$from": named_string(), "path": { "type": "string" } },
    })
}

fn literal_binding() -> Value {
    json!({
        "type": "object",
        "required": ["$literal"],
        "additionalProperties": false,
        "properties": { "$literal": true },
    })
}

fn declaration() -> Value {
    json!({
        "type": "object",
        "required": ["path", "scope"],
        "additionalProperties": false,
        "properties": {
            "path": { "oneOf": [named_string(), input_binding()] },
            "scope": { "enum": ["exact", "subtree"] },
        },
    })
}

fn resources() -> Value {
    let declaration = declaration();
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "reads": { "type": "array", "items": declaration },
            "writes": { "type": "array", "items": declaration },
            "produces": { "type": "array", "items": declaration },
        },
    })
}

/// `when` predicate — `{left, op, right?}`。operand は任意 JSON
/// (binding wrapper 含む)。`right` は `exists` では未使用のため
/// optional。
fn when_predicate() -> Value {
    json!({
        "oneOf": [
            // `exists` は `right` 禁止。
            {
                "type": "object",
                "required": ["left", "op"],
                "additionalProperties": false,
                "properties": {
                    "left": true,
                    "op": { "const": "exists" },
                },
            },
            // それ以外の op は `right` 必須。
            {
                "type": "object",
                "required": ["left", "op", "right"],
                "additionalProperties": false,
                "properties": {
                    "left": true,
                    "op": { "enum": ["eq", "ne", "gt", "lt"] },
                    "right": true,
                },
            },
        ],
    })
}

fn job_schema(kind: &str) -> Value {
    json!({
        "type": "object",
        "required": ["kind", kind, "input"],
        "additionalProperties": false,
        "properties": {
            "kind": { "const": kind },
            kind: named_string(),
            "input": true,
            "needs": { "type": "array", "items": named_string() },
            "resources": resources(),
            "when": when_predicate(),
        },
    })
}

/// `WORKFLOW_JSON_SCHEMA`。`static` 共有のため `LazyLock`。
pub static WORKFLOW_JSON_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["schemaVersion", "inputs", "jobs", "outputs"],
        "additionalProperties": false,
        "properties": {
            "schemaVersion": { "const": 1 },
            "inputs": {
                "type": "object",
                "propertyNames": named_string(),
                "additionalProperties": { "type": "object" },
            },
            "jobs": {
                "type": "object",
                "propertyNames": named_string(),
                "additionalProperties": { "oneOf": [job_schema("tool"), job_schema("worker")] },
            },
            "outputs": {
                "type": "object",
                "propertyNames": named_string(),
                "additionalProperties": {
                    "type": "object",
                    "required": ["schema", "value"],
                    "additionalProperties": false,
                    "properties": {
                        "schema": { "type": "object" },
                        "value": { "oneOf": [input_binding(), from_binding(), literal_binding()] },
                    },
                },
            },
        },
    })
});

static DEFINITION_SCHEMA: LazyLock<CompiledSchema> = LazyLock::new(|| {
    let schema = WORKFLOW_JSON_SCHEMA
        .as_object()
        .expect("workflow schema is an object")
        .clone();
    compile_schema(schema, SchemaKind::Input).expect("static workflow schema compiles")
});

/// `ValidatedDefinition`。
pub struct ValidatedDefinition {
    /// `cloneJson` 済み definition snapshot。
    pub definition: WorkflowDefinition,
    pub inputs: Vec<(String, CompiledSchema)>,
    pub outputs: Vec<(String, CompiledSchema)>,
    pub jobs: Vec<(String, JobValidation)>,
    pub edges: Vec<DependencyEdge>,
}

impl ValidatedDefinition {
    pub fn job_validation(&self, id: &str) -> Option<&JobValidation> {
        self.jobs.iter().find(|(k, _)| k == id).map(|(_, v)| v)
    }
}

/// `compileDefinitionSchema` — compile error を
/// `invalid workflow {kind} schema`(cause 付き)で wrap。
fn compile_definition_schema(
    schema: &Value,
    kind: SchemaKind,
) -> Result<CompiledSchema, CheaperError> {
    compile_schema(schema.as_object().cloned().unwrap_or_default(), kind).map_err(|cause| {
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("invalid workflow {} schema", kind.as_str()),
        )
        .with_cause(cause)
    })
}

/// `workerSnapshot` — validated WorkerDefinition の snapshot。
/// compiled schema は `Arc` で共有する(`CompiledSchema` は immutable
/// で、呼び出し側 metadata の mutation を遮断できるため再 compile
/// は不要)。
fn worker_snapshot(worker: &WorkerDefinition) -> Arc<WorkerDefinition> {
    Arc::new(WorkerDefinition {
        name: worker.name.clone(),
        description: worker.description.clone(),
        tools: worker.tools.clone(),
        cheaper: worker.cheaper.clone(),
        instructions: worker.instructions.clone(),
        source_path: worker.source_path.clone(),
        origin: worker.origin,
        input_schema: worker.input_schema.clone(),
        output_schema: Arc::clone(&worker.output_schema),
    })
}

/// `validateDefinition`。port 呼出し(`workers.get` / `tools.get`)の
/// 任意 throw をそのまま伝播するため error 型は `ToolError`。
pub fn validate_definition(
    raw: &Value,
    ports: &WorkflowCompilerPorts,
) -> Result<ValidatedDefinition, ToolError> {
    let snapshot = clone_json(raw, ErrorCode::InvalidRequest)?;
    if let Err(errors) = DEFINITION_SCHEMA.validate(&snapshot) {
        return Err(Box::new(
            CheaperError::new(ErrorCode::InvalidRequest, "invalid Workflow JSON v1").with_details(
                {
                    let mut details = Map::new();
                    details.insert(
                        "errors".to_string(),
                        serde_json::to_value(&errors).unwrap_or(Value::Null),
                    );
                    details
                },
            ),
        ));
    }
    let definition = WorkflowDefinition::new(snapshot);
    let mut inputs: Vec<(String, CompiledSchema)> = Vec::new();
    let mut outputs: Vec<(String, CompiledSchema)> = Vec::new();
    let mut jobs: Vec<(String, JobValidation)> = Vec::new();
    let mut edges: Vec<DependencyEdge> = Vec::new();

    for (name, schema) in definition.inputs() {
        let validator = compile_definition_schema(schema, SchemaKind::Input)?;
        if let Some(default) = schema.get("default") {
            if validator.validate(default).is_err() {
                return Err(Box::new(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    format!("invalid input default: {name}"),
                )));
            }
        }
        inputs.push((name.clone(), validator));
    }

    for (id, job_value) in definition.jobs() {
        let job = parse_job(job_value);
        match job.kind {
            crate::types::JobKind::Tool => {
                let tool = ports.tools.get(&job.name).ok_or_else(|| {
                    Box::new(CheaperError::new(
                        ErrorCode::InvalidRequest,
                        format!("unknown tool: {}", job.name),
                    )) as ToolError
                })?;
                jobs.push((
                    id.clone(),
                    JobValidation {
                        input: Some(Arc::new(compile_definition_schema(
                            &clone_json(&tool.input_schema, ErrorCode::InvalidRequest)?,
                            SchemaKind::Input,
                        )?)),
                        output: Arc::new(compile_definition_schema(
                            &clone_json(&tool.output_schema, ErrorCode::InvalidRequest)?,
                            SchemaKind::Output,
                        )?),
                        worker: None,
                    },
                ));
            }
            crate::types::JobKind::Worker => {
                let metadata = ports.workers.get(&job.name)?.ok_or_else(|| {
                    Box::new(CheaperError::new(
                        ErrorCode::InvalidRequest,
                        format!("unknown worker: {}", job.name),
                    )) as ToolError
                })?;
                for tool in metadata.tools.iter() {
                    if ports.tools.get(tool.as_str()).is_none() {
                        return Err(Box::new(CheaperError::new(
                            ErrorCode::InvalidRequest,
                            format!("unknown worker capability tool: {}", tool.as_str()),
                        )));
                    }
                }
                let worker = worker_snapshot(metadata);
                jobs.push((
                    id.clone(),
                    JobValidation {
                        input: worker.input_schema.clone(),
                        output: Arc::clone(&worker.output_schema),
                        worker: Some(worker),
                    },
                ));
            }
        }
    }

    // `references(value, consumer)`(free fn — closure capture では
    // `edges` の borrow が既存 push と衝突するため)。
    for (id, job_value) in definition.jobs() {
        let job: WorkflowJob = parse_job(job_value);
        references(&definition, &jobs, &job.input, Some(id), &mut edges)?;
        // `when` 内の `$from`/`$input` も参照解決・dep edge 推論の
        // 対象(producer 完了前に条件評価されないよう edge を積む)。
        if let Some(when) = &job.when {
            references(&definition, &jobs, when, Some(id), &mut edges)?;
        }
        if let Some(needs) = &job.needs {
            for from in needs {
                edges.push(DependencyEdge {
                    from: from.clone(),
                    to: id.clone(),
                });
            }
        }
        if let Some(resources) = &job.resources {
            for declarations in [
                resources.reads.as_deref(),
                resources.writes.as_deref(),
                resources.produces.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                for resource in declarations {
                    if let ScopePath::Input(name) = &resource.path {
                        // `{ $input }` path object — consumer 無しで
                        // `references` を呼ぶ(TS と同じく edge は積まない)。
                        references(
                            &definition,
                            &jobs,
                            &json!({ "$input": name }),
                            None,
                            &mut edges,
                        )?;
                    }
                }
            }
        }
    }

    for (name, output) in definition.outputs() {
        let Some(output_obj) = output.as_object() else {
            continue;
        };
        let value = output_obj.get("value").cloned().unwrap_or(Value::Null);
        parse_binding(&value)?;
        references(&definition, &jobs, &value, None, &mut edges)?;
        let schema = output_obj
            .get("schema")
            .cloned()
            .unwrap_or(Value::Object(Map::new()));
        outputs.push((
            name.clone(),
            compile_definition_schema(&schema, SchemaKind::Output)?,
        ));
    }

    let job_ids: Vec<String> = definition.jobs().keys().cloned().collect();
    let dag = compile_dag(&job_ids, &edges)?;
    Ok(ValidatedDefinition {
        definition,
        inputs,
        outputs,
        jobs,
        edges: dag.edges,
    })
}

/// TS の `references(value, consumer)`。
fn references(
    definition: &WorkflowDefinition,
    jobs: &[(String, JobValidation)],
    value: &Value,
    consumer: Option<&str>,
    edges: &mut Vec<DependencyEdge>,
) -> Result<(), ToolError> {
    let bindings = scan_bindings(value)?;
    for name in &bindings.inputs {
        if !has_own(definition.inputs(), name) {
            return Err(Box::new(CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("unknown input reference: {name}"),
            )));
        }
    }
    for binding in &bindings.from {
        let producer = jobs
            .iter()
            .find(|(id, _)| id == &binding.from)
            .map(|(_, v)| &v.output);
        let Some(producer) = producer else {
            return Err(Box::new(CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("unknown job reference: {}", binding.from),
            )));
        };
        validate_pointer_schema(&producer.schema, &binding.path)?;
        if let Some(consumer) = consumer {
            edges.push(DependencyEdge {
                from: binding.from.clone(),
                to: consumer.to_string(),
            });
        }
    }
    Ok(())
}
