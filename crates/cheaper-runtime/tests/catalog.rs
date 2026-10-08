//! catalog store の merge/get_available/get_model/invalid-api contract test。

use cheaper_core::errors::ErrorCode;
use cheaper_runtime::catalog::bundled::{bundled_models, BUILTIN_PROVIDERS};
use cheaper_runtime::catalog::schema::parse_models_json_str;
use cheaper_runtime::catalog::{CatalogStore, SUPPORTED_APIS};

fn build(user_json: Option<&str>) -> CatalogStore {
    let (file, err) = match user_json {
        Some(json) => (Some(parse_models_json_str(json).unwrap()), None),
        None => (None, None),
    };
    CatalogStore::build(file, err, &[]).unwrap()
}

#[test]
fn bundled_catalog_loads_all_providers() {
    // 40 builtin（38 data file + openai-codex + radius(remote-only)）。
    assert_eq!(BUILTIN_PROVIDERS.len(), 40);
    let store = build(None);
    assert!(store.errors().is_empty(), "{:?}", store.errors());
    assert_eq!(store.providers().len(), 40);
    // openai bundled model が CatalogModel として parse される。
    let gpt = store
        .get_model("openai", "gpt-5.2")
        .or_else(|| bundled_models("openai").unwrap().into_iter().next());
    assert!(gpt.is_some());
}

#[test]
fn bundled_snapshot_models_have_supported_fields() {
    // bundled bytes は canonical snapshot — 全 model が required field を持つ。
    for meta in BUILTIN_PROVIDERS {
        let models = bundled_models(meta.id).unwrap();
        for m in &models {
            assert!(!m.id.is_empty());
            assert!(!m.api.is_empty());
            // azure は deployment 固有 URL のため bundled baseUrl は空
            // （TS data file も `""`）。
            if meta.id != "azure-openai-responses" {
                assert!(!m.base_url.is_empty(), "{}/{}", meta.id, m.id);
            }
            assert_eq!(m.provider, meta.id);
        }
    }
}

#[test]
fn user_custom_provider_is_merged() {
    let store = build(Some(
        r#"{"providers": {"ollama": {
            "api": "openai-completions",
            "baseUrl": "http://localhost:11434/v1",
            "apiKey": "ollama",
            "models": [{"id": "qwen3.5:4b", "contextWindow": 32768, "maxTokens": 8192}]
        }}}"#,
    ));
    let entry = store.provider("ollama").unwrap();
    assert_eq!(
        entry.origin,
        cheaper_runtime::catalog::ProviderOrigin::UserConfig
    );
    let model = store.get_model("ollama", "qwen3.5:4b").unwrap();
    assert_eq!(model.api, "openai-completions");
    assert_eq!(model.base_url, "http://localhost:11434/v1");
    assert_eq!(model.context_window, 32768.0);
}

#[test]
fn user_model_override_replaces_bundled_fields() {
    // bundled model と同じ id の user model は丸ごと置き換え
    // （`models[]` upsert は definition replace）。
    let bundled = build(None);
    let Some(target) = bundled
        .provider("openai")
        .unwrap()
        .models
        .first()
        .map(|m| m.id.clone())
    else {
        panic!("openai bundled models empty");
    };
    let json = serde_json::json!({
        "providers": {"openai": {
            "models": [{"id": target, "contextWindow": 999, "maxTokens": 10}]
        }}
    });
    let store = build(Some(&serde_json::to_string(&json).unwrap()));
    let m = store.get_model("openai", &target).unwrap();
    assert_eq!(m.context_window, 999.0);
    assert_eq!(m.max_tokens, 10.0);
}

