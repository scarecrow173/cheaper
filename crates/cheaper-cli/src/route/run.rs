use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cheaper_core::config::dirs::EnvLookup;
use cheaper_core::duration::parse_duration;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::workspace::paths::canonicalize_workspace_root;
use cheaper_tools::artifact::{
    ArtifactReadRequest, ArtifactReadResult, ArtifactReader, ArtifactSource,
};
use cheaper_tools::catalog::CanonicalToolResult;
use cheaper_tools::tools::exec::RouteCommandContext;
use cheaper_tools::tools::proc::ProcTee;
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use super::log_store::{is_valid_run_id, new_run_id, RouteLogStore, RouteLogWriter};
use super::payload::{self, RoutePayload};
use super::resolve::{self, ResolveInput, RouteResolution, Viability};
use super::routes::{Route, RouteLayer};
use super::supply::{self, RouteSupply};
use crate::cli::{now, BoxError, CliHooks, CliIo};
use crate::envelope::utility_success_envelope;
use crate::errors::invalid_request;
use crate::hook::routing_policy::tokenize_shell;
use crate::usage_log::{record_route, RouteRecordInput};
use crate::workflow_context::WorkflowContext;

pub(crate) struct PreparedRoute {
    pub supply: RouteSupply,
    pub resolution: RouteResolution,
    pub route: Route,
    pub extra_env: Map<String, Value>,
}

fn rejected(reason: &'static str) -> CheaperError {
    invalid_request(
        "route run payload rejected",
        Some(Map::from_iter([(
            "reason".to_string(),
            Value::from(reason),
        )])),
    )
}

fn denied(reason: &'static str) -> CheaperError {
    CheaperError::new(ErrorCode::PolicyViolation, "route run revalidation failed").with_details(
        Map::from_iter([("reason".to_string(), Value::from(reason))]),
    )
}

pub(crate) fn prepare(
    payload: &RoutePayload,
    cwd: &str,
    env: EnvLookup<'_>,
    builtin_dir: Option<&Path>,
) -> Result<PreparedRoute, CheaperError> {
    let mut supply = supply::load(cwd, env, builtin_dir)?;
    let payload_root =
        canonicalize_workspace_root(&payload.cwd).map_err(|_| rejected("cwd-mismatch"))?;
    if payload_root != supply.root {
        return Err(rejected("cwd-mismatch"));
    }
    let segments = tokenize_shell(&payload.command);
    let segment = match segments.as_deref() {
        Some([segment]) if !segment.tokens.is_empty() => segment,
        _ => return Err(rejected("unsafe-command")),
    };
    if segment.unsafe_segment || segment.tokens.iter().any(|t| t.unsafe_token) {
        return Err(rejected("unsafe-command"));
    }
    let tokens: Vec<String> = segment.tokens.iter().map(|t| t.text.clone()).collect();
    let (prefix, rest) = resolve::split_env_prefix(&tokens);
    if payload.argv.is_empty()
        || !resolve::split_env_prefix(&payload.argv).0.is_empty()
        || rest != payload.argv.as_slice()
    {
        return Err(rejected("argv-mismatch"));
    }
    let view = supply.exec_view();
    let resolution = resolve::resolve(&ResolveInput {
        argv: &payload.argv,
        cwd: &payload_root,
        routes: &supply.routes,
        exec_policy: &view,
        unsafe_command: false,
    });
    if !resolution.matched {
        return Err(denied("no-match"));
    }
    if resolution.viability != Some(Viability::Runnable) {
        return Err(denied(
            resolution.viability.map_or("not-runnable", |v| v.as_str()),
        ));
    }
    let route = supply
        .routes
        .routes
        .iter()
        .filter(|r| payload.argv.starts_with(r.command.as_slice()))
        .max_by_key(|r| r.command.len())
        .filter(|r| Some(r.name.as_str()) == resolution.route.as_deref())
        .cloned()
        .ok_or_else(|| denied("route-mismatch"))?;
    if payload
        .env
        .keys()
        .any(|k| !route.passthrough_env.iter().any(|p| p == k))
    {
        return Err(rejected("env-not-allowed"));
    }
    let mut extra_env = Map::new();
    for (k, v) in &payload.env {
        extra_env.insert(k.clone(), Value::from(v.clone()));
    }
    for (k, v) in prefix {
        extra_env.insert(k, Value::from(v));
    }
    if !supply.policy.exec.project_grants(&route.grant_set()) {
        return Err(denied("no-grant"));
    }
    Ok(PreparedRoute {
        supply,
        resolution,
        route,
        extra_env,
    })
}

pub(crate) struct RouteExecution {
    store: RouteLogStore,
    run_id: String,
    writer: Mutex<Option<RouteLogWriter>>,
    count_cap: u64,
    max_read_bytes: u64,
    extra_env: Map<String, Value>,
    exec_cwd: String,
    expected_input: Value,
    failures: Mutex<Vec<String>>,
    captured: Mutex<Option<CanonicalToolResult>>,
    log_ref: Mutex<Option<String>>,
    write_failed: AtomicBool,
}

