//! `executeRun`(`packages/cli/src/cli.ts`)の command module(T3 本体)。
//!
//! 責務(TS `executeRun` と同じ順序):
//!   1. builtinDir 解決 → workspace root canonicalize → `resolve_config_dirs`
//!      → `RunCtx` 記録(root/userConfigDir は worker 解決**前**に置く —
//!      worker 解決 error path の recordUsage fallback が参照する)。
//!   2. worker 解決(`--worker-file` は `resolve_worker_file`、named は
//!      project → user → builtin 優先度の `resolve_worker`)→ `ctx.worker_name`。
//!   3. `read_task_input` + `validate_worker_input`。
//!   4. `load_policy` + `--timeout`(`parse_duration`)による
//!      `restrict_policy`。fixture 有効時は fixture `policy` 節の宣言
//!      field を loaded policy へ適用(`apply_to` — corpus replay 用)。
//!   5. `load_model_config` → `create_runtime`(devin plugin 同梱 + fixture
//!      overlay)→ session factory(fixture 有効時は scripted fake)。
//!   6. signal 中継(SIGINT/SIGTERM + `hooks.signal` → first-reason-wins
//!      `AbortController`)。
//!   7. `run_worker` → `success_envelope` + `model: <prov>/<id> (tier <t>)`
//!      stderr line。error は `WorkerRunError` のまま上げて
//!      `cli.rs` の catch(error envelope + usage 記録)に委ねる。
//!
//! T7(hook/workflow)の worker 実行経路は `crate::compose` の
//! `create_runtime`/`production_session_factory`/`CliCompletionModelFactory`・
//! [`resolve_run_worker`]・[`SignalRelay`] を再利用し、同一の
//! composition root 経路で `run_worker` を呼べる。

#[cfg(feature = "dev-seam")]
use std::time::Duration;

use cheaper_core::config::dirs::ConfigDirs;
use cheaper_core::duration::parse_duration;
use cheaper_core::model::config::{load_model_config, LoadModelConfigOptions};
use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_core::policy::restrict::{restrict_policy, PolicyRestrictions};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::resolve::resolve_worker_file;
use cheaper_core::worker::validate::{validate_worker_input, WorkerDefinition};
#[cfg(feature = "dev-seam")]
use cheaper_runtime::dev::fixture::RunFixture;
use cheaper_runtime::run::{run_worker, RunWorkerDeps};
#[cfg(feature = "dev-seam")]
use cheaper_tools::operation::AbortReason;
use cheaper_tools::tools::proc::ExecutionCorrelationCarrier;
use serde_json::Value;
use tracing::Instrument;

use crate::args::RunCliArgs;
use crate::cli::{emit_catalog_warnings, now, BoxError, CliHooks, CliIo, CommandOut, RunCtx};
use crate::envelope::success_envelope;
#[cfg(feature = "dev-seam")]
use crate::fake_stream::load_test_stream;
use crate::history::identity::ExecutionIdentity;
use crate::history::schema::{ExecutionEvent, ExecutionKind, ExecutionStatus, HistoryEvent};
use crate::history::store::HistoryStore;
use crate::input::read_task_input;
use crate::signal::SignalRelay;

struct StandaloneExecutionContext {
    builtin_dir: std::path::PathBuf,
    root: String,
    dirs: ConfigDirs,
    identity: ExecutionIdentity,
}

// ---------------------------------------------------------------------------
// worker 解決
// ---------------------------------------------------------------------------

fn resolve_run_worker(
    args: &RunCliArgs,
    dirs: &ConfigDirs,
    root: &str,
    #[cfg(feature = "dev-seam")] fixture: Option<&RunFixture>,
) -> Result<WorkerDefinition, BoxError> {
    if let Some(file) = &args.worker_file {
        // 明示 path は fallback なし(TS と同じくそのまま伝播)。
        return resolve_worker_file(file, root).map_err(crate::compose::box_resolve_error);
    }
    // `args.rs` が worker_name XOR worker_file を保証する。
    let name = args.worker_name.as_deref().unwrap_or_default();
    crate::compose::resolve_named_worker(
        name,
        dirs,
        #[cfg(feature = "dev-seam")]
        root,
        #[cfg(feature = "dev-seam")]
        fixture,
    )
}

// ---------------------------------------------------------------------------
// executeRun 本体
// ---------------------------------------------------------------------------

