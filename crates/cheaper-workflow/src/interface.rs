//! Port of `packages/workflow/src/interface.ts`.

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::code_unit_compare;
use cheaper_tools::ToolError;
use serde_json::Value;

use crate::json::{clone_json, has_own};
use crate::schema::validate_definition;
use crate::types::{InterfaceInput, InterfaceOutput, WorkflowInterface};
use crate::WorkflowCompilerPorts;

/// `projectWorkflowInterface` — definition 検証だけを行い、
/// invocation/resource/execution を要求しない。
pub fn project_workflow_interface(
    raw: &Value,
    ports: &WorkflowCompilerPorts<'_>,
) -> Result<WorkflowInterface, ToolError> {
    let validated = validate_definition(raw, ports)?;
    let definition = &validated.definition;
    let mut inputs: Vec<(String, InterfaceInput)> = Vec::new();
    for (name, schema) in definition.inputs() {
        let cloned = clone_json(schema, ErrorCode::InvalidRequest)
            .map_err(|e: CheaperError| Box::new(e) as ToolError)?;
        inputs.push((
            name.clone(),
            InterfaceInput {
                schema: cloned.as_object().cloned().unwrap_or_default(),
                required: !schema.as_object().is_some_and(|o| has_own(o, "default")),
            },
        ));
    }
    // `Object.keys(...).sort()` — UTF-16 code-unit 順。
    inputs.sort_by(|a, b| code_unit_compare(&a.0, &b.0));
    let mut outputs: Vec<(String, InterfaceOutput)> = Vec::new();
    for (name, output) in definition.outputs() {
        // `output?.schema` が undefined なら cloneJson(undefined) は
        // `value must be plain JSON` で失敗する。
        let schema = output.get("schema").cloned().ok_or_else(|| {
            Box::new(CheaperError::new(
                ErrorCode::InvalidRequest,
                "value must be plain JSON",
            )) as ToolError
        })?;
        let cloned = clone_json(&schema, ErrorCode::InvalidRequest)
            .map_err(|e: CheaperError| Box::new(e) as ToolError)?;
        outputs.push((
            name.clone(),
            InterfaceOutput {
                schema: cloned.as_object().cloned().unwrap_or_default(),
            },
        ));
    }
    outputs.sort_by(|a, b| code_unit_compare(&a.0, &b.0));
    Ok(WorkflowInterface { inputs, outputs })
}
