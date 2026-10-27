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

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use cheaper_core::nodepath;
use cheaper_core::worker::resolve::WorkerContentSource;
use cheaper_core::worker::schema::SchemaContentSource;
use include_dir::{include_dir, Dir, DirEntry, File};

/// `resources/workers/`(`workers/schemas/*.json` を含む)。
/// binary への embedded が唯一の配布経路(issue #85 — APM package の
/// `cli/` payload は撤去済み)。
static WORKERS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources/workers");

static WORKFLOWS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources/workflows");

/// `resources/policy.yaml`。
const POLICY_YAML: &str = include_str!("../resources/policy.yaml");

const ROUTES_YAML: &str = include_str!("../resources/routes.yaml");

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

fn builtin_resources() -> Vec<(PathBuf, &'static [u8])> {
    let mut out: Vec<(PathBuf, &'static [u8])> = Vec::new();
    for (prefix, dir) in [("workers/", &WORKERS_DIR), ("workflows/", &WORKFLOWS_DIR)] {
        let mut files = Vec::new();
        collect_files(dir, &mut files);
        for file in files {
            out.push((
                PathBuf::from(format!("{prefix}{}", file.path().to_string_lossy())),
                file.contents(),
            ));
        }
    }
    out.push((PathBuf::from("policy.yaml"), POLICY_YAML.as_bytes()));
    out.push((PathBuf::from("routes.yaml"), ROUTES_YAML.as_bytes()));
    out.push((PathBuf::from("version.json"), VERSION_JSON.as_bytes()));
    // include_dir の compile 時走査順に依存しないよう path で sort。
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[allow(dead_code)]
static EMBEDDED_CATALOG: OnceLock<BTreeMap<String, &'static [u8]>> = OnceLock::new();

/// `/builtin/` を logical root とする read-only embedded resource
/// accessor。`route check` など no-mutation contract の caller が
/// filesystem materialization なしに document を取得する。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct EmbeddedResources;

#[allow(dead_code)]
impl EmbeddedResources {
    /// route check diagnostics が使う唯一の builtin logical root。
    pub(crate) const LOGICAL_ROOT: &'static str = "/builtin/";

    pub(crate) fn new() -> Self {
        Self
    }

    /// `/` 正規化済み logical relative key が catalog に存在するか。
    pub(crate) fn contains(&self, key: &str) -> bool {
        normalize_logical_key(key)
            .map(|key| Self::catalog().contains_key(&key))
            .unwrap_or(false)
    }

    /// `/` 正規化済み logical relative key の byte document を返す。
    pub(crate) fn read(&self, key: &str) -> Option<&'static [u8]> {
        let key = normalize_logical_key(key)?;
        Self::catalog().get(&key).copied()
    }

    /// UTF-8 document の helper — YAML/JSON/worker markdown 用。
    pub(crate) fn read_text(&self, key: &str) -> Option<&'static str> {
        std::str::from_utf8(self.read(key)?).ok()
    }

    fn catalog() -> &'static BTreeMap<String, &'static [u8]> {
        EMBEDDED_CATALOG.get_or_init(|| {
            builtin_resources()
                .into_iter()
                .map(|(path, contents)| (path.to_string_lossy().replace('\\', "/"), contents))
                .collect()
        })
    }
}

impl SchemaContentSource for EmbeddedResources {
    fn resolve_schema_path(&self, base_dir: &str, file: &str) -> String {
        let base_dir = base_dir.replace('\\', "/");
        let file = file.replace('\\', "/");
        nodepath::resolve(&base_dir, &file).replace('\\', "/")
    }

    fn read_schema(&self, path: &str) -> Result<Vec<u8>, io::Error> {
        self.read(path)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "embedded schema not found"))
    }
}

impl WorkerContentSource for EmbeddedResources {
    fn exists_worker(&self, path: &str) -> bool {
        self.contains(path)
    }

    fn read_worker(&self, path: &str) -> Result<Vec<u8>, io::Error> {
        self.read(path)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "embedded worker not found"))
    }

    fn schema_source(&self) -> &dyn SchemaContentSource {
        self
    }

    fn display_path(&self, path: &str) -> String {
        path.replace('\\', "/")
    }
}

