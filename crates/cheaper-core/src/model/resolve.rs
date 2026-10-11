//! Port of `packages/core/src/model/resolve.ts`.

use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::model::classify::{
    build_auto_tier_context, classify_model, classify_models, AutoClassificationDetail,
    AutoTierContext, Classification, ClassifyOptions, ModelInfo, TierSource,
};
use crate::model::config::ModelConfig;
use crate::model::model_ref::{match_model_pattern, model_key, ModelRef};
use crate::model::pricing::{resolve_pricing, ModelPricing};
use crate::model::tier::{compare_tier, next_tier, Tier, TIERS};

/// `ResolvedModel`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModel {
    pub model: ModelInfo,
    /// The tier actually selected (may differ from the request on
    /// fallback).
    pub tier: Tier,
    /// Tier provenance.
    pub tier_source: TierSource,
    /// Effective pricing (`None` when unpriced).
    pub pricing: Option<ModelPricing>,
    /// Auto-classification detail (`tier_source == Auto` only).
    pub auto: Option<AutoClassificationDetail>,
    /// `"tier"` via the tier resolver, `"explicit"` via `--model`.
    pub selection: ModelSelection,
}

/// `selection` tag (`"tier"` | `"explicit"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelSelection {
    Tier,
    Explicit,
}

impl ModelSelection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tier => "tier",
            Self::Explicit => "explicit",
        }
    }
}

/// `policy` slice of `resolveModel` / `resolveExplicitModel`.
#[derive(Debug, Clone, Copy)]
pub struct ModelPolicySlice {
    pub max_tier: Tier,
    pub tier_fallback: crate::policy::schema::TierFallback,
}

fn total_cost(m: &ModelInfo) -> f64 {
    m.cost
        .as_ref()
        .map(|c| c.input + c.output)
        .unwrap_or(f64::INFINITY)
}

fn by_cost_then_provider_then_id(a: &ModelInfo, b: &ModelInfo) -> std::cmp::Ordering {
    // TS: `totalCost(a) - totalCost(b) || a.provider.localeCompare(b.provider)
    // || a.id.localeCompare(b.id)`. `locale_compare` reproduces Node's
    // case-insensitive-primary collation (byte order would sort uppercase
    // providers like "OpenAI" ahead of lowercase ones like "anthropic").
    total_cost(a)
        .partial_cmp(&total_cost(b))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| crate::js::locale_compare(&a.provider, &b.provider))
        .then_with(|| crate::js::locale_compare(&a.id, &b.id))
}

/// `isExcludedModel` (`config.exclude` exact + `provider/*` wildcard).
pub fn is_excluded_model(m: &ModelInfo, config: &ModelConfig) -> bool {
    config
        .exclude
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .any(|pattern| match_model_pattern(&m.provider, &m.id, pattern))
}

/// Order: `preferences` pattern order → rest by `cost.input +
/// cost.output` ascending → provider → id (missing cost = `Infinity` at
/// the tail). Within one pattern the same total order applies.
fn order_candidates(candidates: &[ModelInfo], patterns: &[String]) -> Vec<ModelInfo> {
    let mut remaining: Vec<ModelInfo> = candidates.to_vec();
    let mut picked: Vec<ModelInfo> = Vec::new();
    for pattern in patterns {
        let mut hit: Vec<ModelInfo> = remaining
            .iter()
            .filter(|m| match_model_pattern(&m.provider, &m.id, pattern))
            .cloned()
            .collect();
        hit.sort_by(by_cost_then_provider_then_id);
        for m in &hit {
            if let Some(i) = remaining.iter().position(|r| r == m) {
                remaining.remove(i);
            }
        }
        picked.extend(hit);
    }
    remaining.sort_by(by_cost_then_provider_then_id);
    picked.extend(remaining);
    picked
}

/// `resolveModel` options.
pub struct ResolveModelOptions<'a> {
    pub available: &'a [ModelInfo],
    pub requested_tier: Tier,
    pub policy: ModelPolicySlice,
    pub config: &'a ModelConfig,
    /// Worker workload profile (issue #46); `None` =
    /// `DEFAULT_OUTPUT_PER_INPUT`.
    pub output_per_input: Option<f64>,
}

