//! builtin provider factory — catalog `api` 種別 → Rig provider client 写像
//! (Phase 3 T7)。
//!
//! `cheaper-runtime` の `CompletionModelFactory` は runtime 側の trait で、
//! dependency direction(runtime → adapter)上 adapter 内で実装できない。
//! この module は adapter が提供する組立関数だけを置き、runtime 側
//! (e2e test / Phase 4 composition root)が `SessionSpec` →
//! [`BuiltinModelSpec`] へ写して呼ぶ。
//!
//! # api 種別 → Rig client
//!
//! | catalog `api`              | Rig wire                                     |
//! |----------------------------|----------------------------------------------|
//! | `openai-completions`       | `OpenAIConfig`(OPENAI dialect)+ `.chat()`    |
//! | `openai-responses`         | `OpenAIConfig`(OPENAI dialect)+ `.responses()` |
//! | `openai-codex-responses`   | `OpenAIConfig`(chatgpt::DIALECT)+ `.completion()` |
//! | `anthropic-messages`       | `AnthropicConfig`(ANTHROPIC dialect)+ `.completion()` |
//! | その他                     | `INVALID_MODEL_CONFIG`                        |
//!
//! # 設計メモ
//!
//! - **Ollama 等の openai-compatible endpoint** は `rig::providers::ollama`
//!   ではなく OPENAI dialect + `with_base_url` を使う。catalog の api 種別が
//!   `openai-completions`(= Chat Completions wire)を宣言しており、Ollama の
//!   `/v1/chat/completions` はその方言に合致する。rig の `ollama` provider
//!   は native `/api/chat` wire 用で、catalog api kind と対応しない。
//! - **Dialect は `OPENAI` 固定**。`Dialect` の field は全て `&'static str`
//!   で runtime に構築できない。`base_url`/`auth` は config 側の公開
//!   override(`with_base_url`/`with_auth`)で注入し、wire 振る舞いは
//!   OpenAI baseline(`Quirks::openai` + `completion_route` override 無しで
//!   `.chat()`/`.responses()` が明示 route を選ぶ)に従う。
//!   `OutputCap::OpenAiReasoningFamilies` は model 名で scoped されるため
//!   非 OpenAI model には `max_tokens` がそのまま送られる。
//! - **api_key 未解決**(ambient auth 等で `None`)のときは
//!   `Auth::OptionalBearer` に落とし、空の `Authorization` header を送ら
//!   ない。key 有りは dialect 既定(`Bearer` / `x-api-key`)。
//! - **custom headers** は Rig config 側に任意 header 注入面が無いため、
//!   headers 非空時のみ `reqwest` の `default_headers` を持つ専用 transport
//!   を `connect` へ渡す。per-request header(Bearer 等)は request 側が勝つ
//!   reqwest の規則に従う。headers 空なら shared client で pool を共有する。
//! - **`model.sampling_params` / `compat` / model-level `headers` は未配線**
//!   — request 単位の `additional_params` は `RigSession` 側の stream 構築
//!   でしか載せられず、本 factory の責務(model client 構築)外。
//!   `catalog model.base_url` より `auth.base_url`(credential 依存 endpoint、
//!   github-copilot `toAuth` 相当)を優先する。
//!
//! Rig 型(`OpenAIConfig`/`AnthropicConfig`/`ReqwestClient` 等)はこの
//! module に閉じ、戻り値は opaque [`CompletionModel`] のみ。

use std::collections::BTreeMap;

use cheaper_core::errors::{CheaperError, ErrorCode};
use rig::providers::anthropic::{self, AnthropicConfig};
use rig::providers::openai::{wire::Auth, wire::OPENAI, OpenAIConfig};
use rig::rig_reqwest::ReqwestClient;
use serde_json::{Map, Value};

use crate::agent::CompletionModel;

/// `cheaper-runtime` の `ResolvedAuth` に対応する adapter 所有の入力型
/// (adapter → runtime 方向の依存は禁じられているため、ここで同名 concept を
/// 持つ)。呼び出し側(runtime の `CompletionModelFactory` 実装)が写す。
#[derive(Debug, Clone, Default)]
pub struct BuiltinAuth {
    /// `Authorization: Bearer`(openai 系)/ `x-api-key`(anthropic)として
    /// 送る credential。`None` は ambient/unconfigured。
    pub api_key: Option<String>,
    /// credential 依存の endpoint override(copilot `toAuth` 等)。
    /// `Some` なら catalog `model.base_url` より優先。
    pub base_url: Option<String>,
    /// configured headers 解決済みの最終 request headers
    /// (`authHeader` 由来の `Authorization: Bearer` を含みうる)。
    /// transport の `default_headers` として適用する。
    pub headers: BTreeMap<String, String>,
}

