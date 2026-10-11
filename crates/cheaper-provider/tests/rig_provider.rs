use std::collections::BTreeMap;

use cheaper_runtime::auth::{AuthSource, ProviderEnv, ResolvedAuth};
use rig::completion::CompletionRequest;
use rig::http_client::{self, StatusCode};
use rig::message::{Message, Text, UserContent};
use rig::wire::{Mode, Secret, Wire};

use cheaper_provider::devin::rig_provider::*;
use cheaper_provider::devin::wire::iter_fields;

/// `ProviderEnv` overlay — scoped env の注入 seam を試す最小実装。
struct MapEnv(BTreeMap<String, String>);

impl ProviderEnv for MapEnv {
    fn env(&self, name: &str) -> Option<String> {
        self.0.get(name).filter(|v| !v.trim().is_empty()).cloned()
    }
    fn file_exists(&self, _path: &str) -> bool {
        false
    }
    fn home_dir(&self) -> Option<std::path::PathBuf> {
        None
    }
}

fn env(pairs: &[(&str, &str)]) -> MapEnv {
    MapEnv(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

fn auth(base_url: Option<&str>) -> ResolvedAuth {
    ResolvedAuth {
        api_key: Some("k".to_string()),
        headers: BTreeMap::new(),
        base_url: base_url.map(str::to_string),
        source: AuthSource::OAuth,
    }
}

#[test]
fn resolve_host_prefers_env_then_auth_then_default() {
    // env > auth.base_url > 既定 host。
    let e = env(&[(HOST_ENV, "https://env.example/")]);
    let a = auth(Some("https://cred.example"));
    assert_eq!(resolve_host(&e, Some(&a)), "https://env.example");
    assert_eq!(resolve_host(&env(&[]), Some(&a)), "https://cred.example");
    assert_eq!(resolve_host(&env(&[]), None), DEFAULT_HOST);
    // 空白のみの env は unset 扱い（TS `||` の falsy と同じ）。
    assert_eq!(
        resolve_host(&env(&[(HOST_ENV, "  ")]), Some(&a)),
        "https://cred.example"
    );
}

#[test]
fn resolve_host_strips_one_trailing_slash() {
    // TS `.replace(/\/$/,"")` は 1 個だけ除去する。
    let e = env(&[(HOST_ENV, "https://env.example//")]);
    assert_eq!(resolve_host(&e, None), "https://env.example/");
}

fn completion_request() -> CompletionRequest {
    CompletionRequest {
        model: None,
        chat_history: vec![Message::User {
            content: vec![UserContent::Text(Text::new("hi"))],
        }],
        documents: vec![],
        tools: vec![],
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
        output_schema: None,
        record_telemetry_content: false,
    }
}

#[test]
fn encode_uses_off_thinking_level_uid() {
    // TS: session が `thinkingLevel: "off"` に pin し、
    // `resolveModelUid(model.id, map, "off")` が走る。
    // `gpt-5.6-sol` は off → `gpt-5-6-sol-none` を持つ fallback model。
    let model = cheaper_provider::devin::models::fallback_models()
        .into_iter()
        .find(|m| m.id == "gpt-5.6-sol")
        .unwrap();
    let wire = DevinWire {
        api_key: Secret::from("k"),
        host: "h".to_string(),
        model_id: model.id.clone(),
        thinking_level_map: model.thinking_level_map,
    };
    let payload = wire.encode(completion_request(), Mode::Streaming).unwrap();
    // field 21 = model uid — preferred default（high）ではなく
    // off variant が出ること。
    let uid = iter_fields(&payload.tail)
        .flatten()
        .find(|f| f.num == 21)
        .and_then(|f| f.string());
    assert_eq!(uid.as_deref(), Some("gpt-5-6-sol-none"));
}

#[test]
fn chat_status_message_matches_ts_shape() {
    // TS `GetChatMessage HTTP {status}: {body.slice(0, 300)}`。
    assert_eq!(
        chat_status_message(StatusCode::INTERNAL_SERVER_ERROR, "boom"),
        "GetChatMessage HTTP 500: boom"
    );
    let long = "x".repeat(400);
    let msg = chat_status_message(StatusCode::BAD_GATEWAY, &long);
    assert_eq!(msg, format!("GetChatMessage HTTP 502: {}", "x".repeat(300)));
}

#[test]
fn chat_send_error_preserves_status_and_ts_message() {
    let error = http_client::Error::non_success_with_details(
        StatusCode::TOO_MANY_REQUESTS,
        http::HeaderMap::new(),
        "slow down".to_string(),
    );
    let err = chat_send_error(error);
    // status を保持する → adapter の transient 分類（429 retry）が効く。
    assert_eq!(
        err.provider_response_status(),
        Some(StatusCode::TOO_MANY_REQUESTS)
    );
    assert!(err
        .to_string()
        .contains("GetChatMessage HTTP 429: slow down"));
}
