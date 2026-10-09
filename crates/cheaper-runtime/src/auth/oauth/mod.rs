//! OAuth flow contract（plan step 32）+ shared device-code / retry helpers。
//!
//! pi-ai `src/auth/oauth/*`（device-code.ts の poll loop・flow 共通の
//! `fetch` retry 規則）と `src/auth/types.ts` の `AuthEvent` の port。
//! flow 本体は `OAuthFlow` trait（`auth::service`）を実装し、provider id
//! で `AuthService`/`Runtime` の `oauth_flows` registry へ登録する
//! （flow 未登録の oauth 宣言 provider は `"oauth"` method を advertise
//! しない — plan Rollback 規則）。
//!
//! HTTP と sleep は trait seam（`HttpClient`/`FlowSleeper`）として注入する
//! — test は mock で network・実 sleep を伴わずに flow を駆動できる。
//! 中断（TS `interaction.signal` / `AbortSignal`）は
//! `AuthInteraction::is_cancelled` に集約する — sync CLI では Ctrl+C が
//! process 全体を終了させるため、mid-sleep 中断は行わず sleep 前後の
//! check で拾う。

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::Value;

use super::service::OAuthFlow;

pub mod github_copilot;
pub mod openai_codex;

pub use github_copilot::GitHubCopilotFlow;
pub use openai_codex::OpenAiCodexFlow;

// ---------------------------------------------------------------------------
// AuthEvent — `interaction.notify(event)` の port（TS `AuthEvent` union）
// ---------------------------------------------------------------------------

/// TS `AuthInfoLink` — `info` event に添える補助 link。
#[derive(Debug, Clone, PartialEq)]
pub struct AuthInfoLink {
    pub url: String,
    pub label: Option<String>,
}

/// `AuthEvent` — flow が login 中に interaction へ送る out-of-band 通知。
/// device-code flow（github-copilot / xai / kimi-coding）は `DeviceCode`、
/// browser + loopback/manual-code flow（anthropic / openai-codex /
/// openrouter / radius）は `AuthUrl`（+ `AuthPrompt::ManualCode`）を使う。
#[derive(Debug, Clone, PartialEq)]
pub enum AuthEvent {
    /// `{type:"info"}` — link 付きの案内 message。
    Info {
        message: String,
        links: Vec<AuthInfoLink>,
    },
    /// `{type:"auth_url"}` — browser で開く認可 URL（CLI 側で `openUrl`
    /// best-effort も行う想定。loopback listener は browser flow 実装時に
    /// 各 flow が持つ — T6 scope では contract のみ）。
    AuthUrl {
        url: String,
        instructions: Option<String>,
    },
    /// `{type:"device_code"}` — user code + verification URI。
    DeviceCode {
        user_code: String,
        verification_uri: String,
        interval_seconds: Option<u64>,
        expires_in_seconds: Option<u64>,
    },
    /// `{type:"progress"}` — 進行 message。
    Progress { message: String },
}

// ---------------------------------------------------------------------------
// HTTP seam — `fetch` 相当の注入可能な境界
// ---------------------------------------------------------------------------

/// request method（flow が使うのは GET/POST のみ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

impl HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
        }
    }
}

/// `fetch(url, init)` 相当の request。`timeout` は `AbortSignal.timeout`
/// 相当 — `None` は timeout 無し（TS `fetchJson` 経路と同じ）。
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    /// 順序保持の header list（name は case-insensitive 意味論）。
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout: Option<Duration>,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, url: impl Into<String>) -> Self {
        Self {
            method,
            url: url.into(),
            headers: Vec::new(),
            body: None,
            timeout: None,
        }
    }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
    /// `URLSearchParams` 相当の form body + `Content-Type` header。
    pub fn form_body(mut self, params: &[(&str, &str)]) -> Self {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in params {
            serializer.append_pair(k, v);
        }
        self.body = Some(serializer.finish().into_bytes());
        self.header("Content-Type", "application/x-www-form-urlencoded")
    }
    /// `JSON.stringify` body + `Content-Type: application/json`。
    pub fn json_body(mut self, body: &Value) -> Self {
        self.body = Some(serde_json::to_vec(body).expect("json body serialize"));
        self.header("Content-Type", "application/json")
    }
}