/// `builtin_completion_model` の入力。`SessionSpec`(runtime 型)の model
/// 側 field + 写し変え済み auth を受ける。
pub struct BuiltinModelSpec<'a> {
    /// catalog `model.api` 種別。
    pub api: &'a str,
    /// catalog `model.id`(provider wire の model 名)。
    pub model_id: &'a str,
    /// catalog `model.base_url`(provider 既定 or user `baseUrl`)。空文字は
    /// 「未指定」として dialect 既定を使う。
    pub base_url: &'a str,
    /// catalog `model.maxTokens`(`CompletionModel::with_max_tokens` へ)。
    pub max_tokens: Option<u64>,
    /// 解決済み auth。`None` は credential 未設定 provider(keyless 等)。
    pub auth: Option<BuiltinAuth>,
}

fn invalid_model_config(api: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidModelConfig,
        format!("no builtin provider client for model api `{api}`"),
    )
    .with_details(Map::from_iter([("api".to_string(), Value::from(api))]))
}

/// request headers を持つ reqwest transport(headers 空なら shared client)。
/// header 名/値が invalid なら user config 由来の失敗として
/// `INVALID_MODEL_CONFIG`。
fn http_transport(headers: &BTreeMap<String, String>) -> Result<ReqwestClient, CheaperError> {
    use rig::rig_reqwest::reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    if headers.is_empty() {
        return Ok(ReqwestClient::default());
    }
    let mut map = HeaderMap::with_capacity(headers.len());
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|e| {
            CheaperError::new(
                ErrorCode::InvalidModelConfig,
                format!("invalid provider header name `{name}`"),
            )
            .with_cause(e)
        })?;
        let value = HeaderValue::from_str(value).map_err(|e| {
            CheaperError::new(
                ErrorCode::InvalidModelConfig,
                format!("invalid provider header value for `{name}`"),
            )
            .with_cause(e)
        })?;
        map.append(name, value);
    }
    let client = rig::rig_reqwest::reqwest::Client::builder()
        .default_headers(map)
        .build()
        .map_err(|e| {
            CheaperError::new(
                ErrorCode::InternalError,
                format!("failed to build provider http client: {e}"),
            )
            .with_cause(e)
        })?;
    Ok(ReqwestClient::from(client))
}

/// `api` 種別 + model id + auth から opaque `CompletionModel` を構築する。
/// `cheaper-runtime` の `CompletionModelFactory` 実装(test ・ Phase 4 CLI
/// composition root)が呼ぶ唯一の入口。
pub fn builtin_completion_model(
    spec: &BuiltinModelSpec<'_>,
) -> Result<CompletionModel, CheaperError> {
    let auth = spec.auth.clone().unwrap_or_default();
    let api_key = auth.api_key.clone().unwrap_or_default();
    // credential 依存 endpoint > catalog model base_url > dialect 既定。
    let base_url =
        auth.base_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(if spec.base_url.is_empty() {
                None
            } else {
                Some(spec.base_url)
            });
    match spec.api {
        "openai-completions" | "openai-responses" => {
            let mut config = OpenAIConfig::with_key(&OPENAI, api_key.clone());
            if api_key.is_empty() {
                // ambient/keyless endpoint(Ollama 等)に空 `Bearer ` を送らない。
                config = config.with_auth(Auth::OptionalBearer);
            }
            if let Some(url) = base_url {
                config = config.with_base_url(url);
            }
            let client = config.connect(http_transport(&auth.headers)?);
            let model = if spec.api == "openai-completions" {
                CompletionModel::from_rig(client.chat(spec.model_id))
            } else {
                CompletionModel::from_rig(client.responses(spec.model_id))
            };
            Ok(model.with_max_tokens(spec.max_tokens))
        }
        "openai-codex-responses" => {
            // ChatGPT subscription backend(`/backend-api/codex/responses`)。
            // rig の `chatgpt` dialect が Codex contract(`store:false`・
            // `stream:true` 強制・sampling/metadata 除去・
            // `reasoning.encrypted_content` include・`originator`/
            // `session_id` header)を担う。`completion()` は dialect の
            // `completion_route: Route::Responses` で Responses wire へ
            // 振り分けられる。
            let mut config =
                OpenAIConfig::with_key(&rig::providers::chatgpt::DIALECT, api_key.clone());
            if api_key.is_empty() {
                config = config.with_auth(Auth::OptionalBearer);
            }
            if let Some(url) = base_url {
                // rig dialect は `{base}/responses` を組み立てるため、
                // baseUrl は `.../codex` で終わる形に正規化する
                // (Pi `resolveCodexUrl` 相当 — catalog の
                // `https://chatgpt.com/backend-api` は
                // `.../backend-api/codex` になる)。
                config = config.with_base_url(&normalize_codex_base_url(url));
            }
            // `chatgpt-account-id` は request 側 header(config の
            // `with_account_id`)で送る。transport default header に残すと
            // 二重送信になるため取り除く。configured header に無ければ
            // access token JWT の claim から拾う(Pi `extractAccountId`
            // 相当)。
            let mut headers = auth.headers.clone();
            let account_id = take_account_id_header(&mut headers)
                .or_else(|| extract_account_id(&api_key));
            if let Some(id) = account_id {
                config = config.with_account_id(id);
            }
            let client = config.connect(http_transport(&headers)?);
            Ok(CompletionModel::from_rig(client.completion(spec.model_id))
                .with_max_tokens(spec.max_tokens))
        }
        "anthropic-messages" => {
            let mut config = AnthropicConfig::with_dialect(api_key, &anthropic::ANTHROPIC);
            if let Some(url) = base_url {
                config = config.with_base_url(url);
            }
            let client = config.connect(http_transport(&auth.headers)?);
            Ok(CompletionModel::from_rig(client.completion(spec.model_id))
                .with_max_tokens(spec.max_tokens))
        }
        other => Err(invalid_model_config(other)),
    }
}

