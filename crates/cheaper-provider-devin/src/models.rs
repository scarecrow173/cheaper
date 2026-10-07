//! Devin model catalog — `devin models list --format json` の取得と
//! catalog model 定義への変換（TS `providers/devin/models.ts` の
//! port — vendored from pi-devin, Copyright (c) 2026 Kashyab Ambarani,
//! MIT License。`familyToModels`/`FALLBACK_MODELS` は
//! mizorewww/pi-devin の thinkingLevelMap 修正（未対応 level の明示
//! null・family id を Pi-facing id に）を反映済み）。
//!
//! 出力型は runtime の `ModelDefinitionJson`（TS `ProviderModelConfig`
//! 相当）。

use std::sync::LazyLock;

use regex::Regex;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_runtime::catalog::schema::{
    ModelCostJson, ModelDefinitionJson, ModelInput, ThinkingLevelMap,
};

use crate::cli::DevinCli;

/// `devin models list` の timeout（TS 20_000ms）。
const MODELS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

// ---- catalog schema（`devin models list --format json` の出力） ----

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct DevinVariant {
    pub model_uid: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub max_context_tokens: Option<f64>,
    #[serde(default)]
    pub max_output_tokens: Option<f64>,
    #[serde(default)]
    pub cost_tier: Option<String>,
    #[serde(default)]
    pub cost_summary: Option<String>,
    #[serde(default)]
    pub is_new: Option<bool>,
    #[serde(default)]
    pub is_beta: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct DevinFamily {
    // TS 側は `family_label`/`family_uid`/`slug` を `||` fallback で
    // 読む（欠損は undefined → falsy → fallback）— required にすると
    // 1 family の欠損で catalog 全体が deserialize error になるため
    // optional とし、`family_to_models` で TS 相当の fallback を掛ける。
    #[serde(default)]
    pub family_label: Option<String>,
    #[serde(default)]
    pub family_uid: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub aliases: Option<Vec<String>>,
    pub variants: Vec<DevinVariant>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct DevinCatalog {
    pub families: Vec<DevinFamily>,
}

/// TS `THINKING_ORDER`。
pub const THINKING_ORDER: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

/// `preferredDefault` の探索順（TS `["high","medium","max","xhigh",
/// "low","minimal","off"]`）。
const PREFERRED_ORDER: [&str; 7] = ["high", "medium", "max", "xhigh", "low", "minimal", "off"];

/// `parseCost` — `"$5.00 / MTok In"` / `"$25.00 / MTok Out"`。
/// cacheRead = input*0.1、cacheWrite = input*1.25（小数 4 位丸め）。
fn parse_cost(summary: Option<&str>) -> ModelCostJson {
    let empty = |input: f64, output: f64| ModelCostJson {
        input,
        output,
        cache_read: (input * 0.1 * 10_000.0).round() / 10_000.0,
        cache_write: (input * 1.25 * 10_000.0).round() / 10_000.0,
        tiers: None,
    };
    let Some(summary) = summary else {
        return empty(0.0, 0.0);
    };
    static COST_IN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\$([0-9.]+)\s*/\s*MTok In").expect("cost in regex"));
    static COST_OUT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\$([0-9.]+)\s*/\s*MTok Out").expect("cost out regex"));
    let in_cost = COST_IN
        .captures(summary)
        .and_then(|c| c[1].parse::<f64>().ok())
        .unwrap_or(0.0);
    let out_cost = COST_OUT
        .captures(summary)
        .and_then(|c| c[1].parse::<f64>().ok())
        .unwrap_or(0.0);
    empty(in_cost, out_cost)
}

/// `variantKey` — model uid の末尾 suffix（`{uid}-{suffix}` or uid 全体）。
/// `priority`/`fast`/`thinking-1m` は thinking map から除外する variant
/// 種別を含む。
fn variant_key(uid: &str) -> Option<&'static str> {
    const SUFFIXES: [&str; 20] = [
        "none-priority",
        "low-priority",
        "medium-priority",
        "high-priority",
        "xhigh-priority",
        "max-priority",
        "low-fast",
        "medium-fast",
        "high-fast",
        "xhigh-fast",
        "max-fast",
        "thinking-1m",
        "thinking",
        "none",
        "low",
        "medium",
        "high",
        "xhigh",
        "max",
        "minimal",
    ];
    SUFFIXES
        .iter()
        .copied()
        .find(|suffix| uid == *suffix || uid.ends_with(&format!("-{suffix}")))
}

/// `thinkingFromSuffix` — suffix → thinking level。`None` は
/// 「map に登録しない」。
fn thinking_from_suffix(suffix: Option<&str>) -> Option<&'static str> {
    let Some(suffix) = suffix else {
        return Some("high");
    };
    match () {
        _ if suffix == "none" || suffix == "none-priority" => Some("off"),
        _ if suffix == "minimal" => Some("minimal"),
        _ if suffix.starts_with("low") => Some("low"),
        _ if suffix.starts_with("medium") => Some("medium"),
        _ if suffix.starts_with("high") && !suffix.starts_with("xhigh") => Some("high"),
        _ if suffix.starts_with("xhigh") => Some("xhigh"),
        _ if suffix.starts_with("max") => Some("max"),
        _ if suffix.contains("thinking") => Some("high"),
        _ => None,
    }
}

