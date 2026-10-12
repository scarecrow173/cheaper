//! OpenAI Codex(ChatGPT subscription)OAuth flow
//! (pi-ai `auth/oauth/openai-codex.ts` の port)。
//!
//! browser PKCE flow(`localhost:1455` loopback callback)を default に、
//! `AuthPrompt::Select` で選択可能な device-code flow を持つ。
//! token は `/oauth/token` で取得し、credential には access/refresh/
//! `expires`(epoch ms)/`account_id`(JWT nested claim)を載せる。
//!
//! `AuthInteraction` は sync かつ non-Send のため、loopback callback と
//! ManualCode prompt の真の race は実現しない(Pi の
//! `waitForCallbackOrManualInput` 相当の代替):listener bind 成功時は
//! callback のみを `LOGIN_TIMEOUT` まで待ち、bind 失敗時のみ
//! ManualCode prompt へ fallback する。
//!
//! listener は `settle` 後も token exchange 完了まで保持する(Pi
//! `finally { close() }` parity)。login() は single-threaded で
//! exchange が accept loop を block するため、exchange 中に届いた
//! 接続は OS backlog に積まれ、exchange 後に drain pass が各接続へ
//! `409` を返してから listener を drop する(post-exchange drain)。

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::{
    fetch_json, poll_device_code_flow, AuthEvent, DeviceFlowPollOptions, DevicePoll, FlowSleeper,
    HttpClient, HttpMethod, HttpRequest,
};
use crate::auth::api_key::{AuthSource, ResolvedAuth};
use crate::auth::service::{AuthInteraction, AuthPrompt, AuthSelectOption, OAuthFlow};
use crate::auth::store::{CredentialData, StoredCredential};
use crate::plugin::ProviderPluginContext;

/// Pi `openai-codex.js` `CLIENT_ID`(Codex CLI と共有する public id)。
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const DEVICE_USERCODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
const DEVICE_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const DEVICE_VERIFY_URI: &str = "https://auth.openai.com/codex/device";
const DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const SCOPE: &str = "openid profile email offline_access";
/// browser-wait と device poll に共通する deadline(Pi
/// `DEVICE_CODE_TIMEOUT_SECONDS`=900)。Pi は browser-wait には timeout を
/// 掛けないが、`is_cancelled` が false 固定の caller では無限待ちになる
/// ため本実装は timeout を設ける(意図的な改善)。
const LOGIN_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// loopback poll 間隔。`is_cancelled` の確認粒度でもある。
const CALLBACK_POLL_INTERVAL: Duration = Duration::from_millis(80);
/// accepted socket の read timeout — byte を送らない stray 接続が
/// poll loop/cancel check を長時間塞がないようにする。
const CALLBACK_READ_TIMEOUT: Duration = Duration::from_secs(5);
/// request line + headers の読み取り上限。
const CALLBACK_READ_LIMIT: usize = 8 * 1024;
/// exchange 後の backlog drain の iteration 上限。
const CALLBACK_DRAIN_LIMIT: usize = 16;
/// listener factory の既定 bind address(Pi `getCallbackHost` 相当)。
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:1455";

fn provider_failure(message: impl Into<String>) -> CheaperError {
    CheaperError::new(ErrorCode::ProviderFailure, message)
}

fn epoch_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// PKCE / state / authorize URL
// ---------------------------------------------------------------------------

/// PKCE verifier(32 random bytes → base64url-no-pad)+ S256 challenge
/// (`base64url(SHA-256(verifier))`、Pi `generatePKCE` 相当)。
fn generate_pkce() -> (String, String) {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("CSPRNG failure");
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// `createState` — 16 random bytes → 32-char hex。
fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("CSPRNG failure");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// authorize URL 構築(Pi `buildAuthorizationUrl` 相当の param 一式)。
fn authorize_url(state: &str, code_challenge: &str) -> String {
    let mut url = url::Url::parse(AUTHORIZE_URL).expect("constant URL");
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", CLIENT_ID);
        q.append_pair("redirect_uri", REDIRECT_URI);
        q.append_pair("scope", SCOPE);
        q.append_pair("code_challenge", code_challenge);
        q.append_pair("code_challenge_method", "S256");
        q.append_pair("state", state);
        q.append_pair("id_token_add_organizations", "true");
        q.append_pair("codex_cli_simplified_flow", "true");
        q.append_pair("originator", "cheaper");
    }
    url.to_string()
}

