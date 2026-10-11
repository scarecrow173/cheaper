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
//! T7(hook/workflow)の worker 実行経路は [`create_runtime`]・
//! [`production_session_factory`]・[`CliCompletionModelFactory`]・
//! [`resolve_run_worker`]・[`SignalRelay`] を再利用し、同一の
//! composition root 経路で `run_worker` を呼べる。

use std::collections::BTreeMap;
use std::path::Path;
#[cfg(feature = "dev-seam")]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(feature = "dev-seam")]
use std::time::Duration;

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirs, ConfigDirsOptions};
use cheaper_core::duration::parse_duration;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::config::{load_model_config, LoadModelConfigOptions};
use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_core::policy::restrict::{restrict_policy, PolicyRestrictions};
use cheaper_core::worker::resolve::{resolve_worker, resolve_worker_file, ResolveWorkerError};
use cheaper_core::worker::validate::{validate_worker_input, WorkerDefinition};
use cheaper_core::workspace::paths::canonicalize_workspace_root;
use cheaper_provider::devin::{DevinCompletionModelFactory, DevinProviderPlugin, API_KIND};
use cheaper_rig_adapter::{
    builtin_completion_model, BuiltinAuth, BuiltinModelSpec, CompletionModel,
};
#[cfg(feature = "dev-seam")]
use cheaper_runtime::auth::AuthType;
use cheaper_runtime::auth::{OAuthFlow, ProcessEnv, ProviderEnv};
#[cfg(feature = "dev-seam")]
use cheaper_runtime::fixture::{RunFixture, ScriptedAuthInteraction};
use cheaper_runtime::plugin::CheaperProviderPlugin;
use cheaper_runtime::run::{
    run_worker, CompletionModelFactory, RigSessionFactory, RunWorkerDeps, SessionFactory,
    SessionSpec,
};
use cheaper_runtime::Runtime;
use cheaper_tools::operation::{AbortController, AbortListenerId, AbortReason, AbortSignal};
use serde_json::{json, Map, Value};
use tracing::Instrument;

use crate::args::RunCliArgs;
use crate::cli::{
    emit_catalog_warnings, now, resolve_builtin_dir, BoxError, CliHooks, CliIo, CommandOut, RunCtx,
};
use crate::envelope::success_envelope;
#[cfg(feature = "dev-seam")]
use crate::fake_stream::load_test_stream;
use crate::input::read_task_input;

// ---------------------------------------------------------------------------
// provider env(fixture overlay + process env)
// ---------------------------------------------------------------------------

/// CLI composition root の `ProviderEnv`。base は `ProcessEnv` で、
/// fake-stream fixture の `env` map を overlay として先に引く。
///
/// `io.env`(EnvLookup closure)は `'static` でないため `ProviderEnv`
/// trait object には載せられない — production の `io.env` は
/// `process.env` そのものなので `ProcessEnv` base で等価。fixture が
/// 宣言する env(`$VAR` template 解決用)は overlay で供給する。
#[cfg(feature = "dev-seam")]
struct ScopedProviderEnv {
    overlay: BTreeMap<String, String>,
}

#[cfg(feature = "dev-seam")]
impl ProviderEnv for ScopedProviderEnv {
    fn env(&self, name: &str) -> Option<String> {
        self.overlay
            .get(name)
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| ProcessEnv.env(name))
    }
    fn file_exists(&self, path: &str) -> bool {
        ProcessEnv.file_exists(path)
    }
    fn home_dir(&self) -> Option<PathBuf> {
        ProcessEnv.home_dir()
    }
}

// ---------------------------------------------------------------------------
// runtime / session 配線(T7 も再利用)
// ---------------------------------------------------------------------------

