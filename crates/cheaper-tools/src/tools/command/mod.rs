//! `command` canonical tool の実装(policy-gated command tool design
//! §(b)/(e)/(f)/(g) + issue #165 unified exec grant model §(i))。
//!
//! issue #165 で thin surface 化 — 本 module は `command` surface の
//! schema/details schema/description/identity(`{grant, argv}` details・
//! `"command {name}"` display・`durationMs` 報告)/`{name, args}` input
//! 変換・route context 合成のみを所有し、grant 解決・pin/verify/
//! argv pattern 照合・declaration 等の共通機構は
//! [`crate::tools::exec`] へ集約した。
//!
//! model 入力は `{name, args}` のみ — executable/cwd/env は trusted
//! policy の grant に由来し、model 入力経路には一切乗らない。
//! `!fixed` grant のみが到達可能(disjoint surface — `fixed` grant
//! 参照は "unknown or unauthorized command grant" で fail-closed)。
//! execute glue は [`check_exec_execution`] → `ProcPrepared` →
//! shared `crate::tools::proc::run` の順。pin は registry の
//! `create_tool_runtime` が `names` に `Command` を含む場合のみ行う
//! (per-scope、design §(b)/(g))— `tools::exec::pin_exec_grants`。

use std::sync::Arc;

use cheaper_core::duration::parse_duration;
/// `MAX_PROCESS_OUTPUT_BYTES` — details schema の stream cap。
use cheaper_core::policy::exec::MAX_PROCESS_OUTPUT_BYTES;
use serde_json::{json, Value};

use crate::catalog::CanonicalToolResult;
use crate::operation::AbortSignal;
use crate::tools::exec::{check_exec_execution, ExecToolOptions};
use crate::tools::proc::{aborted_err, minimal_env, ProcIdentity, ProcLimits, ProcPrepared};
use crate::ToolError;

/// `commandSchema` — model input は `{name, args}` のみ。`executable`/
/// `cwd`/`env` 等の余剰 key は `additionalProperties: false` で schema
/// 層の `INVALID_INPUT` に落ち、model 入力経路に乗らない(design §(i))。
/// `required: ["name", "args"]` — `args` 省略不可で model に argv の
/// 明示を強制する。
pub fn command_schema() -> Value {
    json!({
        "type": "object",
        "required": ["name", "args"],
        "properties": {
            "name": {
                "type": "string",
                "pattern": "^[a-z][a-z0-9_-]{0,63}$",
                "description": "Name of a policy-authorized command grant",
            },
            "args": {
                "type": "array",
                "maxItems": 256,
                "items": {"type": "string", "maxLength": 4096},
                "description": "Arguments matched against the grant argv pattern",
            },
        },
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

/// `commandDetailsSchema` — `{grant, argv, state, exitCode, signal,
/// durationMs, stdout, stderr}`。`durationMs`/`argv` は spec が要求する
/// duration/argv 報告で、process details shape を意図的に超過する
/// spec-mandated な差分(design §(e))。
pub fn command_details_schema() -> Value {
    let stream = stream_schema();
    json!({
        "type": "object",
        "properties": {
            "grant": {"type": "string"},
            "argv": {"type": "array", "items": {"type": "string"}},
            "state": {"enum": ["completed", "failed"]},
            "exitCode": {"type": ["integer", "null"]},
            "signal": {"type": ["string", "null"]},
            "durationMs": {"type": "integer", "minimum": 0},
            "stdout": stream,
            "stderr": stream,
            "logRef": {"type": "string"},
            "outputTokens": {"type": "integer", "minimum": 0},
            "outputTokensTruncated": {"type": "boolean"},
        },
        "required": [
            "grant", "argv", "state", "exitCode", "signal", "durationMs",
            "stdout", "stderr",
        ],
        "additionalProperties": false,
    })
}

// ---------------------------------------------------------------------------
// tool surface
// ---------------------------------------------------------------------------

/// `createCommandTool` が返す canonical definition。
pub struct CommandTool {
    description: String,
    options: ExecToolOptions,
}

/// `createCommandTool`。description には `!fixed` grant 名一覧のみを
/// 埋め込む(disjoint surface — `fixed` grant 名は model へ露出
/// しない、design §(i))。
pub fn create_command_tool(options: ExecToolOptions) -> CommandTool {
    let grants: Vec<&str> = options
        .exec
        .grants
        .iter()
        .filter(|(_, g)| !g.fixed)
        .map(|(name, _)| name.as_str())
        .collect();
    CommandTool {
        description: format!(
            "Run a policy-authorized command grant: {}. Only granted executables with matching argv patterns run.",
            grants.join(", ")
        ),
        options,
    }
}

impl CommandTool {
    pub fn name(&self) -> &'static str {
        "command"
    }

    pub fn label(&self) -> &'static str {
        "Command"
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    /// `parameters` JSON schema(`validateToolInput` 接合対象)。
    pub fn parameters(&self) -> Value {
        command_schema()
    }

    /// `execute(input, signal)`。`input.name`/`input.args` の schema
    /// 検証は catalog 側責務(TS 同様)。`signal` は
    /// `signal ?? new AbortController().signal`。
    pub async fn execute(
        &self,
        input: &Value,
        signal: Option<AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        execute(
            &self.options,
            input,
            signal.unwrap_or_else(AbortSignal::detached),
        )
        .await
    }
}

/// `createCommandTool` の `CanonicalToolDefinition` ラッパ。
/// `process_tool_definition` と同じく tool は `Arc` 共有。
pub fn command_tool_definition(
    options: ExecToolOptions,
) -> crate::definition::CanonicalToolDefinition<'static> {
    let tool = Arc::new(create_command_tool(options));
    crate::definition::define_canonical_tool(crate::definition::CanonicalToolDefinition {
        name: cheaper_core::tools::vocabulary::ToolName::Command,
        label: tool.label().to_string(),
        description: tool.description().to_string(),
        parameters: command_schema().as_object().expect("object schema").clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input, signal).await })
        }),
    })
}

