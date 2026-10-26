//! 解決済み catalog identity から History 用データだけを投影する。

use serde::Serialize;

use super::CatalogModel;

/// Execution History に投影できる catalog 情報のみを保持する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogHistoryIdentity {
    provider_id: String,
    model_id: String,
}

impl CatalogHistoryIdentity {
    /// 投影済み provider ID。
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// 投影済み model ID。
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

impl CatalogModel {
    /// resolved model の provider ID と model ID だけを History 用に投影する。
    /// 無効な ID は省略し、catalog 解決には影響させない。
    pub fn history_identity(&self) -> Option<CatalogHistoryIdentity> {
        Some(CatalogHistoryIdentity {
            provider_id: validate_catalog_history_id(&self.provider)?,
            model_id: validate_catalog_history_id(&self.id)?,
        })
    }
}

fn validate_catalog_history_id(value: &str) -> Option<String> {
    let valid = (1..=128).contains(&value.len())
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
        && !["://", "?", "#", "@"]
            .iter()
            .any(|marker| value.contains(marker));

    valid.then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::super::{schema::parse_models_json_str, CatalogStore};

    fn build_store(provider_id: &str, model_id: &str) -> CatalogStore {
        let models_json = serde_json::json!({
            "providers": {
                (provider_id): {
                    "api": "openai-completions",
                    "baseUrl": "https://private.example/v1?key=secret",
                    "apiKey": "secret",
                    "models": [{
                        "id": model_id,
                        "name": "Private display label",
                        "contextWindow": 32768,
                        "maxTokens": 8192
                    }]
                }
            }
        });
        let parsed = parse_models_json_str(&models_json.to_string()).unwrap();
        CatalogStore::build(Some(parsed), None, &[], &BTreeMap::new()).unwrap()
    }

    #[test]
    fn history_identity_preserves_official_catalog_ids_without_endpoint_data() {
        let store = build_store("safe-provider", "amazon.nova-2-lite-v1:0");
        let model = store
            .get_model("safe-provider", "amazon.nova-2-lite-v1:0")
            .expect("valid catalog entry resolves");

        let identity = model.history_identity().expect("official IDs project");
        assert_eq!(identity.provider_id(), "safe-provider");
        assert_eq!(identity.model_id(), "amazon.nova-2-lite-v1:0");
        let serialized = serde_json::to_string(&identity).unwrap();
        assert!(!serialized.contains("private.example"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("Private display label"));
    }

    #[test]
    fn unsafe_catalog_identity_is_omitted_without_rejecting_resolution() {
        let unsafe_provider = "provider://secret?key=x";
        let store = build_store(unsafe_provider, "model-v1");
        let model = store
            .get_model(unsafe_provider, "model-v1")
            .expect("History projection must not reject a catalog entry");
        assert!(model.history_identity().is_none());

        let store = build_store("safe-provider", "model#secret");
        let model = store
            .get_model("safe-provider", "model#secret")
            .expect("History projection must not change model resolution");
        assert!(model.history_identity().is_none());
    }
}
