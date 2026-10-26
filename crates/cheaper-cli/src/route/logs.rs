use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_tools::artifact::{
    ArtifactProducer, ArtifactReadRequest, ArtifactReader, ArtifactReference, ArtifactSource,
};
use serde_json::{json, Map};

use crate::args::{RouteAction, RouteCliArgs};
use crate::cli::{now, BoxError, CliHooks, CliIo, CommandOut};
use crate::compose::resolve_workspace_layout;
use crate::envelope::utility_success_envelope;
use crate::history::identity::ExecutionIdentity;
use crate::history::schema::{ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent};
use crate::history::store::HistoryStore;
use crate::route::log_store::RouteLogStore;
use crate::usage_log::{record_route_refetch_with_correlation, UsageCorrelation};

pub(crate) async fn execute(
    args: &RouteCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    let RouteAction::Logs {
        id,
        offset,
        max_bytes,
        cwd,
    } = &args.action
    else {
        unreachable!("route logs action required");
    };
    let layout = resolve_workspace_layout(cwd.as_deref(), io, hooks)?;
    let builtin_policy_path = layout.builtin_dir.join("policy.yaml");
    let builtin_policy_path = builtin_policy_path.to_string_lossy();
    let user_config_dir = layout.dirs.user_config_dir.to_string_lossy();
    let project_dir = layout.dirs.project_dir.to_string_lossy();
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path,
        user_config_dir: &user_config_dir,
        project_dir: &project_dir,
        invocation: None,
    })?;
    let cap = policy.workflow.artifacts.max_read_bytes;
    let store = RouteLogStore::new(&layout.dirs.user_config_dir);
    let source = ArtifactSource {
        reference: ArtifactReference {
            artifact: id.clone(),
            run_id: id.clone(),
            producer: ArtifactProducer {
                job_id: "exec".to_string(),
                name: "command".to_string(),
            },
            bytes: 0,
            preview: String::new(),
            truncated: false,
        },
    };
    let request = ArtifactReadRequest {
        offset: *offset,
        max_bytes: max_bytes.unwrap_or(cap),
    };
    // A refetch is an independent execution. Publish its start before touching
    // the referenced log, and close it even when the read fails.
    let identity = ExecutionIdentity::from_inherited(
        (io.env)("CHEAPER_TRACE_ID").as_deref(),
        (io.env)("CHEAPER_PARENT_EXECUTION_ID").as_deref(),
    );
    let history = HistoryStore::new(&layout.dirs.user_config_dir);
    let started = HistoryEvent::new(
        &identity,
        ExecutionKind::RouteRefetch,
        ExecutionEvent::Started,
        ExecutionStatus::Running,
    )
    .with_log_ref(id);
    let history_started = history.write_started(&started);
    let read_started_at = std::time::Instant::now();
    let result = store.reader(cap).read(&source, &request);
    let (status, bytes_read) = match &result {
        Ok(result) => (ExecutionStatus::Completed, Some(result.data.len() as u64)),
        Err(_) => (ExecutionStatus::Failed, None),
    };
    if history_started {
        let terminal = HistoryEvent::new(
            &identity,
            ExecutionKind::RouteRefetch,
            ExecutionEvent::Terminal,
            status,
        )
        .with_log_ref(id)
        .with_duration_ms(Some(read_started_at.elapsed().as_secs_f64() * 1000.0));
        let _ = history.write_terminal(&terminal);
    }
    let result = result?;
    let correlation = UsageCorrelation {
        execution_id: Some(identity.execution_id().to_string()),
        parent_execution_id: identity.parent_execution_id().map(|id| id.to_string()),
        trace_id: Some(identity.trace_id().to_string()),
    };
    record_route_refetch_with_correlation(
        id,
        bytes_read.unwrap_or_default(),
        Some(&correlation),
        &layout.root,
        &user_config_dir,
        io.stderr,
        args.quiet,
        &move || now(hooks),
    );
    let mut out = Map::new();
    out.insert(
        "data".to_string(),
        json!(String::from_utf8_lossy(&result.data)),
    );
    out.insert("offset".to_string(), json!(result.offset));
    out.insert("nextOffset".to_string(), json!(result.next_offset));
    out.insert("eof".to_string(), json!(result.eof));
    Ok(CommandOut {
        envelope: utility_success_envelope(out, now(hooks) - started_at),
        exit_code: 0,
        model_line: None,
    })
}
