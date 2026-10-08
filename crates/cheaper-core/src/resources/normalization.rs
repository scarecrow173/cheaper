//! Port of `packages/core/src/resources/normalization.ts`.
//!
//! `normalizeResourceManifest` / `normalizeResourceClaim` accept untyped
//! declaration JSON (the TS runtime validates it), so the Rust API takes
//! `serde_json::Value`. `WorkspaceEscapeError` propagates as the existing
//! boundary (TS comment: "WorkspaceEscapeError は既存 boundary のまま伝播する").

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::nodepath;
use crate::resources::predicates::resource_key;
use crate::resources::types::{
    ClaimMode, Resource, ResourceClaim, ResourceManifest, ResourceScope,
};
use crate::workspace::paths::{
    canonicalize_workspace_root, resolve_workspace_path, SymlinkPolicy, WorkspaceEscapeError,
};

/// Errors from resource normalization: `INVALID_REQUEST`
/// (`CheaperError`) or a propagated `WorkspaceEscapeError`.
#[derive(Debug)]
pub enum ResourceError {
    Invalid(CheaperError),
    Escape(WorkspaceEscapeError),
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(e) => e.fmt(f),
            Self::Escape(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Invalid(e) => Some(e),
            Self::Escape(e) => Some(e),
        }
    }
}

impl From<CheaperError> for ResourceError {
    fn from(e: CheaperError) -> Self {
        Self::Invalid(e)
    }
}

impl From<WorkspaceEscapeError> for ResourceError {
    fn from(e: WorkspaceEscapeError) -> Self {
        Self::Escape(e)
    }
}

/// D5 JSON workspace-relative declaration. `mode` derives from the bucket.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeDeclaration {
    pub path: String,
    pub scope: ResourceScope,
}

/// `ResourceManifestDeclaration`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceManifestDeclaration {
    pub reads: Option<Vec<ScopeDeclaration>>,
    pub writes: Option<Vec<ScopeDeclaration>>,
    pub produces: Option<Vec<ScopeDeclaration>>,
}

fn invalid(message: &str) -> ResourceError {
    ResourceError::Invalid(CheaperError::new(
        ErrorCode::InvalidRequest,
        format!("invalid resource declaration: {message}"),
    ))
}

/// `objectWithKeys`: value must be a plain object; all keys must be in
/// `allowed`, and all `required` keys must be present.
fn object_with_keys<'v>(
    value: &'v Value,
    allowed: &[&str],
    required: &[&str],
) -> Result<&'v Map<String, Value>, ResourceError> {
    let record = match value {
        Value::Object(m) => m,
        _ => return Err(invalid("expected an object")),
    };
    if record.keys().any(|k| !allowed.contains(&k.as_str()))
        || required.iter().any(|k| !record.contains_key(*k))
    {
        return Err(invalid("missing or unknown field"));
    }
    Ok(record)
}

fn validate_scope(value: &Value) -> Result<ResourceScope, ResourceError> {
    match value.as_str() {
        Some("exact") => Ok(ResourceScope::Exact),
        Some("subtree") => Ok(ResourceScope::Subtree),
        _ => Err(invalid("scope must be exact or subtree")),
    }
}

fn validate_path(value: &Value) -> Result<&str, ResourceError> {
    match value.as_str() {
        Some(s) if !s.is_empty() && !s.contains('\0') => Ok(s),
        _ => Err(invalid("path must be a nonempty string without NUL")),
    }
}

