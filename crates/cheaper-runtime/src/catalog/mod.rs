//! Model catalog store（G1）。
//!
//! bundled 静的 snapshot（`bundled.rs`）+ plugin 登録 provider +
//! user `<userConfigDir>/models.json` overlay を統合した catalog。
//! `get_available`（認証済み model のみ候補化）・`get_model` を提供し、
//! 未知・スコープ外の provider api 種別は `INVALID_MODEL_CONFIG` で明示
//! error にする（silent fallback 禁止）。
//!
//! merge 規則は pi-coding-agent `provider-composer.ts` の port:
//! `applyModelsJson`（builtin base + config overlay）→ `applyExtension`
//! （plugin provider model replace/baseUrl）→ `modelOverrides` per-model。
//! compose error は TS の `compositionErrors` 相当として記録し、builtin は
//! base fallback・custom/plugin は provider 自体を drop する。

pub mod bundled;
pub mod schema;

use std::collections::BTreeMap;
use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::plugin::PluginProviderRegistration;

pub use bundled::{
    bundled_generated_at_ms, bundled_models, BuiltinProviderMeta, EnvAuth, OAuthSupport,
    BUILTIN_PROVIDERS,
};
pub use schema::{
    load_models_json, ModelCostJson, ModelCostOverrideJson, ModelCostTierJson, ModelDefinitionJson,
    ModelInput, ModelOverrideJson, ModelsJsonFile, ProviderConfigJson, ThinkingLevelMap,
};

/// `cheaper run`/`models` の対象となる api 種別（初期スコープ:
/// anthropic / openai / openai-compatible(Ollama 含む) + plugin 宣言 api）。
/// `github-copilot` の model はこの 3 種に収まる。devin は plugin 宣言
/// api kind で supported 扱いになる。これ以外の api 種別を持つ model を
/// 選択対象として解決しようとした場合は `INVALID_MODEL_CONFIG` を返す。
pub const SUPPORTED_APIS: &[&str] = &[
    "anthropic-messages",
    "openai-completions",
    "openai-responses",
];

/// cost tier（`ModelCostTier` resolved 形）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCostTier {
    pub input_tokens_above: f64,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// Resolved model cost（bundled/definition は 4 rates 必須、default は 0）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModelCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiers: Option<Vec<CatalogCostTier>>,
}

impl Default for CatalogModelCost {
    fn default() -> Self {
        Self {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            tiers: None,
        }
    }
}

/// Pi `Model<Api>` 相当の resolved model record。bundled data file・
/// `models[]` definition・plugin `ProviderModelConfig` の 3 経路から構築
/// される。`ModelInfo` 変換に必要な field（provider/id/cost/maxTokens）に
/// 加え、adapter が request 組立に使う api/baseUrl/headers/compat/
/// samplingParams も保持する。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogModel {
    pub id: String,
    pub name: String,
    /// api 種別（`anthropic-messages` / `openai-completions` /
    /// `openai-responses` / plugin 宣言種別 / 未知 → `INVALID_MODEL_CONFIG`）。
    pub api: String,
    pub provider: String,
    pub base_url: String,
    pub reasoning: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_level_map: Option<ThinkingLevelMap>,
    pub input: Vec<ModelInput>,
    pub cost: CatalogModelCost,
    pub context_window: f64,
    pub max_tokens: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling_params: Option<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<Map<String, Value>>,
    /// `ProviderCompat` union — adapter が wire 組立に使う生 JSON。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compat: Option<Value>,
}

/// `modelOverrides` 適用後の最終 model（`get_available`/`get_model` が返す型）。
pub type Model = CatalogModel;

/// provider の出自。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderOrigin {
    /// pi-ai builtin（bundled data + 静的 meta）。
    Builtin,
    /// user `models.json` のみで定義された custom provider。
    UserConfig,
    /// `CheaperProviderPlugin` 登録 provider（devin 等）。
    Plugin,
}

/// `radius` gateway 等、`oauth: "radius"` を宣言する provider config 由来の
/// OAuth capability（flow は持たない → method 非 advertise）。
#[derive(Debug, Clone, Copy)]
pub struct OauthDeclaration {
    pub subscription: bool,
}