/// `execute` の entry — `cheaper.cli.command` span(design
/// §instrumentation 配置)。`worker.name` field は worker 解決後に
/// `record` で載せる(`--worker-file` 経路では args に name が無い)。
pub async fn execute(
    args: &RunCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
    ctx: &mut RunCtx,
) -> Result<CommandOut, BoxError> {
    execute_inner(args, io, hooks, started_at, ctx)
        .instrument(tracing::info_span!(
            "cheaper.cli.command",
            command = "run",
            worker.name = tracing::field::Empty,
        ))
        .await
}

async fn execute_inner(
    args: &RunCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
    ctx: &mut RunCtx,
) -> Result<CommandOut, BoxError> {
    let layout = crate::compose::resolve_workspace_layout(args.cwd.as_deref(), io, hooks)?;
    let builtin_dir = layout.builtin_dir;
    let root = layout.root;
    let dirs = layout.dirs;
    ctx.run_root = Some(root.clone());
    ctx.run_user_config_dir = Some(dirs.user_config_dir.to_string_lossy().into_owned());

    // The standalone Worker boundary starts after config-dir selection but
    // before worker/model/input/policy resolution. History writes are always
    // best-effort and do not affect the command result.
    let trace_hint = (io.env)("CHEAPER_TRACE_ID");
    let parent_hint = (io.env)("CHEAPER_PARENT_EXECUTION_ID");
    let identity = ExecutionIdentity::from_inherited(trace_hint.as_deref(), parent_hint.as_deref());
    let store = HistoryStore::new(&dirs.user_config_dir);
    let started = HistoryEvent::new(
        &identity,
        ExecutionKind::Worker,
        ExecutionEvent::Started,
        ExecutionStatus::Running,
    );
    let _ = store.write_started(&started);
    ctx.execution_correlation = Some(crate::usage_log::UsageCorrelation {
        execution_id: Some(identity.execution_id().to_string()),
        parent_execution_id: identity.parent_execution_id().map(|id| id.to_string()),
        trace_id: Some(identity.trace_id().to_string()),
    });

    let execution = StandaloneExecutionContext {
        builtin_dir,
        root,
        dirs,
        identity: identity.clone(),
    };
    let result = execute_started(args, io, hooks, started_at, ctx, &execution).await;
    let (status, error_code) = match &result {
        Ok(_) => (ExecutionStatus::Completed, None),
        Err(error) => {
            let code = history_error_code(error.as_ref());
            let status = match code {
                Some(cheaper_core::errors::ErrorCode::Timeout) => ExecutionStatus::Timeout,
                Some(cheaper_core::errors::ErrorCode::Aborted) => ExecutionStatus::Cancelled,
                _ => ExecutionStatus::Failed,
            };
            (status, code)
        }
    };
    let mut terminal = HistoryEvent::new(
        &identity,
        ExecutionKind::Worker,
        ExecutionEvent::Terminal,
        status,
    )
    .with_worker_id(ctx.worker_name.as_deref().unwrap_or_default())
    .with_duration_ms(Some((now(hooks) - started_at).max(0.0)))
    .with_error_code(error_code);
    if let Ok(out) = &result {
        if let Some(meta) = out.envelope.get("meta") {
            terminal = terminal
                .with_turns(meta.get("turns").and_then(Value::as_f64))
                .with_tool_calls(meta.get("toolCalls").and_then(Value::as_f64));
            if let Some(model) = meta.get("model") {
                if let Some(provider) = model.get("provider").and_then(Value::as_str) {
                    terminal = terminal.with_provider_id(provider);
                }
                if let Some(model_id) = model.get("id").and_then(Value::as_str) {
                    terminal = terminal.with_model_id(model_id);
                }
            }
        }
    }
    let _ = store.write_terminal(&terminal);
    result
}