/// `ThinkingLevelMap` の level accessor（`Some(None)` = 明示 null、
/// `Some(Some(uid))` = mapped、`None` = key 不在）。
fn level_get<'a>(map: &'a ThinkingLevelMap, level: &str) -> Option<&'a Option<String>> {
    match level {
        "off" => map.off.as_ref(),
        "minimal" => map.minimal.as_ref(),
        "low" => map.low.as_ref(),
        "medium" => map.medium.as_ref(),
        "high" => map.high.as_ref(),
        "xhigh" => map.xhigh.as_ref(),
        "max" => map.max.as_ref(),
        _ => None,
    }
}

fn level_set(map: &mut ThinkingLevelMap, level: &str, value: Option<String>) {
    match level {
        "off" => map.off = Some(value),
        "minimal" => map.minimal = Some(value),
        "low" => map.low = Some(value),
        "medium" => map.medium = Some(value),
        "high" => map.high = Some(value),
        "xhigh" => map.xhigh = Some(value),
        "max" => map.max = Some(value),
        _ => {}
    }
}

/// `preferredDefault` — `high→medium→max→xhigh→low→minimal→off` の順で
/// mapped な uid を返す。
fn preferred_default(map: &ThinkingLevelMap) -> Option<String> {
    PREFERRED_ORDER
        .iter()
        .find_map(|level| level_get(map, level).and_then(|v| v.clone()))
}

