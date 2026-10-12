//! Devin Local provider plugin（TS `providers/devin/plugin.ts` の
//! port — SPEC-0026 / DESIGN-0031 相当）。
//!
//! `CheaperProviderPlugin` 実装として `registerProvider("devin", …)`
//! に相当する `PluginProviderRegistration` を登録する:
//!
//! - auth: Devin CLI 委譲（`devin auth login/status/logout` を
//!   isolated env の child process で実行）+ cheaper-owned
//!   credentials.toml 読み取り
//! - catalog: `devin models list --format json`（bootstrap は
//!   `FALLBACK_MODELS`）
//! - stream 面は `rig_provider::DevinCompletionModelFactory` が担う
//!   （`PluginProviderRegistration` に transport hook は無い —
//!   session factory seam 経由で `CompletionModel::from_rig` へ繋ぐ）
//!
//! Pi extension loader・slash command・event handler は持ち込まない。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_runtime::auth::{
    AuthInteraction, AuthSource, CredentialData, ResolvedAuth, StoredCredential,
};
use cheaper_runtime::catalog::schema::ModelDefinitionJson;
use cheaper_runtime::plugin::{
    CheaperProviderPlugin, PluginAuth, PluginCatalog, PluginProviderRegistration,
    ProviderPluginContext, ProviderRegistry,
};
use serde_json::{Map, Value};

use crate::devin::cli::{DevinCli, DevinCliRunner};
use crate::devin::credentials::{
    devin_data_dir, login_with_devin_cli, logout_devin_session, read_devin_credentials,
};
use crate::devin::models::{fallback_models, load_cli_catalog, models_from_catalog};
use crate::devin::rig_provider::{API_KIND, DEFAULT_HOST};

/// provider id（`CheaperProviderPlugin.id` + registration id）。
pub const PROVIDER_ID: &str = "devin";

/// TS `toOAuthCredentials` — `expires` は 365 日後（ms）。
fn to_stored_credential(api_key: String, toml_path: Option<String>) -> StoredCredential {
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
        + 365 * 24 * 60 * 60 * 1000;
    let mut extra = Map::new();
    if let Some(path) = toml_path {
        extra.insert("tomlPath".to_string(), Value::String(path));
    }
    StoredCredential::oauth(CredentialData::Oauth {
        access: api_key,
        refresh: String::new(),
        expires,
        account_id: None,
        subscription: None,
        available_model_ids: None,
        extra,
    })
}

/// `oauth.getApiKey` + `auth.baseUrl` 相当 — credentials.toml
/// （rotated token・`api_server_url` 拾い）優先、stored credential の
/// `access` に fallback。
fn resolve_auth_material(
    data_dir: &Path,
    credential: &StoredCredential,
) -> Option<(String, Option<String>)> {
    if let Some(creds) = read_devin_credentials(data_dir) {
        return Some((creds.api_key, Some(creds.api_server_url)));
    }
    match &credential.data {
        CredentialData::Oauth { access, .. } if !access.is_empty() => Some((access.clone(), None)),
        CredentialData::ApiKey { key } if !key.is_empty() => Some((key.clone(), None)),
        _ => None,
    }
}

/// provider の `PluginAuth`（Devin CLI 委譲 login + credentials.toml 解決）。
struct DevinPluginAuth {
    cli: Arc<DevinCli>,
    data_dir: PathBuf,
    /// Windows credential contract（global toml snapshot + logout 非
    /// 破壊）を有効にする platform seam。test から注入可能にして
    /// `cfg!(windows)` を credentials 層へ散らさない。
    windows: bool,
}

