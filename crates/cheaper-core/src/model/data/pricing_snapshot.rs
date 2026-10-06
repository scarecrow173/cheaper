//! Port of `packages/core/src/model/data/pricing-snapshot.ts`.
//!
//! Bundled supplementary pricing snapshot (ADR-034rzI0Yqw3L8AFM9diZct).
//! Only models missing runtime-catalog pricing are filled in here; it is
//! not an exhaustive catalog. `version` is a hand-edited release marker.
//! Snapshot updates follow the same manual-edit process as TS.

use crate::model::pricing::PricingSnapshot;

/// `PRICING_SNAPSHOT` (`models` is currently empty).
pub static PRICING_SNAPSHOT: PricingSnapshot = PricingSnapshot {
    version: "2026-09-19",
    updated_at: "2026-09-19",
    models: Vec::new(),
};
