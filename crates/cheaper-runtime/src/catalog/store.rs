//! `CatalogStore` — bundled 静的 snapshot + plugin 登録 provider +
//! user `<userConfigDir>/models.json` overlay を compose した catalog。
//! `get_available`（認証済み model のみ候補化）・`get_model` を提供し、
//! 未知・スコープ外の provider api 種別は `INVALID_MODEL_CONFIG` で明示
//! error にする（silent fallback 禁止）。

use super::bundled::{bundled_models, BUILTIN_PROVIDERS};
use super::compose::{
    apply_model_override, collect_model_header_sources, compose_provider, extension_models,
    invalid_model_config, merge_user_model_defs,
};
use super::schema::{ModelDefinitionJson, ModelsJsonFile, ProviderConfigJson};
use super::types::{CatalogModel, ProviderEntry, SUPPORTED_APIS};
use crate::plugin::PluginProviderRegistration;
use cheaper_core::errors::CheaperError;
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// plugin login 後の catalog refresh で差し替わる model list と
/// `rawModelHeaders` source（refresh 時に同じ precedence で再構築した
/// もの — refreshed defs の `headers` が request へ届くよう models と
/// 一緒に差し替える）。
#[derive(Debug)]
struct RefreshedCatalog {
    models: Vec<CatalogModel>,
    model_header_sources: BTreeMap<String, Map<String, Value>>,
}

/// Model catalog store。bundled + plugin + user models.json を compose した
/// 結果を保持する。provider 順は TS `providerIds()` と同じ
/// （builtin → user config → plugin）。
#[derive(Debug)]
pub struct CatalogStore {
    providers: Vec<ProviderEntry>,
    /// `ModelConfig.error` + `compositionErrors` 相当の load/compose error
    /// 文字列（TS `runtime.getError()` 相当。`models` の exit 0 contract を
    /// 維持するため error は throw せずここに記録する）。F13 で plugin
    /// `refreshModels` の実行時 error も追記するため interior mutability
    /// 化（compose 後も追記可能）。
    errors: std::sync::Mutex<Vec<String>>,
    /// plugin login 後の catalog refresh で差し替わる model list
    /// （devin `devin models list`）。
    dynamic_models: std::sync::RwLock<BTreeMap<String, RefreshedCatalog>>,
}

