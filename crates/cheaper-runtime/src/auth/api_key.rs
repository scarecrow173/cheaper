//! API-key / env auth resolution（G3）。
//!
//! pi-coding-agent `resolve-config-value.ts` と provider-composer の
//! `composeApiKeyAuth`（resolve/check）の port。対象値は models.json
//! `apiKey`・`headers`・plugin `apiKey`/`headers` で、解釈は:
//!
//! - `"!cmd"` → shell command を実行し stdout(trim) を使う（process 内
//!   cache、10s timeout、非零 exit/空出力は未解決扱い）。
//! - `"$VAR"` / `"${VAR}"` → env var 展開（1 つでも未設定なら全体未解決）。
//! - `"$$"` → `"$"`、`"$!"` → `"!"` の escape。
//! - それ以外 → literal。
//!
//! `check`（status/getAvailable の configured 判定）は command を実行しない
//! （`isCommandConfigValue` → configured 扱い）。
//!
//! 秘密値は `ResolvedAuth` に閉じ込め、status/diagnostics には
//! `AuthSource` のみ載せる。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{Map, Value};

use super::store::StoredCredential;
use crate::catalog::bundled::{
    has_bedrock_env_credentials, has_vertex_adc_credentials, EnvAuth, ANTHROPIC_API_KEY_ENV,
    ANTHROPIC_AUTH_TOKEN_ENV, ANTHROPIC_OAUTH_TOKEN_ENV,
};

/// provider auth の env 解決 context（pi-ai `authContext` 相当）。
/// CLI は `ProcessEnv`、test は `MapEnv` を注入する。
pub trait ProviderEnv: Send + Sync {
    /// `ctx.env(name)`。空文字・空白のみは unset 扱い（TS
    /// `defaultProviderAuthContext` の `trim().length>0` 判定と
    /// `resolveEnvConfigValue` の `||` フォールバックに揃える）。
    fn env(&self, name: &str) -> Option<String>;
    /// `ctx.fileExists(path)`。`~` 先頭は home 展開する。
    fn file_exists(&self, path: &str) -> bool;
    /// home dir（vertex 既定 ADC path 解決用）。
    fn home_dir(&self) -> Option<PathBuf>;
}

/// `defaultProviderAuthContext` 相当の実装（process env + real fs）。
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessEnv;

impl ProviderEnv for ProcessEnv {
    fn env(&self, name: &str) -> Option<String> {
        std::env::var(name).ok().filter(|v| !v.trim().is_empty())
    }
    fn file_exists(&self, path: &str) -> bool {
        let resolved = match path.strip_prefix('~') {
            Some(rest) => self
                .home_dir()
                .map(|h| h.join(rest.trim_start_matches(['/', '\\']))),
            None => Some(PathBuf::from(path)),
        };
        resolved.map(|p| p.exists()).unwrap_or(false)
    }
    fn home_dir(&self) -> Option<PathBuf> {
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .ok()
    }
}

/// scoped env override + base（`overlayEnvAuthContext` と models.json
/// resolve の `env` 引数 `env?.[name] || process.env[name]` の両方を
/// `overlay.get() || base.env()` で再現する単一 accessor）。
pub struct OverlayEnv<'a> {
    overlay: BTreeMap<String, String>,
    base: &'a dyn ProviderEnv,
}

impl<'a> OverlayEnv<'a> {
    pub fn new(overlay: BTreeMap<String, String>, base: &'a dyn ProviderEnv) -> Self {
        Self { overlay, base }
    }
    /// `resolveEnvConfigValue(name, env)` — overlay 優先、空文字は unset。
    pub fn get(&self, name: &str) -> Option<String> {
        self.overlay
            .get(name)
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| self.base.env(name))
    }
}

/// template part（`parseConfigValueTemplate`）。
#[derive(Debug, Clone, PartialEq)]
enum TemplatePart {
    Literal(String),
    Env(String),
}

const ENV_NAME_RE_OK: fn(char) -> bool = |c| c.is_ascii_alphanumeric() || c == '_';

fn is_env_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_env_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().map(is_env_name_start).unwrap_or(false)
        && s.chars().all(ENV_NAME_RE_OK)
}

