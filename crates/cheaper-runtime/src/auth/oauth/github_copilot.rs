//! GitHub Copilot OAuth flow（pi-ai `auth/oauth/github-copilot.ts` の port）。
//!
//! device-code grant（`POST /login/device/code` → poll
//! `/login/oauth/access_token`）で GitHub access token を取得し、
//! `GET api.<domain>/copilot_internal/v2/token` で Copilot token へ交換
//! する。credential には `refresh` = GitHub access token、`access` =
//! Copilot token、`expires` = `expires_at*1000 - 5min`、`enterpriseUrl`
//! extra claim、`availableModelIds`（`GET <base>/models` + policy enable
//! 後の account catalog）を載せる。`to_auth` は Copilot token の
//! `proxy-ep` claim から per-credential `baseUrl` を導出する。

use std::collections::BTreeMap;
use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Map, Value};

use super::{
    fetch_json, fetch_with_rate_limit_retry, poll_device_code_flow, AuthEvent,
    DeviceFlowPollOptions, DevicePoll, FlowSleeper, HttpClient, HttpMethod, HttpRequest,
    RetryPolicy,
};
use crate::auth::api_key::{AuthSource, ResolvedAuth};
use crate::auth::service::{AuthInteraction, AuthPrompt, OAuthFlow};
use crate::auth::store::{CredentialData, StoredCredential};
use crate::plugin::ProviderPluginContext;

/// GitHub Copilot（VS Code copilot-chat）の OAuth client id。
/// TS は `atob("SXYxLmI1MDdhMDhjODdlY2ZlOTg=")` で実行時 decode している
/// — decode 済みの値を直接保持する（encode 側は秘匿ではなく難読化）。
const CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";

/// TS `COPILOT_HEADERS` — Copilot API request に必須の VS Code 偽装
/// headers。
const COPILOT_HEADERS: &[(&str, &str)] = &[
    ("User-Agent", "GitHubCopilotChat/0.35.0"),
    ("Editor-Version", "vscode/1.107.0"),
    ("Editor-Plugin-Version", "copilot-chat/0.35.0"),
    ("Copilot-Integration-Id", "vscode-chat"),
];
/// TS `COPILOT_API_VERSION`（`X-GitHub-Api-Version` header）。
const COPILOT_API_VERSION: &str = "2026-06-01";
/// device/oauth endpoint へ送る `User-Agent`（TS 各 fetch の header）。
const DEVICE_FLOW_USER_AGENT: &str = "GitHubCopilotChat/0.35.0";
/// enterprise 未指定時の device/oauth host。
const DEFAULT_DOMAIN: &str = "github.com";
/// individual アカウント既定の Copilot API base。
const INDIVIDUAL_BASE_URL: &str = "https://api.individual.githubcopilot.com";
/// credential `expires` の safety margin（TS `expires_at*1000 - 5*60*1000`）。
const EXPIRES_MARGIN_MS: i64 = 5 * 60 * 1000;
/// `refreshGitHubCopilotToken` / `enableGitHubCopilotModel` で使う retry
/// budget。
const NO_RETRY: RetryPolicy = RetryPolicy {
    max_retries: 0,
    max_elapsed_ms: 0,
};
const LOGIN_MODELS_RETRY: RetryPolicy = RetryPolicy {
    max_retries: 2,
    max_elapsed_ms: 5000,
};

fn provider_failure(message: impl Into<String>) -> CheaperError {
    CheaperError::new(ErrorCode::ProviderFailure, message)
}