/// Codex baseUrl の正規化 — rig dialect は `{base}/responses` を
/// 組み立てるため、`.../codex` で終わる形に揃える(Pi `resolveCodexUrl`
/// 相当)。末尾 `/` を除去し、末尾 `/responses` を strip してから、
/// `/codex` で終わっていなければ追加する。catalog の
/// `https://chatgpt.com/backend-api` は `.../backend-api/codex` に、
/// `.../codex`・`.../codex/responses` は `.../codex` に正規化される。
/// `/codex` を含まない `.../v1/responses` 入力は Pi が
/// `.../v1/responses/codex/responses` を生成するのに対し、本実装は
/// `.../v1/codex` を返す(不正 URL の生成を避ける意図的な差異)。
fn normalize_codex_base_url(url: &str) -> String {
    let mut url = url.trim_end_matches('/');
    if let Some(stripped) = url.strip_suffix("/responses") {
        url = stripped.trim_end_matches('/');
    }
    if url.ends_with("/codex") {
        url.to_string()
    } else {
        format!("{url}/codex")
    }
}

/// headers から `chatgpt-account-id`(大小無視)を取り出して除去する。
/// Codex dialect は `with_account_id` 経由で request 側 header を付ける
/// ため、transport default に残すと同名 header が二重送信になる。
fn take_account_id_header(headers: &mut BTreeMap<String, String>) -> Option<String> {
    let key = headers
        .keys()
        .find(|k| k.eq_ignore_ascii_case("chatgpt-account-id"))
        .cloned()?;
    headers
        .remove(&key)
        .filter(|v| !v.is_empty())
}

