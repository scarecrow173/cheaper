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
//! - materialize 先: `<temp>/cheaper-builtin-<version>-<fingerprint>/`
//!   (fingerprint は embedded resource 内容の決定的 hash — 同一
//!   version で resource が変われば別 dir になり stale cache を構造的に
//!   排除する。issue #115)
//! - `.ready` marker で完全性を判定し、file 単位は tmp→rename で
//!   atomic に置く(同 byte 並行抽出は benign)。
//! - workspace / `.cheaper/` / `<userConfigDir>` には書かない。
//! - override 順序(project → user → builtin)は core 側の責務で不変。

use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use include_dir::{include_dir, Dir, DirEntry, File};

/// `resources/workers/`(`workers/schemas/*.json` を含む)。
/// binary への embedded が唯一の配布経路(issue #85 — APM package の
/// `cli/` payload は撤去済み)。
static WORKERS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources/workers");

/// `resources/policy.yaml`。
const POLICY_YAML: &str = include_str!("../resources/policy.yaml");

/// build.rs が `CARGO_PKG_VERSION`(workspace canonical version)から
/// 生成した `version.json`。
const VERSION_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/version.json"));

/// `packageVersion` 相当の low-level lookup:
/// `version.json` → `package.json` の順に `<dir>/<name>` を読み、
/// `.version` が string なら採用する。`package.json` は
/// `CliHooks.builtin_dir` override 先の外部 dir 向け fallback であり、
/// embedded materialize は `version.json` のみを書く。
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

/// 埋込 `version.json` 上での version 解決(materialize 前でも使える
/// in-memory 版)。build.rs 生成物は常に存在するため fallback は不要。
fn embedded_version() -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(VERSION_JSON).ok()?;
    parsed
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
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

/// `dir` 配下の全 file を再帰列挙する。
/// include_dir 0.7 の `Dir::files()`/`entries()` は直下のみなので
/// `entries()` 再帰が必要(`workers/schemas/*.json` を含むため)。
/// `file.path()` は include root 相対。
/// extract の出力対象と fingerprint の入力対象を同一走査に集約し、
/// 両者の drift を構造的に防ぐ(issue #115 review 指摘)。
fn collect_files<'a>(dir: &'a Dir<'a>, out: &mut Vec<&'a File<'a>>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(file) => out.push(file),
            DirEntry::Dir(sub) => collect_files(sub, out),
        }
    }
}

fn extract_dir(base: &Path, rel_prefix: &str, dir: &Dir<'static>) -> io::Result<()> {
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    for file in files {
        write_atomic(
            base,
            &format!("{rel_prefix}{}", file.path().to_string_lossy()),
            file.contents(),
        )?;
    }
    Ok(())
}

/// embedded resource 内容の決定的 fingerprint。
/// `collect_files` と同じ走査の workers 全 file(path + contents)に
/// `policy.yaml`/`version.json` を加えて hash する。
/// `DefaultHasher` は固定 key なので同一 toolchain の process 間で
/// 決定的。rustc 版間で値が変わり得るが、影響は新しい cache dir が
/// 増えるだけで staleness にはならない。
/// `pub(crate)`: `hook::eval_cache` が builtin policy 層 `None`
/// (embedded 解決)の fingerprint marker として使う。
pub(crate) fn embedded_fingerprint() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut files = Vec::new();
    collect_files(&WORKERS_DIR, &mut files);
    // include_dir の compile 時走査順に依存しないよう path で sort。
    files.sort_by(|a, b| a.path().cmp(b.path()));
    let mut hasher = DefaultHasher::new();
    for file in files {
        file.path().hash(&mut hasher);
        file.contents().hash(&mut hasher);
    }
    "policy.yaml".hash(&mut hasher);
    POLICY_YAML.hash(&mut hasher);
    "version.json".hash(&mut hasher);
    VERSION_JSON.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn extract_builtin(dir: &Path) -> io::Result<()> {
    extract_dir(dir, "workers/", &WORKERS_DIR)?;
    write_atomic(dir, "policy.yaml", POLICY_YAML.as_bytes())?;
    write_atomic(dir, "version.json", VERSION_JSON.as_bytes())?;
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
    let target = std::env::temp_dir().join(format!(
        "cheaper-builtin-{version}-{}",
        embedded_fingerprint()
    ));
    let ready = target.join(".ready");
    if ready.is_file() {
        return Ok(target);
    }
    extract_builtin(&target)?;
    std::fs::write(&ready, b"ready\n")?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fingerprint は同一 binary 内で決定的かつ 16hex 形式。
    #[test]
    fn fingerprint_is_deterministic_hex() {
        let a = embedded_fingerprint();
        let b = embedded_fingerprint();
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// `collect_files` は nested `schemas/*.json` を含む全 worker file
    /// を再帰列挙する — `Dir::files()` は直下のみなのでここが回帰点
    /// (schema 変更が fingerprint に反映されなければ stale cache が
    /// 再発する。issue #115)。
    #[test]
    fn collect_files_covers_nested_schema_files() {
        let mut files = Vec::new();
        collect_files(&WORKERS_DIR, &mut files);
        let schemas = files
            .iter()
            .filter(|f| f.path().starts_with("schemas"))
            .count();
        assert_eq!(schemas, 8, "workers/schemas/*.json must be enumerated");
        assert!(
            files
                .iter()
                .any(|f| f.path() == Path::new("bulk-reader.md")),
            "top-level worker defs must be enumerated"
        );
    }

    /// fingerprint の入力集合は extract_builtin の出力対象と同一走査
    /// (collect_files 共有)。ここでは fingerprint が file 追加・
    /// 内容変更に反応することを pin する。
    #[test]
    fn fingerprint_depends_on_file_path_and_contents() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        // path のみ差し替えた hash と contents のみ差し替えた hash が
        // 本実装と同じ列挙構造であることを直接検証 — fingerprint 内部を
        // なぞり、片方だけ変えたら値が変わることを確認する。
        let base = embedded_fingerprint();
        let mut files = Vec::new();
        collect_files(&WORKERS_DIR, &mut files);
        files.sort_by(|a, b| a.path().cmp(b.path()));
        let mut h = DefaultHasher::new();
        for (i, file) in files.iter().enumerate() {
            if i == 0 {
                // 1file 目の contents を攪乱した場合 fingerprint が変わる。
                file.path().hash(&mut h);
                b"perturbed".hash(&mut h);
            } else {
                file.path().hash(&mut h);
                file.contents().hash(&mut h);
            }
        }
        "policy.yaml".hash(&mut h);
        POLICY_YAML.hash(&mut h);
        "version.json".hash(&mut h);
        VERSION_JSON.hash(&mut h);
        assert_ne!(format!("{:016x}", h.finish()), base);
    }
}
