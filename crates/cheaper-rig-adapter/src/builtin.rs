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
//! | catalog `api`        | Rig wire                                     |
//! |----------------------|----------------------------------------------|
//! | `openai-completions` | `OpenAIConfig`(OPENAI dialect)+ `.chat()`    |
//! | `openai-responses`   | `OpenAIConfig`(OPENAI dialect)+ `.responses()` |
//! | `anthropic-messages` | `AnthropicConfig`(ANTHROPIC dialect)+ `.completion()` |
//! | その他               | `INVALID_MODEL_CONFIG`                        |
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
}