impl CatalogStore {
    /// catalog を構築する。`user_models` は `load_models_json` の結果
    /// （parse/schema error は `user_error` で記録）。
    /// `persisted_plugin_models` は provider id → `<userConfigDir>/
    /// providers/<id>/catalog.json` から load 済みの defs（envelope 検証
    /// 済み・`registration.models` は差し替えず別 source として渡す —
    /// persist guard (c) の比較基準を注入前 bootstrap に固定するため）。
    pub fn build(
        user_models: Option<ModelsJsonFile>,
        user_error: Option<String>,
        plugin_registrations: &[PluginProviderRegistration],
        persisted_plugin_models: &BTreeMap<String, Vec<ModelDefinitionJson>>,
    ) -> Result<CatalogStore, CheaperError> {
        let mut errors = Vec::new();
        if let Some(e) = user_error {
            errors.push(e);
        }
        let config = user_models.unwrap_or_default();
        let config_map: BTreeMap<&str, &ProviderConfigJson> = config
            .providers
            .iter()
            .map(|(id, c)| (id.as_str(), c))
            .collect();

        // provider id 順序: builtin → config → plugin（`providerIds()`）。
        let mut provider_ids: Vec<String> = Vec::new();
        let mut push_unique = |id: &str| {
            if !provider_ids.iter().any(|p| p == id) {
                provider_ids.push(id.to_string());
            }
        };
        for meta in BUILTIN_PROVIDERS {
            push_unique(meta.id);
        }
        for (id, _) in &config.providers {
            push_unique(id);
        }
        for reg in plugin_registrations {
            push_unique(&reg.id);
        }

        let plugin_map: BTreeMap<&str, &PluginProviderRegistration> = plugin_registrations
            .iter()
            .map(|p| (p.id.as_str(), p))
            .collect();

        let mut providers = Vec::with_capacity(provider_ids.len());
        for id in &provider_ids {
            let meta = BUILTIN_PROVIDERS.iter().find(|m| m.id == id.as_str());
            let mut base_models = Vec::new();
            if let Some(m) = meta {
                match bundled_models(m.id) {
                    Ok(models) => base_models = models,
                    Err(e) => errors.push(format!("Provider \"{id}\": {e}")),
                }
            }
            // merge error は bare message で `compose_errors` に積まれ、
            // compose 結果の成否に関わらず `Provider "<id>": ` prefix で
            // errors へ集約する（fatal Err の prefix 規則と同じ）。
            let mut compose_errors: Vec<String> = Vec::new();
            let composed = compose_provider(
                id,
                meta,
                &base_models,
                config_map.get(id.as_str()).copied(),
                plugin_map.get(id.as_str()).copied(),
                persisted_plugin_models.get(id).map(Vec::as_slice),
                &mut compose_errors,
            );
            errors.extend(
                compose_errors
                    .drain(..)
                    .map(|msg| format!("Provider \"{id}\": {msg}")),
            );
            match composed {
                Ok(entry) => providers.push(entry),
                Err(msg) => {
                    // `compositionErrors` — builtin は untouched fallback、
                    // custom/plugin-only は provider を drop する。
                    errors.push(format!("Provider \"{id}\": {msg}"));
                    if let Some(meta) = meta {
                        // base fallback（overlay 無しの untouched builtin）。
                        // `plugin: None` のため merge は走らないが
                        // signature を揃え `errors` は同一 vec を共有する。
                        match compose_provider(
                            id,
                            Some(meta),
                            &base_models,
                            None,
                            None,
                            None,
                            &mut compose_errors,
                        ) {
                            Ok(entry) => providers.push(entry),
                            Err(fatal) => {
                                errors.push(format!("Provider \"{id}\": {fatal}"));
                            }
                        }
                        errors.extend(
                            compose_errors
                                .drain(..)
                                .map(|msg| format!("Provider \"{id}\": {msg}")),
                        );
                    }
                }
            }
        }

        // `cheaper.catalog` event(design §instrumentation 配置)—
        // bundled + persisted + user overlay の compose 集計。field は
        // metadata-only の `provider.count`/`error.count`(provider id
        // 一覧・error 本文は載せない)。
        tracing::debug!(
            target: "cheaper.catalog",
            op = "merge",
            provider.count = providers.len(),
            error.count = errors.len(),
            "catalog composed"
        );

        Ok(CatalogStore {
            providers,
            errors: std::sync::Mutex::new(errors),
            dynamic_models: std::sync::RwLock::new(BTreeMap::new()),
        })
    }

