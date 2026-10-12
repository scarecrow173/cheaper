use super::*;
use crate::HttpResponse;
use std::collections::VecDeque;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Mutex;

// -----------------------------------------------------------------
// mocks(tests/oauth.rs と同型 + barrier で block できる step)
// -----------------------------------------------------------------

enum MockStep {
    Respond(HttpResponse),
    /// channel で release されるまで send を block する(exchange 中に
    /// backlog 接続を積む 409 drain 検証用)。
    Blocked(mpsc::Receiver<()>, HttpResponse),
}

struct MockHttp {
    steps: Mutex<VecDeque<MockStep>>,
    requests: Mutex<Vec<HttpRequest>>,
}

impl MockHttp {
    fn new(steps: Vec<MockStep>) -> Arc<Self> {
        Arc::new(Self {
            steps: Mutex::new(steps.into()),
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
        match self.steps.lock().unwrap().pop_front() {
            Some(MockStep::Respond(r)) => Ok(r),
            Some(MockStep::Blocked(rx, r)) => {
                rx.recv()
                    .map_err(|_| CheaperError::new(ErrorCode::InternalError, "barrier dropped"))?;
                Ok(r)
            }
            None => Err(CheaperError::new(
                ErrorCode::InternalError,
                "mock http: response queue exhausted",
            )),
        }
    }
}

fn json_response(status: u16, body: Value) -> HttpResponse {
    HttpResponse {
        status,
        status_text: status.to_string(),
        headers: Vec::new(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}

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

struct ScriptedInteraction {
    answers: Mutex<VecDeque<Result<String, CheaperError>>>,
    events: Arc<Mutex<Vec<AuthEvent>>>,
    cancelled: AtomicBool,
}

impl ScriptedInteraction {
    fn answering(answers: &[&str]) -> Self {
        Self {
            answers: Mutex::new(answers.iter().map(|a| Ok((*a).to_string())).collect()),
            events: Arc::new(Mutex::new(Vec::new())),
            cancelled: AtomicBool::new(false),
        }
    }
    fn events(&self) -> Arc<Mutex<Vec<AuthEvent>>> {
        self.events.clone()
    }
}

impl AuthInteraction for ScriptedInteraction {
    fn prompt(&mut self, _prompt: &AuthPrompt<'_>) -> Result<String, CheaperError> {
        self.answers.lock().unwrap().pop_front().unwrap_or_else(|| {
            Err(CheaperError::new(
                ErrorCode::InternalError,
                "scripted interaction: answer queue exhausted",
            ))
        })
    }
    fn event(&mut self, event: &AuthEvent) -> Result<(), CheaperError> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

fn flow_ctx() -> ProviderPluginContext {
    ProviderPluginContext {
        user_config_dir: PathBuf::from("/nonexistent"),
        env: Arc::new(crate::auth::api_key::ProcessEnv),
    }
}

// -----------------------------------------------------------------
// helpers — JWT / token response / listener factory / callback drive
// -----------------------------------------------------------------

/// `{"https://api.openai.com/auth":{"chatgpt_account_id":..}}` を持つ
/// 形だけの JWT。`exp` は optional。
fn fake_jwt(account_id: &str, exp: Option<i64>) -> String {
    let mut payload = serde_json::json!({
        "https://api.openai.com/auth": { "chatgpt_account_id": account_id },
    });
    if let Some(exp) = exp {
        payload["exp"] = Value::from(exp);
    }
    format!(
        "header.{}.sig",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap())
    )
}

fn token_json() -> Value {
    serde_json::json!({
        "access_token": fake_jwt("acct-1", None),
        "refresh_token": "refresh-1",
        "expires_in": 3600,
    })
}

/// factory に pre-bind した listener を 1 回だけ返させる。
fn single_listener_factory(
    listener: TcpListener,
) -> Arc<dyn Fn() -> std::io::Result<TcpListener> + Send + Sync> {
    let slot = Mutex::new(Some(listener));
    Arc::new(move || {
        slot.lock()
            .unwrap()
            .take()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::AddrInUse, "already used"))
    })
}

fn failing_factory() -> Arc<dyn Fn() -> std::io::Result<TcpListener> + Send + Sync> {
    Arc::new(|| {
        Err(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "port occupied",
        ))
    })
}

/// AuthUrl event が emit されるまで待ち、URL の `state` param を返す。
fn wait_for_auth_url_state(events: &Arc<Mutex<Vec<AuthEvent>>>) -> String {
    for _ in 0..500 {
        {
            let guard = events.lock().unwrap();
            for event in guard.iter() {
                if let AuthEvent::AuthUrl { url, .. } = event {
                    let parsed = url::Url::parse(url).unwrap();
                    return parsed
                        .query_pairs()
                        .find(|(k, _)| k == "state")
                        .map(|(_, v)| v.into_owned())
                        .expect("state param");
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("AuthUrl event not emitted");
}

/// callback request を送り、HTTP status line を返す。
fn drive_callback(addr: std::net::SocketAddr, target: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect callback");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(stream, "GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    stream.flush().unwrap();
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    buf.lines().next().unwrap_or_default().to_string()
}

fn oauth_credential() -> StoredCredential {
    StoredCredential::oauth(CredentialData::Oauth {
        access: fake_jwt("acct-1", None),
        refresh: "refresh-1".to_string(),
        expires: epoch_ms_now() + 3_600_000,
        account_id: Some("acct-1".to_string()),
        subscription: None,
        available_model_ids: None,
        extra: Map::new(),
    })
}

// -----------------------------------------------------------------
// parse_authorization_input
// -----------------------------------------------------------------

#[test]
fn parse_authorization_input_table() {
    // 空入力 → error
    assert!(parse_authorization_input("").is_err());
    assert!(parse_authorization_input("   ").is_err());
    // 絶対 URL — code/state を読む。code 無し URL は error。
    let parsed =
        parse_authorization_input("http://localhost:1455/auth/callback?code=C&state=S").unwrap();
    assert_eq!(parsed.code, "C");
    assert_eq!(parsed.state.as_deref(), Some("S"));
    assert!(parse_authorization_input("http://localhost:1455/auth/callback?state=S").is_err());
    // `a#b` → {a,b};`a#b#c` も {a,b}(split("#",2) parity)。
    let parsed = parse_authorization_input("a#b").unwrap();
    assert_eq!(parsed.code, "a");
    assert_eq!(parsed.state.as_deref(), Some("b"));
    let parsed = parse_authorization_input("a#b#c").unwrap();
    assert_eq!(parsed.code, "a");
    assert_eq!(parsed.state.as_deref(), Some("b"));
    // `code=` を含む query(Pi substring check)。
    let parsed = parse_authorization_input("code=X&state=Y").unwrap();
    assert_eq!(parsed.code, "X");
    assert_eq!(parsed.state.as_deref(), Some("Y"));
    // bare code — state 無しで照合 skip。
    let parsed = parse_authorization_input("just-a-code").unwrap();
    assert_eq!(parsed.code, "just-a-code");
    assert_eq!(parsed.state, None);
}

// -----------------------------------------------------------------
// credentials_from_token
// -----------------------------------------------------------------

#[test]
fn credentials_from_token_computes_fields() {
    // expires_in 優先
    let token = TokenResponse {
        access: fake_jwt("acct-9", None),
        refresh: "r".to_string(),
        expires_in_seconds: Some(100.0),
    };
    let cred = credentials_from_token(&token, "test").unwrap();
    match cred.data {
        CredentialData::Oauth {
            access,
            expires,
            account_id,
            ..
        } => {
            assert_eq!(account_id.as_deref(), Some("acct-9"));
            let expected = epoch_ms_now() + 100_000;
            assert!((expires - expected).abs() < 5_000);
            assert!(access.contains('.'));
        }
        _ => panic!("oauth data"),
    }
    // expires_in 欠落 → JWT exp(s)→ms fallback
    let token = TokenResponse {
        access: fake_jwt("acct-9", Some(2_000_000_000)),
        refresh: "r".to_string(),
        expires_in_seconds: None,
    };
    let cred = credentials_from_token(&token, "test").unwrap();
    match cred.data {
        CredentialData::Oauth { expires, .. } => {
            assert_eq!(expires, 2_000_000_000_000);
        }
        _ => panic!(),
    }
    // account_id 抽出不能 → error
    let token = TokenResponse {
        access: "not-a-jwt".to_string(),
        refresh: "r".to_string(),
        expires_in_seconds: Some(1.0),
    };
    assert!(credentials_from_token(&token, "test").is_err());
    // expires_in も exp も無い → error
    let token = TokenResponse {
        access: fake_jwt("acct-9", None),
        refresh: "r".to_string(),
        expires_in_seconds: None,
    };
    assert!(credentials_from_token(&token, "test").is_err());
    // refresh_token 欠落は read_token_response が reject
    assert!(read_token_response(serde_json::json!({
        "access_token": "a"
    }))
    .is_err());
}

// -----------------------------------------------------------------
// callback request evaluation ordering(Pi callback-server.js parity)
// -----------------------------------------------------------------

#[test]
fn callback_request_evaluation_ordering() {
    // path/method 不一致 → 404 + continue
    let (status, _, outcome) = evaluate_callback_request("GET /favicon.ico HTTP/1.1", "S", false);
    assert_eq!(status, 404);
    assert!(matches!(outcome, CallbackOutcome::NotCallback));
    let (status, _, _) =
        evaluate_callback_request("POST /auth/callback?code=C&state=S HTTP/1.1", "S", false);
    assert_eq!(status, 404);
    // state 不一致/欠落 → 400 + continue(`?error=` より先)
    let (status, _, outcome) =
        evaluate_callback_request("GET /auth/callback?code=C&state=WRONG HTTP/1.1", "S", false);
    assert_eq!(status, 400);
    assert!(matches!(outcome, CallbackOutcome::StateMismatch));
    let (status, _, outcome) = evaluate_callback_request(
        "GET /auth/callback?error=access_denied HTTP/1.1",
        "S",
        false,
    );
    assert_eq!(status, 400);
    assert!(matches!(outcome, CallbackOutcome::StateMismatch));
    // settle 済み → 409(state は一致が必要)
    let (status, _, outcome) =
        evaluate_callback_request("GET /auth/callback?code=C&state=S HTTP/1.1", "S", true);
    assert_eq!(status, 409);
    assert!(matches!(outcome, CallbackOutcome::AlreadySettled));
    // `?error=` + 正しい state → flow fail
    let (status, _, outcome) = evaluate_callback_request(
        "GET /auth/callback?error=access_denied&state=S HTTP/1.1",
        "S",
        false,
    );
    assert_eq!(status, 400);
    match outcome {
        CallbackOutcome::Error(m) => assert!(m.contains("access_denied")),
        _ => panic!("expected Error"),
    }
    // code 欠落 + 正しい state → 400 + continue
    let (status, _, outcome) =
        evaluate_callback_request("GET /auth/callback?state=S HTTP/1.1", "S", false);
    assert_eq!(status, 400);
    assert!(matches!(outcome, CallbackOutcome::MissingCode));
    // valid → settle
    let (status, _, outcome) =
        evaluate_callback_request("GET /auth/callback?code=CODE&state=S HTTP/1.1", "S", false);
    assert_eq!(status, 200);
    match outcome {
        CallbackOutcome::Code(c) => assert_eq!(c, "CODE"),
        _ => panic!("expected Code"),
    }
}

// -----------------------------------------------------------------
// browser login path(実 listener + 実 TcpStream で駆動)
// -----------------------------------------------------------------

#[test]
fn browser_login_happy_path() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let http = MockHttp::new(vec![MockStep::Respond(json_response(200, token_json()))]);
    let http_for_assert = http.clone();
    let flow =
        OpenAiCodexFlow::with_client(http, recording_sleeper(), single_listener_factory(listener));
    let mut interaction = ScriptedInteraction::answering(&["browser"]);
    let events = interaction.events();
    let ctx = flow_ctx();
    let result = std::thread::scope(|scope| {
        let handle = scope.spawn(|| flow.login(&mut interaction, &ctx));
        let state = wait_for_auth_url_state(&events);
        let status_line = drive_callback(
            addr,
            &format!("/auth/callback?code=TEST_CODE&state={state}"),
        );
        assert!(status_line.contains("200"), "{status_line}");
        handle.join().unwrap()
    });
    let credential = result.expect("login");
    match credential.data {
        CredentialData::Oauth {
            refresh,
            account_id,
            ..
        } => {
            assert_eq!(refresh, "refresh-1");
            assert_eq!(account_id.as_deref(), Some("acct-1"));
        }
        _ => panic!(),
    }
    // authorize URL の全 param を検証。
    let url = {
        let guard = events.lock().unwrap();
        guard
            .iter()
            .find_map(|e| match e {
                AuthEvent::AuthUrl { url, .. } => Some(url.clone()),
                _ => None,
            })
            .unwrap()
    };
    let parsed = url::Url::parse(&url).unwrap();
    let params: BTreeMap<String, String> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    assert_eq!(
        params.get("response_type").map(String::as_str),
        Some("code")
    );
    assert_eq!(params.get("client_id").map(String::as_str), Some(CLIENT_ID));
    assert_eq!(
        params.get("redirect_uri").map(String::as_str),
        Some(REDIRECT_URI)
    );
    assert_eq!(params.get("scope").map(String::as_str), Some(SCOPE));
    assert!(params.contains_key("code_challenge"));
    assert_eq!(
        params.get("code_challenge_method").map(String::as_str),
        Some("S256")
    );
    assert!(params.contains_key("state"));
    assert_eq!(
        params.get("id_token_add_organizations").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        params.get("codex_cli_simplified_flow").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        params.get("originator").map(String::as_str),
        Some("cheaper")
    );
    // exchange request は form で code/verifier/redirect_uri を含む。
    let requests = http_for_assert.requests();
    assert_eq!(requests.len(), 1);
    let body = String::from_utf8(requests[0].body.clone().unwrap()).unwrap();
    assert!(body.contains("grant_type=authorization_code"));
    assert!(body.contains("code=TEST_CODE"));
    assert!(body.contains("code_verifier="));
    assert!(body.contains("redirect_uri="));
}

#[test]
fn browser_login_stray_requests_do_not_kill_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let http = MockHttp::new(vec![MockStep::Respond(json_response(200, token_json()))]);
    let flow =
        OpenAiCodexFlow::with_client(http, recording_sleeper(), single_listener_factory(listener));
    let mut interaction = ScriptedInteraction::answering(&["browser"]);
    let events = interaction.events();
    let ctx = flow_ctx();
    let result = std::thread::scope(|scope| {
        let handle = scope.spawn(|| flow.login(&mut interaction, &ctx));
        let state = wait_for_auth_url_state(&events);
        // 非 callback path → 404 で継続。
        assert!(drive_callback(addr, "/favicon.ico").contains("404"));
        // state 不一致 → 400 で継続。
        assert!(drive_callback(addr, "/auth/callback?code=X&state=WRONG").contains("400"));
        // code 欠落 → 400 で継続。
        assert!(drive_callback(addr, &format!("/auth/callback?state={state}")).contains("400"));
        // valid → settle。
        assert!(
            drive_callback(addr, &format!("/auth/callback?code=OK&state={state}")).contains("200")
        );
        handle.join().unwrap()
    });
    result.expect("login");
}

#[test]
fn browser_login_error_param_fails_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let http = MockHttp::new(vec![]);
    let flow =
        OpenAiCodexFlow::with_client(http, recording_sleeper(), single_listener_factory(listener));
    let mut interaction = ScriptedInteraction::answering(&["browser"]);
    let events = interaction.events();
    let ctx = flow_ctx();
    let err = std::thread::scope(|scope| {
        let handle = scope.spawn(|| flow.login(&mut interaction, &ctx));
        let state = wait_for_auth_url_state(&events);
        assert!(drive_callback(
            addr,
            &format!("/auth/callback?error=access_denied&state={state}")
        )
        .contains("400"));
        handle.join().unwrap().unwrap_err()
    });
    assert!(err.message().contains("access_denied"), "{}", err.message());
}

#[test]
fn browser_login_deadline_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let http = MockHttp::new(vec![]);
    let flow =
        OpenAiCodexFlow::with_client(http, recording_sleeper(), single_listener_factory(listener))
            .with_login_timeout(Duration::from_millis(30));
    let mut interaction = ScriptedInteraction::answering(&["browser"]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::Timeout);
    assert!(err.message().contains("device_code"));
}

#[test]
fn browser_login_bind_failure_falls_back_to_manual_code() {
    let http = MockHttp::new(vec![MockStep::Respond(json_response(200, token_json()))]);
    let http_for_assert = http.clone();
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    // bare code — state 照合は skip。
    let mut interaction = ScriptedInteraction::answering(&["browser", "MANUAL_CODE"]);
    let credential = flow
        .login(&mut interaction, &flow_ctx())
        .expect("manual login");
    match credential.data {
        CredentialData::Oauth { refresh, .. } => assert_eq!(refresh, "refresh-1"),
        _ => panic!(),
    }
    let requests = http_for_assert.requests();
    assert_eq!(requests.len(), 1);
    let body = String::from_utf8(requests[0].body.clone().unwrap()).unwrap();
    assert!(body.contains("code=MANUAL_CODE"));
}

#[test]
fn browser_login_manual_state_mismatch_aborts() {
    let http = MockHttp::new(vec![]);
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    let mut interaction = ScriptedInteraction::answering(&["browser", "some-code#wrong-state"]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert!(err.message().contains("State mismatch"));
}

#[test]
fn browser_login_exchange_failure_is_provider_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let http = MockHttp::new(vec![MockStep::Respond(json_response(
        400,
        serde_json::json!({"error": "invalid_grant"}),
    ))]);
    let flow =
        OpenAiCodexFlow::with_client(http, recording_sleeper(), single_listener_factory(listener));
    let mut interaction = ScriptedInteraction::answering(&["browser"]);
    let events = interaction.events();
    let ctx = flow_ctx();
    let err = std::thread::scope(|scope| {
        let handle = scope.spawn(|| flow.login(&mut interaction, &ctx));
        let state = wait_for_auth_url_state(&events);
        drive_callback(addr, &format!("/auth/callback?code=C&state={state}"));
        handle.join().unwrap().unwrap_err()
    });
    assert_eq!(err.code(), ErrorCode::ProviderFailure);
}

#[test]
fn browser_login_post_settle_409_drain() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel::<()>();
    let http = MockHttp::new(vec![MockStep::Blocked(
        rx,
        json_response(200, token_json()),
    )]);
    let flow =
        OpenAiCodexFlow::with_client(http, recording_sleeper(), single_listener_factory(listener));
    let mut interaction = ScriptedInteraction::answering(&["browser"]);
    let events = interaction.events();
    let ctx = flow_ctx();
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| flow.login(&mut interaction, &ctx));
        let state = wait_for_auth_url_state(&events);
        // settle — flow は exchange の mock が block される点で停止。
        assert!(
            drive_callback(addr, &format!("/auth/callback?code=C&state={state}")).contains("200")
        );
        // exchange 中に backlog へ 2 本目の接続を積む。
        let mut probe = TcpStream::connect(addr).unwrap();
        probe
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        write!(
            probe,
            "GET /auth/callback?code=LATE&state={state} HTTP/1.1\r\nHost: localhost\r\n\r\n"
        )
        .unwrap();
        probe.flush().unwrap();
        // exchange を解放 → drain が 409 を返す。
        tx.send(()).unwrap();
        let mut buf = String::new();
        probe.read_to_string(&mut buf).unwrap();
        assert!(
            buf.lines().next().unwrap_or_default().contains("409"),
            "{buf}"
        );
        handle.join().unwrap().expect("login");
    });
}