/// `fetch` response（status/headers/body）。`ok` は `response.ok` 相当。
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// `response.ok`（2xx）。
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
    /// `response.headers.get(name)` — case-insensitive、先勝ち。
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    /// `await response.text()`。
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// `fetch` の注入 seam。flow は network を直接持たず、この trait 経由で
/// 送る（test は mock で request 形状・応答列を制御する）。
pub trait HttpClient: Send + Sync {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, CheaperError>;
}

/// 既定 `HttpClient` — `ureq`（sync）実装。
/// `ureq` は非 2xx を `Error::Status` として返すが、response として取り
/// 出して扱う（`fetch` は HTTP error status で reject しない）。
pub struct UreqHttpClient {
    agent: ureq::Agent,
}

impl UreqHttpClient {
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new().build(),
        }
    }
}

impl Default for UreqHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for UreqHttpClient {
    fn send(&self, request: &HttpRequest) -> Result<HttpResponse, CheaperError> {
        let mut req = self.agent.request(request.method.as_str(), &request.url);
        if let Some(timeout) = request.timeout {
            req = req.timeout(timeout);
        }
        for (name, value) in &request.headers {
            req = req.set(name, value);
        }
        let result = match &request.body {
            Some(body) => req.send_bytes(body),
            None => req.call(),
        };
        // `fetch` parity — 非 2xx(`Error::Status`)も response として扱う。
        let response = match result {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(e) => return Err(transport_error(&request.url, &e)),
        };
        let headers = response
            .headers_names()
            .into_iter()
            .filter_map(|name| {
                response
                    .header(&name)
                    .map(|value| (name.clone(), value.to_string()))
            })
            .collect();
        let status = response.status();
        let status_text = response.status_text().to_string();
        let mut body = Vec::new();
        std::io::Read::read_to_end(&mut response.into_reader(), &mut body).map_err(|e| {
            CheaperError::new(
                ErrorCode::ProviderFailure,
                format!(
                    "OAuth request to {} failed while reading body: {e}",
                    request.url
                ),
            )
        })?;
        Ok(HttpResponse {
            status,
            status_text,
            headers,
            body,
        })
    }
}

fn transport_error(url: &str, e: &ureq::Error) -> CheaperError {
    CheaperError::new(
        ErrorCode::ProviderFailure,
        format!("OAuth request to {url} failed: {e}"),
    )
}

/// poll interval / retry backoff の sleep seam（test は no-op で駆動し、
/// sleep 予定時間を記録できる）。
pub trait FlowSleeper: Send + Sync {
    fn sleep_ms(&self, ms: u64);
}

/// 実 sleep 実装（`std::thread::sleep`）。
pub struct ThreadSleeper;

impl FlowSleeper for ThreadSleeper {
    fn sleep_ms(&self, ms: u64) {
        std::thread::sleep(Duration::from_millis(ms));
    }
}

// ---------------------------------------------------------------------------
// device-code poll — `auth/oauth/device-code.ts` `pollOAuthDeviceCodeFlow`
// ---------------------------------------------------------------------------

/// TS `CANCEL_MESSAGE`（`signal.aborted` 時に abortableSleep が投げる値）。
pub const LOGIN_CANCELLED_MESSAGE: &str = "Login cancelled";
/// TS `TIMEOUT_MESSAGE`。
pub const DEVICE_FLOW_TIMEOUT_MESSAGE: &str = "Device flow timed out";
/// TS `SLOW_DOWN_TIMEOUT_MESSAGE`。
pub const DEVICE_FLOW_SLOW_DOWN_TIMEOUT_MESSAGE: &str = "Device flow timed out after one or more slow_down responses. This is often caused by clock drift in WSL or VM environments. Please sync or restart the VM clock and try again.";
/// TS `MINIMUM_INTERVAL_MS`。
const MINIMUM_INTERVAL_MS: u64 = 1000;
/// RFC 8628 §3.2 — server が `interval` を省略したときの default 5s。
const DEFAULT_POLL_INTERVAL_SECONDS: f64 = 5.0;
/// RFC 8628 §3.5 — `slow_down` は interval +5s。
const SLOW_DOWN_INTERVAL_INCREMENT_MS: u64 = 5000;

