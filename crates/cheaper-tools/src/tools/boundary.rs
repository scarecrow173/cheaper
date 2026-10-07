//! Port of `packages/runtime-pi/src/tools/operations.ts`.
//!
//! Boundary operations: workspace path resolve + denylist + access
//! observation + 同ディレクトリ tmp/rename 原子書き込み + mutation journal。
//! write/edit/ls の canonical ops 群を提供する。

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use cheaper_core::errors::node_io_error;
use cheaper_core::resources::types::{ClaimMode, ResourceScope};
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{
    realpath, resolve_workspace_path, ResolvedWorkspacePath, SymlinkPolicy,
};

use crate::access::{AccessObserver, ResourceScopeOptions};
use crate::journal::MutationJournal;
use crate::walker::{rethrow_as_denied, DeniedError};
use crate::ToolError;

/// `BoundaryOperationsOptions`。
pub struct BoundaryOperationsOptions<'a> {
    /// `observeAccess`(無ければ `ResourceAccess` から生成される)。
    pub observe_access: Option<&'a AccessObserver<'a>>,
    /// canonical 化済み workspace root。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    pub journal: Option<&'a MutationJournal>,
    /// privacy deny list。match した path への全 access を拒否する。
    pub denylist: Option<&'a PathDenylist>,
    /// `ResourceScopeOptions`(共通 `ResourceAccess` 構築用)。
    pub scope: ResourceScopeOptions,
}

// ---------------------------------------------------------------------------
// `withFileMutationQueue`(Pi `file-mutation-queue.js`): 同一 file の
// mutation を直列化する。key は caller が決める resolved path
// (TS: `realpath(resolve(filePath))`、ENOENT/ENOTDIR は resolved をそのまま)。
// ---------------------------------------------------------------------------

type MutationQueueMap = HashMap<String, Arc<Mutex<()>>>;

fn mutation_queues() -> &'static Mutex<MutationQueueMap> {
    static QUEUES: OnceLock<Mutex<MutationQueueMap>> = OnceLock::new();
    QUEUES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `withFileMutationQueue(filePath, fn)` 相当。`key` は呼び出し側が
/// 解決した absolute path(TS の `realpath || resolved` 相当)。
pub fn with_file_mutation_queue<R>(
    key: &str,
    f: impl FnOnce() -> Result<R, ToolError>,
) -> Result<R, ToolError> {
    let lock: Arc<Mutex<()>> = {
        let mut map = mutation_queues().lock().unwrap_or_else(|p| p.into_inner());
        map.entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    };
    let result = {
        // 同一 key の mutation を直列化する。poisoned mutex も lock を
        // 継続する(TS の promise chain 相当 — 先行 op の失敗で後続を
        // 止めない)。
        let _guard: MutexGuard<'_, ()> = lock.lock().unwrap_or_else(|p| p.into_inner());
        f()
    };
    // 最後の利用者が key を除去する(TS の `fileMutationQueues.get(key)
    // === chainedQueue` 判定相当 — 後続 waiter は Arc clone で strong
    // count を残すため残存する)。
    {
        let mut map = mutation_queues().lock().unwrap_or_else(|p| p.into_inner());
        if Arc::strong_count(&lock) == 1 {
            map.remove(key);
        }
    }
    result
}

/// `realpath(resolve(filePath))` 相当の mutation queue key。
/// core `realpath` は `dunce::canonicalize`(`\\?\` verbatim prefix strip)
/// で Node `realpath` と揃える。ENOENT/ENOTDIR 時は resolved をそのまま。
pub fn mutation_queue_key(root: &str, path: &str) -> String {
    let absolute = cheaper_core::nodepath::resolve(root, path);
    realpath(&absolute).unwrap_or(absolute)
}

/// 遅延 observer: `observeAccess` があればそれ、無ければ options から
/// `ResourceAccess` を作り `observe` へ委譲する(TS `observer()`)。
enum Observer<'a> {
    Borrowed(&'a AccessObserver<'a>),
    Owned(Box<AccessObserver<'a>>),
}