// ---------------------------------------------------------------------------
// manual code input — `parseAuthorizationInput` 相当
// ---------------------------------------------------------------------------

/// 手動貼り付け入力の parse 結果。`state` は貼り付けに含まれた場合のみ。
struct ParsedAuthorizationInput {
    code: String,
    state: Option<String>,
}

/// `parseAuthorizationInput`(Pi parity、順序: 絶対 URL → `#` split →
/// `code=` substring → bare code)。state は存在する場合のみ照合対象。
fn parse_authorization_input(input: &str) -> Result<ParsedAuthorizationInput, CheaperError> {
    let value = input.trim();
    if value.is_empty() {
        return Err(provider_failure("Missing authorization code"));
    }
    // 絶対 URL — `new URL(value)` 相当。parse 成功したら他 branch へ
    // fallthrough せず query param から code/state を読む(code 無しは
    // error)。
    if let Ok(parsed) = url::Url::parse(value) {
        let mut code = None;
        let mut state = None;
        for (k, v) in parsed.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                _ => {}
            }
        }
        let code = code.ok_or_else(|| provider_failure("Missing authorization code"))?;
        return Ok(ParsedAuthorizationInput { code, state });
    }
    // `a#b` / `a#b#c` — JS `split("#",2)` 相当(先頭 2 要素、3 個目以降は
    // 破棄。`splitn(2, '#')` では `b#c` が state になるため不可)。
    if value.contains('#') {
        let mut parts = value.split('#');
        let code = parts.next().unwrap_or_default().to_string();
        let state = parts.next().map(|s| s.to_string());
        if code.is_empty() {
            return Err(provider_failure("Missing authorization code"));
        }
        return Ok(ParsedAuthorizationInput { code, state });
    }
    // `code=` を含む query 文字列(Pi は substring check —
    // `value.includes("code=")`)。
    if value.contains("code=") {
        let mut code = None;
        let mut state = None;
        for (k, v) in url::form_urlencoded::parse(value.trim_start_matches('?').as_bytes()) {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                _ => {}
            }
        }
        let code = code.ok_or_else(|| provider_failure("Missing authorization code"))?;
        return Ok(ParsedAuthorizationInput { code, state });
    }
    Ok(ParsedAuthorizationInput {
        code: value.to_string(),
        state: None,
    })
}

// ---------------------------------------------------------------------------
// loopback callback listener
// ---------------------------------------------------------------------------

/// callback request の評価結果(Pi `callback-server.js` の分岐順序)。
enum CallbackOutcome {
    /// method/path 不一致 → `404` + poll 継続。
    NotCallback,
    /// `state` 不一致(欠落含む)→ `400` + poll 継続。
    StateMismatch,
    /// settle 済み → `409` + poll 継続。
    AlreadySettled,
    /// `?error=` → flow 失敗終了(`error_description` は補助 text)。
    Error(String),
    /// `code` 欠落 → `400` + poll 継続。
    MissingCode,
    /// valid callback → settle。
    Code(String),
}

/// request 1 件の評価 + 応答 HTML。Pi の評価順序: path/method → state →
/// claimed → error → code。
fn evaluate_callback_request(
    request_line: &str,
    expected_state: &str,
    settled: bool,
) -> (u16, &'static str, CallbackOutcome) {
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    // `GET /auth/callback?...` のみ受理対象。
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if method != "GET" || path != "/auth/callback" {
        return (
            404,
            "<html><body><h1>Not found</h1></body></html>",
            CallbackOutcome::NotCallback,
        );
    }
    let params: BTreeMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    // state は routing guard — 欠落・不一致は fail させず継続する
    // (stray hit/別 session の遅延 redirect で login が死なない)。
    if params.get("state").map(String::as_str) != Some(expected_state) {
        return (
            400,
            "<html><body><h1>State mismatch</h1></body></html>",
            CallbackOutcome::StateMismatch,
        );
    }
    if settled {
        return (
            409,
            "<html><body><h1>Already completed</h1></body></html>",
            CallbackOutcome::AlreadySettled,
        );
    }
    if let Some(error) = params.get("error") {
        let detail = params
            .get("error_description")
            .map(|d| format!(": {d}"))
            .unwrap_or_default();
        return (
            400,
            "<html><body><h1>Sign-in failed</h1></body></html>",
            CallbackOutcome::Error(format!("{error}{detail}")),
        );
    }
    match params.get("code") {
        Some(code) if !code.is_empty() => (
            200,
            "<html><body><h1>Sign-in complete</h1><p>You can close this window.</p></body></html>",
            CallbackOutcome::Code(code.clone()),
        ),
        _ => (
            400,
            "<html><body><h1>Missing code</h1></body></html>",
            CallbackOutcome::MissingCode,
        ),
    }
}

