//! Port of `packages/core/src/policy/workflow.ts`.

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::js;

/// `WorkflowArtifactPolicy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowArtifactPolicy {
    pub max_artifact_bytes: u64,
    pub max_total_bytes: u64,
    pub max_preview_bytes: u64,
    pub max_read_bytes: u64,
}

/// `WorkflowPolicy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowPolicy {
    pub max_jobs: u64,
    pub artifacts: WorkflowArtifactPolicy,
}

const DEFAULT_ARTIFACTS: WorkflowArtifactPolicy = WorkflowArtifactPolicy {
    max_artifact_bytes: 8_388_608,
    max_total_bytes: 33_554_432,
    max_preview_bytes: 2_048,
    max_read_bytes: 65_536,
};

fn cap(minimum: i64) -> Value {
    json!({
        "type": "integer",
        "minimum": minimum,
        "maximum": js::MAX_SAFE_INTEGER as u64,
    })
}

/// `WORKFLOW_POLICY_PROPERTIES` verbatim.
pub fn workflow_policy_properties() -> Value {
    json!({
        "maxJobs": cap(1),
        "artifacts": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "maxArtifactBytes": cap(0),
                "maxTotalBytes": cap(0),
                "maxPreviewBytes": cap(0),
                "maxReadBytes": cap(1),
            },
        },
    })
}

/// `Number.isSafeInteger` on a `serde_json::Value`.
pub(crate) fn is_safe_integer(v: &Value) -> bool {
    crate::js::safe_integer(v).is_some()
}

fn invalid(message: &str) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidRuntimePolicy, message.to_string())
}

