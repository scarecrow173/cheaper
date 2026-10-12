//! Port of `packages/tool-runtime/src/process-executable.ts`.
//!
//! native argv0 解決だけを所有し、shell・PATH 子 env・任意 spawn API は
//! 提供しない。FS アクセスは `std::fs` 同期実装(walker 確立パターン)。
//!
//! 設計固定ルール差分: TS は `command === "node"` かつ
//! `basename(execPath) ≈ node` の時に `process.execPath` を native 候補と
//! して使う fast-path を持つが、Rust binary に等価物が無い。Rust 側は
//! bare `node` を PATH 解決せず `POLICY_VIOLATION` の明示 error に
//! 置き換える(`docs/designs/rust-rig-runtime-migration-architecture.md`
//! §tool 移植の固定ルール)。

use std::io::Read;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::policy::process::is_shell_process_executable;
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
        format!(
            "{subject} requires a supported native executable; shell and script shims are unsupported"
        ),
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

/// 4-byte magic 検査(MZ/ELF/Mach-O 系)。TS は partial read を zero-fill で
/// 判定するため、同じく zero-init buffer に読み込む。
fn native_magic_supported(header: &[u8; 4]) -> bool {
    let hex: String = header.iter().map(|b| format!("{b:02x}")).collect();
    if cfg!(windows) {
        return hex.starts_with("4d5a");
    }
    hex == "7f454c46"
        || (cfg!(target_os = "macos")
            && [
                "feedface", "feedfacf", "cefaedfe", "cffaedfe", "cafebabe", "bebafeca", "cafebabf",
                "bfbafeca",
            ]
            .contains(&hex.as_str()))
}

/// `native(candidate)`: realpath → shell/shim → isFile → win32 .exe/.com →
/// X_OK → native magic。どれか一つでも不成立なら `None`(fail closed)。
fn native(candidate: &str) -> Option<String> {
    let resolved = realpath(candidate).ok()?;
    if is_shell_process_executable(&resolved) {
        return None;
    }
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
    // executable text を execvp/kernel の interpreter fallback へ渡さない。
    let mut file = std::fs::File::open(&resolved).ok()?;
    let mut header = [0u8; 4];
    let _ = file.read(&mut header); // partial read → 残り zero-fill(TS parity)
    if native_magic_supported(&header) {
        Some(resolved)
    } else {
        None
    }
}

/// pin 済み executable の spawn 前再検証用 `native()` wrapper
/// (design §(b) — `command` grant の pin path 差替え fail-closed)。
/// `native()` の candidate 検証(realpath → shell/shim → is_file →
/// win32 ext → X_OK → native magic)を再適用し、resolved canonical
/// path を返す。不成立なら `None`(fail closed)。
pub(crate) fn native_executable(candidate: &str) -> Option<String> {
    native(candidate)
}