/// 接続 1 本に応答 HTML を書いて close。
fn respond_callback(mut stream: std::net::TcpStream, status: u16, html: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Internal Server Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// 接続 1 本を処理する(read timeout + 8KiB cap 適用済みの socket 前提)。
/// 応答を返し、`CallbackOutcome` を返す。
/// request は `\r\n\r\n`(header 終端)または cap まで loop read する —
/// 未 read の受信データを残したまま close すると Windows が RST を返し、
/// browser が success page の代わりに接続エラーを表示し得る。
fn handle_callback_connection(
    mut stream: std::net::TcpStream,
    expected_state: &str,
    settled: bool,
) -> CallbackOutcome {
    // Windows では accepted socket が listener の nonblocking を継承
    // する。nonblocking のまま read すると request byte 未到着で
    // WouldBlock → 未 read のまま close → RST が browser へ返るため、
    // 明示的に blocking へ戻してから read timeout を適用する。
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(CALLBACK_READ_TIMEOUT));
    let mut buf = Vec::with_capacity(CALLBACK_READ_LIMIT);
    let mut chunk = [0u8; CALLBACK_READ_LIMIT];
    let n = loop {
        if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() >= CALLBACK_READ_LIMIT {
            break buf.len();
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) if buf.is_empty() => {
                // byte を送らない/timeout の接続 — 応答せず破棄して継続。
                return CallbackOutcome::NotCallback;
            }
            Ok(0) | Err(_) => break buf.len(),
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() >= CALLBACK_READ_LIMIT {
                    buf.truncate(CALLBACK_READ_LIMIT);
                }
            }
        }
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    let request_line = request.lines().next().unwrap_or_default();
    let (status, html, outcome) = evaluate_callback_request(request_line, expected_state, settled);
    respond_callback(stream, status, html);
    outcome
}

/// `waitForCallback` — listener bind 成功時の callback 待機。
/// `LOGIN_TIMEOUT` deadline まで nonblocking accept を poll し、
/// `is_cancelled` を毎回確認する。settle した code を返す(listener は
/// caller が exchange まで保持する)。
fn wait_for_callback(
    listener: &TcpListener,
    expected_state: &str,
    deadline: Instant,
    sleeper: &dyn FlowSleeper,
    cancelled: &dyn Fn() -> bool,
) -> Result<String, CheaperError> {
    listener
        .set_nonblocking(true)
        .map_err(|e| provider_failure(format!("callback listener setup failed: {e}")))?;
    loop {
        if cancelled() {
            return Err(super::login_cancelled());
        }
        if Instant::now() >= deadline {
            return Err(CheaperError::new(
                ErrorCode::Timeout,
                "OAuth callback timed out. Retry with --method device_code",
            ));
        }
        match listener.accept() {
            Ok((stream, _)) => match handle_callback_connection(stream, expected_state, false) {
                CallbackOutcome::Code(code) => return Ok(code),
                CallbackOutcome::Error(message) => {
                    return Err(provider_failure(format!("OAuth sign-in failed: {message}")))
                }
                _ => {}
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {}
        }
        sleeper.sleep_ms(CALLBACK_POLL_INTERVAL.as_millis() as u64);
    }
}

/// post-exchange drain — exchange 中に OS backlog に積まれた接続へ
/// `409`(または通常評価結果)を返してから listener を手放す。
/// drain でも 5s read timeout・8KiB cap・有界 iteration を維持する。
fn drain_callback_backlog(listener: &TcpListener, expected_state: &str) {
    for _ in 0..CALLBACK_DRAIN_LIMIT {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = handle_callback_connection(stream, expected_state, true);
            }
            Err(_) => break,
        }
    }
}

// ---------------------------------------------------------------------------
// JWT claims — account_id / exp
// ---------------------------------------------------------------------------