/// provider 単位の auth 関連 metadata。plugin provider は `plugin` field が
/// credential/catalog/logout を一手に引き受ける。
#[derive(Clone)]
pub struct ProviderEntry {
    pub id: String,
    /// display name（`ProviderAuthInfo.name`）。
    pub name: String,
    pub base_url: Option<String>,
    pub origin: ProviderOrigin,
    /// api_key login prompt name（Some = api_key login 可能）。
    /// builtin は meta の `auth.apiKey.name`、custom は `"API key"` 固定
    /// （`composeApiKeyAuth` の fabricated login `"Enter API key"`）。
    pub api_key_name: Option<String>,
    /// builtin env var auth 規則（custom/plugin は `EnvAuth::None`）。
    pub env_auth: EnvAuth,
    /// models.json `apiKey` 生値（template/`!command` を含みうる）。
    pub configured_api_key: Option<String>,
    /// `authHeader` — Authorization Bearer を request headers に差す flag。
    pub auth_header: bool,
    /// provider-level configured headers（config.headers + plugin headers
    /// の merge。request auth 側に merge される template 文字列）。
    pub configured_headers: Option<Map<String, Value>>,
    /// OAuth capability 宣言（builtin meta or `oauth: "radius"` config or
    /// plugin oauth）。
    pub oauth: Option<OauthDeclaration>,
    /// plugin provider の auth/catalog/logout hook（plugin origin のみ）。
    pub plugin_auth: Option<Arc<dyn crate::plugin::PluginAuth>>,
    /// plugin `refreshModels` seam — login 成功後に live catalog を
    /// 更新するため runtime が保持する handle(F13)。
    pub plugin_catalog: Option<Arc<dyn crate::plugin::PluginCatalog>>,
    /// plugin 宣言 api kind（plugin provider のみ。supported 扱い）。
    pub plugin_api: Option<String>,
    /// resolved models（`get_available`/`get_model` が返す最終形）。
    pub models: Vec<CatalogModel>,
    /// `modelOverrides` 適用後に per-model で残す headers merge source:
    /// `{modelOverride.headers, models[].headers, plugin model.headers}`。
    /// （`rawModelHeaders` port — catalog 側は template 未解決のまま保持）
    model_header_sources: BTreeMap<String, Map<String, Value>>,
    /// user `modelOverrides` — plugin `refreshModels` で差し替わる
    /// model list にも再適用するため compose 時の値を保持する。
    model_overrides: Option<BTreeMap<String, ModelOverrideJson>>,
}

impl std::fmt::Debug for ProviderEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderEntry")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("origin", &self.origin)
            .field("api_key_name", &self.api_key_name)
            .field("env_auth", &self.env_auth)
            .field("has_configured_api_key", &self.configured_api_key.is_some())
            .field("auth_header", &self.auth_header)
            .field("oauth", &self.oauth)
            .field("has_plugin_auth", &self.plugin_auth.is_some())
            .field("has_plugin_catalog", &self.plugin_catalog.is_some())
            .field("plugin_api", &self.plugin_api)
            .field("models", &self.models.len())
            .finish()
    }
}

impl ProviderEntry {
    /// model の request headers（raw template 文字列 merge 済み）。
    /// precedence: modelOverride.headers < models[].headers < plugin model
    /// headers（`rawModelHeaders` の spread 順を保持）。
    pub fn model_headers(&self, model_id: &str) -> Option<Map<String, Value>> {
        self.model_header_sources.get(model_id).cloned()
    }
}

fn invalid_model_config(message: String, details: Map<String, Value>) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidModelConfig, message).with_details(details)
}

