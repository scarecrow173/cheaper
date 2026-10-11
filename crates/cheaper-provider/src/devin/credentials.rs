//! Devin CLI credential 読み取り + login flow（TS
//! `providers/devin/credentials.ts` の port。upstream pi-devin（MIT）
//! `src/credentials.ts` を、global `~/.local/share/devin` ではなく
//! cheaper data dir のみ読む形に改変したものの Rust 版 —
//! ADR-0029 相当。global credential・Devin Desktop credential は
//! 暗黙利用しない。Windows contract（ADR-034D9BG9UKuUaVNeCIKfNo）
//! では global toml を login 時の明示的な snapshot source としてのみ
//! 参照する — API key の供給源は常に cheaper-owned toml。

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use cheaper_runtime::auth::AuthInteraction;
use regex::Regex;

use crate::devin::cli::{CliError, DevinCli};

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
    /// 読み取り元 path。cheaper toml は
    /// `<dataDir>/devin/credentials.toml`、global parse（Windows
    /// contract の freshness 判定用）は解決済みの global path。
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

/// 任意 path の credentials.toml を parse する共通実装 —
/// cheaper toml と global toml の双方に使う（global 側は
/// Windows contract の freshness 判定専用）。
///
/// credential load ok/miss の `cheaper.provider.devin` event
/// （design §instrumentation 配置）— credential 値は一切載せない。
pub fn read_credentials_at(cred_path: &Path) -> Option<DevinCredentials> {
    let creds = read_credentials_impl(cred_path);
    if creds.is_some() {
        tracing::debug!(target: "cheaper.provider.devin", "credential load ok");
    } else {
        tracing::debug!(target: "cheaper.provider.devin", "credential load miss");
    }
    creds
}

fn read_credentials_impl(cred_path: &Path) -> Option<DevinCredentials> {
    if !cred_path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(cred_path).ok()?;
    // UTF-8 BOM は `^\s*` にマッチしないため先に剥がす（Windows で
    // 手編集された global toml が parse 不能になるのを防ぐ）。
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let entries = parse_toml_strings(text);
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
        path: cred_path.to_path_buf(),
    })
}

/// `readDevinCredentials` — cheaper data dir 内の credentials.toml
/// だけを読む。global path（`~/.local/share/devin` 等）は参照しない。
pub fn read_devin_credentials(data_dir: &Path) -> Option<DevinCredentials> {
    read_credentials_at(&devin_credentials_path(data_dir))
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

/// Windows contract — Devin CLI 3000.x が実際に使う global
/// credential path を `devin auth status` 出力の `File:` 行から解決
/// する。exit code 非 `0`（logged-out）でも出力に `File:` 行があれば
/// 採用し、複数一致は最後を取る。値が空・`credentials.toml` 以外を
/// 指す `File:` 行は credential path ではないため採用しない。
/// parse 不能・CLI 未到達時は `APPDATA/devin/credentials.toml` に
/// fallback、それも無ければ `None`。
pub fn resolve_global_credentials_path(cli: &DevinCli, appdata: Option<&str>) -> Option<PathBuf> {
    static FILE_LINE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\s*File:\s*(.+?)\s*$").expect("file line regex"));
    if let Ok(result) = cli.run(&["auth", "status"], Some(AUTH_TIMEOUT), false) {
        let text = format!("{}\n{}", result.stdout, result.stderr);
        if let Some(path) = FILE_LINE
            .captures_iter(&text)
            .filter_map(|caps| {
                let path = PathBuf::from(caps[1].trim());
                // 空値・credentials.toml 以外を指す File: 行は別用途
                // の出力とみなし fallback 側へ回す（bogus な File: 値が
                // fallback を抑止・別 file を snapshot するのを防ぐ）。
                (path.file_name() == Some(std::ffi::OsStr::new("credentials.toml"))).then_some(path)
            })
            .last()
        {
            return Some(path);
        }
    }
    appdata
        .filter(|dir| !dir.trim().is_empty())
        .map(|dir| Path::new(dir).join("devin").join("credentials.toml"))
}

/// global toml → cheaper toml の atomic copy（`catalog/persisted.rs`
/// convention: 同 dir の一意 temp へ copy → `fs::rename` 置換。
/// `fs::rename` は Windows でも `MOVEFILE_REPLACE_EXISTING` で既存
/// target を置換するため remove-first workaround は踏襲しない）。
fn copy_credentials_atomic(src: &Path, dest: &Path) -> std::io::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination has no parent",
        )
    })?;
    std::fs::create_dir_all(parent)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = parent.join(format!(
        ".credentials.toml.tmp-{}-{nanos}",
        std::process::id()
    ));
    let result = std::fs::copy(src, &tmp).and_then(|_| std::fs::rename(&tmp, dest));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map(|_| ())
}

