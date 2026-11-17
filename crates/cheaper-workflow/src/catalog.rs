//! project scope workflow catalog(`.cheaper/workflows/catalog.json`)の
//! bounded read/validate。catalog は発見用 index であり、Workflow 定義の
//! 正本・権限/実行の根拠ではない(ADR-034ZixZvVK6PLmS1LMRJqu)。entry
//! path は resolve/deny/stat のみ行い、definition 本文・compiler には
//! 触れない。

use std::collections::HashMap;
use std::io::{ErrorKind, Read};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::policy::schema::FilesystemPolicy;
use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
use cheaper_core::workspace::paths::{resolve_workspace_path, SymlinkPolicy};
use serde::Serialize;
use serde_json::Value;

use crate::loader::check_workflow_path_denied;

/// workspace root 相対の固定 path(POSIX 区切り。list 出力にもそのまま載る)。
pub const CATALOG_RELATIVE_PATH: &str = ".cheaper/workflows/catalog.json";
pub const MAX_CATALOG_ENTRIES: usize = 256;
const CATALOG_SCHEMA_VERSION: u64 = 1;
const MAX_CATALOG_BYTES: u64 = 64 * 1024;
const MAX_TAGS: usize = 16;
const MAX_TAG_CHARS: usize = 64;
/// entry string field の文字数上限(超過は manifest 破損)。
const FIELD_CAPS: [(&str, usize); 3] = [("id", 64), ("path", 512), ("summary", 500)];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryStatus {
    Ok,
    Missing,
    Invalid,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticKind {
    MalformedEntry,
    DuplicateId,
    MissingPath,
    PathOutOfWorkspace,
    DeniedPath,
    NotRegularFile,
}

/// manifest field の射影 + status。string でない field は `None`、
/// `tags` は string 配列のときのみ保持する(raw JSON は出さない)。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatalogEntry {
    pub id: Option<String>,
    pub path: Option<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub status: EntryStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CatalogDiagnostic {
    pub kind: DiagnosticKind,
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub message: String,
}

/// `present:false` は manifest 欠如(空 catalog)。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CatalogRead {
    pub present: bool,
    pub workflows: Vec<CatalogEntry>,
    pub diagnostics: Vec<CatalogDiagnostic>,
}

/// well-formed entry の検証済み field(raw への借用)。
struct WellFormed<'a> {
    id: &'a str,
    path: &'a str,
}

/// entry 単位の異常(status + diagnostic)。
struct Finding {
    status: EntryStatus,
    kind: DiagnosticKind,
    message: String,
}

impl Finding {
    fn new(status: EntryStatus, kind: DiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            status,
            kind,
            message: message.into(),
        }
    }
}

/// `<root>/.cheaper/workflows/catalog.json` を読む。manifest 破損
/// (read 失敗・JSON parse・schemaVersion・`workflows` 非 array・bound
/// 超過)は Err、entry 単位の異常は status + diagnostics で返す。
/// `root` は canonical 化済み workspace root。
pub fn read_catalog(
    root: &str,
    filesystem: &FilesystemPolicy,
) -> Result<CatalogRead, CheaperError> {
    let Some(bytes) = read_manifest(root)? else {
        return Ok(CatalogRead::default());
    };
    let entries = parse_manifest(&bytes)?;
    let shapes: Vec<Result<WellFormed, &'static str>> = entries.iter().map(well_formed).collect();
    let mut id_counts: HashMap<&str, usize> = HashMap::new();
    for entry in shapes.iter().flatten() {
        *id_counts.entry(entry.id).or_default() += 1;
    }
    let deny: Vec<&str> = filesystem.deny_paths.iter().map(String::as_str).collect();
    let denylist = create_path_denylist(&deny);
    let mut read = CatalogRead {
        present: true,
        ..CatalogRead::default()
    };
    for (index, (raw, shape)) in entries.iter().zip(&shapes).enumerate() {
        let id = string_field(raw, "id");
        let path = string_field(raw, "path");
        let finding = match shape {
            Err(message) => Some(Finding::new(
                EntryStatus::Invalid,
                DiagnosticKind::MalformedEntry,
                *message,
            )),
            Ok(entry) if id_counts[entry.id] > 1 => Some(Finding::new(
                EntryStatus::Duplicate,
                DiagnosticKind::DuplicateId,
                "workflow id is declared by multiple entries",
            )),
            Ok(entry) => check_entry_path(root, entry.path, filesystem.symlinks, &denylist),
        };
        let status = match finding {
            Some(finding) => {
                read.diagnostics.push(CatalogDiagnostic {
                    kind: finding.kind,
                    index,
                    id: id.clone(),
                    path: path.clone(),
                    message: finding.message,
                });
                finding.status
            }
            None => EntryStatus::Ok,
        };
        read.workflows.push(CatalogEntry {
            id,
            path,
            summary: string_field(raw, "summary"),
            tags: tags_field(raw),
            status,
        });
    }
    Ok(read)
}

fn invalid_input(message: impl Into<String>) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidInput, message.into())
}

/// bounded read。不在は `None`、上限超過は parse 前に Err。
fn read_manifest(root: &str) -> Result<Option<Vec<u8>>, CheaperError> {
    let read_error = |e: std::io::Error| {
        CheaperError::new(ErrorCode::InvalidRequest, "cannot read workflow catalog").with_cause(e)
    };
    let file = match std::fs::File::open(nodepath::join(&[root, CATALOG_RELATIVE_PATH])) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(read_error(e)),
    };
    let mut bytes = Vec::new();
    file.take(MAX_CATALOG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(read_error)?;
    if bytes.len() as u64 > MAX_CATALOG_BYTES {
        return Err(invalid_input(format!(
            "workflow catalog exceeds {MAX_CATALOG_BYTES} bytes"
        )));
    }
    Ok(Some(bytes))
}