/// `normalizeDomain` — enterprise 入力 → hostname（`new URL` 相当の
/// scheme 補完 + hostname 抽出）。空入力・parse 不能は `None`。
fn normalize_domain(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    url::Url::parse(&candidate)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// `getUrls` — device-code/access-token/copilot-token の 3 endpoint。
struct CopilotUrls {
    device_code_url: String,
    access_token_url: String,
    copilot_token_url: String,
}

fn copilot_urls(domain: &str) -> CopilotUrls {
    CopilotUrls {
        device_code_url: format!("https://{domain}/login/device/code"),
        access_token_url: format!("https://{domain}/login/oauth/access_token"),
        copilot_token_url: format!("https://api.{domain}/copilot_internal/v2/token"),
    }
}

/// `getBaseUrlFromToken` — Copilot token の `proxy-ep=proxy.X` claim →
/// `https://api.X`。
fn base_url_from_token(token: &str) -> Option<String> {
    let start = token.find("proxy-ep=")? + "proxy-ep=".len();
    let rest = &token[start..];
    let end = rest.find(';').unwrap_or(rest.len());
    let proxy_host = &rest[..end];
    // `proxy-ep=([^;]+)` は 1 文字以上必須。
    if proxy_host.is_empty() {
        return None;
    }
    let api_host = proxy_host
        .strip_prefix("proxy.")
        .map(|h| format!("api.{h}"))
        .unwrap_or_else(|| proxy_host.to_string());
    Some(format!("https://{api_host}"))
}

/// `getGitHubCopilotBaseUrl` — token の `proxy-ep` → enterprise →
/// individual の順で API base を導出。
fn copilot_base_url(token: Option<&str>, enterprise_domain: Option<&str>) -> String {
    if let Some(t) = token {
        if let Some(from_token) = base_url_from_token(t) {
            return from_token;
        }
    }
    if let Some(domain) = enterprise_domain {
        return format!("https://copilot-api.{domain}");
    }
    INDIVIDUAL_BASE_URL.to_string()
}

/// account model catalog の 1 entry（`parseGitHubCopilotModelCatalog` の
/// `accountModels` element）。
struct AccountModel {
    id: String,
    picker_enabled: bool,
    policy_state: Option<String>,
}

/// `{availableModelIds, policyModelIds}`。
struct CopilotModelCatalog {
    available_model_ids: Vec<String>,
    policy_model_ids: Vec<String>,
}

/// `parseGitHubCopilotModelCatalog` の port。
/// `known_models` は `GITHUB_COPILOT_MODELS`（bundled catalog id 集合）。
fn parse_copilot_model_catalog(
    raw: &Value,
    allow_policy_fallback: bool,
    known_models: &dyn Fn(&str) -> bool,
) -> Result<CopilotModelCatalog, CheaperError> {
    let data = raw
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| provider_failure("Invalid Copilot models response"))?;
    let mut account_models = Vec::new();
    for item in data {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let Some(id) = obj.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let tool_calls = obj
            .get("capabilities")
            .and_then(|c| c.get("supports"))
            .and_then(|s| s.get("tool_calls"));
        if tool_calls == Some(&Value::Bool(false)) {
            continue;
        }
        account_models.push(AccountModel {
            id: id.to_string(),
            picker_enabled: obj.get("model_picker_enabled") == Some(&Value::Bool(true)),
            policy_state: obj
                .get("policy")
                .and_then(|p| p.get("state"))
                .and_then(|s| s.as_str())
                .map(String::from),
        });
    }
    let picker_model_ids: Vec<String> = account_models
        .iter()
        .filter(|m| m.picker_enabled && m.policy_state.as_deref() != Some("disabled"))
        .map(|m| m.id.clone())
        .collect();
    let use_policy_fallback = allow_policy_fallback && picker_model_ids.is_empty();
    let available_model_ids = if !picker_model_ids.is_empty() || !allow_policy_fallback {
        picker_model_ids
    } else {
        account_models
            .iter()
            .filter(|m| m.policy_state.as_deref() == Some("enabled"))
            .map(|m| m.id.clone())
            .collect()
    };
    let policy_model_ids = account_models
        .iter()
        .filter(|m| {
            m.policy_state.as_deref() == Some("unconfigured")
                && known_models(&m.id)
                && (m.picker_enabled || use_policy_fallback)
        })
        .map(|m| m.id.clone())
        .collect();
    Ok(CopilotModelCatalog {
        available_model_ids,
        policy_model_ids,
    })
}

/// `GITHUB_COPILOT_MODELS` — bundled catalog の github-copilot model id
/// 集合（pi-ai は生成済み model map の key で判定する）。
fn is_known_copilot_model(id: &str) -> bool {
    crate::catalog::bundled::bundled_models("github-copilot")
        .map(|models| models.iter().any(|m| m.id == id))
        .unwrap_or(false)
}