/// `mergeCompat` — shallow merge + nested key
/// （`openRouterRouting`/`vercelGatewayRouting`/`chatTemplateKwargs`/
/// `chatTemplateArgs`）のみ per-key merge。
fn merge_compat(base: &Option<Value>, override_: &Option<Value>) -> Option<Value> {
    let override_ = override_.as_ref()?;
    let mut merged = match base {
        Some(Value::Object(m)) => m.clone(),
        _ => Map::new(),
    };
    if let Value::Object(ov) = override_ {
        for (k, v) in ov {
            merged.insert(k.clone(), v.clone());
        }
    }
    const NESTED: &[&str] = &[
        "openRouterRouting",
        "vercelGatewayRouting",
        "chatTemplateKwargs",
        "chatTemplateArgs",
    ];
    if let Some(Value::Object(ov_obj)) = Some(override_) {
        for key in NESTED {
            let base_v = base
                .as_ref()
                .and_then(|b| b.as_object())
                .and_then(|o| o.get(*key));
            let ov_v = ov_obj.get(*key);
            let base_is_obj = matches!(base_v, Some(Value::Object(_)));
            let ov_is_obj = matches!(ov_v, Some(Value::Object(_)));
            if base_is_obj || ov_is_obj {
                let mut nested = match base_v {
                    Some(Value::Object(m)) => m.clone(),
                    _ => Map::new(),
                };
                if let Some(Value::Object(m)) = ov_v {
                    for (k, v) in m {
                        nested.insert(k.clone(), v.clone());
                    }
                }
                merged.insert((*key).to_string(), Value::Object(nested));
            }
        }
    }
    Some(Value::Object(merged))
}

/// `applyModelOverride` — modelOverrides entry の per-field merge。
fn apply_model_override(model: &mut CatalogModel, override_: &ModelOverrideJson) {
    if let Some(v) = &override_.name {
        model.name = v.clone();
    }
    if let Some(v) = override_.reasoning {
        model.reasoning = v;
    }
    if let Some(v) = &override_.thinking_level_map {
        model.thinking_level_map = Some(match &model.thinking_level_map {
            Some(base) => base.merged(v),
            None => v.clone(),
        });
    }
    if let Some(v) = &override_.input {
        model.input = v.clone();
    }
    if let Some(cost) = &override_.cost {
        model.cost.input = cost.input.unwrap_or(model.cost.input);
        model.cost.output = cost.output.unwrap_or(model.cost.output);
        model.cost.cache_read = cost.cache_read.unwrap_or(model.cost.cache_read);
        model.cost.cache_write = cost.cache_write.unwrap_or(model.cost.cache_write);
        if cost.tiers.is_some() {
            model.cost.tiers = cost
                .tiers
                .as_ref()
                .map(|ts| ts.iter().map(cost_tier_from_json).collect());
        }
    }
    if let Some(v) = override_.context_window {
        model.context_window = v;
    }
    if let Some(v) = override_.max_tokens {
        model.max_tokens = v;
    }
    if let Some(v) = &override_.sampling_params {
        let merged = model.sampling_params.get_or_insert_with(Map::new);
        for (k, val) in v {
            merged.insert(k.clone(), val.clone());
        }
    }
    model.compat = merge_compat(&model.compat, &override_.compat);
}

fn cost_tier_from_json(t: &ModelCostTierJson) -> CatalogCostTier {
    CatalogCostTier {
        input_tokens_above: t.input_tokens_above,
        input: t.input,
        output: t.output,
        cache_read: t.cache_read,
        cache_write: t.cache_write,
    }
}

/// `findModelDefaults` — definition の defaults 参照先 model。
fn find_model_defaults<'a>(
    models: &'a [CatalogModel],
    model_id: &str,
    api: Option<&str>,
) -> Option<&'a CatalogModel> {
    models
        .iter()
        .find(|m| m.id == model_id)
        .or_else(|| api.and_then(|a| models.iter().find(|m| m.api == a)))
        .or_else(|| models.iter().find(|m| m.api == "openai-completions"))
        .or_else(|| models.first())
}

