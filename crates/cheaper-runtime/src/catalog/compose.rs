//! Catalog composition — `applyModelsJson`/`applyExtension`/
//! `modelOverrides` の merge 規則（pi-coding-agent `provider-composer.ts`
//! の port）。private helper 群は `pub(crate)` で store/types へ公開するが
//! crate 外には露出しない。

use super::bundled::{BuiltinProviderMeta, EnvAuth};
use super::schema::{
    ModelCostTierJson, ModelDefinitionJson, ModelInput, ModelOverrideJson, ProviderConfigJson,
};
use super::types::{
    CatalogCostTier, CatalogModel, CatalogModelCost, OauthDeclaration, ProviderEntry,
    ProviderOrigin, RefreshState,
};
use crate::plugin::PluginProviderRegistration;
use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub(crate) fn invalid_model_config(message: String, details: Map<String, Value>) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidModelConfig, message).with_details(details)
}

/// `mergeCompat` — shallow merge + nested key
/// （`openRouterRouting`/`vercelGatewayRouting`/`chatTemplateKwargs`/
/// `chatTemplateArgs`）のみ per-key merge。
pub(crate) fn merge_compat(base: &Option<Value>, override_: &Option<Value>) -> Option<Value> {
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
pub(crate) fn apply_model_override(model: &mut CatalogModel, override_: &ModelOverrideJson) {
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

pub(crate) fn cost_tier_from_json(t: &ModelCostTierJson) -> CatalogCostTier {
    CatalogCostTier {
        input_tokens_above: t.input_tokens_above,
        input: t.input,
        output: t.output,
        cache_read: t.cache_read,
        cache_write: t.cache_write,
    }
}

/// `findModelDefaults` — definition の defaults 参照先 model。
pub(crate) fn find_model_defaults<'a>(
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

/// `ModelDefinitionJson` → `CatalogModel` の field mapping 共有部。
/// `api`/`base_url` は caller 側の経路固有規則で解決済みの値を
/// 受け取る:
/// - `model_from_json` — config/provider-level fallback → defaults で
///   解決し、`context_window`/`max_tokens <= 0` を reject する。
/// - `extension_models` — plugin-level fallback で解決し、
///   cw/mt の非正値は検証せず definition verbatim で通す。
///
/// `context_window`/`max_tokens`/`reasoning`/`input`/`cost` の
/// 未指定時 default 埋め(128_000/16_384/false/[Text]/default)は
/// 両経路で共通の現行挙動。
pub(crate) fn catalog_model_from_definition(
    provider_id: &str,
    definition: &ModelDefinitionJson,
    api: String,
    base_url: String,
) -> CatalogModel {
    CatalogModel {
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
        // `headers` は常に `None` — request 時に `rawModelHeaders`
        // 経由で別 merge される。
        headers: None,
        compat: definition.compat.clone(),
    }
}

/// `modelFromJson` — `models[]` definition → resolved `CatalogModel`。
pub(crate) fn model_from_json(
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
    Ok(catalog_model_from_definition(
        provider_id,
        definition,
        api,
        base_url,
    ))
}

/// `applyModelsJson` の provider-level 検証 — custom/plugin 両経路で
/// 同一 semantics（`oauth` ⇒ `baseUrl` 必須・実効 no-op config の拒否）。
/// plugin provider でも緩めない（error 文字列を経路間で複製しないための
/// 共有 helper）。
pub(crate) fn validate_provider_config(
    provider_id: &str,
    config: &ProviderConfigJson,
) -> Result<(), String> {
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
    Ok(())
}

/// `applyModelsJson` の base-model overlay — base models への
/// `baseUrl`/`compat` merge（`{"oauth":"radius"}` の quirk 含む —
/// radius 宣言 provider は base model の baseUrl を config で上書き
/// しない）。plugin provider でも同一規則で走らせる共有 helper。
pub(crate) fn apply_provider_overlay(
    base_models: &[CatalogModel],
    config: &ProviderConfigJson,
) -> Vec<CatalogModel> {
    base_models
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
        .collect()
}

/// `applyModelsJson` — builtin/plugin base models + user config overlay。
/// provider-level 検証と base overlay は `validate_provider_config` /
/// `apply_provider_overlay` へ集約（plugin provider 経路が検証+overlay
/// のみを再利用する）。per-def `config.models` 解決は custom 経路では
/// 従来どおり `?` 伝播で fatal（provider drop）。
pub(crate) fn apply_models_json(
    provider_id: &str,
    base_models: &[CatalogModel],
    config: &ProviderConfigJson,
) -> Result<Vec<CatalogModel>, String> {
    validate_provider_config(provider_id, config)?;
    let mut models = apply_provider_overlay(base_models, config);
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
/// extension defs は `persisted`（`catalog.json` snapshot）があればそちらを、
/// 無ければ `plugin.models`（bootstrap）を使う。effective defs がある
/// 場合は model list を丸ごと replace する — `plugin.models` が空でも
/// persisted defs があれば `extension_models` が走り、effective defs が
/// 空のときのみ従来の baseUrl-override 分岐を踏む。
pub(crate) fn apply_extension(
    provider_id: &str,
    models: Vec<CatalogModel>,
    plugin: &PluginProviderRegistration,
    persisted: Option<&[ModelDefinitionJson]>,
) -> Result<Vec<CatalogModel>, String> {
    let effective = persisted.unwrap_or(&plugin.models);
    if effective.is_empty() {
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
    // TS applyExtension 相当: `{...definition, api, provider, baseUrl}` +
    // 未指定 field の default 埋め(catalog_model_from_definition)。
    // TS は cost/contextWindow を verbatim で残すが現行実装は default
    // を埋める — 挙動差の是非は Deferred(spec §Deferred)、ここでは
    // 現行挙動を保持する。headers は model 側を None にし
    // rawModelHeaders 経由で扱う。
    extension_models(
        provider_id,
        effective,
        &plugin.api,
        &plugin.base_url,
        &models,
    )
}

/// `applyExtension` の per-definition model 変換（bootstrap models と
/// `refreshModels` 結果の両経路で共有 — F13 DRY）。`plugin_api`/
/// `plugin_base_url` は definition 欠落時の fallback、`base_models` は
/// `findModelDefaults` の参照先。
pub(crate) fn extension_models(
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
            Ok(catalog_model_from_definition(
                provider_id,
                definition,
                api,
                base_url,
            ))
        })
        .collect()
}

/// plugin provider の user `models[]` additive merge — compose と
/// refresh で共有する唯一の merge 実装（DRY）。post-extension list の
/// id と衝突しない def のみ `model_from_json` で解決して末尾へ追加
/// する（plugin 定義順 → user 追加順の file 順）。衝突・解決不能は
/// その def のみ skip + `errors` へ記録し、provider・plugin models は
/// 残す。`plugin.models` の空否に依らず全 plugin provider で走る。
///
/// `fallback_api`/`fallback_base_url` は caller が合成した
/// `config → plugin` の provider-level fallback。非空 filter はこの
/// helper 内部の単一 site に閉じ込める — `model_from_json` は
/// `Some("")` を verbatim 受け入れるため、call site 側で個別に filter
/// すると compose/refresh で規則が drift する。
///
/// 戻り値は model 化に成功した defs（衝突・解決不能で skip された def
/// は含まない）。`collect_model_header_sources` の user_def headers
/// source として caller がそのまま渡す — skip された def の `headers`
/// が残存 model へ混入しないようにするための唯一の判定基準。
pub(crate) fn merge_user_model_defs<'a>(
    provider_id: &str,
    models: &mut Vec<CatalogModel>,
    user_defs: &'a [ModelDefinitionJson],
    fallback_api: Option<&str>,
    fallback_base_url: Option<&str>,
    errors: &mut Vec<String>,
) -> Vec<&'a ModelDefinitionJson> {
    let fallback_api = fallback_api.filter(|s| !s.is_empty());
    let fallback_base_url = fallback_base_url.filter(|s| !s.is_empty());
    let mut merged = Vec::new();
    for definition in user_defs {
        // 衝突判定は merge 時点の post-extension list 全 id が対象
        // （plugin def だけでなく builtin base・追加済み user def も含む）。
        // list 側が authoritative で user def は shadow しない。
        if models.iter().any(|m| m.id == definition.id) {
            errors.push(format!(
                "model {}: user model definition conflicts with an existing model; keeping the existing definition.",
                definition.id
            ));
            continue;
        }
        let defaults = find_model_defaults(
            models,
            &definition.id,
            definition.api.as_deref().or(fallback_api),
        );
        match model_from_json(
            provider_id,
            definition,
            fallback_api,
            fallback_base_url,
            defaults,
        ) {
            Ok(model) => {
                models.push(model);
                merged.push(definition);
            }
            Err(msg) => errors.push(msg),
        }
    }
    merged
}

