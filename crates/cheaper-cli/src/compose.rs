//! Composition root — `Runtime::create` + bundled plugin 登録・
//! session factory 配線・named worker 解決・workspace preamble の
//! 単一入口。command 実装は `run.rs`/`auth.rs`/`usage_report.rs` 等に
//! 残し、runtime 構築・共通解決の配線はここに集約する。
//!
//! dev-seam(fake-stream fixture)の plumbing は `fake_stream.rs` 境界に
//! 集約し、ここでは呼び出しのみを行う。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirs, ConfigDirsOptions};
use cheaper_core::errors::CheaperError;
#[cfg(feature = "dev-seam")]
use cheaper_core::errors::ErrorCode;
use cheaper_core::worker::resolve::{resolve_worker, ResolveWorkerError};
use cheaper_core::worker::validate::WorkerDefinition;
use cheaper_core::workspace::paths::canonicalize_workspace_root;
use cheaper_provider::devin::{DevinCompletionModelFactory, DevinProviderPlugin, API_KIND};
use cheaper_rig_adapter::{
    builtin_completion_model, BuiltinAuth, BuiltinModelSpec, CompletionModel,
};
#[cfg(feature = "dev-seam")]
use cheaper_runtime::dev::fixture::RunFixture;
use cheaper_runtime::plugin::CheaperProviderPlugin;
use cheaper_runtime::run::{
    CompletionModelFactory, RigSessionFactory, SessionFactory, SessionSpec,
};
use cheaper_runtime::Runtime;

use crate::cli::{resolve_builtin_dir, BoxError, CliHooks, CliIo};

/// `bundledProviderPlugins()` 相当 — `DevinProviderPlugin` を常時
/// bundle する(Pi extension loader は使わない)。
pub(crate) fn bundled_provider_plugins() -> Vec<Arc<dyn CheaperProviderPlugin>> {
    vec![Arc::new(DevinProviderPlugin::new())]
}

/// `createModelRuntime({env})` 相当の CLI composition root 単一入口。
/// builtin oauth flow seed は `Runtime::create` 内部で行われる
/// (TS `createModelRuntime` 相当)。
///
/// fake-stream fixture 有効時(`dev-seam` feature)は
/// env overlay・`cheaper-fake` plugin・`auth_logins`(→
/// `AuthService::login(api_key)` credential 永続化)まで適用する —
/// replay harness と同じ経路。plumbing 実体は `fake_stream` に集約。
pub(crate) fn create_runtime(
    user_config_dir: &Path,
    #[cfg(feature = "dev-seam")] fixture: Option<&RunFixture>,
) -> Result<Runtime, CheaperError> {
    #[cfg(feature = "dev-seam")]
    let env = crate::fake_stream::fixture_env(fixture);
    #[cfg(not(feature = "dev-seam"))]
    let env: Arc<dyn cheaper_runtime::auth::ProviderEnv> =
        Arc::new(cheaper_runtime::auth::ProcessEnv);
    #[allow(unused_mut)]
    let mut plugins = bundled_provider_plugins();
    #[cfg(feature = "dev-seam")]
    if let Some(f) = fixture {
        plugins.push(crate::fake_stream::fixture_plugin(f));
    }
    let runtime = Runtime::create(
        user_config_dir.to_path_buf(),
        env,
        &plugins,
        BTreeMap::<String, Arc<dyn cheaper_runtime::auth::OAuthFlow>>::new(),
    )?;
    #[cfg(feature = "dev-seam")]
    if let Some(f) = fixture {
        crate::fake_stream::apply_fixture_auth_logins(&runtime, f)?;
    }
    Ok(runtime)
}

// ---------------------------------------------------------------------------
// session factory 配線
// ---------------------------------------------------------------------------

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
/// factory dispatch)。`cheaper run`/`workflow` 両経路で同一。
pub(crate) fn production_session_factory() -> Box<dyn SessionFactory> {
    Box::new(RigSessionFactory::new(CliCompletionModelFactory::new()))
}

/// fake-stream fixture 有効時は scripted `FakeSessionFactory`、それ
/// 以外は [`production_session_factory`]。
#[cfg(feature = "dev-seam")]
pub(crate) fn session_factory(fixture: Option<&RunFixture>) -> Box<dyn SessionFactory> {
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

// ---------------------------------------------------------------------------
// workspace preamble
// ---------------------------------------------------------------------------

/// `builtinDir` 解決 → workspace root canonicalize →
/// `resolve_config_dirs` の共通 preamble 結果。
pub(crate) struct WorkspaceLayout {
    pub builtin_dir: PathBuf,
    /// canonicalized workspace root(`--cwd` or process cwd)。
    pub root: String,
    /// `resolve_config_dirs` 結果(user/project config dir)。
    pub dirs: ConfigDirs,
}

/// command 冒頭の workspace preamble 共有 helper。`run`/`models`/
/// `workflow` の 3 経路で同一 sequence を複製していたものを集約。
///
/// `usage`(read-only 集計)は canonicalize しない経路のため対象外 —
/// io.cwd をそのまま `resolve_config_dirs` へ渡す既存挙動を維持する。
pub(crate) fn resolve_workspace_layout(
    cwd: Option<&str>,
    io: &CliIo<'_>,
    hooks: &CliHooks,
) -> Result<WorkspaceLayout, BoxError> {
    let builtin_dir = resolve_builtin_dir(hooks)?;
    let root = canonicalize_workspace_root(cwd.unwrap_or(&io.cwd))?;
    let dirs = resolve_config_dirs(&ConfigDirsOptions {
        cwd: Path::new(&root),
        builtin_dir: &builtin_dir,
        env: Some(io.env),
        homedir: None,
    });
    Ok(WorkspaceLayout {
        builtin_dir,
        root,
        dirs,
    })
}
