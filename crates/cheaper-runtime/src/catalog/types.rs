//! Catalog resolved-type definitions — `CatalogModel`/`ProviderEntry`/
//! `ProviderOrigin`/`OauthDeclaration`/`SUPPORTED_APIS`。
//! compose 規則は `compose.rs`、store 本体は `store.rs`。

use super::bundled::EnvAuth;
use super::schema::{ModelDefinitionJson, ModelInput, ModelOverrideJson, ThinkingLevelMap};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// `cheaper run`/`models` の対象となる api 種別（初期スコープ:
/// anthropic / openai / openai-compatible(Ollama 含む) + plugin 宣言 api）。
/// `github-copilot` の model はこの 3 種に収まる。devin は plugin 宣言
/// api kind で supported 扱いになる。これ以外の api 種別を持つ model を
/// 選択対象として解決しようとした場合は `INVALID_MODEL_CONFIG` を返す。
pub const SUPPORTED_APIS: &[&str] = &[
    "anthropic-messages",
    "openai-completions",
    "openai-responses",
    "openai-codex-responses",
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
/// OAuth capability。builtin `OAuthSupport` と同じく declaration の有無
/// だけで method を advertise する（flow 登録は login 実行可否のみ制御）。
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
    pub(crate) model_header_sources: BTreeMap<String, Map<String, Value>>,
    /// user `modelOverrides` — plugin `refreshModels` で差し替わる
    /// model list にも再適用するため compose 時の値を保持する。
    pub(crate) model_overrides: Option<BTreeMap<String, ModelOverrideJson>>,
    /// plugin provider の raw user `models[]` defs（refresh 時に同一 merge
    /// を再実行するため resolved 形ではなく definition のまま保持。
    /// `model_overrides` と同じ構造）。plugin provider 以外は空。
    pub(crate) plugin_user_defs: Vec<ModelDefinitionJson>,
    /// compose 時に `apply_extension` が受け取った pre-extension base list
    /// （post-overlay builtin base — plugin-only provider では空）。
    /// `refresh_plugin_models` が `extension_models` を compose と同一の
    /// defaults pool で再実行するために保持する — 現在の resolved list
    /// （merge 済み user def・前回 refresh model を含む）を渡すと compose
    /// 不可能な経路で `baseUrl` が継承され、その def が snapshot 化されると
    /// 次回 boot の compose が同じ defs で失敗し provider ごと drop する。
    /// plugin provider 以外は空。
    pub(crate) extension_base_models: Vec<CatalogModel>,
    /// merge fallback chain の config 層（`config.api`）。
    pub(crate) config_api: Option<String>,
    /// merge fallback chain の config 層（`config.baseUrl`）。
    pub(crate) config_base_url: Option<String>,
    /// `register()` が emit した `registration.models` の不変 copy —
    /// persist guard (c) の比較基準（snapshot 注入の影響を受けない）。
    /// `auth/service.rs` の `persist_plugin_catalog` が読むため
    /// `pub(crate)`。plugin provider 以外は空。
    pub(crate) plugin_bootstrap_defs: Vec<ModelDefinitionJson>,
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
            .field("extension_base_models", &self.extension_base_models.len())
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