/// device-code token endpoint 1 回分の poll 結果（TS `poll()` の戻り値
/// union: `{status:"complete"|"pending"|"slow_down"|"failed"}`）。
#[derive(Debug, Clone, PartialEq)]
pub enum DevicePoll {
    /// `{status:"complete", value}` — access token 取得。
    Complete(String),
    /// `{status:"pending"}` — `authorization_pending`。
    Pending,
    /// `{status:"slow_down"}` — server 提示 interval があればそれを採用。
    SlowDown { interval_seconds: Option<f64> },
    /// `{status:"failed", message}` — 終端 error（denial 等）。
    Failed { message: String },
}

/// `pollOAuthDeviceCodeFlow` の options。`poll` は token endpoint への
/// 1 request を行い `DevicePoll` を返す closure（transport error は
/// `Err` で flow を打ち切る — TS の fetch reject と同じ）。
pub struct DeviceFlowPollOptions<F>
where
    F: FnMut() -> Result<DevicePoll, CheaperError>,
{
    pub interval_seconds: Option<f64>,
    pub expires_in_seconds: Option<f64>,
    pub wait_before_first_poll: bool,
    pub poll: F,
}

/// `"Login cancelled"` error（`signal.aborted` 相当の中断）。
pub(super) fn login_cancelled() -> CheaperError {
    CheaperError::new(ErrorCode::Aborted, LOGIN_CANCELLED_MESSAGE)
}

fn provider_failure(message: impl Into<String>) -> CheaperError {
    CheaperError::new(ErrorCode::ProviderFailure, message)
}

/// `abortableSleep` — mid-sleep 中断は持たず、sleep 前後で cancel を拾う。
fn abortable_sleep(
    sleeper: &dyn FlowSleeper,
    ms: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), CheaperError> {
    if cancelled() {
        return Err(login_cancelled());
    }
    sleeper.sleep_ms(ms);
    if cancelled() {
        return Err(login_cancelled());
    }
    Ok(())
}

/// `expires_in` 秒 → `Instant` deadline（非 finite/overflow → 無期限 =
/// TS `Number.POSITIVE_INFINITY`）。負値は即時 timeout（deadline = now）。
fn expires_deadline(expires_in_seconds: Option<f64>) -> Option<Instant> {
    let secs = expires_in_seconds?;
    if !secs.is_finite() {
        return None;
    }
    Duration::try_from_secs_f64(secs.max(0.0))
        .ok()
        .and_then(|d| Instant::now().checked_add(d))
}

/// `pollOAuthDeviceCodeFlow` の port — device-code grant の token endpoint
/// polling loop（interval/slow_down/deadline/cancel の規則を忠実に再現）。
pub fn poll_device_code_flow<F>(
    options: &mut DeviceFlowPollOptions<F>,
    sleeper: &dyn FlowSleeper,
    cancelled: &dyn Fn() -> bool,
) -> Result<String, CheaperError>
where
    F: FnMut() -> Result<DevicePoll, CheaperError>,
{
    let deadline = expires_deadline(options.expires_in_seconds);
    let mut interval_ms = MINIMUM_INTERVAL_MS.max(
        (options
            .interval_seconds
            .unwrap_or(DEFAULT_POLL_INTERVAL_SECONDS)
            * 1000.0)
            .floor() as u64,
    );
    let mut slow_down_responses = 0u32;
    if options.wait_before_first_poll {
        let remaining_ms =
            deadline.map(|d| d.saturating_duration_since(Instant::now()).as_millis() as u64);
        match remaining_ms {
            Some(0) => {}
            Some(rem) => abortable_sleep(sleeper, interval_ms.min(rem), cancelled)?,
            None => abortable_sleep(sleeper, interval_ms, cancelled)?,
        }
    }
    loop {
        if let Some(d) = deadline {
            if Instant::now() >= d {
                break;
            }
        }
        if cancelled() {
            return Err(login_cancelled());
        }
        match (options.poll)()? {
            DevicePoll::Complete(value) => return Ok(value),
            DevicePoll::Failed { message } => return Err(provider_failure(message)),
            DevicePoll::Pending => {}
            DevicePoll::SlowDown { interval_seconds } => {
                slow_down_responses += 1;
                // server 提示 interval が有効ならそれを採用（GitHub は新しい
                // 最小 interval を `interval` で返す）。無ければ RFC 8628
                // §3.5 の +5s。
                interval_ms = match interval_seconds {
                    Some(s) if s.is_finite() && s > 0.0 => {
                        MINIMUM_INTERVAL_MS.max((s * 1000.0).floor() as u64)
                    }
                    _ => MINIMUM_INTERVAL_MS.max(interval_ms + SLOW_DOWN_INTERVAL_INCREMENT_MS),
                };
            }
        }
        match deadline {
            Some(d) => {
                let remaining = d.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                abortable_sleep(
                    sleeper,
                    interval_ms.min(remaining.as_millis() as u64),
                    cancelled,
                )?;
            }
            None => abortable_sleep(sleeper, interval_ms, cancelled)?,
        }
    }
    Err(CheaperError::new(
        ErrorCode::Timeout,
        if slow_down_responses > 0 {
            DEVICE_FLOW_SLOW_DOWN_TIMEOUT_MESSAGE
        } else {
            DEVICE_FLOW_TIMEOUT_MESSAGE
        },
    ))
}

