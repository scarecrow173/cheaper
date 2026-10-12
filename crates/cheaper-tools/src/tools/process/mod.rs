//! Port of `packages/tool-runtime/src/process.ts` の task 固有 glue。
//!
//! named task 解決・prepare(cwd/deny/writeScope 検査)・details{task,
//! state,exitCode,signal,stdout,stderr}・`processOperationDeclaration`
//! (pool + access 導出)を所有し、managed-subprocess lifecycle
//! (spawn/capture/timeout/heartbeat/cleanup/tree-kill)は shared
//! `crate::tools::proc` へ移設済み(design §(e))。execute は
//! `ProcLimits`/`ProcPrepared`/`ProcIdentity` を組み立てて
//! [`crate::tools::proc::run`] を呼ぶ。
//!
//! T2-7 統合済み seam(T2-5 時点のローカル定義は撤去):
//! - `AbortSignal`/`AbortController`/`AbortReason` — `crate::operation`。
//! - result は `crate::catalog::CanonicalToolResult`。
//! - `process_operation_declaration` は `crate::scheduler` の
//!   `OperationDeclaration`/`OperationPool` を返す。

use std::collections::HashSet;
use std::sync::Arc;

use cheaper_core::duration::parse_duration;
use cheaper_core::errors::CheaperError;
/// `MAX_PROCESS_OUTPUT_BYTES` re-export(TS `export { ... }` parity)。
pub use cheaper_core::policy::process::MAX_PROCESS_OUTPUT_BYTES;
use cheaper_core::policy::process::{resolve_process_task, ResolvedProcessPolicy};
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::{json, Value};

use crate::catalog::CanonicalToolResult;
use crate::operation::AbortSignal;
use crate::scheduler::{OperationDeclaration, OperationPool};
use crate::ToolError;

mod prepare;
#[cfg(test)]
mod tests;

use prepare::prepare;

// `tools::proc` へ移設した item の公開 path 維持 re-export
// (design §(e)/(l) — registry・workflow_run・既存 test が
// `tools::process::{find_cheaper_error,OnTimeout,...}` を参照するため)。
pub use crate::tools::proc::executable;
pub use crate::tools::proc::tree;
pub use crate::tools::proc::{find_cheaper_error, OnTimeout, StreamCaptureResult};

// ---------------------------------------------------------------------------
// Schemas / operation declaration / tool surface
// ---------------------------------------------------------------------------

pub fn process_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "task": {
                "type": "string",
                "minLength": 1,
                "maxLength": 64,
                "pattern": "^[a-z][a-z0-9_-]*$",
                "description": "Name of a policy-defined process task",
            },
        },
        "required": ["task"],
        "additionalProperties": false,
    })
}

fn stream_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "text": {"type": "string", "maxLength": MAX_PROCESS_OUTPUT_BYTES},
            "bytes": {"type": "integer", "minimum": 0},
            "capturedBytes": {"type": "integer", "minimum": 0, "maximum": MAX_PROCESS_OUTPUT_BYTES},
            "truncated": {"type": "boolean"},
        },
        "required": ["text", "bytes", "capturedBytes", "truncated"],
        "additionalProperties": false,
    })
}

/// `processDetailsSchema`(TypeBox 生成物を literal JSON 化)。
pub fn process_details_schema() -> Value {
    let stream = stream_schema();
    json!({
        "type": "object",
        "properties": {
            "task": {"type": "string"},
            "state": {"enum": ["completed", "failed"]},
            "exitCode": {"type": ["integer", "null"]},
            "signal": {"type": ["string", "null"]},
            "stdout": stream,
            "stderr": stream,
        },
        "required": ["task", "state", "exitCode", "signal", "stdout", "stderr"],
        "additionalProperties": false,
    })
}

/// `processOperationDeclaration`: task 解決 → outputs 有無で access 導出、
/// pool は `{name:"process", maxConcurrent}`。返すのは scheduler の
/// canonical `OperationDeclaration`(T2-5 のローカル seam 型は撤去)。
pub fn process_operation_declaration(
    policy: &ResolvedProcessPolicy,
    task: &str,
) -> Result<OperationDeclaration, CheaperError> {
    let task = resolve_process_task(policy, task)?;
    let mut declaration = OperationDeclaration::new(if task.outputs.is_empty() {
        cheaper_core::resources::types::ClaimMode::Shared
    } else {
        cheaper_core::resources::types::ClaimMode::Exclusive
    });
    declaration.pool = Some(OperationPool {
        name: "process".to_string(),
        max_concurrent: policy.max_concurrent as i64,
    });
    Ok(declaration)
}

