//! Port of `packages/core/src/worker/resolve.ts`.

use serde_json::{json, Map};

use crate::config::dirs::ConfigDirs;
use crate::errors::{CheaperError, ErrorCode};
use crate::nodepath;
use crate::worker::parse::parse_worker_file;
use crate::worker::validate::{validate_worker_definition, WorkerDefinition, WorkerOrigin};

const NAME_PATTERN: &str = "^[a-z0-9][a-z0-9-]*$";

fn name_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(NAME_PATTERN).unwrap())
}

/// `fs.access`-style existence check: any error is "does not exist".
fn exists(p: &str) -> bool {
    std::fs::metadata(p).is_ok()
}

/// Raw worker-file read failure. TS `loadWorker` propagates the
/// `fs.readFile` `ErrnoException` as-is (not a `CheaperError`), so this
/// carries only the OS message — no structured `details` — and stays
/// distinguishable for the CLI boundary, which maps it to
/// INTERNAL_ERROR. `Display` approximates Node's
/// `"{CODE}: {msg}, open '{path}'"` (the wording is non-contractual).
#[derive(Debug)]
pub struct WorkerReadError {
    path: String,
    source: std::io::Error,
}

impl WorkerReadError {
    /// The worker file path that failed to read.
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl std::fmt::Display for WorkerReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}, open '{}'",
            crate::errors::io_errno_display(&self.path, &self.source),
            self.source,
            self.path
        )
    }
}

impl std::error::Error for WorkerReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// `resolveWorker` / `resolveWorkerFile` error: a `CheaperError` for
/// contract violations (not-found, invalid definition), or a raw
/// [`WorkerReadError`] when the file exists but cannot be read — like
/// the TS propagation of the `ErrnoException`.
#[derive(Debug)]
pub enum ResolveWorkerError {
    Cheaper(CheaperError),
    Read(WorkerReadError),
}

impl std::fmt::Display for ResolveWorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cheaper(e) => e.fmt(f),
            Self::Read(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ResolveWorkerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cheaper(e) => Some(e),
            Self::Read(e) => Some(e),
        }
    }
}

impl From<CheaperError> for ResolveWorkerError {
    fn from(e: CheaperError) -> Self {
        Self::Cheaper(e)
    }
}

impl From<WorkerReadError> for ResolveWorkerError {
    fn from(e: WorkerReadError) -> Self {
        Self::Read(e)
    }
}

fn load_worker(
    source_path: &str,
    origin: WorkerOrigin,
) -> Result<WorkerDefinition, ResolveWorkerError> {
    // TS propagates the raw read error (not a CheaperError); the CLI maps
    // unknown errors to INTERNAL_ERROR with no details — keep that
    // shape via `WorkerReadError`.
    let bytes = std::fs::read(source_path).map_err(|e| WorkerReadError {
        path: source_path.to_string(),
        source: e,
    })?;
    let content = String::from_utf8_lossy(&bytes);
    Ok(validate_worker_definition(
        &parse_worker_file(&content, source_path)?,
        source_path,
        origin,
    )?)
}

/// `resolveWorker`: project → user → builtin precedence, `searched`
/// enumeration in the NOT_FOUND details.
pub fn resolve_worker(
    name: &str,
    dirs: &ConfigDirs,
) -> Result<WorkerDefinition, ResolveWorkerError> {
    if !name_re().is_match(name) {
        let mut details = Map::new();
        details.insert("name".to_string(), json!(name));
        return Err(CheaperError::new(
            ErrorCode::InvalidWorkerName,
            format!("invalid worker name: {name}"),
        )
        .with_details(details)
        .into());
    }
    let candidates: [(String, WorkerOrigin); 3] = [
        (
            dirs.project_dir.to_string_lossy().into_owned(),
            WorkerOrigin::Project,
        ),
        (
            dirs.user_config_dir.to_string_lossy().into_owned(),
            WorkerOrigin::User,
        ),
        (
            dirs.builtin_dir.to_string_lossy().into_owned(),
            WorkerOrigin::Builtin,
        ),
    ];
    let mut searched: Vec<String> = Vec::new();
    for (dir, origin) in candidates {
        let p = nodepath::join(&[&dir, "workers", &format!("{name}.md")]);
        searched.push(p.clone());
        if exists(&p) {
            return load_worker(&p, origin);
        }
    }
    let mut details = Map::new();
    details.insert("name".to_string(), json!(name));
    details.insert("searched".to_string(), json!(searched));
    Err(CheaperError::new(
        ErrorCode::WorkerNotFound,
        format!("worker not found: {name}"),
    )
    .with_details(details)
    .into())
}