// ---------------------------------------------------------------------------
// rate-limit retry — github-copilot `fetchWithRateLimitRetry` / `fetchJson`
// ---------------------------------------------------------------------------

/// `retryPolicy` — `{maxRetries, maxElapsedMs}`。
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub max_elapsed_ms: u64,
}

/// retry を伴わない request（TS `fetchJson` 経路）は `fetch` の既定挙動に
/// 合わせ timeout 無し。retry 経路は attempt 毎に 5s timeout を掛ける
/// （TS `AbortSignal.timeout(5000)`）。
pub const RETRY_ATTEMPT_TIMEOUT: Duration = Duration::from_millis(5000);

/// `fetchJson` — `!response.ok` で `{status} {statusText}: {text}` error、
/// 成功時は body を JSON parse。
pub fn fetch_json(http: &dyn HttpClient, request: &HttpRequest) -> Result<Value, CheaperError> {
    let response = http.send(request)?;
    if !response.ok() {
        return Err(provider_failure(format!(
            "{} {}: {}",
            response.status,
            response.status_text,
            response.text()
        )));
    }
    serde_json::from_slice(&response.body)
        .map_err(|e| provider_failure(format!("invalid JSON in OAuth response: {e}")))
}

/// `fetchWithRateLimitRetry` の port — 429 に対して `500 * 2^retry` の
/// exponential backoff（`Retry-After` があればそちらを優先）で retry。
/// `maxElapsedMs` は retry budget deadline（`AbortSignal.timeout` 相当）。
/// request の per-attempt timeout はこの helper が 5s に固定する。
pub fn fetch_with_rate_limit_retry(
    http: &dyn HttpClient,
    request: &HttpRequest,
    policy: RetryPolicy,
    sleeper: &dyn FlowSleeper,
    cancelled: &dyn Fn() -> bool,
) -> Result<HttpResponse, CheaperError> {
    let deadline = (policy.max_retries > 0 && policy.max_elapsed_ms > 0)
        .then(|| Instant::now() + Duration::from_millis(policy.max_elapsed_ms));
    let mut req = request.clone();
    req.timeout = Some(RETRY_ATTEMPT_TIMEOUT);
    let mut retry = 0u32;
    loop {
        let response = http.send(&req)?;
        if response.status != 429 || retry == policy.max_retries {
            return Ok(response);
        }
        let mut delay_ms = 500.0 * 2f64.powi(retry as i32);
        if let Some(retry_after) = response.header("retry-after") {
            match retry_after_delay_ms(retry_after, epoch_ms_now()) {
                Some(v) => delay_ms = v.max(0.0),
                // `Date.parse` も `parseFloat` も NaN → TS は retry せず
                // response を返す。
                None => return Ok(response),
            }
        }
        if let Some(d) = deadline {
            let remaining = d.saturating_duration_since(Instant::now());
            if delay_ms >= remaining.as_millis() as f64 {
                return Ok(response);
            }
        }
        abortable_sleep(sleeper, delay_ms as u64, cancelled)?;
        retry += 1;
    }
}

