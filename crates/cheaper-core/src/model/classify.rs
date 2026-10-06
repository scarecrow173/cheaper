//! Port of `packages/core/src/model/classify.ts`.

use serde::Serialize;

use crate::model::config::ModelConfig;
use crate::model::exceptions::find_builtin_model_exception;
use crate::model::model_ref::model_key;
use crate::model::pricing::{resolve_pricing, ModelCost, ModelPricing, PricedLike};
use crate::model::tier::Tier;

/// `ModelInfo`: core-side minimal model shape (no Pi dependency).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInfo {
    pub provider: String,
    pub id: String,
    pub cost: Option<ModelCost>,
    pub max_tokens: Option<u64>,
}

impl PricedLike for ModelInfo {
    fn provider(&self) -> &str {
        &self.provider
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn cost(&self) -> Option<&ModelCost> {
        self.cost.as_ref()
    }
}

/// `TierSource` — tier classification provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TierSource {
    /// models.yaml `overrides`.
    ExplicitOverride,
    /// Bundled curated exceptions.
    CuratedException,
    /// Workload effectiveCost percentile over the population.
    Auto,
    /// No classification basis; falls back to `medium`.
    Unclassified,
}

impl TierSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitOverride => "explicit-override",
            Self::CuratedException => "curated-exception",
            Self::Auto => "auto",
            Self::Unclassified => "unclassified",
        }
    }
}

/// `DEFAULT_OUTPUT_PER_INPUT` (issue #46): the neutral equal-weight
/// workload (same ordering as total cost).
pub const DEFAULT_OUTPUT_PER_INPUT: f64 = 1.0;

/// `AutoClassificationDetail` — present only when `source == Auto`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AutoClassificationDetail {
    /// Workload-weighted cost in USD/1M-input-tokens units.
    #[serde(rename = "effectiveCost")]
    pub effective_cost: f64,
    /// Midrank percentile inside the population (0.5 when the whole
    /// population ties / has one member).
    #[serde(rename = "effectiveCostPercentile")]
    pub effective_cost_percentile: f64,
    /// The workload profile used for classification.
    #[serde(rename = "outputPerInput")]
    pub output_per_input: f64,
}

/// `Classification`.
#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub tier: Tier,
    pub source: TierSource,
    pub auto: Option<AutoClassificationDetail>,
}

/// `AutoTierContext`: the auto-classification ranking population.
/// `costs` = every population model's effectiveCost, ascending,
/// ties included.
#[derive(Debug, Clone)]
pub struct AutoTierContext {
    pub output_per_input: f64,
    pub costs: Vec<f64>,
}

/// `effectiveCost` (issue #46). Pricing is normalized
/// `usd-per-1m-tokens`.
pub fn effective_cost(pricing: &ModelPricing, output_per_input: f64) -> f64 {
    pricing.input + pricing.output * output_per_input
}

/// `isPricedForRanking`: input=output=0 is not "free" but an opaque
/// billing observation (local model / subscription alias / unknown), so
/// it is excluded from both the ranking population and auto
/// classification (issue #46 / #45 invariant).
fn is_priced_for_ranking(pricing: Option<&ModelPricing>) -> bool {
    pricing.is_some_and(|p| p.input + p.output > 0.0)
}

/// `buildAutoTierContext`: the population includes override/curated
/// models (ranks reflect the full price observation; only the tier
/// assignment skips auto).
pub fn build_auto_tier_context(
    models: &[ModelInfo],
    output_per_input: Option<f64>,
) -> AutoTierContext {
    let output_per_input = output_per_input.unwrap_or(DEFAULT_OUTPUT_PER_INPUT);
    let mut costs: Vec<f64> = models
        .iter()
        .filter_map(|m| resolve_pricing(m, None))
        .filter(|p| is_priced_for_ranking(Some(p)))
        .map(|p| effective_cost(&p, output_per_input))
        .collect();
    costs.sort_by(|a, b| a.total_cmp(b));
    AutoTierContext {
        output_per_input,
        costs,
    }
}

