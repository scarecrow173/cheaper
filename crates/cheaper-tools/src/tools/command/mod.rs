//! `command` canonical tool の T1 deny-stub(policy-gated command tool
//! design §(i))。catalog/registry の closed `ToolName` match arm を
//! compile 充足するための `{name,args}` input schema・details schema・
//! `POLICY_VIOLATION "command unavailable"` で失敗する definition のみ —
//! grant 解決・argv match・pin 検証・execute glue は T3 が実装し、
//! `command` は T3 まで catalog `NAMES`/registered set に入れない。
//!
//! `process` tool の surface 規約(schema literal 化・
//! `define_canonical_tool` wrapper)を mirror するが、process の
//! subprocess 実行 path とは共有しない — managed-process 共通基盤の
//! `proc` 抽出は T2 が担う。

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Value};

use cheaper_core::policy::process::MAX_PROCESS_OUTPUT_BYTES;

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
/// spec-mandated な差分(design §(i))。
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
        },
        "required": [
            "grant", "argv", "state", "exitCode", "signal", "durationMs",
            "stdout", "stderr",
        ],
        "additionalProperties": false,
    })
}

/// deny-stub definition。`command` は T3 まで `names`/registered set に
/// 含まれないため execute へ到達しないが、closed enum arm の compile
/// 充足として `POLICY_VIOLATION "command unavailable"` を返す定義を
/// 置く(誤って到達した場合も fail-closed)。
pub fn command_tool_definition() -> crate::definition::CanonicalToolDefinition<'static> {
    crate::definition::define_canonical_tool(crate::definition::CanonicalToolDefinition {
        name: cheaper_core::tools::vocabulary::ToolName::Command,
        label: "Command".to_string(),
        description: "Run a policy-authorized command grant".to_string(),
        parameters: command_schema().as_object().expect("object schema").clone(),
        prepare_arguments: None,
        execute: Box::new(move |_input, _signal| {
            Box::pin(async move {
                Err(Box::new(CheaperError::new(
                    ErrorCode::PolicyViolation,
                    "command unavailable",
                )) as crate::ToolError)
            }) as crate::definition::CanonicalExecuteFuture<'static>
        }),
    })
}