/// `modelFromJson` — `models[]` definition → resolved `CatalogModel`。
fn model_from_json(
    provider_id: &str,
    definition: &ModelDefinitionJson,
    provider_api: Option<&str>,
    provider_base_url: Option<&str>,
    defaults: Option<&CatalogModel>,
) -> Result<CatalogModel, String> {
    let api = definition
        .api
        .clone()
        .or_else(|| provider_api.map(str::to_string))
        .or_else(|| defaults.map(|d| d.api.clone()));
    let Some(api) = api else {
        return Err(format!(
            "Provider {provider_id}, model {}: no \"api\" specified. Set at provider or model level.",
            definition.id
        ));
    };
    let base_url = definition
        .base_url
        .clone()
        .or_else(|| provider_base_url.map(str::to_string))
        .or_else(|| defaults.map(|d| d.base_url.clone()));
    let Some(base_url) = base_url else {
        return Err(format!(
            "Provider {provider_id}: \"baseUrl\" is required when defining custom models."
        ));
    };
    if let Some(cw) = definition.context_window {
        if cw <= 0.0 {
            return Err(format!(
                "Provider {provider_id}, model {}: invalid contextWindow",
                definition.id
            ));
        }
    }
    if let Some(mt) = definition.max_tokens {
        if mt <= 0.0 {
            return Err(format!(
                "Provider {provider_id}, model {}: invalid maxTokens",
                definition.id
            ));
        }
    }
    Ok(CatalogModel {
        id: definition.id.clone(),
        name: definition
            .name
            .clone()
            .unwrap_or_else(|| definition.id.clone()),
        api,
        provider: provider_id.to_string(),
        base_url,
        reasoning: definition.reasoning.unwrap_or(false),
        thinking_level_map: definition.thinking_level_map.clone(),
        input: definition
            .input
            .clone()
            .unwrap_or_else(|| vec![ModelInput::Text]),
        cost: definition
            .cost
            .as_ref()
            .map(|c| CatalogModelCost {
                input: c.input,
                output: c.output,
                cache_read: c.cache_read,
                cache_write: c.cache_write,
                tiers: c
                    .tiers
                    .as_ref()
                    .map(|ts| ts.iter().map(cost_tier_from_json).collect()),
            })
            .unwrap_or_default(),
        context_window: definition.context_window.unwrap_or(128_000.0),
        max_tokens: definition.max_tokens.unwrap_or(16_384.0),
        sampling_params: definition.sampling_params.clone(),
        // `modelFromJson` は `headers: undefined` を置く（headers は
        // `rawModelHeaders` 経由で request 時に別 merge される）。
        headers: None,
        compat: definition.compat.clone(),
    })
}

/// `applyModelsJson` — builtin/plugin base models + user config overlay。
fn apply_models_json(
    provider_id: &str,
    base_models: &[CatalogModel],
    config: &ProviderConfigJson,
) -> Result<Vec<CatalogModel>, String> {
    if config.oauth.is_some() && config.base_url.is_none() {
        return Err(format!(
            "Provider {provider_id}: \"baseUrl\" is required when \"oauth\" is set."
        ));
    }
    let has_overrides = config
        .model_overrides
        .as_ref()
        .map(|m| !m.is_empty())
        .unwrap_or(false);
    let models_empty = config.models.as_ref().map(|m| m.is_empty()).unwrap_or(true);
    if models_empty
        && config.base_url.is_none()
        && config.headers.is_none()
        && config.compat.is_none()
        && !has_overrides
        && config.api_key.is_none()
        && config.oauth.is_none()
        && config.auth_header.is_none()
    {
        return Err(format!(
            "Provider {provider_id}: must specify \"baseUrl\", \"headers\", \"compat\", \"modelOverrides\", or \"models\"."
        ));
    }
    let mut models: Vec<CatalogModel> = base_models
        .iter()
        .map(|m| CatalogModel {
            base_url: if config.oauth.as_deref() == Some("radius") {
                m.base_url.clone()
            } else {
                config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| m.base_url.clone())
            },
            compat: merge_compat(&m.compat, &config.compat),
            ..m.clone()
        })
        .collect();
    for definition in config.models.as_deref().unwrap_or(&[]) {
        let existing = models.iter().position(|m| m.id == definition.id);
        let defaults = find_model_defaults(
            &models,
            &definition.id,
            definition.api.as_deref().or(config.api.as_deref()),
        );
        let model = model_from_json(
            provider_id,
            definition,
            config.api.as_deref(),
            config.base_url.as_deref(),
            defaults,
        )?;
        match existing {
            Some(idx) => models[idx] = model,
            None => models.push(model),
        }
    }
    Ok(models)
}