/// access token JWT の payload を decode(`base64` dep の URL_SAFE_NO_PAD
/// — adapter 側の inline decoder とは別実装、小さな重複として容認)。
fn jwt_payload(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let raw = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// `https://api.openai.com/auth.chatgpt_account_id` nested claim
/// (Pi `extractAccountId` 相当)。
fn extract_account_id(token: &str) -> Option<String> {
    jwt_payload(token)?
        .get("https://api.openai.com/auth")?
        .get("chatgpt_account_id")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// JWT `exp` claim(秒)→ epoch ms。
fn jwt_expiry_ms(token: &str) -> Option<i64> {
    let exp = jwt_payload(token)?.get("exp")?.as_f64()?;
    Some((exp * 1000.0) as i64)
}

// ---------------------------------------------------------------------------
// token exchange / credentials
// ---------------------------------------------------------------------------

/// `POST /oauth/token` の応答(Pi `readTokenResponse` 相当)。
/// `access_token`/`refresh_token` は必須、`expires_in` は省略可
/// (`credentials_from_token` 側で JWT `exp` fallback)。
struct TokenResponse {
    access: String,
    refresh: String,
    expires_in_seconds: Option<f64>,
}

fn read_token_response(raw: Value) -> Result<TokenResponse, CheaperError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| provider_failure("Invalid token response"))?;
    let (access, refresh) = match (
        obj.get("access_token").and_then(|v| v.as_str()),
        obj.get("refresh_token").and_then(|v| v.as_str()),
    ) {
        (Some(a), Some(r)) if !a.is_empty() && !r.is_empty() => (a, r),
        _ => return Err(provider_failure("Invalid token response")),
    };
    Ok(TokenResponse {
        access: access.to_string(),
        refresh: refresh.to_string(),
        expires_in_seconds: obj.get("expires_in").and_then(|v| v.as_f64()),
    })
}

/// `exchangeAuthorizationCode` — `POST /oauth/token` form。
fn exchange_authorization_code(
    http: &dyn HttpClient,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, CheaperError> {
    let raw = fetch_json(
        http,
        &HttpRequest::new(HttpMethod::Post, TOKEN_URL).form_body(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", redirect_uri),
        ]),
    )?;
    read_token_response(raw)
}

/// `credentialsFromToken` — token response → oauth credential。
/// `account_id` は JWT nested claim から必須で抽出(失敗は error)。
/// `expires` = `now_ms + expires_in*1000`、`expires_in` 欠落時は JWT
/// `exp`(superset fallback — Pi は `expires_in` を必須とする)。
/// どちらも取れなければ error(期限不明の credential を残さない)。
fn credentials_from_token(
    token: &TokenResponse,
    context: &str,
) -> Result<StoredCredential, CheaperError> {
    let account_id = extract_account_id(&token.access).ok_or_else(|| {
        provider_failure(format!(
            "Could not extract account id from access token during {context}"
        ))
    })?;
    let expires = token
        .expires_in_seconds
        .map(|secs| epoch_ms_now() + (secs * 1000.0) as i64)
        .or_else(|| jwt_expiry_ms(&token.access))
        .ok_or_else(|| {
            provider_failure(format!("Token response missing expiry during {context}"))
        })?;
    Ok(StoredCredential::oauth(CredentialData::Oauth {
        access: token.access.clone(),
        refresh: token.refresh.clone(),
        expires,
        account_id: Some(account_id),
        subscription: None,
        available_model_ids: None,
        extra: Map::new(),
    }))
}

// ---------------------------------------------------------------------------
// device-code flow
// ---------------------------------------------------------------------------

/// `requestDeviceUserCode` の応答(validation 済み)。
struct DeviceUserCode {
    device_auth_id: String,
    user_code: String,
    interval_seconds: f64,
}

/// OpenAI Codex OAuth flow(`openAiCodexOAuth`)。
/// `http`/`sleeper`/`listener_factory` は注入 seam — production は
/// `ureq` + thread sleep + `127.0.0.1:1455` bind、test は mock HTTP・
/// fake sleeper・pre-bound ephemeral listener で全 step を駆動する。
/// listener factory は Pi `PI_OAUTH_CALLBACK_HOST` 相当の override 点も
/// 兼ねる(host override 自体は現 scope では未配線)。
pub struct OpenAiCodexFlow {
    http: Arc<dyn HttpClient>,
    sleeper: Arc<dyn FlowSleeper>,
    listener_factory: Arc<dyn Fn() -> std::io::Result<TcpListener> + Send + Sync>,
    login_timeout: Duration,
}