/// `familyToModels` — 1 family → 1 `ModelDefinitionJson`。
/// priority/fast/thinking-1m variant は除外し、全除外なら全 variant を
/// 対象に戻す（TS `source = usable.length > 0 ? usable : family.variants`）。
fn family_to_models(family: &DevinFamily) -> Vec<ModelDefinitionJson> {
    let usable: Vec<&DevinVariant> = family
        .variants
        .iter()
        .filter(|variant| match variant_key(&variant.model_uid) {
            None => true,
            Some(key) => !key.contains("priority") && !key.contains("fast") && key != "thinking-1m",
        })
        .collect();
    let source: Vec<&DevinVariant> = if usable.is_empty() {
        family.variants.iter().collect()
    } else {
        usable
    };

    let mut thinking_level_map = ThinkingLevelMap::default();
    for variant in &source {
        // TS: `if (level && thinkingLevelMap[level] === undefined)` —
        // key 不在のみ登録。`Option<Option<String>>` の `None` が key 不在。
        if let Some(level) = thinking_from_suffix(variant_key(&variant.model_uid)) {
            if level_get(&thinking_level_map, level).is_none() {
                level_set(
                    &mut thinking_level_map,
                    level,
                    Some(variant.model_uid.clone()),
                );
            }
        }
    }
    // family が出荷しない level は明示 null（picker の不存在 variant
    // への default fallback を防ぐ — mizorewww/pi-devin の同修正）。
    for level in THINKING_ORDER {
        if level_get(&thinking_level_map, level).is_none() {
            level_set(&mut thinking_level_map, level, None);
        }
    }

    let default_uid = preferred_default(&thinking_level_map)
        .or_else(|| source.first().map(|v| v.model_uid.clone()))
        // TS `?? family.family_uid` — 欠損時は undefined 相当（空文字）。
        .or_else(|| family.family_uid.clone())
        .unwrap_or_default();
    let sample = source
        .iter()
        .find(|v| v.model_uid == default_uid)
        .or_else(|| source.first());
    let Some(sample) = sample else {
        return Vec::new();
    };

    let mapped_levels = THINKING_ORDER
        .iter()
        .filter(|level| {
            level_get(&thinking_level_map, level)
                .and_then(|v| v.as_ref())
                .is_some()
        })
        .count();
    let reasoning = mapped_levels > 1;

    // thinking map がある限り Pi-facing id は wire に出ない
    // （resolveModelUid が常に variant UID へ写像する）ため、reasoning
    // family では family id を Pi-facing id にする。
    // TS `family.slug || family.family_uid || defaultUid` — `||` は
    // 欠損（None）と空文字の両方を skip する。
    let non_empty = |value: &Option<String>| value.clone().filter(|v| !v.is_empty());
    let family_id = non_empty(&family.slug)
        .or_else(|| non_empty(&family.family_uid))
        .unwrap_or_else(|| default_uid.clone());

    vec![ModelDefinitionJson {
        id: if reasoning {
            family_id
        } else {
            default_uid.clone()
        },
        // TS `family.family_label || family.slug || defaultUid`。
        name: Some(
            non_empty(&family.family_label)
                .or_else(|| non_empty(&family.slug))
                .unwrap_or_else(|| default_uid.clone()),
        ),
        api: None,
        base_url: None,
        reasoning: Some(reasoning),
        thinking_level_map: reasoning.then_some(thinking_level_map),
        input: Some(vec![ModelInput::Text, ModelInput::Image]),
        cost: Some(parse_cost(sample.cost_summary.as_deref())),
        context_window: Some(sample.max_context_tokens.unwrap_or(256_000.0)),
        max_tokens: Some(sample.max_output_tokens.unwrap_or(128_000.0)),
        sampling_params: None,
        headers: None,
        compat: None,
    }]
}