impl<'a> Observer<'a> {
    fn call(
        &self,
        path: &str,
        mode: ClaimMode,
        scope: Option<ResourceScope>,
        support: bool,
    ) -> Result<(), ToolError> {
        match self {
            Self::Borrowed(f) => f(path, mode, scope, support),
            Self::Owned(f) => f(path, mode, scope, support),
        }
    }
}

/// `resolve` helper: `resolveWorkspacePath` + `rethrowAsDenied` + denylist。
fn resolve(
    opts: &BoundaryOperationsOptions<'_>,
    absolute_path: &str,
) -> Result<ResolvedWorkspacePath, ToolError> {
    let resolved = resolve_workspace_path(opts.root, absolute_path, opts.symlink_policy)
        .map_err(|e| rethrow_as_denied(Box::new(e)))?;
    if opts
        .denylist
        .is_some_and(|d| d.is_denied(&resolved.display))
    {
        return Err(Box::new(DeniedError::policy()));
    }
    Ok(resolved)
}

fn build_observer<'a>(
    opts: &BoundaryOperationsOptions<'a>,
    names: &'a [cheaper_core::tools::vocabulary::ToolName],
) -> Result<Observer<'a>, ToolError> {
    if let Some(observe) = opts.observe_access {
        return Ok(Observer::Borrowed(observe));
    }
    let access = crate::access::ResourceAccess::new(
        crate::access::ScopeAccessOptions {
            scope: ResourceScopeOptions {
                resource_manifest: opts.scope.resource_manifest.clone(),
                write_scope: opts.scope.write_scope.clone(),
                read_strictness: opts.scope.read_strictness,
            },
            root: opts.root.to_string(),
            symlink_policy: opts.symlink_policy,
            names: names.to_vec(),
            denylist: opts.denylist.cloned(),
            // observer 用途では `operation()` の catalog 宣言導出は呼ばれ
            // ないが、構造上必須のため default policy で構築する。
            catalog: crate::catalog::create_tool_catalog(None)?,
        },
        None,
    )?;
    let access = Arc::new(access);
    Ok(Observer::Owned(Box::new(move |p, mode, scope, support| {
        access.observe(p, mode, scope, None, support)
    })))
}

/// write/edit/mkdir 用: boundary+deny のうえ write scope membership を
/// 要求する(`resolveForWrite`)。
fn resolve_for_write(
    opts: &BoundaryOperationsOptions<'_>,
    observer: &Observer<'_>,
    absolute_path: &str,
) -> Result<ResolvedWorkspacePath, ToolError> {
    let resolved = resolve(opts, absolute_path)?;
    // `observed.observeAccess(resolved.real, "exclusive")` — observed は
    // 解決済み observer(提供値 or ResourceAccess 生成値)。
    observer.call(&resolved.real, ClaimMode::Exclusive, None, false)?;
    Ok(resolved)
}

/// `.{basename}.cheaper-tmp-<rand36>`(DESIGN-0002)。
fn tmp_candidate(target: &str) -> String {
    let dir = Path::new(target)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let base = Path::new(target)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // `randomInt(0, 2**31).toString(36)`。
    let rand = fastrand::u32(0..(1 << 31));
    let mut n = rand;
    let mut s = String::new();
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    loop {
        s.insert(0, DIGITS[(n % 36) as usize] as char);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    dir.join(format!(".{base}.cheaper-tmp-{s}"))
        .to_string_lossy()
        .into_owned()
}

/// `atomicWrite`: 同ディレクトリ tmp + rename(DESIGN-0002)。tmp path も
/// boundary 判定を通し、失敗時は finally で tmp を unlink する。
fn atomic_write(
    opts: &BoundaryOperationsOptions<'_>,
    observer: &Observer<'_>,
    absolute_path: &str,
    content: &str,
) -> Result<(), ToolError> {
    let resolved = resolve_for_write(opts, observer, absolute_path)?;
    if let Some(journal) = opts.journal {
        journal.capture_before(&resolved.real, &resolved.display);
    }
    // tmp は target 認可済み write の実装詳細のため scope check は通さない
    // (deny check は resolve 内で引き続き有効)。
    let tmp_path = tmp_candidate(&resolved.real);
    let tmp_resolved = resolve(opts, &tmp_path)?;
    let tmp = tmp_resolved.real;
    // Node fs error の `{code,message}` shape を保つため `node_io_error`
    // で wrap する(TS では `writeFile`/`rename` の UVException がそのまま
    // 伝播する)。
    let result = (|| -> Result<(), ToolError> {
        fs::write(&tmp, content)
            .map_err(|e| Box::new(node_io_error("open", &tmp, e)) as ToolError)?;
        fs::rename(&tmp, &resolved.real)
            .map_err(|e| Box::new(node_io_error("rename", &tmp, e)) as ToolError)?;
        Ok(())
    })();
    // finally: tmp unlink(best effort)。
    let _ = fs::remove_file(&tmp);
    result
}

/// `WriteOperations`。
pub struct WriteOperations<'a> {
    opts: BoundaryOperationsOptions<'a>,
    observer: Observer<'a>,
}