impl OpenAiCodexFlow {
    /// 実 HTTP 経路の既定構築。
    pub fn new() -> Self {
        Self {
            http: Arc::new(super::UreqHttpClient::new()),
            sleeper: Arc::new(super::ThreadSleeper),
            listener_factory: Arc::new(|| TcpListener::bind(DEFAULT_BIND_ADDR)),
            login_timeout: LOGIN_TIMEOUT,
        }
    }

    /// test 注入(mock HTTP + fake sleeper + listener factory)。
    pub fn with_client(
        http: Arc<dyn HttpClient>,
        sleeper: Arc<dyn FlowSleeper>,
        listener_factory: Arc<dyn Fn() -> std::io::Result<TcpListener> + Send + Sync>,
    ) -> Self {
        Self {
            http,
            sleeper,
            listener_factory,
            login_timeout: LOGIN_TIMEOUT,
        }
    }

    /// browser-wait deadline の override(test で即時 timeout を駆動する)。
    pub fn with_login_timeout(mut self, timeout: Duration) -> Self {
        self.login_timeout = timeout;
        self
    }

    /// `requestDeviceUserCode` — `POST
    /// /api/accounts/deviceauth/usercode`(JSON `{client_id}`)。
    /// 404 は "device code login is not enabled for this server"。
    /// `interval` は number または numeric string(Pi `Number(trim())`
    /// 相当の寛容 parse)。
    fn request_device_user_code(&self) -> Result<DeviceUserCode, CheaperError> {
        let response = self.http.send(
            &HttpRequest::new(HttpMethod::Post, DEVICE_USERCODE_URL)
                .json_body(&serde_json::json!({ "client_id": CLIENT_ID })),
        )?;
        if response.status == 404 {
            return Err(provider_failure(
                "Device code login is not enabled for this server",
            ));
        }
        if !response.ok() {
            return Err(provider_failure(format!(
                "{} {}: {}",
                response.status,
                response.status_text,
                response.text()
            )));
        }
        let raw: Value = serde_json::from_slice(&response.body).map_err(|e| {
            provider_failure(format!("invalid JSON in device usercode response: {e}"))
        })?;
        let obj = raw
            .as_object()
            .ok_or_else(|| provider_failure("Invalid device usercode response"))?;
        let (device_auth_id, user_code) = match (
            obj.get("device_auth_id").and_then(|v| v.as_str()),
            obj.get("user_code").and_then(|v| v.as_str()),
        ) {
            (Some(d), Some(u)) if !d.is_empty() && !u.is_empty() => (d, u),
            _ => return Err(provider_failure("Invalid device usercode response")),
        };
        // `interval` は number または numeric string(`trim` 後 parse)。
        // 非 finite・負値は invalid response。
        let interval = match obj.get("interval") {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(s)) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| provider_failure("Invalid device usercode response"))?;
        Ok(DeviceUserCode {
            device_auth_id: device_auth_id.to_string(),
            user_code: user_code.to_string(),
            interval_seconds: interval,
        })
    }

    /// `pollOpenAICodexDeviceAuth` — `/api/accounts/deviceauth/token` を
    /// `poll_device_code_flow` で poll(endpoint は RFC-8628 形でない
    /// ため `fetch_json` ではなく `http.send` で status を見る)。
    /// `200` の `{authorization_code, code_verifier}` pair を返す —
    /// `poll_device_code_flow` は `Result<String>` のため、verifier は
    /// `&mut Option<String>` へ out-param として書き戻す。
    fn poll_device_token(
        &self,
        device: &DeviceUserCode,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(String, String), CheaperError> {
        let http = self.http.clone();
        let device_auth_id = device.device_auth_id.clone();
        let user_code = device.user_code.clone();
        let mut code_verifier: Option<String> = None;
        let authorization_code = {
            let code_verifier = &mut code_verifier;
            let mut options = DeviceFlowPollOptions {
                interval_seconds: Some(device.interval_seconds),
                // Pi `DEVICE_CODE_TIMEOUT_SECONDS` parity — response
                // `expires_in` は参照しない。
                expires_in_seconds: Some(self.login_timeout.as_secs_f64()),
                wait_before_first_poll: false,
                poll: move || {
                    let response = http.send(
                        &HttpRequest::new(HttpMethod::Post, DEVICE_TOKEN_URL).json_body(
                            &serde_json::json!({
                                "device_auth_id": device_auth_id,
                                "user_code": user_code,
                            }),
                        ),
                    )?;
                    if response.status == 200 {
                        let raw: Value = match serde_json::from_slice(&response.body) {
                            Ok(v) => v,
                            Err(_) => {
                                return Ok(DevicePoll::Failed {
                                    message: "Invalid device token response".to_string(),
                                })
                            }
                        };
                        let (code, verifier) = match (
                            raw.get("authorization_code").and_then(|v| v.as_str()),
                            raw.get("code_verifier").and_then(|v| v.as_str()),
                        ) {
                            (Some(c), Some(v)) if !c.is_empty() && !v.is_empty() => (c, v),
                            _ => {
                                return Ok(DevicePoll::Failed {
                                    message: "Invalid device token response".to_string(),
                                })
                            }
                        };
                        *code_verifier = Some(verifier.to_string());
                        return Ok(DevicePoll::Complete(code.to_string()));
                    }
                    if response.status == 403 || response.status == 404 {
                        return Ok(DevicePoll::Pending);
                    }
                    // 非 2xx — body の `error` は string または
                    // `{code: string}` object の両形。
                    let raw: Value = serde_json::from_slice(&response.body).unwrap_or(Value::Null);
                    let error_code = match raw.get("error") {
                        Some(Value::String(s)) => Some(s.clone()),
                        Some(Value::Object(o)) => {
                            o.get("code").and_then(|v| v.as_str()).map(String::from)
                        }
                        _ => None,
                    };
                    match error_code.as_deref() {
                        Some("deviceauth_authorization_pending") => Ok(DevicePoll::Pending),
                        Some("slow_down") => Ok(DevicePoll::SlowDown {
                            interval_seconds: raw.get("interval").and_then(|v| v.as_f64()),
                        }),
                        _ => Ok(DevicePoll::Failed {
                            message: format!(
                                "Device auth failed: {} {}",
                                response.status,
                                response.text()
                            ),
                        }),
                    }
                },
            };
            poll_device_code_flow(&mut options, self.sleeper.as_ref(), cancelled)?
        };
        let code_verifier =
            code_verifier.ok_or_else(|| provider_failure("Invalid device token response"))?;
        Ok((authorization_code, code_verifier))
    }

    /// device-code login path 全体(usercode 要求 → event emit → poll →
    /// exchange → credential)。
    fn login_device_code(
        &self,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<StoredCredential, CheaperError> {
        let device = self.request_device_user_code()?;
        interaction.event(&AuthEvent::DeviceCode {
            user_code: device.user_code.clone(),
            verification_uri: DEVICE_VERIFY_URI.to_string(),
            interval_seconds: Some(device.interval_seconds as u64),
            expires_in_seconds: Some(self.login_timeout.as_secs()),
        })?;
        let (code, verifier) = self.poll_device_token(&device, &|| interaction.is_cancelled())?;
        let token =
            exchange_authorization_code(self.http.as_ref(), &code, &verifier, DEVICE_REDIRECT_URI)?;
        credentials_from_token(&token, "device code login")
    }

    /// browser PKCE login path 全体(PKCE → AuthUrl emit →
    /// listener bind で callback 待機 / bind 失敗時は ManualCode
    /// fallback → exchange → credential)。
    fn login_browser(
        &self,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<StoredCredential, CheaperError> {
        let (verifier, challenge) = generate_pkce();
        let state = generate_state();
        interaction.event(&AuthEvent::AuthUrl {
            url: authorize_url(&state, &challenge),
            instructions: Some(
                "A browser window should open. Complete login to finish.".to_string(),
            ),
        })?;
        let code = match (self.listener_factory)() {
            Ok(listener) => {
                let code = wait_for_callback(
                    &listener,
                    &state,
                    Instant::now() + self.login_timeout,
                    self.sleeper.as_ref(),
                    &|| interaction.is_cancelled(),
                )?;
                // settle 後も listener は exchange 完了まで保持し、
                // backlog 接続へ 409 を返す(post-exchange drain)。
                let token =
                    exchange_authorization_code(self.http.as_ref(), &code, &verifier, REDIRECT_URI);
                drain_callback_backlog(&listener, &state);
                drop(listener);
                return credentials_from_token(&token?, "browser login");
            }
            Err(_) => {
                interaction.event(&AuthEvent::Info {
                    message: "Could not start the local callback listener. Paste the authorization code manually, or retry with --method device_code".to_string(),
                    links: Vec::new(),
                })?;
                let input = interaction.prompt(&AuthPrompt::ManualCode {
                    message: "Paste the authorization code (or the full callback URL)",
                    placeholder: Some("code or http://localhost:1455/auth/callback?code=..."),
                })?;
                let parsed = parse_authorization_input(&input)?;
                if let Some(given) = &parsed.state {
                    if given != &state {
                        return Err(provider_failure("State mismatch"));
                    }
                }
                parsed.code
            }
        };
        let token =
            exchange_authorization_code(self.http.as_ref(), &code, &verifier, REDIRECT_URI)?;
        credentials_from_token(&token, "browser login")
    }
}

impl Default for OpenAiCodexFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthFlow for OpenAiCodexFlow {
    /// `loginOpenAiCodex` — `Select` で method を選び browser/device path
    /// へ dispatch(未知 id は error、Pi `Unknown login method` parity)。
    fn login(
        &self,
        interaction: &mut dyn AuthInteraction,
        _ctx: &ProviderPluginContext,
    ) -> Result<StoredCredential, CheaperError> {
        let method = interaction.prompt(&AuthPrompt::Select {
            message: "OpenAI Codex login method",
            options: &[
                AuthSelectOption {
                    id: "browser",
                    label: "Browser sign-in",
                    description: Some("Opens a browser and waits for the localhost callback"),
                },
                AuthSelectOption {
                    id: "device_code",
                    label: "Device code",
                    description: Some("Use a one-time code on another device"),
                },
            ],
        })?;
        match method.as_str() {
            "browser" => self.login_browser(interaction),
            "device_code" => self.login_device_code(interaction),
            _ => Err(provider_failure(format!("Unknown login method: {method}"))),
        }
    }

    /// `refreshOpenAiCodexToken` — `POST /oauth/token`
    /// `{grant_type:refresh_token}`。`invalid_grant`/400 系・
    /// `refresh_token` 欠落・`account_id` 抽出失敗は re-login 案内を
    /// message に含める(service は credential を保持するため run 毎に
    /// 同じ失敗を繰り返す — 案内が唯一の mitigation)。
    fn refresh(
        &self,
        credential: &StoredCredential,
        _ctx: &ProviderPluginContext,
    ) -> Result<StoredCredential, CheaperError> {
        let refresh_token = match &credential.data {
            CredentialData::Oauth { refresh, .. } => refresh.clone(),
            _ => {
                return Err(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "openai-codex refresh requires an oauth credential",
                ))
            }
        };
        let raw = fetch_json(
            self.http.as_ref(),
            &HttpRequest::new(HttpMethod::Post, TOKEN_URL).form_body(&[
                ("grant_type", "refresh_token"),
                ("client_id", CLIENT_ID),
                ("refresh_token", &refresh_token),
            ]),
        )
        .map_err(|e| {
            provider_failure(format!(
                "OpenAI Codex token refresh failed (re-login may be required): {}",
                e.message()
            ))
        })?;
        let token = read_token_response(raw).map_err(|e| {
            provider_failure(format!(
                "OpenAI Codex token refresh failed (re-login may be required): {}",
                e.message()
            ))
        })?;
        credentials_from_token(&token, "token refresh — please re-login").map_err(|e| {
            provider_failure(format!(
                "OpenAI Codex token refresh failed (re-login may be required): {}",
                e.message()
            ))
        })
    }

    /// `toAuth` — `{apiKey: access}` + `chatgpt-account-id` header。
    /// `account_id` は credential field 優先、欠落時は JWT claim から
    /// 再抽出(両方取れなければ header 無し — adapter が JWT から
    /// 拾う fallback がある)。
    fn to_auth(&self, credential: &StoredCredential) -> Result<ResolvedAuth, CheaperError> {
        let (access, account_id) = match &credential.data {
            CredentialData::Oauth {
                access, account_id, ..
            } => (access.clone(), account_id.clone()),
            _ => {
                return Err(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "openai-codex toAuth requires an oauth credential",
                ))
            }
        };
        let mut headers = BTreeMap::new();
        if let Some(id) = account_id.or_else(|| extract_account_id(&access)) {
            headers.insert("chatgpt-account-id".to_string(), id);
        }
        Ok(ResolvedAuth {
            api_key: Some(access),
            headers,
            base_url: None,
            source: AuthSource::OAuth,
        })
    }
}

#[cfg(test)]
mod tests;
