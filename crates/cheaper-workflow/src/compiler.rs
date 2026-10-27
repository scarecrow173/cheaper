//! Port of `packages/workflow/src/compiler.ts`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::code_unit_compare;
use cheaper_core::worker::validate::validate_worker_input;
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use crate::bindings::{materialize_bindings, scan_bindings, MaterializeOptions};
use crate::dag::compile_dag;
use crate::interface::project_workflow_interface;
use crate::json::{clone_json, has_own, is_object};
use crate::schema::{validate_definition, ValidatedDefinition};
use crate::types::{
    parse_job, CompiledJob, CompiledWorkflow, JobValidation, ResourceInference,
    ResourceInferenceJob, ScopePath, ToolCatalog, WorkerCatalog, WorkflowDefinition,
    WorkflowInterface, WorkflowJobState, WorkflowResources,
};

/// `WorkflowCompilerPorts`。tool/resource は借用 catalog、worker は
/// per-document 解決済み index の共有所有権を持つ。
#[derive(Clone)]
pub struct WorkflowCompilerPorts<'a> {
    pub tools: &'a dyn ToolCatalog,
    pub workers: Arc<dyn WorkerCatalog>,
    pub resources: Option<&'a dyn ResourceInference>,
}

/// `validateJobInput`。
fn validate_job_input(metadata: &JobValidation, input: &Value) -> Result<(), CheaperError> {
    if let Some(worker) = &metadata.worker {
        validate_worker_input(worker, input)
    } else if let Some(schema) = &metadata.input {
        schema.validate(input).map_err(|errors| {
            CheaperError::new(
                ErrorCode::InvalidInput,
                "input does not match tool input schema",
            )
            .with_details({
                let mut details = Map::new();
                details.insert(
                    "errors".to_string(),
                    serde_json::to_value(&errors).unwrap_or(Value::Null),
                );
                details
            })
        })
    } else {
        Ok(())
    }
}

/// `invocationInputs`: provided inputs の validate + default 適用 +
/// snapshot。戻り値の key 順は `Object.keys(definition.inputs).sort()`。
fn invocation_inputs(
    validated: &ValidatedDefinition,
    raw: &Value,
) -> Result<Map<String, Value>, CheaperError> {
    let provided = clone_json(raw, ErrorCode::InvalidInput)?;
    if !is_object(&provided) {
        return Err(CheaperError::new(
            ErrorCode::InvalidInput,
            "workflow inputs must be an object",
        ));
    }
    for key in provided.as_object().expect("checked").keys() {
        if !has_own(validated.definition.inputs(), key) {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                format!("unknown workflow input: {key}"),
            ));
        }
    }
    let mut names: Vec<&String> = validated.definition.inputs().keys().collect();
    names.sort_by(|a, b| code_unit_compare(a, b));
    let mut materialized = Map::with_capacity(names.len());
    for name in names {
        let schema = &validated.definition.inputs()[name];
        // `hasOwn(schema, "default")` — schema は validation 済みの
        // object(definition schema が `{type:"object"}` を要求)。
        let has_default = schema.as_object().is_some_and(|o| has_own(o, "default"));
        if !has_own(provided.as_object().expect("checked"), name) && !has_default {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                format!("required workflow input: {name}"),
            ));
        }
        let source = if has_own(provided.as_object().expect("checked"), name) {
            &provided[name]
        } else {
            &schema["default"]
        };
        let value = clone_json(source, ErrorCode::InvalidInput)?;
        let validator = validated
            .inputs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
            .expect("validated input exists");
        if let Err(errors) = validator.validate(&value) {
            return Err(CheaperError::new(
                ErrorCode::InvalidInput,
                format!("invalid workflow input: {name}"),
            )
            .with_details({
                let mut details = Map::new();
                details.insert(
                    "errors".to_string(),
                    serde_json::to_value(&errors).unwrap_or(Value::Null),
                );
                details
            }));
        }
        materialized.insert(name.clone(), value);
    }
    Ok(materialized)
}