/// `createModelRuntime({env})` 相当の CLI composition root:
/// `DevinProviderPlugin` を常時 bundle(Pi extension loader は使わない
/// 既存方針)。fake-stream fixture 有効時(`dev-seam` feature)は
/// `env` overlay・`cheaper-fake` plugin・`auth_logins`(→
/// `AuthService::login(api_key)` credential 永続化)まで適用する —
/// replay harness と同じ経路。
///
/// T7 workflow の `ensureModel` でも再利用する前提の pub(crate) seam。
pub(crate) fn create_runtime(
    user_config_dir: &Path,
    #[cfg(feature = "dev-seam")] fixture: Option<&RunFixture>,
) -> Result<Runtime, CheaperError> {
    #[cfg(feature = "dev-seam")]
    let env: Arc<dyn ProviderEnv> = Arc::new(ScopedProviderEnv {
        overlay: fixture.map(|f| f.env.clone()).unwrap_or_default(),
    });
    #[cfg(not(feature = "dev-seam"))]
    let env: Arc<dyn ProviderEnv> = Arc::new(ProcessEnv);
    #[cfg(feature = "dev-seam")]
    let mut plugins: Vec<Arc<dyn CheaperProviderPlugin>> =
        vec![Arc::new(DevinProviderPlugin::new())];
    #[cfg(not(feature = "dev-seam"))]
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![Arc::new(DevinProviderPlugin::new())];
    #[cfg(feature = "dev-seam")]
    if let Some(f) = fixture {
        plugins.push(Arc::new(f.plugin()));
    }
    let runtime = Runtime::create(
        user_config_dir.to_path_buf(),
        env,
        &plugins,
        BTreeMap::<String, Arc<dyn OAuthFlow>>::new(),
    )?;
    #[cfg(feature = "dev-seam")]
    if let Some(f) = fixture {
        for (provider_id, secret) in &f.auth_logins {
            let mut interaction = ScriptedAuthInteraction::new(secret.clone());
            runtime.auth().login(
                runtime.catalog(),
                provider_id,
                AuthType::ApiKey,
                &mut interaction,
            )?;
        }
    }
    Ok(runtime)
}

/// builtin API(openai-completions/openai-responses/anthropic-messages)
/// と devin plugin API(`devin-local`)を `SessionSpec.model.api` で
/// dispatch する `CompletionModelFactory`。runtime 層は Rig/provider
/// 型を知らないため dispatch は composition root(本 crate)の責務。
///
/// devin factory の env は `ProcessEnv`(`DevinCompletionModelFactory::
/// default`)— fixture 経路では session factory 自体が fake になるため
/// ここを通らない。
pub(crate) struct CliCompletionModelFactory {
    devin: DevinCompletionModelFactory,
}

impl CliCompletionModelFactory {
    pub(crate) fn new() -> Self {
        Self {
            devin: DevinCompletionModelFactory::default(),
        }
    }
}

impl CompletionModelFactory for CliCompletionModelFactory {
    fn completion_model(&self, spec: &SessionSpec<'_>) -> Result<CompletionModel, CheaperError> {
        if spec.model.api == API_KIND {
            return self.devin.completion_model(spec);
        }
        builtin_completion_model(&BuiltinModelSpec {
            api: &spec.model.api,
            model_id: &spec.model.id,
            base_url: &spec.model.base_url,
            max_tokens: Some(spec.model.max_tokens as u64),
            auth: spec.auth.map(|a| BuiltinAuth {
                api_key: a.api_key.clone(),
                base_url: a.base_url.clone(),
                headers: a.headers.clone(),
            }),
        })
    }
}

/// production session factory(`RigSessionFactory` + cli の model
/// factory dispatch)。T7 も同一経路を使う。
pub(crate) fn production_session_factory() -> Box<dyn SessionFactory> {
    Box::new(RigSessionFactory::new(CliCompletionModelFactory::new()))
}

/// fake-stream fixture 有効時は scripted `FakeSessionFactory`、それ
/// 以外は [`production_session_factory`]。
#[cfg(feature = "dev-seam")]
fn session_factory(fixture: Option<&RunFixture>) -> Box<dyn SessionFactory> {
    match fixture {
        Some(f) => Box::new(f.session_factory()),
        None => production_session_factory(),
    }
}

