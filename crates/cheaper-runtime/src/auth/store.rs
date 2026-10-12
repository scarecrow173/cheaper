//! Credential store（G2/G4）。
//!
//! `<userConfigDir>/credentials.json`（**pi の `auth.json` とは別の新規
//! schema**）に credential を永続化する。atomic write（temp file +
//! rename）を必須とし、pi 由来の file（`auth.json` / `models.json` /
//! `models-store.json`、`<userConfigDir>/pi/` 以下、`PI_CODING_AGENT_DIR`
//! 参照）は一切読まない。
//!
//! file schema:
//! ```json
//! {
//!   "version": 1,
//!   "providers": {
//!     "<providerId>": { "type": "api_key", "key": "..." } |
//!                     { "type": "oauth", "access": "...", "refresh": "...",
//!                       "expires": 0, "accountId"?, "subscription"?,
//!                       "availableModelIds"? }
//!   }
//! }
//! ```
//!
//! `availableModelIds` は pi-ai `OAuthCredentials.availableModelIds` 相当
//! （github-copilot catalog filter）。extra field は credential kind が
//! 保持したい追加 claim（devin `tomlPath` / `devinHome` / `apiKey` 等）。
//! secret はこの file 以外（status/stdout/diagnostics）へ出さない。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// credential kind（`AuthType` — CLI `auth login --method` と status の
/// `"api_key"`/`"oauth"` 文字列表現に対応）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ApiKey,
    Oauth,
}

impl AuthType {
    /// CLI 入力表現（`api_key`/`api-key`/`apikey`）。
    pub fn parse(s: &str) -> Option<AuthType> {
        match s {
            "api_key" | "api-key" | "apikey" => Some(AuthType::ApiKey),
            "oauth" => Some(AuthType::Oauth),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthType::ApiKey => "api_key",
            AuthType::Oauth => "oauth",
        }
    }
}

/// credential kind 固有の data。TS `oauthCredentials[providerId]` の
/// `OAuthCredentials` union の port + api_key credential を同格化。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialData {
    ApiKey {
        key: String,
    },
    Oauth {
        access: String,
        refresh: String,
        /// epoch ms（pi-ai `expires`）。
        expires: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subscription: Option<String>,
        /// github-copilot catalog filter（`availableModelIds`）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        available_model_ids: Option<Vec<String>>,
        /// provider 固有の追加 claim（devin `tomlPath`/`apiKey` 等）。
        #[serde(default, flatten)]
        extra: Map<String, Value>,
    },
}

/// providerId ごとの永続 credential。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCredential {
    #[serde(flatten)]
    pub data: CredentialData,
    /// credential 保存元 method（status `credentials[]` の `method` field
    /// — TS は `oauthCredentials` 存在をもって oauth method 表示を推測する
    /// だけで明示保存しないが、`auth status` の method 明示に必要なので
    /// 新規 schema 側に残す）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<AuthType>,
}

impl StoredCredential {
    pub fn api_key(key: String) -> Self {
        Self {
            data: CredentialData::ApiKey { key },
            method: Some(AuthType::ApiKey),
        }
    }
    pub fn oauth(data: CredentialData) -> Self {
        Self {
            data,
            method: Some(AuthType::Oauth),
        }
    }
    pub fn auth_type(&self) -> AuthType {
        match &self.data {
            CredentialData::ApiKey { .. } => AuthType::ApiKey,
            CredentialData::Oauth { .. } => AuthType::Oauth,
        }
    }
    /// api key として解決可能なら値を返す（`oauth.getApiKey` 的な
    /// oauth→key 変換は plugin/flow 側の責務で、store は呼ばない）。
    pub fn as_api_key(&self) -> Option<&str> {
        match &self.data {
            CredentialData::ApiKey { key } => Some(key),
            CredentialData::Oauth { .. } => None,
        }
    }
    /// oauth credential の `availableModelIds`（catalog filter 用）。
    pub fn available_model_ids(&self) -> Option<Vec<String>> {
        match &self.data {
            CredentialData::Oauth {
                available_model_ids,
                ..
            } => available_model_ids.clone(),
            CredentialData::ApiKey { .. } => None,
        }
    }
    /// secret を status/diagnostics へ出さないための debug 表現。
    pub fn describe(&self) -> String {
        match &self.data {
            CredentialData::ApiKey { .. } => "api_key".to_string(),
            CredentialData::Oauth { expires, .. } => {
                format!("oauth(expires={expires})")
            }
        }
    }
}