impl PluginAuth for DevinPluginAuth {
    fn login(
        &self,
        ctx: &ProviderPluginContext,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<StoredCredential, CheaperError> {
        // login 起動の notify は spawn site（credentials.rs）へ移動済み
        // — reuse 経路では発火しない。
        let creds = login_with_devin_cli(
            &self.cli,
            &self.data_dir,
            self.windows,
            ctx.env.env("APPDATA").as_deref(),
            interaction,
        )
        .map_err(crate::devin::cli::cli_error)?;
        Ok(to_stored_credential(
            creds.api_key,
            Some(creds.path.to_string_lossy().into_owned()),
        ))
    }

    fn to_auth(
        &self,
        _ctx: &ProviderPluginContext,
        credential: &StoredCredential,
    ) -> Result<ResolvedAuth, CheaperError> {
        let (api_key, base_url) =
            resolve_auth_material(&self.data_dir, credential).ok_or_else(|| {
                CheaperError::new(
                    ErrorCode::ProviderFailure,
                    "No Devin credentials. Run `cheaper auth login devin` (uses the local Devin CLI).",
                )
            })?;
        Ok(ResolvedAuth {
            api_key: Some(api_key),
            headers: BTreeMap::new(),
            base_url,
            source: AuthSource::OAuth,
        })
    }

    // `check` は trait default（stored credential 存在のみ）を使う。
    // TS `plugin.ts` は `oauth.check` を定義しないため Pi は
    // credential-exists を `configured: true` とみなす — subprocess
    // （`devin auth status`）は spawn しない。
    // `devin_auth_status`（CLI authoritative + credentials.toml
    // fallback）が残る経路は login-reuse
    // （`login_with_devin_cli`）のみ — TS と同じ配置。

    /// `oauth.refreshToken` 相当 — credentials.toml があれば最新 key で
    /// credential を更新、無ければ既存を維持（`None`）。
    fn refresh(
        &self,
        _ctx: &ProviderPluginContext,
        credential: &StoredCredential,
    ) -> Result<Option<StoredCredential>, CheaperError> {
        match read_devin_credentials(&self.data_dir) {
            Some(creds) => Ok(Some(to_stored_credential(
                creds.api_key,
                Some(creds.path.to_string_lossy().into_owned()),
            ))),
            None => {
                let _ = credential;
                Ok(None)
            }
        }
    }

    /// `auth logout devin` の外部 state 片付け — 非 Windows は
    /// `devin auth logout`（best-effort）、Windows contract は global
    /// session を残して cheaper snapshot のみ削除 + notify。
    fn logout(
        &self,
        _ctx: &ProviderPluginContext,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<(), CheaperError> {
        logout_devin_session(&self.cli, &self.data_dir, self.windows, interaction);
        Ok(())
    }
}

/// provider の `PluginCatalog`（`refreshModels` 相当）。
struct DevinPluginCatalog {
    cli: Arc<DevinCli>,
}

impl PluginCatalog for DevinPluginCatalog {
    fn refresh_models(
        &self,
        _ctx: &ProviderPluginContext,
        credential: Option<&StoredCredential>,
    ) -> Result<Option<Vec<ModelDefinitionJson>>, CheaperError> {
        // TS: `credential === undefined → FALLBACK_MODELS`。
        if credential.is_none() {
            return Ok(Some(fallback_models()));
        }
        let catalog = load_cli_catalog(&self.cli)?;
        Ok(Some(models_from_catalog(Some(&catalog))))
    }
}

/// `createDevinProviderPlugin` — `runner`・`windows` は test seam。
pub struct DevinProviderPlugin {
    runner: Option<Arc<dyn DevinCliRunner>>,
    windows: bool,
}

impl DevinProviderPlugin {
    pub fn new() -> Self {
        Self {
            runner: None,
            windows: cfg!(windows),
        }
    }
    /// test が注入する child-process runner seam。
    pub fn with_runner(runner: Arc<dyn DevinCliRunner>) -> Self {
        Self {
            runner: Some(runner),
            windows: cfg!(windows),
        }
    }
    /// test が注入する platform seam（Windows credential contract）。
    /// `with_runner(r).with_windows(true)` のように chain できる。
    pub fn with_windows(mut self, windows: bool) -> Self {
        self.windows = windows;
        self
    }
}

impl Default for DevinProviderPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CheaperProviderPlugin for DevinProviderPlugin {
    fn id(&self) -> &str {
        PROVIDER_ID
    }

    fn register(&self, registry: &mut ProviderRegistry, context: &ProviderPluginContext) {
        let data_dir = devin_data_dir(&context.user_config_dir);
        // child env の base は process env。scoped env（`ctx.env`）は
        // 列挙できないため `DEVIN_CLI` のみ明示的に取り込む
        // （TS `context.env.DEVIN_CLI` 相当）。
        // `std::env::vars()` は非 UTF-8 の env var で panic する
        // （Unix では任意 byte 列が合法）— `vars_os()` +
        // `to_string_lossy()` で起動時 crash を防ぐ。
        let mut env: BTreeMap<String, String> = std::env::vars_os()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect();
        if let Some(cli_override) = context.env.env("DEVIN_CLI") {
            env.insert("DEVIN_CLI".to_string(), cli_override);
        }
        let cli = Arc::new(DevinCli::new(env, &data_dir, self.runner.clone()));

        registry.register_provider(PluginProviderRegistration {
            id: PROVIDER_ID.to_string(),
            name: "Devin Local".to_string(),
            api: API_KIND.to_string(),
            base_url: DEFAULT_HOST.to_string(),
            api_key: None,
            auth_header: None,
            headers: None,
            models: fallback_models(),
            auth: Some(Arc::new(DevinPluginAuth {
                cli: Arc::clone(&cli),
                data_dir: data_dir.clone(),
                windows: self.windows,
            })),
            // Devin CLI の account/subscription-backed session を使うため
            // auth metadata 上も subscription provider として扱う。
            subscription: true,
            catalog: Some(Arc::new(DevinPluginCatalog { cli })),
        });
    }
}