/// wall-clock epoch ms（`Date.now()` 相当 — `Retry-After` の HTTP-date 値
/// との差分計算に使う）。
fn epoch_ms_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// `Retry-After` header → delay ms。`Number.parseFloat`（先頭 float prefix）
/// で秒解釈し、失敗したら `Date.parse` 相当（IMF-fixdate HTTP-date のみ
/// サポート — `Date.parse` の全書式は port しない）で epoch ms との差分。
/// どちらも解釈不能なら `None`（TS の NaN → return response 分岐）。
fn retry_after_delay_ms(value: &str, now_ms: f64) -> Option<f64> {
    if let Some(seconds) = parse_float_prefix(value) {
        return Some(seconds * 1000.0);
    }
    parse_http_date_ms(value).map(|then_ms| then_ms - now_ms)
}

/// `Number.parseFloat` 相当 — 先頭 whitespace → sign → digits → `.` →
/// digits → exponent の prefix を parse（prefix 不一致なら None）。
fn parse_float_prefix(value: &str) -> Option<f64> {
    let s = value.trim_start();
    let bytes = s.as_bytes();
    let mut i = 0;
    if matches!(bytes.get(i), Some(b'+') | Some(b'-')) {
        i += 1;
    }
    let int_start = i;
    while matches!(bytes.get(i), Some(b) if b.is_ascii_digit()) {
        i += 1;
    }
    let has_int = i > int_start;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while matches!(bytes.get(i), Some(b) if b.is_ascii_digit()) {
            i += 1;
        }
        if !has_int && i == frac_start {
            return None; // "." のみ / ".x"（x 非数字）
        }
    } else if !has_int {
        return None;
    }
    if matches!(bytes.get(i), Some(b'e') | Some(b'E')) {
        let mut j = i + 1;
        if matches!(bytes.get(j), Some(b'+') | Some(b'-')) {
            j += 1;
        }
        let exp_start = j;
        while matches!(bytes.get(j), Some(b) if b.is_ascii_digit()) {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    s[..i].parse::<f64>().ok()
}

/// HTTP-date（IMF-fixdate `"Sun, 06 Nov 1994 08:49:37 GMT"`）→ epoch ms。
/// `Date.parse` の subset（OAuth `Retry-After` で観測される形式はこれ）。
fn parse_http_date_ms(value: &str) -> Option<f64> {
    let rest = value.split_once(',')?.1.trim_start();
    let mut parts = rest.split_whitespace();
    let day: i64 = parts.next()?.parse().ok()?;
    let month = match parts.next()? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i64 = parts.next()?.parse().ok()?;
    let mut hms = parts.next()?.split(':');
    let hour: i64 = hms.next()?.parse().ok()?;
    let minute: i64 = hms.next()?.parse().ok()?;
    let second: i64 = hms.next()?.parse().ok()?;
    if parts.next() != Some("GMT") {
        return None;
    }
    let days = crate::catalog::bundled::days_from_civil(year, month, day);
    Some((days * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1_000) as f64)
}

// ---------------------------------------------------------------------------
// builtin flow registry
// ---------------------------------------------------------------------------

/// builtin OAuth flow の既定 registry。`Runtime::create` が seed として
/// 使い、caller の `oauth_flows` entry は同名 id を override する
/// （test で mock flow/HTTP を差し込む経路）。
///
/// ISS-23: github-copilot が必須 1 件。openai-codex は Codex
/// subscription provider 対応で追加。anthropic 等の残りは Deferred
/// scope 決定時に同形の独立 sub-task として追加する。
pub fn builtin_oauth_flows() -> BTreeMap<String, Arc<dyn OAuthFlow>> {
    let mut flows: BTreeMap<String, Arc<dyn OAuthFlow>> = BTreeMap::new();
    flows.insert(
        "github-copilot".to_string(),
        Arc::new(GitHubCopilotFlow::new()),
    );
    flows.insert("openai-codex".to_string(), Arc::new(OpenAiCodexFlow::new()));
    flows
}