/// `parseConfigValueTemplate` の byte index 版（`$`/`{`/`}` は ASCII のため
/// byte 走査で安全）。
fn parse_config_value_template(config: &str) -> Vec<TemplatePart> {
    let mut parts: Vec<TemplatePart> = Vec::new();
    let bytes = config.as_bytes();
    let mut index = 0;
    let push_literal = |parts: &mut Vec<TemplatePart>, s: &str| {
        if s.is_empty() {
            return;
        }
        match parts.last_mut() {
            Some(TemplatePart::Literal(v)) => v.push_str(s),
            _ => parts.push(TemplatePart::Literal(s.to_string())),
        }
    };
    while index < bytes.len() {
        let Some(rel) = bytes[index..].iter().position(|b| *b == b'$') else {
            push_literal(&mut parts, &config[index..]);
            break;
        };
        let dollar = index + rel;
        push_literal(&mut parts, &config[index..dollar]);
        let next = bytes.get(dollar + 1).copied();
        match next {
            Some(c @ (b'$' | b'!')) => {
                push_literal(&mut parts, &(c as char).to_string());
                index = dollar + 2;
            }
            Some(b'{') => {
                let end = config[dollar + 2..].find('}').map(|i| dollar + 2 + i);
                match end {
                    None => {
                        push_literal(&mut parts, "$");
                        index = dollar + 1;
                    }
                    Some(end) => {
                        let name = &config[dollar + 2..end];
                        if is_env_name(name) {
                            parts.push(TemplatePart::Env(name.to_string()));
                        } else {
                            push_literal(&mut parts, &config[dollar..=end]);
                        }
                        index = end + 1;
                    }
                }
            }
            _ => {
                // `$NAME` — prefix regex `[A-Za-z_][A-Za-z0-9_]*`。先頭は
                // alpha/`_`、以降は alnum/`_`。
                let rest = &config[dollar + 1..];
                let mut name = String::new();
                let mut end_byte = dollar + 1;
                let mut chars = rest.char_indices();
                if let Some((_, first)) = chars.next() {
                    if is_env_name_start(first) {
                        name.push(first);
                        end_byte = dollar + 1 + first.len_utf8();
                        for (i, c) in chars {
                            if ENV_NAME_RE_OK(c) {
                                name.push(c);
                                end_byte = dollar + 1 + i + c.len_utf8();
                            } else {
                                break;
                            }
                        }
                    }
                }
                if name.is_empty() {
                    push_literal(&mut parts, "$");
                    index = dollar + 1;
                } else {
                    parts.push(TemplatePart::Env(name));
                    index = end_byte;
                }
            }
        }
    }
    parts
}

/// `parseConfigValueReference` — `"!"` 先頭のみ command。
pub fn is_command_config_value(config: &str) -> bool {
    config.starts_with('!')
}