impl RouteExecution {
    fn new(
        store: RouteLogStore,
        extra_env: Map<String, Value>,
        exec_cwd: String,
        grant: &str,
        exec_args: &[String],
        count_cap: u64,
        max_read_bytes: u64,
    ) -> Self {
        let mut failures = Vec::new();
        if store.sweep().is_err() {
            failures.push("log-sweep".to_string());
        }
        let (run_id, writer) = match store.begin() {
            Ok(writer) => (writer.run_id().to_string(), Some(writer)),
            Err(_) => {
                failures.push("log-begin".to_string());
                (new_run_id(), None)
            }
        };
        Self {
            store,
            run_id,
            writer: Mutex::new(writer),
            count_cap,
            max_read_bytes,
            extra_env,
            exec_cwd,
            expected_input: json!({"name": grant, "args": exec_args}),
            failures: Mutex::new(failures),
            captured: Mutex::new(None),
            log_ref: Mutex::new(None),
            write_failed: AtomicBool::new(false),
        }
    }

    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    pub(crate) fn expected_input(&self) -> &Value {
        &self.expected_input
    }

    fn push_failure(&self, stage: &str) {
        if let Ok(mut failures) = self.failures.lock() {
            failures.push(stage.to_string());
        }
    }

    pub(crate) fn failures(&self) -> Vec<String> {
        self.failures.lock().map(|f| f.clone()).unwrap_or_default()
    }

    pub(crate) fn captured(&self) -> Option<CanonicalToolResult> {
        self.captured.lock().ok().and_then(|c| c.clone())
    }

    pub(crate) fn log_ref(&self) -> Option<String> {
        self.log_ref.lock().ok().and_then(|r| r.clone())
    }

    pub(crate) fn compose_reader(
        self: &Arc<Self>,
        inner: Arc<dyn ArtifactReader>,
    ) -> Arc<dyn ArtifactReader> {
        Arc::new(ComposedReader {
            route: self.store.reader(self.max_read_bytes),
            inner,
        })
    }

    pub(crate) fn context(self: &Arc<Self>) -> RouteCommandContext {
        let this = Arc::clone(self);
        let tee: ProcTee = Arc::new(move |chunk: &[u8]| this.tee_write(chunk));
        RouteCommandContext {
            extra_env: self.extra_env.clone(),
            exec_cwd: Some(self.exec_cwd.clone()),
            accept_any_exit_code: true,
            log_tee: Some(tee),
        }
    }

    fn tee_write(&self, chunk: &[u8]) {
        let failed = match self.writer.lock() {
            Ok(mut guard) => match guard.as_mut() {
                Some(writer) => writer.write_all(chunk).is_err(),
                None => false,
            },
            Err(_) => true,
        };
        if failed && !self.write_failed.swap(true, Ordering::Relaxed) {
            self.push_failure("log-write");
        }
    }

    pub(crate) fn complete(&self, result: &mut CanonicalToolResult) {
        let writer = self.writer.lock().ok().and_then(|mut g| g.take());
        if let Some(mut writer) = writer {
            if writer.flush().is_err() {
                self.push_failure("log-flush");
            }
            let tmp = writer.tmp_path().to_path_buf();
            let read = std::fs::File::open(&tmp).and_then(|file| {
                let mut buf = Vec::new();
                file.take(self.count_cap.saturating_add(1))
                    .read_to_end(&mut buf)
                    .map(|_| buf)
            });
            match read {
                Ok(bytes) => {
                    let partial = bytes.len() as u64 > self.count_cap;
                    let text = String::from_utf8_lossy(
                        &bytes[..self.count_cap.min(bytes.len() as u64) as usize],
                    )
                    .into_owned();
                    match cheaper_core::token::count::count_text_tokens(&text) {
                        Ok(tokens) => {
                            if partial {
                                self.push_failure("partial-count");
                            }
                            Self::set_detail(result, "outputTokens", json!(tokens));
                            Self::set_detail(result, "outputTokensTruncated", json!(partial));
                        }
                        Err(_) => self.push_failure("token-count"),
                    }
                }
                Err(_) => self.push_failure("token-count"),
            }
            match writer.finish() {
                Ok(_) => {
                    *self.log_ref.lock().expect("log_ref poisoned") = Some(self.run_id.clone());
                    Self::set_detail(result, "logRef", json!(self.run_id));
                }
                Err(_) => self.push_failure("log-finish"),
            }
        }
        if let Ok(mut captured) = self.captured.lock() {
            *captured = Some(result.clone());
        }
    }

    fn set_detail(result: &mut CanonicalToolResult, key: &str, value: Value) {
        if result.details.is_none() {
            result.details = Some(json!({}));
        }
        if let Some(map) = result.details.as_mut().and_then(Value::as_object_mut) {
            map.insert(key.to_string(), value);
        }
    }
}

struct ComposedReader {
    route: super::log_store::RouteLogReader,
    inner: Arc<dyn ArtifactReader>,
}