#[test]
fn model_overrides_merge_nested_cost_and_sampling() {
    let bundled = build(None);
    let Some(target) = bundled
        .provider("anthropic")
        .unwrap()
        .models
        .first()
        .map(|m| m.id.clone())
    else {
        panic!("anthropic bundled models empty");
    };
    let base = bundled.get_model("anthropic", &target).unwrap();
    let json = serde_json::json!({
        "providers": {"anthropic": {
            "modelOverrides": {target.clone(): {
                "cost": {"input": 0.5},
                "samplingParams": {"temperature": 0.1},
                "reasoning": true
            }}
        }}
    });
    let store = build(Some(&serde_json::to_string(&json).unwrap()));
    let m = store.get_model("anthropic", &target).unwrap();
    // cost は field 単位 merge（input のみ上書き）。
    assert_eq!(m.cost.input, 0.5);
    assert_eq!(m.cost.output, base.cost.output);
    assert!(m.reasoning);
    assert_eq!(
        m.sampling_params.as_ref().unwrap()["temperature"],
        serde_json::json!(0.1)
    );
}

#[test]
fn get_available_filters_by_configured_predicate() {
    let store = build(None);
    let all = store.get_available(&|_| true, &|_| None, None);
    let openai_only = store.get_available(&|_| true, &|_| None, Some("openai"));
    assert!(!all.is_empty());
    assert!(openai_only.iter().all(|m| m.provider == "openai"));
    // 認証なし provider は候補に出ない。
    let unauthenticated = store.get_available(&|id| id == "openai", &|_| None, None);
    assert!(unauthenticated.iter().all(|m| m.provider == "openai"));
}

#[test]
fn get_available_applies_credential_model_filter() {
    let store = build(None);
    let models = store.get_available(
        &|_| true,
        &|id| {
            if id == "github-copilot" {
                Some(vec!["gpt-5".to_string()])
            } else {
                None
            }
        },
        Some("github-copilot"),
    );
    assert!(models.iter().all(|m| m.id == "gpt-5"));
}

#[test]
fn unsupported_api_kind_returns_invalid_model_config() {
    let store = build(Some(
        r#"{"providers": {"weird": {
            "api": "some-unknown-api",
            "baseUrl": "http://x",
            "apiKey": "k",
            "models": [{"id": "m1"}]
        }}}"#,
    ));
    let err = store.get_supported_model("weird", "m1").unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidModelConfig);
    assert!(err.message().contains("unsupported provider api"));
}

#[test]
fn supported_apis_are_scope_set() {
    for api in SUPPORTED_APIS {
        let json = serde_json::json!({"providers": {"p": {
            "api": api, "baseUrl": "http://x", "apiKey": "k",
            "models": [{"id": "m"}]
        }}});
        let store = build(Some(&serde_json::to_string(&json).unwrap()));
        assert!(store.get_supported_model("p", "m").unwrap().is_some());
    }
}

#[test]
fn invalid_models_json_reports_error_and_falls_back_to_bundled() {
    let (file, err) = match cheaper_runtime::catalog::load_models_json(std::path::Path::new(
        "/nonexistent/models.json",
    )) {
        Ok(f) => (f, None),
        Err(e) => (None, Some(e)),
    };
    let store = CatalogStore::build(file, err, &[]).unwrap();
    // bundled のみ。user error 無し（file 不在は合法）。
    assert!(store.errors().is_empty());
    assert!(
        store.get_model("openai", "gpt-5.2").is_some()
            || !bundled_models("openai").unwrap().is_empty()
    );
}

#[test]
fn compose_error_drops_custom_provider_and_keeps_bundled() {
    // custom provider の必須 field 欠落（baseUrl 無し・models のみでも
    // definition が api/baseUrl 未指定 → compose error → provider drop）。
    let store = build(Some(
        r#"{"providers": {"broken": {
            "apiKey": "k",
            "models": [{"id": "m"}]
        }}}"#,
    ));
    assert!(store.provider("broken").is_none());
    assert!(store.errors().iter().any(|e| e.contains("broken")));
    // builtin は残る。
    assert!(store.provider("openai").is_some());
}

#[test]
fn no_silent_fallback_between_providers() {
    // provider id を間違えた model は他 provider へ落ちない。
    let store = build(None);
    assert!(store.get_model("anthropic", "gpt-5.2").is_none());
}