/// `resolveWorkerFile`: cwd-resolved explicit path with `origin:
/// "explicit"`.
pub fn resolve_worker_file(
    file_path: &str,
    cwd: &str,
) -> Result<WorkerDefinition, ResolveWorkerError> {
    let resolved = nodepath::resolve(cwd, file_path);
    if !exists(&resolved) {
        let mut details = Map::new();
        details.insert("filePath".to_string(), json!(file_path));
        details.insert("searched".to_string(), json!([resolved]));
        return Err(CheaperError::new(
            ErrorCode::WorkerNotFound,
            format!("worker file not found: {file_path}"),
        )
        .with_details(details)
        .into());
    }
    load_worker(&resolved, WorkerOrigin::Explicit)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(base: &std::path::Path) -> ConfigDirs {
        ConfigDirs {
            user_config_dir: base.join("user"),
            project_dir: base.join("project"),
            builtin_dir: base.join("builtin"),
        }
    }

    fn write_worker(dir: &std::path::Path, name: &str, tier: &str) {
        let workers = dir.join("workers");
        std::fs::create_dir_all(&workers).unwrap();
        std::fs::write(
            workers.join(format!("{name}.md")),
            format!("---\nname: {name}\ndescription: d\ntools: [read]\ncheaper:\n  model: {{tier: {tier}}}\n  output: {{schema: {{inline: {{type: string}}}}}}\n---\nbody\n"),
        )
        .unwrap();
    }

    #[test]
    fn resolves_in_project_user_builtin_order() {
        let base = tempfile::tempdir().unwrap();
        write_worker(&base.path().join("builtin"), "w", "high");
        write_worker(&base.path().join("user"), "w", "medium");
        write_worker(&base.path().join("project"), "w", "low");
        let def = resolve_worker("w", &dirs(base.path())).unwrap();
        assert_eq!(def.origin, WorkerOrigin::Project);
        assert_eq!(def.cheaper.model.tier, crate::model::tier::Tier::Low);
    }

    #[test]
    fn falls_back_to_user_then_builtin() {
        let base = tempfile::tempdir().unwrap();
        write_worker(&base.path().join("user"), "w", "medium");
        write_worker(&base.path().join("builtin"), "w", "high");
        assert_eq!(
            resolve_worker("w", &dirs(base.path())).unwrap().origin,
            WorkerOrigin::User
        );
    }

    fn expect_cheaper(e: ResolveWorkerError) -> CheaperError {
        match e {
            ResolveWorkerError::Cheaper(e) => e,
            other => panic!("expected CheaperError, got {other}"),
        }
    }

    #[test]
    fn not_found_lists_searched() {
        let base = tempfile::tempdir().unwrap();
        let e = expect_cheaper(resolve_worker("missing", &dirs(base.path())).unwrap_err());
        assert_eq!(e.code(), ErrorCode::WorkerNotFound);
        assert_eq!(
            e.details().unwrap()["searched"].as_array().unwrap().len(),
            3
        );
    }

    #[test]
    fn invalid_name_rejected() {
        let base = tempfile::tempdir().unwrap();
        let e = expect_cheaper(resolve_worker("Bad Name", &dirs(base.path())).unwrap_err());
        assert_eq!(e.code(), ErrorCode::InvalidWorkerName);
    }

    #[test]
    fn read_failure_is_a_plain_io_error_without_details() {
        // TS propagates the raw fs error (not a CheaperError): an
        // existing-but-unreadable worker file surfaces as
        // `WorkerReadError` carrying only the OS message.
        let base = tempfile::tempdir().unwrap();
        let workers = base.path().join("project").join("workers");
        std::fs::create_dir_all(workers.join("w.md")).unwrap();
        match resolve_worker("w", &dirs(base.path())).unwrap_err() {
            ResolveWorkerError::Read(e) => {
                assert!(e.to_string().contains("open '"));
                assert!(e.to_string().contains("w.md"));
            }
            other => panic!("expected WorkerReadError, got {other}"),
        }
    }

    #[test]
    fn resolve_worker_file_marks_explicit_origin() {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join("defs");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("custom.md"),
            "---\nname: custom\ndescription: d\ntools: [read]\ncheaper:\n  model: {tier: low}\n  output: {schema: {inline: {type: string}}}\n---\nbody\n",
        )
        .unwrap();
        let def = resolve_worker_file("defs/custom.md", base.path().to_str().unwrap()).unwrap();
        assert_eq!(def.origin, WorkerOrigin::Explicit);
        let e = expect_cheaper(
            resolve_worker_file("nope.md", base.path().to_str().unwrap()).unwrap_err(),
        );
        assert_eq!(e.code(), ErrorCode::WorkerNotFound);
    }
}
