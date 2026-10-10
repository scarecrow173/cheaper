//! `GetUserJwt` — Devin user JWT の mint + process 内 cache（TS
//! `providers/devin/jwt.ts` の port — vendored from pi-devin, Copyright
//! (c) 2026 Kashyab Ambarani, MIT License）。
//!
//! - `application/proto` unary POST（Connect-Protocol-Version: 1）
//! - 応答 proto の field 1（string、先頭 `eyJ`）を JWT として取得
//! - `exp` claim から expiry を導出（失敗時は now+600s）
//! - api key + host 単位で cache（expiry 60s 前まで再利用）・in-flight dedup
//! - 30s timeout（TS `AbortSignal.timeout(30_000)`）

use std::sync::Mutex as StdMutex;
use std::time::Duration;

use base64::Engine;
use bytes::Bytes;
use rig::http_client::{self, DynHttpClient, HttpClientExt, Request, StatusCode};
use tokio::sync::Mutex;

use crate::metadata::{build_metadata, MetadataInput};
use crate::wire::{encode_message, iter_fields, FieldValue};

/// JWT mint timeout（TS `AbortSignal.timeout(30_000)`）。
const MINT_TIMEOUT: Duration = Duration::from_secs(30);
/// exp claim が取れないときの既定寿命（秒）。
const FALLBACK_EXPIRES_SECS: i64 = 600;
/// cache 再利用の猶予（expiry の 60s 前まで）。
const REUSE_MARGIN_SECS: i64 = 60;

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    /// transport level 失敗（DNS/接続/応答 body 読み取り等）。
    /// `http_client::Error` は大きいため box（clippy::result_large_err）。
    #[error("GetUserJwt transport error: {0}")]
    Transport(Box<http_client::Error>),
    /// 非 2xx 応答（TS `GetUserJwt HTTP {status}: {body[..240]}`）。
    #[error("GetUserJwt HTTP {status}: {body}")]
    Status { status: StatusCode, body: String },
    /// 応答 proto に `eyJ` 始まりの field 1 が無い。
    #[error("GetUserJwt returned no JWT")]
    NoJwt,
    /// 30s timeout。
    #[error("GetUserJwt timed out")]
    Timeout,
    /// request 組み立て失敗。
    #[error("GetUserJwt request build failed: {0}")]
    Request(String),
}

impl From<http_client::Error> for JwtError {
    fn from(error: http_client::Error) -> Self {
        Self::Transport(Box::new(error))
    }
}

/// test seam が注入する mint 関数型。
pub type MintFn = fn(&str, &str) -> Result<MintedUserJwt, JwtError>;