/// True when `p` looks like a drive-relative or drive-absolute win32 path
/// (`/^[a-z]:/i` in TS — checked on every platform).
fn has_drive_prefix(p: &str) -> bool {
    let b = p.as_bytes();
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

fn declaration(value: &Value) -> Result<ScopeDeclaration, ResourceError> {
    let record = object_with_keys(value, &["path", "scope"], &["path", "scope"])?;
    let path = validate_path(&record["path"])?;
    let scope = validate_scope(&record["scope"])?;
    if nodepath::is_absolute(path) || has_drive_prefix(path) {
        return Err(invalid("declaration path must be workspace-relative"));
    }
    Ok(ScopeDeclaration {
        path: path.to_string(),
        scope,
    })
}

/// Typed-input variant of `declaration` (`normalizeResourceClaim` takes a
/// `ScopeDeclaration` in TS but validates the same invariants).
fn validate_declaration(input: &ScopeDeclaration) -> Result<(), ResourceError> {
    if input.path.is_empty() || input.path.contains('\0') {
        return Err(invalid("path must be a nonempty string without NUL"));
    }
    if nodepath::is_absolute(&input.path) || has_drive_prefix(&input.path) {
        return Err(invalid("declaration path must be workspace-relative"));
    }
    Ok(())
}

fn canonical_claim(value: &Value, mode: ClaimMode) -> Result<ResourceClaim, ResourceError> {
    let record = object_with_keys(value, &["resource", "mode"], &["resource", "mode"])?;
    let claim_mode = match record["mode"].as_str() {
        Some("shared") => ClaimMode::Shared,
        Some("exclusive") => ClaimMode::Exclusive,
        _ => return Err(invalid("mode must be shared or exclusive")),
    };
    if claim_mode != mode {
        return Err(invalid("bucket and claim mode disagree"));
    }
    let resource = object_with_keys(
        &record["resource"],
        &["kind", "path", "scope"],
        &["kind", "path", "scope"],
    )?;
    if resource["kind"].as_str() != Some("workspace") {
        return Err(invalid("unknown resource kind"));
    }
    let path = validate_path(&resource["path"])?;
    let scope = validate_scope(&resource["scope"])?;
    if !nodepath::is_absolute(path) {
        return Err(invalid("canonical claim path must be absolute"));
    }
    Ok(ResourceClaim {
        resource: Resource::workspace(path, scope),
        mode,
    })
}

/// POSIX backslash is an escape meta; on Windows it is a separator.
fn components(input: &str) -> Vec<&str> {
    if cfg!(windows) {
        input.split(['/', '\\']).collect()
    } else {
        input.split('/').collect()
    }
}

fn is_glob_component(component: &str) -> bool {
    component
        .chars()
        .any(|c| matches!(c, '*' | '?' | '[' | ']' | '{' | '}' | '(' | ')'))
        || (!cfg!(windows) && component.contains('\\'))
}

struct DeclaredScope {
    path: String,
    scope: ResourceScope,
    glob: bool,
}

fn declared_scope(
    input: &ScopeDeclaration,
    ownership: bool,
) -> Result<DeclaredScope, ResourceError> {
    let parts = components(&input.path);
    let negated = input.path.starts_with('!');
    let first_meta = if negated {
        Some(0)
    } else {
        parts.iter().position(|c| is_glob_component(c))
    };
    let Some(first_meta) = first_meta else {
        return Ok(DeclaredScope {
            path: input.path.clone(),
            scope: input.scope,
            glob: false,
        });
    };
    if parts[first_meta..].contains(&"..") {
        return Err(invalid("traversal after glob meta"));
    }
    if ownership
        && (negated
            || input.scope != ResourceScope::Subtree
            || first_meta != parts.len() - 1
            || parts[first_meta] != "**")
    {
        return Err(invalid(
            "partial glob producer requires enumeration or directory ownership",
        ));
    }
    Ok(DeclaredScope {
        path: {
            let joined = parts[..first_meta].join(&nodepath::SEP.to_string());
            if joined.is_empty() {
                ".".to_string()
            } else {
                joined
            }
        },
        scope: ResourceScope::Subtree,
        glob: true,
    })
}

fn resolve_declaration(
    root: &str,
    input: &ScopeDeclaration,
    mode: ClaimMode,
    policy: SymlinkPolicy,
    ownership: bool,
) -> Result<ResourceClaim, ResourceError> {
    let scope = declared_scope(input, ownership)?;
    // Boundary check on the literal ancestor before glob expansion.
    // ownership never expands.
    let resolved = resolve_workspace_path(root, &scope.path, policy)?;
    let identity = if scope.glob && !ownership && policy == SymlinkPolicy::WithinWorkspace {
        root.to_string()
    } else {
        resolved.real
    };
    Ok(ResourceClaim {
        resource: Resource::workspace(identity, scope.scope),
        mode,
    })
}

/// `normalizeResourceClaim`.
pub fn normalize_resource_claim(
    workspace_root: &str,
    input: &ScopeDeclaration,
    mode: ClaimMode,
    symlink_policy: SymlinkPolicy,
) -> Result<ResourceClaim, ResourceError> {
    validate_declaration(input)?;
    let root = canonicalize_workspace_root(workspace_root)?;
    resolve_declaration(&root, input, mode, symlink_policy, false)
}

/// `left < right ? -1 : left > right ? 1 : 0` in TS — JS relational
/// string order = UTF-16 code units (not `localeCompare`).
fn compare(left: &str, right: &str) -> std::cmp::Ordering {
    crate::js::code_unit_compare(left, right)
}

fn deduplicate(mut claims: Vec<ResourceClaim>) -> Vec<ResourceClaim> {
    claims.sort_by(|a, b| {
        compare(&resource_key(&a.resource), &resource_key(&b.resource))
            .then_with(|| compare(a.mode.as_str(), b.mode.as_str()))
            .then_with(|| compare(a.resource.path(), b.resource.path()))
    });
    let mut seen = std::collections::HashSet::new();
    claims.retain(|claim| {
        let key =
            serde_json::json!([resource_key(&claim.resource), claim.mode.as_str()]).to_string();
        seen.insert(key)
    });
    claims
}

fn normalize_bucket(
    root: &str,
    input: Option<&Value>,
    mode: ClaimMode,
    policy: SymlinkPolicy,
    ownership: bool,
) -> Result<Vec<ResourceClaim>, ResourceError> {
    match input {
        None => {
            if ownership {
                // `undefined` producer bucket: no ownership claims.
                return Ok(Vec::new());
            }
            Ok(vec![ResourceClaim {
                resource: Resource::workspace(root, ResourceScope::Subtree),
                mode,
            }])
        }
        Some(Value::Null) => {
            if ownership {
                return Err(invalid("unknown producer ownership"));
            }
            Ok(vec![ResourceClaim {
                resource: Resource::workspace(root, ResourceScope::Subtree),
                mode,
            }])
        }
        Some(value) => {
            let arr = match value {
                Value::Array(a) => a,
                _ => return Err(invalid("bucket must be an array")),
            };
            let mut representation: Option<&'static str> = None;
            let mut claims = Vec::new();
            for item in arr {
                let record = object_with_keys(item, &["resource", "mode", "path", "scope"], &[])?;
                let current = if record.contains_key("resource") {
                    "claim"
                } else {
                    "declaration"
                };
                if representation.is_some_and(|r| r != current) {
                    return Err(invalid("mixed bucket representations"));
                }
                representation = Some(current);
                if current == "claim" {
                    let claim = canonical_claim(item, mode)?;
                    // canonical path is a literal identity, not re-globbed.
                    let resolved = resolve_workspace_path(root, claim.resource.path(), policy)?;
                    claims.push(ResourceClaim {
                        resource: Resource::workspace(resolved.real, claim.resource.scope()),
                        mode,
                    });
                } else {
                    let decl = declaration(item)?;
                    claims.push(resolve_declaration(root, &decl, mode, policy, ownership)?);
                }
            }
            Ok(deduplicate(claims))
        }
    }
}

/// `normalizeResourceManifest`. `input` absent and `null` per-bucket
/// follow the TS `undefined`/`null` distinction.
pub fn normalize_resource_manifest(
    workspace_root: &str,
    input: Option<&Value>,
    symlink_policy: SymlinkPolicy,
) -> Result<ResourceManifest, ResourceError> {
    let default = Map::new();
    let record = match input {
        None => &default,
        Some(v) => object_with_keys(v, &["reads", "writes", "produces"], &[])?,
    };
    let root = canonicalize_workspace_root(workspace_root)?;
    Ok(ResourceManifest {
        reads: normalize_bucket(
            &root,
            record.get("reads"),
            ClaimMode::Shared,
            symlink_policy,
            false,
        )?,
        writes: normalize_bucket(
            &root,
            record.get("writes"),
            ClaimMode::Exclusive,
            symlink_policy,
            false,
        )?,
        produces: normalize_bucket(
            &root,
            record.get("produces"),
            ClaimMode::Exclusive,
            symlink_policy,
            true,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn tmpdir() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = canonicalize_workspace_root(dir.path().to_str().unwrap()).unwrap();
        (dir, root)
    }

    fn err(input: Option<&Value>) -> CheaperError {
        let dir = tempfile::tempdir().unwrap();
        match normalize_resource_manifest(dir.path().to_str().unwrap(), input, SymlinkPolicy::Deny)
        {
            Err(ResourceError::Invalid(e)) => e,
            other => panic!("expected invalid, got {other:?}"),
        }
    }

    #[test]
    fn empty_manifest_defaults_to_subtree_buckets() {
        let (_d, root) = tmpdir();
        let m = normalize_resource_manifest(&root, None, SymlinkPolicy::Deny).unwrap();
        assert_eq!(
            m.reads,
            vec![ResourceClaim {
                resource: Resource::workspace(root.clone(), ResourceScope::Subtree),
                mode: ClaimMode::Shared,
            }]
        );
        assert_eq!(
            m.writes,
            vec![ResourceClaim {
                resource: Resource::workspace(root.clone(), ResourceScope::Subtree),
                mode: ClaimMode::Exclusive,
            }]
        );
        assert!(m.produces.is_empty());
    }

    #[test]
    fn declaration_resolves_relative_paths() {
        let (_d, root) = tmpdir();
        let input = json!({
            "reads": [{"path": "src", "scope": "subtree"}],
        });
        let m = normalize_resource_manifest(&root, Some(&input), SymlinkPolicy::Deny).unwrap();
        assert_eq!(m.reads.len(), 1);
        assert_eq!(m.reads[0].resource.path(), nodepath::join(&[&root, "src"]));
        assert_eq!(m.reads[0].resource.scope(), ResourceScope::Subtree);
    }

    #[test]
    fn rejects_absolute_declaration_paths() {
        let abs = if cfg!(windows) { "C:/x" } else { "/x" };
        let e = err(Some(&json!({"reads": [{"path": abs, "scope": "subtree"}]})));
        assert_eq!(e.code(), ErrorCode::InvalidRequest);
        assert!(e
            .message()
            .contains("declaration path must be workspace-relative"));
    }

    #[test]
    fn rejects_mixed_bucket_representations() {
        let root_path = if cfg!(windows) { "C:\\x" } else { "/x" };
        let e = err(Some(&json!({
            "reads": [
                {"path": "src", "scope": "subtree"},
                {"resource": {"kind": "workspace", "path": root_path, "scope": "exact"},
                 "mode": "shared"},
            ]
        })));
        assert!(e.message().contains("mixed bucket representations"));
    }

    #[test]
    fn rejects_traversal_after_glob_meta() {
        let e = err(Some(&json!({
            "reads": [{"path": "a/*/../b", "scope": "subtree"}]
        })));
        assert!(e.message().contains("traversal after glob meta"));
    }

    #[test]
    fn producer_glob_requires_directory_ownership() {
        // Non-`**`-terminal glob for produces is rejected.
        let e = err(Some(&json!({
            "produces": [{"path": "out/*.log", "scope": "subtree"}]
        })));
        assert!(e.message().contains("partial glob producer"));
        // `dir/**` with subtree scope is accepted.
        let (_d, root) = tmpdir();
        let ok = normalize_resource_manifest(
            &root,
            Some(&json!({"produces": [{"path": "out/**", "scope": "subtree"}]})),
            SymlinkPolicy::Deny,
        )
        .unwrap();
        assert_eq!(
            ok.produces[0].resource.path(),
            nodepath::join(&[&root, "out"])
        );
    }

    #[test]
    fn null_producer_is_unknown_ownership() {
        let e = err(Some(&json!({"produces": null})));
        assert!(e.message().contains("unknown producer ownership"));
    }

    #[test]
    fn dedupes_and_sorts_claims() {
        let (_d, root) = tmpdir();
        let input = json!({
            "reads": [
                {"path": "b", "scope": "exact"},
                {"path": "a", "scope": "exact"},
                {"path": "a", "scope": "exact"},
            ]
        });
        let m = normalize_resource_manifest(&root, Some(&input), SymlinkPolicy::Deny).unwrap();
        let paths: Vec<&str> = m.reads.iter().map(|c| c.resource.path()).collect();
        assert_eq!(
            paths,
            [
                nodepath::join(&[&root, "a"]).as_str(),
                nodepath::join(&[&root, "b"]).as_str()
            ]
        );
    }

    #[test]
    fn glob_declaration_expands_to_root_under_within_workspace() {
        let (_d, root) = tmpdir();
        let input = json!({"reads": [{"path": "src/**", "scope": "subtree"}]});
        let m = normalize_resource_manifest(&root, Some(&input), SymlinkPolicy::WithinWorkspace)
            .unwrap();
        // glob + !ownership + within-workspace → identity is the root.
        assert_eq!(m.reads[0].resource.path(), root);
        assert_eq!(m.reads[0].resource.scope(), ResourceScope::Subtree);
        // Under deny, identity is the literal ancestor.
        let m2 = normalize_resource_manifest(&root, Some(&input), SymlinkPolicy::Deny).unwrap();
        assert_eq!(m2.reads[0].resource.path(), nodepath::join(&[&root, "src"]));
    }

    #[test]
    fn escapes_propagate_as_workspace_escape_error() {
        let (_d, root) = tmpdir();
        let input = json!({"reads": [{"path": "../outside", "scope": "subtree"}]});
        match normalize_resource_manifest(&root, Some(&input), SymlinkPolicy::Deny) {
            Err(ResourceError::Escape(_)) => {}
            other => panic!("expected escape, got {other:?}"),
        }
    }
}
