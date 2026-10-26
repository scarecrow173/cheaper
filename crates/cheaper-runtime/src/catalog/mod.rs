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
mod compose;
mod history_identity;
pub mod persisted;
pub mod schema;
mod store;
mod types;

pub use bundled::{
    bundled_generated_at_ms, bundled_models, BuiltinProviderMeta, EnvAuth, OAuthSupport,
    BUILTIN_PROVIDERS,
};
pub use history_identity::CatalogHistoryIdentity;
pub use schema::{
    load_models_json, ModelCostJson, ModelCostOverrideJson, ModelCostTierJson, ModelDefinitionJson,
    ModelInput, ModelOverrideJson, ModelsJsonFile, ProviderConfigJson, ThinkingLevelMap,
};
pub use store::CatalogStore;
pub use types::{
    CatalogCostTier, CatalogModel, CatalogModelCost, Model, OauthDeclaration, ProviderEntry,
    ProviderOrigin, SUPPORTED_APIS,
};