/// `rawModelHeaders` 相当の per-model source merge（template 未解決）。
/// precedence（後勝ち）: modelOverrides.headers < models[].headers
/// < plugin def headers。compose・refresh の両経路で共有する。
/// `user_defs` は実際に model 化された defs のみ渡す（plugin 経路は
/// `merge_user_model_defs` の戻り値、custom/builtin 経路は def error が
/// fatal のため `config.models` 全件）— id 衝突・解決不能で skip された
/// def の `headers` が同名の残存 model へ混入しないようにするため。
/// `plugin_defs` は effective defs（persisted ?? `plugin.models`、
/// refresh 時は refreshed definitions）を渡す。
pub(crate) fn collect_model_header_sources(
    models: &[CatalogModel],
    overrides: Option<&BTreeMap<String, ModelOverrideJson>>,
    user_defs: &[&ModelDefinitionJson],
    plugin_defs: &[ModelDefinitionJson],
) -> BTreeMap<String, Map<String, Value>> {
    let mut model_header_sources: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for model in models {
        let mut merged = Map::new();
        if let Some(h) = overrides
            .and_then(|o| o.get(&model.id))
            .and_then(|o| o.headers.as_ref())
        {
            for (k, v) in h {
                merged.insert(k.clone(), v.clone());
            }
        }
        if let Some(h) = user_defs
            .iter()
            .find(|d| d.id == model.id)
            .and_then(|d| d.headers.as_ref())
        {
            for (k, v) in h {
                merged.insert(k.clone(), v.clone());
            }
        }
        if let Some(h) = plugin_defs
            .iter()
            .find(|d| d.id == model.id)
            .and_then(|d| d.headers.as_ref())
        {
            for (k, v) in h {
                merged.insert(k.clone(), v.clone());
            }
        }
        if !merged.is_empty() {
            model_header_sources.insert(model.id.clone(), merged);
        }
    }
    model_header_sources
}