/// manifest-level 検査(schemaVersion・`workflows` array・entry 数・
/// field 長 cap)。entry shape は検査しない。
fn parse_manifest(bytes: &[u8]) -> Result<Vec<Value>, CheaperError> {
    // parse error の message は manifest 本文を含み得るため公開しない。
    let mut raw: Value = serde_json::from_slice(bytes)
        .map_err(|_| invalid_input("workflow catalog must contain valid JSON"))?;
    if raw.get("schemaVersion").and_then(Value::as_u64) != Some(CATALOG_SCHEMA_VERSION) {
        return Err(invalid_input(format!(
            "workflow catalog schemaVersion must be {CATALOG_SCHEMA_VERSION}"
        )));
    }
    let Some(Value::Array(entries)) = raw.get_mut("workflows").map(Value::take) else {
        return Err(invalid_input(
            "workflow catalog `workflows` must be an array",
        ));
    };
    if entries.len() > MAX_CATALOG_ENTRIES {
        return Err(invalid_input(format!(
            "workflow catalog exceeds {MAX_CATALOG_ENTRIES} entries"
        )));
    }
    for (index, entry) in entries.iter().enumerate() {
        check_entry_bounds(index, entry)?;
    }
    Ok(entries)
}

fn check_entry_bounds(index: usize, entry: &Value) -> Result<(), CheaperError> {
    let exceeds = |s: &str, max: usize| s.chars().count() > max;
    for (key, max) in FIELD_CAPS {
        if entry
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|s| exceeds(s, max))
        {
            return Err(invalid_input(format!(
                "workflow catalog entry {index} `{key}` exceeds {max} characters"
            )));
        }
    }
    if let Some(Value::Array(tags)) = entry.get("tags") {
        if tags.len() > MAX_TAGS {
            return Err(invalid_input(format!(
                "workflow catalog entry {index} `tags` exceeds {MAX_TAGS} items"
            )));
        }
        if tags
            .iter()
            .filter_map(Value::as_str)
            .any(|tag| exceeds(tag, MAX_TAG_CHARS))
        {
            return Err(invalid_input(format!(
                "workflow catalog entry {index} tag exceeds {MAX_TAG_CHARS} characters"
            )));
        }
    }
    Ok(())
}

/// `[A-Za-z0-9][A-Za-z0-9._-]*`(長さ上限は bound 検査が先に担う)。
fn is_valid_id(id: &str) -> bool {
    let mut bytes = id.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphanumeric())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// entry shape 検査。
fn well_formed(entry: &Value) -> Result<WellFormed<'_>, &'static str> {
    if !entry.is_object() {
        return Err("workflow entry must be an object");
    }
    let id = entry
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| is_valid_id(id))
        .ok_or("workflow entry `id` must match [A-Za-z0-9][A-Za-z0-9._-]*")?;
    let path = match entry.get("path").and_then(Value::as_str) {
        None | Some("") => return Err("workflow entry `path` must be a non-empty string"),
        Some(path) if nodepath::is_absolute(path) => {
            return Err("workflow entry `path` must be workspace-relative");
        }
        Some(path) => path,
    };
    if !entry.get("summary").is_some_and(Value::is_string) {
        return Err("workflow entry `summary` must be a string");
    }
    match entry.get("tags") {
        None => {}
        Some(Value::Array(tags)) if tags.iter().all(Value::is_string) => {}
        Some(_) => return Err("workflow entry `tags` must be an array of strings"),
    }
    Ok(WellFormed { id, path })
}

/// loader と同一の resolve(symlink policy)→ deny → stat 規則。
/// file 本文は読まない。
fn check_entry_path(
    root: &str,
    path: &str,
    symlinks: SymlinkPolicy,
    denylist: &PathDenylist,
) -> Option<Finding> {
    let resolved = match resolve_workspace_path(root, path, symlinks) {
        Ok(resolved) => resolved,
        Err(e) => {
            return Some(Finding::new(
                EntryStatus::Invalid,
                DiagnosticKind::PathOutOfWorkspace,
                e.to_string(),
            ));
        }
    };
    if let Err(e) = check_workflow_path_denied(root, path, &resolved, denylist) {
        return Some(Finding::new(
            EntryStatus::Invalid,
            DiagnosticKind::DeniedPath,
            e.message(),
        ));
    }
    match std::fs::metadata(&resolved.real) {
        Ok(metadata) if metadata.is_file() => None,
        Ok(_) => Some(Finding::new(
            EntryStatus::Invalid,
            DiagnosticKind::NotRegularFile,
            "workflow path must be a regular file",
        )),
        Err(e) if e.kind() == ErrorKind::NotFound => Some(Finding::new(
            EntryStatus::Missing,
            DiagnosticKind::MissingPath,
            "workflow file does not exist",
        )),
        Err(e) => Some(Finding::new(
            EntryStatus::Missing,
            DiagnosticKind::MissingPath,
            format!("workflow file cannot be accessed: {:?}", e.kind()),
        )),
    }
}

fn string_field(entry: &Value, key: &str) -> Option<String> {
    entry.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn tags_field(entry: &Value) -> Vec<String> {
    entry
        .get("tags")
        .and_then(Value::as_array)
        .and_then(|tags| {
            tags.iter()
                .map(|tag| tag.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