/// `applyExtension` — plugin provider の models/baseUrl を適用する。
/// `extension.models` がある場合は model list を丸ごと replace する。
fn apply_extension(
    provider_id: &str,
    models: Vec<CatalogModel>,
    plugin: &PluginProviderRegistration,
) -> Result<Vec<CatalogModel>, String> {
    if plugin.models.is_empty() {
        // extension models 無し → baseUrl override のみ。
        if plugin.base_url.is_empty() {
            return Ok(models);
        }
        return Ok(models
            .into_iter()
            .map(|m| CatalogModel {
                base_url: plugin.base_url.clone(),
                ..m
            })
            .collect());
    }
    // TS applyExtension: `{...definition, api, provider, baseUrl, headers:
    // undefined}` — cost/contextWindow 等は definition verbatim（default 値
    // を埋めない）。headers は model 側を undefined にし rawModelHeaders
    // 経由で扱う。
    extension_models(
        provider_id,
        &plugin.models,
        &plugin.api,
        &plugin.base_url,
        &models,
    )
}

/// `applyExtension` の per-definition model 変換（bootstrap models と
/// `refreshModels` 結果の両経路で共有 — F13 DRY）。`plugin_api`/
/// `plugin_base_url` は definition 欠落時の fallback、`base_models` は
/// `findModelDefaults` の参照先。
fn extension_models(
    provider_id: &str,
    definitions: &[ModelDefinitionJson],
    plugin_api: &str,
    plugin_base_url: &str,
    base_models: &[CatalogModel],
) -> Result<Vec<CatalogModel>, String> {
    definitions
        .iter()
        .map(|definition| {
            let defaults = find_model_defaults(
                base_models,
                &definition.id,
                definition.api.as_deref().or(Some(plugin_api)),
            );
            let api = definition
                .api
                .clone()
                .unwrap_or_else(|| plugin_api.to_string());
            let base_url = definition
                .base_url
                .clone()
                .or_else(|| {
                    if plugin_base_url.is_empty() {
                        None
                    } else {
                        Some(plugin_base_url.to_string())
                    }
                })
                .or_else(|| defaults.map(|d| d.base_url.clone()));
            let Some(base_url) = base_url else {
                return Err(format!(
                    "Provider {provider_id}: \"baseUrl\" is required when defining custom models."
                ));
            };
            Ok(CatalogModel {
                id: definition.id.clone(),
                name: definition
                    .name
                    .clone()
                    .unwrap_or_else(|| definition.id.clone()),
                api,
                provider: provider_id.to_string(),
                base_url,
                reasoning: definition.reasoning.unwrap_or(false),
                thinking_level_map: definition.thinking_level_map.clone(),
                input: definition
                    .input
                    .clone()
                    .unwrap_or_else(|| vec![ModelInput::Text]),
                cost: definition
                    .cost
                    .as_ref()
                    .map(|c| CatalogModelCost {
                        input: c.input,
                        output: c.output,
                        cache_read: c.cache_read,
                        cache_write: c.cache_write,
                        tiers: c
                            .tiers
                            .as_ref()
                            .map(|ts| ts.iter().map(cost_tier_from_json).collect()),
                    })
                    .unwrap_or_default(),
                context_window: definition.context_window.unwrap_or(128_000.0),
                max_tokens: definition.max_tokens.unwrap_or(16_384.0),
                sampling_params: definition.sampling_params.clone(),
                headers: None,
                compat: definition.compat.clone(),
            })
        })
        .collect()
}

