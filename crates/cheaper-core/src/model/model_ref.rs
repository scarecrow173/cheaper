//! Port of `packages/core/src/model/ref.ts` (file named `model_ref.rs`
//! because `ref` is a Rust reserved word).

/// `modelKey`: the canonical `"<provider>/<id>"` key.
pub fn model_key(provider: &str, id: &str) -> String {
    format!("{provider}/{id}")
}

/// `matchModelPattern`: `"<provider>/<id>"` exact or `"<provider>/*"`
/// wildcard match, shared by exclude / preferences / exceptions.
pub fn match_model_pattern(provider: &str, id: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return provider == prefix;
    }
    pattern == model_key(provider, id)
}

/// `parseModelRef`: split on the first `/` only (a model id may itself
/// contain `/`). `None` for malformed refs.
pub fn parse_model_ref(ref_str: &str) -> Option<ModelRef> {
    let i = ref_str.find('/')?;
    if i == 0 || i == ref_str.len() - 1 {
        return None;
    }
    Some(ModelRef {
        provider: ref_str[..i].to_string(),
        id: ref_str[i + 1..].to_string(),
    })
}

/// Parsed `"<provider>/<id>"` reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRef {
    pub provider: String,
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_key_joins_provider_and_id() {
        assert_eq!(model_key("openai", "gpt-5"), "openai/gpt-5");
    }

    #[test]
    fn pattern_matching() {
        assert!(match_model_pattern("openai", "gpt-5", "openai/*"));
        assert!(!match_model_pattern("openai", "gpt-5", "anthropic/*"));
        assert!(match_model_pattern("openai", "gpt-5", "openai/gpt-5"));
        // `provider/*` does not match a deeper id's exact pattern.
        assert!(!match_model_pattern("openai", "gpt-5", "openai/gpt-6"));
        // A bare `*` (no slash) is an exact pattern, not a wildcard.
        assert!(!match_model_pattern("openai", "gpt-5", "*"));
    }

    #[test]
    fn parse_model_ref_splits_on_first_slash() {
        let r = parse_model_ref("openai/gpt-5").unwrap();
        assert_eq!(r.provider, "openai");
        assert_eq!(r.id, "gpt-5");
        // ids may contain `/`.
        let r = parse_model_ref("p/a/b").unwrap();
        assert_eq!(r.id, "a/b");
        assert!(parse_model_ref("/x").is_none());
        assert!(parse_model_ref("x/").is_none());
        assert!(parse_model_ref("nodelim").is_none());
    }
}