// -----------------------------------------------------------------
// device-code login path
// -----------------------------------------------------------------

#[test]
fn device_code_login_happy_path() {
    let http = MockHttp::new(vec![
        MockStep::Respond(json_response(
            200,
            serde_json::json!({
                "device_auth_id": "d-1",
                "user_code": "USER-CODE",
                "interval": "1",
            }),
        )),
        // pending(403)→ complete(200)の poll 列。
        MockStep::Respond(json_response(403, serde_json::json!({}))),
        MockStep::Respond(json_response(
            200,
            serde_json::json!({
                "authorization_code": "DEV-CODE",
                "code_verifier": "DEV-VERIFIER",
            }),
        )),
        MockStep::Respond(json_response(200, token_json())),
    ]);
    let http_for_assert = http.clone();
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    let mut interaction = ScriptedInteraction::answering(&["device_code"]);
    let credential = flow
        .login(&mut interaction, &flow_ctx())
        .expect("device login");
    match credential.data {
        CredentialData::Oauth { refresh, .. } => assert_eq!(refresh, "refresh-1"),
        _ => panic!(),
    }
    let requests = http_for_assert.requests();
    assert_eq!(requests.len(), 4);
    // usercode は JSON body。
    assert_eq!(requests[0].url, DEVICE_USERCODE_URL);
    let body: Value = serde_json::from_slice(&requests[0].body.clone().unwrap()).unwrap();
    assert_eq!(body["client_id"], CLIENT_ID);
    // poll は device_auth_id/user_code を送る。
    let body: Value = serde_json::from_slice(&requests[1].body.clone().unwrap()).unwrap();
    assert_eq!(body["device_auth_id"], "d-1");
    assert_eq!(body["user_code"], "USER-CODE");
    // exchange は DEVICE_REDIRECT_URI + device code_verifier。
    let body = String::from_utf8(requests[3].body.clone().unwrap()).unwrap();
    assert!(body.contains("code=DEV-CODE"));
    assert!(body.contains("code_verifier=DEV-VERIFIER"));
    assert!(body.contains("deviceauth%2Fcallback"));
    // DeviceCode event。
    let guard = interaction.events();
    let guard = guard.lock().unwrap();
    assert!(guard.iter().any(|e| matches!(
        e,
        AuthEvent::DeviceCode {
            user_code,
            verification_uri,
            ..
        } if user_code == "USER-CODE" && verification_uri == DEVICE_VERIFY_URI
    )));
}