/// provider 単位の compose（`recomposeProvider` + `composeModelProvider`）。
/// `base_meta`/`base_models` は builtin descriptor + bundled models、
/// `config` は user models.json entry、`plugin` は plugin registration。
/// compose error は Err(String)（TS `compositionErrors` message 相当）で
/// caller が fallback/drop を決める。
pub(crate) fn compose_provider(
    provider_id: &str,
    base_meta: Option<&BuiltinProviderMeta>,
    base_models: &[CatalogModel],
    config: Option<&ProviderConfigJson>,
    plugin: Option<&PluginProviderRegistration>,
    persisted: Option<&[ModelDefinitionJson]>,
    errors: &mut Vec<String>,
) -> Result<ProviderEntry, String> {
    // models 層:
    //   custom/builtin: applyModelsJson（検証 + overlay + per-def resolve、
    //     def error は fatal）→ modelOverrides。
    //   plugin: 検証 + overlay（custom と同一 semantics）→ applyExtension
    //     （persisted ?? plugin.models）→ merge_user_model_defs（user
    //     models[] の additive merge — def error は per-def skip + errors
    //     記録で provider は残す）→ modelOverrides。
    // `extension_base_models` は `apply_extension` が受け取った
    // pre-extension list（post-overlay builtin base、plugin-only では空）
    // — `refresh_plugin_models` が同一 defaults pool で `extension_models`
    // を再実行できるよう entry へ保持する。
    // `effective_user_defs` は `collect_model_header_sources` の user_def
    // headers source — model 化に成功した defs のみ（custom/builtin は
    // def error が fatal のため `config.models` 全件、plugin は merge
    // 成功 defs のみ。衝突 skip された def の headers は残存 model に
    // 混入させない）。
    let mut extension_base_models: Vec<CatalogModel> = Vec::new();
    let (mut models, effective_user_defs) = match plugin {
        Some(p) => {
            let mut models = match config {
                Some(c) => {
                    // provider-level 検証（oauth ⇒ baseUrl・「must
                    // specify …」）は custom と同一 semantics のまま
                    // 維持する（緩めない・fatal class を増やさない）。
                    // per-def `config.models` 解決は merge stage へ defer。
                    validate_provider_config(provider_id, c)?;
                    apply_provider_overlay(base_models, c)
                }
                None => base_models.to_vec(),
            };
            extension_base_models = models.clone();
            models = apply_extension(provider_id, models, p, persisted)?;
            let merged = merge_user_model_defs(
                provider_id,
                &mut models,
                config.and_then(|c| c.models.as_deref()).unwrap_or(&[]),
                config
                    .and_then(|c| c.api.as_deref())
                    .or(Some(p.api.as_str())),
                config
                    .and_then(|c| c.base_url.as_deref())
                    .or(Some(p.base_url.as_str())),
                errors,
            );
            (models, merged)
        }
        None => {
            // custom/builtin: `apply_models_json` は def 解決失敗を fatal
            // (`?`)にするため全 def が model 化される — header source の
            // 参照先は `config.models` 全件でよい。
            let models = match config {
                Some(c) => apply_models_json(provider_id, base_models, c)?,
                None => base_models.to_vec(),
            };
            let user_defs = config
                .and_then(|c| c.models.as_deref())
                .unwrap_or(&[])
                .iter()
                .collect();
            (models, user_defs)
        }
    };
    // modelOverrides は最終層（custom-model upsert・extension replace・
    // user def merge の後）。
    if let Some(overrides) = config.and_then(|c| c.model_overrides.as_ref()) {
        for model in &mut models {
            if let Some(ov) = overrides.get(&model.id) {
                apply_model_override(model, ov);
            }
        }
    }

    // `rawModelHeaders` 相当の per-model source merge（template 未解決）。
    // plugin_defs は effective defs（persisted ?? plugin.models）を渡す —
    // persisted snapshot 由来の `headers` も request へ届く。
    let model_header_sources = collect_model_header_sources(
        &models,
        config.and_then(|c| c.model_overrides.as_ref()),
        &effective_user_defs,
        plugin
            .map(|p| persisted.unwrap_or(p.models.as_slice()))
            .unwrap_or(&[]),
    );

    let auth = resolve_auth(provider_id, base_meta, config, plugin)?;

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
        api_key_name: auth.api_key_name,
        env_auth: auth.env_auth,
        configured_api_key: auth.configured_api_key,
        auth_header: auth.auth_header,
        configured_headers,
        oauth: auth.oauth,
        plugin_auth: plugin.and_then(|p| p.auth.clone()),
        plugin_catalog: plugin.and_then(|p| p.catalog.clone()),
        plugin_api: plugin.map(|p| p.api.clone()),
        models,
        model_header_sources,
        refresh: RefreshState {
            model_overrides: config.and_then(|c| c.model_overrides.clone()),
            plugin_user_defs: if plugin.is_some() {
                config.and_then(|c| c.models.clone()).unwrap_or_default()
            } else {
                Vec::new()
            },
            extension_base_models,
            config_api: config.and_then(|c| c.api.clone()),
            config_base_url: config.and_then(|c| c.base_url.clone()),
        },
        plugin_bootstrap_defs: plugin.map(|p| p.models.clone()).unwrap_or_default(),
    })
}