/// provider 単位の compose（`recomposeProvider` + `composeModelProvider`）。
/// `base_meta`/`base_models` は builtin descriptor + bundled models、
/// `config` は user models.json entry、`plugin` は plugin registration。
/// compose error は Err(String)（TS `compositionErrors` message 相当）で
/// caller が fallback/drop を決める。
fn compose_provider(
    provider_id: &str,
    base_meta: Option<&BuiltinProviderMeta>,
    base_models: &[CatalogModel],
    config: Option<&ProviderConfigJson>,
    plugin: Option<&PluginProviderRegistration>,
) -> Result<ProviderEntry, String> {
    // models 層: applyModelsJson → applyExtension → modelOverrides。
    let mut models = match config {
        Some(c) => apply_models_json(provider_id, base_models, c)?,
        None => base_models.to_vec(),
    };
    if let Some(p) = plugin {
        models = apply_extension(provider_id, models, p)?;
    }
    // modelOverrides は最終層（custom-model upsert・extension replace の後）。
    if let Some(overrides) = config.and_then(|c| c.model_overrides.as_ref()) {
        for model in &mut models {
            if let Some(ov) = overrides.get(&model.id) {
                apply_model_override(model, ov);
            }
        }
    }

    // `rawModelHeaders` 相当の per-model source merge（template 未解決）。
    let mut model_header_sources: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    {
        let overrides = config.and_then(|c| c.model_overrides.as_ref());
        let defs = config.and_then(|c| c.models.as_deref());
        for model in &models {
            let mut merged = Map::new();
            if let Some(h) = overrides
                .and_then(|o| o.get(&model.id))
                .and_then(|o| o.headers.as_ref())
            {
                for (k, v) in h {
                    merged.insert(k.clone(), v.clone());
                }
            }
            if let Some(h) = defs
                .and_then(|ds| ds.iter().find(|d| d.id == model.id))
                .and_then(|d| d.headers.as_ref())
            {
                for (k, v) in h {
                    merged.insert(k.clone(), v.clone());
                }
            }
            if let Some(p) = plugin {
                if let Some(h) = p
                    .models
                    .iter()
                    .find(|d| d.id == model.id)
                    .and_then(|d| d.headers.as_ref())
                {
                    for (k, v) in h {
                        merged.insert(k.clone(), v.clone());
                    }
                }
            }
            if !merged.is_empty() {
                model_header_sources.insert(model.id.clone(), merged);
            }
        }
    }

    // auth 層（`composeApiKeyAuth`/`composeOAuthAuth` の method 有無判定）。
    // `rawKey = configuredApiKey(config, extension)` = plugin.apiKey ?? config.apiKey。
    let raw_key = plugin
        .and_then(|p| p.api_key.clone())
        .or_else(|| config.and_then(|c| c.api_key.clone()));
    // TS の `oauth = extension?.oauth ?? base?.auth.oauth`。`config.oauth`
    // （models.json の "radius" 宣言）は method 判定に使われない。
    let ts_oauth = plugin.map(|p| p.auth.is_some()).unwrap_or(false)
        || base_meta.map(|m| m.oauth.is_some()).unwrap_or(false);
    // `oauth:"radius"` 宣言 provider は `radiusProvider` 相当に差し替わる
    // （api_key=RADIUS_API_KEY env + radius oauth 宣言 + remote catalog）。
    let radius_gateway = config.and_then(|c| c.oauth.as_deref()) == Some("radius");
    let oauth_decl: Option<OauthDeclaration> = if let Some(p) = plugin {
        if p.auth.is_some() {
            Some(OauthDeclaration {
                subscription: p.subscription,
            })
        } else {
            base_meta.and_then(|m| m.oauth).map(|o| OauthDeclaration {
                subscription: o.subscription,
            })
        }
    } else {
        base_meta
            .and_then(|m| m.oauth)
            .map(|o| OauthDeclaration {
                subscription: o.subscription,
            })
            .or(radius_gateway.then_some(OauthDeclaration {
                subscription: false,
            }))
    };
    // `!inherited && rawKey === undefined && oauth` → api_key auth なし。
    let inherited = base_meta.and_then(|m| m.api_key_name);
    let api_key_name: Option<String> = if inherited.is_none() && raw_key.is_none() && ts_oauth {
        None
    } else if radius_gateway && inherited.is_none() {
        Some("Radius API key".to_string())
    } else {
        Some(inherited.unwrap_or("API key").to_string())
    };
    if api_key_name.is_none() && oauth_decl.is_none() {
        return Err(format!(
            "Provider {provider_id}: no authentication method configured."
        ));
    }

    let name = plugin
        .map(|p| p.name.clone())
        .or_else(|| config.and_then(|c| c.name.clone()))
        .or_else(|| base_meta.map(|m| m.name.to_string()))
        .unwrap_or_else(|| provider_id.to_string());
    let base_url = plugin
        .map(|p| p.base_url.clone())
        .or_else(|| config.and_then(|c| c.base_url.clone()))
        .or_else(|| base_meta.and_then(|m| m.base_url.map(str::to_string)))
        .or_else(|| models.first().map(|m| m.base_url.clone()));
    // `configuredHeaders(config, extension)`: config.headers + plugin headers。
    let configured_headers = {
        let mut merged = Map::new();
        if let Some(h) = config.and_then(|c| c.headers.as_ref()) {
            for (k, v) in h {
                merged.insert(k.clone(), v.clone());
            }
        }
        if let Some(h) = plugin.and_then(|p| p.headers.as_ref()) {
            for (k, v) in h {
                merged.insert(k.clone(), v.clone());
            }
        }
        if merged.is_empty() {
            None
        } else {
            Some(merged)
        }
    };

    Ok(ProviderEntry {
        id: provider_id.to_string(),
        name,
        base_url,
        origin: if plugin.is_some() {
            ProviderOrigin::Plugin
        } else if base_meta.is_some() {
            ProviderOrigin::Builtin
        } else {
            ProviderOrigin::UserConfig
        },
        api_key_name,
        env_auth: if radius_gateway {
            EnvAuth::Vars(&["RADIUS_API_KEY"])
        } else {
            base_meta.map(|m| m.env_auth).unwrap_or(EnvAuth::None)
        },
        configured_api_key: raw_key,
        auth_header: plugin
            .and_then(|p| p.auth_header)
            .or_else(|| config.and_then(|c| c.auth_header))
            .unwrap_or(false),
        configured_headers,
        oauth: oauth_decl,
        plugin_auth: plugin.and_then(|p| p.auth.clone()),
        plugin_catalog: plugin.and_then(|p| p.catalog.clone()),
        plugin_api: plugin.map(|p| p.api.clone()),
        models,
        model_header_sources,
        model_overrides: config.and_then(|c| c.model_overrides.clone()),
    })
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
    dynamic_models: std::sync::RwLock<BTreeMap<String, Vec<CatalogModel>>>,
}

