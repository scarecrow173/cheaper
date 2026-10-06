//! Port of `packages/core/src/policy/merge.ts`.
//!
//! Merge rules (引継ぎ §22): objects deep-merge, scalars and arrays are
//! replaced wholesale by the override. `restrict-concurrency` mode takes
//! `min` over the 8 numeric fields, duration-min over the 3 duration
//! fields, and forces `execution.concurrency.mutations` to `serialized`.

use serde_json::{Map, Value};

use crate::duration::{parse_duration, InvalidDurationError};

/// `options.mode` of `mergePolicyLayers`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMode {
    Replace,
    RestrictConcurrency,
}

/// Restrict-mode numeric fields (value = `min(existing, override)`).
const RESTRICT_NUMERIC_MIN_FIELDS: &[&str] = &[
    "execution.concurrency.maxOperations",
    "workflow.maxJobs",
    "process.maxConcurrent",
    "process.maxOutputBytes",
    "workflow.artifacts.maxArtifactBytes",
    "workflow.artifacts.maxTotalBytes",
    "workflow.artifacts.maxPreviewBytes",
    "workflow.artifacts.maxReadBytes",
];

/// Restrict-mode duration fields (parsed and compared in ms).
const RESTRICT_DURATION_MIN_FIELDS: &[&str] = &[
    "process.timeout",
    "process.killGrace",
    "process.heartbeat.interval",
];

/// `mergePolicyLayers`. Inputs are not mutated; a new object is returned.
///
/// `Err(InvalidDurationError)` only from restrict mode (TS propagates the
/// `parseDuration` `RangeError` raw).
pub fn merge_policy_layers(
    base: &Map<String, Value>,
    override_: &Map<String, Value>,
    mode: MergeMode,
) -> Result<Map<String, Value>, InvalidDurationError> {
    merge(base, override_, mode == MergeMode::RestrictConcurrency, &[])
}

