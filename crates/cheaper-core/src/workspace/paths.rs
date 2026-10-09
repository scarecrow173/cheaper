//! Port of `packages/core/src/workspace/paths.ts`.
//!
//! DESIGN-0002 `resolveWorkspacePath` algorithm 1-5. Paths are handled as
//! strings through [`crate::nodepath`] so the byte-level results match
//! Node `path` (lexical `..`/`.` resolution, `\` separators on win32).

use serde::{Deserialize, Serialize};
use serde_json::{json, Map};

use crate::errors::{CheaperError, ErrorCode};
use crate::nodepath;

/// `SymlinkPolicy`: `"deny" | "within-workspace"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SymlinkPolicy {
    Deny,
    WithinWorkspace,
}

/// Port of `ResolvedWorkspacePath`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWorkspacePath {
    /// Canonical absolute path (nonexistent tail = deepest existing
    /// ancestor's realpath + remaining components).
    pub real: String,
    /// Workspace-relative, POSIX separators.
    pub display: String,
}

/// `WorkspaceEscapeError`: a plain error (not `CheaperError`) so the tool
/// layer can catch it and convert it to a tool error result. Carries the
/// original `input` like the TS class.
#[derive(Debug)]
pub struct WorkspaceEscapeError {
    input: String,
    reason: String,
}

impl WorkspaceEscapeError {
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The `reason` half of `workspace escape: <reason>` (not stored as a
    /// separate field in TS; kept here for test assertions).
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for WorkspaceEscapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "workspace escape: {}", self.reason)
    }
}

impl std::error::Error for WorkspaceEscapeError {}

fn escape(input: &str, reason: &str) -> WorkspaceEscapeError {
    WorkspaceEscapeError {
        input: input.to_string(),
        reason: reason.to_string(),
    }
}

/// `fs.realpath` equivalent: canonicalize and strip the Windows `\\?\`
/// verbatim prefix via `dunce` (design doc fixed rule). The tool layer's
/// walker uses this as the canonical cycle-prevention key and symlink
/// target resolver.
pub fn realpath(p: &str) -> std::io::Result<String> {
    let resolved = dunce::canonicalize(p)?;
    Ok(resolved.to_string_lossy().into_owned())
}

/// `canonicalizeWorkspaceRoot` (DESIGN-0002): existence check
/// (nonexistent → `INVALID_REQUEST`) → realpath.
pub fn canonicalize_workspace_root(cwd: &str) -> Result<String, CheaperError> {
    if let Err(e) = std::fs::metadata(cwd) {
        let mut details = Map::new();
        details.insert("cwd".to_string(), json!(cwd));
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("workspace root does not exist: {cwd}"),
        )
        .with_details(details)
        .with_cause(e));
    }
    realpath(cwd).map_err(|e| {
        let mut details = Map::new();
        details.insert("cwd".to_string(), json!(cwd));
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("workspace root does not exist: {cwd}"),
        )
        .with_details(details)
        .with_cause(e)
    })
}

/// `isInsideWorkspaceRoot`. `case_insensitive` mirrors the win32
/// case-insensitive FS comparison.
pub fn is_inside_workspace_root(root: &str, candidate: &str, case_insensitive: bool) -> bool {
    let (r, c) = if case_insensitive {
        (root.to_lowercase(), candidate.to_lowercase())
    } else {
        (root.to_string(), candidate.to_string())
    };
    let prefix = if r.ends_with(nodepath::SEP) {
        r.clone()
    } else {
        format!("{r}{}", nodepath::SEP)
    };
    c == r || c.starts_with(&prefix)
}

fn lstat_is_symlink(p: &str) -> Option<bool> {
    // lstatOrNull: any error (including EACCES) reads as "nonexistent".
    std::fs::symlink_metadata(p)
        .ok()
        .map(|m| m.file_type().is_symlink())
}