struct TierEntry {
    model: ModelInfo,
    classification: Classification,
    pricing: Option<ModelPricing>,
}

/// `resolveModel`. Candidates = `available ∖ exclude` classified as the
/// requested tier. `requested > maxTier` → `POLICY_VIOLATION`. Empty tier
/// with `tierFallback: "higher"` climbs one tier at a time up to
/// `maxTier`; exhaustion → `MODEL_TIER_UNAVAILABLE` with
/// `details.candidatesPerTier`.
pub fn resolve_model(opts: &ResolveModelOptions<'_>) -> Result<ResolvedModel, CheaperError> {
    let eligible: Vec<ModelInfo> = opts
        .available
        .iter()
        .filter(|m| !is_excluded_model(m, opts.config))
        .cloned()
        .collect();
    if eligible.is_empty() {
        let mut details = Map::new();
        details.insert("noAuthenticatedModels".to_string(), json!(true));
        return Err(CheaperError::new(
            ErrorCode::ModelTierUnavailable,
            "no authenticated models available",
        )
        .with_details(details));
    }
    if compare_tier(opts.requested_tier, opts.policy.max_tier) > 0 {
        let mut details = Map::new();
        details.insert(
            "requestedTier".to_string(),
            json!(opts.requested_tier.as_str()),
        );
        details.insert("maxTier".to_string(), json!(opts.policy.max_tier.as_str()));
        return Err(CheaperError::new(
            ErrorCode::PolicyViolation,
            format!(
                "requested tier {} exceeds policy maxTier {}",
                opts.requested_tier.as_str(),
                opts.policy.max_tier.as_str()
            ),
        )
        .with_details(details));
    }

    let classifications = classify_models(&eligible, opts.config, opts.output_per_input);
    let mut by_tier: std::collections::HashMap<Tier, Vec<TierEntry>> =
        std::collections::HashMap::new();
    for (m, classification) in eligible.iter().cloned().zip(classifications) {
        let pricing = resolve_pricing(&m, None);
        by_tier
            .entry(classification.tier)
            .or_default()
            .push(TierEntry {
                model: m,
                classification,
                pricing,
            });
    }

    let mut tier = Some(opts.requested_tier);
    while let Some(t) = tier {
        if compare_tier(t, opts.policy.max_tier) > 0 {
            break;
        }
        let entries = by_tier.get(&t);
        let empty: &[TierEntry] = &[];
        let entries = entries.map(Vec::as_slice).unwrap_or(empty);
        let ordered = order_candidates(
            &entries.iter().map(|e| e.model.clone()).collect::<Vec<_>>(),
            opts.config.preferences.for_tier(t).unwrap_or(&[]),
        );
        if let Some(model) = ordered.first() {
            let entry = entries
                .iter()
                .find(|e| &e.model == model)
                .expect("ordered model from entries");
            let resolved = ResolvedModel {
                model: model.clone(),
                tier: t,
                tier_source: entry.classification.source,
                pricing: entry.pricing.clone(),
                auto: entry.classification.auto.clone(),
                selection: ModelSelection::Tier,
            };
            // `cheaper-core` event(design §instrumentation 配置)—
            // resolved model の分類値のみ(cost 値・config 内容は載せない)。
            tracing::debug!(
                model.provider = resolved.model.provider.as_str(),
                model.id = resolved.model.id.as_str(),
                model.tier = resolved.tier.as_str(),
                tier.source = resolved.tier_source.as_str(),
                selection = resolved.selection.as_str(),
                "model resolved"
            );
            return Ok(resolved);
        }
        if opts.policy.tier_fallback != crate::policy::schema::TierFallback::Higher {
            break;
        }
        tier = next_tier(t);
    }

    let mut candidates_per_tier = Map::new();
    for t in TIERS {
        candidates_per_tier.insert(
            t.as_str().to_string(),
            json!(by_tier.get(&t).map(Vec::len).unwrap_or(0)),
        );
    }
    let mut details = Map::new();
    details.insert(
        "requestedTier".to_string(),
        json!(opts.requested_tier.as_str()),
    );
    details.insert(
        "candidatesPerTier".to_string(),
        Value::Object(candidates_per_tier),
    );
    Err(CheaperError::new(
        ErrorCode::ModelTierUnavailable,
        format!(
            "no model available for tier {}",
            opts.requested_tier.as_str()
        ),
    )
    .with_details(details))
}