/// `materializeResources`: `{ $input }` path を materialize して文字列
/// path に固定する。
fn materialize_resources(
    resources: &WorkflowResources,
    inputs: &Map<String, Value>,
) -> Result<WorkflowResources, CheaperError> {
    let empty_results = HashMap::new();
    let bucket = |declarations: &Option<Vec<crate::types::ScopeDeclaration>>| {
        declarations
            .as_ref()
            .map(|decls| {
                decls
                    .iter()
                    .map(|decl| {
                        let path = match &decl.path {
                            ScopePath::Literal(p) => Value::String(p.clone()),
                            ScopePath::Input(name) => materialize_bindings(
                                &json!({ "$input": name }),
                                inputs,
                                &empty_results,
                                MaterializeOptions::default(),
                            )?,
                        };
                        let Value::String(path) = path else {
                            return Err(CheaperError::new(
                                ErrorCode::InvalidInput,
                                "resource path must resolve to a string",
                            ));
                        };
                        Ok(crate::types::ScopeDeclaration {
                            path: ScopePath::Literal(path),
                            scope: decl.scope,
                        })
                    })
                    .collect::<Result<Vec<_>, CheaperError>>()
            })
            .transpose()
    };
    Ok(WorkflowResources {
        reads: bucket(&resources.reads)?,
        writes: bucket(&resources.writes)?,
        produces: bucket(&resources.produces)?,
    })
}

impl CompiledWorkflow {
    /// `materializeJobInput(jobId, results)`。`skipped` は
    /// condition-skipped producer の job id 集合(該当 `$from` は
    /// `null` に materialize される)。
    pub fn materialize_job_input(
        &self,
        job_id: &str,
        results: &HashMap<String, Value>,
        skipped: &HashSet<String>,
    ) -> Result<Value, CheaperError> {
        let job = self.definition.job(job_id);
        let metadata = self.job_validation(job_id);
        match (job, metadata) {
            (Some(job), Some(metadata)) => {
                let input = materialize_bindings(
                    &job.input,
                    &self.materialized_inputs,
                    results,
                    MaterializeOptions {
                        skipped_from: Some(skipped),
                        ..Default::default()
                    },
                )?;
                validate_job_input(metadata, &input)?;
                Ok(input)
            }
            _ => Err(CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("unknown compiled job: {job_id}"),
            )),
        }
    }

    /// `validateJobOutput(jobId, value)`。
    pub fn validate_job_output(&self, job_id: &str, value: &Value) -> Result<Value, CheaperError> {
        let Some(metadata) = self.job_validation(job_id) else {
            return Err(CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("unknown compiled job: {job_id}"),
            ));
        };
        let snapshot = clone_json(value, ErrorCode::InvalidOutput)?;
        metadata.output.validate(&snapshot).map_err(|errors| {
            CheaperError::new(
                ErrorCode::InvalidOutput,
                format!("invalid job output: {job_id}"),
            )
            .with_details({
                let mut details = Map::new();
                details.insert(
                    "errors".to_string(),
                    serde_json::to_value(&errors).unwrap_or(Value::Null),
                );
                details
            })
        })?;
        Ok(snapshot)
    }

    /// `#results(states)` — 全 job が terminal で `result` 持ちのみ収集。
    fn results_of(
        &self,
        states: &HashMap<String, WorkflowJobState>,
    ) -> Result<HashMap<String, Value>, CheaperError> {
        let mut results = HashMap::new();
        for id in self.definition.jobs().keys() {
            let state = states.get(id);
            if !state.is_some_and(|s| s.terminal) {
                return Err(CheaperError::new(
                    ErrorCode::InvalidOutput,
                    format!("job is not terminal: {id}"),
                ));
            }
            if let Some(result) = state.and_then(|s| s.result.clone()) {
                results.insert(id.clone(), result);
            }
        }
        Ok(results)
    }

    /// `#skipped(states)` — `condition_skipped` job id 集合。
    /// `when` 評価・`$from` null materialization の両方で使う。
    fn skipped_of(states: &HashMap<String, WorkflowJobState>) -> HashSet<String> {
        states
            .iter()
            .filter(|(_, state)| state.condition_skipped)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// `#output(name, results)`。
    fn output_value(
        &self,
        name: &str,
        results: &HashMap<String, Value>,
        skipped: &HashSet<String>,
    ) -> Result<Value, CheaperError> {
        let output = self.definition.outputs().get(name);
        let validator = self.output_validation(name);
        let (Some(output), Some(validator)) = (output, validator) else {
            return Err(CheaperError::new(
                ErrorCode::InvalidOutput,
                format!("unknown workflow output: {name}"),
            ));
        };
        let raw_value = output.get("value").cloned().unwrap_or(Value::Null);
        let value = materialize_bindings(
            &raw_value,
            &self.materialized_inputs,
            results,
            MaterializeOptions {
                defer_from: false,
                code: Some(ErrorCode::InvalidOutput),
                skipped_from: Some(skipped),
            },
        )?;
        validator.validate(&value).map_err(|errors| {
            CheaperError::new(
                ErrorCode::InvalidOutput,
                format!("invalid workflow output: {name}"),
            )
            .with_details({
                let mut details = Map::new();
                details.insert(
                    "errors".to_string(),
                    serde_json::to_value(&errors).unwrap_or(Value::Null),
                );
                details
            })
        })?;
        Ok(value)
    }

    /// `materializeOutput(name, states)`。
    pub fn materialize_output(
        &self,
        name: &str,
        states: &HashMap<String, WorkflowJobState>,
    ) -> Result<Value, CheaperError> {
        let results = self.results_of(states)?;
        self.output_value(name, &results, &Self::skipped_of(states))
    }

    /// `materializeOutputs(states)` — output 名は code-unit sort。
    pub fn materialize_outputs(
        &self,
        states: &HashMap<String, WorkflowJobState>,
    ) -> Result<Map<String, Value>, CheaperError> {
        let results = self.results_of(states)?;
        let skipped = Self::skipped_of(states);
        let mut names: Vec<&String> = self.definition.outputs().keys().collect();
        names.sort_by(|a, b| code_unit_compare(a, b));
        let mut out = Map::with_capacity(names.len());
        for name in names {
            out.insert(name.clone(), self.output_value(name, &results, &skipped)?);
        }
        Ok(out)
    }
}