/// credentials file の on-disk schema。`providers` は object map だが
/// file 内の provider 宣言順を `listCredentials` で再現するため
/// `Vec<(id, credential)>` として保持する（TS `oauthCredentials`
/// object の key 順 = 挿入順）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CredentialsFile {
    version: u32,
    #[serde(default, with = "ordered_providers")]
    providers: Vec<(String, StoredCredential)>,
}

/// JSON object `{"<id>": <credential>, ...}` ⇔ 順序保持 `Vec<(String, T)>`。
mod ordered_providers {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_json::{Map, Value};

    use super::StoredCredential;

    pub fn serialize<S: Serializer>(
        providers: &[(String, StoredCredential)],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = Map::with_capacity(providers.len());
        for (id, credential) in providers {
            map.insert(
                id.clone(),
                serde_json::to_value(credential).map_err(serde::ser::Error::custom)?,
            );
        }
        map.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<(String, StoredCredential)>, D::Error> {
        let map = Map::<String, Value>::deserialize(deserializer)?;
        map.into_iter()
            .map(|(id, value)| {
                StoredCredential::deserialize(value)
                    .map(|credential| (id, credential))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

/// `<userConfigDir>/credentials.json` への atomic 永続化 layer。
/// legacy pi file（`auth.json` / `pi/` 以下）の read は一切行わない。
#[derive(Debug, Clone)]
pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    /// `user_config_dir` は `cheaper_core::config::resolve_user_config_dir`
    /// の結果をそのまま受ける（この module 側で解決 policy を複製しない）。
    pub fn new(user_config_dir: &Path) -> Self {
        Self {
            path: user_config_dir.join("credentials.json"),
        }
    }

    /// injectable constructor（test で任意 path を指せるように）。
    pub fn at_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read_file(&self) -> Result<Vec<(String, StoredCredential)>, CheaperError> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => {
                let parsed: CredentialsFile = serde_json::from_str(&raw).map_err(|e| {
                    let mut details = Map::new();
                    details.insert(
                        "file".to_string(),
                        Value::from(self.path.display().to_string()),
                    );
                    CheaperError::new(
                        ErrorCode::InternalError,
                        format!("credentials.json が壊れています: {e}"),
                    )
                    .with_details(details)
                })?;
                Ok(parsed.providers)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(self.io_error("credentials.json の読み込みに失敗しました", e)),
        }
    }

    /// atomic write: 同 directory の temp file へ書き、`rename` で置換する。
    /// Unix の `rename(2)` は既存 target を原子置換するため remove しない
    /// （F12 — remove→rename では crash window に credentials.json が
    /// 消える）。Windows の `MoveFile` 系は target 存在で失敗するため
    /// そちらのみ事前 remove を維持する。
    fn write_file(&self, providers: &[(String, StoredCredential)]) -> Result<(), CheaperError> {
        let dir = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(dir)
            .map_err(|e| self.io_error("credential 保存 directory の作成に失敗しました", e))?;
        let file = CredentialsFile {
            version: 1,
            providers: providers.to_vec(),
        };
        let body = serde_json::to_string_pretty(&file).expect("credentials serialize");
        // Windows: target 存在で rename 失敗するため事前 remove を維持
        // （persisted catalog 経路とは option で差分を吸収）。
        crate::fs_util::atomic_write(
            &self.path,
            &body,
            &crate::fs_util::AtomicWriteOptions {
                windows_remove_first: true,
                #[cfg(unix)]
                unix_mode: Some(0o600),
            },
        )
        .map_err(|e| self.io_error("credentials.json の保存に失敗しました", e))
    }

    fn io_error(&self, message: &str, e: io::Error) -> CheaperError {
        let mut details = Map::new();
        details.insert(
            "file".to_string(),
            Value::from(self.path.display().to_string()),
        );
        details.insert("ioError".to_string(), Value::from(e.to_string()));
        CheaperError::new(ErrorCode::InternalError, format!("{message}: {e}")).with_details(details)
    }

    /// `cheaper.auth.store` event の `file.name` field — basename のみ
    /// （full path は `<userConfigDir>` 経由で OS username を含み得る
    /// ため metadata-only 規約の allowlist 外）。
    fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// `cheaper.auth.store` event(design §instrumentation 配置)—
    /// load/save op 失敗時の warn emit。credential 値・error message
    /// （full path を含み得る）は載せず `error.code` のみ。
    fn warn_op(&self, op: &'static str, provider: Option<&str>, e: &CheaperError) {
        match provider {
            Some(provider) => tracing::warn!(
                target: "cheaper.auth.store",
                op = op,
                provider = provider,
                file.name = self.file_name().as_str(),
                error.code = e.code().as_str(),
                "credential store operation failed"
            ),
            None => tracing::warn!(
                target: "cheaper.auth.store",
                op = op,
                file.name = self.file_name().as_str(),
                error.code = e.code().as_str(),
                "credential store operation failed"
            ),
        }
    }

    pub fn get(&self, provider_id: &str) -> Result<Option<StoredCredential>, CheaperError> {
        let result = self.read_file().map(|providers| {
            providers
                .into_iter()
                .find(|(id, _)| id == provider_id)
                .map(|(_, credential)| credential)
        });
        match &result {
            Ok(credential) => tracing::debug!(
                target: "cheaper.auth.store",
                op = "load",
                provider = provider_id,
                file.name = self.file_name().as_str(),
                hit = credential.is_some(),
                "credential store load"
            ),
            Err(e) => self.warn_op("load", Some(provider_id), e),
        }
        result
    }

    pub fn set(&self, provider_id: &str, credential: StoredCredential) -> Result<(), CheaperError> {
        let result = (|| {
            let mut providers = self.read_file()?;
            match providers.iter_mut().find(|(id, _)| id == provider_id) {
                Some(slot) => slot.1 = credential,
                None => providers.push((provider_id.to_string(), credential)),
            }
            self.write_file(&providers)
        })();
        match &result {
            Ok(()) => tracing::debug!(
                target: "cheaper.auth.store",
                op = "save",
                provider = provider_id,
                file.name = self.file_name().as_str(),
                "credential store save"
            ),
            Err(e) => self.warn_op("save", Some(provider_id), e),
        }
        result
    }

    /// `auth logout` / `auth logout --all` — credential のみを削除し、
    /// models.json・settings 等の非 credential file は触れない。
    /// plugin logout hook の呼び出しは auth service 層の責務。
    pub fn remove(&self, provider_id: &str) -> Result<bool, CheaperError> {
        let result = (|| {
            let mut providers = self.read_file()?;
            let before = providers.len();
            providers.retain(|(id, _)| id != provider_id);
            let removed = providers.len() != before;
            if removed || self.path.exists() {
                self.write_file(&providers)?;
            }
            Ok(removed)
        })();
        match &result {
            Ok(removed) => tracing::debug!(
                target: "cheaper.auth.store",
                op = "save",
                provider = provider_id,
                file.name = self.file_name().as_str(),
                removed = *removed,
                "credential store save"
            ),
            Err(e) => self.warn_op("save", Some(provider_id), e),
        }
        result
    }

    /// `auth logout --all` — file 全体を空にする（plugin hook 呼び出しは
    /// service 層）。
    pub fn clear(&self) -> Result<Vec<String>, CheaperError> {
        let result = (|| {
            let providers = self.read_file()?;
            let ids: Vec<String> = providers.iter().map(|(id, _)| id.clone()).collect();
            if !ids.is_empty() {
                self.write_file(&[])?;
            }
            Ok(ids)
        })();
        match &result {
            Ok(ids) => tracing::debug!(
                target: "cheaper.auth.store",
                op = "save",
                file.name = self.file_name().as_str(),
                provider.count = ids.len(),
                "credential store save"
            ),
            Err(e) => self.warn_op("save", None, e),
        }
        result
    }

    /// status 用の全 credential（file 内の宣言順。secret 自体を含むので
    /// caller は status payload へ secret を展開しないこと）。
    pub fn all(&self) -> Result<Vec<(String, StoredCredential)>, CheaperError> {
        let result = self.read_file();
        match &result {
            Ok(providers) => tracing::debug!(
                target: "cheaper.auth.store",
                op = "load",
                file.name = self.file_name().as_str(),
                provider.count = providers.len(),
                "credential store load"
            ),
            Err(e) => self.warn_op("load", None, e),
        }
        result
    }
}