/// `resolveExplicitModel` options.
pub struct ResolveExplicitModelOptions<'a> {
    pub available: &'a [ModelInfo],
    pub model_ref: &'a ModelRef,
    pub policy_max_tier: Tier,
    pub config: &'a ModelConfig,
    /// Worker workload profile (used to classify the policy-check tier).
    pub output_per_input: Option<f64>,
}

/// `resolveExplicitModel` (`--model <provider>/<id>`): bypasses tier
/// candidate selection but not exclude / availability / maxTier (issue
/// #45).
pub fn resolve_explicit_model(
    opts: &ResolveExplicitModelOptions<'_>,
) -> Result<ResolvedModel, CheaperError> {
    let key = model_key(&opts.model_ref.provider, &opts.model_ref.id);
    let target = ModelInfo {
        provider: opts.model_ref.provider.clone(),
        id: opts.model_ref.id.clone(),
        cost: None,
        max_tokens: None,
    };
    if is_excluded_model(&target, opts.config) {
        let mut details = Map::new();
        details.insert("reason".to_string(), json!("model-excluded"));
        details.insert("model".to_string(), json!(key));
        return Err(CheaperError::new(
            ErrorCode::PolicyViolation,
            format!("model {key} is excluded by models config"),
        )
        .with_details(details));
    }
    let Some(model) = opts
        .available
        .iter()
        .find(|m| m.provider == opts.model_ref.provider && m.id == opts.model_ref.id)
    else {
        let mut details = Map::new();
        details.insert("reason".to_string(), json!("model-unavailable"));
        details.insert("model".to_string(), json!(key));
        return Err(CheaperError::new(
            ErrorCode::ModelTierUnavailable,
            format!("model {key} is not available"),
        )
        .with_details(details));
    };
    let eligible: Vec<ModelInfo> = opts
        .available
        .iter()
        .filter(|m| !is_excluded_model(m, opts.config))
        .cloned()
        .collect();
    let auto_context: AutoTierContext = build_auto_tier_context(&eligible, opts.output_per_input);
    let pricing = resolve_pricing(model, None);
    let classification = classify_model(
        model,
        opts.config,
        &ClassifyOptions {
            pricing: pricing.clone(),
            auto_context: Some(&auto_context),
        },
    );
    if compare_tier(classification.tier, opts.policy_max_tier) > 0 {
        let mut details = Map::new();
        details.insert("reason".to_string(), json!("model-exceeds-max-tier"));
        details.insert("model".to_string(), json!(key));
        details.insert("tier".to_string(), json!(classification.tier.as_str()));
        details.insert("maxTier".to_string(), json!(opts.policy_max_tier.as_str()));
        return Err(CheaperError::new(
            ErrorCode::PolicyViolation,
            format!(
                "model {key} tier {} exceeds policy maxTier {}",
                classification.tier.as_str(),
                opts.policy_max_tier.as_str()
            ),
        )
        .with_details(details));
    }
    let resolved = ResolvedModel {
        model: model.clone(),
        tier: classification.tier,
        tier_source: classification.source,
        pricing,
        auto: classification.auto,
        selection: ModelSelection::Explicit,
    };
    tracing::debug!(
        model.provider = resolved.model.provider.as_str(),
        model.id = resolved.model.id.as_str(),
        model.tier = resolved.tier.as_str(),
        tier.source = resolved.tier_source.as_str(),
        selection = resolved.selection.as_str(),
        "model resolved"
    );
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::pricing::ModelCost;
    use crate::policy::schema::TierFallback;

    fn model(provider: &str, id: &str, input: f64, output: f64) -> ModelInfo {
        ModelInfo {
            provider: provider.into(),
            id: id.into(),
            cost: Some(ModelCost {
                input,
                output,
                cache_read: None,
                cache_write: None,
            }),
            max_tokens: None,
        }
    }

    fn no_cost(provider: &str, id: &str) -> ModelInfo {
        ModelInfo {
            provider: provider.into(),
            id: id.into(),
            cost: None,
            max_tokens: None,
        }
    }

    fn policy(max_tier: Tier, fallback: TierFallback) -> ModelPolicySlice {
        ModelPolicySlice {
            max_tier,
            tier_fallback: fallback,
        }
    }

    #[test]
    fn no_authenticated_models() {
        let config = ModelConfig::default();
        let e = resolve_model(&ResolveModelOptions {
            available: &[],
            requested_tier: Tier::Low,
            policy: policy(Tier::High, TierFallback::None),
            config: &config,
            output_per_input: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::ModelTierUnavailable);
        assert_eq!(e.details().unwrap()["noAuthenticatedModels"], json!(true));
    }

    #[test]
    fn requested_over_max_tier_is_policy_violation() {
        let config = ModelConfig::default();
        let available = vec![model("p", "m", 1.0, 1.0)];
        let e = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::High,
            policy: policy(Tier::Low, TierFallback::None),
            config: &config,
            output_per_input: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
    }

    #[test]
    fn cheapest_of_tier_wins() {
        // population: 3 models → percentiles low/medium/high.
        let config = ModelConfig::default();
        let available = vec![
            model("p", "dear", 50.0, 50.0),
            model("p", "cheap2", 0.2, 0.2),
            model("p", "cheap1", 0.1, 0.1),
        ];
        let r = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::Low,
            policy: policy(Tier::High, TierFallback::None),
            config: &config,
            output_per_input: None,
        })
        .unwrap();
        // "cheap1" and "cheap2" classify low together? No — percentile
        // boundaries: costs sorted [0.4, 0.8, 100] wait effective cost =
        // input+output: 0.2,0.4,100 → 3 members → low/medium/high.
        assert_eq!(r.model.id, "cheap1");
        assert_eq!(r.tier, Tier::Low);
        assert_eq!(r.tier_source, TierSource::Auto);
        assert_eq!(r.selection, ModelSelection::Tier);
    }

    #[test]
    fn preferences_order_wins_over_cost() {
        let config = ModelConfig {
            overrides: vec![("p/a".into(), Tier::Low), ("p/b".into(), Tier::Low)],
            preferences: crate::model::config::TierPreferences {
                low: Some(vec!["p/b".into()]),
                ..Default::default()
            },
            ..Default::default()
        };
        let available = vec![model("p", "a", 0.1, 0.1), model("p", "b", 0.5, 0.5)];
        let r = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::Low,
            policy: policy(Tier::High, TierFallback::None),
            config: &config,
            output_per_input: None,
        })
        .unwrap();
        assert_eq!(r.model.id, "b");
        assert_eq!(r.tier_source, TierSource::ExplicitOverride);
    }

    #[test]
    fn exclude_filters_candidates() {
        let config = ModelConfig {
            exclude: Some(vec!["p/*".into()]),
            ..Default::default()
        };
        let available = vec![model("p", "a", 1.0, 1.0), model("q", "b", 1.0, 1.0)];
        assert!(is_excluded_model(&available[0], &config));
        assert!(!is_excluded_model(&available[1], &config));
        let r = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::Medium,
            policy: policy(Tier::High, TierFallback::None),
            config: &config,
            output_per_input: None,
        })
        .unwrap();
        assert_eq!(r.model.id, "b");
    }

    #[test]
    fn tier_fallback_climbs_one_at_a_time() {
        // Only a high-priced model; request low with fallback higher.
        let config = ModelConfig::default();
        let available = vec![model("p", "dear", 50.0, 50.0)];
        let r = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::Low,
            policy: policy(Tier::High, TierFallback::Higher),
            config: &config,
            output_per_input: None,
        })
        .unwrap();
        assert_eq!(r.tier, Tier::Medium); // single-model population → medium
        let e = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::Low,
            policy: policy(Tier::Low, TierFallback::Higher),
            config: &config,
            output_per_input: None,
        })
        .unwrap_err();
        // maxTier=low stops the climb → unavailable with candidatesPerTier.
        assert_eq!(e.code(), ErrorCode::ModelTierUnavailable);
        assert_eq!(
            e.details().unwrap()["candidatesPerTier"],
            json!({"low": 0, "medium": 1, "high": 0})
        );
    }

    #[test]
    fn explicit_model_bypasses_tier_choice_but_not_policy() {
        let config = ModelConfig {
            exclude: Some(vec!["p/blocked".into()]),
            ..Default::default()
        };
        let available = vec![model("p", "m", 1.0, 1.0)];
        // excluded → POLICY_VIOLATION
        let e = resolve_explicit_model(&ResolveExplicitModelOptions {
            available: &available,
            model_ref: &ModelRef {
                provider: "p".into(),
                id: "blocked".into(),
            },
            policy_max_tier: Tier::High,
            config: &config,
            output_per_input: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        // unavailable → MODEL_TIER_UNAVAILABLE
        let e = resolve_explicit_model(&ResolveExplicitModelOptions {
            available: &available,
            model_ref: &ModelRef {
                provider: "p".into(),
                id: "ghost".into(),
            },
            policy_max_tier: Tier::High,
            config: &config,
            output_per_input: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::ModelTierUnavailable);
        // selected → explicit
        let r = resolve_explicit_model(&ResolveExplicitModelOptions {
            available: &available,
            model_ref: &ModelRef {
                provider: "p".into(),
                id: "m".into(),
            },
            policy_max_tier: Tier::High,
            config: &config,
            output_per_input: None,
        })
        .unwrap();
        assert_eq!(r.selection, ModelSelection::Explicit);
        assert_eq!(r.tier, Tier::Medium); // unclassified → medium (pop=1)
    }

    #[test]
    fn explicit_over_max_tier_is_violation() {
        let config = ModelConfig {
            overrides: vec![("p/m".into(), Tier::High)],
            ..Default::default()
        };
        let available = vec![model("p", "m", 1.0, 1.0)];
        let e = resolve_explicit_model(&ResolveExplicitModelOptions {
            available: &available,
            model_ref: &ModelRef {
                provider: "p".into(),
                id: "m".into(),
            },
            policy_max_tier: Tier::Low,
            config: &config,
            output_per_input: None,
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("model-exceeds-max-tier")
        );
    }

    #[test]
    fn provider_tiebreak_uses_locale_compare() {
        // localeCompare: "anthropic" < "OpenAI" (base letters before
        // case); byte order would pick "OpenAI" first.
        let config = ModelConfig {
            overrides: vec![
                ("OpenAI/m".into(), Tier::Low),
                ("anthropic/m".into(), Tier::Low),
            ],
            ..Default::default()
        };
        let available = vec![
            model("OpenAI", "m", 1.0, 1.0),
            model("anthropic", "m", 1.0, 1.0),
        ];
        let r = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::Low,
            policy: policy(Tier::High, TierFallback::None),
            config: &config,
            output_per_input: None,
        })
        .unwrap();
        assert_eq!(r.model.provider, "anthropic");
    }

    #[test]
    fn unpriced_models_sort_last() {
        let config = ModelConfig {
            overrides: vec![("p/priced".into(), Tier::Low), ("p/free".into(), Tier::Low)],
            ..Default::default()
        };
        let available = vec![no_cost("p", "free"), model("p", "priced", 1.0, 1.0)];
        let r = resolve_model(&ResolveModelOptions {
            available: &available,
            requested_tier: Tier::Low,
            policy: policy(Tier::High, TierFallback::None),
            config: &config,
            output_per_input: None,
        })
        .unwrap();
        assert_eq!(r.model.id, "priced");
    }
}