#[test]
fn device_code_usercode_404_is_specific_error() {
    let http = MockHttp::new(vec![MockStep::Respond(json_response(
        404,
        serde_json::json!({}),
    ))]);
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    let mut interaction = ScriptedInteraction::answering(&["device_code"]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert!(err.message().contains("not enabled"), "{}", err.message());
}

#[test]
fn device_code_exchange_failure_is_provider_failure() {
    let http = MockHttp::new(vec![
        MockStep::Respond(json_response(
            200,
            serde_json::json!({
                "device_auth_id": "d-1",
                "user_code": "U",
                "interval": 1,
            }),
        )),
        MockStep::Respond(json_response(
            200,
            serde_json::json!({
                "authorization_code": "C",
                "code_verifier": "V",
            }),
        )),
        MockStep::Respond(json_response(
            400,
            serde_json::json!({"error": "invalid_grant"}),
        )),
    ]);
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    let mut interaction = ScriptedInteraction::answering(&["device_code"]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert_eq!(err.code(), ErrorCode::ProviderFailure);
}

#[test]
fn unknown_login_method_is_error() {
    let flow = OpenAiCodexFlow::with_client(
        MockHttp::new(vec![]),
        recording_sleeper(),
        failing_factory(),
    );
    let mut interaction = ScriptedInteraction::answering(&["mystery"]);
    let err = flow.login(&mut interaction, &flow_ctx()).unwrap_err();
    assert!(err.message().contains("Unknown login method"));
}

// -----------------------------------------------------------------
// refresh / to_auth
// -----------------------------------------------------------------

#[test]
fn refresh_success_rotates_credential() {
    let http = MockHttp::new(vec![MockStep::Respond(json_response(
        200,
        serde_json::json!({
            "access_token": fake_jwt("acct-2", None),
            "refresh_token": "refresh-2",
            "expires_in": 7200,
        }),
    ))]);
    let http_for_assert = http.clone();
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    let credential = flow
        .refresh(&oauth_credential(), &flow_ctx())
        .expect("refresh");
    match credential.data {
        CredentialData::Oauth {
            refresh,
            account_id,
            ..
        } => {
            assert_eq!(refresh, "refresh-2");
            assert_eq!(account_id.as_deref(), Some("acct-2"));
        }
        _ => panic!(),
    }
    let requests = http_for_assert.requests();
    let body = String::from_utf8(requests[0].body.clone().unwrap()).unwrap();
    assert!(body.contains("grant_type=refresh_token"));
    assert!(body.contains("refresh_token=refresh-1"));
}

#[test]
fn refresh_invalid_grant_hints_relogin() {
    let http = MockHttp::new(vec![MockStep::Respond(json_response(
        400,
        serde_json::json!({"error": "invalid_grant"}),
    ))]);
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    let err = flow.refresh(&oauth_credential(), &flow_ctx()).unwrap_err();
    assert!(err.message().contains("re-login"), "{}", err.message());
}

#[test]
fn refresh_missing_refresh_token_hints_relogin() {
    let http = MockHttp::new(vec![MockStep::Respond(json_response(
        200,
        serde_json::json!({
            "access_token": fake_jwt("acct-1", None),
            "expires_in": 3600,
        }),
    ))]);
    let flow = OpenAiCodexFlow::with_client(http, recording_sleeper(), failing_factory());
    let err = flow.refresh(&oauth_credential(), &flow_ctx()).unwrap_err();
    assert!(err.message().contains("re-login"), "{}", err.message());
}

#[test]
fn to_auth_returns_access_and_account_id_header() {
    let flow = OpenAiCodexFlow::with_client(
        MockHttp::new(vec![]),
        recording_sleeper(),
        failing_factory(),
    );
    let auth = flow.to_auth(&oauth_credential()).expect("to_auth");
    assert!(auth.api_key.is_some());
    assert_eq!(
        auth.headers.get("chatgpt-account-id").map(String::as_str),
        Some("acct-1")
    );
    // credential の account_id が空でも JWT から再抽出。
    let credential = StoredCredential::oauth(CredentialData::Oauth {
        access: fake_jwt("acct-jwt", None),
        refresh: "r".to_string(),
        expires: epoch_ms_now() + 1_000,
        account_id: None,
        subscription: None,
        available_model_ids: None,
        extra: Map::new(),
    });
    let auth = flow.to_auth(&credential).expect("to_auth fallback");
    assert_eq!(
        auth.headers.get("chatgpt-account-id").map(String::as_str),
        Some("acct-jwt")
    );
}
