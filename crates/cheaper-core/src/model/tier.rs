//! Port of `packages/core/src/model/tier.ts`.

use serde::{Deserialize, Serialize};

/// `TIERS` in declaration order (low < medium < high).
pub const TIERS: [Tier; 3] = [Tier::Low, Tier::Medium, Tier::High];

/// `Tier`. Serialized as the lowercase string form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tier {
    Low,
    Medium,
    High,
}

impl Tier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    fn index(self) -> usize {
        TIERS
            .iter()
            .position(|t| *t == self)
            .expect("tier in TIERS")
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `compareTier`: negative/zero/positive like `TIERS.indexOf` difference.
pub fn compare_tier(a: Tier, b: Tier) -> i64 {
    a.index() as i64 - b.index() as i64
}

/// `isTier` (TS type guard on an untyped value).
pub fn is_tier(v: &serde_json::Value) -> bool {
    v.as_str()
        .is_some_and(|s| s == "low" || s == "medium" || s == "high")
}

/// Parse a tier string; `None` for non-tier values.
pub fn parse_tier(v: &serde_json::Value) -> Option<Tier> {
    match v.as_str() {
        Some("low") => Some(Tier::Low),
        Some("medium") => Some(Tier::Medium),
        Some("high") => Some(Tier::High),
        _ => None,
    }
}

/// `nextTier`: the tier one step up, `None` above `high`.
pub fn next_tier(t: Tier) -> Option<Tier> {
    match t {
        Tier::Low => Some(Tier::Medium),
        Tier::Medium => Some(Tier::High),
        Tier::High => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn ordering_is_low_medium_high() {
        assert!(compare_tier(Tier::Low, Tier::Medium) < 0);
        assert_eq!(compare_tier(Tier::High, Tier::High), 0);
        assert!(compare_tier(Tier::High, Tier::Low) > 0);
    }

    #[test]
    fn is_tier_recognizes_only_the_three_names() {
        assert!(is_tier(&json!("low")));
        assert!(is_tier(&json!("medium")));
        assert!(is_tier(&json!("high")));
        assert!(!is_tier(&json!("LOW")));
        assert!(!is_tier(&json!(1)));
        assert!(!is_tier(&Value::Null));
    }

    #[test]
    fn next_tier_stops_at_high() {
        assert_eq!(next_tier(Tier::Low), Some(Tier::Medium));
        assert_eq!(next_tier(Tier::Medium), Some(Tier::High));
        assert_eq!(next_tier(Tier::High), None);
    }
}
