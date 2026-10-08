//! `include_dir!` builtin resource 埋込(G8)と version 解決
//! (`packages/cli/src/version.ts` 相当)。
//!
//! TS SEA 版は `useVfs` で workers/・policy.yaml・version.json を含む
//! VFS root を `builtinDir` として使う。Rust 版は `include_dir!` で
//! バイナリ埋込した resource を、core の filesystem ベース解決
//! (`resolve_worker` が `<dir>/workers/<name>.md` を存在確認 → 読込、
//! worker `.md` の `./schemas/*.json` 相対参照、`load_policy` の
//! `<dir>/policy.yaml`)へそのまま流せるよう、起動時に version 付き
//! temp dir へ materialize する。
//!
//! - materialize 先: `<temp>/cheaper-builtin-<version>/`
//! - `.ready` marker で完全性を判定し、file 単位は tmp→rename で
//!   atomic に置く(同 byte 並行抽出は benign)。
//! - workspace / `.cheaper/` / `<userConfigDir>` には書かない。
//! - override 順序(project → user → builtin)は core 側の責務で不変。

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use include_dir::{include_dir, Dir, DirEntry};

/// `packages/cli/workers/`(`workers/schemas/*.json` を含む)。
static WORKERS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../packages/cli/workers");

/// `packages/cli/policy.yaml`。
const POLICY_YAML: &str = include_str!("../../../packages/cli/policy.yaml");

/// build.rs が `packages/cli/package.json` から生成した `version.json`
/// (SEA bundle の配置と同じ優先順位の第 1 候補)。
const VERSION_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/version.json"));

/// dev checkout 用 fallback(SEA VFS にも同様に配置される)。
const PACKAGE_JSON: &str = include_str!("../../../packages/cli/package.json");

/// `packageVersion`(version.ts)相当の low-level lookup:
/// `version.json` → `package.json` の順に `<dir>/<name>` を読み、
/// `.version` が string なら採用する。
fn version_from_dir(dir: &Path) -> Option<String> {
    for name in ["version.json", "package.json"] {
        let Ok(text) = std::fs::read_to_string(dir.join(name)) else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(v) = parsed.get("version").and_then(serde_json::Value::as_str) {
            return Some(v.to_string());
        }
    }
    None
}

/// 埋込 resource 上での version 解決(materialize 前でも使える
/// in-memory 版)。`version.json` → `package.json`。
fn embedded_version() -> Option<String> {
    for text in [VERSION_JSON, PACKAGE_JSON] {
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) else {
            continue;
        };
        if let Some(v) = parsed.get("version").and_then(serde_json::Value::as_str) {
            return Some(v.to_string());
        }
    }
    None
}

/// `packageVersion(builtinDir)` — `--version` 出力用。
/// `builtinDir` 起点で `version.json` → `package.json` を試し、
/// 両方とも解決できなければ plain error(TS 同様 raw `Error` →
/// INTERNAL_ERROR envelope に畳まれる)。
pub fn package_version(builtin_dir: &Path) -> Result<String, std::io::Error> {
    version_from_dir(builtin_dir)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "failed to resolve package version"))
}

fn write_atomic(dir: &Path, rel: &str, contents: &[u8]) -> io::Result<()> {
    let final_path = dir.join(rel);
    if let Some(parent) = final_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = final_path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, &final_path)?;
    Ok(())
}

fn extract_dir(base: &Path, rel_prefix: &str, dir: &Dir<'static>) -> io::Result<()> {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => {
                write_atomic(
                    base,
                    &format!("{rel_prefix}{}", file.path().to_string_lossy()),
                    file.contents(),
                )?;
            }
            DirEntry::Dir(sub) => {
                // `file.path()` は include root 相対なので nested dir は
                // 再帰せず entries を flatten しても正しい — ただし
                // include_dir の Dir::entries() は直下のみ列挙するため
                // 再帰して全階層を書き出す。
                extract_dir(base, rel_prefix, sub)?;
            }
        }
    }
    Ok(())
}

fn extract_builtin(dir: &Path) -> io::Result<()> {
    extract_dir(dir, "workers/", &WORKERS_DIR)?;
    write_atomic(dir, "policy.yaml", POLICY_YAML.as_bytes())?;
    write_atomic(dir, "version.json", VERSION_JSON.as_bytes())?;
    write_atomic(dir, "package.json", PACKAGE_JSON.as_bytes())?;
    Ok(())
}

/// builtin resource を materialize した dir を返す(process 内 cache)。
/// `CliHooks.builtin_dir` が指定された場合はそちらを使うので、
/// この関数は `hooks.builtin_dir` 無しの default 経路専用。
pub fn builtin_dir() -> Result<PathBuf, io::Error> {
    static CACHE: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let result = CACHE.get_or_init(|| materialize_builtin_dir().map_err(|e| e.to_string()));
    match result {
        Ok(p) => Ok(p.clone()),
        Err(msg) => Err(io::Error::other(msg.clone())),
    }
}

fn materialize_builtin_dir() -> io::Result<PathBuf> {
    let version = embedded_version().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "failed to resolve package version")
    })?;
    let target = std::env::temp_dir().join(format!("cheaper-builtin-{version}"));
    let ready = target.join(".ready");
    if ready.is_file() {
        return Ok(target);
    }
    extract_builtin(&target)?;
    std::fs::write(&ready, b"ready\n")?;
    Ok(target)
}
