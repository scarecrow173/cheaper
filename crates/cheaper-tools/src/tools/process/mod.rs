//! Port of `packages/tool-runtime/src/process.ts` の task 固有 glue。
//!
//! issue #165(unified exec grant model)で thin surface 化 —
//! 本 module は `process` surface の schema/details schema/
//! description/identity(`{task}` details・`"process {name}"`
//! display・`"process"` event prefix)/`{task}` input 変換のみを
//! 所有し、grant 解決・pin/prepare・declaration 等の共通機構は
//! [`crate::tools::exec`] へ集約した。managed-subprocess lifecycle
//! (spawn/capture/timeout/heartbeat/cleanup/tree-kill)は shared
//! [`crate::tools::proc`] が担う。
//!
//! fixed grant(`process.tasks` 由来 + `command.grants` の
//! `fixed:true` 宣言)のみが到達可能 — `resolve_exec_grant(.., true)`
//! の disjoint 解決で `!fixed` grant 参照は unknown task として
//! fail-closed される(design §(i))。execute は `ProcLimits`/
//! `ProcPrepared`/`ProcIdentity` を組み立てて
//! [`crate::tools::proc::run`] を呼ぶ。

use std::sync::Arc;

use cheaper_core::duration::parse_duration;
use cheaper_core::policy::exec::resolve_exec_grant;
/// `MAX_PROCESS_OUTPUT_BYTES` re-export(TS `export { ... }` parity —
/// issue #165 で `policy::process` → `policy::exec` へ origin を移動)。
pub use cheaper_core::policy::exec::MAX_PROCESS_OUTPUT_BYTES;
use serde_json::{json, Value};

use crate::catalog::CanonicalToolResult;
use crate::operation::AbortSignal;
use crate::tools::exec::{prepare_exec_grant, ExecToolOptions, PreparedExec};
use crate::ToolError;

#[cfg(test)]
mod tests;

// `tools::proc` へ移設した item の公開 path 維持 re-export
// (design §(e)/(l) — registry・workflow_run・既存 test が
// `tools::process::{find_cheaper_error,OnTimeout,...}` を参照するため)。
pub use crate::tools::proc::executable;
pub use crate::tools::proc::tree;
pub use crate::tools::proc::{find_cheaper_error, OnTimeout, StreamCaptureResult};

// ---------------------------------------------------------------------------
// Schemas / tool surface
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

/// `createProcessTool` が返す canonical definition。
pub struct ProcessTool {
    description: String,
    options: ExecToolOptions,
}

/// `createProcessTool`。description には fixed grant 名一覧のみを
/// 埋め込む(disjoint surface — `!fixed` grant 名は model へ露出
/// しない、design §(i))。
pub fn create_process_tool(options: ExecToolOptions) -> ProcessTool {
    let tasks: Vec<&str> = options
        .exec
        .grants
        .iter()
        .filter(|(_, g)| g.fixed)
        .map(|(name, _)| name.as_str())
        .collect();
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
    options: ExecToolOptions,
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
// execute(fixed grant 解決 → prepare → shared proc::run)
// ---------------------------------------------------------------------------
async fn execute(
    options: &ExecToolOptions,
    name: &str,
    outer: AbortSignal,
) -> Result<CanonicalToolResult, ToolError> {
    use crate::tools::proc::{aborted_err, minimal_env, ProcIdentity, ProcLimits, ProcPrepared};

    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }
    // disjoint surface — fixed grant のみ到達可能(`!fixed` 参照は
    // "unknown or unauthorized process task" で fail-closed)。
    let grant =
        resolve_exec_grant(&options.exec, name, true).map_err(|e| Box::new(e) as ToolError)?;
    let prepared: PreparedExec = prepare_exec_grant(options, name, &grant)?;
    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }

    // per-grant override 畳み込み済みの effective limit を
    // `ProcLimits` へ流す(`parse_duration` は policy resolve で
    // 検証済みのため到達不能想定)。heartbeat は `process` defaults
    // block の section-level のみ。
    let limits = ProcLimits {
        timeout_ms: parse_duration(&grant.timeout)?,
        kill_grace_ms: parse_duration(&grant.kill_grace)?,
        max_output_bytes: grant.max_output_bytes,
        heartbeat_interval_ms: parse_duration(&options.exec.process.heartbeat.interval)?,
        accepted_exit_codes: Some(grant.exit_codes.clone()),
    };
    let prepared = ProcPrepared {
        executable: prepared.executable,
        // fixed grant の `argv` は literal argv(task の command[1..]
        // 由来) — pattern 照合は行わない。
        argv: grant.argv.clone().unwrap_or_default(),
        cwd: prepared.cwd,
        env: minimal_env(&grant.env),
    };
    // `process` の identity: details `{task: name}`・display
    // `"process {name}"`・diagnostics event prefix `"process"`。
    let identity = ProcIdentity {
        details: vec![("task".to_string(), json!(name))],
        label: name.to_string(),
        display: format!("process {name}"),
        event_prefix: "process",
        // `durationMs` は command 専用の spec-mandated field —
        // process details schema は不変(AC1)。
        report_duration_ms: false,
    };
    crate::tools::proc::run(
        &prepared,
        &limits,
        &identity,
        outer,
        options.on_timeout.as_ref(),
        None,
    )
    .await
}
