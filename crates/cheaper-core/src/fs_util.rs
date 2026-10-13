//! crate 間共有の filesystem helper。
//!
//! `cheaper-runtime`(`auth/store.rs`・`catalog/persisted.rs`)と
//! `cheaper-cli`(`policy grant` の user `policy.yaml` write)が共有する
//! temp+rename atomic write。runtime から core へ移動した単一実装
//! (design §(g) — 複製しない)。

use std::io;
use std::path::{Path, PathBuf};

/// `atomic_write` の経路差分 option。
pub struct AtomicWriteOptions {
    /// Windows で rename 前に既存 target を事前 remove する。
    /// `std::fs::rename` は `MoveFileExW`+`MOVEFILE_REPLACE_EXISTING`
    /// semantics で本来不要だが、credentials 保存の継承 workaround は
    /// この option で維持する（persisted catalog・user policy は
    /// `false`）。
    pub windows_remove_first: bool,
    /// unix で write 後に temp file へ適用する permission mode
    /// （例: credentials・policy.yaml の `0o600`）。
    #[cfg(unix)]
    pub unix_mode: Option<u32>,
}

/// temp+rename atomic write。同 dir の temp file
/// （`.<file name>.tmp-<pid>-<nanos>`）へ write → `std::fs::rename` で
/// 置換する。失敗時は temp file を best-effort で remove する。
///
/// parent dir の `create_dir_all` は caller の責務 — dir 作成失敗時の
/// error message が経路ごとの contract になっているため、分離して
/// 各 caller が固有 message を維持する。
/// `auth/store.rs`（credentials）と `catalog/persisted.rs`
/// （plugin catalog）で複製していた同一骨格を集約。
pub fn atomic_write(
    path: &Path,
    body: &str,
    options: &AtomicWriteOptions,
) -> Result<(), io::Error> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let tmp = temp_path(dir, path);
    let result = (|| -> io::Result<()> {
        std::fs::write(&tmp, body)?;
        #[cfg(unix)]
        if let Some(mode) = options.unix_mode {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode))?;
        }
        // Unix: rename が既存 target を原子置換。
        // Windows: 継承 workaround 経路のみ target 存在で rename 失敗を
        // 避けるため事前 remove。
        #[cfg(windows)]
        if options.windows_remove_first && path.exists() {
            std::fs::remove_file(path)?;
        }
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn temp_path(dir: &Path, target: &Path) -> PathBuf {
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    dir.join(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}
