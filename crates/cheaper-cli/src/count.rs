//! Port of `packages/cli/src/count.ts` — `count-tokens` command と
//! `cheaper hook` routing policy が共有する workspace file token 計測経路
//! (DESIGN-0012)。
//!
//! Rust 版は同期実装(core は std::fs ベース)。error は呼び出し側
//! (cli.rs の envelope 経路 / hook routing)が context に応じて
//! CheaperError または routing verdict へ変換する — そのため
//! `CountTokensError` enum ではなく内側の concrete error
//! (`FileTooLargeError`/`WorkspaceEscapeError`/`SpecialTokenError`/
//! `NodeIoError`/`CheaperError`)をそのまま box して返す
//! (`downcast_ref` で TS の `instanceof` 分岐を再現するため)。

use std::error::Error;
use std::path::Path;

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirsOptions, EnvLookup};
use cheaper_core::nodepath;
use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_core::token::count::{count_file_tokens, CountFileTokensOptions, FileTokenCount};
use cheaper_core::workspace::paths::canonicalize_workspace_root;

/// `Box<dyn Error>` — run/CLI 経路の共通 error 畳み込み型
/// (`cli.rs::BoxError` と同型)。
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// `CHEAPER_COUNT_MAX_BYTES` の解釈(DESIGN-0017)。
/// 正の整数のみ有効: trim 後が 10 進数字列のみで構成され、
/// `Number.isSafeInteger` かつ `n > 0` を満たす値だけを受け付ける。
/// `parseInt` は使わない(`16MB` → 16 のような部分 parse を許容しない)。
/// 不正値・未設定は core の default に任せるため `None` を返す。
pub fn count_max_bytes(env: EnvLookup<'_>) -> Option<u64> {
    let raw = env("CHEAPER_COUNT_MAX_BYTES")?;
    let trimmed = cheaper_core::js::js_trim(&raw);
    if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // JS `Number(trimmed)`: 先頭 0 や巨大桁数を含めて f64 へ。
    let n: f64 = trimmed.parse().ok()?;
    if n.fract() == 0.0 && n > 0.0 && n <= cheaper_core::js::MAX_SAFE_INTEGER {
        Some(n as u64)
    } else {
        None
    }
}

/// `WorkspaceFileCount`。
pub struct WorkspaceFileCount {
    pub count: FileTokenCount,
    /// canonicalize 済み workspace root。
    pub root: String,
    pub user_config_dir: std::path::PathBuf,
}

/// `countWorkspaceFileTokens` — canonicalize workspace root → config dirs
/// → policy(symlinks のみ使用)→ `countFileTokens`。
pub fn count_workspace_file_tokens(
    cwd: &str,
    path: &str,
    builtin_dir: &Path,
    env: EnvLookup<'_>,
) -> Result<WorkspaceFileCount, BoxError> {
    let root = canonicalize_workspace_root(cwd).map_err(BoxError::from)?;
    let dirs = resolve_config_dirs(&ConfigDirsOptions {
        cwd: Path::new(&root),
        builtin_dir,
        env: Some(env),
        homedir: None,
    });
    let user_config_dir = dirs.user_config_dir.to_string_lossy().into_owned();
    let project_dir = dirs.project_dir.to_string_lossy().into_owned();
    let builtin_policy_path = nodepath::join(&[&builtin_dir.to_string_lossy(), "policy.yaml"]);
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path,
        user_config_dir: &user_config_dir,
        project_dir: &project_dir,
    })
    .map_err(BoxError::from)?;
    let max_bytes = count_max_bytes(env);
    let count = count_file_tokens(&CountFileTokensOptions {
        root: &root,
        path,
        symlink_policy: policy.filesystem.symlinks,
        max_bytes,
    })
    .map_err(|e| -> BoxError {
        // enum ではなく内側の concrete error を box して返す —
        // cli.rs が downcast_ref で TS `instanceof` 分岐を再現するため。
        use cheaper_core::token::count::CountTokensError as E;
        match e {
            E::Escape(inner) => Box::new(inner),
            E::TooLarge(inner) => Box::new(inner),
            E::SpecialToken(inner) => Box::new(inner),
            E::Io(inner) => Box::new(inner),
        }
    })?;
    Ok(WorkspaceFileCount {
        count,
        root,
        user_config_dir: dirs.user_config_dir,
    })
}