/// `resolveProcessExecutable`。bare `node` は Rust 版で host Node が
/// 存在しないため PATH 解決せず明示 `POLICY_VIOLATION` error(固定ルール)。
/// PATH 上に実在する `node.exe` があっても解決しない。
///
/// `subject` は error message の宣言種別 label(`"process"` →
/// `"process requires a supported native executable..."`・
/// `"process task \"node\" is unsupported..."`)。`process` 経路は
/// `"process"` を渡して従来 message を維持し、`command` 経路(T3)は
/// grant 文脈の label を渡して `"process task"` を出さないようにする
/// (design §(b))。
pub fn resolve_process_executable(
    command: &str,
    options: &ProcessExecutableOptions<'_>,
    subject: &str,
) -> Result<String, ToolError> {
    if is_shell_process_executable(command) {
        return Err(unsupported(subject).into());
    }
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
    // 設計固定ルール: `process.execPath` fast-path の等価物なし。`node` は
    // PATH 解決しない(候補探索の前に明示 error)。
    if command == "node" {
        return Err(CheaperError::new(
            ErrorCode::PolicyViolation,
            format!(
                "{subject} task \"node\" is unsupported: the Rust runtime has no Node.js host executable"
            ),
        )
        .into());
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

    fn native_bytes() -> Vec<u8> {
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
    fn bare_node_is_an_explicit_policy_violation() {
        let dir = TempDir::new();
        let root = dir.root();
        // PATH 上に実在する node.exe があっても解決しない(固定ルール)。
        let bin_dir = dir.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let node = bin_dir.join(if cfg!(windows) { "node.exe" } else { "node" });
        write(&node, &native_bytes());
        set_executable(&node);
        for search_path in [
            Some(""),
            Some(bin_dir.to_str().unwrap()),
            None, // 親 PATH 継承でも拒否
        ] {
            let opts = ProcessExecutableOptions {
                search_path,
                ..options(&root)
            };
            let err = resolve_process_executable("node", &opts, "process").unwrap_err();
            assert!(policy_violation(&err));
            assert!(err.to_string().contains("\"node\""));
        }
    }

    /// subject 引数は error message の宣言種別 label になる —
    /// `command` 経路(T3)で "process task" wording を出さないための
    /// parameterize(design §(b))。
    #[test]
    fn error_subject_is_parameterized_by_caller() {
        let dir = TempDir::new();
        let root = dir.root();
        let err = resolve_process_executable("node", &options(&root), "command grant").unwrap_err();
        assert!(policy_violation(&err));
        assert!(err.to_string().contains("command grant task"));
        assert!(!err.to_string().contains("process task"));
        let err =
            resolve_process_executable("missing-tool", &options(&root), "command").unwrap_err();
        assert!(err
            .to_string()
            .contains("command requires a supported native executable"));
    }

    #[test]
    fn resolves_native_executable_from_search_path() {
        let dir = TempDir::new();
        let root = dir.root();
        let bin_dir = dir.join("native");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join(if cfg!(windows) { "tool.exe" } else { "tool" });
        write(&bin, &native_bytes());
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
            // 見つからなければ unsupported(固定ルールは bare `node` のみ)。
            let err = resolve_process_executable(command, &options(&root), "process").unwrap_err();
            assert!(policy_violation(&err), "{command}");
        }
    }

    #[test]
    fn refuses_script_shims_and_executable_text() {
        let dir = TempDir::new();
        let root = dir.root();
        let shim = dir.join("pnpm.cmd");
        write(&shim, b"echo shim");
        let err = resolve_process_executable(shim.to_str().unwrap(), &options(&root), "process")
            .unwrap_err();
        assert!(policy_violation(&err));
        let text = dir.join(if cfg!(windows) {
            "script.exe"
        } else {
            "script"
        });
        write(&text, b"#!/bin/sh\necho hidden-shell\n");
        set_executable(&text);
        let err = resolve_process_executable(text.to_str().unwrap(), &options(&root), "process")
            .unwrap_err();
        assert!(policy_violation(&err));
    }

    #[test]
    fn resolves_workspace_relative_and_absolute_native_paths() {
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
        write(&bin, &native_bytes());
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
    fn nonexistent_or_unsupported_magic_fails_closed() {
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
        // native magic と一致しない file は reject。
        let wrong = dir.join(if cfg!(windows) { "wrong.exe" } else { "wrong" });
        write(&wrong, if cfg!(windows) { b"NOPE" } else { b"NOT!" });
        set_executable(&wrong);
        let err = resolve_process_executable(wrong.to_str().unwrap(), &options(&root), "process")
            .unwrap_err();
        assert!(policy_violation(&err));
        // parity note: TS は `Buffer.alloc(4)` への partial read を
        // zero-fill のまま hex 判定するため、Windows で先頭が "MZ" の
        // short file は accepted("4d5a0000".startsWith("4d5a"))。
        #[cfg(windows)]
        {
            let short = dir.join("short.exe");
            write(&short, b"MZ");
            assert!(resolve_process_executable(
                short.to_str().unwrap(),
                &options(&root),
                "process"
            )
            .is_ok());
        }
        // POSIX の ELF は先頭 byte だけでは一致しない。
        #[cfg(unix)]
        {
            let short = dir.join("short");
            write(&short, &[0x7f, 0x45]);
            set_executable(&short);
            let err =
                resolve_process_executable(short.to_str().unwrap(), &options(&root), "process")
                    .unwrap_err();
            assert!(policy_violation(&err));
        }
    }
}
