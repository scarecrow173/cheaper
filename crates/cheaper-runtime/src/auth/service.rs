//! Auth service（service-level API、G3/G5）。
//!
//! TS `packages/runtime-pi/src/auth.ts`（`describeProviderAuth` /
//! `loginProvider` / `logoutProvider`）+ pi-ai `Models.checkProviderAuth`
//! の port。CLI wiring は Phase 4 なので、この module は
//! interaction/registry を trait 注入で受け、結果は
//! `ProviderAuthInfo`/`LoginResult`/`LogoutResult` として返す
//! （credential 値は一切含まない — secret 非開示契約）。
//!
//! - `status` → `result.providers[]` + `result.credentials[]`
//! - `login` → `{provider, authenticated, method, subscription}`
//! - `logout` → `{provider, loggedOut: true}` / `--all`
//!
//! OAuth method の advertise は declaration ベース — `auth.ts` の
//! `provider.auth.oauth !== undefined` parity で、flow 登録の有無は
//! `status`/`describe` 射影に関与しない。`OAuthFlow` trait の登録は
//! `login --type oauth` の実行可否のみを制御する（T6 で flow を注入
//! する seam。flow 未登録の oauth 宣言 provider への oauth login は
//! `INVALID_REQUEST`）。

use std::collections::BTreeMap;
use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde::Serialize;
use serde_json::{Map, Value};

use super::api_key::{check_api_key_auth, resolve_api_key_auth, ProviderEnv, ResolvedAuth};
use super::oauth::AuthEvent;
use super::store::{AuthType, CredentialStore, StoredCredential};
use crate::catalog::persisted::{
    write_plugin_catalog, PluginCatalogFile, PLUGIN_CATALOG_FILE_VERSION,
};
use crate::catalog::{CatalogStore, ModelDefinitionJson, ProviderEntry};
use crate::plugin::{PluginAuth, ProviderPluginContext};

/// login 対話 seam（pi-ai `AuthInteraction` の必要部分）。
/// CLI 層が terminal I/O を bridge して実装する。
pub trait AuthInteraction {
    /// `interaction.prompt({type, message, placeholder?})` 相当。
    /// `Select` は option id を返す。cancel/abort は Err。
    fn prompt(&mut self, prompt: &AuthPrompt<'_>) -> Result<String, CheaperError>;
    /// `interaction.notify(message)` 相当（URL 表示等）。default no-op。
    fn notify(&mut self, _message: &str) -> Result<(), CheaperError> {
        Ok(())
    }
    /// `interaction.notify(event)` — OAuth flow が送る structured
    /// `AuthEvent`（device_code/auth_url/progress/info）。default は
    /// CLI `notify` 相当の text rendering へ畳んで `notify` へ流す —
    /// event を構造のまま扱いたい層は override する。
    fn event(&mut self, event: &AuthEvent) -> Result<(), CheaperError> {
        self.notify(&render_event(event))
    }
    /// `interaction.signal.aborted` 相当 — login flow の中断 check。
    /// default false（CLI 側の signal 配線は Phase 4。Ctrl+C による
    /// process 終了が実質の中断経路）。
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// `AuthInteraction::event` の default text rendering — CLI
/// `notify(event)` の stderr 出力に合わせる。
fn render_event(event: &AuthEvent) -> String {
    match event {
        AuthEvent::Info { message, links } => {
            let mut out = message.clone();
            for link in links {
                out.push_str(&format!(
                    "\n  {}: {}",
                    link.label.as_deref().unwrap_or("link"),
                    link.url
                ));
            }
            out
        }
        AuthEvent::AuthUrl { url, instructions } => {
            let mut out = format!("Open this URL to sign in:\n  {url}");
            if let Some(instructions) = instructions {
                out.push_str(&format!("\n{instructions}"));
            }
            out
        }
        AuthEvent::DeviceCode {
            user_code,
            verification_uri,
            expires_in_seconds,
            ..
        } => {
            let mut out = format!("Go to {verification_uri} and enter code: {user_code}");
            if let Some(seconds) = expires_in_seconds {
                out.push_str(&format!(" (expires in {seconds}s)"));
            }
            out
        }
        AuthEvent::Progress { message } => message.clone(),
    }
}

/// `AuthPrompt` — secret は echo 無し入力。TS prompt union の port
/// （`placeholder`/`manual_code`/`select` 含む）。
#[derive(Debug)]
pub enum AuthPrompt<'a> {
    Secret {
        message: &'a str,
        placeholder: Option<&'a str>,
    },
    Text {
        message: &'a str,
        placeholder: Option<&'a str>,
    },
    /// `manual_code` — auth_url を開いた後の code 貼り付け（browser +
    /// loopback flow が callback と race させる prompt）。
    ManualCode {
        message: &'a str,
        placeholder: Option<&'a str>,
    },
    /// `select` — option id を返す。
    Select {
        message: &'a str,
        options: &'a [AuthSelectOption<'a>],
    },
}

