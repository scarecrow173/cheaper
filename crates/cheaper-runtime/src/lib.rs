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
//! - `abort` — run 集約 abort（first-reason-wins 分類）。
//! - `grace` — `promptWithAbortGrace`（abort grace 既定 5000ms）。
//! - `output` — structured output 抽出（fence 除去・JSON parse・schema）。
//! - `repair` — bounded 修復 loop（shared usage 畳込）。
//! - `meta` — `WorkerRunMeta` 構築（accumulator 正源・cheaper pricing）。
//! - `run_error` — `WorkerRunError`（code + runMeta）。
//! - `panic_boundary` — panic/`JoinError` → `INTERNAL_ERROR`。
//! - `fake_provider` — T3 最小 scripted provider seam。

pub mod auth;
pub mod catalog;
pub mod plugin;
pub mod runtime;

// T3（run orchestration）。
pub mod abort;
pub mod fake_provider;
pub mod fixture;
pub mod grace;
pub mod meta;
pub mod output;
pub mod panic_boundary;
pub mod repair;
pub mod run;
pub mod run_error;

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

pub use abort::{RunAbort, RunAbortReason};
pub use fake_provider::{
    CheaperFakePlugin, FakeResponse, FakeSessionFactory, RecordedSpec, ScriptStep, ScriptedSession,
    ToolCallSpec,
};
pub use fixture::{
    default_policy as fixture_default_policy, ExpectedStatus, FixtureExpect, FixtureMetaExpect,
    FixtureStep, RunFixture, ScriptedAuthInteraction,
};
pub use grace::{prompt_with_abort_grace, PromptOutcome};
pub use meta::{
    build_meta, merge_snapshots, MetaInputs, ModelMeta, WorkerRunMeta, WorkerSavings, WorkerUsage,
    WorkloadMeta,
};
pub use output::{extract_output, strip_code_fence, ExtractFailureReason, ExtractedOutput};
pub use panic_boundary::{catch_run_panic, join_error_to_internal, run_panicked_to_internal};
pub use repair::{repair_output, RepairOptions, RepairResult};
pub use run::{
    run_worker, CompletionModelFactory, ExecutionBinding, RigSessionFactory, RunWorkerDeps,
    RunWorkerOk, SessionFactory, SessionSpec, WorkerSession, DEFAULT_ABORT_GRACE_MS,
};
pub use run_error::{PartialRunMeta, WorkerRunError};