impl ArtifactReader for ComposedReader {
    fn read(
        &self,
        source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, ToolError> {
        if is_valid_run_id(&source.reference.run_id) {
            self.route.read(source, request)
        } else {
            self.inner.read(source, request)
        }
    }
}

pub struct RouteRunOutput {
    pub envelope: Value,
    pub text: String,
    pub exit_code: i32,
}

fn deadline_ms(grant_timeout: &str) -> Result<u64, CheaperError> {
    let timeout = parse_duration(grant_timeout).map_err(|_| {
        CheaperError::new(ErrorCode::InvalidRuntimePolicy, "invalid grant timeout").with_details(
            Map::from_iter([("reason".to_string(), json!("grant-timeout"))]),
        )
    })?;
    Ok(timeout.saturating_add(120_000))
}

pub async fn execute(
    token: &str,
    cwd: Option<&str>,
    quiet: bool,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<RouteRunOutput, BoxError> {
    let payload = payload::decode(token)?;
    let prepared = prepare(
        &payload,
        cwd.unwrap_or(&io.cwd),
        io.env,
        hooks.builtin_dir.as_deref(),
    )?;
    let layout = crate::compose::resolve_workspace_layout(cwd, io, hooks)?;
    let file_root = match prepared.route.layer {
        RouteLayer::Builtin => layout.builtin_dir.join("workflows"),
        RouteLayer::User => layout.dirs.user_config_dir.clone(),
        RouteLayer::Project => layout.dirs.project_dir.clone(),
    };
    let file_root = canonicalize_workspace_root(&file_root.to_string_lossy())?;
    let grant_timeout = prepared
        .supply
        .policy
        .exec
        .grants
        .get(&prepared.route.grant)
        .map(|g| g.timeout.clone())
        .ok_or_else(|| denied("no-grant"))?;
    let deadline = deadline_ms(&grant_timeout)?;
    let mut policy = prepared.supply.policy.clone();
    policy.execution.timeout_ms = deadline;
    let ctx = WorkflowContext::from_policy(
        prepared.supply.root.clone(),
        layout.dirs.clone(),
        policy.clone(),
    );
    let count_cap = crate::count::count_max_bytes(io.env)
        .unwrap_or(cheaper_core::token::count::DEFAULT_COUNT_MAX_BYTES);
    let route_exec = Arc::new(RouteExecution::new(
        RouteLogStore::new(&layout.dirs.user_config_dir),
        prepared.extra_env.clone(),
        prepared.supply.root.clone(),
        &prepared.route.grant,
        &prepared.resolution.exec_args,
        count_cap,
        policy.workflow.artifacts.max_read_bytes,
    ));
    let run_id = route_exec.run_id().to_string();
    let inputs = Map::from_iter([
        ("grant".to_string(), json!(prepared.route.grant)),
        (
            "executable".to_string(),
            json!(prepared.resolution.executable),
        ),
        ("argv".to_string(), json!(prepared.resolution.argv)),
        ("args".to_string(), json!(prepared.resolution.args)),
        ("execArgs".to_string(), json!(prepared.resolution.exec_args)),
        ("cwd".to_string(), json!(prepared.supply.root)),
        (
            "tokenThreshold".to_string(),
            json!(prepared.route.token_threshold.unwrap_or(8192.0)),
        ),
        ("runId".to_string(), json!(run_id)),
    ]);
    let outcome = crate::workflow_run::execute_routed(crate::workflow_run::RoutedRun {
        ctx: &ctx,
        file_root: &file_root,
        workflow_path: &prepared.route.target_path.to_string_lossy(),
        inputs,
        effective_policy: &policy,
        route_exec: Arc::clone(&route_exec),
        io,
        hooks,
    })
    .await?;
    finish(
        outcome,
        FinishArgs {
            prepared: &prepared,
            route_exec: &route_exec,
            payload: &payload,
            user_config_dir: &layout.dirs.user_config_dir.to_string_lossy(),
            hooks,
            quiet,
            started_at,
        },
        io,
    )
}

struct FinishArgs<'a> {
    prepared: &'a PreparedRoute,
    route_exec: &'a Arc<RouteExecution>,
    payload: &'a RoutePayload,
    user_config_dir: &'a str,
    hooks: &'a CliHooks,
    quiet: bool,
    started_at: f64,
}

fn finish(
    outcome: Result<cheaper_workflow::WorkflowOutcome, CheaperError>,
    args: FinishArgs<'_>,
    io: &mut CliIo<'_>,
) -> Result<RouteRunOutput, BoxError> {
    let FinishArgs {
        prepared,
        route_exec,
        payload,
        user_config_dir,
        hooks,
        quiet,
        started_at,
    } = args;
    let Some(captured) = route_exec.captured() else {
        return Err(match outcome {
            Err(e) => Box::new(e) as BoxError,
            Ok(outcome) => match outcome.error {
                Some(error) => Box::new(CheaperError::new(
                    error.code,
                    "routed command did not complete",
                )) as BoxError,
                None => Box::new(CheaperError::new(
                    ErrorCode::InternalError,
                    "routed exec result was not captured",
                )) as BoxError,
            },
        });
    };
    let details = captured.details.as_ref().and_then(Value::as_object);
    let detail_i64 = |key: &str| details.and_then(|d| d.get(key)).and_then(Value::as_i64);
    let exit_code = detail_i64("exitCode")
        .and_then(|v| i32::try_from(v).ok())
        .ok_or_else(|| {
            Box::new(CheaperError::new(
                ErrorCode::InternalError,
                "captured exec result has no valid exit code",
            )) as BoxError
        })?;
    let stream = |key: &str| -> (String, bool) {
        let s = details.and_then(|d| d.get(key)).and_then(Value::as_object);
        (
            s.and_then(|s| s.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            s.and_then(|s| s.get("truncated"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        )
    };
    let (stdout_text, stdout_truncated) = stream("stdout");
    let (stderr_text, stderr_truncated) = stream("stderr");
    let mut failures = route_exec.failures();
    let mut post_failed = false;
    match &outcome {
        Err(_) => {
            post_failed = true;
            failures.push("workflow-error".to_string());
        }
        Ok(outcome) => {
            for job in &outcome.jobs {
                if job.id != "exec"
                    && matches!(
                        job.status,
                        cheaper_workflow::JobStatus::Failed
                            | cheaper_workflow::JobStatus::Cancelled
                    )
                {
                    post_failed = true;
                    failures.push(format!("job-failed:{}", job.id));
                }
            }
            for error in &outcome.meta.output_errors {
                post_failed = true;
                failures.push(format!("output-error:{}", error.name));
            }
            if (!outcome.ok || outcome.error.is_some()) && !post_failed {
                post_failed = true;
                failures.push("workflow-error".to_string());
            }
        }
    }
    let outcome = outcome.ok();
    let clean = outcome.as_ref().is_some_and(|o| o.ok && o.error.is_none()) && !post_failed;
    let fallback = match (stdout_text.is_empty(), stderr_text.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout_text,
        (true, false) => stderr_text,
        (false, false) => format!("{stdout_text}\n{stderr_text}"),
    };
    let body = if clean {
        outcome
            .as_ref()
            .and_then(|o| o.result.get("result"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or(fallback)
    } else {
        fallback
    };
    let log_ref = route_exec.log_ref();
    let truncated = stdout_truncated || stderr_truncated;
    let text = format!(
        "{body}\n[cheaper route log: {}; truncated: {truncated}]\n",
        log_ref.as_deref().unwrap_or("unavailable"),
    );
    let returned_tokens = match cheaper_core::token::count::count_text_tokens(&text) {
        Ok(tokens) => tokens as f64,
        Err(_) => {
            failures.push("returned-token-count".to_string());
            0.0
        }
    };
    let original_tokens = detail_i64("outputTokens").map(|v| v as f64).unwrap_or(0.0);
    let analyze_meta = outcome
        .as_ref()
        .and_then(|o| o.jobs.iter().find(|j| j.id == "analyze"))
        .map(|j| &j.meta);
    let worker = analyze_meta
        .map(|meta| {
            let mut m = Map::new();
            for key in ["model", "usage"] {
                if let Some(v) = meta.extra.get(key) {
                    m.insert(key.to_string(), v.clone());
                }
            }
            Value::Object(m)
        })
        .unwrap_or(Value::Object(Map::new()));
    let analyze_ms = analyze_meta.and_then(|m| m.duration_ms).unwrap_or(0.0);
    let exec_ms = detail_i64("durationMs").map(|v| v as f64).unwrap_or(0.0);
    let duration_ms = json!({
        "exec": exec_ms,
        "analyze": analyze_ms,
        "total": now(hooks) - started_at,
    });
    let recorded = record_route(
        &RouteRecordInput {
            route: prepared.route.name.clone(),
            command: payload.command.clone(),
            original_tokens,
            returned_tokens,
            worker,
            duration_ms,
            log_ref: log_ref.unwrap_or_default(),
            refetches: 0.0,
            postprocess_failures: failures.len() as f64,
            suppressed: prepared.resolution.suppressed.clone(),
            exit_code,
            ok: exit_code == 0 && failures.is_empty(),
        },
        &prepared.supply.root,
        user_config_dir,
        io.stderr,
        quiet,
        &move || now(hooks),
    );
    if !recorded {
        failures.push("usage-record".to_string());
    }
    let mut result = Map::new();
    result.insert("output".to_string(), json!(text));
    result.insert(
        "execResult".to_string(),
        serde_json::to_value(&captured).unwrap_or(Value::Null),
    );
    if let Some(log_ref) = &route_exec.log_ref() {
        result.insert("logRef".to_string(), json!(log_ref));
    }
    result.insert("postprocessFailures".to_string(), json!(failures));
    Ok(RouteRunOutput {
        envelope: utility_success_envelope(result, now(hooks) - started_at),
        text,
        exit_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    struct Fx {
        _tmp: tempfile::TempDir,
        ws: String,
        cfg: PathBuf,
        builtin: PathBuf,
    }

    fn fixture() -> Fx {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let ws = root.join("ws");
        let cfg = root.join("cfg");
        let builtin = root.join("builtin");
        std::fs::create_dir_all(ws.join(".cheaper")).unwrap();
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::create_dir_all(&builtin).unwrap();
        std::fs::write(
            builtin.join("policy.yaml"),
            include_str!("../../resources/policy.yaml"),
        )
        .unwrap();
        Fx {
            _tmp: tmp,
            ws: ws.to_string_lossy().into_owned(),
            cfg,
            builtin,
        }
    }

    fn test_exe() -> String {
        std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    fn routed_fx() -> Fx {
        let fx = fixture();
        std::fs::write(
            fx.cfg.join("routes.yaml"),
            "version: 1\nroutes:\n  - name: cargo-test\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: cargo-test\n    passthroughEnv: [CI, FOO]\n",
        )
        .unwrap();
        std::fs::write(
            fx.cfg.join("routed-exec.workflow.json"),
            r#"{"version":1,"jobs":{}}"#,
        )
        .unwrap();
        let exe = test_exe();
        std::fs::write(
            fx.cfg.join("policy.yaml"),
            format!(
                "version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: '{exe}'\n      argv: [test, \"*\"]\n      env:\n        FOO: grant-val\n        KEEP: keep\n    other:\n      executable: '{exe}'\n      argv: [\"*\"]\n"
            ),
        )
        .unwrap();
        fx
    }

    fn env_lookup(cfg: &Path) -> impl Fn(&str) -> Option<String> {
        let cfg = cfg.to_string_lossy().into_owned();
        move |k: &str| (k == "CHEAPER_CONFIG_DIR").then(|| cfg.clone())
    }

    fn payload(
        ws: &str,
        command: &str,
        argv: &[&str],
        env: BTreeMap<String, String>,
    ) -> RoutePayload {
        RoutePayload {
            command: command.to_string(),
            executable: "cargo".to_string(),
            argv: argv.iter().map(|s| s.to_string()).collect(),
            args: Vec::new(),
            cwd: ws.to_string(),
            env,
        }
    }

    #[test]
    fn prepare_projects_grant_set() {
        let fx = routed_fx();
        // route に member `other` を宣言(user 層 — trusted
        // authoring) — `grant_set()` = {cargo-test, other} が投影に
        // 残り、未宣言 grant は落ちる。
        std::fs::write(
            fx.cfg.join("routes.yaml"),
            "version: 1\nroutes:\n  - name: cargo-test\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: cargo-test\n    grants: [other]\n    passthroughEnv: [CI, FOO]\n",
        )
        .unwrap();
        let exe = test_exe();
        std::fs::write(
            fx.cfg.join("policy.yaml"),
            format!(
                "version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: '{exe}'\n      argv: [test, \"*\"]\n    other:\n      executable: '{exe}'\n      argv: [\"*\"]\n    undeclared:\n      executable: '{exe}'\n      argv: [\"*\"]\n"
            ),
        )
        .unwrap();
        let env = env_lookup(&fx.cfg);
        let p = payload(
            &fx.ws,
            "cargo test --workspace",
            &["cargo", "test", "--workspace"],
            BTreeMap::new(),
        );
        let out = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).unwrap();
        // `project_grants` は `{route.grant} ∪ grants ∪ {fixed
        // grants}` を残す — undeclared は投影から落ちる。
        let grants = &out.supply.policy.exec.grants;
        assert_eq!(grants.len(), 2);
        assert!(grants.contains_key("cargo-test"));
        assert!(grants.contains_key("other"));
        assert!(!grants.contains_key("undeclared"));
        assert_eq!(out.route.name, "cargo-test");
        assert_eq!(out.route.grant, "cargo-test");
        assert_eq!(out.resolution.exec_args, vec!["test", "--workspace"]);
    }

    #[test]
    fn prepare_merges_prefix_over_ambient_and_preserves_grant_env() {
        let fx = routed_fx();
        let env = env_lookup(&fx.cfg);
        let p = payload(
            &fx.ws,
            "FOO=prefix BAR=1 cargo test",
            &["cargo", "test"],
            BTreeMap::from([
                ("FOO".to_string(), "ambient".to_string()),
                ("CI".to_string(), "1".to_string()),
            ]),
        );
        let out = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).unwrap();
        assert_eq!(out.extra_env["FOO"], Value::from("prefix"));
        assert_eq!(out.extra_env["BAR"], Value::from("1"));
        assert_eq!(out.extra_env["CI"], Value::from("1"));
        let grant = &out.supply.policy.exec.grants["cargo-test"];
        assert_eq!(grant.env["FOO"], Value::from("grant-val"));
        assert_eq!(grant.env["KEEP"], Value::from("keep"));
    }

    #[test]
    fn prepare_rejects_command_argv_mismatch() {
        let fx = routed_fx();
        let env = env_lookup(&fx.cfg);
        let p = payload(
            &fx.ws,
            "cargo test --workspace",
            &["cargo", "check"],
            BTreeMap::new(),
        );
        let err = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).err().unwrap();
        assert_eq!(err.code(), ErrorCode::InvalidRequest);
        let p = payload(
            &fx.ws,
            "cargo test",
            &["CI=1", "cargo", "test"],
            BTreeMap::new(),
        );
        assert!(prepare(&p, &fx.ws, &env, Some(&fx.builtin)).is_err());
    }

    #[test]
    fn prepare_rejects_compound_and_unsafe_command() {
        let fx = routed_fx();
        let env = env_lookup(&fx.cfg);
        for command in [
            "cargo test && echo done",
            "cargo test > out.txt",
            "cargo test | wc -l",
            "cargo 'test",
            "CI=1",
            "",
        ] {
            let p = payload(&fx.ws, command, &["cargo", "test"], BTreeMap::new());
            assert!(
                prepare(&p, &fx.ws, &env, Some(&fx.builtin)).is_err(),
                "{command}"
            );
        }
    }

    #[test]
    fn prepare_rejects_foreign_cwd() {
        let fx = routed_fx();
        let other = tempfile::tempdir().unwrap();
        let env = env_lookup(&fx.cfg);
        let mut p = payload(&fx.ws, "cargo test", &["cargo", "test"], BTreeMap::new());
        p.cwd = other.path().to_string_lossy().into_owned();
        let err = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).err().unwrap();
        assert_eq!(err.code(), ErrorCode::InvalidRequest);
    }

    #[test]
    fn prepare_rejects_unallowlisted_env_without_leak() {
        let fx = routed_fx();
        let env = env_lookup(&fx.cfg);
        let mut p = payload(&fx.ws, "cargo test", &["cargo", "test"], BTreeMap::new());
        p.env
            .insert("SECRET".to_string(), "s3cr3t-value".to_string());
        let err = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).err().unwrap();
        let rendered = format!("{err:?}");
        assert!(!rendered.contains("s3cr3t-value"), "{rendered}");
        assert!(!rendered.contains("SECRET"), "{rendered}");
    }

    #[test]
    fn prepare_rejects_when_grant_missing() {
        let fx = fixture();
        std::fs::write(
            fx.cfg.join("routes.yaml"),
            "version: 1\nroutes:\n  - name: cargo-test\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: absent\n",
        )
        .unwrap();
        std::fs::write(fx.cfg.join("routed-exec.workflow.json"), "{}").unwrap();
        let exe = test_exe();
        std::fs::write(
            fx.cfg.join("policy.yaml"),
            format!("version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: '{exe}'\n      argv: [test, \"*\"]\n"),
        )
        .unwrap();
        let env = env_lookup(&fx.cfg);
        let p = payload(&fx.ws, "cargo test", &["cargo", "test"], BTreeMap::new());
        let err = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).err().unwrap();
        assert_eq!(err.code(), ErrorCode::PolicyViolation);
    }

    #[test]
    fn prepare_rejects_when_target_missing() {
        let fx = fixture();
        std::fs::write(
            fx.cfg.join("routes.yaml"),
            "version: 1\nroutes:\n  - name: cargo-test\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: cargo-test\n",
        )
        .unwrap();
        let exe = test_exe();
        std::fs::write(
            fx.cfg.join("policy.yaml"),
            format!("version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: '{exe}'\n      argv: [test, \"*\"]\n"),
        )
        .unwrap();
        let env = env_lookup(&fx.cfg);
        let p = payload(&fx.ws, "cargo test", &["cargo", "test"], BTreeMap::new());
        let err = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).err().unwrap();
        assert_eq!(err.code(), ErrorCode::PolicyViolation);
    }

    #[test]
    fn prepare_rejects_when_no_route_matches() {
        let fx = routed_fx();
        let env = env_lookup(&fx.cfg);
        let p = payload(&fx.ws, "cargo check", &["cargo", "check"], BTreeMap::new());
        let err = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).err().unwrap();
        assert_eq!(err.code(), ErrorCode::PolicyViolation);
    }

    #[test]
    fn deadline_is_grant_timeout_plus_120s() {
        assert_eq!(deadline_ms("600s").unwrap(), 720_000);
        assert_eq!(deadline_ms("5s").unwrap(), 125_000);
        assert_eq!(deadline_ms("2m").unwrap(), 240_000);
        assert!(deadline_ms("bogus").is_err());
    }

    #[test]
    fn prepare_ignores_payload_executable_and_args() {
        let fx = routed_fx();
        let env = env_lookup(&fx.cfg);
        let mut p = payload(
            &fx.ws,
            "cargo test -p x",
            &["cargo", "test", "-p", "x"],
            BTreeMap::new(),
        );
        p.executable = "forged".to_string();
        p.args = vec!["forged-arg".to_string()];
        let out = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).unwrap();
        assert_eq!(out.resolution.executable.as_deref(), Some("cargo"));
        assert_eq!(out.resolution.argv, vec!["cargo", "test", "-p", "x"]);
        assert_eq!(out.resolution.exec_args, vec!["test", "-p", "x"]);
        assert_eq!(out.resolution.args, vec!["-p", "x"]);
    }

