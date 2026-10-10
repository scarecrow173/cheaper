//! github-copilot OAuth flow + device-code poll の contract test
//! （mock HTTP / scripted interaction で全 step を駆動。実 network・
//! 実 sleep は使わない）。

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_runtime::auth::oauth::{
    poll_device_code_flow, AuthEvent, DeviceFlowPollOptions, DevicePoll, FlowSleeper,
    GitHubCopilotFlow, HttpClient, HttpMethod, HttpRequest, HttpResponse,
};
use cheaper_runtime::auth::service::{AuthInteraction, AuthPrompt, OAuthFlow};
use cheaper_runtime::auth::store::{AuthType, CredentialData, CredentialStore, StoredCredential};
use cheaper_runtime::auth::ProviderEnv;
use cheaper_runtime::plugin::ProviderPluginContext;
use cheaper_runtime::Runtime;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// mocks
// ---------------------------------------------------------------------------

/// queue された response を順に返し、全 request を記録する mock client。
struct MockHttp {
    responses: Mutex<VecDeque<Result<HttpResponse, CheaperError>>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl MockHttp {
    fn new(responses: Vec<HttpResponse>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into_iter().map(Ok).collect()),
            requests: Mutex::new(Vec::new()),
        })
    }
    fn requests(&self) -> Vec<HttpRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl HttpClient for MockHttp {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, CheaperError> {
        self.requests.lock().unwrap().push(request.clone());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| {
                Err(CheaperError::new(
                    ErrorCode::InternalError,
                    "mock http: response queue exhausted",
                ))
            })
    }
}