/// `AuthPrompt::Select` の option（`{id, label, description?}`）。
#[derive(Debug)]
pub struct AuthSelectOption<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub description: Option<&'a str>,
}

/// builtin provider 向け OAuth flow seam（T6 で各 provider flow が実装
/// する）。`oauth.login`/`refresh`/`toAuth` の port。
pub trait OAuthFlow: Send + Sync {
    /// 対話 login → 永続化する oauth-typed credential。
    fn login(
        &self,
        interaction: &mut dyn AuthInteraction,
        ctx: &ProviderPluginContext,
    ) -> Result<StoredCredential, CheaperError>;
    /// expiring credential の更新（5 分 window 内なら service が呼ぶ）。
    /// 更新後 credential は service が store へ persist する。
    fn refresh(
        &self,
        credential: &StoredCredential,
        ctx: &ProviderPluginContext,
    ) -> Result<StoredCredential, CheaperError>;
    /// `oauth.toAuth` — credential → request 用 auth。
    fn to_auth(&self, credential: &StoredCredential) -> Result<ResolvedAuth, CheaperError>;
}

/// `ProviderAuthInfo` — `auth status` の `result.providers[]` element。
/// credential 値は含まない（`providerId`/`name`/`methods`/`subscription`/
/// `configured` のみ）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthInfo {
    pub provider_id: String,
    pub name: String,
    /// `"api_key"` / `"oauth"`（対話 login 可能な method のみ）。
    pub methods: Vec<&'static str>,
    /// oauth method が subscription 課金に紐づくか。
    pub subscription: bool,
    /// credential が設定済みか（`checkProviderAuth` 結果）。
    pub configured: bool,
}

/// `auth status` の `result.credentials[]` element。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub provider_id: String,
    /// `"api_key"` / `"oauth"`。
    #[serde(rename = "type")]
    pub credential_type: &'static str,
}

/// `auth status` 結果（`result.providers` + `result.credentials`）。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuthStatus {
    pub providers: Vec<ProviderAuthInfo>,
    pub credentials: Vec<CredentialStatus>,
}

/// `auth login` 結果（`{provider, authenticated, method, subscription}`）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub provider: String,
    pub authenticated: bool,
    pub method: &'static str,
    pub subscription: bool,
}

/// `auth logout` 結果（`{provider, loggedOut: true}`）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResult {
    pub provider: String,
    pub logged_out: bool,
}

fn invalid_request(message: String, details: Map<String, Value>) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidRequest, message).with_details(details)
}

fn unknown_provider(provider_id: &str) -> CheaperError {
    let mut details = Map::new();
    details.insert("providerId".to_string(), Value::from(provider_id));
    invalid_request(format!("unknown provider: {provider_id}"), details)
}

/// oauth credential が refresh 対象になる残り有効期間
/// （`DEFAULT_OAUTH_MINIMUM_VALIDITY_MS` = 5 分）。
pub const OAUTH_MINIMUM_VALIDITY_MS: i64 = 5 * 60 * 1000;

/// Auth service。catalog・credential store・env accessor・oauth flow
/// registry・plugin hook を束ねる。`CatalogStore` とは分離して保持する
/// （catalog は auth 判定 predicate を受ける側）。
pub struct AuthService {
    ctx: ProviderPluginContext,
    store: CredentialStore,
    /// builtin oauth 宣言に対する登録済み flow（T6 が providerId 毎に注入）。
    oauth_flows: BTreeMap<String, Arc<dyn OAuthFlow>>,
    /// `ProviderRegistry.logout_hooks` 由来の追加 hook。
    logout_hooks: BTreeMap<String, Arc<dyn PluginAuth>>,
}

