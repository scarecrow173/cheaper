//! Devin client metadata（TS `providers/devin/metadata.ts` の port —
//! vendored from pi-devin, Copyright (c) 2026 Kashyab Ambarani, MIT
//! License）。
//!
//! Cognition は Devin Local 専用 model（GPT-5.6 系: Sol/Terra/Luna）を
//! client ide 名で gate する。`ide="windsurf"` では GetChatMessage が
//! "This model is only in Devin Local." で拒否し、`ide="devin-desktop"`
//! なら応答する（upstream 実測済みの挙動を維持する）。

use std::sync::OnceLock;

use crate::wire::{encode_message, encode_string, encode_timestamp_body, encode_varint_field};

const FALLBACK_WINDSURF_VERSION: &str = "3.6.27";
/// macOS Devin Desktop が同梱する product.json（`windsurfVersion` 参照元）。
#[cfg(target_os = "macos")]
const PRODUCT_JSON: &str = "/Applications/Devin.app/Contents/Resources/app/product.json";

/// Devin Desktop の windsurfVersion。macOS のみ product.json を読み、
/// それ以外・読み取り失敗は fallback（TS `desktopWindsurfVersion`）。
fn desktop_windsurf_version() -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(raw) = std::fs::read_to_string(PRODUCT_JSON) {
            if let Ok(product) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(v) = product.get("windsurfVersion").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        return v.to_string();
                    }
                }
            }
        }
    }
    FALLBACK_WINDSURF_VERSION.to_string()
}

/// TS module 初期化時の `CLIENT_VERSION` 相当（初回参照時に解決・cache）。
pub fn client_version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(desktop_windsurf_version).as_str()
}

/// TS `CLIENT_IDE`（`"devin-desktop"` — Devin Local model gate の key）。
pub const CLIENT_IDE: &str = "devin-desktop";

/// `process.platform` 相当の OS 名（macOS→darwin / Windows→windows /
/// その他→linux）。
fn platform_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        _ => "linux",
    }
}

/// `MetadataInput` — buildMetadata の入力。
pub struct MetadataInput<'a> {
    pub api_key: &'a str,
    /// `GetUserJwt` で mint した JWT（mint request 自身は付けない）。
    pub user_jwt: Option<&'a str>,
    pub session_id: &'a str,
    /// `Date.now()`（epoch millis）。
    pub request_id: u64,
    pub trigger_id: &'a str,
    /// 省略時 `client_version()`。
    pub version: Option<&'a str>,
    /// 省略時 `CLIENT_IDE`。
    pub ide: Option<&'a str>,
}

/// `buildMetadata` — field 配置は TS と同一順（1 ide, 2 version,
/// 3 apiKey, 4 "en", 5 os, 7 version, 9 requestId, 10 sessionId,
/// 12 ide, 16 timestamp, 21? jwt, 25 triggerId, 26 "Unset", 28 ide）。
///
/// jwt は TS では parts の末尾に append されるため、body 末尾へ
/// `encode_string(21, jwt)` を連結したものと byte 一致する。
pub fn build_metadata(input: &MetadataInput<'_>) -> Vec<u8> {
    let version = input.version.unwrap_or_else(|| client_version());
    let ide = input.ide.unwrap_or(CLIENT_IDE);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut out = Vec::new();
    out.extend_from_slice(&encode_string(1, ide));
    out.extend_from_slice(&encode_string(2, version));
    out.extend_from_slice(&encode_string(3, input.api_key));
    out.extend_from_slice(&encode_string(4, "en"));
    out.extend_from_slice(&encode_string(5, platform_os()));
    out.extend_from_slice(&encode_string(7, version));
    out.extend_from_slice(&encode_varint_field(9, input.request_id));
    out.extend_from_slice(&encode_string(10, input.session_id));
    out.extend_from_slice(&encode_string(12, ide));
    out.extend_from_slice(&encode_message(16, &encode_timestamp_body(now_ms)));
    out.extend_from_slice(&encode_string(25, input.trigger_id));
    out.extend_from_slice(&encode_string(26, "Unset"));
    out.extend_from_slice(&encode_string(28, ide));
    if let Some(jwt) = input.user_jwt {
        out.extend_from_slice(&encode_string(21, jwt));
    }
    out
}