/// `Options`(TS `interface Options`)。
pub struct ProcessToolOptions {
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    pub process: ResolvedProcessPolicy,
    /// `onTimeout(reason)`。
    pub on_timeout: Option<OnTimeout>,
    pub denylist: Option<PathDenylist>,
    pub write_scope: Option<HashSet<String>>,
}

/// `createProcessTool` が返す canonical definition。
pub struct ProcessTool {
    description: String,
    options: ProcessToolOptions,
}

/// `createProcessTool`。description に task 一覧を埋め込む。
pub fn create_process_tool(options: ProcessToolOptions) -> ProcessTool {
    let tasks: Vec<&str> = options.process.tasks.keys().map(String::as_str).collect();
    ProcessTool {
        description: format!(
            "Run a policy-defined native process task: {}. No arbitrary command or shell.",
            tasks.join(", ")
        ),
        options,
    }
}

impl ProcessTool {
    pub fn name(&self) -> &'static str {
        "process"
    }

    pub fn label(&self) -> &'static str {
        "Process"
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    /// `parameters` JSON schema(`validateToolInput` 接合対象)。
    pub fn parameters(&self) -> Value {
        process_schema()
    }

    /// `execute(input, signal)`。`input.task` の schema 検証は catalog
    /// 側責務(TS 同様)。`signal` は `signal ?? new AbortController().signal`。
    pub async fn execute(
        &self,
        input: &Value,
        signal: Option<AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let task = input
            .get("task")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        execute(
            &self.options,
            &task,
            signal.unwrap_or_else(AbortSignal::detached),
        )
        .await
    }
}

/// `createProcessTool` の `CanonicalToolDefinition` ラッパ(TS
/// `defineCanonicalTool` 相当)。registry は `name`/`label`/`description`/
/// `parameters` をここから取得し、`execute` は canonical
/// `CanonicalExecute` seam(future)で呼ぶ。tool は `Arc` 共有 — 返り値
/// future が env borrow を持たず `'static` で独立する。
pub fn process_tool_definition(
    options: ProcessToolOptions,
) -> crate::definition::CanonicalToolDefinition<'static> {
    let tool = Arc::new(create_process_tool(options));
    crate::definition::define_canonical_tool(crate::definition::CanonicalToolDefinition {
        name: cheaper_core::tools::vocabulary::ToolName::Process,
        label: tool.label().to_string(),
        description: tool.description().to_string(),
        parameters: process_schema().as_object().expect("object schema").clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input, signal).await })
        }),
    })
}

// ---------------------------------------------------------------------------
// execute(task 固有 glue → shared proc::run)
// ---------------------------------------------------------------------------
async fn execute(
    options: &ProcessToolOptions,
    name: &str,
    outer: AbortSignal,
) -> Result<CanonicalToolResult, ToolError> {
    use crate::tools::proc::{aborted_err, minimal_env, ProcIdentity, ProcLimits, ProcPrepared};

    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }
    let task = resolve_process_task(&options.process, name)?;
    let prepared = prepare(options, name, &task)?;
    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }

    // `options.process.*` → `ProcLimits`(parse 済み ms)。
    let limits = ProcLimits {
        timeout_ms: parse_duration(&options.process.timeout)?,
        kill_grace_ms: parse_duration(&options.process.kill_grace)?,
        max_output_bytes: options.process.max_output_bytes,
        heartbeat_interval_ms: parse_duration(&options.process.heartbeat.interval)?,
    };
    let prepared = ProcPrepared {
        executable: prepared.executable,
        argv: task.command[1..].to_vec(),
        cwd: prepared.cwd,
        env: minimal_env(&task.env),
    };
    // `process` の identity: details `{task: name}`・display
    // `"process {name}"`・diagnostics event prefix `"process"`。
    let identity = ProcIdentity {
        details: vec![("task".to_string(), json!(name))],
        label: name.to_string(),
        display: format!("process {name}"),
        event_prefix: "process",
    };
    crate::tools::proc::run(
        &prepared,
        &limits,
        &identity,
        outer,
        options.on_timeout.as_ref(),
    )
    .await
}
