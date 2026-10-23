//! Port of `packages/tool-runtime/src/process-executable.ts`.
//!
//! native argv0 解決だけを所有し、shell・PATH 子 env・任意 spawn API は
//! 提供しない。FS アクセスは `std::fs` 同期実装(walker 確立パターン)。
//!
//! executable の内容は審査しない — spawn viability(realpath・
//! is_file・win32 `.exe`/`.com`・X_OK)のみを判定し、bare `node` を
//! 含む任意名は通常の PATH 解決経路を通る。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{realpath, resolve_workspace_path, SymlinkPolicy};

use crate::ToolError;

/// `resolveProcessExecutable` の host 解決 context。子 env へは継承しない。
/// TS の `execPath` option は Rust 版に存在しない(設計固定ルール)。
pub struct ProcessExecutableOptions<'a> {
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    pub denylist: Option<&'a PathDenylist>,
    /// `options.searchPath ?? process.env.PATH` 相当。`None` で親 `PATH`。
    pub search_path: Option<&'a str>,
}

/// `subject` は error message の宣言種別を表す呼出し側 label
/// (`resolve_process_executable` の `subject` 引数)。
fn unsupported(subject: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::PolicyViolation,
        format!("{subject} could not be resolved to a spawnable executable"),
    )
}

/// `options.searchPath ?? process.env.PATH ?? ""` 相当。
fn search_path(option: Option<&str>) -> String {
    option
        .map(str::to_string)
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default()
}

/// `fs.access(resolved, constants.X_OK)` 相当。
fn x_ok(resolved: &str) -> bool {
    #[cfg(unix)]
    {
        let c = match std::ffi::CString::new(resolved) {
            Ok(c) => c,
            Err(_) => return false,
        };
        // SAFETY: `c` is a valid NUL-terminated path string.
        unsafe { libc::access(c.as_ptr(), libc::X_OK) == 0 }
    }
    #[cfg(not(unix))]
    {
        // libuv は win32 で X_OK を file 存在の確認として扱う
        // (`isFile` 判定済みのため常に成功相当)。
        let _ = resolved;
        true
    }
}

/// `native(candidate)`: spawn viability 判定 — realpath → isFile →
/// win32 .exe/.com → X_OK。どれか一つでも不成立なら `None`
/// (fail closed)。
fn native(candidate: &str) -> Option<String> {
    let resolved = realpath(candidate).ok()?;
    let meta = std::fs::metadata(&resolved).ok()?;
    if !meta.is_file() {
        return None;
    }
    if cfg!(windows) {
        let lower = resolved.to_lowercase();
        if !(lower.ends_with(".exe") || lower.ends_with(".com")) {
            return None;
        }
    }
    if !x_ok(&resolved) {
        return None;
    }
    Some(resolved)
}

/// pin 済み executable の spawn 前再検証用 `native()` wrapper
/// (design §(b) — `command` grant の pin path 差替え fail-closed)。
/// `native()` の spawn viability 検証(realpath → is_file → win32
/// ext → X_OK)を再適用し、resolved canonical path を返す。
/// 不成立なら `None`(fail closed)。
pub(crate) fn native_executable(candidate: &str) -> Option<String> {
    native(candidate)
}