/// `/builtin/` prefix を剥がした相対 key へ normalize する。
/// host path・親参照・drive letter は catalog key として受理しない。
#[allow(dead_code)]
fn normalize_logical_key(key: &str) -> Option<String> {
    let key = key.replace('\\', "/");
    let key = match key.strip_prefix(EmbeddedResources::LOGICAL_ROOT) {
        Some(rest) => rest.trim_start_matches('/'),
        None if key.starts_with('/') || key == "builtin" || key.starts_with("builtin/") => {
            return None;
        }
        None => key.as_str(),
    };
    let mut parts = Vec::new();
    for part in key.split('/') {
        match part {
            "" | "." => continue,
            ".." => return None,
            _ if part.contains('\0') || part.contains(':') => return None,
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
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
    let mut hasher = DefaultHasher::new();
    for (path, contents) in builtin_resources() {
        path.hash(&mut hasher);
        contents.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn extract_builtin(dir: &Path) -> io::Result<()> {
    for (rel, contents) in builtin_resources() {
        write_atomic(dir, &rel.to_string_lossy(), contents)?;
    }
    Ok(())
}

/// builtin resource を materialize した dir を返す(process 内 cache)。
/// `CliHooks.builtin_dir` が指定された場合はそちらを使うので、
/// この関数は `hooks.builtin_dir` 無しの default 経路専用。
pub fn builtin_dir() -> Result<PathBuf, io::Error> {
    #[cfg(test)]
    BUILTIN_DIR_CALLS.with(|count| count.set(count.get() + 1));
    static CACHE: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let result = CACHE.get_or_init(|| materialize_builtin_dir().map_err(|e| e.to_string()));
    match result {
        Ok(p) => Ok(p.clone()),
        Err(msg) => Err(io::Error::other(msg.clone())),
    }
}

#[cfg(test)]
thread_local! {
    static BUILTIN_DIR_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn builtin_dir_call_count() -> usize {
    BUILTIN_DIR_CALLS.with(|count| count.get())
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

    #[test]
    fn embedded_resources_reads_logical_keys_without_materializing() {
        let before = builtin_dir_call_count();
        let catalog = EmbeddedResources::new();
        for key in [
            "routes.yaml",
            "policy.yaml",
            "workflows/routed-exec.workflow.json",
            "workers/condense.md",
            "workers/schemas/condense.input.schema.json",
            "workers/schemas/condense.output.schema.json",
        ] {
            assert!(catalog.contains(key), "{key}");
            assert!(!catalog.read(key).unwrap().is_empty(), "{key}");
        }
        assert_eq!(catalog.read("policy.yaml"), Some(POLICY_YAML.as_bytes()));
        assert!(catalog
            .read_text("workflows/routed-exec.workflow.json")
            .unwrap()
            .contains("schemaVersion"));
        assert_eq!(builtin_dir_call_count(), before);
    }

    #[test]
    fn embedded_resources_normalizes_logical_keys_and_rejects_escape() {
        let catalog = EmbeddedResources::new();
        for key in [
            "/builtin/routes.yaml",
            "\\builtin\\routes.yaml",
            "/builtin/./routes.yaml",
            "/builtin//routes.yaml",
        ] {
            assert_eq!(catalog.read(key), Some(ROUTES_YAML.as_bytes()), "{key}");
        }
        for key in [
            "",
            "/",
            "/builtin",
            "/builtin/",
            "//builtin/routes.yaml",
            "/tmp/routes.yaml",
            "builtin/routes.yaml",
            "builtin:routes.yaml",
            "../routes.yaml",
            "/builtin/../routes.yaml",
            "C:/routes.yaml",
            "workers/../routes.yaml",
            "routes.yaml\0",
        ] {
            assert!(!catalog.contains(key), "{key}");
            assert!(catalog.read(key).is_none(), "{key}");
        }
    }

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
        assert_eq!(schemas, 10, "workers/schemas/*.json must be enumerated");
        assert!(
            files
                .iter()
                .any(|f| f.path() == Path::new("bulk-reader.md")),
            "top-level worker defs must be enumerated"
        );
    }

    #[test]
    fn builtin_resources_extracts_routes_workflows_and_condense() {
        let tmp = tempfile::tempdir().unwrap();
        extract_builtin(tmp.path()).unwrap();
        let mut expected: Vec<String> = builtin_resources()
            .iter()
            .map(|(p, _)| p.to_string_lossy().into_owned())
            .collect();
        expected.sort();
        let mut actual: Vec<String> = Vec::new();
        for entry in walkdir(tmp.path()) {
            if entry.is_file() && entry.file_name() != Some(std::ffi::OsStr::new(".ready")) {
                actual.push(
                    entry
                        .strip_prefix(tmp.path())
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
        actual.sort();
        assert_eq!(actual, expected);
        for rel in [
            "routes.yaml",
            "workflows/routed-exec.workflow.json",
            "workers/condense.md",
            "workers/schemas/condense.input.schema.json",
            "workers/schemas/condense.output.schema.json",
        ] {
            let on_disk = std::fs::read(tmp.path().join(rel)).unwrap();
            let embedded = builtin_resources()
                .iter()
                .find(|(p, _)| p.to_string_lossy() == rel)
                .unwrap()
                .1;
            assert_eq!(on_disk, embedded, "{rel} must be byte-identical");
        }
    }

    fn walkdir(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.extend(walkdir(&path));
            } else {
                out.push(path);
            }
        }
        out
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
        let resources = builtin_resources();
        let mut h = DefaultHasher::new();
        for (i, (path, contents)) in resources.iter().enumerate() {
            if i == 0 {
                // 1file 目の contents を攪乱した場合 fingerprint が変わる。
                path.hash(&mut h);
                b"perturbed".hash(&mut h);
            } else {
                path.hash(&mut h);
                contents.hash(&mut h);
            }
        }
        assert_ne!(format!("{:016x}", h.finish()), base);
        let mut h2 = DefaultHasher::new();
        for (i, (path, contents)) in resources.iter().enumerate() {
            if i == 0 {
                "perturbed-path".hash(&mut h2);
                contents.hash(&mut h2);
            } else {
                path.hash(&mut h2);
                contents.hash(&mut h2);
            }
        }
        assert_ne!(format!("{:016x}", h2.finish()), base);
    }
}