/// `resolve_auth` の結果 — `ProviderEntry` の auth 関連 field。
pub(crate) struct ComposedAuth {
    /// `rawKey = configuredApiKey(config, extension)`
    /// = plugin.apiKey ?? config.apiKey。
    configured_api_key: Option<String>,
    api_key_name: Option<String>,
    oauth: Option<OauthDeclaration>,
    env_auth: EnvAuth,
    auth_header: bool,
}

/// `compose_provider` の auth 層 — `composeApiKeyAuth`/`composeOAuthAuth`
/// の method 有無判定。auth 関連 field（`configured_api_key`/
/// `api_key_name`/`oauth`/`env_auth`/`auth_header`）をまとめて解決し、
/// auth method が 1 つも無い provider は compose error にする。
pub(crate) fn resolve_auth(
    provider_id: &str,
    base_meta: Option<&BuiltinProviderMeta>,
    config: Option<&ProviderConfigJson>,
    plugin: Option<&PluginProviderRegistration>,
) -> Result<ComposedAuth, String> {
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
    Ok(ComposedAuth {
        configured_api_key: raw_key,
        api_key_name,
        oauth: oauth_decl,
        env_auth: if radius_gateway {
            EnvAuth::Vars(&["RADIUS_API_KEY"])
        } else {
            base_meta.map(|m| m.env_auth).unwrap_or(EnvAuth::None)
        },
        auth_header: plugin
            .and_then(|p| p.auth_header)
            .or_else(|| config.and_then(|c| c.auth_header))
            .unwrap_or(false),
    })
}