/// `percentileTier` — midrank percentile `p = (countBelow +
/// countEqual/2)/n` with integer boundary comparison (float-error-free):
/// low iff `3(2·countBelow + countEqual) < 2n`, medium iff `< 4n`, else
/// high. Equal values share one `p`/tier (no arbitrary splits).
fn percentile_tier(costs: &[f64], cost: f64) -> Option<(Tier, f64)> {
    let n = costs.len();
    if n == 0 {
        return None;
    }
    let mut count_below: u64 = 0;
    let mut count_equal: u64 = 0;
    for c in costs {
        if *c < cost {
            count_below += 1;
        } else if *c == cost {
            count_equal += 1;
        } else {
            break;
        }
    }
    let percentile = (count_below as f64 + count_equal as f64 / 2.0) / n as f64;
    let score = 3 * (2 * count_below + count_equal);
    let n64 = n as u64;
    let tier = if score < 2 * n64 {
        Tier::Low
    } else if score < 4 * n64 {
        Tier::Medium
    } else {
        Tier::High
    };
    Some((tier, percentile))
}

/// `classifyModel` options.
#[derive(Debug, Default)]
pub struct ClassifyOptions<'a> {
    /// Pre-resolved pricing (`None` → `resolvePricing(model)`).
    pub pricing: Option<ModelPricing>,
    /// Population context from [`build_auto_tier_context`]; without it
    /// auto classification is skipped and the model is unclassified.
    pub auto_context: Option<&'a AutoTierContext>,
}

/// `classifyModel`: first match in `overrides > curated exceptions >
/// auto > unclassified` order wins. Unclassified → `medium` (existing
/// invariant) distinguished by `source`.
pub fn classify_model(
    model: &ModelInfo,
    config: &ModelConfig,
    opts: &ClassifyOptions<'_>,
) -> Classification {
    let key = model_key(&model.provider, &model.id);
    if let Some(tier) = config.override_tier(&key) {
        return Classification {
            tier,
            source: TierSource::ExplicitOverride,
            auto: None,
        };
    }
    if let Some(exception) = find_builtin_model_exception(&model.provider, &model.id) {
        return Classification {
            tier: exception.tier,
            source: TierSource::CuratedException,
            auto: None,
        };
    }
    let pricing = opts
        .pricing
        .clone()
        .or_else(|| resolve_pricing(model, None));
    if let Some(ctx) = opts.auto_context {
        if is_priced_for_ranking(pricing.as_ref()) {
            let cost = effective_cost(pricing.as_ref().unwrap(), ctx.output_per_input);
            if let Some((tier, percentile)) = percentile_tier(&ctx.costs, cost) {
                return Classification {
                    tier,
                    source: TierSource::Auto,
                    auto: Some(AutoClassificationDetail {
                        effective_cost: cost,
                        effective_cost_percentile: percentile,
                        output_per_input: ctx.output_per_input,
                    }),
                };
            }
        }
    }
    Classification {
        tier: Tier::Medium,
        source: TierSource::Unclassified,
        auto: None,
    }
}