async fn execute_started(
    args: &RunCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
    ctx: &mut RunCtx,
    execution: &StandaloneExecutionContext,
) -> Result<CommandOut, BoxError> {
    let StandaloneExecutionContext {
        builtin_dir,
        root,
        dirs,
        identity,
    } = execution;
    // fake-stream seam(`NODE_ENV === "test"` + `CHEAPER_TEST_FAKE_STREAM`
    // のみ有効 — `fake_stream::load_test_stream`)。worker fallback に
    // 使うため resolution 前に load する(TS は modelConfig 後に load
    // する — fixture load 失敗と worker not-found が両立する場合のみ
    // error 優先順位が違う、test-only seam 内の許容差分)。
    // `dev-seam` feature off(default build)では seam 自体が compile
    // 対象外 — env が残っていても fixture は読まれない。
    #[cfg(feature = "dev-seam")]
    let fixture = load_test_stream(io.env)?;

    let worker = resolve_run_worker(
        args,
        dirs,
        root,
        #[cfg(feature = "dev-seam")]
        fixture.as_ref(),
    )?;
    ctx.worker_name = Some(worker.name.clone());
    // `cheaper.cli.command` span の `worker.name` field(分類値のみ)。
    tracing::Span::current().record("worker.name", worker.name.as_str());

    // `--resume`(issue #111)— checkpoint を load・validate し、
    // 解決済み worker との一致を検査する(named resolution・
    // `--worker-file` ともに resolved `worker.name` と比較)。
    // 不一致・不正 file は `INVALID_INPUT`。
    let checkpoint = match &args.resume {
        Some(path) => {
            let cp = cheaper_runtime::run::checkpoint::load_checkpoint(std::path::Path::new(path))?;
            let cp_worker = cp.get("worker").and_then(Value::as_str).unwrap_or_default();
            if cp_worker != worker.name.as_str() {
                return Err(cheaper_core::errors::CheaperError::new(
                    cheaper_core::errors::ErrorCode::InvalidInput,
                    format!(
                        "checkpoint worker mismatch: checkpoint is for `{cp_worker}`, not `{}`",
                        worker.name
                    ),
                )
                .into());
            }
            Some(cp)
        }
        None => None,
    };

    // input 解決 — 明示 `--input`(値 `-` の stdin 指定を含む)が
    // checkpoint stored input に優先する。resume かつ未指定時は
    // checkpoint input を使い stdin を読まない(design §3c)。
    let input = if args.input_explicit || checkpoint.is_none() {
        Value::Object(read_task_input(&args.input, io.stdin)?)
    } else {
        checkpoint
            .as_ref()
            .and_then(|cp| cp.get("input").cloned())
            .expect("load_checkpoint guarantees `input` object")
    };
    validate_worker_input(&worker, &input)?;
    // runtime 側へ渡す継続状態 — 観測注入(continuation section)と
    // 連鎖 resume の観測累積に使われる。
    let resume = checkpoint
        .as_ref()
        .map(cheaper_runtime::run::checkpoint::ResumeCheckpoint::from_checkpoint);

    // `--grant` 値列 → 合成 invocation layer + 宣言順 names
    // (`grant.rs` — workflow run 経路と共有する builder)。
    let (invocation_layer, grant_names) = crate::grant::build_invocation_layer(&args.grants)?;

    let builtin_policy_path = builtin_dir.join("policy.yaml");
    let ucd = dirs.user_config_dir.to_string_lossy();
    let pdir = dirs.project_dir.to_string_lossy();
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path.to_string_lossy(),
        user_config_dir: &ucd,
        project_dir: &pdir,
        invocation: invocation_layer.as_ref(),
    })?;
    #[cfg(feature = "dev-seam")]
    let policy = {
        let mut policy = policy;
        if let Some(f) = &fixture {
            // fixture `policy` 節は宣言 field のみ loaded policy へ適用
            // (`apply_to`)— corpus fixture の maxTurns/timeout 等を実
            // policy layer の上へ重ねる。
            f.policy.apply_to(&mut policy);
        }
        policy
    };

    // `grant: <names>` — invocation-sourced grant を policy-sourced run
    // と区別する audit surface(`model:` line・catalog warning と同じ
    // stderr diagnostics 規約、`--quiet` で抑制)。
    if !grant_names.is_empty() && !args.quiet {
        io.stderr
            .write_str(&format!("grant: {}\n", grant_names.join(", ")));
        // worker が `command` tool を宣言しない場合 grant は露出しない
        // — warn-and-continue(`tools.allow` 除外・executable pin 失敗は
        // 下流の hard POLICY_VIOLATION のまま、warning へ変換しない)。
        if !worker.tools.contains(&ToolName::Command) {
            io.stderr.write_str(
                "warning: --grant supplied but worker does not declare the command tool\n",
            );
        }
    }

    let timeout_ms = match &args.timeout {
        Some(t) => Some(parse_duration(t).map_err(|_| crate::errors::invalid_timeout(t))?),
        None => None,
    };
    let effective_policy = restrict_policy(
        &policy,
        &PolicyRestrictions {
            timeout_ms,
            concurrency: None,
        },
    )?;
    // usage.jsonl `kind:"run"` record の `execution` echo 供給源 —
    // `--timeout` narrow・policy 層 merge を通った**解決後**の値を載せる
    // (declared 値ではない — spec Req 2)。usage-bearing error は全て
    // `restrict_policy` 以降の `run_worker` 内で発生するため、ここで
    // 設定すれば record 対象 run で echo が `None` になる経路は
    // 構造的に存在しない(`Option` の fail-soft 規約は維持)。
    ctx.execution_echo = Some((&effective_policy.execution).into());

    let model_config = load_model_config(&LoadModelConfigOptions {
        user_config_dir: &ucd,
        project_dir: &pdir,
    })?;

    let runtime = crate::compose::create_runtime(
        &dirs.user_config_dir,
        #[cfg(feature = "dev-seam")]
        fixture.as_ref(),
    )?;
    // spec Req 4 — catalog error を `run_worker` より先に `warning:`
    // 行として stderr へ出す(後続の `?` で warning が欠落しない)。
    emit_catalog_warnings(
        &runtime.catalog().errors(),
        &mut |s: &str| io.stderr.write_str(s),
        args.quiet,
    );
    #[cfg(feature = "dev-seam")]
    let session_factory = crate::compose::session_factory(fixture.as_ref());
    #[cfg(not(feature = "dev-seam"))]
    let session_factory = crate::compose::production_session_factory();

    // `--model` は常に優先。fake-stream 有効かつ未指定なら fixture の
    // `cheaper-fake/<model>` explicit ref を使う(TS の
    // `explicitModel: args.model ?? fixture.explicitModel()` 相当)。
    let explicit_model: Option<String> = match &args.model {
        Some(m) => Some(m.clone()),
        #[cfg(feature = "dev-seam")]
        None => fixture.as_ref().map(|f| f.explicit_model()),
        #[cfg(not(feature = "dev-seam"))]
        None => None,
    };

    // timeout タイマーは runtime 側の policy.execution.timeoutMs が
    // 担う — CLI は SIGINT/SIGTERM(+hooks.signal)を AbortController で
    // 中継するだけ(TS cli.ts 同様)。
    let relay = SignalRelay::install(hooks);
    let signal = relay.signal();

    // fixture `abort` 節 — replay harness と同じく `after_ms` 経過後に
    // `AbortReason::new("fixture", message, None)` を発火する。
    #[cfg(feature = "dev-seam")]
    let fixture_abort_task = fixture.as_ref().and_then(|f| f.abort.as_ref()).map(|a| {
        let ac = relay.controller();
        let after_ms = a.after_ms;
        let message = a
            .message
            .clone()
            .unwrap_or_else(|| "user abort".to_string());
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(after_ms)).await;
            ac.abort(AbortReason::new("fixture", message, None));
        })
    });

    // harness default(80ms)に揃える — production(feature off または
    // fixture 無し)は `None`(DEFAULT_ABORT_GRACE_MS = 5000)。
    #[cfg(feature = "dev-seam")]
    let abort_grace_ms = fixture.as_ref().map(|f| f.abort_grace_ms.unwrap_or(80.0));
    #[cfg(not(feature = "dev-seam"))]
    let abort_grace_ms: Option<f64> = None;

    let deps = RunWorkerDeps {
        worker: &worker,
        input: &input,
        root: root.clone(),
        policy: &effective_policy,
        model_config: &model_config,
        runtime: &runtime,
        session_factory: session_factory.as_ref(),
        explicit_model: explicit_model.as_deref(),
        signal: Some(&signal),
        abort_grace_ms,
        execution_correlation: Some(ExecutionCorrelationCarrier {
            trace_id: Some(identity.trace_id().to_string()),
            parent_execution_id: Some(identity.execution_id().to_string()),
        }),
        execution: None,
        operation_clock: None,
        invocation_grants: &grant_names,
        resume: resume.as_ref(),
    };

    let outcome = run_worker(&deps).await;
    // TS `finally` — listener/task 解放を envelope 組立より先に行う。
    drop(relay);
    #[cfg(feature = "dev-seam")]
    if let Some(t) = &fixture_abort_task {
        t.abort();
    }

    let ok = outcome.map_err(BoxError::from)?;
    let duration_ms = now(hooks) - started_at;
    Ok(CommandOut {
        envelope: success_envelope(&worker.name, ok.result, &ok.meta, duration_ms),
        exit_code: 0,
        model_line: Some(format!(
            "model: {}/{} (tier {})",
            ok.meta.model.provider, ok.meta.model.id, ok.meta.model.tier
        )),
    })
}

fn history_error_code(
    error: &(dyn std::error::Error + 'static),
) -> Option<cheaper_core::errors::ErrorCode> {
    if let Some(error) = error.downcast_ref::<cheaper_core::errors::CheaperError>() {
        return Some(error.code());
    }
    error.source().and_then(history_error_code)
}