impl CatalogStore {
    /// catalog を構築する。`user_models` は `load_models_json` の結果
    /// （parse/schema error は `user_error` で記録）。
    pub fn build(
        user_models: Option<ModelsJsonFile>,
        user_error: Option<String>,
        plugin_registrations: &[PluginProviderRegistration],
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
            match compose_provider(
                id,
                meta,
                &base_models,
                config_map.get(id.as_str()).copied(),
                plugin_map.get(id.as_str()).copied(),
            ) {
                Ok(entry) => providers.push(entry),
                Err(msg) => {
                    // `compositionErrors` — builtin は untouched fallback、
                    // custom/plugin-only は provider を drop する。
                    errors.push(format!("Provider \"{id}\": {msg}"));
                    if let Some(meta) = meta {
                        // base fallback（overlay 無しの untouched builtin）。
                        match compose_provider(id, Some(meta), &base_models, None, None) {
                            Ok(entry) => providers.push(entry),
                            Err(fatal) => {
                                errors.push(format!("Provider \"{id}\": {fatal}"));
                            }
                        }
                    }
                }
            }
        }

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
            if let Some(models) = guard.get(&entry.id) {
                return models.clone();
            }
        }
        entry.models.clone()
    }

    /// plugin login 後の catalog refresh（devin `devin models list`）。
    /// `modelOverrides` は差し替え後の model にも適用する。
    pub fn replace_plugin_models(&self, provider_id: &str, models: Vec<CatalogModel>) {
        if let Ok(mut guard) = self.dynamic_models.write() {
            guard.insert(provider_id.to_string(), models);
        }
    }

    /// plugin `refreshModels` の結果（`ModelDefinitionJson` list）を
    /// `applyExtension` と同一規則で resolved `CatalogModel` へ変換し、
    /// user `modelOverrides` を再適用してから `replace_plugin_models` で
    /// 反映する(F13)。空 list は bootstrap list 維持として扱う
    /// （TS `refreshModels` は model 消失を意味しない）。
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
        let mut models = extension_models(
            provider_id,
            definitions,
            entry.plugin_api.as_deref().unwrap_or_default(),
            entry.base_url.as_deref().unwrap_or_default(),
            &self.models_of(entry),
        )?;
        if let Some(overrides) = &entry.model_overrides {
            for model in &mut models {
                if let Some(ov) = overrides.get(&model.id) {
                    apply_model_override(model, ov);
                }
            }
        }
        self.replace_plugin_models(provider_id, models);
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
