//! Structured output 抽出(packages/runtime-pi/src/output.ts 相当)。
//!
//! TS の `extractOutput`:
//! 1. last assistant message の text block を連結(Rig では stream の
//!    `Finished.output` がこの値に対応 — adapter が finalResponse の
//!    text を連結して返す)。
//! 2. 外側の markdown code fence を 1 層剥がす。
//! 3. JSON.parse。
//! 4. plain object 必須。
//! 5. outputSchema 検証。

use cheaper_core::worker::schema::{CompiledSchema, SchemaErrorEntry};
use serde_json::{Map, Value};

/// `ExtractedOutput` の failure reason 文字列(TS と同一値)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractFailureReason {
    /// assistant の final response が無い。
    NoAssistantMessage,
    /// JSON.parse 失敗。
    InvalidJson,
    /// object 非一致 / schema 検証失敗。
    SchemaMismatch,
}

impl ExtractFailureReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoAssistantMessage => "no-assistant-message",
            Self::InvalidJson => "invalid-json",
            Self::SchemaMismatch => "schema-mismatch",
        }
    }
}

/// `extractOutput` 相当の結果。
#[derive(Debug, Clone)]
pub enum ExtractedOutput {
    /// validated object + 元の assistant text(fence 除去前 — TS
    /// `rawOutput: raw` parity)。
    Ok {
        result: Map<String, Value>,
        raw_output: String,
    },
    Err {
        reason: ExtractFailureReason,
        /// 失敗時の raw text(`no-assistant-message` では空文字列)。
        raw_output: String,
        errors: Vec<SchemaErrorEntry>,
    },
}

/// `extractOutput(text, schema)` — `text` は last assistant message の
/// 連結 text(stream `Finished.output`)。`None` = assistant message 無し。
pub fn extract_output(raw_output: Option<&str>, schema: &CompiledSchema) -> ExtractedOutput {
    let Some(raw_output) = raw_output else {
        return ExtractedOutput::Err {
            reason: ExtractFailureReason::NoAssistantMessage,
            raw_output: String::new(),
            errors: Vec::new(),
        };
    };

    // F4: `rawOutput` contract は元の assistant text そのもの
    // (`rawOutput: raw` — fence 除去前)。parse/検証は fence 除去後の
    // `stripped` に対して行うが、返り値の raw_output と repair prompt
    // には original を渡す(TS extractOutput parity)。
    let stripped = strip_code_fence(raw_output);
    let raw_output = raw_output.to_string();
    let parsed: Value = match serde_json::from_str(&stripped) {
        Ok(v) => v,
        Err(err) => {
            // JS SyntaxError message とは文字列が異なる(serde_json の
            // message)。reason/contract は同じ。
            return ExtractedOutput::Err {
                reason: ExtractFailureReason::InvalidJson,
                raw_output,
                errors: vec![SchemaErrorEntry {
                    instance_path: String::new(),
                    message: err.to_string(),
                }],
            };
        }
    };
    if !parsed.is_object() {
        return ExtractedOutput::Err {
            reason: ExtractFailureReason::SchemaMismatch,
            raw_output,
            errors: vec![SchemaErrorEntry {
                instance_path: String::new(),
                message: "output is not a JSON object".to_string(),
            }],
        };
    }
    match schema.validate(&parsed) {
        Ok(()) => ExtractedOutput::Ok {
            result: parsed.as_object().expect("checked is_object").clone(),
            raw_output,
        },
        Err(errors) => ExtractedOutput::Err {
            reason: ExtractFailureReason::SchemaMismatch,
            raw_output,
            errors,
        },
    }
}

/// TS `stripCodeFence` の port。外側 1 層のみ:
/// `^```[^\n]*\n` を剥がし、`\n?```\s*$` を剥がし、trim。
pub fn strip_code_fence(text: &str) -> String {
    let mut t = text.trim();
    // `^```[^\n]*\n` — ``` で始まり最初の改行までを除去。
    if t.starts_with("```") {
        if let Some(newline) = t.find('\n') {
            t = &t[newline + 1..];
        }
    }
    // `/```\s*$/` test → `/\n?```\s*$/` replace(末尾 ``` とその直前
    // の改行 1 つまでを除去)。
    let trimmed = t.trim_end();
    if let Some(core) = trimmed.strip_suffix("```") {
        t = core.strip_suffix('\n').unwrap_or(core);
    }
    t.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn object_schema() -> CompiledSchema {
        cheaper_core::worker::schema::compile_schema(
            Map::from_iter([("type".into(), json!("object"))]),
            cheaper_core::worker::schema::SchemaKind::Output,
        )
        .unwrap()
    }

    #[test]
    fn strips_single_fence_and_validates() {
        let schema = object_schema();
        let out = extract_output(Some("```json\n{\"a\":1}\n```"), &schema);
        match out {
            ExtractedOutput::Ok { result, raw_output } => {
                assert_eq!(result["a"], json!(1));
                // F4: rawOutput は fence 除去前の original text
                // (repair prompt が model の実出力を見るため)。
                assert_eq!(raw_output, "```json\n{\"a\":1}\n```");
            }
            ExtractedOutput::Err { .. } => panic!("expected ok"),
        }
    }

    #[test]
    fn failure_variants_keep_original_raw_output() {
        let schema = object_schema();
        // fenced だが内部が invalid JSON → reason=invalid-json、
        // raw_output は fence 付き original。
        match extract_output(Some("```json\nnot json\n```"), &schema) {
            ExtractedOutput::Err {
                reason, raw_output, ..
            } => {
                assert_eq!(reason.as_str(), "invalid-json");
                assert_eq!(raw_output, "```json\nnot json\n```");
            }
            _ => panic!(),
        }
        // fenced + schema mismatch も同じく original。
        match extract_output(Some("```json\n[1,2]\n```"), &schema) {
            ExtractedOutput::Err {
                reason, raw_output, ..
            } => {
                assert_eq!(reason.as_str(), "schema-mismatch");
                assert_eq!(raw_output, "```json\n[1,2]\n```");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn no_assistant_message() {
        match extract_output(None, &object_schema()) {
            ExtractedOutput::Err { reason, .. } => {
                assert_eq!(reason.as_str(), "no-assistant-message")
            }
            _ => panic!("expected err"),
        }
    }

    #[test]
    fn invalid_json_and_non_object() {
        let schema = object_schema();
        match extract_output(Some("not json"), &schema) {
            ExtractedOutput::Err { reason, errors, .. } => {
                assert_eq!(reason.as_str(), "invalid-json");
                assert_eq!(errors.len(), 1);
            }
            _ => panic!(),
        }
        match extract_output(Some("[1,2]"), &schema) {
            ExtractedOutput::Err { reason, .. } => {
                assert_eq!(reason.as_str(), "schema-mismatch")
            }
            _ => panic!(),
        }
    }
}