/// `resolveProcessExecutable`。declared executable を workspace-
/// relative path / absolute path / ambient PATH の順で spawn viability
/// 判定へ解決する。executable 名・内容は審査しない — 解決不能のみ
/// `POLICY_VIOLATION`。
///
/// `subject` は error message の宣言種別 label(`"process"` →
/// `"process could not be resolved to a spawnable executable"`)。
/// `process` 経路は `"process"` を渡し、`command` 経路(T3)は
/// grant 文脈の label を渡す(design §(b))。
pub fn resolve_process_executable(
    command: &str,
    options: &ProcessExecutableOptions<'_>,
    subject: &str,
) -> Result<String, ToolError> {
    if command.contains('/') || command.contains('\\') || nodepath::is_absolute(command) {
        let candidate = if nodepath::is_absolute(command) {
            command.to_string()
        } else {
            let resolved = resolve_workspace_path(options.root, command, options.symlink_policy)?;
            if let Some(denylist) = options.denylist {
                if denylist.is_denied(&resolved.display) {
                    return Err(unsupported(subject).into());
                }
            }
            resolved.real
        };
        return match native(&candidate) {
            Some(found) => Ok(found),
            None => Err(unsupported(subject).into()),
        };
    }
    let candidates: Vec<String> = if cfg!(windows) && nodepath::extname(command).is_empty() {
        vec![format!("{command}.exe"), format!("{command}.com")]
    } else {
        vec![command.to_string()]
    };
    for directory in search_path(options.search_path).split(if cfg!(windows) { ';' } else { ':' }) {
        if !nodepath::is_absolute(directory) {
            continue;
        }
        for name in &candidates {
            if let Some(found) = native(&nodepath::join(&[directory, name])) {
                return Ok(found);
            }
        }
    }
    Err(unsupported(subject).into())
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use cheaper_core::workspace::paths::canonicalize_workspace_root;

    use super::*;

    /// walker 確立の tmpdir パターン(`std::env::temp_dir` + `fastrand`)。
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!(
                "cheaper-proc-exe-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            std::fs::create_dir_all(&base).unwrap();
            Self(base)
        }

        /// Canonical workspace root(dunce realpath 済み)。
        fn root(&self) -> String {
            canonicalize_workspace_root(self.0.to_str().unwrap()).unwrap()
        }

        fn join(&self, p: &str) -> PathBuf {
            self.0.join(p)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write(path: &Path, bytes: &[u8]) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(bytes).unwrap();
    }

    /// `options()` fixture(`searchPath: ""` で PATH を遮断)。
    fn options(root: &str) -> ProcessExecutableOptions<'_> {
        ProcessExecutableOptions {
            root,
            symlink_policy: SymlinkPolicy::Deny,
            denylist: None,
            search_path: Some(""),
        }
    }

    /// 実在 executable file の fixture bytes — 内容は検査対象では
    /// なく、file の実在性と実行 bit だけが意味を持つ。
    fn executable_file_bytes() -> Vec<u8> {
        if cfg!(windows) {
            b"MZbinary".to_vec()
        } else {
            vec![0x7f, 0x45, 0x4c, 0x46]
        }
    }

    fn set_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let _ = path;
    }

    fn policy_violation(err: &ToolError) -> bool {
        err.downcast_ref::<CheaperError>()
            .is_some_and(|e| e.code() == ErrorCode::PolicyViolation)
    }

    #[test]
    fn bare_node_resolves_via_search_path() {
        let dir = TempDir::new();
        let root = dir.root();
        // bare `node` は通常の PATH 解決経路を通る — `search_path` 上の
        // 実在 file が解決される。
        let bin_dir = dir.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let node = bin_dir.join(if cfg!(windows) { "node.exe" } else { "node" });
        write(&node, &executable_file_bytes());
        set_executable(&node);
        let opts = ProcessExecutableOptions {
            search_path: Some(bin_dir.to_str().unwrap()),
            ..options(&root)
        };
        let found = resolve_process_executable("node", &opts, "process").unwrap();
        assert_eq!(found, realpath(node.to_str().unwrap()).unwrap());
        // PATH 遮断(`Some("")`)では解決不能として通常の unsupported
        // error。
        let err = resolve_process_executable("node", &options(&root), "process").unwrap_err();
        assert!(policy_violation(&err));
    }

    /// subject 引数は error message の宣言種別 label になる —
    /// `command` 経路(T3)で "process" wording を出さないための
    /// parameterize(design §(b))。
    #[test]
    fn error_subject_is_parameterized_by_caller() {
        let dir = TempDir::new();
        let root = dir.root();
        let err = resolve_process_executable("missing-tool", &options(&root), "command grant")
            .unwrap_err();
        assert!(policy_violation(&err));
        assert!(err
            .to_string()
            .contains("command grant could not be resolved"));
        assert!(!err.to_string().contains("process could not"));
        let err =
            resolve_process_executable("missing-tool", &options(&root), "command").unwrap_err();
        assert!(err
            .to_string()
            .contains("command could not be resolved to a spawnable executable"));
    }

    #[test]
    fn resolves_executable_from_search_path() {
        let dir = TempDir::new();
        let root = dir.root();
        let bin_dir = dir.join("native");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join(if cfg!(windows) { "tool.exe" } else { "tool" });
        write(&bin, &executable_file_bytes());
        set_executable(&bin);
        let opts = ProcessExecutableOptions {
            search_path: Some(bin_dir.to_str().unwrap()),
            ..options(&root)
        };
        let found = resolve_process_executable("tool", &opts, "process").unwrap();
        assert_eq!(found, realpath(bin.to_str().unwrap()).unwrap());
    }

    #[test]
    fn fails_closed_when_search_path_has_no_supported_executable() {
        let dir = TempDir::new();
        let root = dir.root();
        for command in ["missing-tool", "node.exe"] {
            // `"node.exe"` は literal `node` と別名として PATH 解決を試み、
            // 見つからなければ unsupported。
            let err = resolve_process_executable(command, &options(&root), "process").unwrap_err();
            assert!(policy_violation(&err), "{command}");
        }
    }

    #[test]
    fn script_files_resolve_by_spawn_viability() {
        let dir = TempDir::new();
        let root = dir.root();
        // script file の内容は審査しない — unix では +X file が spawn
        // viability で解決される。
        let shim = dir.join("pnpm.cmd");
        write(&shim, b"echo shim");
        set_executable(&shim);
        let resolved =
            resolve_process_executable(shim.to_str().unwrap(), &options(&root), "process");
        #[cfg(unix)]
        assert_eq!(resolved.unwrap(), realpath(shim.to_str().unwrap()).unwrap());
        // win32 では `.cmd`(非 `.exe`/`.com`)は引き続き viability で
        // reject される(CreateProcess が直接 spawn できる拡張子の制約)。
        #[cfg(windows)]
        assert!(policy_violation(&resolved.unwrap_err()));
        let text = dir.join(if cfg!(windows) {
            "script.exe"
        } else {
            "script"
        });
        write(&text, b"#!/bin/sh\necho hidden-shell\n");
        set_executable(&text);
        // unix の +X text file・win32 の `.exe` 拡張子 file 共に解決
        // される — file 内容は判定対象外。
        let found =
            resolve_process_executable(text.to_str().unwrap(), &options(&root), "process").unwrap();
        assert_eq!(found, realpath(text.to_str().unwrap()).unwrap());
    }

    #[test]
    fn resolves_workspace_relative_and_absolute_executable_paths() {
        let dir = TempDir::new();
        let root = dir.root();
        let rel_name = if cfg!(windows) {
            "native\\bin.exe"
        } else {
            "native/bin"
        };
        let bin = dir.join(if cfg!(windows) {
            "native/bin.exe"
        } else {
            "native/bin"
        });
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        write(&bin, &executable_file_bytes());
        set_executable(&bin);
        // relative → workspace-validated。
        let found = resolve_process_executable(rel_name, &options(&root), "process").unwrap();
        assert_eq!(found, realpath(bin.to_str().unwrap()).unwrap());
        // absolute → allowed。
        let found =
            resolve_process_executable(bin.to_str().unwrap(), &options(&root), "process").unwrap();
        assert_eq!(found, realpath(bin.to_str().unwrap()).unwrap());
    }

    #[test]
    fn workspace_relative_escape_is_rejected() {
        let dir = TempDir::new();
        let root = dir.root();
        let escape = if cfg!(windows) {
            "..\\outside.exe"
        } else {
            "../outside"
        };
        let err = resolve_process_executable(escape, &options(&root), "process").unwrap_err();
        assert!(err.is::<cheaper_core::workspace::paths::WorkspaceEscapeError>());
    }

    #[test]
    fn unresolvable_fails_closed_regardless_of_content() {
        let dir = TempDir::new();
        let root = dir.root();
        let missing = dir.join(if cfg!(windows) {
            "missing.exe"
        } else {
            "missing"
        });
        let err = resolve_process_executable(missing.to_str().unwrap(), &options(&root), "process")
            .unwrap_err();
        assert!(policy_violation(&err));
        // file の内容は判定対象外 — spawn viability を満たす file は
        // 内容に関わらず解決される。
        let wrong = dir.join(if cfg!(windows) { "wrong.exe" } else { "wrong" });
        write(&wrong, if cfg!(windows) { b"NOPE" } else { b"NOT!" });
        set_executable(&wrong);
        let found = resolve_process_executable(wrong.to_str().unwrap(), &options(&root), "process")
            .unwrap();
        assert_eq!(found, realpath(wrong.to_str().unwrap()).unwrap());
        // 短い file も両 platform で解決される。
        let short = dir.join(if cfg!(windows) { "short.exe" } else { "short" });
        write(&short, &[0x7f, 0x45]);
        set_executable(&short);
        let found = resolve_process_executable(short.to_str().unwrap(), &options(&root), "process")
            .unwrap();
        assert_eq!(found, realpath(short.to_str().unwrap()).unwrap());
    }
}