/// `createWriteOperations`。observer の `names` は `["write"]`。
pub fn create_write_operations(
    opts: BoundaryOperationsOptions<'_>,
) -> Result<WriteOperations<'_>, ToolError> {
    let observer = build_observer(&opts, &[cheaper_core::tools::vocabulary::ToolName::Write])?;
    Ok(WriteOperations { opts, observer })
}

impl WriteOperations<'_> {
    /// canonical 化済み workspace root(tool 側の `resolveToCwd` 相当で
    /// 必要)。
    pub fn root(&self) -> &str {
        self.opts.root
    }

    /// `ops.writeFile(absolutePath, content)`(atomic write + journal)。
    pub fn write_file(&self, absolute_path: &str, content: &str) -> Result<(), ToolError> {
        atomic_write(&self.opts, &self.observer, absolute_path, content)
    }

    /// `ops.mkdir(dir)`: scope path の祖先 dir に限り許可(`support=true`)。
    pub fn mkdir(&self, dir: &str) -> Result<(), ToolError> {
        let resolved = resolve(&self.opts, dir)?;
        self.observer.call(
            &resolved.real,
            ClaimMode::Exclusive,
            Some(ResourceScope::Exact),
            true,
        )?;
        fs::create_dir_all(&resolved.real)
            .map_err(|e| Box::new(node_io_error("mkdir", &resolved.real, e)) as ToolError)
    }
}

/// `EditOperations`。
pub struct EditOperations<'a> {
    opts: BoundaryOperationsOptions<'a>,
    observer: Observer<'a>,
}

/// `createEditOperations`。observer の `names` は `["edit"]`。
pub fn create_edit_operations(
    opts: BoundaryOperationsOptions<'_>,
) -> Result<EditOperations<'_>, ToolError> {
    let observer = build_observer(&opts, &[cheaper_core::tools::vocabulary::ToolName::Edit])?;
    Ok(EditOperations { opts, observer })
}

impl EditOperations<'_> {
    /// canonical 化済み workspace root。
    pub fn root(&self) -> &str {
        self.opts.root
    }

    /// `ops.readFile(absolutePath) → Buffer`。呼び出し側が utf8 lossy に
    /// decode する。
    pub fn read_file(&self, absolute_path: &str) -> Result<Vec<u8>, ToolError> {
        let resolved = resolve(&self.opts, absolute_path)?;
        self.observer
            .call(&resolved.real, ClaimMode::Shared, None, false)?;
        fs::read(&resolved.real)
            .map_err(|e| Box::new(node_io_error("open", &resolved.real, e)) as ToolError)
    }

    /// `ops.writeFile(absolutePath, content)`(atomic write + journal)。
    pub fn write_file(&self, absolute_path: &str, content: &str) -> Result<(), ToolError> {
        atomic_write(&self.opts, &self.observer, absolute_path, content)
    }

    /// `ops.access(absolutePath, R_OK|W_OK)`。resolve+observe 後、
    /// read+write 双方の権限があるかを確認する。
    pub fn access(&self, absolute_path: &str) -> Result<(), ToolError> {
        let resolved = resolve(&self.opts, absolute_path)?;
        self.observer
            .call(&resolved.real, ClaimMode::Shared, None, false)?;
        check_read_write(&resolved.real)
    }
}