impl AuthService {
    pub fn new(
        ctx: ProviderPluginContext,
        store: CredentialStore,
        oauth_flows: BTreeMap<String, Arc<dyn OAuthFlow>>,
        logout_hooks: BTreeMap<String, Arc<dyn PluginAuth>>,
    ) -> Self {
        Self {
            ctx,
            store,
            oauth_flows,
            logout_hooks,
        }
    }

    pub fn context(&self) -> &ProviderPluginContext {
        &self.ctx
    }
    pub fn env(&self) -> &dyn ProviderEnv {
        self.ctx.env.as_ref()
    }
    pub fn credentials(&self) -> &CredentialStore {
        &self.store
    }

    fn require_provider<'a>(
        &self,
        catalog: &'a CatalogStore,
        provider_id: &str,
    ) -> Result<&'a ProviderEntry, CheaperError> {
        catalog
            .provider(provider_id)
            .ok_or_else(|| unknown_provider(provider_id))
    }

    /// `checkProviderAuth` の port — `configured` 判定 + `get_available`
    /// 候補化に使う真偽値。stored oauth → provider oauth auth/plugin auth
    /// 有りなら configured。api_key → composed check（command config は
    /// 実行しない）。
    pub fn check_provider_auth(
        &self,
        entry: &ProviderEntry,
        credential: Option<&StoredCredential>,
    ) -> Result<bool, CheaperError> {
        if matches!(credential.map(|c| c.auth_type()), Some(AuthType::Oauth)) {
            if let Some(plugin) = &entry.plugin_auth {
                return plugin.check(&self.ctx, credential);
            }
            return Ok(entry.oauth.is_some());
        }
        // oauth-only provider に api_key stored → handler 無しで
        // unconfigured（no silent fallback）。
        if entry.api_key_name.is_none() {
            return Ok(false);
        }
        Ok(check_api_key_auth(
            entry.env_auth,
            entry.configured_api_key.as_deref(),
            credential,
            self.env(),
        ))
    }

    /// catalog `get_available` へ渡す configured predicate を組み立てる
    /// （credential read を provider 毎に 1 回にまとめるため map を先に
    /// 構築する — store read は file IO のため `all()` 一括）。
    pub fn configured_providers(
        &self,
        catalog: &CatalogStore,
    ) -> Result<BTreeMap<String, bool>, CheaperError> {
        let creds = self.store.all()?;
        let mut out = BTreeMap::new();
        for entry in catalog.providers() {
            out.insert(
                entry.id.clone(),
                self.check_provider_auth(entry, find_credential(&creds, &entry.id))?,
            );
        }
        Ok(out)
    }

    /// `get_available` の model filter（github-copilot
    /// `availableModelIds` filter 相当）。
    pub fn model_filter(&self, provider_id: &str) -> Result<Option<Vec<String>>, CheaperError> {
        Ok(self
            .store
            .get(provider_id)?
            .and_then(|c| c.available_model_ids()))
    }

    /// `describeProviderAuth` — 安全 metadata のみ。
    pub fn describe_provider(
        &self,
        catalog: &CatalogStore,
        provider_id: &str,
    ) -> Result<ProviderAuthInfo, CheaperError> {
        // `cheaper.auth` span(design §instrumentation 配置)— op 単位の
        // auth 境界。field は分類値のみ(credential 値は載せない)。
        let _op =
            tracing::info_span!("cheaper.auth", op = "status", provider = provider_id,).entered();
        let entry = self.require_provider(catalog, provider_id)?;
        let credential = self.store.get(provider_id)?;
        let configured = self.check_provider_auth(entry, credential.as_ref())?;
        Ok(ProviderAuthInfo {
            provider_id: entry.id.clone(),
            name: entry.name.clone(),
            methods: self.methods_of(entry),
            subscription: self.subscription_of(entry),
            configured,
        })
    }

    /// oauth method を advertise するか — TS `auth.ts` L56-66 は
    /// `provider.auth.oauth !== undefined` の declaration のみで判定
    /// し、登録済み flow の有無は status 射影に関与しない。plugin
    /// auth は catalog 構築時に oauth 宣言へ畳み込み済みのため
    /// `entry.oauth` の有無が declaration 判定と一致する。
    /// flow 登録は `login --type oauth` の実行可否のみを制御する
    /// （宣言のみの provider の oauth login は `INVALID_REQUEST`）。
    fn oauth_advertised(&self, entry: &ProviderEntry) -> bool {
        entry.oauth.is_some()
    }

    fn methods_of(&self, entry: &ProviderEntry) -> Vec<&'static str> {
        let mut methods = Vec::new();
        // `apiKey?.login !== undefined` → api_key。composed apiKey auth は
        // login を常に持つので `api_key_name` の有無 = method 有無。
        if entry.api_key_name.is_some() {
            methods.push("api_key");
        }
        if self.oauth_advertised(entry) {
            methods.push("oauth");
        }
        methods
    }

    /// `provider.auth.oauth?.isSubscription === true` — declaration
    /// 由来のみで、flow 登録に依存しない。
    fn subscription_of(&self, entry: &ProviderEntry) -> bool {
        entry.oauth.map(|o| o.subscription).unwrap_or(false)
    }

    /// `listProviderAuth` — `result.providers[]`。
    pub fn status(&self, catalog: &CatalogStore) -> Result<AuthStatus, CheaperError> {
        let _op = tracing::info_span!("cheaper.auth", op = "status").entered();
        let creds = self.store.all()?;
        let mut providers = Vec::with_capacity(catalog.providers().len());
        for entry in catalog.providers() {
            providers.push(ProviderAuthInfo {
                provider_id: entry.id.clone(),
                name: entry.name.clone(),
                methods: self.methods_of(entry),
                subscription: self.subscription_of(entry),
                configured: self.check_provider_auth(entry, find_credential(&creds, &entry.id))?,
            });
        }
        // credentials.json の宣言順をそのまま維持する（TS auth.json の
        // object key 順 = `listCredentials` 順と同じ semantics）。
        let credentials: Vec<CredentialStatus> = creds
            .into_iter()
            .map(|(id, c)| CredentialStatus {
                provider_id: id,
                credential_type: c.auth_type().as_str(),
            })
            .collect();
        Ok(AuthStatus {
            providers,
            credentials,
        })
    }

    /// `loginProvider` — api_key は secret prompt、oauth は flow/plugin
    /// login。credential は `CredentialStore` が persist し、戻り値には
    /// 値を含めない。
    pub fn login(
        &self,
        catalog: &CatalogStore,
        provider_id: &str,
        method: AuthType,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<LoginResult, CheaperError> {
        let _op = tracing::info_span!(
            "cheaper.auth",
            op = "login",
            provider = provider_id,
            auth.type = method.as_str(),
        )
        .entered();
        let entry = self.require_provider(catalog, provider_id)?;
        match method {
            AuthType::ApiKey => {
                let Some(name) = &entry.api_key_name else {
                    let mut details = Map::new();
                    details.insert("providerId".to_string(), Value::from(provider_id));
                    details.insert("type".to_string(), Value::from("api_key"));
                    return Err(invalid_request(
                        format!("provider {provider_id} does not support api_key login"),
                        details,
                    ));
                };
                let key = interaction.prompt(&AuthPrompt::Secret {
                    message: &format!("Enter {name}"),
                    placeholder: None,
                })?;
                self.store
                    .set(provider_id, StoredCredential::api_key(key))?;
            }
            AuthType::Oauth => {
                let credential = if let Some(plugin) = &entry.plugin_auth {
                    plugin.login(&self.ctx, interaction)?
                } else if entry.oauth.is_some() {
                    let Some(flow) = self.oauth_flows.get(provider_id) else {
                        let mut details = Map::new();
                        details.insert("providerId".to_string(), Value::from(provider_id));
                        details.insert("type".to_string(), Value::from("oauth"));
                        return Err(invalid_request(
                            format!("provider {provider_id} does not support oauth login"),
                            details,
                        ));
                    };
                    flow.login(interaction, &self.ctx)?
                } else {
                    let mut details = Map::new();
                    details.insert("providerId".to_string(), Value::from(provider_id));
                    details.insert("type".to_string(), Value::from("oauth"));
                    return Err(invalid_request(
                        format!("provider {provider_id} does not support oauth login"),
                        details,
                    ));
                };
                self.store.set(provider_id, credential)?;
            }
        }
        // F13: plugin `refreshModels` — login 成功後に live catalog を
        // 更新する(devin `devin models list` 経路)。refresh/変換の失敗は
        // login を失敗させず catalog errors へ記録する(TS parity —
        // `refreshModels().catch(() => {})` で login は継続)。
        self.refresh_plugin_catalog(catalog, entry);
        Ok(LoginResult {
            provider: provider_id.to_string(),
            authenticated: true,
            method: method.as_str(),
            subscription: self.subscription_of(entry),
        })
    }

    /// login 成功後の plugin catalog refresh。`Ok(None)` は bootstrap
    /// list 維持、error は catalog errors への記録のみで login 結果を
    /// 変えない。
    fn refresh_plugin_catalog(&self, catalog: &CatalogStore, entry: &ProviderEntry) {
        let Some(plugin_catalog) = &entry.plugin_catalog else {
            return;
        };
        // credential read 失敗も login 自体の成功を変えない —
        // `refreshModels` には `None` を渡す(TS: credential 未到達でも
        // refresh は走る)。
        let credential = self.store.get(&entry.id).unwrap_or_default();
        match plugin_catalog.refresh_models(&self.ctx, credential.as_ref()) {
            Ok(Some(definitions)) => {
                match catalog.refresh_plugin_models(&entry.id, &definitions) {
                    // in-memory 反映に成功した defs のみ persist する —
                    // `extension_models` 変換に失敗する defs を snapshot 化
                    // すると次回 boot の compose が同じ defs で再び失敗して
                    // provider ごと drop し、`require_provider` が
                    // `unknown provider` を返すため login 経由の refresh
                    // による自己修復が不可能になる。
                    Ok(()) => self.persist_plugin_catalog(
                        catalog,
                        entry,
                        credential.is_some(),
                        &definitions,
                    ),
                    Err(e) => catalog.record_error(format!("Provider \"{}\": {e}", entry.id)),
                }
            }
            Ok(None) => {}
            Err(e) => {
                catalog.record_error(format!(
                    "Provider \"{}\": refreshModels failed: {e}",
                    entry.id
                ));
            }
        }
    }

    /// `refreshModels` 成功 defs の catalog snapshot 永続化
    /// （`<userConfigDir>/providers/<id>/catalog.json`、temp+rename
    /// atomic write）。guard — いずれかで write skip:
    ///
    /// - (a) `has_credential` — credential 無しで取得した defs は
    ///   persist しない（取得条件の不明な catalog を後続 process へ
    ///   配らない）。
    /// - (b) definitions 非空 — 空 list は `load_plugin_catalog` の
    ///   検証で必ず失敗する自己矛盾的 file になるため skip +
    ///   `record_error`（観測性のため）。
    /// - (c) `definitions != entry.plugin_bootstrap_defs` — bootstrap
    ///   等価 defs は snapshot 化しない。credential 有りでも
    ///   `refresh_models` が bootstrap fallback list を返す経路が
    ///   あり、それを snapshot 化すると正常な catalog を破壊し得る
    ///   （residual staleness は受容済み — `fetchedAt` で診断可能）。
    ///
    /// write 失敗は `record_error` のみ — login/refresh を失敗させない
    /// （F13 契約）。`pub(crate)` は guard を login 経路なしに inline
    /// test するため。
    pub(crate) fn persist_plugin_catalog(
        &self,
        catalog: &CatalogStore,
        entry: &ProviderEntry,
        has_credential: bool,
        definitions: &[ModelDefinitionJson],
    ) {
        if !has_credential {
            return;
        }
        if definitions.is_empty() {
            catalog.record_error(format!(
                "Provider \"{}\": refreshModels returned an empty model list; catalog snapshot was not persisted",
                entry.id
            ));
            return;
        }
        if definitions == entry.plugin_bootstrap_defs.as_slice() {
            return;
        }
        let fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let file = PluginCatalogFile {
            version: PLUGIN_CATALOG_FILE_VERSION,
            provider: entry.id.clone(),
            fetched_at,
            models: definitions.to_vec(),
        };
        // `cheaper.catalog` event(design §instrumentation 配置)—
        // persist 境界。error message は full path を含み得るため載せず
        // `provider`・`file.name`(basename)・`model.count` のみ。
        if let Err(e) = write_plugin_catalog(&self.ctx.catalog_path(&entry.id), &file) {
            tracing::warn!(
                target: "cheaper.catalog",
                op = "persist",
                provider = entry.id.as_str(),
                file.name = "catalog.json",
                "plugin catalog persist failed"
            );
            catalog.record_error(format!("Provider \"{}\": {e}", entry.id));
        } else {
            tracing::debug!(
                target: "cheaper.catalog",
                op = "persist",
                provider = entry.id.as_str(),
                model.count = file.models.len(),
                file.name = "catalog.json",
                "plugin catalog persisted"
            );
        }
    }

    /// `logoutProvider` — plugin/logout hook → credential 削除。未登録
    /// provider でも冪等に成功扱いする（TS と同じ）。
    pub fn logout(
        &self,
        catalog: &CatalogStore,
        provider_id: &str,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<LogoutResult, CheaperError> {
        let _op =
            tracing::info_span!("cheaper.auth", op = "logout", provider = provider_id).entered();
        self.require_provider(catalog, provider_id)?;
        if let Some(hook) = self.logout_hook_for(catalog, provider_id) {
            hook.logout(&self.ctx, interaction)?;
        }
        self.store.remove(provider_id)?;
        Ok(LogoutResult {
            provider: provider_id.to_string(),
            logged_out: true,
        })
    }

    /// `auth logout --all` — 全 credential 削除 + 全 logout hook 起動。
    pub fn logout_all(
        &self,
        catalog: &CatalogStore,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<Vec<LogoutResult>, CheaperError> {
        let _op = tracing::info_span!("cheaper.auth", op = "logout").entered();
        let mut results = Vec::new();
        for entry in catalog.providers() {
            if let Some(hook) = self.logout_hook_for(catalog, &entry.id) {
                hook.logout(&self.ctx, interaction)?;
            }
        }
        // hook だけ登録されている（provider 未登録の）fake 用。
        for id in self.logout_hooks.keys() {
            if catalog.provider(id).is_none() {
                if let Some(hook) = self.logout_hooks.get(id) {
                    hook.logout(&self.ctx, interaction)?;
                }
            }
        }
        for id in self.store.clear()? {
            results.push(LogoutResult {
                provider: id,
                logged_out: true,
            });
        }
        Ok(results)
    }

    fn logout_hook_for(
        &self,
        catalog: &CatalogStore,
        provider_id: &str,
    ) -> Option<Arc<dyn PluginAuth>> {
        catalog
            .provider(provider_id)
            .and_then(|e| e.plugin_auth.clone())
            .or_else(|| self.logout_hooks.get(provider_id).cloned())
    }

    /// `resolveProviderAuth` の port — request 用 auth 解決（run 経路が
    /// 使う）。stored credential → handler 無しなら None（no env fallback）。
    /// oauth stored → refresh（5 分 window）→ `to_auth`。
    /// `overrides_env` は request scoped env overlay（models.json env
    /// overlay や CLI 注入 env。`OverlayEnv` と合成して渡す）。
    pub fn get_auth(
        &self,
        entry: &ProviderEntry,
        overrides_env: &BTreeMap<String, String>,
    ) -> Result<Option<ResolvedAuth>, CheaperError> {
        // `cheaper.auth` span — request auth 解決(run 経路の auth
        // boundary)。`auth.type` は stored credential 判明後に record
        // する(credential 値そのものは載せない)。
        let span = tracing::info_span!(
            "cheaper.auth",
            op = "resolve",
            provider = entry.id.as_str(),
            auth.type = tracing::field::Empty,
        );
        let _op = span.enter();
        let credential = self.store.get(&entry.id)?;
        span.record(
            "auth.type",
            credential
                .as_ref()
                .map(|c| c.auth_type().as_str())
                .unwrap_or("none"),
        );
        match credential {
            Some(cred) if cred.auth_type() == AuthType::Oauth => {
                if let Some(plugin) = &entry.plugin_auth {
                    return plugin.to_auth(&self.ctx, &cred).map(Some).map_err(|e| {
                        CheaperError::new(
                            e.code(),
                            format!("OAuth auth derivation failed for {}: {e}", entry.id),
                        )
                    });
                }
                // handler 無しの oauth credential → env fallback しない。
                let Some(flow) = self.oauth_flows.get(&entry.id) else {
                    return Ok(None);
                };
                // 5 分 window 内なら refresh して persist。
                let cred = if let super::store::CredentialData::Oauth { expires, .. } = &cred.data {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    if now + OAUTH_MINIMUM_VALIDITY_MS >= *expires {
                        let refreshed = flow.refresh(&cred, &self.ctx).map_err(|e| {
                            CheaperError::new(
                                ErrorCode::ProviderFailure,
                                format!("OAuth refresh failed for {}: {e}", entry.id),
                            )
                        })?;
                        self.store.set(&entry.id, refreshed.clone())?;
                        refreshed
                    } else {
                        cred.clone()
                    }
                } else {
                    cred.clone()
                };
                flow.to_auth(&cred).map(Some).map_err(|e| {
                    CheaperError::new(
                        ErrorCode::ProviderFailure,
                        format!("OAuth auth derivation failed for {}: {e}", entry.id),
                    )
                })
            }
            Some(cred) => {
                if entry.api_key_name.is_none() {
                    return Ok(None);
                }
                resolve_api_key_auth(
                    &entry.id,
                    entry.env_auth,
                    entry.configured_api_key.as_deref(),
                    entry.configured_headers.as_ref(),
                    entry.auth_header,
                    Some(&cred),
                    &OverlayOrProcess {
                        overlay: overrides_env,
                        base: self.env(),
                    },
                )
            }
            None => {
                if entry.api_key_name.is_none() {
                    return Ok(None);
                }
                resolve_api_key_auth(
                    &entry.id,
                    entry.env_auth,
                    entry.configured_api_key.as_deref(),
                    entry.configured_headers.as_ref(),
                    entry.auth_header,
                    None,
                    &OverlayOrProcess {
                        overlay: overrides_env,
                        base: self.env(),
                    },
                )
            }
        }
    }
}

/// `CredentialStore::all()` の順序保持 Vec から id 検索する補助。
fn find_credential<'a>(
    creds: &'a [(String, StoredCredential)],
    provider_id: &str,
) -> Option<&'a StoredCredential> {
    creds
        .iter()
        .find(|(id, _)| id == provider_id)
        .map(|(_, c)| c)
}