/// bootstrap 用の最小 model list — `devin` provider を auth 前に
/// catalog へ認識させるためだけに使い、live catalog
/// （`devin models list`）を必ず優先する。
pub fn fallback_models() -> Vec<ModelDefinitionJson> {
    fn level_map(entries: &[(&str, Option<&str>)]) -> ThinkingLevelMap {
        let mut map = ThinkingLevelMap::default();
        for (level, uid) in entries {
            level_set(&mut map, level, uid.map(str::to_string));
        }
        map
    }
    /// `{input, output, cache_read, cache_write}` — fallback 価格表。
    struct FallbackCost {
        input: f64,
        output: f64,
        cache_read: f64,
        cache_write: f64,
    }
    fn model(
        id: &str,
        name: &str,
        levels: &[(&str, Option<&str>)],
        cost: FallbackCost,
        context: f64,
    ) -> ModelDefinitionJson {
        ModelDefinitionJson {
            id: id.to_string(),
            name: Some(name.to_string()),
            api: None,
            base_url: None,
            reasoning: Some(true),
            thinking_level_map: Some(level_map(levels)),
            input: Some(vec![ModelInput::Text, ModelInput::Image]),
            cost: Some(ModelCostJson {
                input: cost.input,
                output: cost.output,
                cache_read: cost.cache_read,
                cache_write: cost.cache_write,
                tiers: None,
            }),
            context_window: Some(context),
            max_tokens: Some(128_000.0),
            sampling_params: None,
            headers: None,
            compat: None,
        }
    }
    vec![
        model(
            "claude-opus-5",
            "Claude Opus 5",
            &[
                ("off", None),
                ("minimal", None),
                ("low", Some("claude-opus-5-low")),
                ("medium", Some("claude-opus-5-medium")),
                ("high", Some("claude-opus-5-high")),
                ("xhigh", Some("claude-opus-5-xhigh")),
                ("max", Some("claude-opus-5-max")),
            ],
            FallbackCost {
                input: 5.0,
                output: 25.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            1_000_000.0,
        ),
        model(
            "claude-fable-5",
            "Claude Fable 5",
            &[
                ("off", None),
                ("minimal", None),
                ("low", Some("claude-5-fable-low")),
                ("medium", Some("claude-5-fable-medium")),
                ("high", Some("claude-5-fable-high")),
                ("xhigh", Some("claude-5-fable-xhigh")),
                ("max", Some("claude-5-fable-max")),
            ],
            FallbackCost {
                input: 10.0,
                output: 50.0,
                cache_read: 1.0,
                cache_write: 12.5,
            },
            1_000_000.0,
        ),
        model(
            "gpt-5.6-sol",
            "GPT-5.6 Sol",
            &[
                ("off", Some("gpt-5-6-sol-none")),
                ("minimal", None),
                ("low", Some("gpt-5-6-sol-low")),
                ("medium", Some("gpt-5-6-sol-medium")),
                ("high", Some("gpt-5-6-sol-high")),
                ("xhigh", Some("gpt-5-6-sol-xhigh")),
                ("max", Some("gpt-5-6-sol-max")),
            ],
            FallbackCost {
                input: 5.0,
                output: 30.0,
                cache_read: 0.5,
                cache_write: 6.25,
            },
            1_000_000.0,
        ),
        model(
            "swe-1-7",
            "SWE-1.7",
            &[
                ("off", None),
                ("minimal", None),
                ("low", None),
                ("medium", Some("swe-1-7-medium")),
                ("high", Some("swe-1-7")),
                ("xhigh", None),
                ("max", None),
            ],
            FallbackCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            262_000.0,
        ),
    ]
}

/// `modelsFromCatalog` — live catalog → models。空・欠損時は
/// `FALLBACK_MODELS`。
pub fn models_from_catalog(catalog: Option<&DevinCatalog>) -> Vec<ModelDefinitionJson> {
    let Some(catalog) = catalog else {
        return fallback_models();
    };
    if catalog.families.is_empty() {
        return fallback_models();
    }
    let models: Vec<ModelDefinitionJson> =
        catalog.families.iter().flat_map(family_to_models).collect();
    if models.is_empty() {
        return fallback_models();
    }
    models
}

/// `loadCliCatalog` — `devin models list --format json` を isolated
/// env の child process で実行し JSON parse。exit != 0 は stderr 込み
/// error、malformed JSON / families 欠落は明示 error。
pub fn load_cli_catalog(cli: &DevinCli) -> Result<DevinCatalog, CheaperError> {
    let result = cli
        .run(
            &["models", "list", "--format", "json"],
            Some(MODELS_TIMEOUT),
            false,
        )
        .map_err(|e| CheaperError::new(ErrorCode::ProviderFailure, e.to_string()).with_cause(e))?;
    if result.code != 0 {
        let stderr = result.stderr.trim();
        return Err(CheaperError::new(
            ErrorCode::ProviderFailure,
            if stderr.is_empty() {
                format!("devin models list exited {}", result.code)
            } else {
                stderr.to_string()
            },
        ));
    }
    let catalog: DevinCatalog = serde_json::from_str(&result.stdout).map_err(|e| {
        CheaperError::new(
            ErrorCode::ProviderFailure,
            format!("devin models list returned malformed JSON: {e}"),
        )
    })?;
    // `catalog.families` は型で必須 — TS の `!Array.isArray(catalog.families)`
    // は `families` 欠落 JSON が deserialize で reject されることで同等に
    // カバーされる。
    Ok(catalog)
}

/// `resolveModelUid` — 要求 reasoning level → variant uid、なければ
/// preferred default → model id。
pub fn resolve_model_uid(
    model_id: &str,
    thinking_level_map: Option<&ThinkingLevelMap>,
    reasoning: Option<&str>,
) -> String {
    if let (Some(reasoning), Some(map)) = (reasoning, thinking_level_map) {
        if let Some(Some(uid)) = level_get(map, reasoning).cloned() {
            return uid;
        }
    }
    if let Some(map) = thinking_level_map {
        if let Some(uid) = preferred_default(map) {
            return uid;
        }
    }
    model_id.to_string()
}