/// `access(path, R_OK|W_OK)` 相当。Node は権限不足で `EACCES`/`EROFS` を
/// 投げる。Unix では Node が呼ぶのと同じ `access(2)` をそのまま使い、
/// real uid/gid・mount 時の readonly(EROFS)・dir への R_OK|W_OK 通過
/// まで semantic を一致させる。
#[cfg(unix)]
fn check_read_write(path: &str) -> Result<(), ToolError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    // NUL 込み path は Node が `ERR_INVALID_ARG_VALUE` で事前に弾く。
    // resolve 済み path が NUL を含むことは実用上ないが、CString 化
    // 不能時は Node と同じく access 失敗側の error に畳む。
    let c = match CString::new(Path::new(path).as_os_str().as_bytes()) {
        Ok(c) => c,
        Err(_) => {
            return Err(Box::new(node_io_error(
                "access",
                path,
                io::Error::from(io::ErrorKind::InvalidInput),
            )) as ToolError);
        }
    };
    // SAFETY: `c` は NUL 終端の有効な buffer。`access(2)` は fd を
    // 持たず path だけを読むため同期 call で完結する。
    if unsafe { libc::access(c.as_ptr(), libc::R_OK | libc::W_OK) } == 0 {
        Ok(())
    } else {
        Err(Box::new(node_io_error("access", path, io::Error::last_os_error())) as ToolError)
    }
}

/// Windows 近似: `access(2)` 相当 syscall は無いため r/w open で近似的に
/// 判定する。`R_OK|W_OK` の real-uid 評価・ACL の fine-grained 判定
/// (write-only file への R_OK 拒否等)は `access(2)` と一致しない点を
/// 許容差異とする。dir への r/w open は `IsADirectory` で失敗するが、
/// Node の `access` は dir に対しても R_OK|W_OK を通すため dir は通す。
#[cfg(not(unix))]
fn check_read_write(path: &str) -> Result<(), ToolError> {
    match fs::OpenOptions::new().read(true).write(true).open(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::IsADirectory => Ok(()),
        Err(e) => {
            // libuv の Windows `access` は write 要求が readonly
            // attribute で失敗すると `EPERM` を返す(open 由来の
            // `ERROR_ACCESS_DENIED`→`EACCES` とは別 code)。
            let readonly = e.kind() == io::ErrorKind::PermissionDenied
                && fs::metadata(path)
                    .map(|m| m.permissions().readonly())
                    .unwrap_or(false);
            if readonly {
                Err(Box::new(cheaper_core::errors::node_io_error_with_code(
                    "access", path, "EPERM", e,
                )) as ToolError)
            } else {
                Err(Box::new(node_io_error("access", path, e)) as ToolError)
            }
        }
    }
}

/// `LsOperations`。
pub struct LsOperations<'a> {
    opts: BoundaryOperationsOptions<'a>,
    observer: Observer<'a>,
}

/// `createLsOperations`。observer の `names` は `["ls"]`。
pub fn create_ls_operations(
    opts: BoundaryOperationsOptions<'_>,
) -> Result<LsOperations<'_>, ToolError> {
    let observer = build_observer(&opts, &[cheaper_core::tools::vocabulary::ToolName::Ls])?;
    Ok(LsOperations { opts, observer })
}

/// `ops.stat` の返り値最小面(`{ isDirectory() }`)。
pub struct LsStat {
    is_dir: bool,
}

impl LsStat {
    pub fn is_directory(&self) -> bool {
        self.is_dir
    }
}