/// `startDeviceFlow` の応答（validation 済み）。
struct DeviceCodeGrant {
    device_code: String,
    user_code: String,
    verification_uri: String,
    /// RFC 8628 `interval`（省略可）。
    interval_seconds: Option<f64>,
    expires_in_seconds: f64,
}

/// GitHub Copilot OAuth flow（`githubCopilotOAuth`）。
/// `http`/`sleeper` は注入 seam — production は `ureq` + thread sleep、
/// test は mock で全 step を駆動する。
pub struct GitHubCopilotFlow {
    http: Arc<dyn HttpClient>,
    sleeper: Arc<dyn FlowSleeper>,
}

impl GitHubCopilotFlow {
    /// 実 HTTP 経路の既定構築。
    pub fn new() -> Self {
        Self {
            http: Arc::new(super::UreqHttpClient::new()),
            sleeper: Arc::new(super::ThreadSleeper),
        }
    }

    /// test 注入（mock HTTP + no-op/記録 sleeper）。
    pub fn with_client(http: Arc<dyn HttpClient>, sleeper: Arc<dyn FlowSleeper>) -> Self {
        Self { http, sleeper }
    }

    /// `startDeviceFlow` — `POST /login/device/code`
    /// （client_id + `read:user` scope）。`verification_uri` は URL として
    /// parse し http(s) scheme 必須で正規化する（TS の untrusted-uri
    /// guard）。
    fn start_device_flow(&self, domain: &str) -> Result<DeviceCodeGrant, CheaperError> {
        let urls = copilot_urls(domain);
        let data = fetch_json(
            self.http.as_ref(),
            &HttpRequest::new(HttpMethod::Post, &urls.device_code_url)
                .header("Accept", "application/json")
                .header("User-Agent", DEVICE_FLOW_USER_AGENT)
                .form_body(&[("client_id", CLIENT_ID), ("scope", "read:user")]),
        )?;
        let obj = data
            .as_object()
            .ok_or_else(|| provider_failure("Invalid device code response"))?;
        let str_field = |name: &str| obj.get(name).and_then(|v| v.as_str());
        let num_field = |name: &str| obj.get(name).and_then(|v| v.as_f64());
        let (device_code, user_code, verification_uri, expires_in) = match (
            str_field("device_code"),
            str_field("user_code"),
            str_field("verification_uri"),
            num_field("expires_in"),
        ) {
            (Some(d), Some(u), Some(v), Some(e)) => (d, u, v, e),
            _ => return Err(provider_failure("Invalid device code response fields")),
        };
        // `interval` は省略可だが、存在するなら number 必須。
        if obj.get("interval").is_some() && num_field("interval").is_none() {
            return Err(provider_failure("Invalid device code response fields"));
        }
        // `open` で executable 等を開かないよう URL 必須 + http(s) scheme
        // 限定（TS `parsedUri.href` で正規化した値を返す）。
        let parsed = url::Url::parse(verification_uri)
            .map_err(|_| provider_failure("Untrusted verification_uri in device code response"))?;
        if parsed.scheme() != "https" && parsed.scheme() != "http" {
            return Err(provider_failure(
                "Untrusted verification_uri in device code response",
            ));
        }
        Ok(DeviceCodeGrant {
            device_code: device_code.to_string(),
            user_code: user_code.to_string(),
            verification_uri: parsed.to_string(),
            interval_seconds: num_field("interval"),
            expires_in_seconds: expires_in,
        })
    }