/// access token JWT の `https://api.openai.com/auth.chatgpt_account_id`
/// claim を読む(Pi `extractAccountId` 相当)。解釈不能なら `None`。
fn extract_account_id(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let raw = base64url_decode(payload)?;
    let value: Value = serde_json::from_slice(&raw).ok()?;
    value
        .get("https://api.openai.com/auth")?
        .get("chatgpt_account_id")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// base64url(padding 任意)の最小 decode — workspace に encode 専用の
/// base64 利用はあるが、decode のためだけに adapter へ依存を増やさない。
fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    fn sextet(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'-' | b'+' => Some(62),
            b'_' | b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    if bytes.is_empty() || bytes.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        for &b in chunk {
            acc = (acc << 6) | u32::from(sextet(b)?);
        }
        acc <<= 6 * (4 - chunk.len());
        for i in 0..chunk.len().saturating_sub(1) {
            out.push((acc >> (16 - 8 * i)) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec<'a>(api: &'a str, auth: Option<BuiltinAuth>) -> BuiltinModelSpec<'a> {
        BuiltinModelSpec {
            api,
            model_id: "m-1",
            base_url: "http://localhost:11434/v1",
            max_tokens: Some(4096),
            auth,
        }
    }

    #[test]
    fn builds_models_for_all_supported_api_kinds() {
        for api in [
            "openai-completions",
            "openai-responses",
            "openai-codex-responses",
            "anthropic-messages",
        ] {
            if let Err(e) = builtin_completion_model(&spec(api, None)) {
                panic!("{api}: {e}");
            }
        }
    }

    #[test]
    fn unknown_api_is_invalid_model_config() {
        let err = match builtin_completion_model(&spec("devin-chat", None)) {
            Err(e) => e,
            Ok(_) => panic!("expected INVALID_MODEL_CONFIG"),
        };
        assert_eq!(err.code(), ErrorCode::InvalidModelConfig);
    }

    #[test]
    fn invalid_header_name_is_invalid_model_config() {
        let mut headers = BTreeMap::new();
        headers.insert("bad header".to_string(), "v".to_string());
        let err = match builtin_completion_model(&spec(
            "openai-completions",
            Some(BuiltinAuth {
                api_key: Some("k".to_string()),
                base_url: None,
                headers,
            }),
        )) {
            Err(e) => e,
            Ok(_) => panic!("expected INVALID_MODEL_CONFIG"),
        };
        assert_eq!(err.code(), ErrorCode::InvalidModelConfig);
    }

    #[test]
    fn codex_model_builds_with_jwt_derived_account_id() {
        // `{"https://api.openai.com/auth":{"chatgpt_account_id":"acc-1"}}`
        // を payload に持つ署名無しの形だけの JWT。
        let payload = b64url(b"{\"https://api.openai.com/auth\":{\"chatgpt_account_id\":\"acc-1\"}}");
        let token = format!("header.{payload}.sig");
        assert_eq!(extract_account_id(&token).as_deref(), Some("acc-1"));
        let err = builtin_completion_model(&spec(
            "openai-codex-responses",
            Some(BuiltinAuth {
                api_key: Some(token),
                base_url: None,
                headers: BTreeMap::new(),
            }),
        ));
        if let Err(e) = err {
            panic!("{e}");
        }
    }

    #[test]
    fn account_id_header_is_consumed_not_duplicated() {
        let mut headers = BTreeMap::new();
        headers.insert("ChatGPT-Account-Id".to_string(), "acc-9".to_string());
        headers.insert("x-keep".to_string(), "v".to_string());
        assert_eq!(
            take_account_id_header(&mut headers).as_deref(),
            Some("acc-9")
        );
        assert!(!headers.contains_key("ChatGPT-Account-Id"));
        assert_eq!(headers.get("x-keep").map(String::as_str), Some("v"));
    }

    #[test]
    fn codex_base_url_is_normalized_to_codex_path() {
        // catalog の値(rig は `+ /responses` で `.../codex/responses` になる)
        assert_eq!(
            normalize_codex_base_url("https://chatgpt.com/backend-api"),
            "https://chatgpt.com/backend-api/codex"
        );
        assert_eq!(
            normalize_codex_base_url("https://chatgpt.com/backend-api/"),
            "https://chatgpt.com/backend-api/codex"
        );
        assert_eq!(
            normalize_codex_base_url("https://chatgpt.com/backend-api/codex"),
            "https://chatgpt.com/backend-api/codex"
        );
        assert_eq!(
            normalize_codex_base_url("https://chatgpt.com/backend-api/codex/"),
            "https://chatgpt.com/backend-api/codex"
        );
        assert_eq!(
            normalize_codex_base_url("https://chatgpt.com/backend-api/codex/responses"),
            "https://chatgpt.com/backend-api/codex"
        );
        // credential/custom baseUrl も同一規則
        assert_eq!(
            normalize_codex_base_url("https://relay.example/v1"),
            "https://relay.example/v1/codex"
        );
        // `/codex` を含まない `.../responses` は strip して `/codex` 追加
        // (Pi は `.../v1/responses/codex/responses` を生成する — 意図的差異)
        assert_eq!(
            normalize_codex_base_url("https://relay.example/v1/responses"),
            "https://relay.example/v1/codex"
        );
        assert_eq!(
            normalize_codex_base_url("https://chatgpt.com/backend-api/responses"),
            "https://chatgpt.com/backend-api/codex"
        );
    }

    #[test]
    fn base64url_decodes_padded_and_unpadded() {
        assert_eq!(
            base64url_decode("aGVsbG8").as_deref(),
            Some(b"hello".as_slice())
        );
        assert_eq!(
            base64url_decode("aGVsbG8=").as_deref(),
            Some(b"hello".as_slice())
        );
        assert_eq!(
            base64url_decode("aGVsbG9v").as_deref(),
            Some(b"helloo".as_slice())
        );
        assert_eq!(
            base64url_decode("aGVsbG9vYg").as_deref(),
            Some(b"helloob".as_slice())
        );
        assert!(base64url_decode("a").is_none());
        assert!(base64url_decode("aGVsb*G8").is_none());
    }

    /// test 用の base64url encode(padding 無し)。
    fn b64url(bytes: &[u8]) -> String {
        const T: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::new();
        for chunk in bytes.chunks(3) {
            let n = chunk.len();
            let acc = u32::from(chunk[0]) << 16
                | u32::from(*chunk.get(1).unwrap_or(&0)) << 8
                | u32::from(*chunk.get(2).unwrap_or(&0));
            out.push(T[(acc >> 18) as usize & 63] as char);
            out.push(T[(acc >> 12) as usize & 63] as char);
            if n > 1 {
                out.push(T[(acc >> 6) as usize & 63] as char);
            }
            if n > 2 {
                out.push(T[acc as usize & 63] as char);
            }
        }
        out
    }
}