fn merge(
    base: &Map<String, Value>,
    override_: &Map<String, Value>,
    restrict: bool,
    parent: &[String],
) -> Result<Map<String, Value>, InvalidDurationError> {
    let mut out = base.clone();
    for (key, value) in override_ {
        let existing = out.get(key);
        let mut keys = parent.to_vec();
        keys.push(key.clone());
        let field = keys.join(".");
        if restrict
            && RESTRICT_NUMERIC_MIN_FIELDS.contains(&field.as_str())
            && existing.is_some_and(Value::is_number)
            && value.is_number()
        {
            // JS Math.min on the raw numbers; keep the winning operand's
            // own representation (integers stay integers).
            let min = if existing.and_then(Value::as_f64).unwrap_or(f64::INFINITY)
                <= value.as_f64().unwrap_or(f64::INFINITY)
            {
                existing.unwrap().clone()
            } else {
                value.clone()
            };
            out.insert(key.clone(), min);
        } else if restrict
            && RESTRICT_DURATION_MIN_FIELDS.contains(&field.as_str())
            && existing.is_some_and(Value::is_string)
            && value.is_string()
        {
            let existing_ms = parse_duration(existing.and_then(Value::as_str).unwrap())?;
            let value_ms = parse_duration(value.as_str().unwrap())?;
            out.insert(
                key.clone(),
                if existing_ms <= value_ms {
                    existing.cloned().unwrap()
                } else {
                    value.clone()
                },
            );
        } else if restrict
            && field == "execution.concurrency.mutations"
            && (existing.and_then(Value::as_str) == Some("serialized")
                || value.as_str() == Some("serialized"))
        {
            out.insert(key.clone(), Value::from("serialized"));
        } else {
            let merged = match (existing, value) {
                (Some(Value::Object(b)), Value::Object(o)) => {
                    Value::Object(merge(b, o, restrict, &keys)?)
                }
                _ => value.clone(),
            };
            out.insert(key.clone(), merged);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn merge_json(base: Value, over: Value, restrict: bool) -> Value {
        Value::Object(
            merge_policy_layers(
                base.as_object().unwrap(),
                over.as_object().unwrap(),
                if restrict {
                    MergeMode::RestrictConcurrency
                } else {
                    MergeMode::Replace
                },
            )
            .unwrap(),
        )
    }

    #[test]
    fn objects_deep_merge_scalars_and_arrays_replace() {
        let base = json!({"a": {"x": 1, "y": 2}, "b": [1, 2], "c": "s"});
        let over = json!({"a": {"x": 9}, "b": [3], "c": "t", "d": true});
        let merged = merge_json(base.clone(), over, false);
        assert_eq!(merged["a"], json!({"x": 9, "y": 2}));
        assert_eq!(merged["b"], json!([3]));
        assert_eq!(merged["c"], json!("t"));
        assert_eq!(merged["d"], json!(true));
        // Inputs unchanged.
        assert_eq!(base["a"]["x"], json!(1));
    }

    #[test]
    fn restrict_concurrency_takes_min_on_numeric_fields() {
        let base = json!({
            "execution": {"concurrency": {"maxOperations": 8, "mutations": "serialized"}},
            "workflow": {"maxJobs": 4, "artifacts": {"maxReadBytes": 65536}},
            "process": {"maxConcurrent": 2, "maxOutputBytes": 100, "timeout": "2m"},
        });
        let over = json!({
            "execution": {"concurrency": {"maxOperations": 16, "mutations": "per-resource"}},
            "workflow": {"maxJobs": 1, "artifacts": {"maxReadBytes": 99999}},
            "process": {"maxConcurrent": 8, "maxOutputBytes": 50, "timeout": "5m", "killGrace": "1s"},
        });
        let merged = merge_json(base, over, true);
        assert_eq!(
            merged["execution"]["concurrency"]["maxOperations"],
            json!(8)
        );
        assert_eq!(
            merged["execution"]["concurrency"]["mutations"],
            json!("serialized")
        );
        assert_eq!(merged["workflow"]["maxJobs"], json!(1));
        assert_eq!(
            merged["workflow"]["artifacts"]["maxReadBytes"],
            json!(65536)
        );
        assert_eq!(merged["process"]["maxConcurrent"], json!(2));
        assert_eq!(merged["process"]["maxOutputBytes"], json!(50));
        // duration-min: min(2m, 5m) = 2m (existing wins).
        assert_eq!(merged["process"]["timeout"], json!("2m"));
        // killGrace added under restriction → shorter side kept
        // (no existing value → not a string → plain replace).
        assert_eq!(merged["process"]["killGrace"], json!("1s"));
    }

    #[test]
    fn restrict_mutation_mode_only_tightens() {
        let base = json!({"execution": {"concurrency": {"mutations": "per-resource"}}});
        let over = json!({"execution": {"concurrency": {"mutations": "serialized"}}});
        let merged = merge_json(base, over.clone(), true);
        assert_eq!(
            merged["execution"]["concurrency"]["mutations"],
            json!("serialized")
        );
        // serialized cannot be relaxed.
        let base = json!({"execution": {"concurrency": {"mutations": "serialized"}}});
        let over = json!({"execution": {"concurrency": {"mutations": "per-resource"}}});
        let merged = merge_json(base, over, true);
        assert_eq!(
            merged["execution"]["concurrency"]["mutations"],
            json!("serialized")
        );
    }

    #[test]
    fn restrict_duration_min_uses_parsed_ms() {
        let base = json!({"process": {"timeout": "90s"}});
        let over = json!({"process": {"timeout": "2m"}});
        let merged = merge_json(base.clone(), over, true);
        assert_eq!(merged["process"]["timeout"], json!("90s"));
        let over = json!({"process": {"timeout": "1m"}});
        let merged = merge_json(base, over, true);
        assert_eq!(merged["process"]["timeout"], json!("1m"));
    }

    #[test]
    fn replace_mode_replaces_even_for_listed_fields() {
        let base = json!({"process": {"maxConcurrent": 2}});
        let over = json!({"process": {"maxConcurrent": 8}});
        let merged = merge_json(base, over, false);
        assert_eq!(merged["process"]["maxConcurrent"], json!(8));
    }
}