    /// load/compose/refresh error（TS `runtime.getError()` 相当）。
    pub fn errors(&self) -> Vec<String> {
        self.errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 実行時 error の追記 — plugin `refreshModels` 失敗等、login 自体を
    /// 失敗させず catalog error として記録する経路(F13)。
    pub fn record_error(&self, message: String) {
        self.errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(message);
    }

    /// provider 一覧（登録順）。
    pub fn providers(&self) -> &[ProviderEntry] {
        &self.providers
    }

    pub fn provider(&self, provider_id: &str) -> Option<&ProviderEntry> {
        self.providers.iter().find(|p| p.id == provider_id)
    }

    /// `getAvailable(providerId)` 相当。`configured` 判定（credential + env +
    /// configured apiKey）は caller が注入する（auth 層との循環を避けるため
    /// predicate 注入）。github-copilot の oauth credential
    /// `availableModelIds` filter だけは credential 値に依存するため
    /// `credential_models_filter` も受ける。
    ///
    /// `configured`: providerId → 認証済みか。
    /// `model_filter`: providerId → Some(allowed ids) のとき model を絞る
    /// （github-copilot `filterModels` 相当）。
    pub fn get_available(
        &self,
        configured: &dyn Fn(&str) -> bool,
        model_filter: &dyn Fn(&str) -> Option<Vec<String>>,
        provider_id: Option<&str>,
    ) -> Vec<CatalogModel> {
        let mut out = Vec::new();
        for entry in &self.providers {
            if let Some(filter) = provider_id {
                if entry.id != filter {
                    continue;
                }
            }
            if !configured(&entry.id) {
                continue;
            }
            let base_models = self.models_of(entry);
            match model_filter(&entry.id) {
                Some(allowed) => {
                    let allowed: std::collections::HashSet<&str> =
                        allowed.iter().map(String::as_str).collect();
                    out.extend(
                        base_models
                            .into_iter()
                            .filter(|m| allowed.contains(m.id.as_str())),
                    );
                }
                None => out.extend(base_models),
            }
        }
        out
    }

    /// provider の現在の model list（plugin refresh で差し替わった list
    /// があればそれを優先する）。
    fn models_of(&self, entry: &ProviderEntry) -> Vec<CatalogModel> {
        if let Ok(guard) = self.dynamic_models.read() {
            if let Some(refreshed) = guard.get(&entry.id) {
                return refreshed.models.clone();
            }
        }
        entry.models.clone()
    }

    /// model の request headers merge source — plugin refresh 済みの
    /// source を優先し、refresh 未実行なら compose 時の
    /// `entry.model_header_sources` へ fallback する。
    /// （`ProviderEntry::model_headers` は compose-time source の
    /// accessor。request header 解決は store accessor を経由させる）
    pub fn model_headers(&self, provider_id: &str, model_id: &str) -> Option<Map<String, Value>> {
        if let Ok(guard) = self.dynamic_models.read() {
            if let Some(refreshed) = guard.get(provider_id) {
                return refreshed.model_header_sources.get(model_id).cloned();
            }
        }
        self.provider(provider_id)
            .and_then(|entry| entry.model_headers(model_id))
    }

    /// plugin login 後の catalog refresh（devin `devin models list`）。
    /// `modelOverrides` は差し替え後の model にも適用する。model list と
    /// header source は `RefreshedCatalog` として一緒に差し替える。
    fn replace_plugin_models(&self, provider_id: &str, refreshed: RefreshedCatalog) {
        if let Ok(mut guard) = self.dynamic_models.write() {
            guard.insert(provider_id.to_string(), refreshed);
        }
    }

    /// plugin `refreshModels` の結果（`ModelDefinitionJson` list）を
    /// `applyExtension` と同一規則で resolved `CatalogModel` へ変換し、
    /// compose と同一の `merge_user_model_defs` で user `models[]` を
    /// re-merge・`model_overrides` を再適用・header source を再構築して
    /// `replace_plugin_models` で反映する(F13)。空 list は現在の model
    /// list 維持として扱う（persisted snapshot や前回 refresh の結果を
    /// 巻き戻さない — TS `refreshModels` は model 消失を意味しない）。
    /// merge の衝突判定は refresh ごとに再評価される —
    /// plugin catalog から消えた id の user def は effective になり、
    /// 復活した id では list 側が優先 + error 記録される。
    pub fn refresh_plugin_models(
        &self,
        provider_id: &str,
        definitions: &[ModelDefinitionJson],
    ) -> Result<(), String> {
        let Some(entry) = self.provider(provider_id) else {
            return Err(format!("Provider {provider_id}: not registered"));
        };
        if definitions.is_empty() {
            return Ok(());
        }
        // defaults pool は compose 時に `apply_extension` が受け取った
        // pre-extension base list（plugin-only provider では空）を使う —
        // 現在の resolved list を渡すと merge 済み user def・前回 refresh
        // model の `baseUrl` が継承され、compose 時は失敗するはずの def
        // が変換できて snapshot 化される（次回 boot compose が失敗し
        // provider ごと drop → login 修復不能）のを防ぐため。
        let mut models = extension_models(
            provider_id,
            definitions,
            entry.plugin_api.as_deref().unwrap_or_default(),
            entry.base_url.as_deref().unwrap_or_default(),
            &entry.refresh.extension_base_models,
        )?;
        // compose と同一 helper・同一 fallback chain で user `models[]`
        // merge を再実行。非空 filter は helper 内部の単一 site のため
        // ここでは `as_deref()` のみで filter しない。戻り値の merged
        // defs は header source の参照先（skip された def の headers は
        // 残存 model に混入させない）。
        let mut merge_errors = Vec::new();
        let merged_user_defs = merge_user_model_defs(
            provider_id,
            &mut models,
            &entry.refresh.plugin_user_defs,
            entry
                .refresh
                .config_api
                .as_deref()
                .or(entry.plugin_api.as_deref()),
            entry
                .refresh
                .config_base_url
                .as_deref()
                .or(entry.base_url.as_deref()),
            &mut merge_errors,
        );
        let merge_error_count = merge_errors.len();
        for msg in merge_errors {
            self.record_error(format!("Provider \"{provider_id}\": {msg}"));
        }
        if let Some(overrides) = &entry.refresh.model_overrides {
            for model in &mut models {
                if let Some(ov) = overrides.get(&model.id) {
                    apply_model_override(model, ov);
                }
            }
        }
        let model_header_sources = collect_model_header_sources(
            &models,
            entry.refresh.model_overrides.as_ref(),
            &merged_user_defs,
            definitions,
        );
        let model_count = models.len();
        self.replace_plugin_models(
            provider_id,
            RefreshedCatalog {
                models,
                model_header_sources,
            },
        );
        // `cheaper.catalog` event — plugin refresh defs の re-merge
        // 境界(compose と同一 helper 群)。`model.count` は反映後の
        // resolved model 数、`error.count` は merge 由来の記録数。
        tracing::debug!(
            target: "cheaper.catalog",
            op = "merge",
            provider = provider_id,
            model.count = model_count,
            error.count = merge_error_count,
            "plugin catalog merged"
        );
        Ok(())
    }

    /// `getModel(providerId, modelId)` 相当。
    pub fn get_model(&self, provider_id: &str, model_id: &str) -> Option<CatalogModel> {
        let entry = self.provider(provider_id)?;
        self.models_of(entry).into_iter().find(|m| m.id == model_id)
    }

    /// model の api 種別が初期スコープ内か（builtin 3 種 or plugin 宣言
    /// api）。`ensure_supported_api` の判定用に分離。
    pub fn is_supported_api(&self, model: &CatalogModel) -> bool {
        if SUPPORTED_APIS.contains(&model.api.as_str()) {
            return true;
        }
        self.provider(&model.provider)
            .and_then(|p| p.plugin_api.as_deref())
            .filter(|a| !a.is_empty())
            .map(|a| a == model.api)
            .unwrap_or(false)
    }

    /// G1 contract: 未知・スコープ外 api 種別の model を選択対象にしようと
    /// した場合の明示 error。silent fallback は行わない。
    pub fn ensure_supported_api(&self, model: &CatalogModel) -> Result<(), CheaperError> {
        if self.is_supported_api(model) {
            return Ok(());
        }
        let mut details = Map::new();
        details.insert("provider".to_string(), Value::from(model.provider.clone()));
        details.insert("id".to_string(), Value::from(model.id.clone()));
        details.insert("api".to_string(), Value::from(model.api.clone()));
        details.insert(
            "supportedApis".to_string(),
            Value::Array(SUPPORTED_APIS.iter().map(|s| Value::from(*s)).collect()),
        );
        Err(invalid_model_config(
            format!(
                "unsupported provider api: {} (model {}/{})",
                model.api, model.provider, model.id
            ),
            details,
        ))
    }

    /// `get_model` + api 種別 check。選択対象（explicit resolve 後段・run
    /// 経路）で使う。model 不在は Ok(None)。
    pub fn get_supported_model(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Option<CatalogModel>, CheaperError> {
        let Some(model) = self.get_model(provider_id, model_id) else {
            return Ok(None);
        };
        self.ensure_supported_api(&model)?;
        Ok(Some(model))
    }
}