    struct NopWrite;
    impl crate::cli::CliWrite for NopWrite {
        fn write_str(&mut self, _s: &str) {}
    }
    struct NopStdin;
    impl crate::cli::CliStdin for NopStdin {
        fn is_tty(&self) -> bool {
            false
        }
        fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
            Ok(Vec::new())
        }
    }

    fn completed_exec_result(exit_code: i64) -> CanonicalToolResult {
        let mut result = CanonicalToolResult::text("child stdout");
        result.details = Some(json!({
            "grant": "cargo-test",
            "argv": ["test"],
            "state": "completed",
            "exitCode": exit_code,
            "signal": null,
            "durationMs": 5,
            "stdout": {"text": "OUT", "truncated": false},
            "stderr": {"text": "", "truncated": false},
        }));
        result
    }

    struct FinishFx {
        fx: Fx,
        prepared: PreparedRoute,
        route_exec: Arc<RouteExecution>,
        payload: RoutePayload,
    }

    fn finish_fx() -> FinishFx {
        let fx = routed_fx();
        let env = env_lookup(&fx.cfg);
        let p = payload(&fx.ws, "cargo test", &["cargo", "test"], BTreeMap::new());
        let prepared = prepare(&p, &fx.ws, &env, Some(&fx.builtin)).unwrap();
        let route_exec = Arc::new(RouteExecution::new(
            RouteLogStore::new(&fx.cfg),
            Map::new(),
            fx.ws.clone(),
            &prepared.route.grant,
            &prepared.resolution.exec_args,
            cheaper_core::token::count::DEFAULT_COUNT_MAX_BYTES,
            65_536,
        ));
        FinishFx {
            fx,
            prepared,
            route_exec,
            payload: p,
        }
    }

    fn run_finish(
        ffx: &FinishFx,
        outcome: Result<cheaper_workflow::WorkflowOutcome, CheaperError>,
    ) -> Result<RouteRunOutput, BoxError> {
        let mut stdin = NopStdin;
        let mut out = NopWrite;
        let mut err = NopWrite;
        let env = env_lookup(&ffx.fx.cfg);
        let ucd = ffx.fx.cfg.to_string_lossy().into_owned();
        let mut io = CliIo {
            stdin: &mut stdin,
            stdout: &mut out,
            stderr: &mut err,
            env: &env,
            cwd: ffx.fx.ws.clone(),
        };
        let hooks = CliHooks::default();
        finish(
            outcome,
            FinishArgs {
                prepared: &ffx.prepared,
                route_exec: &ffx.route_exec,
                payload: &ffx.payload,
                user_config_dir: &ucd,
                hooks: &hooks,
                quiet: false,
                started_at: 0.0,
            },
            &mut io,
        )
    }

    #[test]
    fn finish_mirrors_captured_exit_and_records_workflow_error() {
        let ffx = finish_fx();
        let mut result = completed_exec_result(7);
        ffx.route_exec.complete(&mut result);
        let out = run_finish(
            &ffx,
            Err(CheaperError::new(ErrorCode::Timeout, "engine error")),
        )
        .unwrap();
        assert_eq!(out.exit_code, 7);
        let failures = out.envelope["result"]["postprocessFailures"]
            .as_array()
            .unwrap();
        assert!(
            failures.iter().any(|s| s == "workflow-error"),
            "failures: {failures:?}"
        );
        let log = std::fs::read_to_string(ffx.fx.cfg.join("usage.jsonl")).unwrap();
        let record: Value = log
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .find(|r| r["kind"] == "route")
            .unwrap();
        assert_eq!(record["exitCode"], 7);
        assert_eq!(record["ok"], false);
    }

    #[test]
    fn finish_without_capture_propagates_engine_error() {
        let ffx = finish_fx();
        let err = run_finish(
            &ffx,
            Err(CheaperError::new(ErrorCode::Timeout, "engine error")),
        )
        .err()
        .unwrap();
        assert_eq!(
            err.downcast_ref::<CheaperError>().map(CheaperError::code),
            Some(ErrorCode::Timeout)
        );
    }

    #[test]
    fn finish_without_capture_returns_outcome_error_code() {
        let ffx = finish_fx();
        let outcome = cheaper_workflow::WorkflowOutcome {
            ok: false,
            result: Map::new(),
            jobs: vec![],
            meta: cheaper_workflow::WorkflowOutcomeMeta {
                duration_ms: 0.0,
                execution: None,
                output_errors: vec![],
            },
            error: Some(cheaper_workflow::WorkflowOutcomeError {
                code: ErrorCode::PolicyViolation,
                message: "blocked".to_string(),
                exit_code: 2,
                retryable: false,
            }),
        };
        let err = run_finish(&ffx, Ok(outcome)).err().unwrap();
        let cheaper = err.downcast_ref::<CheaperError>().unwrap();
        assert_eq!(cheaper.code(), ErrorCode::PolicyViolation);
        assert_eq!(cheaper.message(), "routed command did not complete");
    }

    #[test]
    fn finish_invalid_captured_exit_code_is_infra_error() {
        let ffx = finish_fx();
        let mut result = CanonicalToolResult::text("x");
        result.details = Some(json!({
            "grant": "cargo-test",
            "argv": [],
            "state": "failed",
            "exitCode": null,
            "signal": null,
            "durationMs": 5,
            "stdout": {"text": "", "truncated": false},
            "stderr": {"text": "", "truncated": false},
        }));
        ffx.route_exec.complete(&mut result);
        let outcome = cheaper_workflow::WorkflowOutcome {
            ok: true,
            result: Map::new(),
            jobs: vec![],
            meta: cheaper_workflow::WorkflowOutcomeMeta {
                duration_ms: 0.0,
                execution: None,
                output_errors: vec![],
            },
            error: None,
        };
        let err = run_finish(&ffx, Ok(outcome)).err().unwrap();
        assert_eq!(
            err.downcast_ref::<CheaperError>().map(CheaperError::code),
            Some(ErrorCode::InternalError)
        );
    }

    #[cfg(feature = "dev-seam")]
    async fn run_worker_read(max_read_bytes: u64) -> (bool, usize, f64, Value) {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("ws");
        let cfg = tmp.path().join("cfg");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::create_dir_all(&cfg).unwrap();
        let builtin = crate::embedded::builtin_dir().unwrap();
        let store = RouteLogStore::new(&cfg);
        let seed = "ROUTE-WORKER-READ-MARK\n".repeat(64);
        let mut writer = store.begin().unwrap();
        let seeded_id = writer.run_id().to_string();
        std::io::Write::write_all(&mut writer, seed.as_bytes()).unwrap();
        writer.finish().unwrap();
        let fixture_path = tmp.path().join("fake.json");
        let fx = serde_json::to_string(&json!({
            "name": "worker-read",
            "sessions": [{
                "steps": [
                    {"turn": {"tool_calls": true}},
                    {"tool_calls": [{
                        "name": "read",
                        "input": {
                            "source": {
                                "kind": "artifact",
                                "reference": {
                                    "artifact": seeded_id,
                                    "runId": seeded_id,
                                    "producer": {"jobId": "exec", "name": "command"},
                                    "bytes": 0,
                                    "preview": "",
                                    "truncated": false,
                                },
                            },
                            "offset": 0,
                            "maxBytes": seed.len(),
                        },
                    }]},
                    {"usage_delta": {"input": 120, "output": 30}},
                    {"finish": {"text": "{\"summary\":\"WORKER-READ-FINISHED\"}"}},
                ],
            }],
        }))
        .unwrap();
        std::fs::write(&fixture_path, fx).unwrap();
        let ws_str = ws.to_string_lossy().into_owned();
        let cfg_str = cfg.to_string_lossy().into_owned();
        let fx_str = fixture_path.to_string_lossy().into_owned();
        let cfg_for_env = cfg_str.clone();
        let env = move |k: &str| match k {
            "NODE_ENV" => Some("test".to_string()),
            "CHEAPER_TEST_FAKE_STREAM" => Some(fx_str.clone()),
            "CHEAPER_CONFIG_DIR" => Some(cfg_for_env.clone()),
            _ => None,
        };
        let mut stdin = NopStdin;
        let mut out = NopWrite;
        let mut err = NopWrite;
        let io = CliIo {
            stdin: &mut stdin,
            stdout: &mut out,
            stderr: &mut err,
            env: &env,
            cwd: ws_str.clone(),
        };
        let hooks = CliHooks {
            builtin_dir: Some(builtin.clone()),
            now: None,
            signal: None,
        };
        let mut policy = cheaper_core::policy::load::load_policy(
            &cheaper_core::policy::load::LoadPolicyOptions {
                builtin_policy_path: &builtin.join("policy.yaml").to_string_lossy(),
                user_config_dir: &cfg_str,
                project_dir: &ws_str,
                invocation: None,
            },
        )
        .unwrap();
        policy.workflow.artifacts.max_read_bytes = max_read_bytes;
        let dirs = cheaper_core::config::dirs::ConfigDirs {
            user_config_dir: cfg.clone(),
            project_dir: ws.join(".cheaper"),
            builtin_dir: builtin.clone(),
        };
        let ctx = crate::workflow_context::WorkflowContext::from_policy(
            ws_str.clone(),
            dirs,
            policy.clone(),
        );
        let file_root = tmp.path().join("flows");
        std::fs::create_dir_all(&file_root).unwrap();
        std::fs::write(
            file_root.join("w.json"),
            serde_json::to_string(&json!({
                "schemaVersion": 1,
                "inputs": {
                    "logRef": {"type": "string"},
                    "originalTokens": {"type": "integer"},
                    "command": {"type": "array", "items": {"type": "string"}}
                },
                "jobs": {
                    "analyze": {
                        "kind": "worker",
                        "worker": "condense",
                        "input": {
                            "logRef": {"$input": "logRef"},
                            "originalTokens": {"$input": "originalTokens"},
                            "command": {"$input": "command"}
                        }
                    }
                },
                "outputs": {
                    "result": {
                        "schema": {"type": ["string", "null"]},
                        "value": {"$from": "analyze", "path": "/summary"}
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let loaded = ctx
            .load_workflow_from("w.json", &file_root.to_string_lossy())
            .await
            .unwrap();
        let seed_tokens = cheaper_core::token::count::count_text_tokens(&seed).unwrap();
        let inputs = Map::from_iter([
            ("logRef".to_string(), json!(seeded_id)),
            ("originalTokens".to_string(), json!(seed_tokens as i64)),
            ("command".to_string(), json!(["routed"])),
        ]);
        let workflow = loaded
            .compiler
            .compile(loaded.definition.raw(), Some(&Value::Object(inputs)))
            .await
            .unwrap();
        let route_exec = Arc::new(RouteExecution::new(
            RouteLogStore::new(&cfg),
            Map::new(),
            ws_str,
            "unused",
            &[],
            cheaper_core::token::count::DEFAULT_COUNT_MAX_BYTES,
            max_read_bytes,
        ));
        let (_shared, outcome) =
            crate::workflow_run::execute_compiled(crate::workflow_run::CompiledRun {
                ctx: &ctx,
                workflow: &workflow,
                effective_policy: &policy,
                invocation_grants: Vec::new(),
                route_exec: Some(route_exec),
                io: &io,
                hooks: &hooks,
            })
            .await
            .unwrap();
        let outcome = outcome.unwrap();
        let analyze = outcome
            .jobs
            .iter()
            .find(|j| j.id == "analyze")
            .expect("analyze job outcome");
        let source_tokens = analyze
            .meta
            .extra
            .get("savings")
            .and_then(|s| s.get("sourceTokens"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let tool_calls = analyze
            .meta
            .extra
            .get("toolCalls")
            .and_then(Value::as_f64)
            .unwrap_or(-1.0);
        let succeeded = analyze.status == cheaper_workflow::engine::JobStatus::Succeeded;
        let result = outcome.result.get("result").cloned().unwrap_or_default();
        (
            outcome.ok && succeeded,
            source_tokens as usize,
            tool_calls,
            result,
        )
    }

    #[cfg(feature = "dev-seam")]
    #[tokio::test(flavor = "current_thread")]
    async fn worker_reads_route_log_through_inherited_reader() {
        let seed = "ROUTE-WORKER-READ-MARK\n".repeat(64);
        let expected = cheaper_core::token::count::count_text_tokens(&seed).unwrap();
        let (ok, source_tokens, tool_calls, result) = run_worker_read(65536).await;
        assert!(ok);
        assert_eq!(result, "WORKER-READ-FINISHED", "fixed fake summary finish");
        assert_eq!(
            source_tokens, expected,
            "worker must meter the exact seeded log text"
        );
        assert_eq!(tool_calls, 1.0);
    }

    #[cfg(feature = "dev-seam")]
    #[tokio::test(flavor = "current_thread")]
    async fn worker_route_log_read_denied_by_policy_cap() {
        let (ok, source_tokens, tool_calls, result) = run_worker_read(1).await;
        assert!(ok, "denied read does not fail the run — fixed fake finish");
        assert_eq!(result, "WORKER-READ-FINISHED");
        assert_eq!(
            source_tokens, 0,
            "cap below request must meter zero (read denied)"
        );
        assert_eq!(tool_calls, 1.0);
    }
}