// ---------------------------------------------------------------------------
// worker 解決
// ---------------------------------------------------------------------------

/// `ResolveWorkerError` を dispatch の `BoxError` へ畳む。`Cheaper`
/// variant は contract violation(そのまま envelope code)、`Read`
/// variant は `WorkerReadError`(TS の ErrnoException 伝播相当 —
/// `error_envelope` で INTERNAL_ERROR 化)。
pub(crate) fn box_resolve_error(e: ResolveWorkerError) -> BoxError {
    match e {
        ResolveWorkerError::Cheaper(e) => Box::new(e),
        ResolveWorkerError::Read(e) => Box::new(e),
    }
}

/// named worker 解決(project → user → builtin)。fixture 有効時は
/// `WORKER_NOT_FOUND` のみ fixture `worker` 節で生成した定義へ
/// fallback する(`cheaper run <fixture.worker.name>` で corpus
/// fixture をそのまま replay できる)。`WORKER_NOT_FOUND` 以外
/// (INVALID_WORKER_NAME / definition 不備 / read error)はそのまま
/// 上げる — 明確な利用者 error に fallback で蓋をしない。
pub(crate) fn resolve_named_worker(
    name: &str,
    dirs: &ConfigDirs,
    // `root` は fixture `worker` 節 fallback(`source_path` 組立)でのみ
    // 使うため fixture param と同じく seam-gate する。
    #[cfg(feature = "dev-seam")] root: &str,
    #[cfg(feature = "dev-seam")] fixture: Option<&RunFixture>,
) -> Result<WorkerDefinition, BoxError> {
    match resolve_worker(name, dirs) {
        Ok(w) => Ok(w),
        #[cfg(feature = "dev-seam")]
        Err(ResolveWorkerError::Cheaper(e))
            if fixture.is_some() && e.code() == ErrorCode::WorkerNotFound =>
        {
            let f = fixture.expect("fixture checked");
            // `source_path` は provenance/deny_paths 判定用 — 実 file は
            // 生成しない(harness は書き込むが workspace 汚染を避ける)。
            f.worker_definition(&Path::new(root).join(format!("{}.md", f.worker.name)))
                .map_err(BoxError::from)
        }
        Err(e) => Err(box_resolve_error(e)),
    }
}

fn resolve_run_worker(
    args: &RunCliArgs,
    dirs: &ConfigDirs,
    root: &str,
    #[cfg(feature = "dev-seam")] fixture: Option<&RunFixture>,
) -> Result<WorkerDefinition, BoxError> {
    if let Some(file) = &args.worker_file {
        // 明示 path は fallback なし(TS と同じくそのまま伝播)。
        return resolve_worker_file(file, root).map_err(box_resolve_error);
    }
    // `args.rs` が worker_name XOR worker_file を保証する。
    let name = args.worker_name.as_deref().unwrap_or_default();
    resolve_named_worker(
        name,
        dirs,
        #[cfg(feature = "dev-seam")]
        root,
        #[cfg(feature = "dev-seam")]
        fixture,
    )
}

// ---------------------------------------------------------------------------
// signal 中継
// ---------------------------------------------------------------------------

/// `process.on("SIGINT"/"SIGTERM", onAbort)` + `hooks.signal` listener
/// を `AbortController` へ中継する scope。first reason は
/// `AbortSignal` の OnceLock が守る。
/// Drop で OS signal task を abort し `hooks.signal` listener を外す
/// (TS `finally` の `process.off`/`removeEventListener` 相当)。
pub(crate) struct SignalRelay {
    controller: AbortController,
    os_task: tokio::task::JoinHandle<()>,
    external: Option<(AbortSignal, AbortListenerId)>,
}