    /// `pollForGitHubAccessToken` — `pollOAuthDeviceCodeFlow` で
    /// `/login/oauth/access_token` を poll し GitHub access token を返す。
    fn poll_access_token(
        &self,
        domain: &str,
        device: &DeviceCodeGrant,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<String, CheaperError> {
        let urls = copilot_urls(domain);
        let http = self.http.clone();
        let mut options = DeviceFlowPollOptions {
            interval_seconds: device.interval_seconds,
            expires_in_seconds: Some(device.expires_in_seconds),
            wait_before_first_poll: true,
            poll: move || {
                let raw = fetch_json(
                    http.as_ref(),
                    &HttpRequest::new(HttpMethod::Post, &urls.access_token_url)
                        .header("Accept", "application/json")
                        .header("User-Agent", DEVICE_FLOW_USER_AGENT)
                        .form_body(&[
                            ("client_id", CLIENT_ID),
                            ("device_code", device.device_code.as_str()),
                            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                        ]),
                )?;
                let Some(obj) = raw.as_object() else {
                    return Ok(DevicePoll::Failed {
                        message: "Invalid device token response".to_string(),
                    });
                };
                if let Some(token) = obj.get("access_token").and_then(|v| v.as_str()) {
                    return Ok(DevicePoll::Complete(token.to_string()));
                }
                if let Some(error) = obj.get("error").and_then(|v| v.as_str()) {
                    let description = obj.get("error_description").and_then(|v| v.as_str());
                    return Ok(match error {
                        "authorization_pending" => DevicePoll::Pending,
                        "slow_down" => DevicePoll::SlowDown {
                            interval_seconds: obj.get("interval").and_then(|v| v.as_f64()),
                        },
                        _ => DevicePoll::Failed {
                            message: format!(
                                "Device flow failed: {error}{}",
                                description.map(|d| format!(": {d}")).unwrap_or_default()
                            ),
                        },
                    });
                }
                Ok(DevicePoll::Failed {
                    message: "Invalid device token response".to_string(),
                })
            },
        };
        poll_device_code_flow(&mut options, self.sleeper.as_ref(), cancelled)
    }

    /// `refreshGitHubCopilotAccessToken` — `GET
    /// api.<domain>/copilot_internal/v2/token`（Bearer = GitHub access
    /// token）で Copilot token へ交換し oauth credential を組み立てる
    /// （`availableModelIds` は caller が別途埋める）。
    fn refresh_access_token(
        &self,
        refresh_token: &str,
        enterprise_domain: Option<&str>,
    ) -> Result<StoredCredential, CheaperError> {
        let domain = enterprise_domain.unwrap_or(DEFAULT_DOMAIN);
        let urls = copilot_urls(domain);
        let mut request = HttpRequest::new(HttpMethod::Get, &urls.copilot_token_url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {refresh_token}"));
        for (name, value) in COPILOT_HEADERS {
            request = request.header(*name, *value);
        }
        let raw = fetch_json(self.http.as_ref(), &request)?;
        let obj = raw
            .as_object()
            .ok_or_else(|| provider_failure("Invalid Copilot token response"))?;
        let (token, expires_at) = match (
            obj.get("token").and_then(|v| v.as_str()),
            obj.get("expires_at").and_then(|v| v.as_f64()),
        ) {
            (Some(t), Some(e)) => (t, e),
            _ => return Err(provider_failure("Invalid Copilot token response fields")),
        };
        let mut extra = Map::new();
        // `enterpriseUrl` は TS と同じ claim 名（camelCase）で extra に載せる。
        if let Some(domain) = enterprise_domain {
            extra.insert("enterpriseUrl".to_string(), Value::from(domain));
        }
        Ok(StoredCredential::oauth(CredentialData::Oauth {
            access: token.to_string(),
            refresh: refresh_token.to_string(),
            expires: (expires_at * 1000.0) as i64 - EXPIRES_MARGIN_MS,
            account_id: None,
            subscription: None,
            available_model_ids: None,
            extra,
        }))
    }

    /// `fetchGitHubCopilotModels` — `GET <base>/models`（Bearer = Copilot
    /// token + copilot headers + `X-GitHub-Api-Version`）。
    /// `allowPolicyFallback` は individual base のときのみ（TS と同じ
    /// enterprise 判定）。
    fn fetch_models(
        &self,
        copilot_token: &str,
        enterprise_domain: Option<&str>,
        retry: RetryPolicy,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<CopilotModelCatalog, CheaperError> {
        let base_url = copilot_base_url(Some(copilot_token), enterprise_domain);
        let allow_policy_fallback = base_url == INDIVIDUAL_BASE_URL;
        let mut request = HttpRequest::new(HttpMethod::Get, format!("{base_url}/models"))
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {copilot_token}"));
        for (name, value) in COPILOT_HEADERS {
            request = request.header(*name, *value);
        }
        request = request.header("X-GitHub-Api-Version", COPILOT_API_VERSION);
        let response = fetch_with_rate_limit_retry(
            self.http.as_ref(),
            &request,
            retry,
            self.sleeper.as_ref(),
            cancelled,
        )?;
        if !response.ok() {
            return Err(provider_failure(format!(
                "{} {}: {}",
                response.status,
                response.status_text,
                response.text()
            )));
        }
        let raw: Value = serde_json::from_slice(&response.body).map_err(|e| {
            provider_failure(format!("invalid JSON in Copilot models response: {e}"))
        })?;
        parse_copilot_model_catalog(&raw, allow_policy_fallback, &is_known_copilot_model)
    }

    /// `enableGitHubCopilotModel` — `POST <base>/models/<id>/policy`
    /// `{state:"enabled"}`。429（retry 尽き後）は error、それ以外の非 2xx・
    /// transport error は `Ok(false)`（TS の `return false`）。
    fn enable_model(
        &self,
        copilot_token: &str,
        model_id: &str,
        enterprise_domain: Option<&str>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<bool, CheaperError> {
        let base_url = copilot_base_url(Some(copilot_token), enterprise_domain);
        let mut request = HttpRequest::new(
            HttpMethod::Post,
            format!("{base_url}/models/{model_id}/policy"),
        )
        .header("Authorization", format!("Bearer {copilot_token}"))
        .json_body(&json!({ "state": "enabled" }));
        for (name, value) in COPILOT_HEADERS {
            request = request.header(*name, *value);
        }
        request = request
            .header("openai-intent", "chat-policy")
            .header("x-interaction-type", "chat-policy");
        let response = match fetch_with_rate_limit_retry(
            self.http.as_ref(),
            &request,
            LOGIN_MODELS_RETRY,
            self.sleeper.as_ref(),
            cancelled,
        ) {
            Ok(r) => r,
            // transport error は中断時のみ伝播、それ以外は enable 失敗扱い
            // （TS `catch { if aborted throw; return false }`）。
            Err(e) => {
                if cancelled() {
                    return Err(e);
                }
                return Ok(false);
            }
        };
        if response.status == 429 {
            return Err(provider_failure(format!(
                "{} {}: {}",
                response.status,
                response.status_text,
                response.text()
            )));
        }
        Ok(response.ok())
    }

    /// `enableGitHubCopilotModels` — policy enable を順に試し成功 id を返す。
    /// 失敗（非中断 error）は batch を打ち切る（TS `break`）。
    fn enable_models(
        &self,
        copilot_token: &str,
        model_ids: &[String],
        enterprise_domain: Option<&str>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Vec<String>, CheaperError> {
        let mut enabled = Vec::new();
        for model_id in model_ids {
            match self.enable_model(copilot_token, model_id, enterprise_domain, cancelled) {
                Ok(true) => enabled.push(model_id.clone()),
                Ok(false) => {}
                Err(e) => {
                    if cancelled() {
                        return Err(e);
                    }
                    break;
                }
            }
        }
        Ok(enabled)
    }
}

impl Default for GitHubCopilotFlow {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthFlow for GitHubCopilotFlow {
    /// `loginGitHubCopilot` — enterprise domain prompt → device flow →
    /// Copilot token 交換 → models fetch + policy enable → credential。
    fn login(
        &self,
        interaction: &mut dyn AuthInteraction,
        _ctx: &ProviderPluginContext,
    ) -> Result<StoredCredential, CheaperError> {
        let input = interaction.prompt(&AuthPrompt::Text {
            message: "GitHub Enterprise URL/domain (blank for github.com)",
            placeholder: Some("company.ghe.com"),
        })?;
        if interaction.is_cancelled() {
            return Err(super::login_cancelled());
        }
        let trimmed = input.trim();
        let enterprise_domain = normalize_domain(&input);
        if !trimmed.is_empty() && enterprise_domain.is_none() {
            return Err(CheaperError::new(
                ErrorCode::InvalidRequest,
                "Invalid GitHub Enterprise URL/domain",
            ));
        }
        let domain = enterprise_domain.as_deref().unwrap_or(DEFAULT_DOMAIN);
        let device = self.start_device_flow(domain)?;
        interaction.event(&AuthEvent::DeviceCode {
            user_code: device.user_code.clone(),
            verification_uri: device.verification_uri.clone(),
            interval_seconds: device.interval_seconds.map(|s| s as u64),
            expires_in_seconds: Some(device.expires_in_seconds as u64),
        })?;
        let github_access_token =
            self.poll_access_token(domain, &device, &|| interaction.is_cancelled())?;
        let mut credential =
            self.refresh_access_token(&github_access_token, enterprise_domain.as_deref())?;
        let access = match &credential.data {
            CredentialData::Oauth { access, .. } => access.clone(),
            _ => unreachable!("refresh_access_token returns oauth data"),
        };
        let models = self.fetch_models(
            &access,
            enterprise_domain.as_deref(),
            LOGIN_MODELS_RETRY,
            &|| interaction.is_cancelled(),
        )?;
        let mut enabled_model_ids = Vec::new();
        if !models.policy_model_ids.is_empty() {
            interaction.event(&AuthEvent::Progress {
                message: "Enabling models...".to_string(),
            })?;
            enabled_model_ids = self.enable_models(
                &access,
                &models.policy_model_ids,
                enterprise_domain.as_deref(),
                &|| interaction.is_cancelled(),
            )?;
        }
        // `[...new Set([...availableModelIds, ...enabledModelIds])]` —
        // 順序保持の dedup union。
        let mut seen = std::collections::BTreeSet::new();
        let mut available_model_ids = Vec::new();
        for id in models
            .available_model_ids
            .into_iter()
            .chain(enabled_model_ids)
        {
            if seen.insert(id.clone()) {
                available_model_ids.push(id);
            }
        }
        if let CredentialData::Oauth {
            available_model_ids: slot,
            ..
        } = &mut credential.data
        {
            *slot = Some(available_model_ids);
        }
        Ok(credential)
    }

    /// `refresh` — `refreshGitHubCopilotToken`（Copilot token 交換 +
    /// models fetch、retry 無し）。`refresh` claim は GitHub access token
    /// でそのまま温存する。
    fn refresh(
        &self,
        credential: &StoredCredential,
        _ctx: &ProviderPluginContext,
    ) -> Result<StoredCredential, CheaperError> {
        let (refresh_token, enterprise_domain) = match &credential.data {
            CredentialData::Oauth { refresh, extra, .. } => {
                (refresh.clone(), copilot_enterprise_domain(extra))
            }
            _ => {
                return Err(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "github-copilot refresh requires an oauth credential",
                ))
            }
        };
        let mut refreshed =
            self.refresh_access_token(&refresh_token, enterprise_domain.as_deref())?;
        let access = match &refreshed.data {
            CredentialData::Oauth { access, .. } => access.clone(),
            _ => unreachable!(),
        };
        let models =
            self.fetch_models(&access, enterprise_domain.as_deref(), NO_RETRY, &|| false)?;
        if let CredentialData::Oauth {
            available_model_ids: slot,
            ..
        } = &mut refreshed.data
        {
            *slot = Some(models.available_model_ids);
        }
        Ok(refreshed)
    }

    /// `toAuth` — `{apiKey: access, baseUrl: getGitHubCopilotBaseUrl(...)}`。
    fn to_auth(&self, credential: &StoredCredential) -> Result<ResolvedAuth, CheaperError> {
        let (access, enterprise_domain) = match &credential.data {
            CredentialData::Oauth { access, extra, .. } => {
                (access.clone(), copilot_enterprise_domain(extra))
            }
            _ => {
                return Err(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "github-copilot toAuth requires an oauth credential",
                ))
            }
        };
        Ok(ResolvedAuth {
            api_key: Some(access.clone()),
            headers: BTreeMap::new(),
            base_url: Some(copilot_base_url(
                Some(&access),
                enterprise_domain.as_deref(),
            )),
            source: AuthSource::OAuth,
        })
    }
}

/// `copilotEnterpriseDomain` — credential `enterpriseUrl` claim を
/// `normalizeDomain` で正規化して返す（非文字列・空・parse 不能は None）。
fn copilot_enterprise_domain(extra: &Map<String, Value>) -> Option<String> {
    extra
        .get("enterpriseUrl")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(normalize_domain)
}
