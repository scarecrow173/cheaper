//! Devin CLI credential 読み取り + login flow（TS
//! `providers/devin/credentials.ts` の port。upstream pi-devin（MIT）
//! `src/credentials.ts` を、global `~/.local/share/devin` ではなく
//! cheaper data dir のみ読む形に改変したものの Rust 版 —
//! ADR-0029 相当。global credential・Devin Desktop credential は
//! 暗黙利用しない）。

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::cli::{CliError, DevinCli};

/// `auth status` / `auth logout` の timeout（TS 15_000ms）。
const AUTH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// `DevinCredentials` — credentials.toml の parse 結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevinCredentials {
    pub api_key: String,
    /// trailing slash 除去済み（既定 `https://server.codeium.com`）。
    pub api_server_url: String,
    /// 既定 `app.devin.ai`。
    pub webapp_host: String,
    /// 既定 `https://api.devin.ai`。
    pub api_url: String,
    /// 読み取り元 path（`<dataDir>/devin/credentials.toml`）。
    pub path: PathBuf,
}

/// `devinDataDir` — `<userConfigDir>/providers/devin/data`。
/// child env の `XDG_DATA_HOME` にそのまま渡す。
pub fn devin_data_dir(user_config_dir: &Path) -> PathBuf {
    user_config_dir.join("providers").join("devin").join("data")
}

/// `devinCredentialsPath` — `XDG_DATA_HOME=<dataDir>` の Devin CLI が
/// 書く credential path（`<dataDir>/devin/credentials.toml`）。
pub fn devin_credentials_path(data_dir: &Path) -> PathBuf {
    data_dir.join("devin").join("credentials.toml")
}

/// `parseTomlStrings` — `key = "value"` の simple 行のみ抽出
/// （TS 正規表現 `^\s*([A-Za-z0-9_]+)\s*=\s*"(.*)"\s*$` の port）。
/// 同一 key の後勝ちは map への insert 順序で再現。
fn parse_toml_strings(text: &str) -> std::collections::BTreeMap<String, String> {
    static LINE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\s*([A-Za-z0-9_]+)\s*=\s*"(.*)"\s*$"#).expect("toml line regex")
    });
    let mut out = std::collections::BTreeMap::new();
    for line in text.split('\n') {
        if let Some(caps) = LINE.captures(line) {
            out.insert(caps[1].to_string(), caps[2].to_string());
        }
    }
    out
}

/// `readDevinCredentials` — cheaper data dir 内の credentials.toml
/// だけを読む。global path（`~/.local/share/devin` 等）は参照しない。
pub fn read_devin_credentials(data_dir: &Path) -> Option<DevinCredentials> {
    let cred_path = devin_credentials_path(data_dir);
    if !cred_path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&cred_path).ok()?;
    let entries = parse_toml_strings(&text);
    let get = |key: &str| entries.get(key).cloned();
    let api_key = get("windsurf_api_key").or_else(|| get("api_key"))?;
    Some(DevinCredentials {
        api_key,
        api_server_url: {
            // TS `.replace(/\/$/,"")` — trailing slash は 1 個だけ除去。
            let url =
                get("api_server_url").unwrap_or_else(|| "https://server.codeium.com".to_string());
            url.strip_suffix('/').unwrap_or(&url).to_string()
        },
        webapp_host: get("devin_webapp_host").unwrap_or_else(|| "app.devin.ai".to_string()),
        api_url: get("devin_api_url").unwrap_or_else(|| "https://api.devin.ai".to_string()),
        path: cred_path,
    })
}

/// `devinAuthStatus` の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevinAuthStatus {
    pub logged_in: bool,
    pub summary: String,
}

/// `devinAuthStatus` — `devin auth status` の CLI 判定を
/// authoritative にし、CLI 未到達時のみ credentials.toml 存在へ
/// fallback（TS PR #30 review の semantics を維持）。
pub fn devin_auth_status(cli: &DevinCli, data_dir: &Path) -> DevinAuthStatus {
    let creds = read_devin_credentials(data_dir);
    match cli.run(&["auth", "status"], Some(AUTH_TIMEOUT), false) {
        Ok(result) => {
            let text = format!("{}\n{}", result.stdout, result.stderr)
                .trim()
                .to_string();
            static LOGGED_IN: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"(?i)logged in").expect("status regex"));
            DevinAuthStatus {
                logged_in: result.code == 0 && LOGGED_IN.is_match(&text),
                summary: if text.is_empty() {
                    if creds.is_some() {
                        "credentials.toml present".to_string()
                    } else {
                        "not signed in".to_string()
                    }
                } else {
                    text
                },
            }
        }
        Err(error) => {
            if creds.is_some() {
                DevinAuthStatus {
                    logged_in: true,
                    summary: format!(
                        "Devin credentials present at {}",
                        devin_credentials_path(data_dir).display()
                    ),
                }
            } else {
                DevinAuthStatus {
                    logged_in: false,
                    summary: error.to_string(),
                }
            }
        }
    }
}

/// `loginWithDevinCli` — `devin auth status` が logged-in なら既存
/// credential を再利用し、そうでなければ `devin auth login` を
/// inherit stdio + isolated env で実行してから credentials.toml を
/// 再読する。
pub fn login_with_devin_cli(cli: &DevinCli, data_dir: &Path) -> Result<DevinCredentials, CliError> {
    // credentials.toml が存在し、`devin auth status` が logged-in と
    // 答える場合のみ既存 credential を再利用する（stale なら login）。
    if let Some(existing) = read_devin_credentials(data_dir) {
        if devin_auth_status(cli, data_dir).logged_in {
            return Ok(existing);
        }
    }

    let result = cli.run(&["auth", "login"], None, true)?;
    match read_devin_credentials(data_dir) {
        Some(creds) => Ok(creds),
        None => Err(CliError::Io(std::io::Error::other(format!(
            "`devin auth login` {} but {} is missing. \
             Run `devin auth login` with XDG_DATA_HOME={} yourself, \
             then retry `cheaper auth login devin`.",
            if result.code == 0 {
                "finished".to_string()
            } else {
                format!("exited {}", result.code)
            },
            devin_credentials_path(data_dir).display(),
            data_dir.display(),
        )))),
    }
}

/// `logoutDevinSession` — `devin auth logout`（best-effort）+ cheaper
/// owned credentials.toml の削除（best-effort）。CLI 未到達・削除失敗
/// でも error にしない（runtime 側の credential 削除は別途行われる）。
pub fn logout_devin_session(cli: &DevinCli, data_dir: &Path) {
    let _ = cli.run(&["auth", "logout"], Some(AUTH_TIMEOUT), false);
    let _ = std::fs::remove_file(devin_credentials_path(data_dir));
}