impl SignalRelay {
    pub(crate) fn install(hooks: &CliHooks) -> Self {
        let controller = AbortController::new();
        let external = hooks.signal.as_ref().map(|sig| {
            let ac = controller.clone();
            // TS は `ac.abort()` で reason を捨てる(既定 AbortError) —
            // `AbortReason::aborted()` で同じ分類(signal → ABORTED)に
            // 揃える。
            let id = sig.on_abort(move |_| ac.abort(AbortReason::aborted()));
            // 登録後の aborted 確認 — abort 済み signal の listener は
            // 発火しない(TS `if (hooks.signal?.aborted) ac.abort()`)。
            if sig.is_aborted() {
                controller.abort(AbortReason::aborted());
            }
            (sig.clone(), id)
        });
        let ac = controller.clone();
        let os_task = tokio::spawn(async move {
            wait_os_signal().await;
            ac.abort(AbortReason::aborted());
        });
        Self {
            controller,
            os_task,
            external,
        }
    }

    /// 集約 abort 側(`deps.signal` / fixture `abort` 発火先)の signal。
    pub(crate) fn signal(&self) -> AbortSignal {
        self.controller.signal()
    }

    /// 中継先 controller(fixture `abort` の遅延発火に使う)。
    /// seam-only — `dev-seam` feature でのみ sole caller(fixture abort
    /// task)が存在する。
    #[cfg(feature = "dev-seam")]
    fn controller(&self) -> AbortController {
        self.controller.clone()
    }
}

impl Drop for SignalRelay {
    fn drop(&mut self) {
        self.os_task.abort();
        if let Some((sig, id)) = self.external.take() {
            sig.remove_listener(id);
        }
    }
}

#[cfg(unix)]
async fn wait_os_signal() {
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
        }
        // SIGTERM listener 構築失敗時は SIGINT のみ中継する。
        Err(_) => {
            let _ = tokio::signal::ctrl_c().await;
        }
    }
}

#[cfg(not(unix))]
async fn wait_os_signal() {
    let _ = tokio::signal::ctrl_c().await;
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
    let builtin_dir = resolve_builtin_dir(hooks)?;
    let root = canonicalize_workspace_root(args.cwd.as_deref().unwrap_or(&io.cwd))?;
    let dirs = resolve_config_dirs(&ConfigDirsOptions {
        cwd: Path::new(&root),
        builtin_dir: &builtin_dir,
        env: Some(io.env),
        homedir: None,
    });
    ctx.run_root = Some(root.clone());
    ctx.run_user_config_dir = Some(dirs.user_config_dir.to_string_lossy().into_owned());

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
        &dirs,
        &root,
        #[cfg(feature = "dev-seam")]
        fixture.as_ref(),
    )?;
    ctx.worker_name = Some(worker.name.clone());
    // `cheaper.cli.command` span の `worker.name` field(分類値のみ)。
    tracing::Span::current().record("worker.name", worker.name.as_str());

    let input_map = read_task_input(&args.input, io.stdin)?;
    let input = Value::Object(input_map);
    validate_worker_input(&worker, &input)?;

    let builtin_policy_path = builtin_dir.join("policy.yaml");
    let ucd = dirs.user_config_dir.to_string_lossy();
    let pdir = dirs.project_dir.to_string_lossy();
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path.to_string_lossy(),
        user_config_dir: &ucd,
        project_dir: &pdir,
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

    let timeout_ms = match &args.timeout {
        Some(t) => Some(parse_duration(t).map_err(|_| {
            CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("invalid --timeout duration: {t}"),
            )
            .with_details(Map::from_iter([("timeout".to_string(), json!(t))]))
        })?),
        None => None,
    };
    let effective_policy = restrict_policy(
        &policy,
        &PolicyRestrictions {
            timeout_ms,
            concurrency: None,
        },
    )?;

    let model_config = load_model_config(&LoadModelConfigOptions {
        user_config_dir: &ucd,
        project_dir: &pdir,
    })?;

    let runtime = create_runtime(
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
    let session_factory = session_factory(fixture.as_ref());
    #[cfg(not(feature = "dev-seam"))]
    let session_factory = production_session_factory();

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
        execution: None,
        operation_clock: None,
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
