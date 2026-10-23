//! Port of `packages/core/src/policy/restrict.ts`.
//!
//! CLI-style policy restriction (引継ぎ §30 / shared ADR D2): `--timeout`
//! exceeding `policy.execution.timeoutMs` is `POLICY_VIOLATION` (no silent
//! clamp); concurrency may only be narrowed.

use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::policy::schema::{resolve_concurrency_policy, MutationsMode, RuntimePolicy};

/// `restrictPolicy` options.
#[derive(Debug, Clone, Default)]
pub struct PolicyRestrictions {
    /// `restrictions.timeoutMs` — a `u64` field, so JSON `null`/string
    /// values cannot be expressed; TS would assign them to the result
    /// verbatim (`{timeoutMs: null}` → `timeoutMs: null` — residual
    /// divergence, the typed `u64` keeps the previous value instead).
    pub timeout_ms: Option<u64>,
    /// `restrictions.concurrency` verbatim: `None` = key absent
    /// (`undefined`); `Some(Value::Null)` = explicit null — present, so
    /// the `{...current, ...restrictions.concurrency}` merge still runs
    /// (spreading null contributes nothing).
    pub concurrency: Option<Value>,
}

/// `restrictPolicy`.
pub fn restrict_policy(
    policy: &RuntimePolicy,
    restrictions: &PolicyRestrictions,
) -> Result<RuntimePolicy, CheaperError> {
    let requested = restrictions.timeout_ms;
    if requested.is_none() && restrictions.concurrency.is_none() {
        return Ok(policy.clone());
    }
    if let Some(req) = requested {
        if req > policy.execution.timeout_ms {
            let mut details = Map::new();
            details.insert("requested".to_string(), json!(req));
            details.insert("policy".to_string(), json!(policy.execution.timeout_ms));
            return Err(CheaperError::new(
                ErrorCode::PolicyViolation,
                format!(
                    "--timeout {}ms exceeds policy timeout {}ms",
                    req, policy.execution.timeout_ms
                ),
            )
            .with_details(details));
        }
    }
    let mut execution = policy.execution.clone();
    if let Some(req) = requested {
        execution.timeout_ms = req;
    }
    if let Some(restriction) = restrictions.concurrency.as_ref() {
        // `{...current, ...restrictions.concurrency}` over the resolved
        // current value — the same own-enumerable spread rules as the
        // workflow artifacts merge, so `{maxOperations: null}` lands as
        // null in the merge and `??` resolves it to the *default* (8),
        // not the current value.
        let current = resolve_concurrency_policy(
            policy
                .execution
                .concurrency
                .map(|c| {
                    json!({
                        "maxOperations": c.max_operations,
                        "mutations": c.mutations.as_str(),
                    })
                })
                .as_ref(),
        )?;
        let mut merged = Map::new();
        merged.insert("maxOperations".to_string(), json!(current.max_operations));
        merged.insert("mutations".to_string(), json!(current.mutations.as_str()));
        crate::js::spread_into(&mut merged, restriction);
        let merged_input = Value::Object(merged);
        let next = resolve_concurrency_policy(Some(&merged_input))?;
        if next.max_operations > current.max_operations
            || (current.mutations == MutationsMode::Serialized
                && next.mutations != MutationsMode::Serialized)
        {
            let mut details = Map::new();
            details.insert(
                "requested".to_string(),
                serde_json::to_value(next).unwrap_or(Value::Null),
            );
            details.insert(
                "policy".to_string(),
                serde_json::to_value(current).unwrap_or(Value::Null),
            );
            return Err(CheaperError::new(
                ErrorCode::PolicyViolation,
                "concurrency restriction exceeds runtime policy",
            )
            .with_details(details));
        }
        execution.concurrency = Some(next);
    }
    Ok(RuntimePolicy {
        execution,
        ..policy.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::schema::*;
    use crate::policy::security::SecurityPolicy;
    use crate::policy::workflow::{WorkflowArtifactPolicy, WorkflowPolicy};

    fn policy() -> RuntimePolicy {
        RuntimePolicy {
            workflow: WorkflowPolicy {
                max_jobs: 1,
                artifacts: WorkflowArtifactPolicy {
                    max_artifact_bytes: 1,
                    max_total_bytes: 1,
                    max_preview_bytes: 1,
                    max_read_bytes: 1,
                },
            },
            exec: crate::policy::exec::ResolvedExecPolicy {
                grants: std::collections::BTreeMap::new(),
                command: crate::policy::exec::ExecDefaults {
                    max_concurrent: 1,
                    timeout: "2m".to_string(),
                    max_output_bytes: 65536,
                    kill_grace: "2s".to_string(),
                    heartbeat: crate::policy::exec::ExecHeartbeat {
                        interval: "30s".to_string(),
                    },
                },
                process: crate::policy::exec::ExecDefaults {
                    max_concurrent: 1,
                    timeout: "2m".to_string(),
                    max_output_bytes: 65536,
                    kill_grace: "2s".to_string(),
                    heartbeat: crate::policy::exec::ExecHeartbeat {
                        interval: "30s".to_string(),
                    },
                },
            },
            route: RoutePolicy::default(),
            security: SecurityPolicy::default(),
            execution: ExecutionPolicy {
                timeout_ms: 300_000,
                max_turns: Some(10),
                max_tool_calls: 20,
                concurrency: Some(ConcurrencyPolicy {
                    max_operations: 8,
                    mutations: MutationsMode::Serialized,
                }),
            },
            tools: ToolsPolicy {
                allow: vec![crate::tools::vocabulary::ToolName::Read],
            },
            filesystem: FilesystemPolicy {
                symlinks: crate::workspace::paths::SymlinkPolicy::Deny,
                deny_paths: vec![],
            },
            model: ModelPolicy {
                max_tier: crate::model::tier::Tier::High,
                tier_fallback: TierFallback::None,
            },
            output: OutputPolicy {
                max_tokens: 4096,
                repair_attempts: 0,
            },
            retry: RetryPolicy {
                transient_attempts: 0,
            },
        }
    }

    #[test]
    fn empty_restrictions_return_policy_unchanged() {
        let p = policy();
        let r = restrict_policy(&p, &PolicyRestrictions::default()).unwrap();
        assert_eq!(r.execution.timeout_ms, 300_000);
    }

    #[test]
    fn timeout_over_policy_is_violation() {
        let p = policy();
        let e = restrict_policy(
            &p,
            &PolicyRestrictions {
                timeout_ms: Some(300_001),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        let ok = restrict_policy(
            &p,
            &PolicyRestrictions {
                timeout_ms: Some(60_000),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(ok.execution.timeout_ms, 60_000);
    }

    #[test]
    fn concurrency_can_only_shrink() {
        let p = policy();
        // Wider maxOperations → violation.
        let e = restrict_policy(
            &p,
            &PolicyRestrictions {
                concurrency: Some(json!({"maxOperations": 16})),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        // serialized → per-resource is a widening → violation.
        let e = restrict_policy(
            &p,
            &PolicyRestrictions {
                concurrency: Some(json!({"mutations": "per-resource"})),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        // Narrower is accepted.
        let ok = restrict_policy(
            &p,
            &PolicyRestrictions {
                concurrency: Some(json!({"maxOperations": 2})),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(ok.execution.concurrency.unwrap().max_operations, 2);
    }

    /// `{...current, ...restrictions.concurrency}` spread/nullish parity —
    /// outcomes probed against TS `restrictPolicy` (policy
    /// `maxOperations: 4` in the probe).
    #[test]
    fn concurrency_restriction_null_semantics() {
        let mut p = policy();
        p.execution.concurrency = Some(ConcurrencyPolicy {
            max_operations: 4,
            mutations: MutationsMode::Serialized,
        });
        // `maxOperations: null` merges as null → `??` → *default* 8
        // (not the current 4) → widening → violation.
        let e = restrict_policy(
            &p,
            &PolicyRestrictions {
                concurrency: Some(json!({"maxOperations": null})),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        // `mutations: null` → `??` → "serialized" → same as current.
        let ok = restrict_policy(
            &p,
            &PolicyRestrictions {
                concurrency: Some(json!({"mutations": null})),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(ok.execution.concurrency.unwrap().max_operations, 4);
        // `concurrency: null` is present (`!== undefined`) → the merge
        // runs but spreads nothing → resolved current, no violation.
        let ok = restrict_policy(
            &p,
            &PolicyRestrictions {
                concurrency: Some(Value::Null),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(ok.execution.concurrency.unwrap().max_operations, 4);
    }
}
