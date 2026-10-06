//! Port of `packages/core/src/policy/capability.ts`.
//!
//! Capability intersection (DESIGN-0001 / 引継ぎ §25, §27): requested
//! tools ⊄ `policy.tools.allow` → `POLICY_VIOLATION` (with `missing` in
//! details); worker tier > `policy.model.maxTier` → `POLICY_VIOLATION`.

use serde_json::{json, Map};

use crate::errors::{CheaperError, ErrorCode};
use crate::model::tier::compare_tier;
use crate::policy::schema::RuntimePolicy;
use crate::tools::vocabulary::ToolName;
use crate::worker::validate::WorkerDefinition;

/// `checkCapabilities`. On success returns the effective tools (=
/// `worker.tools`); no partial execution.
pub fn check_capabilities(
    worker: &WorkerDefinition,
    policy: &RuntimePolicy,
) -> Result<Vec<ToolName>, CheaperError> {
    let missing: Vec<ToolName> = worker
        .tools
        .iter()
        .copied()
        .filter(|t| !policy.tools.allow.contains(t))
        .collect();
    if !missing.is_empty() {
        let mut details = Map::new();
        details.insert(
            "missing".to_string(),
            json!(missing.iter().map(|t| t.as_str()).collect::<Vec<_>>()),
        );
        return Err(CheaperError::new(
            ErrorCode::PolicyViolation,
            format!(
                "worker requires tools not allowed by policy: {}",
                missing
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
        .with_details(details));
    }
    let requested_tier = worker.cheaper.model.tier;
    if compare_tier(requested_tier, policy.model.max_tier) > 0 {
        let mut details = Map::new();
        details.insert("requestedTier".to_string(), json!(requested_tier.as_str()));
        details.insert("maxTier".to_string(), json!(policy.model.max_tier.as_str()));
        return Err(CheaperError::new(
            ErrorCode::PolicyViolation,
            format!(
                "worker tier {} exceeds policy maxTier {}",
                requested_tier.as_str(),
                policy.model.max_tier.as_str()
            ),
        )
        .with_details(details));
    }
    Ok(worker.tools.clone())
}

#[cfg(test)]
mod tests {
    use serde_json::Map as JsonMap;

    use super::*;
    use crate::model::tier::Tier;
    use crate::policy::schema::*;
    use crate::policy::workflow::{WorkflowArtifactPolicy, WorkflowPolicy};
    use crate::worker::parse::parse_worker_file;
    use crate::worker::validate::{validate_worker_definition, WorkerOrigin};

    fn worker(tools: &str, tier: &str) -> WorkerDefinition {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("w.md");
        let content = format!(
            "---\nname: w\ndescription: d\ntools: {tools}\ncheaper:\n  model: {{tier: {tier}}}\n  output: {{schema: {{inline: {{type: string}}}}}}\n---\nbody\n"
        );
        let parsed = parse_worker_file(&content, p.to_str().unwrap()).unwrap();
        validate_worker_definition(&parsed, p.to_str().unwrap(), WorkerOrigin::Project).unwrap()
    }

    fn policy(max_tier: Tier, allow: &[ToolName]) -> RuntimePolicy {
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
            process: crate::policy::process::ResolvedProcessPolicy {
                max_concurrent: 1,
                tasks: JsonMap::new(),
                timeout: "2m".into(),
                max_output_bytes: 1,
                kill_grace: "2s".into(),
                heartbeat: crate::policy::process::ProcessHeartbeat {
                    interval: "30s".into(),
                },
            },
            execution: ExecutionPolicy {
                timeout_ms: 1,
                max_turns: 1,
                max_tool_calls: 1,
                concurrency: None,
            },
            tools: ToolsPolicy {
                allow: allow.to_vec(),
            },
            filesystem: FilesystemPolicy {
                symlinks: crate::workspace::paths::SymlinkPolicy::Deny,
                deny_paths: vec![],
            },
            model: ModelPolicy {
                max_tier,
                tier_fallback: TierFallback::None,
            },
            output: OutputPolicy {
                max_tokens: 1,
                repair_attempts: 0,
            },
            retry: RetryPolicy {
                transient_attempts: 0,
            },
        }
    }

    #[test]
    fn missing_tools_are_policy_violation_with_details() {
        let w = worker("[read, write]", "low");
        let p = policy(Tier::High, &[ToolName::Read]);
        let e = check_capabilities(&w, &p).unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        assert_eq!(
            e.details().unwrap()["missing"],
            serde_json::json!(["write"])
        );
    }

    #[test]
    fn tier_over_max_is_violation() {
        let w = worker("[read]", "high");
        let p = policy(Tier::Medium, &[ToolName::Read]);
        let e = check_capabilities(&w, &p).unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
    }

    #[test]
    fn allowed_worker_returns_tools() {
        let w = worker("[read, grep]", "low");
        let p = policy(
            Tier::High,
            &[ToolName::Read, ToolName::Grep, ToolName::Write],
        );
        let tools = check_capabilities(&w, &p).unwrap();
        assert_eq!(tools, vec![ToolName::Read, ToolName::Grep]);
    }
}