pub struct MintedUserJwt {
    pub jwt: String,
    /// expiry（epoch seconds）。
    pub expires_at: i64,
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// JWT payload の `exp`（base64url JSON）を読む。失敗時は `now + 600`。
/// TS と同じく base64 pad を補ってから decode する。
pub fn jwt_expires_at(jwt: &str, now_secs: i64) -> i64 {
    let fallback = now_secs + FALLBACK_EXPIRES_SECS;
    let payload = match jwt.split('.').nth(1) {
        Some(p) => p,
        None => return fallback,
    };
    let padded = format!("{payload}{}", "=".repeat((4 - payload.len() % 4) % 4));
    let Ok(raw) = base64::engine::general_purpose::URL_SAFE.decode(&padded) else {
        return fallback;
    };
    let Ok(text) = String::from_utf8(raw) else {
        return fallback;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return fallback;
    };
    // TS `typeof parsed.exp === "number"` — integer も float も受ける。
    // `as_i64` は float を拒否して 600s fallback へ落ちるため、
    // `as_f64`（整数もカバー）で読み i64 へ truncate する。
    match value.get("exp").and_then(|v| v.as_f64()) {
        Some(exp) => exp as i64,
        None => fallback,
    }
}

/// `mintUserJwt` — `POST {host}/exa.auth_pb.AuthService/GetUserJwt`。
/// body は `encodeMessage(1, metadata)`（metadata は JWT 無しで組み立てる）。
pub async fn mint_user_jwt(
    http: &DynHttpClient,
    api_key: &str,
    host: &str,
) -> Result<MintedUserJwt, JwtError> {
    // TS `host.replace(/\/$/,"")` — trailing slash は 1 個だけ除去。
    let host = host.strip_suffix('/').unwrap_or(host);
    let metadata = build_metadata(&MetadataInput {
        api_key,
        user_jwt: None,
        session_id: &uuid::Uuid::new_v4().to_string(),
        request_id: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        trigger_id: &uuid::Uuid::new_v4().to_string(),
        version: None,
        ide: None,
    });
    let body = Bytes::from(encode_message(1, &metadata));
    let request = Request::post(format!("{host}/exa.auth_pb.AuthService/GetUserJwt"))
        .header("content-type", "application/proto")
        .header("connect-protocol-version", "1")
        .body(body)
        .map_err(|e| JwtError::Request(e.to_string()))?;

    // TS `AbortSignal.timeout(30_000)` は send + body 読み取り全体を
    // 覆う単一 deadline — 2 段 timeout（30s + 30s）にしない。
    let (status, body) = tokio::time::timeout(MINT_TIMEOUT, async {
        let response = http.send::<Bytes, Vec<u8>>(request).await?;
        let status = response.status();
        let body = response.into_body().await?;
        Ok::<_, JwtError>((status, body))
    })
    .await
    .map_err(|_| JwtError::Timeout)??;
    if !status.is_success() {
        let mut text = String::from_utf8_lossy(&body).into_owned();
        // TS `buf.toString("utf8").slice(0, 240)`。
        if text.chars().count() > 240 {
            text = text.chars().take(240).collect();
        }
        return Err(JwtError::Status { status, body: text });
    }

    for field in iter_fields(&body).flatten() {
        if field.num == 1 && field.wire == 2 {
            if let FieldValue::Bytes(raw) = field.value {
                let value = String::from_utf8_lossy(raw);
                if value.starts_with("eyJ") {
                    let jwt = value.into_owned();
                    let expires_at = jwt_expires_at(&jwt, now_secs());
                    return Ok(MintedUserJwt { jwt, expires_at });
                }
            }
        }
    }
    Err(JwtError::NoJwt)
}

struct CachedJwt {
    jwt: String,
    expires_at: i64,
    api_key: String,
    host: String,
}

/// process 内 cache。`getCachedUserJwt` の cache + in-flight dedup 相当
/// （tokio Mutex で mint 全体を serialize するため、同 (host, apiKey) の
/// 並行呼出しは後続が cache hit して dedup される）。
static CACHE: Mutex<Option<CachedJwt>> = Mutex::const_new(None);
/// mint 経路を差し替える test seam（単体 test が HTTP 無しで cache 意味論を
/// 検証する）。本番では `None`。
static MINT_OVERRIDE: StdMutex<Option<MintFn>> = StdMutex::new(None);

/// `getCachedUserJwt` — cache hit（expiry > now+60s）は即返し、
/// それ以外は mint して cache 更新。
pub async fn get_cached_user_jwt(
    http: &DynHttpClient,
    api_key: &str,
    host: &str,
) -> Result<String, JwtError> {
    // TS `host.replace(/\/$/,"")` — trailing slash は 1 個だけ除去。
    let host = host.strip_suffix('/').unwrap_or(host).to_string();
    let mut guard = CACHE.lock().await;
    let now = now_secs();
    if let Some(cache) = guard.as_ref() {
        if cache.api_key == api_key
            && cache.host == host
            && cache.expires_at > now + REUSE_MARGIN_SECS
        {
            return Ok(cache.jwt.clone());
        }
    }
    let minted = {
        let override_fn = MINT_OVERRIDE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .to_owned();
        match override_fn {
            Some(f) => f(api_key, &host)?,
            None => mint_user_jwt(http, api_key, &host).await?,
        }
    };
    let jwt = minted.jwt.clone();
    *guard = Some(CachedJwt {
        jwt: minted.jwt,
        expires_at: minted.expires_at,
        api_key: api_key.to_string(),
        host,
    });
    Ok(jwt)
}

/// `clearCachedUserJwt` — cache 全消去（logout / test）。
pub async fn clear_cached_user_jwt() {
    *CACHE.lock().await = None;
}

/// test 専用 seam — mint を差し替える。`None` で元に戻す。
#[doc(hidden)]
pub fn __set_mint_override_for_tests(f: Option<MintFn>) {
    *MINT_OVERRIDE.lock().unwrap_or_else(|e| e.into_inner()) = f;
}