impl LsOperations<'_> {
    /// canonical 化済み workspace root。
    pub fn root(&self) -> &str {
        self.opts.root
    }

    /// `ops.exists(absolutePath)`: boundary+observe 後 `access(F_OK)`。
    /// observe/resolve 自体の error は伝播する(TS と同じ)。
    pub fn exists(&self, absolute_path: &str) -> Result<bool, ToolError> {
        let resolved = resolve(&self.opts, absolute_path)?;
        self.observer
            .call(&resolved.real, ClaimMode::Shared, None, false)?;
        // `access(F_OK)` — 存在すれば true。権限不足等の error は
        // TS では catch → false に畳まれる。
        Ok(Path::new(&resolved.real).try_exists().unwrap_or(false))
    }

    /// `ops.stat(absolutePath)`。
    pub fn stat(&self, absolute_path: &str) -> Result<LsStat, ToolError> {
        let resolved = resolve(&self.opts, absolute_path)?;
        self.observer
            .call(&resolved.real, ClaimMode::Shared, None, false)?;
        let md = fs::metadata(&resolved.real)
            .map_err(|e| Box::new(node_io_error("stat", &resolved.real, e)) as ToolError)?;
        Ok(LsStat {
            is_dir: md.is_dir(),
        })
    }

    /// `ops.readdir(absolutePath)`: subtree scope で observe し、deny
    /// entry は listing に出さない(存在秘匿)。
    pub fn readdir(&self, absolute_path: &str) -> Result<Vec<String>, ToolError> {
        let resolved = resolve(&self.opts, absolute_path)?;
        self.observer.call(
            &resolved.real,
            ClaimMode::Shared,
            Some(ResourceScope::Subtree),
            false,
        )?;
        let entries = fs::read_dir(&resolved.real)
            .map_err(|e| Box::new(node_io_error("scandir", &resolved.real, e)) as ToolError)?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|e| Box::new(node_io_error("scandir", &resolved.real, e)) as ToolError)?;
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        let Some(denylist) = self.opts.denylist else {
            return Ok(names);
        };
        let prefix = if resolved.display == "." {
            String::new()
        } else {
            format!("{}/", resolved.display)
        };
        Ok(names
            .into_iter()
            .filter(|name| !denylist.is_denied(&format!("{prefix}{name}")))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use cheaper_core::workspace::deny_paths::create_path_denylist;
    use cheaper_core::workspace::paths::canonicalize_workspace_root;

    use super::*;
    use crate::journal::MutationJournal;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-boundary-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn opts<'a>(
        root: &'a str,
        journal: Option<&'a MutationJournal>,
        denylist: Option<&'a PathDenylist>,
    ) -> BoundaryOperationsOptions<'a> {
        BoundaryOperationsOptions {
            observe_access: None,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            journal,
            denylist,
            scope: ResourceScopeOptions::default(),
        }
    }

    #[test]
    fn fs_boundary_journal_capture_before_write() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "old\n").unwrap();
        let journal = MutationJournal::new();
        let ops = create_write_operations(opts(&root, Some(&journal), None)).unwrap();
        let target = dir.join("f.txt").to_string_lossy().into_owned();
        ops.write_file(&target, "new\n").unwrap();
        let summary = journal.finalize();
        assert_eq!(summary.totals.files, 1);
        assert_eq!(summary.files[0].path, "f.txt");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_boundary_denied_write_does_not_capture_journal() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let journal = MutationJournal::new();
        let denylist = create_path_denylist(&[]);
        let ops = create_write_operations(opts(&root, Some(&journal), Some(&denylist))).unwrap();
        let target = dir.join(".env").to_string_lossy().into_owned();
        assert!(ops.write_file(&target, "SECRET=1").is_err());
        assert!(journal.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_boundary_escape_denied() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let ops = create_write_operations(opts(&root, None, None)).unwrap();
        let err = ops
            .write_file(&format!("{}/../out.txt", dir.to_string_lossy()), "x")
            .unwrap_err();
        assert_eq!(err.to_string(), crate::walker::ACCESS_DENIED_MESSAGE);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn mutation_queue_serializes_same_key() {
        // 同一 key を同時に走らせても critical section は直列化される。
        let key = format!("queue-key-{}", fastrand::u64(..));
        let in_flight = AtomicUsize::new(0);
        let max_seen = AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| {
                    with_file_mutation_queue(&key, || -> Result<(), ToolError> {
                        let n = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                        max_seen.fetch_max(n, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(5));
                        in_flight.fetch_sub(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .unwrap();
                });
            }
        });
        assert_eq!(max_seen.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mutation_queue_key_fallback_for_missing_file() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let missing = format!("{}/missing.txt", dir.to_string_lossy());
        // 存在しない file は lexical resolved を使う。
        let key = mutation_queue_key(&root, &missing);
        assert!(key.contains("missing.txt"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