/// `WorkflowCompiler`。
pub struct WorkflowCompiler<'a> {
    ports: WorkflowCompilerPorts<'a>,
}

impl<'a> WorkflowCompiler<'a> {
    pub fn new(ports: WorkflowCompilerPorts<'a>) -> Self {
        Self { ports }
    }

    pub fn ports(&self) -> &WorkflowCompilerPorts<'a> {
        &self.ports
    }

    /// `validateDefinition`。
    pub fn validate_definition(&self, raw: &Value) -> Result<WorkflowDefinition, ToolError> {
        Ok(validate_definition(raw, &self.ports)?.definition)
    }

    /// `validateInputs`(TS の `inputs` default `{}` は `None` で表現)。
    pub fn validate_inputs(
        &self,
        raw: &Value,
        inputs: Option<&Value>,
    ) -> Result<Map<String, Value>, ToolError> {
        let validated = validate_definition(raw, &self.ports)?;
        let empty = Value::Object(Map::new());
        Ok(invocation_inputs(&validated, inputs.unwrap_or(&empty))?)
    }

    /// `projectInterface`。
    pub fn project_interface(&self, raw: &Value) -> Result<WorkflowInterface, ToolError> {
        project_workflow_interface(raw, &self.ports)
    }

    /// `compile`。inference port の失敗はそのまま伝播する。
    pub async fn compile(
        &self,
        raw: &Value,
        inputs: Option<&Value>,
    ) -> Result<Arc<CompiledWorkflow>, ToolError> {
        let validated = validate_definition(raw, &self.ports)?;
        let empty = Value::Object(Map::new());
        let snapshot = invocation_inputs(&validated, inputs.unwrap_or(&empty))?;
        let definition = &validated.definition;
        let mut sorted_ids: Vec<&String> = definition.jobs().keys().collect();
        sorted_ids.sort_by(|a, b| code_unit_compare(a, b));
        let empty_results = HashMap::new();
        let mut jobs: Vec<(String, CompiledJob)> = Vec::with_capacity(sorted_ids.len());
        for id in sorted_ids {
            let original = parse_job(&definition.jobs()[id]);
            let input = materialize_bindings(
                &original.input,
                &snapshot,
                &empty_results,
                MaterializeOptions {
                    defer_from: true,
                    ..Default::default()
                },
            )?;
            if scan_bindings(&original.input)?.from.is_empty() {
                validate_job_input(validated.job_validation(id).expect("job validated"), &input)?;
            }
            let needs = original.needs.map(|list| {
                let mut seen = HashSet::new();
                let mut deduped: Vec<String> = list
                    .into_iter()
                    .filter(|n| seen.insert(n.clone()))
                    .collect();
                deduped.sort_by(|a, b| code_unit_compare(a, b));
                deduped
            });
            let resources = original
                .resources
                .map(|r| materialize_resources(&r, &snapshot))
                .transpose()?;
            jobs.push((
                id.clone(),
                CompiledJob {
                    kind: original.kind,
                    name: original.name,
                    input,
                    needs,
                    resources,
                    when: original.when,
                    manifest: None,
                },
            ));
        }
        let mut edges = validated.edges.clone();
        let mut conflicts = None;
        let mut diagnostics = None;
        // `job.resources && Object.keys(job.resources).length > 0`
        let any_resources = jobs.iter().any(|(_, job)| {
            job.resources
                .as_ref()
                .is_some_and(|r| r.reads.is_some() || r.writes.is_some() || r.produces.is_some())
        });
        if self.ports.resources.is_none() && any_resources {
            return Err(Box::new(CheaperError::new(
                ErrorCode::InvalidRequest,
                "resource inference port is required for explicit resources",
            )));
        }
        if let Some(inference) = self.ports.resources {
            let entries: Vec<(String, ResourceInferenceJob)> = jobs
                .iter()
                .map(|(id, job)| {
                    let worker = validated.job_validation(id).and_then(|v| v.worker.clone());
                    (
                        id.clone(),
                        ResourceInferenceJob {
                            process: None,
                            job: crate::types::WorkflowJob {
                                kind: job.kind,
                                name: job.name.clone(),
                                input: job.input.clone(),
                                needs: job.needs.clone(),
                                resources: job.resources.clone(),
                                when: job.when.clone(),
                            },
                            worker,
                        },
                    )
                })
                .collect();
            let inferred = inference.infer(&entries).await?;
            for (id, job) in &mut jobs {
                // `inferred.manifests[id]` — Record 重複 key は後勝ち。
                let manifest = inferred
                    .manifests
                    .iter()
                    .rev()
                    .find(|(k, _)| k == id)
                    .map(|(_, m)| m);
                let Some(manifest) = manifest else {
                    return Err(Box::new(CheaperError::new(
                        ErrorCode::InvalidRequest,
                        format!("missing resource manifest: {id}"),
                    )));
                };
                job.manifest = Some(manifest.clone());
            }
            edges.extend(inferred.edges.iter().cloned());
            conflicts = inferred.conflicts;
            diagnostics = inferred.diagnostics;
        }
        let job_ids: Vec<String> = jobs.iter().map(|(id, _)| id.clone()).collect();
        let dag = compile_dag(&job_ids, &edges)?;
        Ok(Arc::new(CompiledWorkflow {
            inputs: clone_json(&Value::Object(snapshot.clone()), ErrorCode::InvalidRequest)?
                .as_object()
                .cloned()
                .unwrap_or_default(),
            jobs,
            outputs: clone_json(
                &Value::Object(validated.definition.outputs().clone()),
                ErrorCode::InvalidRequest,
            )?
            .as_object()
            .cloned()
            .unwrap_or_default(),
            order: dag.order,
            edges: dag.edges,
            conflicts,
            diagnostics,
            definition: validated.definition,
            validated_outputs: validated.outputs,
            validated_jobs: validated.jobs,
            materialized_inputs: snapshot,
        }))
    }
}
