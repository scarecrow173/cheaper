//! `Runtime` — catalog store + auth service + plugin registry の
//! composition point（TS `createModelRuntime` 相当の offline 版）。
//!
//! - `<userConfigDir>/models.json` を `load_models_json` で読み catalog
//!   overlay にする（parse/schema error は catalog errors へ記録し
//!   runtime 自体は構築される — TS `ModelConfig.error` 相当）。
//! - `<userConfigDir>/credentials.json` は `CredentialStore` が担当
//!   （pi の `auth.json`/`pi/` 以下は一切読まない）。
//! - plugin は `CheaperProviderPlugin` を composition root が渡し、
//!   `ProviderRegistry` 経由で provider/logout hook を登録する。
//! - builtin oauth flow（T6: github-copilot）は `builtin_oauth_flows()`
//!   で自動登録する。`oauth_flows` 引数は同名 id の override（test 注入）
//!   と追加 provider flow 用。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use cheaper_core::errors::CheaperError;

use crate::auth::service::{AuthService, OAuthFlow};
use crate::auth::store::CredentialStore;
use crate::auth::{ProviderEnv, ResolvedAuth};
use crate::catalog::{load_models_json, CatalogModel, CatalogStore};
use crate::plugin::{CheaperProviderPlugin, PluginAuth, ProviderPluginContext, ProviderRegistry};

/// Runtime（catalog + auth）。`get_available` 等は auth 判定を内部で
/// wiring する。
pub struct Runtime {
    user_config_dir: PathBuf,
    context: ProviderPluginContext,
    catalog: CatalogStore,
    auth: AuthService,
}

impl Runtime {
    /// runtime 構築。`plugins` は composition root が集めた登録済み
    /// plugin。oauth flow は `builtin_oauth_flows()` を seed に
    /// `oauth_flows` を重ねる（同名 id は引数側が勝ち — test 注入用。
    /// flow 未登録の oauth 宣言 provider は method を advertise しない）。
    pub fn create(
        user_config_dir: PathBuf,
        env: Arc<dyn ProviderEnv>,
        plugins: &[Arc<dyn CheaperProviderPlugin>],
        oauth_flows: BTreeMap<String, Arc<dyn OAuthFlow>>,
    ) -> Result<Runtime, CheaperError> {
        let context = ProviderPluginContext {
            user_config_dir: user_config_dir.clone(),
            env,
        };
        let mut registry = ProviderRegistry::default();
        for plugin in plugins {
            plugin.register(&mut registry, &context);
        }
        let logout_hooks: BTreeMap<String, Arc<dyn PluginAuth>> = registry.logout_hooks().clone();
        let registrations = registry.into_registrations();

        let (user_models, user_error) = match load_models_json(&user_config_dir.join("models.json"))
        {
            Ok(file) => (file, None),
            Err(e) => (None, Some(e)),
        };
        // TODO(T3): `<userConfigDir>/providers/<id>/catalog.json` の
        // snapshot load loop をここへ追加し、load 済み defs を渡す。
        let catalog =
            CatalogStore::build(user_models, user_error, &registrations, &BTreeMap::new())?;
        let store = CredentialStore::new(&user_config_dir);
        let mut flows = crate::auth::oauth::builtin_oauth_flows();
        flows.extend(oauth_flows);
        let auth = AuthService::new(context.clone(), store, flows, logout_hooks);
        Ok(Runtime {
            user_config_dir,
            context,
            catalog,
            auth,
        })
    }

    pub fn user_config_dir(&self) -> &PathBuf {
        &self.user_config_dir
    }
    pub fn context(&self) -> &ProviderPluginContext {
        &self.context
    }
    pub fn catalog(&self) -> &CatalogStore {
        &self.catalog
    }
    pub fn auth(&self) -> &AuthService {
        &self.auth
    }

    /// `models.getAvailable(providerId)` 相当 — configured provider の
    /// model のみ（github-copilot の `availableModelIds` filter 込み）。
    pub fn get_available(
        &self,
        provider_id: Option<&str>,
    ) -> Result<Vec<CatalogModel>, CheaperError> {
        let configured = self.auth.configured_providers(&self.catalog)?;
        let filters: BTreeMap<String, Vec<String>> = {
            let mut m = BTreeMap::new();
            for entry in self.catalog.providers() {
                if let Some(f) = self.auth.model_filter(&entry.id)? {
                    m.insert(entry.id.clone(), f);
                }
            }
            m
        };
        Ok(self.catalog.get_available(
            &|id| configured.get(id).copied().unwrap_or(false),
            &|id| filters.get(id).cloned(),
            provider_id,
        ))
    }

    /// `get_supported_model` + auth 解決の合成（run 経路が使う）。
    /// model 不在 → None、api 種別がスコープ外 → `INVALID_MODEL_CONFIG`。
    pub fn resolve_model_auth(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Option<(CatalogModel, Option<ResolvedAuth>)>, CheaperError> {
        let Some(model) = self.catalog.get_supported_model(provider_id, model_id)? else {
            return Ok(None);
        };
        let entry = self
            .catalog
            .provider(provider_id)
            .expect("get_model implies provider exists");
        let auth = self.auth.get_auth(entry, &BTreeMap::new())?;
        Ok(Some((model, auth)))
    }
}