// ---------------------------------------------------------------------------
// execute(check_exec_execution → ProcPrepared → proc::run)
// ---------------------------------------------------------------------------

async fn execute(
    options: &ExecToolOptions,
    input: &Value,
    outer: AbortSignal,
) -> Result<CanonicalToolResult, ToolError> {
    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    // schema 検証済みのため全要素 string — 非 string は捨てて
    // pattern match 側の個数不一致で fail-closed に落ちる。
    let args: Vec<String> = input
        .get("args")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    // 事前 check は `check_exec_execution` へ集約(design
    // §viability — `route::resolve` と同一 predicate)。pin map に
    // grant が無ければ即 `POLICY_VIOLATION`、pin 済み path は
    // spawn 前再検証(`native_executable` 再解決 + path 同一性)を通す。
    let checked = check_exec_execution(options, &name, &args)?;
    let grant = checked.grant;
    let cwd = checked.cwd;
    let pinned = checked.executable;
    let route_ctx = options.route_context.as_ref();
    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }

    // per-grant override 畳み込み済みの effective limit を
    // `ProcLimits` へ流す(`parse_duration` は policy resolve で
    // 検証済みのため到達不能想定)。heartbeat は `command` defaults
    // block の section-level のみ。
    let limits = ProcLimits {
        timeout_ms: parse_duration(&grant.timeout)?,
        kill_grace_ms: parse_duration(&grant.kill_grace)?,
        max_output_bytes: grant.max_output_bytes,
        heartbeat_interval_ms: parse_duration(&options.exec.command.heartbeat.interval)?,
        // route context の `accept_any_exit_code` は declared grant の
        // `exitCodes` を上書きして任意の completed exit を result 化
        // する(C2 — routed scope でのみ注入される)。
        accepted_exit_codes: if route_ctx.is_some_and(|r| r.accept_any_exit_code) {
            None
        } else {
            Some(grant.exit_codes.clone())
        },
    };
    // `extra_env` は `grant.env` への override — 合成済み map を
    // `minimal_env` へ渡し正規化を一箇所へ集約する。
    let mut declared_env = grant.env.clone();
    if let Some(extra) = route_ctx.map(|r| &r.extra_env) {
        for (key, value) in extra {
            declared_env.insert(key.clone(), value.clone());
        }
    }
    let prepared = ProcPrepared {
        executable: pinned.clone(),
        argv: args.clone(),
        cwd,
        env: minimal_env(&declared_env),
        correlation: options.execution_correlation.clone(),
    };
    // `command` の identity: details `{grant: name, argv: args}`・
    // display `"command {name}"`・diagnostics event prefix `"command"`。
    // `durationMs` は details schema required のため identity flag で
    // proc 側 details 組み立てに載せる。
    let identity = ProcIdentity {
        details: vec![
            ("grant".to_string(), json!(name)),
            ("argv".to_string(), json!(args)),
        ],
        label: name.clone(),
        display: format!("command {name}"),
        event_prefix: "command",
        report_duration_ms: true,
    };
    crate::tools::proc::run(
        &prepared,
        &limits,
        &identity,
        outer,
        options.on_timeout.as_ref(),
        route_ctx.and_then(|r| r.log_tee.clone()),
    )
    .await
}
