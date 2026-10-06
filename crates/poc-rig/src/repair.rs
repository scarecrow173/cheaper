use std::sync::Arc;

use serde::Serialize;

use crate::agent;
use crate::usage::{Accumulator, Snapshot};

/// The contract a repaired run must satisfy.
pub const OUTPUT_SCHEMA: &str = r#"{
    "type": "object",
    "properties": { "answer": { "type": "string" } },
    "required": ["answer"],
    "additionalProperties": false
}"#;

const MAX_ATTEMPTS: u64 = 3;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairReport {
    pub schema_valid: bool,
    pub attempts: u64,
    pub output: Option<String>,
    #[serde(flatten)]
    pub snapshot: Snapshot,
}

/// Repair session: run → extract final assistant text → `jsonschema`
/// validate → corrective-feedback retry, all sharing one accumulator so
/// usage/turns merge across attempts (design: repair sessions contribute to
/// the same run accumulator).
pub async fn run_repair(model: rig::DynModel<rig::operation::Completion>) -> RepairReport {
    let acc = Arc::new(Accumulator::default());
    let schema: serde_json::Value = serde_json::from_str(OUTPUT_SCHEMA).unwrap();

    let mut prompt = "Reply with a JSON object matching {answer: string}.".to_string();
    let mut output = None;
    let mut schema_valid = false;
    let mut attempts = 0u64;

    for attempt in 1..=MAX_ATTEMPTS {
        attempts = attempt;
        let agent = agent::build(model.clone(), acc.clone(), vec![]);
        match agent::run_blocking(&agent, &prompt, 4, 0).await {
            Ok(response) => {
                let text = response.output;
                let parsed = serde_json::from_str::<serde_json::Value>(text.trim());
                let valid = parsed
                    .as_ref()
                    .map(|doc| jsonschema::is_valid(&schema, doc))
                    .unwrap_or(false);
                if valid {
                    schema_valid = true;
                    output = Some(text);
                    break;
                }
                acc.record_repair();
                prompt = format!(
                    "Your previous reply failed the output schema: {:?}. \
                     Reply with ONLY a JSON object matching {{answer: string}}.",
                    parsed.err().map(|e| e.to_string())
                );
                output = Some(text);
            }
            Err(err) => {
                output = Some(format!("run failed: {err}"));
                break;
            }
        }
    }

    RepairReport {
        schema_valid,
        attempts,
        output,
        snapshot: acc.snapshot(),
    }
}