/// `resolveWorkspacePath` algorithm 1-5. `~` is not expanded (the caller
/// resolves to an absolute or workspace-relative path first).
pub fn resolve_workspace_path(
    root: &str,
    input: &str,
    symlink_policy: SymlinkPolicy,
) -> Result<ResolvedWorkspacePath, WorkspaceEscapeError> {
    // 1. NUL rejection.
    if input.contains('\0') {
        return Err(escape(input, "nul-character"));
    }

    // 2. resolve (absolute input is taken as-is).
    let abs = nodepath::resolve(root, input);

    // win32 filesystems are case-insensitive, so the comparison is too.
    // POSIX uses strict comparison (blocks case-differing sibling escapes).
    let case_insensitive = cfg!(windows);

    // 3. Under root?
    if !is_inside_workspace_root(root, &abs, case_insensitive) {
        return Err(escape(input, "outside-workspace"));
    }

    // 4. Walk components from root; symlinks are judged by policy.
    let rel = nodepath::relative(root, &abs);
    let components: Vec<&str> = if rel.is_empty() {
        Vec::new()
    } else {
        rel.split(nodepath::SEP).collect()
    };
    let mut current = root.to_string();
    let mut i = 0;
    while i < components.len() {
        let next = nodepath::join(&[&current, components[i]]);
        match lstat_is_symlink(&next) {
            // Nonexistent tail is allowed (creation target).
            None => break,
            Some(true) => {
                if symlink_policy == SymlinkPolicy::Deny {
                    return Err(escape(input, "symlink-denied"));
                }
                let target = match realpath(&next) {
                    Ok(t) => t,
                    // Dangling symlink: containment cannot be verified,
                    // fail closed.
                    Err(_) => return Err(escape(input, "symlink-unresolvable")),
                };
                if !is_inside_workspace_root(root, &target, case_insensitive) {
                    return Err(escape(input, "symlink-outside-workspace"));
                }
                current = target; // re-walk from the realpath side
                i += 1;
                continue;
            }
            Some(false) => {
                current = next;
                i += 1;
            }
        }
    }
    // `current` is now the deepest existing ancestor (each symlink was
    // verified under root; non-symlink components stay under root because
    // they are relative-walked from root).
    let mut parts = vec![current.as_str()];
    parts.extend(&components[i..]);
    let real = nodepath::join(&parts);

    // 5. display = workspace-relative, POSIX separators.
    let display_rel = nodepath::relative(root, &real);
    let display = if display_rel.is_empty() {
        ".".to_string()
    } else {
        display_rel
            .split(nodepath::SEP)
            .collect::<Vec<_>>()
            .join("/")
    };
    Ok(ResolvedWorkspacePath { real, display })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tmpdir")
    }

    #[test]
    fn canonicalize_returns_realpath() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.path().to_str().unwrap()).unwrap();
        let expected = dunce::canonicalize(dir.path())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert_eq!(root, expected);
    }

    #[test]
    fn canonicalize_rejects_missing_dir() {
        let dir = tmpdir();
        let missing = dir.path().join("nope");
        let err = canonicalize_workspace_root(missing.to_str().unwrap()).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRequest);
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn is_inside_workspace_root_cases() {
        let root = nodepath::join(&["tmp", "work"]);
        assert!(is_inside_workspace_root(&root, &root, false));
        assert!(is_inside_workspace_root(
            &root,
            &nodepath::join(&[&root, "a"]),
            false
        ));
        assert!(!is_inside_workspace_root(
            &root,
            &nodepath::join(&["tmp", "workx", "a"]),
            false
        ));
        let cased = nodepath::join(&["tmp", "Work", "secret"]);
        assert!(!is_inside_workspace_root(&root, &cased, false));
        assert!(is_inside_workspace_root(&root, &cased, true));
    }

    #[test]
    fn is_inside_workspace_root_volume_root() {
        // issue #77: root が SEP で終わる場合（volume root）に prefix の
        // 二重化で配下を outside と誤判定しないこと。`nodepath::SEP` は
        // build 時確定なので host 形状のみ検証する。
        #[cfg(windows)]
        {
            assert!(is_inside_workspace_root("C:\\", "C:\\resource-probe", true));
            // trailing-sep root は volume root 以外でも同じ contract。
            assert!(is_inside_workspace_root("C:\\work\\", "C:\\work\\a", true));
        }
        #[cfg(not(windows))]
        {
            assert!(is_inside_workspace_root("/", "/a", false));
            assert!(is_inside_workspace_root("/work/", "/work/a", false));
        }
        // 別 drive は platform 不問で拒否（fail-closed 維持）。
        assert!(!is_inside_workspace_root("C:\\", "D:\\x", true));
    }

    #[test]
    fn resolve_workspace_path_at_volume_root() {
        // root 直下の一意な nonexistent tail で構成し、実 FS の symlink
        // （macOS の /tmp → /private/tmp 等）や drive 実在に依存しない。
        let uniq = format!("cheaper-volroot-probe-{}", std::process::id());
        #[cfg(windows)]
        {
            let input = format!("{uniq}\\a.txt");
            let resolved = resolve_workspace_path("C:\\", &input, SymlinkPolicy::Deny).unwrap();
            assert_eq!(resolved.real, format!("C:\\{uniq}\\a.txt"));
            assert_eq!(resolved.display, format!("{uniq}/a.txt"));
        }
        #[cfg(not(windows))]
        {
            // canonicalize が trailing-sep root を実際に生成する seam も
            // pin する。
            let root = canonicalize_workspace_root("/").unwrap();
            let input = format!("{uniq}/a.txt");
            let resolved = resolve_workspace_path(&root, &input, SymlinkPolicy::Deny).unwrap();
            assert_eq!(resolved.real, format!("/{uniq}/a.txt"));
            assert_eq!(resolved.display, format!("{uniq}/a.txt"));
        }
    }

    fn workspace() -> (tempfile::TempDir, String) {
        let dir = tmpdir();
        std::fs::create_dir_all(dir.path().join("sub").join("deep")).unwrap();
        std::fs::write(dir.path().join("sub").join("file.txt"), "x").unwrap();
        let root = canonicalize_workspace_root(dir.path().to_str().unwrap()).unwrap();
        (dir, root)
    }

    #[test]
    fn rejects_nul() {
        let (_d, root) = workspace();
        let err = resolve_workspace_path(&root, "a\0b", SymlinkPolicy::Deny).unwrap_err();
        assert_eq!(err.reason(), "nul-character");
        assert_eq!(err.input(), "a\0b");
    }

    #[test]
    fn rejects_dotdot_escape() {
        let (_d, root) = workspace();
        for policy in [SymlinkPolicy::Deny, SymlinkPolicy::WithinWorkspace] {
            assert!(resolve_workspace_path(&root, "../outside.txt", policy).is_err());
            assert!(resolve_workspace_path(&root, "sub/../../outside.txt", policy).is_err());
        }
    }

    #[test]
    fn rejects_absolute_outside_root() {
        let (_d, root) = workspace();
        let outside = nodepath::join(&[&nodepath::dirname(&root), "definitely-outside.txt"]);
        assert!(resolve_workspace_path(&root, &outside, SymlinkPolicy::Deny).is_err());
    }

    #[test]
    fn accepts_absolute_inside_root() {
        let (_d, root) = workspace();
        let inside = nodepath::join(&[&root, "sub", "file.txt"]);
        let resolved = resolve_workspace_path(&root, &inside, SymlinkPolicy::Deny).unwrap();
        assert_eq!(resolved.display, "sub/file.txt");
    }

    #[test]
    fn tilde_is_a_literal_in_workspace_path() {
        let (_d, root) = workspace();
        let resolved = resolve_workspace_path(&root, "~/x", SymlinkPolicy::Deny).unwrap();
        assert_eq!(resolved.real, nodepath::join(&[&root, "~", "x"]));
        assert_eq!(resolved.display, "~/x");
    }

    #[test]
    fn allows_nonexistent_tail() {
        let (_d, root) = workspace();
        let resolved =
            resolve_workspace_path(&root, "sub/deep/new-file.txt", SymlinkPolicy::Deny).unwrap();
        assert_eq!(
            resolved.real,
            nodepath::join(&[&root, "sub", "deep", "new-file.txt"])
        );
        assert_eq!(resolved.display, "sub/deep/new-file.txt");
        let fresh = resolve_workspace_path(&root, "a/b/c.txt", SymlinkPolicy::Deny).unwrap();
        assert_eq!(fresh.real, nodepath::join(&[&root, "a", "b", "c.txt"]));
    }

    #[test]
    fn normalizes_dot_and_interior_dotdot() {
        let (_d, root) = workspace();
        let resolved =
            resolve_workspace_path(&root, "sub/../sub/./file.txt", SymlinkPolicy::Deny).unwrap();
        assert_eq!(resolved.display, "sub/file.txt");
        let dot = resolve_workspace_path(&root, ".", SymlinkPolicy::Deny).unwrap();
        assert_eq!(dot.real, root);
        assert_eq!(dot.display, ".");
    }

    #[test]
    fn symlink_policy() {
        let dir = tmpdir();
        let ws = dir.path().join("ws");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(ws.join("real")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(ws.join("real").join("in.txt"), "in").unwrap();
        std::fs::write(outside.join("out.txt"), "out").unwrap();
        #[cfg(windows)]
        {
            if std::os::windows::fs::symlink_dir(&outside, ws.join("link-out")).is_err()
                || std::os::windows::fs::symlink_dir(ws.join("real"), ws.join("link-in")).is_err()
                || std::os::windows::fs::symlink_file(
                    outside.join("out.txt"),
                    ws.join("link-file-out"),
                )
                .is_err()
                || std::os::windows::fs::symlink_file(
                    ws.join("real").join("in.txt"),
                    ws.join("link-file-in"),
                )
                .is_err()
            {
                eprintln!("symlink tests skipped: could not create symlink (EPERM)");
                return;
            }
        }
        #[cfg(unix)]
        {
            if std::os::unix::fs::symlink(&outside, ws.join("link-out")).is_err()
                || std::os::unix::fs::symlink(ws.join("real"), ws.join("link-in")).is_err()
                || std::os::unix::fs::symlink(outside.join("out.txt"), ws.join("link-file-out"))
                    .is_err()
                || std::os::unix::fs::symlink(
                    ws.join("real").join("in.txt"),
                    ws.join("link-file-in"),
                )
                .is_err()
            {
                eprintln!("symlink tests skipped");
                return;
            }
        }
        let root = canonicalize_workspace_root(ws.to_str().unwrap()).unwrap();

        // deny rejects in-workspace symlink
        assert!(resolve_workspace_path(&root, "link-in/in.txt", SymlinkPolicy::Deny).is_err());
        // within-workspace rejects outside dir/file symlinks
        assert!(
            resolve_workspace_path(&root, "link-out/out.txt", SymlinkPolicy::WithinWorkspace)
                .is_err()
        );
        assert!(
            resolve_workspace_path(&root, "link-file-out", SymlinkPolicy::WithinWorkspace).is_err()
        );
        // within-workspace allows inside symlinks and re-walks the target
        let resolved =
            resolve_workspace_path(&root, "link-in/in.txt", SymlinkPolicy::WithinWorkspace)
                .unwrap();
        assert_eq!(resolved.real, nodepath::join(&[&root, "real", "in.txt"]));
        assert_eq!(resolved.display, "real/in.txt");
        let f =
            resolve_workspace_path(&root, "link-file-in", SymlinkPolicy::WithinWorkspace).unwrap();
        assert_eq!(f.real, nodepath::join(&[&root, "real", "in.txt"]));
        let new_tail = resolve_workspace_path(
            &root,
            "link-in/new-file.txt",
            SymlinkPolicy::WithinWorkspace,
        )
        .unwrap();
        assert_eq!(
            new_tail.real,
            nodepath::join(&[&root, "real", "new-file.txt"])
        );
    }
}