/// `classifyModels`: batch classification over the whole input
/// population with one shared `autoContext`.
///
/// The TS version returns `Map<ModelInfo, Classification>` keyed by
/// object identity; the port returns an index-aligned `Vec` (element i
/// is the classification of `models[i]`).
pub fn classify_models(
    models: &[ModelInfo],
    config: &ModelConfig,
    output_per_input: Option<f64>,
) -> Vec<Classification> {
    let auto_context = build_auto_tier_context(models, output_per_input);
    models
        .iter()
        .map(|m| {
            classify_model(
                m,
                config,
                &ClassifyOptions {
                    pricing: resolve_pricing(m, None),
                    auto_context: Some(&auto_context),
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(provider: &str, id: &str, input: f64, output: f64) -> ModelInfo {
        ModelInfo {
            provider: provider.to_string(),
            id: id.to_string(),
            cost: Some(ModelCost {
                input,
                output,
                cache_read: None,
                cache_write: None,
            }),
            max_tokens: None,
        }
    }

    fn empty_config() -> ModelConfig {
        ModelConfig::default()
    }

    #[test]
    fn effective_cost_weights_output() {
        let p = ModelPricing {
            input: 1.0,
            output: 2.0,
            cache_read: None,
            cache_write: None,
            unit: "usd-per-1m-tokens".into(),
            currency: "USD".into(),
            source: crate::model::pricing::PricingSource::RuntimeCatalog,
            snapshot_version: None,
            updated_at: None,
        };
        assert_eq!(effective_cost(&p, 1.0), 3.0);
        assert_eq!(effective_cost(&p, 0.5), 2.0);
    }

    #[test]
    fn zero_price_is_excluded_from_ranking_and_auto() {
        let zero = model("z", "free", 0.0, 0.0);
        let priced = model("p", "m", 1.0, 1.0);
        let ctx = build_auto_tier_context(&[zero, priced], None);
        assert_eq!(ctx.costs.len(), 1);
    }

    #[test]
    fn percentile_boundaries_use_integer_math() {
        // 3 models: cheapest → low boundary `3(0+1)=3 < 6`; mid → `3(2+1)=9 !< 6, < 12` medium.
        let costs = vec![1.0, 5.0, 9.0];
        assert_eq!(percentile_tier(&costs, 1.0).unwrap().0, Tier::Low);
        assert_eq!(percentile_tier(&costs, 5.0).unwrap().0, Tier::Medium);
        assert_eq!(percentile_tier(&costs, 9.0).unwrap().0, Tier::High);
        // Ties share one tier; all-equal → medium at p=0.5.
        let tied = vec![2.0, 2.0];
        let (t, p) = percentile_tier(&tied, 2.0).unwrap();
        assert_eq!(t, Tier::Medium);
        assert_eq!(p, 0.5);
        let single = vec![7.0];
        assert_eq!(percentile_tier(&single, 7.0).unwrap().0, Tier::Medium);
        assert!(percentile_tier(&[], 1.0).is_none());
    }

    #[test]
    fn classification_priority() {
        let mut config = empty_config();
        config.overrides.push(("p/m".to_string(), Tier::High));
        // override wins over everything.
        let c = classify_model(
            &model("p", "m", 0.0, 0.0),
            &config,
            &ClassifyOptions::default(),
        );
        assert_eq!(c.tier, Tier::High);
        assert_eq!(c.source, TierSource::ExplicitOverride);
        // curated exception beats auto/unclassified.
        let c = classify_model(
            &model("ollama", "x", 5.0, 5.0),
            &config,
            &ClassifyOptions::default(),
        );
        assert_eq!(c.tier, Tier::Low);
        assert_eq!(c.source, TierSource::CuratedException);
        // no context → unclassified medium.
        let c = classify_model(
            &model("p", "other", 5.0, 5.0),
            &config,
            &ClassifyOptions::default(),
        );
        assert_eq!(c.tier, Tier::Medium);
        assert_eq!(c.source, TierSource::Unclassified);
    }

    #[test]
    fn classify_models_population_relative() {
        let models = vec![
            model("p", "cheap", 0.1, 0.1),
            model("p", "mid", 5.0, 5.0),
            model("p", "dear", 50.0, 50.0),
        ];
        let config = empty_config();
        let out = classify_models(&models, &config, None);
        assert_eq!(out[0].tier, Tier::Low);
        assert_eq!(out[0].source, TierSource::Auto);
        assert_eq!(out[1].tier, Tier::Medium);
        assert_eq!(out[2].tier, Tier::High);
        assert_eq!(out[0].auto.as_ref().unwrap().output_per_input, 1.0);
        // Workload weighting shifts effectiveCost.
        let out = classify_models(&models, &config, Some(10.0));
        assert_eq!(out[0].auto.as_ref().unwrap().output_per_input, 10.0);
    }
}
