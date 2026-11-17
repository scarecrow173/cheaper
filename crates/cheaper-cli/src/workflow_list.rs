//! `workflow list` — project workflow catalog の read-only 一覧
//! (DESIGN-034ZiAzpyOt6fMKWKB2XEE)。catalog の manifest field + status
//! のみを投影し、workflow file の read・compile・worker/model 起動・
//! usage 記録・file 書込みを行わない。

use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_workflow::{read_catalog, CATALOG_RELATIVE_PATH, MAX_CATALOG_ENTRIES};
use serde_json::{json, Value};

use crate::args::WorkflowListCliArgs;
use crate::cli::{now, parse_limit, BoxError, CliHooks, CliIo};
use crate::envelope::utility_success_envelope;

/// `{catalog:{path,present}, workflows, diagnostics, truncated}` を
/// utility envelope で返す。manifest 破損は Err(汎用 catch の error
/// envelope)。
pub(crate) fn execute_list(
    args: &WorkflowListCliArgs,
    io: &CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<Value, BoxError> {
    let limit = args
        .limit
        .as_deref()
        .map(|value| parse_limit(value, MAX_CATALOG_ENTRIES))
        .transpose()?;
    let layout = crate::compose::resolve_workspace_layout(args.cwd.as_deref(), io, hooks)?;
    let builtin_policy_path = layout.builtin_dir.join("policy.yaml");
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path.to_string_lossy(),
        user_config_dir: &layout.dirs.user_config_dir.to_string_lossy(),
        project_dir: &layout.dirs.project_dir.to_string_lossy(),
        invocation: None,
    })?;
    let mut catalog = read_catalog(&layout.root, &policy.filesystem)?;
    let truncated = limit.is_some_and(|limit| catalog.workflows.len() > limit);
    if let Some(limit) = limit {
        catalog.workflows.truncate(limit);
    }
    let Value::Object(result) = json!({
        "catalog": { "path": CATALOG_RELATIVE_PATH, "present": catalog.present },
        "workflows": catalog.workflows,
        "diagnostics": catalog.diagnostics,
        "truncated": truncated,
    }) else {
        unreachable!("json object literal");
    };
    Ok(utility_success_envelope(result, now(hooks) - started_at))
}