/// `resolveWorkflowPolicy`: defaults for omitted fields (trusted merge
/// 後・project restriction 前の defaults と runtime options の正本).
///
/// Input is the raw `workflow` section (`Option<&Value>`): `undefined`
/// and `null`/`{...non-object}` both collapse like the TS `input?.x`
/// access.
pub fn resolve_workflow_policy(input: Option<&Value>) -> Result<WorkflowPolicy, CheaperError> {
    let obj = input.and_then(Value::as_object);
    // `input?.maxJobs ?? 1` — explicit null coalesces to the default
    // (nullish coalescing, not "invalid field").
    let max_jobs = match obj.and_then(|o| o.get("maxJobs")).filter(|v| !v.is_null()) {
        None => Some(1),
        Some(v) => js::safe_u64(v).filter(|n| *n >= 1),
    };
    let Some(max_jobs) = max_jobs else {
        return Err(invalid("invalid workflow.maxJobs"));
    };

    // `{...DEFAULT_ARTIFACTS, ...input?.artifacts}` — defaults occupy the
    // first positions; runtime keys then override (keeping the default
    // position) or append in JS own-enumeration order, so the first
    // invalid field reports in object insertion order. Unknown keys are
    // validated alongside the defaults, then dropped from the typed
    // result — the golden `rustDropUnknownArtifactKeys` marker strips
    // them from the TS expectation for replay.
    let mut merged = Map::new();
    merged.insert(
        "maxArtifactBytes".to_string(),
        json!(DEFAULT_ARTIFACTS.max_artifact_bytes),
    );
    merged.insert(
        "maxTotalBytes".to_string(),
        json!(DEFAULT_ARTIFACTS.max_total_bytes),
    );
    merged.insert(
        "maxPreviewBytes".to_string(),
        json!(DEFAULT_ARTIFACTS.max_preview_bytes),
    );
    merged.insert(
        "maxReadBytes".to_string(),
        json!(DEFAULT_ARTIFACTS.max_read_bytes),
    );
    if let Some(artifacts) = obj.and_then(|o| o.get("artifacts")) {
        js::spread_into(&mut merged, artifacts);
    }
    // `Object.entries` enumerates canonical array-index keys numerically
    // first, then string keys in insertion order — the first invalid
    // field reports in that order.
    let array_index_key = |k: &str| -> Option<u64> {
        k.parse::<u64>()
            .ok()
            .filter(|n| *n < 4_294_967_295 && n.to_string() == k)
    };
    let mut entries: Vec<(&String, &Value)> = merged.iter().collect();
    entries.sort_by_key(|(k, _)| match array_index_key(k) {
        Some(n) => (0, n),
        None => (1, 0),
    });
    for (field, value) in entries {
        let minimum = if field.as_str() == "maxReadBytes" {
            1.0
        } else {
            0.0
        };
        let ok = is_safe_integer(value) && value.as_f64().unwrap_or(f64::NAN) >= minimum;
        if !ok {
            return Err(invalid(&format!("invalid workflow.artifacts.{field}")));
        }
    }
    // All entries are validated safe non-negative integers — the four
    // default names always resolve.
    let field = |name: &str| js::safe_u64(&merged[name]).expect("validated artifact field");
    let artifacts = WorkflowArtifactPolicy {
        max_artifact_bytes: field("maxArtifactBytes"),
        max_total_bytes: field("maxTotalBytes"),
        max_preview_bytes: field("maxPreviewBytes"),
        max_read_bytes: field("maxReadBytes"),
    };
    Ok(WorkflowPolicy {
        max_jobs,
        artifacts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_absent() {
        let p = resolve_workflow_policy(None).unwrap();
        assert_eq!(p.max_jobs, 1);
        assert_eq!(p.artifacts.max_artifact_bytes, 8_388_608);
        assert_eq!(p.artifacts.max_total_bytes, 33_554_432);
        assert_eq!(p.artifacts.max_preview_bytes, 2_048);
        assert_eq!(p.artifacts.max_read_bytes, 65_536);
    }

    #[test]
    fn accepts_overrides() {
        let p = resolve_workflow_policy(Some(&json!({
            "maxJobs": 4,
            "artifacts": {"maxArtifactBytes": 10}
        })))
        .unwrap();
        assert_eq!(p.max_jobs, 4);
        assert_eq!(p.artifacts.max_artifact_bytes, 10);
        assert_eq!(p.artifacts.max_total_bytes, 33_554_432);
    }

    #[test]
    fn rejects_invalid() {
        for raw in [
            json!({"maxJobs": 0}),
            json!({"maxJobs": 1.5}),
            json!({"artifacts": {"maxReadBytes": 0}}),
            json!({"artifacts": {"maxArtifactBytes": -1}}),
            json!({"artifacts": {"maxTotalBytes": "x"}}),
        ] {
            assert!(resolve_workflow_policy(Some(&raw)).is_err(), "{raw}");
        }
    }

    /// `??` and `{...defaults, ...input.artifacts}` spread parity —
    /// expected outcomes probed against TS `resolveWorkflowPolicy`.
    #[test]
    fn nullish_and_spread_semantics() {
        // `maxJobs: null` → default 1 (not "invalid").
        assert_eq!(
            resolve_workflow_policy(Some(&json!({"maxJobs": null})))
                .unwrap()
                .max_jobs,
            1
        );
        // Field null overrides the default and fails safeInteger.
        let e = resolve_workflow_policy(Some(&json!({
            "artifacts": {"maxArtifactBytes": null}
        })))
        .unwrap_err();
        assert_eq!(e.message(), "invalid workflow.artifacts.maxArtifactBytes");
        // Unknown keys are validated in insertion order — defaults hold
        // their positions, so a default name reports before `bogus`
        // even when `bogus` was inserted first.
        let e = resolve_workflow_policy(Some(&json!({
            "artifacts": {"bogus": "x"}
        })))
        .unwrap_err();
        assert_eq!(e.message(), "invalid workflow.artifacts.bogus");
        let e = resolve_workflow_policy(Some(&json!({
            "artifacts": {"bogus": "x", "maxArtifactBytes": "y"}
        })))
        .unwrap_err();
        assert_eq!(e.message(), "invalid workflow.artifacts.maxArtifactBytes");
        // `{..."ab"}` spreads index keys "0"/"1" → `.0` = "a" is not an
        // integer.
        let e = resolve_workflow_policy(Some(&json!({"artifacts": "ab"}))).unwrap_err();
        assert_eq!(e.message(), "invalid workflow.artifacts.0");
        // `{...number}` contributes nothing → defaults preserved.
        assert_eq!(
            resolve_workflow_policy(Some(&json!({"artifacts": 5})))
                .unwrap()
                .artifacts
                .max_artifact_bytes,
            8_388_608
        );
        // Array spread contributes valid integer index entries — valid
        // and dropped from the typed result.
        let p = resolve_workflow_policy(Some(&json!({"artifacts": [1, 2]}))).unwrap();
        assert_eq!(p.artifacts.max_artifact_bytes, 8_388_608);
    }
}
