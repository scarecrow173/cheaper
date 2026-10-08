//! Port of `packages/core/src/model/pricing.ts`.
//!
//! Normalized pricing metadata contract shared by cost/savings
//! estimation (#44) and the auto tier classifier (#46). Unit/currency are
//! fixed to USD / 1M tokens.

use serde::{Deserialize, Serialize};

use crate::model::data::pricing_snapshot::PRICING_SNAPSHOT;
use crate::model::model_ref::match_model_pattern;

/// `PricingSource`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PricingSource {
    RuntimeCatalog,
    BundledSnapshot,
}

impl PricingSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeCatalog => "runtime-catalog",
            Self::BundledSnapshot => "bundled-snapshot",
        }
    }
}

/// `ModelPricing` — normalized pricing (USD per 1M tokens).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// USD / 1M input tokens.
    pub input: f64,
    /// USD / 1M output tokens.
    pub output: f64,
    #[serde(rename = "cacheRead", skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(rename = "cacheWrite", skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    /// Always `"usd-per-1m-tokens"`.
    pub unit: String,
    /// Always `"USD"`.
    pub currency: String,
    pub source: PricingSource,
    /// Snapshot version when sourced from the bundled snapshot.
    #[serde(rename = "snapshotVersion", skip_serializing_if = "Option::is_none")]
    pub snapshot_version: Option<String>,
    /// Source-side observation/update date (ISO date).
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// `PricingSnapshotEntry`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingSnapshotEntry {
    pub input: f64,
    pub output: f64,
    #[serde(rename = "cacheRead", skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    #[serde(rename = "cacheWrite", skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// `PricingSnapshot`. `models` preserves key insertion order — the
/// `Object.keys` iteration order that `resolvePricing`'s pattern scan
/// relies on (serde_json `preserve_order` / ordered Vec port).
#[derive(Debug, Clone, PartialEq)]
pub struct PricingSnapshot {
    pub version: &'static str,
    pub updated_at: &'static str,
    /// `("<provider>/<id>" | "<provider>/*", entry)` in insertion order.
    pub models: Vec<(String, PricingSnapshotEntry)>,
}

/// Input shape for `resolvePricing` (TS `PricedLike`).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}

/// The provider/id/cost view `resolvePricing` needs.
pub trait PricedLike {
    fn provider(&self) -> &str;
    fn id(&self) -> &str;
    fn cost(&self) -> Option<&ModelCost>;
}

/// `resolvePricing`: runtime catalog `cost` wins when present (a zero
/// price is a valid "free" observation); the bundled snapshot fills in
/// only for missing cost. `None` = unpriced (not an error).
pub fn resolve_pricing<M: PricedLike>(
    model: &M,
    snapshot: Option<&PricingSnapshot>,
) -> Option<ModelPricing> {
    if let Some(cost) = model.cost() {
        return Some(ModelPricing {
            input: cost.input,
            output: cost.output,
            cache_read: cost.cache_read,
            cache_write: cost.cache_write,
            unit: "usd-per-1m-tokens".to_string(),
            currency: "USD".to_string(),
            source: PricingSource::RuntimeCatalog,
            snapshot_version: None,
            updated_at: None,
        });
    }
    let snapshot = snapshot.unwrap_or(&PRICING_SNAPSHOT);
    let entry = snapshot
        .models
        .iter()
        .find(|(pattern, _)| match_model_pattern(model.provider(), model.id(), pattern))
        .map(|(_, e)| e)?;
    Some(ModelPricing {
        input: entry.input,
        output: entry.output,
        cache_read: entry.cache_read,
        cache_write: entry.cache_write,
        unit: "usd-per-1m-tokens".to_string(),
        currency: "USD".to_string(),
        source: PricingSource::BundledSnapshot,
        snapshot_version: Some(snapshot.version.to_string()),
        updated_at: Some(
            entry
                .updated_at
                .clone()
                .unwrap_or_else(|| snapshot.updated_at.to_string()),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::classify::ModelInfo;

    #[test]
    fn runtime_catalog_wins_and_keeps_zero_price() {
        let model = ModelInfo {
            provider: "p".to_string(),
            id: "m".to_string(),
            cost: Some(ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: None,
                cache_write: None,
            }),
            max_tokens: None,
        };
        let snapshot = PricingSnapshot {
            version: "v1",
            updated_at: "2026-01-01",
            models: vec![(
                "p/*".to_string(),
                PricingSnapshotEntry {
                    input: 1.0,
                    output: 2.0,
                    cache_read: None,
                    cache_write: None,
                    updated_at: None,
                },
            )],
        };
        let pricing = resolve_pricing(&model, Some(&snapshot)).unwrap();
        assert_eq!(pricing.source, PricingSource::RuntimeCatalog);
        assert_eq!(pricing.input, 0.0);
        assert!(pricing.snapshot_version.is_none());
    }

    #[test]
    fn snapshot_fills_missing_cost_in_insertion_order() {
        let model = ModelInfo {
            provider: "p".to_string(),
            id: "m".to_string(),
            cost: None,
            max_tokens: None,
        };
        let snapshot = PricingSnapshot {
            version: "v2",
            updated_at: "2026-01-02",
            models: vec![
                (
                    "p/*".to_string(),
                    PricingSnapshotEntry {
                        input: 1.0,
                        output: 2.0,
                        cache_read: None,
                        cache_write: None,
                        updated_at: Some("2025-12-31".to_string()),
                    },
                ),
                (
                    "p/m".to_string(),
                    PricingSnapshotEntry {
                        input: 9.0,
                        output: 9.0,
                        cache_read: None,
                        cache_write: None,
                        updated_at: None,
                    },
                ),
            ],
        };
        // Insertion order wins: "p/*" precedes the more specific "p/m".
        let pricing = resolve_pricing(&model, Some(&snapshot)).unwrap();
        assert_eq!(pricing.source, PricingSource::BundledSnapshot);
        assert_eq!(pricing.input, 1.0);
        assert_eq!(pricing.snapshot_version.as_deref(), Some("v2"));
        assert_eq!(pricing.updated_at.as_deref(), Some("2025-12-31"));
    }

    #[test]
    fn unpriced_model_is_none() {
        let model = ModelInfo {
            provider: "nope".to_string(),
            id: "x".to_string(),
            cost: None,
            max_tokens: None,
        };
        assert!(resolve_pricing(&model, None).is_none());
        let snapshot = PricingSnapshot {
            version: "v",
            updated_at: "d",
            models: vec![],
        };
        assert!(resolve_pricing(&model, Some(&snapshot)).is_none());
    }
}
