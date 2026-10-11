//! `cheaper-runtime` — `packages/runtime-pi` の Rust port。
//!
//! T2 scope（Phase 3 slice T2）:
//!
//! - `catalog` — model catalog store（bundled snapshot + plugin + user
//!   `models.json` merge、`get_available`/`get_model`、未知 api 種別の
//!   `INVALID_MODEL_CONFIG`）。
//! - `auth` — credential store（`<userConfigDir>/credentials.json`、atomic
//!   write、pi state 不読）、config/env api-key 解決、`status`/`login`/
//!   `logout` service API。
//! - `plugin` — `CheaperProviderPlugin` contract（devin/fake provider の
//!   登録 seam。runtime が contract を所有し、Rig type は含まない）。
//! - `runtime` — catalog + auth + plugin registry を束ねる composition
//!   point。
//!
//! T3 scope（Phase 3 slice T3）:
//!
//! - `run` — `runWorker` orchestration（capability check → 集約 abort →
//!   tool scope → model 解決 → session → extract/repair → meta）。
//!   run pipeline は `run::{abort,error,grace,meta,output,
//!   panic_boundary,repair}` submodule に分割される:
//!   - `abort` — run 集約 abort（first-reason-wins 分類）。
//!   - `grace` — `promptWithAbortGrace`（abort grace 既定 5000ms）。
//!   - `output` — structured output 抽出（fence 除去・JSON parse・schema）。
//!   - `repair` — bounded 修復 loop（shared usage 畳込）。
//!   - `meta` — `WorkerRunMeta` 構築（accumulator 正源・cheaper pricing）。
//!   - `error` — `WorkerRunError`（code + runMeta）。
//!   - `panic_boundary` — panic/`JoinError` → `INTERNAL_ERROR`。
//! - `dev` — dev/test seam(`fake_provider` scripted provider +
//!   `fixture` DSL)。`dev-seam` cargo feature でのみ compile 対象
//!   (ADR-034DyGyITd0YaATg8fkld7)。

pub mod auth;
pub mod catalog;
pub mod plugin;
pub mod runtime;

// dev/test seam — default build(consumer install)からは除外され、
// `dev-seam` feature 有効時のみ compile される。
#[cfg(feature = "dev-seam")]
pub mod dev;

// T3（run orchestration）。run pipeline は `run/` 配下の submodule に
// 再配置済み。旧 flat module path（`cheaper_runtime::{abort,grace,meta,
// output,panic_boundary,repair,run_error}`）は下の alias re-export で維持する。
pub mod run;

// module path 互換: `cheaper_runtime::{fixture,fake_provider}::*` の
// 現行 path を `pub use` で維持する(consumer 書き換え最小化)。
#[cfg(feature = "dev-seam")]
pub use dev::{fake_provider, fixture};

pub use run::abort;
pub use run::error as run_error;
pub use run::grace;
pub use run::meta;
pub use run::output;
pub use run::panic_boundary;
pub use run::repair;

pub use auth::{
    builtin_oauth_flows, AuthEvent, AuthInfoLink, AuthInteraction, AuthPrompt, AuthSelectOption,
    AuthService, AuthStatus, AuthType, CredentialData, CredentialStatus, CredentialStore,
    DevicePoll, FlowSleeper, GitHubCopilotFlow, HttpClient, HttpMethod, HttpRequest, HttpResponse,
    LoginResult, LogoutResult, OAuthFlow, OpenAiCodexFlow, ProviderAuthInfo, ProviderEnv,
    ResolvedAuth, StoredCredential, ThreadSleeper, UreqHttpClient,
};
pub use catalog::{
    bundled_models, load_models_json, CatalogModel, CatalogStore, Model, ModelsJsonFile,
    ProviderEntry, ProviderOrigin, SUPPORTED_APIS,
};
pub use plugin::{
    CheaperProviderPlugin, PluginAuth, PluginCatalog, PluginProviderRegistration,
    ProviderPluginContext, ProviderRegistry,
};
pub use runtime::Runtime;

#[cfg(feature = "dev-seam")]
pub use dev::fake_provider::{
    CheaperFakePlugin, FakeResponse, FakeSessionFactory, RecordedSpec, ScriptStep, ScriptedSession,
    ToolCallSpec,
};
#[cfg(feature = "dev-seam")]
pub use dev::fixture::{
    default_policy, ExpectedStatus, FixtureExpect, FixtureMetaExpect, FixtureStep, RunFixture,
    ScriptedAuthInteraction,
};
pub use run::abort::{RunAbort, RunAbortReason};
pub use run::error::{PartialRunMeta, WorkerRunError};
pub use run::grace::{prompt_with_abort_grace, PromptOutcome};
pub use run::meta::{
    build_meta, merge_snapshots, MetaInputs, ModelMeta, WorkerRunMeta, WorkerSavings, WorkerUsage,
    WorkloadMeta,
};
pub use run::output::{extract_output, strip_code_fence, ExtractFailureReason, ExtractedOutput};
pub use run::panic_boundary::{catch_run_panic, join_error_to_internal, run_panicked_to_internal};
pub use run::repair::{repair_output, RepairOptions, RepairResult};
pub use run::{
    run_worker, CompletionModelFactory, ExecutionBinding, RigSessionFactory, RunWorkerDeps,
    RunWorkerOk, SessionFactory, SessionSpec, WorkerSession, DEFAULT_ABORT_GRACE_MS,
};