/// `getConfigValueEnvVarNames` — template 中の env var 名（重複除去・
/// 出現順）。command 値では空。
pub fn get_config_value_env_var_names(config: &str) -> Vec<String> {
    if is_command_config_value(config) {
        return Vec::new();
    }
    let mut names = Vec::new();
    for part in parse_config_value_template(config) {
        if let TemplatePart::Env(name) = part {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// `getMissingConfigValueEnvVarNames`。
pub fn get_missing_config_value_env_var_names(config: &str, env: &OverlayEnv<'_>) -> Vec<String> {
    get_config_value_env_var_names(config)
        .into_iter()
        .filter(|n| env.get(n).is_none())
        .collect()
}

/// `isConfigValueConfigured` — template の全 env var が解決可能か。
/// command は実行せず configured 扱い（check 相当）。
pub fn is_config_value_configured(config: &str, env: &OverlayEnv<'_>) -> bool {
    is_command_config_value(config)
        || get_missing_config_value_env_var_names(config, env).is_empty()
}

fn resolve_template(parts: &[TemplatePart], env: &OverlayEnv<'_>) -> Option<String> {
    let mut resolved = String::new();
    for part in parts {
        match part {
            TemplatePart::Literal(s) => resolved.push_str(s),
            TemplatePart::Env(name) => {
                resolved.push_str(&env.get(name)?);
            }
        }
    }
    Some(resolved)
}

/// command 実行結果の process 内 cache（`commandResultCache`）。
fn command_cache() -> &'static Mutex<BTreeMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<BTreeMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// `executeCommand` — shell 経由で `"!"` 以降を実行し stdout(trim) を返す。
/// timeout 10s、非零 exit・実行不能・空出力は None。stderr は捨てる。
fn execute_command_uncached(command_config: &str) -> Option<String> {
    let command = &command_config[1..];
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };
    let mut child = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    // 10s timeout（`spawnSync`/`execSync` の `timeout: 10000` 相当）。
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut status = None;
    while status.is_none() {
        match child.try_wait() {
            Ok(Some(s)) => status = Some(s),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
    if !status?.success() {
        return None;
    }
    let mut out = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut out).ok();
    let trimmed = out.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn execute_command(command_config: &str) -> Option<String> {
    let mut cache = command_cache().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(v) = cache.get(command_config) {
        return v.clone();
    }
    let result = execute_command_uncached(command_config);
    cache.insert(command_config.to_string(), result.clone());
    result
}

/// `resolveConfigValue` — command は cache 付き実行。
pub fn resolve_config_value(config: &str, env: &OverlayEnv<'_>) -> Option<String> {
    if is_command_config_value(config) {
        return execute_command(config);
    }
    resolve_template(&parse_config_value_template(config), env)
}

/// `resolveConfigValueUncached` — command cache を使わない（header 解決等
/// cache しない経路。TS でも header は uncached 経路のみ参照される）。
fn resolve_config_value_uncached(config: &str, env: &OverlayEnv<'_>) -> Option<String> {
    if is_command_config_value(config) {
        return execute_command_uncached(config);
    }
    resolve_template(&parse_config_value_template(config), env)
}

fn auth_error(message: String) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidModelConfig, message)
}

/// `resolveConfigValueOrThrow` — error message は TS と同じ文面。
pub fn resolve_config_value_or_throw(
    config: &str,
    description: &str,
    env: &OverlayEnv<'_>,
) -> Result<String, CheaperError> {
    if let Some(v) = resolve_config_value_uncached(config, env) {
        return Ok(v);
    }
    if is_command_config_value(config) {
        return Err(auth_error(format!(
            "Failed to resolve {description} from shell command: {}",
            &config[1..]
        )));
    }
    let missing = get_missing_config_value_env_var_names(config, env);
    if missing.len() == 1 {
        return Err(auth_error(format!(
            "Failed to resolve {description} from environment variable: {}",
            missing[0]
        )));
    }
    if missing.len() > 1 {
        return Err(auth_error(format!(
            "Failed to resolve {description} from environment variables: {}",
            missing.join(", ")
        )));
    }
    Err(auth_error(format!("Failed to resolve {description}")))
}

/// `resolveHeadersOrThrow` — template 値を全て解決する（1 つでも未解決
/// なら throw）。
pub fn resolve_headers_or_throw(
    headers: Option<&Map<String, Value>>,
    description: &str,
    env: &OverlayEnv<'_>,
) -> Result<Option<BTreeMap<String, String>>, CheaperError> {
    let Some(headers) = headers else {
        return Ok(None);
    };
    let mut resolved = BTreeMap::new();
    for (key, value) in headers {
        let s = value.as_str().unwrap_or_default();
        resolved.insert(
            key.clone(),
            resolve_config_value_or_throw(s, &format!("{description} header \"{key}\""), env)?,
        );
    }
    Ok(if resolved.is_empty() {
        None
    } else {
        Some(resolved)
    })
}

/// `configContextEnv` — template 値群が参照する env var を base env から
/// 集め explicit overlay に載せる（template resolve の前段）。
pub fn config_context_env(
    values: impl Iterator<Item = String>,
    env: &dyn ProviderEnv,
    explicit: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut out = explicit.clone();
    let mut names: Vec<String> = Vec::new();
    for v in values {
        for n in get_config_value_env_var_names(&v) {
            if !names.contains(&n) {
                names.push(n);
            }
        }
    }
    for name in names {
        if out.contains_key(&name) {
            continue;
        }
        if let Some(value) = env.env(&name) {
            out.insert(name, value);
        }
    }
    out
}

/// 解決後の wire auth（pi-ai `resolution.auth` 相当）。
/// `api_key` は `auth.apiKey`、ambient（vertex ADC / bedrock）では None。
/// `headers` は configured headers 解決済み + `authHeader` 由来 Bearer を
/// 含む最終 request headers。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedAuth {
    pub api_key: Option<String>,
    pub headers: BTreeMap<String, String>,
    /// `auth.baseUrl` — credential 依存の per-credential endpoint
    /// （github-copilot `toAuth` が token の `proxy-ep` claim から導く）。
    /// api_key 経路では `None`（model の catalog `baseUrl` を使う）。
    pub base_url: Option<String>,
    /// 解決元 label（diagnostic のみ。secret は含まない）。
    pub source: AuthSource,
}

/// `resolution.source` 相当（status/log にはこれのみ出す）。
#[derive(Debug, Clone, PartialEq)]
pub enum AuthSource {
    /// `"stored credential"`
    StoredCredential,
    /// `"configured API key"`
    ConfiguredApiKey,
    /// `"OAuth"`
    OAuth,
    /// env var 名（例 `"OPENAI_API_KEY"`）。
    EnvVar(String),
    /// `"ambient credentials"`（vertex ADC / bedrock AWS env 等）。
    Ambient(String),
}

/// `withConfiguredAuth` — configured headers merge + `authHeader` Bearer。
fn with_configured_auth(
    api_key: Option<String>,
    source: AuthSource,
    headers: Option<BTreeMap<String, String>>,
    auth_header: bool,
    provider_id: &str,
) -> Result<ResolvedAuth, CheaperError> {
    let mut merged = headers.unwrap_or_default();
    if auth_header {
        let Some(key) = &api_key else {
            return Err(auth_error(format!(
                "Provider {provider_id}: authHeader requires a resolved API key"
            )));
        };
        merged.insert("Authorization".to_string(), format!("Bearer {key}"));
    }
    Ok(ResolvedAuth {
        api_key,
        headers: merged,
        base_url: None,
        source,
    })
}

/// env table 解決（`inherited.resolve` 相当 — envApiKeyAuth + vertex +
/// bedrock の ambient 分岐）。
fn resolve_env_auth(
    env_auth: EnvAuth,
    env: &dyn ProviderEnv,
) -> Option<(Option<String>, AuthSource)> {
    match env_auth {
        EnvAuth::None => None,
        EnvAuth::Vars(vars) => vars.iter().find_map(|name| {
            env.env(name)
                .map(|v| (Some(v), AuthSource::EnvVar((*name).to_string())))
        }),
        EnvAuth::Anthropic => {
            // `getEnvApiKey`: AUTH_TOKEN が見つかっても request 値には使わず
            // OAUTH_TOKEN / API_KEY を優先する。
            for name in [ANTHROPIC_OAUTH_TOKEN_ENV, ANTHROPIC_API_KEY_ENV] {
                if let Some(v) = env.env(name) {
                    return Some((Some(v), AuthSource::EnvVar(name.to_string())));
                }
            }
            // AUTH_TOKEN は Bearer header 値だが wire 層が同じ扱いのため
            // api_key として返す（configured 判定にも使われる）。
            env.env(ANTHROPIC_AUTH_TOKEN_ENV).map(|v| {
                (
                    Some(v),
                    AuthSource::EnvVar(ANTHROPIC_AUTH_TOKEN_ENV.to_string()),
                )
            })
        }
        EnvAuth::GoogleVertex => {
            if let Some(v) = env.env("GOOGLE_CLOUD_API_KEY") {
                return Some((
                    Some(v),
                    AuthSource::EnvVar("GOOGLE_CLOUD_API_KEY".to_string()),
                ));
            }
            let has_credentials = has_vertex_adc_credentials(&|n| env.env(n), &|| env.home_dir());
            let has_project =
                env.env("GOOGLE_CLOUD_PROJECT").is_some() || env.env("GCLOUD_PROJECT").is_some();
            let has_location = env.env("GOOGLE_CLOUD_LOCATION").is_some();
            if has_credentials && has_project && has_location {
                return Some((None, AuthSource::Ambient("google-vertex ADC".to_string())));
            }
            None
        }
        EnvAuth::AmazonBedrock => {
            if has_bedrock_env_credentials(&|n| env.env(n)) {
                return Some((None, AuthSource::Ambient("amazon-bedrock".to_string())));
            }
            None
        }
    }
}

/// `composeApiKeyAuth(...).check` の port — status/getAvailable の
/// configured 判定。command config は実行しない。
/// `credential` は stored credential（api_key のみ渡す — oauth は
/// service 層で先に分岐する）。
pub fn check_api_key_auth(
    env_auth: EnvAuth,
    configured_key: Option<&str>,
    credential: Option<&StoredCredential>,
    env: &dyn ProviderEnv,
) -> bool {
    if let Some(cred) = credential {
        if cred.as_api_key().is_some() {
            return true;
        }
        // credential 無 key（事実上起きない）→ env fallback。
        return resolve_env_auth(env_auth, env).is_some();
    }
    if let Some(raw_key) = configured_key {
        if is_command_config_value(raw_key) {
            return true;
        }
        let overlay = OverlayEnv::new(BTreeMap::new(), env);
        return get_missing_config_value_env_var_names(raw_key, &overlay).is_empty();
    }
    // `inherited?.check` は builtin envApiKeyAuth に無い → resolve 相当。
    // custom provider の env_auth は `EnvAuth::None`（inherited 無しと等価）。
    resolve_env_auth(env_auth, env).is_some()
}

/// `composeApiKeyAuth(...).resolve` の port — request 用 auth 解決。
/// 呼び出し側は stored oauth credential を api_key resolve へ渡さない
/// （service 層で先に oauth 分岐する — "No silent env fallback"）。
pub fn resolve_api_key_auth(
    provider_id: &str,
    env_auth: EnvAuth,
    configured_key: Option<&str>,
    configured_headers: Option<&Map<String, Value>>,
    auth_header: bool,
    credential: Option<&StoredCredential>,
    env: &dyn ProviderEnv,
) -> Result<Option<ResolvedAuth>, CheaperError> {
    let (api_key, source, credential_env): (Option<String>, AuthSource, BTreeMap<String, String>) =
        if let Some(cred) = credential {
            match cred.as_api_key() {
                Some(key) => (
                    Some(key.to_string()),
                    AuthSource::StoredCredential,
                    BTreeMap::new(),
                ),
                // inherited.resolve: `credential?.key` が無ければ env table。
                None => match resolve_env_auth(env_auth, env) {
                    Some((k, s)) => (k, s, BTreeMap::new()),
                    None => return Ok(None),
                },
            }
        } else if let Some(raw_key) = configured_key {
            // `configContextEnv([rawKey])` → overlay に template var を集めて
            // `resolveConfigValueOrThrow`。
            let explicit =
                config_context_env(std::iter::once(raw_key.to_string()), env, &BTreeMap::new());
            let overlay = OverlayEnv::new(explicit, env);
            let key = resolve_config_value_or_throw(
                raw_key,
                &format!("API key for provider \"{provider_id}\""),
                &overlay,
            )?;
            (Some(key), AuthSource::ConfiguredApiKey, BTreeMap::new())
        } else {
            match resolve_env_auth(env_auth, env) {
                Some((k, s)) => (k, s, BTreeMap::new()),
                None => return Ok(None),
            }
        };
    // configured headers: `configContextEnv(Object.values(rawHeaders),
    // explicitEnv)` → `resolveHeadersOrThrow` → `withConfiguredAuth`。
    let header_values = configured_headers
        .map(|h| {
            h.values()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let explicit_env = config_context_env(header_values.into_iter(), env, &credential_env);
    let overlay = OverlayEnv::new(explicit_env, env);
    let headers = resolve_headers_or_throw(
        configured_headers,
        &format!("provider \"{provider_id}\""),
        &overlay,
    )?;
    Ok(Some(with_configured_auth(
        api_key,
        source,
        headers,
        auth_header,
        provider_id,
    )?))
}