/// overrides env + process env の合成 accessor（`resolveProviderAuth` の
/// `overrides.env` overlay 相当）。
struct OverlayOrProcess<'a> {
    overlay: &'a BTreeMap<String, String>,
    base: &'a dyn ProviderEnv,
}

impl ProviderEnv for OverlayOrProcess<'_> {
    fn env(&self, name: &str) -> Option<String> {
        self.overlay
            .get(name)
            .filter(|v| !v.trim().is_empty())
            .cloned()
            .or_else(|| self.base.env(name))
    }
    fn file_exists(&self, path: &str) -> bool {
        self.base.file_exists(path)
    }
    fn home_dir(&self) -> Option<std::path::PathBuf> {
        self.base.home_dir()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::catalog::persisted::load_plugin_catalog;
    use crate::plugin::PluginProviderRegistration;

    /// env accessor stub — persist 経路は env を読まない。
    struct NullEnv;
    impl ProviderEnv for NullEnv {
        fn env(&self, _name: &str) -> Option<String> {
            None
        }
        fn file_exists(&self, _path: &str) -> bool {
            false
        }
        fn home_dir(&self) -> Option<std::path::PathBuf> {
            None
        }
    }

    fn model_def(id: &str) -> ModelDefinitionJson {
        serde_json::from_str(&format!(r#"{{"id": "{id}", "name": "Model {id}"}}"#)).unwrap()
    }

    /// `ProviderEntry` は private field（`model_header_sources` 等）を
    /// 持ち literal 構築できないため、fake `PluginProviderRegistration`
    /// （`tests/plugin.rs` の `FakePlugin::register` と同形）を渡した
    /// `CatalogStore::build` で catalog を組み立て `provider(id)` で
    /// entry を取得する。`catalog` seam は persist の対象外のため
    /// `catalog: None` で足りる。
    fn fixture(dir: &std::path::Path) -> (AuthService, CatalogStore) {
        let ctx = ProviderPluginContext {
            user_config_dir: dir.to_path_buf(),
            env: Arc::new(NullEnv),
        };
        let registration = PluginProviderRegistration {
            id: "fake".to_string(),
            name: "Fake Provider".to_string(),
            api: "fake-api".to_string(),
            base_url: "http://fake.local".to_string(),
            api_key: None,
            auth_header: None,
            headers: None,
            models: vec![model_def("fake-1")],
            auth: None,
            subscription: false,
            catalog: None,
        };
        let catalog = CatalogStore::build(None, None, &[registration], &BTreeMap::new()).unwrap();
        let service = AuthService::new(
            ctx,
            CredentialStore::new(dir),
            BTreeMap::new(),
            BTreeMap::new(),
        );
        (service, catalog)
    }

    /// guard skip が「既存 file を上書きしない」ことの検証用 sentinel。
    /// bootstrap・refresh defs とは異なる model を持つ有効な envelope。
    fn sentinel_file() -> PluginCatalogFile {
        PluginCatalogFile {
            version: PLUGIN_CATALOG_FILE_VERSION,
            provider: "fake".to_string(),
            fetched_at: 1,
            models: vec![model_def("sentinel")],
        }
    }

    #[test]
    fn persist_skips_without_credential() {
        // guard (a): credential 無し → 新規 file を作らず、既存 file も
        // 上書きしない（error も記録しない silent skip）。
        let dir = tempfile::tempdir().unwrap();
        let (service, catalog) = fixture(dir.path());
        let entry = catalog.provider("fake").unwrap();
        let path = service.context().catalog_path("fake");

        service.persist_plugin_catalog(&catalog, entry, false, &[model_def("fake-2")]);
        assert!(!path.exists());
        assert!(catalog.errors().is_empty());

        write_plugin_catalog(&path, &sentinel_file()).unwrap();
        service.persist_plugin_catalog(&catalog, entry, false, &[model_def("fake-2")]);
        assert_eq!(
            load_plugin_catalog(&path, "fake").unwrap().unwrap(),
            sentinel_file()
        );
    }

    #[test]
    fn persist_skips_empty_definitions() {
        // guard (b): 空 definitions → write skip + `record_error`
        // （空 file は `load_plugin_catalog` の検証で必ず失敗する
        // 自己矛盾のため書かない。既存 file も温存する）。
        let dir = tempfile::tempdir().unwrap();
        let (service, catalog) = fixture(dir.path());
        let entry = catalog.provider("fake").unwrap();
        let path = service.context().catalog_path("fake");

        write_plugin_catalog(&path, &sentinel_file()).unwrap();
        let before = catalog.errors().len();
        service.persist_plugin_catalog(&catalog, entry, true, &[]);
        assert_eq!(
            load_plugin_catalog(&path, "fake").unwrap().unwrap(),
            sentinel_file()
        );
        let errors = catalog.errors();
        assert_eq!(errors.len(), before + 1, "{errors:?}");
        assert!(errors.last().unwrap().contains("\"fake\""), "{errors:?}");
    }

    #[test]
    fn persist_skips_bootstrap_equal_definitions() {
        // guard (c): bootstrap 等価 defs（`refresh_models` が fallback
        // list を返す経路）は snapshot 化しない — 新規 file を作らず
        // 既存 file も温存する。
        let dir = tempfile::tempdir().unwrap();
        let (service, catalog) = fixture(dir.path());
        let entry = catalog.provider("fake").unwrap();
        let path = service.context().catalog_path("fake");

        service.persist_plugin_catalog(&catalog, entry, true, &[model_def("fake-1")]);
        assert!(!path.exists());
        assert!(catalog.errors().is_empty());

        write_plugin_catalog(&path, &sentinel_file()).unwrap();
        service.persist_plugin_catalog(&catalog, entry, true, &[model_def("fake-1")]);
        assert_eq!(
            load_plugin_catalog(&path, "fake").unwrap().unwrap(),
            sentinel_file()
        );
    }

    #[test]
    fn persist_writes_valid_envelope() {
        // happy path: guard 全通過 → `catalog_path` へ version 1
        // envelope が書かれ `load_plugin_catalog` で読み戻せる。
        let dir = tempfile::tempdir().unwrap();
        let (service, catalog) = fixture(dir.path());
        let entry = catalog.provider("fake").unwrap();
        let defs = vec![model_def("fake-2"), model_def("fake-3")];

        service.persist_plugin_catalog(&catalog, entry, true, &defs);

        let path = service.context().catalog_path("fake");
        let file = load_plugin_catalog(&path, "fake").unwrap().unwrap();
        assert_eq!(file.version, PLUGIN_CATALOG_FILE_VERSION);
        assert_eq!(file.provider, "fake");
        assert!(file.fetched_at > 0);
        assert_eq!(file.models, defs);
        assert!(catalog.errors().is_empty());
    }
}
