//! Provider auth — credential store + config/env api-key 解決 + service API。
//!
//! - `store` — `<userConfigDir>/credentials.json`（atomic write、新規
//!   schema。pi の `auth.json`/`pi/` 以下は読まない）。
//! - `api_key` — config value template/command 解決 + provider api-key
//!   auth の composed resolve/check（`resolve-config-value.ts` +
//!   `composeApiKeyAuth` の port）。
//! - `service` — `auth status`/`login`/`logout` の service-level API +
//!   `AuthInteraction`/`OAuthFlow` seam。
//! - `oauth` — OAuth flow contract（`AuthEvent`・device-code poll・HTTP
//!   seam）+ github-copilot flow（T6）。

pub mod api_key;
pub mod oauth;
pub mod service;
pub mod store;

pub use api_key::{
    check_api_key_auth, config_context_env, get_config_value_env_var_names,
    get_missing_config_value_env_var_names, is_command_config_value, is_config_value_configured,
    resolve_api_key_auth, resolve_config_value, resolve_config_value_or_throw,
    resolve_headers_or_throw, AuthSource, OverlayEnv, ProcessEnv, ProviderEnv, ResolvedAuth,
};
pub use oauth::{
    builtin_oauth_flows, AuthEvent, AuthInfoLink, DeviceFlowPollOptions, DevicePoll, FlowSleeper,
    GitHubCopilotFlow, HttpClient, HttpMethod, HttpRequest, HttpResponse, OpenAiCodexFlow,
    RetryPolicy, ThreadSleeper, UreqHttpClient,
};
pub use service::{
    AuthInteraction, AuthPrompt, AuthSelectOption, AuthService, AuthStatus, CredentialStatus,
    LoginResult, LogoutResult, OAuthFlow, ProviderAuthInfo, OAUTH_MINIMUM_VALIDITY_MS,
};
pub use store::{AuthType, CredentialData, CredentialStore, StoredCredential};