fn json_response(status: u16, body: Value) -> HttpResponse {
    HttpResponse {
        status,
        status_text: match status {
            200 => "OK".to_string(),
            400 => "Bad Request".to_string(),
            429 => "Too Many Requests".to_string(),
            _ => "Error".to_string(),
        },
        headers: Vec::new(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}

/// sleep せず、要求時間だけ記録する sleeper。
struct RecordedSleeper {
    sleeps: Mutex<Vec<u64>>,
}
impl FlowSleeper for RecordedSleeper {
    fn sleep_ms(&self, ms: u64) {
        self.sleeps.lock().unwrap().push(ms);
    }
}
fn recording_sleeper() -> Arc<RecordedSleeper> {
    Arc::new(RecordedSleeper {
        sleeps: Mutex::new(Vec::new()),
    })
}

/// prompt 応答と event/cancel を制御する scripted interaction。
struct ScriptedInteraction {
    answers: VecDeque<Result<String, CheaperError>>,
    events: Vec<AuthEvent>,
    notify_messages: Vec<String>,
    cancelled: bool,
}

impl ScriptedInteraction {
    fn answering(answers: &[&str]) -> Self {
        Self {
            answers: answers
                .iter()
                .map(|a| Ok((*a).to_string()))
                .collect::<VecDeque<_>>(),
            events: Vec::new(),
            notify_messages: Vec::new(),
            cancelled: false,
        }
    }
}

impl AuthInteraction for ScriptedInteraction {
    fn prompt(&mut self, _prompt: &AuthPrompt<'_>) -> Result<String, CheaperError> {
        self.answers.pop_front().unwrap_or_else(|| {
            Err(CheaperError::new(
                ErrorCode::InternalError,
                "scripted interaction: answer queue exhausted",
            ))
        })
    }
    fn notify(&mut self, message: &str) -> Result<(), CheaperError> {
        self.notify_messages.push(message.to_string());
        Ok(())
    }
    fn event(&mut self, event: &AuthEvent) -> Result<(), CheaperError> {
        self.events.push(event.clone());
        Ok(())
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

struct MapEnv(BTreeMap<String, String>);
impl ProviderEnv for MapEnv {
    fn env(&self, name: &str) -> Option<String> {
        self.0.get(name).filter(|v| !v.trim().is_empty()).cloned()
    }
    fn file_exists(&self, path: &str) -> bool {
        PathBuf::from(path).exists()
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
}

fn tmp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn make_flow(http: Arc<dyn HttpClient>, sleeper: Arc<dyn FlowSleeper>) -> GitHubCopilotFlow {
    GitHubCopilotFlow::with_client(http, sleeper)
}

fn flow_ctx() -> ProviderPluginContext {
    ProviderPluginContext {
        user_config_dir: PathBuf::from("/nonexistent"),
        env: Arc::new(MapEnv(BTreeMap::new())),
    }
}

fn oauth_credential(access: &str, refresh: &str, extra: &[(&str, Value)]) -> StoredCredential {
    let mut extra_map = serde_json::Map::new();
    for (k, v) in extra {
        extra_map.insert((*k).to_string(), v.clone());
    }
    StoredCredential::oauth(CredentialData::Oauth {
        access: access.to_string(),
        refresh: refresh.to_string(),
        expires: i64::MAX,
        account_id: None,
        subscription: None,
        available_model_ids: None,
        extra: extra_map,
    })
}

fn device_code_body() -> Value {
    json!({
        "device_code": "dc-123",
        "user_code": "ABCD-1234",
        "verification_uri": "https://github.com/login/device",
        "interval": 1,
        "expires_in": 900,
    })
}

fn copilot_token_body() -> Value {
    // expires_at は十分未来（~2100）にして `get_auth` の refresh window
    // （5 分）に入らないようにする。
    json!({ "token": "tid=x;proxy-ep=proxy.individual.githubcopilot.com;sku=1", "expires_at": 4_102_444_800.0 })
}

fn models_body() -> Value {
    json!({ "data": [
        {
            "id": "gpt-5-mini",
            "model_picker_enabled": true,
            "capabilities": { "supports": { "tool_calls": true } },
            "policy": { "state": "enabled" },
        },
        {
            "id": "mai-code-1.1-flash",
            "model_picker_enabled": true,
            "capabilities": { "supports": { "tool_calls": true } },
            "policy": { "state": "unconfigured" },
        },
        {
            "id": "no-tools-model",
            "model_picker_enabled": true,
            "capabilities": { "supports": { "tool_calls": false } },
            "policy": { "state": "enabled" },
        },
    ]})
}

fn form_params(request: &HttpRequest) -> BTreeMap<String, String> {
    let body = String::from_utf8(request.body.clone().unwrap()).unwrap();
    body.split('&')
        .map(|pair| {
            let mut it = pair.splitn(2, '=');
            (
                it.next().unwrap().to_string(),
                it.next().unwrap_or("").to_string(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// login flow
// ---------------------------------------------------------------------------

#[test]
fn login_device_flow_persists_credential_and_enables_policy_models() {
    let http = MockHttp::new(vec![
        json_response(200, device_code_body()),
        // poll: pending → complete
        json_response(200, json!({"error": "authorization_pending"})),
        json_response(200, json!({"access_token": "gho_access"})),
        json_response(200, copilot_token_body()),
        json_response(200, models_body()),
        // policy enable for mai-code-1.1-flash (unconfigured + known model)
        json_response(200, json!({})),
    ]);
    let sleeper = recording_sleeper();
    let flow = make_flow(http.clone(), sleeper.clone());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let credential = flow.login(&mut interaction, &flow_ctx()).unwrap();

    // prompt → device_code event → progress event。
    assert_eq!(
        interaction.events,
        vec![
            AuthEvent::DeviceCode {
                user_code: "ABCD-1234".to_string(),
                verification_uri: "https://github.com/login/device".to_string(),
                interval_seconds: Some(1),
                expires_in_seconds: Some(900),
            },
            AuthEvent::Progress {
                message: "Enabling models...".to_string(),
            },
        ]
    );

    // request 形状: device code POST → poll POST ×2 → copilot token GET →
    // models GET → policy POST。
    let requests = http.requests();
    assert_eq!(requests.len(), 6);
    assert_eq!(requests[0].method, HttpMethod::Post);
    assert_eq!(requests[0].url, "https://github.com/login/device/code");
    let device_params = form_params(&requests[0]);
    assert_eq!(device_params["client_id"], "Iv1.b507a08c87ecfe98");
    assert_eq!(device_params["scope"], "read%3Auser");
    assert_eq!(
        requests[0]
            .headers
            .iter()
            .find(|(k, _)| k == "Accept")
            .map(|(_, v)| v.as_str()),
        Some("application/json")
    );
    for poll_req in [&requests[1], &requests[2]] {
        assert_eq!(poll_req.url, "https://github.com/login/oauth/access_token");
        let params = form_params(poll_req);
        assert_eq!(params["device_code"], "dc-123");
        assert_eq!(
            params["grant_type"],
            "urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code"
        );
    }
    // copilot token exchange: Bearer = GitHub access token。
    assert_eq!(requests[3].method, HttpMethod::Get);
    assert_eq!(
        requests[3].url,
        "https://api.github.com/copilot_internal/v2/token"
    );
    assert_eq!(
        requests[3]
            .headers
            .iter()
            .find(|(k, _)| k == "Authorization")
            .map(|(_, v)| v.as_str()),
        Some("Bearer gho_access")
    );
    assert!(requests[3]
        .headers
        .iter()
        .any(|(k, v)| k == "Copilot-Integration-Id" && v == "vscode-chat"));
    // models fetch。
    assert_eq!(requests[4].method, HttpMethod::Get);
    assert_eq!(
        requests[4].url,
        "https://api.individual.githubcopilot.com/models"
    );
    assert!(requests[4]
        .headers
        .iter()
        .any(|(k, v)| k == "X-GitHub-Api-Version" && v == "2026-06-01"));
    // policy enable（known + unconfigured model のみ、enable も Bearer
    // Copilot token + chat-policy headers）。
    assert_eq!(requests[5].method, HttpMethod::Post);
    assert_eq!(
        requests[5].url,
        "https://api.individual.githubcopilot.com/models/mai-code-1.1-flash/policy"
    );
    assert!(requests[5]
        .headers
        .iter()
        .any(|(k, v)| k == "openai-intent" && v == "chat-policy"));
    let policy_body: Value = serde_json::from_slice(requests[5].body.as_ref().unwrap()).unwrap();
    assert_eq!(policy_body, json!({"state": "enabled"}));

    // credential 形状: refresh = GitHub token、access = Copilot token、
    // expires = expires_at*1000 - 5min、availableModelIds = picker ∪ enabled。
    let CredentialData::Oauth {
        access,
        refresh,
        expires,
        available_model_ids,
        extra,
        ..
    } = &credential.data
    else {
        panic!("expected oauth credential");
    };
    assert_eq!(refresh, "gho_access");
    assert_eq!(
        access,
        "tid=x;proxy-ep=proxy.individual.githubcopilot.com;sku=1"
    );
    assert_eq!(*expires, 4_102_444_800_000 - 300_000);
    assert!(extra.get("enterpriseUrl").is_none());
    assert_eq!(
        available_model_ids,
        &Some(vec![
            "gpt-5-mini".to_string(),
            "mai-code-1.1-flash".to_string()
        ])
    );

    // waitBeforeFirstPoll(1s) + pending 後の 1s sleep。
    assert_eq!(*sleeper.sleeps.lock().unwrap(), vec![1000, 1000]);
}

#[test]
fn login_enterprise_domain_uses_ghe_endpoints_and_stores_claim() {
    let http = MockHttp::new(vec![
        json_response(200, device_code_body()),
        json_response(200, json!({"access_token": "gho"})),
        json_response(
            200,
            json!({ "token": "copilot-token-no-proxy", "expires_at": 1_700_000_000.0 }),
        ),
        json_response(200, json!({ "data": [] })),
    ]);
    let flow = make_flow(http.clone(), recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&["ghe.example.com"]);
    let credential = flow.login(&mut interaction, &flow_ctx()).unwrap();

    let requests = http.requests();
    assert_eq!(requests[0].url, "https://ghe.example.com/login/device/code");
    assert_eq!(
        requests[2].url,
        "https://api.ghe.example.com/copilot_internal/v2/token"
    );
    // token に proxy-ep が無い → enterprise base で models fetch。
    assert_eq!(
        requests[3].url,
        "https://copilot-api.ghe.example.com/models"
    );
    let CredentialData::Oauth { extra, .. } = &credential.data else {
        panic!()
    };
    assert_eq!(extra["enterpriseUrl"], json!("ghe.example.com"));
}

#[test]
fn login_rejects_invalid_enterprise_domain() {
    let http = MockHttp::new(vec![]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&["not a domain"]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
    assert!(err.message().contains("Invalid GitHub Enterprise"));
}

#[test]
fn login_cancelled_flag_aborts_after_prompt() {
    let http = MockHttp::new(vec![]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    interaction.cancelled = true;
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Aborted);
    assert_eq!(err.message(), "Login cancelled");
}

#[test]
fn device_code_response_validation() {
    // verification_uri が http(s) でない → untrusted。
    let http = MockHttp::new(vec![json_response(
        200,
        json!({
            "device_code": "d", "user_code": "u",
            "verification_uri": "javascript:alert(1)", "expires_in": 1,
        }),
    )]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert!(err.message().contains("Untrusted verification_uri"));

    // 必須 field 欠落 → invalid fields。
    let http = MockHttp::new(vec![json_response(200, json!({"device_code": "d"}))]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert!(err
        .message()
        .contains("Invalid device code response fields"));

    // endpoint 非 2xx → fetchJson error。
    let http = MockHttp::new(vec![json_response(400, json!("bad"))]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert!(err.message().contains("400 Bad Request"));
}

#[test]
fn poll_slow_down_uses_server_interval_then_denied_fails() {
    let http = MockHttp::new(vec![
        json_response(200, device_code_body()),
        json_response(200, json!({"error": "slow_down", "interval": 10})),
        json_response(
            200,
            json!({"error": "access_denied", "error_description": "nope"}),
        ),
    ]);
    let sleeper = recording_sleeper();
    let flow = make_flow(http, sleeper.clone());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::ProviderFailure);
    assert_eq!(err.message(), "Device flow failed: access_denied: nope");
    // wait-first(1s) → slow_down → 10s。
    assert_eq!(*sleeper.sleeps.lock().unwrap(), vec![1000, 10000]);
}

#[test]
fn poll_times_out_when_device_code_expired() {
    let http = MockHttp::new(vec![json_response(
        200,
        json!({
            "device_code": "d", "user_code": "u",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 0,
        }),
    )]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Timeout);
    assert_eq!(err.message(), "Device flow timed out");
}

#[test]
fn models_fetch_retries_on_429() {
    let http = MockHttp::new(vec![
        json_response(200, device_code_body()),
        json_response(200, json!({"access_token": "gho"})),
        json_response(200, copilot_token_body()),
        json_response(429, json!("rate limited")),
        json_response(200, models_body()),
        json_response(200, json!({})),
    ]);
    let sleeper = recording_sleeper();
    let flow = make_flow(http.clone(), sleeper.clone());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let credential = flow.login(&mut interaction, &flow_ctx()).unwrap();
    let requests = http.requests();
    // device → poll → token → models(429) → models(200) → policy。
    assert_eq!(requests.len(), 6);
    assert_eq!(
        requests[3].url,
        "https://api.individual.githubcopilot.com/models"
    );
    assert_eq!(requests[4].url, requests[3].url);
    assert!(requests[5]
        .url
        .ends_with("/models/mai-code-1.1-flash/policy"));
    // 429 → 500ms backoff。
    assert!(sleeper.sleeps.lock().unwrap().contains(&500));
    let CredentialData::Oauth {
        available_model_ids,
        ..
    } = &credential.data
    else {
        panic!()
    };
    assert!(available_model_ids
        .as_ref()
        .unwrap()
        .contains(&"gpt-5-mini".to_string()));
}

// ---------------------------------------------------------------------------
// refresh / to_auth
// ---------------------------------------------------------------------------

#[test]
fn refresh_exchanges_and_stores_model_ids() {
    let http = MockHttp::new(vec![
        json_response(200, copilot_token_body()),
        json_response(200, models_body()),
    ]);
    let flow = make_flow(http.clone(), recording_sleeper());
    let credential = oauth_credential(
        "old-access",
        "gho_refresh",
        &[("enterpriseUrl", json!("ghe.corp.example"))],
    );
    let refreshed = flow.refresh(&credential, &flow_ctx()).unwrap();

    let requests = http.requests();
    assert_eq!(requests.len(), 2);
    // enterprise domain → api.ghe.corp.example。models fetch は token の
    // proxy-ep（individual）由来 base — `enterpriseDomain` は fallback のみ。
    assert_eq!(
        requests[0].url,
        "https://api.ghe.corp.example/copilot_internal/v2/token"
    );
    assert_eq!(
        requests[0]
            .headers
            .iter()
            .find(|(k, _)| k == "Authorization")
            .map(|(_, v)| v.as_str()),
        Some("Bearer gho_refresh")
    );
    assert_eq!(
        requests[1].url,
        "https://api.individual.githubcopilot.com/models"
    );
    let CredentialData::Oauth {
        access,
        refresh,
        available_model_ids,
        ..
    } = &refreshed.data
    else {
        panic!()
    };
    assert_eq!(
        access,
        "tid=x;proxy-ep=proxy.individual.githubcopilot.com;sku=1"
    );
    assert_eq!(refresh, "gho_refresh");
    // refresh 経路は policy enable を行わない — picker/unconfigured 両方が
    // availableModelIds に入り、policy POST は 1 件も発行されない
    // （requests.len() == 2 が証明）。
    assert_eq!(
        available_model_ids,
        &Some(vec![
            "gpt-5-mini".to_string(),
            "mai-code-1.1-flash".to_string()
        ])
    );
}

#[test]
fn refresh_http_error_is_provider_failure() {
    let http = MockHttp::new(vec![json_response(400, json!("expired"))]);
    let flow = make_flow(http, recording_sleeper());
    let credential = oauth_credential("a", "r", &[]);
    let err = flow.refresh(&credential, &flow_ctx()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::ProviderFailure);
}

#[test]
fn to_auth_derives_base_url_from_proxy_ep() {
    let flow = make_flow(MockHttp::new(vec![]), recording_sleeper());
    let credential = oauth_credential(
        "tid=a;exp=1;proxy-ep=proxy.business.githubcopilot.com;x=1",
        "r",
        &[],
    );
    let auth = flow.to_auth(&credential).unwrap();
    assert_eq!(
        auth.api_key.as_deref(),
        Some("tid=a;exp=1;proxy-ep=proxy.business.githubcopilot.com;x=1")
    );
    assert_eq!(
        auth.base_url.as_deref(),
        Some("https://api.business.githubcopilot.com")
    );
    // proxy-ep 無し + enterprise claim → copilot-api.<domain>。
    let credential = oauth_credential("plain-token", "r", &[("enterpriseUrl", json!("ghe.x.io"))]);
    let auth = flow.to_auth(&credential).unwrap();
    assert_eq!(
        auth.base_url.as_deref(),
        Some("https://copilot-api.ghe.x.io")
    );
    // proxy-ep 無し + enterprise 無し → individual default。
    let credential = oauth_credential("plain-token", "r", &[]);
    let auth = flow.to_auth(&credential).unwrap();
    assert_eq!(
        auth.base_url.as_deref(),
        Some("https://api.individual.githubcopilot.com")
    );
    // api_key credential は error。
    let err = flow
        .to_auth(&StoredCredential::api_key("k".into()))
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
}

// ---------------------------------------------------------------------------
// policy fallback（individual endpoint のみ）
// ---------------------------------------------------------------------------

#[test]
fn models_policy_fallback_only_for_individual() {
    let all_disabled_models = json!({ "data": [
        {
            "id": "custom-unknown-model",
            "model_picker_enabled": false,
            "capabilities": { "supports": { "tool_calls": true } },
            "policy": { "state": "enabled" },
        },
    ]});
    // individual（proxy-ep 無し token、enterprise 無し）→ fallback で
    // policy enabled id が available になる。unknown model は picker 無し
    // でも fallback 対象（known check は policyModelIds のみ）。
    let http = MockHttp::new(vec![
        json_response(200, device_code_body()),
        json_response(200, json!({"access_token": "gho"})),
        json_response(200, json!({ "token": "no-proxy-ep", "expires_at": 1.0 })),
        json_response(200, all_disabled_models.clone()),
    ]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&[""]);
    let credential = flow.login(&mut interaction, &flow_ctx()).unwrap();
    let CredentialData::Oauth {
        available_model_ids,
        ..
    } = &credential.data
    else {
        panic!()
    };
    assert_eq!(
        available_model_ids,
        &Some(vec!["custom-unknown-model".to_string()])
    );

    // enterprise → fallback 無し（picker 0 なら available も空）。
    let http = MockHttp::new(vec![
        json_response(200, device_code_body()),
        json_response(200, json!({"access_token": "gho"})),
        json_response(200, json!({ "token": "no-proxy-ep", "expires_at": 1.0 })),
        json_response(200, all_disabled_models),
    ]);
    let flow = make_flow(http, recording_sleeper());
    let mut interaction = ScriptedInteraction::answering(&["ghe.example.com"]);
    let credential = flow.login(&mut interaction, &flow_ctx()).unwrap();
    let CredentialData::Oauth {
        available_model_ids,
        ..
    } = &credential.data
    else {
        panic!()
    };
    assert_eq!(available_model_ids, &Some(Vec::<String>::new()));
}

// ---------------------------------------------------------------------------
// service wiring
// ---------------------------------------------------------------------------

#[test]
fn runtime_registers_builtin_copilot_flow_and_advertises_oauth() {
    let dir = tmp_dir();
    let runtime = Runtime::create(
        dir.path().to_path_buf(),
        Arc::new(MapEnv(BTreeMap::new())),
        &[],
        BTreeMap::new(),
    )
    .unwrap();
    let info = runtime
        .auth()
        .describe_provider(runtime.catalog(), "github-copilot")
        .unwrap();
    // oauth 宣言 + subscription あり → declaration ベースで advertise。
    assert!(info.methods.contains(&"oauth"));
    assert!(info.methods.contains(&"api_key"));
    assert!(info.subscription);
    // openai-codex は oauth-only(provider に api_key method 無し)→
    // declaration ベースで methods=["oauth"](flow 未実装でも advertise)。
    let codex = runtime
        .auth()
        .describe_provider(runtime.catalog(), "openai-codex")
        .unwrap();
    assert_eq!(codex.methods, vec!["oauth"]);
}

#[test]
fn service_login_runs_flow_and_persists_then_logout_removes() {
    let http = MockHttp::new(vec![
        json_response(200, device_code_body()),
        json_response(200, json!({"access_token": "gho"})),
        json_response(200, copilot_token_body()),
        json_response(200, models_body()),
        json_response(200, json!({})),
    ]);
    let dir = tmp_dir();
    let mut flows: BTreeMap<String, Arc<dyn OAuthFlow>> = BTreeMap::new();
    flows.insert(
        "github-copilot".to_string(),
        Arc::new(GitHubCopilotFlow::with_client(http, recording_sleeper())),
    );
    let runtime = Runtime::create(
        dir.path().to_path_buf(),
        Arc::new(MapEnv(BTreeMap::new())),
        &[],
        flows,
    )
    .unwrap();

    let mut interaction = ScriptedInteraction::answering(&[""]);
    let result = runtime
        .auth()
        .login(
            runtime.catalog(),
            "github-copilot",
            AuthType::Oauth,
            &mut interaction,
        )
        .unwrap();
    assert!(result.authenticated);
    assert_eq!(result.method, "oauth");
    assert!(result.subscription);

    // credentials.json に永続化（file 直接 read で schema 確認）。
    let raw: Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("credentials.json")).unwrap(),
    )
    .unwrap();
    let stored = &raw["providers"]["github-copilot"];
    assert_eq!(stored["type"], "oauth");
    assert_eq!(stored["refresh"], "gho");
    // credentials.json の固有 field は snake_case（T2 の新規 schema）。
    // extra claim の `enterpriseUrl` だけは TS 互換の camelCase 名。
    assert_eq!(
        stored["available_model_ids"],
        json!(["gpt-5-mini", "mai-code-1.1-flash"])
    );

    // status は configured + oauth credential 行、secret 非開示。
    let status = runtime.auth().status(runtime.catalog()).unwrap();
    let copilot = status
        .providers
        .iter()
        .find(|p| p.provider_id == "github-copilot")
        .unwrap();
    assert!(copilot.configured);
    assert!(copilot.methods.contains(&"oauth"));
    let cred = status
        .credentials
        .iter()
        .find(|c| c.provider_id == "github-copilot")
        .unwrap();
    assert_eq!(cred.credential_type, "oauth");
    let status_json = serde_json::to_string(&status).unwrap();
    assert!(!status_json.contains("gho"), "secret leaked: {status_json}");

    // get_auth → to_auth（base_url は proxy-ep 由来、期限先なので refresh
    // は走らない）。
    let entry = runtime.catalog().provider("github-copilot").unwrap();
    let auth = runtime
        .auth()
        .get_auth(entry, &BTreeMap::new())
        .unwrap()
        .unwrap();
    assert_eq!(
        auth.api_key.as_deref(),
        Some("tid=x;proxy-ep=proxy.individual.githubcopilot.com;sku=1")
    );
    assert_eq!(
        auth.base_url.as_deref(),
        Some("https://api.individual.githubcopilot.com")
    );

    // logout → credential 削除、冪等。
    let out = runtime
        .auth()
        .logout(
            runtime.catalog(),
            "github-copilot",
            &mut ScriptedInteraction::answering(&[]),
        )
        .unwrap();
    assert!(out.logged_out);
    assert!(CredentialStore::new(dir.path())
        .get("github-copilot")
        .unwrap()
        .is_none());
    let status = runtime.auth().status(runtime.catalog()).unwrap();
    assert!(status.credentials.is_empty());
}

// ---------------------------------------------------------------------------
// device-code poll contract（pollOAuthDeviceCodeFlow 直検証）
// ---------------------------------------------------------------------------

#[test]
fn poll_default_slow_down_increments_by_five_seconds() {
    let sleeper = recording_sleeper();
    let mut polls = VecDeque::from([
        DevicePoll::Pending,
        DevicePoll::SlowDown {
            interval_seconds: None,
        },
        DevicePoll::Complete("tok".to_string()),
    ]);
    let mut options = DeviceFlowPollOptions {
        interval_seconds: Some(2.0),
        expires_in_seconds: Some(900.0),
        wait_before_first_poll: true,
        poll: move || Ok(polls.pop_front().unwrap()),
    };
    let value = poll_device_code_flow(&mut options, sleeper.as_ref(), &|| false).unwrap();
    assert_eq!(value, "tok");
    // wait-first(2s) → pending 後 2s → slow_down（interval 無し → +5s = 7s）。
    assert_eq!(*sleeper.sleeps.lock().unwrap(), vec![2000, 2000, 7000]);
}

#[test]
fn poll_slow_down_timeout_uses_wsl_clock_message() {
    let sleeper = recording_sleeper();
    // expires をわずかに残し少なくとも 1 回 slow_down を観測させる
    // （mock sleeper は実時間を消費しないので loop は deadline まで回る）。
    let mut options = DeviceFlowPollOptions {
        interval_seconds: None,
        expires_in_seconds: Some(0.05),
        wait_before_first_poll: false,
        poll: || {
            Ok(DevicePoll::SlowDown {
                interval_seconds: None,
            })
        },
    };
    let err = poll_device_code_flow(&mut options, sleeper.as_ref(), &|| false).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Timeout);
    assert!(err.message().contains("slow_down"), "{}", err.message());
}

#[test]
fn poll_propagates_transport_error() {
    let sleeper = recording_sleeper();
    let mut options = DeviceFlowPollOptions {
        interval_seconds: None,
        expires_in_seconds: Some(60.0),
        wait_before_first_poll: false,
        poll: || {
            Err(CheaperError::new(
                ErrorCode::ProviderFailure,
                "network down",
            ))
        },
    };
    let err = poll_device_code_flow(&mut options, sleeper.as_ref(), &|| false).unwrap_err();
    assert_eq!(err.message(), "network down");
}

// ---------------------------------------------------------------------------
// AuthInteraction::event default rendering（notify への fallback）
// ---------------------------------------------------------------------------

#[test]
fn default_event_rendering_falls_back_to_notify() {
    /// `event` を override しない interaction → default が notify へ畳む。
    struct NotifyOnly {
        messages: Vec<String>,
    }
    impl AuthInteraction for NotifyOnly {
        fn prompt(&mut self, _p: &AuthPrompt<'_>) -> Result<String, CheaperError> {
            Ok(String::new())
        }
        fn notify(&mut self, message: &str) -> Result<(), CheaperError> {
            self.messages.push(message.to_string());
            Ok(())
        }
    }
    let mut it = NotifyOnly {
        messages: Vec::new(),
    };
    it.event(&AuthEvent::DeviceCode {
        user_code: "WXYZ".to_string(),
        verification_uri: "https://github.com/login/device".to_string(),
        interval_seconds: Some(5),
        expires_in_seconds: Some(900),
    })
    .unwrap();
    it.event(&AuthEvent::AuthUrl {
        url: "https://example.com/auth".to_string(),
        instructions: Some("do it".to_string()),
    })
    .unwrap();
    it.event(&AuthEvent::Progress {
        message: "Enabling models...".to_string(),
    })
    .unwrap();
    assert_eq!(
        it.messages,
        vec![
            "Go to https://github.com/login/device and enter code: WXYZ (expires in 900s)"
                .to_string(),
            "Open this URL to sign in:\n  https://example.com/auth\ndo it".to_string(),
            "Enabling models...".to_string(),
        ]
    );
}