/// `loginWithDevinCli` — `devin auth status` が logged-in なら既存
/// credential を再利用し、そうでなければ `devin auth login` を
/// inherit stdio + isolated env で実行してから credentials.toml を
/// 再読する。
///
/// Windows contract（`windows` flag）: Devin CLI 3000.x は env
/// redirect を無視して global `%APPDATA%\devin\credentials.toml` に
/// 書くため、global toml との `api_key` 一致でのみ reuse し、
/// `devin auth login` exit `0` 後に global toml を cheaper data dir
/// へ atomic copy（snapshot）する。global session への副作用は
/// `devin auth login` 自体のみ。
pub fn login_with_devin_cli(
    cli: &DevinCli,
    data_dir: &Path,
    windows: bool,
    appdata: Option<&str>,
    interaction: &mut dyn AuthInteraction,
) -> Result<DevinCredentials, CliError> {
    if windows {
        return login_windows_snapshot(cli, data_dir, appdata, interaction);
    }
    // 非 Windows — 現行 contract 不変。credentials.toml が存在し、
    // `devin auth status` が logged-in と答える場合のみ既存
    // credential を再利用する（stale なら login）。
    if let Some(existing) = read_devin_credentials(data_dir) {
        if devin_auth_status(cli, data_dir).logged_in {
            return Ok(existing);
        }
    }

    let _ = interaction.notify("Launching `devin auth login` (Devin CLI, isolated data dir).");
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

/// Windows login contract — global toml を freshness 判定に使い、
/// login 成功後に cheaper data dir へ snapshot する。
fn login_windows_snapshot(
    cli: &DevinCli,
    data_dir: &Path,
    appdata: Option<&str>,
    interaction: &mut dyn AuthInteraction,
) -> Result<DevinCredentials, CliError> {
    let cheaper = read_devin_credentials(data_dir);
    // `auth status` の spawn は `File:` parse のこの 1 回のみ
    // （reuse 判定に `devin_auth_status` は使わない — global
    // session ではなく file 上の key 一致で判定する）。
    let global_path = resolve_global_credentials_path(cli, appdata);
    let global = global_path.as_deref().and_then(read_credentials_at);
    match (&cheaper, &global) {
        (Some(c), Some(g)) if c.api_key == g.api_key => return Ok(c.clone()),
        // global も parse できる場合のみ「別 account」が確定するため
        // ここでだけ告知する（global 不在・parse 不可では出さない）。
        (Some(_), Some(_)) => {
            let _ = interaction.notify(
                "Devin: cheaper credential is for a different account than the global \
                 Devin session. `devin auth login` will update the global session or \
                 adopt the existing one.",
            );
        }
        _ => {}
    }
    let _ = interaction.notify(
        "Launching `devin auth login` (Devin CLI; on Windows credentials are written \
         to the global Devin dir and snapshotted into cheaper).",
    );
    let result = cli.run(&["auth", "login"], None, true)?;
    if result.code != 0 {
        return Err(CliError::Io(std::io::Error::other(format!(
            "`devin auth login` exited {}",
            result.code
        ))));
    }
    // login 前に解決済みの path を再利用（`auth status` の 2 度目の
    // spawn はしない — 別 path へ fallback し直して誤検知するのを防ぐ）。
    let global_path = global_path.ok_or_else(|| {
        CliError::Io(std::io::Error::other(
            "`devin auth login` exited 0 but the global Devin credentials path could \
             not be resolved (`devin auth status` reported no File: line and APPDATA \
             is unset).",
        ))
    })?;
    if !global_path.is_file() {
        return Err(CliError::Io(std::io::Error::other(format!(
            "`devin auth login` exited 0 but no credentials file appeared at {}.",
            global_path.display()
        ))));
    }
    let dest = devin_credentials_path(data_dir);
    copy_credentials_atomic(&global_path, &dest).map_err(|e| {
        CliError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "failed to snapshot {} -> {}: {e}",
                global_path.display(),
                dest.display()
            ),
        ))
    })?;
    read_credentials_at(&dest).ok_or_else(|| {
        CliError::Io(std::io::Error::other(format!(
            "copied credentials at {} failed to parse (api_key missing)",
            dest.display()
        )))
    })
}

/// `logoutDevinSession` — cheaper owned credentials.toml の削除
/// （best-effort）。CLI 未到達・削除失敗でも error にしない
/// （runtime 側の credential 削除は別途行われる）。
///
/// 非 Windows: `devin auth logout`（best-effort）を spawn してから
/// toml を削除する。
/// Windows contract: Devin CLI 3000.x は global credential を使うため
/// `devin auth logout` は spawn せず（global session を破壊しない）、
/// cheaper snapshot のみ削除して global session 継続を notify する。
pub fn logout_devin_session(
    cli: &DevinCli,
    data_dir: &Path,
    windows: bool,
    interaction: &mut dyn AuthInteraction,
) {
    if !windows {
        let _ = cli.run(&["auth", "logout"], Some(AUTH_TIMEOUT), false);
        let _ = std::fs::remove_file(devin_credentials_path(data_dir));
        return;
    }
    let _ = std::fs::remove_file(devin_credentials_path(data_dir));
    let _ = interaction.notify(
        "Removed only the cheaper Devin credential snapshot. The global Devin session \
         remains logged in; run `devin auth logout` yourself to sign out completely.",
    );
}
