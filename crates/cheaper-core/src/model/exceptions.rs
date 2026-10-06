//! Port of `packages/core/src/model/exceptions.ts`.
//!
//! Bundled curated exceptions pin a tier (with reason/provenance) for
//! models the pricing metadata cannot handle — not an exhaustive tier
//! list (issue #45 / ADR-034rzI0Yqw3L8AFM9diZct).

use serde::Serialize;

use crate::model::model_ref::match_model_pattern;
use crate::model::tier::Tier;

/// `ModelException`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelException {
    pub tier: Tier,
    /// Why the pricing/auto classification cannot handle this model.
    pub reason: &'static str,
    /// Information source (issue / doc / external URL).
    pub source: &'static str,
    /// ISO date the entry was last checked.
    pub checked_at: &'static str,
}

/// `MODEL_EXCEPTIONS`: `"<provider>/<id>"` exact or `"<provider>/*"`
/// wildcard keys, in declaration order.
///
/// Local models: `price = 0` does not mean an API price, so they are not
/// left to auto classification but pinned to `low` (issue #45 "do not
/// make low just because price=0" — the exception side).
pub static MODEL_EXCEPTIONS: &[(&str, ModelException)] = &[(
    "ollama/*",
    ModelException {
        tier: Tier::Low,
        reason: "local Ollama model; zero price は API 価格を意味しないため curated で low に固定",
        source: "https://github.com/scarecrow173/cheaper/issues/45",
        checked_at: "2026-09-19",
    },
)];

/// `findModelException`: exact key wins over wildcards; wildcard
/// candidates are scanned in sorted key order and the first match is
/// returned (deterministic).
pub fn find_model_exception<'e>(
    provider: &str,
    id: &str,
    exceptions: &'e [(&'static str, ModelException)],
) -> Option<&'e ModelException> {
    let exact_key = format!("{provider}/{id}");
    if let Some((_, e)) = exceptions.iter().find(|(k, _)| *k == exact_key) {
        return Some(e);
    }
    // `Object.keys(...).sort()` — default JS sort = UTF-16 code-unit
    // order (not `localeCompare`).
    let mut keys: Vec<&'static str> = exceptions.iter().map(|(k, _)| *k).collect();
    keys.sort_by(|a, b| crate::js::code_unit_compare(a, b));
    for key in keys {
        if key.ends_with("/*") && match_model_pattern(provider, id, key) {
            return exceptions.iter().find(|(k, _)| *k == key).map(|(_, e)| e);
        }
    }
    None
}

/// `findModelException` over the bundled table.
pub fn find_builtin_model_exception(provider: &str, id: &str) -> Option<&'static ModelException> {
    find_model_exception(provider, id, MODEL_EXCEPTIONS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_table_pins_ollama_low() {
        let e = find_builtin_model_exception("ollama", "qwen3.5:9b").unwrap();
        assert_eq!(e.tier, Tier::Low);
        assert_eq!(e.checked_at, "2026-09-19");
        assert!(find_builtin_model_exception("openai", "gpt-5").is_none());
    }

    #[test]
    fn exact_beats_wildcard_and_wildcards_are_sorted() {
        let exceptions: &[(&'static str, ModelException)] = &[
            ("z/*", exc(Tier::High)),
            ("a/*", exc(Tier::Medium)),
            ("p/m", exc(Tier::Low)),
        ];
        // Exact wins.
        assert_eq!(
            find_model_exception("p", "m", exceptions).unwrap().tier,
            Tier::Low
        );
        // Two wildcards could match "x/*" only one exists; sorted order is
        // used when several wildcards match.
        let exceptions2: &[(&'static str, ModelException)] = &[
            ("zz/*", exc(Tier::High)),
            ("aa/*", exc(Tier::Medium)),
            ("p/*", exc(Tier::Low)),
        ];
        assert_eq!(
            find_model_exception("p", "anything", exceptions2)
                .unwrap()
                .tier,
            Tier::Low
        );
    }

    fn exc(tier: Tier) -> ModelException {
        ModelException {
            tier,
            reason: "r",
            source: "s",
            checked_at: "2026-01-01",
        }
    }
}
