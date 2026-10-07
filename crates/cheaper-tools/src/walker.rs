//! Port of `packages/tool-runtime/src/walker.ts`.
//!
//! `walkFiles` の async generator は pull ベースの [`Iterator`] として
//! 再現する。`next()` 1 回が高々 1 dirent ぶんの FS 処理を行い、呼出し側が
//! 途中で止めると walk 全体が止まる(`for await` + `break` と同じ意味論)。
//! FS アクセスは `cheaper-core` と同じ `std::fs` 同期実装とし、
//! `observeAccess` の async callback は Rust 側では同期 `Fn` になる
//! (`access::ResourceAccess::observe` の処理は全て同期 FS で完結するため)。

use std::collections::HashSet;
use std::fs;
use std::io::Read;

use cheaper_core::nodepath;
use cheaper_core::resources::types::{ClaimMode, ResourceScope};
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{
    is_inside_workspace_root, realpath, resolve_workspace_path, SymlinkPolicy, WorkspaceEscapeError,
};

use crate::access::AccessObserver;
use crate::limits::SNIFF_BYTES;
use crate::ToolError;

pub const ACCESS_DENIED_MESSAGE: &str = "Access denied: path is outside the workspace";
pub const POLICY_DENIED_MESSAGE: &str = "Access denied: path is denied by policy";
pub const SCOPE_DENIED_MESSAGE: &str = "Access denied: path is outside the write scope";

/// `new Error(<deny message>)` 相当の plain error。deny 3 定数のいずれかを
/// 運ぶ(`is_access_denied_error` の message 一致判定と対になる唯一の
/// 生成経路)。
#[derive(Debug)]
pub struct DeniedError {
    message: &'static str,
}

impl DeniedError {
    /// `new Error(ACCESS_DENIED_MESSAGE)` 相当。
    pub fn access() -> Self {
        Self {
            message: ACCESS_DENIED_MESSAGE,
        }
    }

    /// `new Error(POLICY_DENIED_MESSAGE)` 相当。
    pub fn policy() -> Self {
        Self {
            message: POLICY_DENIED_MESSAGE,
        }
    }

    /// `new Error(SCOPE_DENIED_MESSAGE)` 相当。
    pub fn scope() -> Self {
        Self {
            message: SCOPE_DENIED_MESSAGE,
        }
    }
}

impl std::fmt::Display for DeniedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for DeniedError {}

/// tool 層の escape → error result 変換(`rethrowAsDenied`)。
/// `WorkspaceEscapeError` は ACCESS_DENIED error に写像し、それ以外の
/// error はそのまま返す。TS は `throw` するため、呼出し側では
/// `.map_err(|e| rethrow_as_denied(e.into()))` 等で伝播させる。
pub fn rethrow_as_denied(e: ToolError) -> ToolError {
    if e.downcast_ref::<WorkspaceEscapeError>().is_some() {
        return Box::new(DeniedError::access());
    }
    e
}

const DENIED_MESSAGES: [&str; 3] = [
    ACCESS_DENIED_MESSAGE,
    POLICY_DENIED_MESSAGE,
    SCOPE_DENIED_MESSAGE,
];

/// boundary/policy/scope 拒否の error result か(rejectedToolCalls 計上判定に
/// 使用)。TS は `e instanceof Error && DENIED_MESSAGES.has(e.message)` —
/// message 一致が判定 contract なので Display 文字列で比較する。
pub fn is_access_denied_error(e: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    DENIED_MESSAGES.contains(&e.to_string().as_str())
}

const SKIP_DIRS: [&str; 2] = [".git", "node_modules"];

/// `WalkOptions`。
pub struct WalkOptions<'a> {
    /// `observeAccess`(access 層の観測 callback)。
    pub observe_access: Option<&'a AccessObserver<'a>>,
    /// canonical 化済み workspace root。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    /// privacy deny list。deny file は yield せず deny dir には潜らない。
    pub denylist: Option<&'a PathDenylist>,
}

/// `WalkedFile`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedFile {
    /// I/O に使う実 path(symlink 解決済み)。
    pub real: String,
    /// workspace 相対 POSIX 区切り(model 向け表示)。
    pub display: String,
}

/// `WalkIssue["reason"]`: `"unreadable"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkIssueReason {
    Unreadable,
}

impl WalkIssueReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
        }
    }
}

/// 列挙失敗の通知(`WalkIssue`)。deny 対象は filter 段で消えるためここには
/// 載らない(存在秘匿)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkIssue {
    /// workspace 相対 POSIX 区切り(model 向け表示)。
    pub display: String,
    pub reason: WalkIssueReason,
}

/// `WalkIssueHandler`。
pub type WalkIssueHandler<'a> = dyn FnMut(WalkIssue) + 'a;

fn to_display(root: &str, logical_abs: &str) -> String {
    let rel = nodepath::relative(root, logical_abs);
    if rel.is_empty() {
        ".".to_string()
    } else {
        rel.split(nodepath::SEP).collect::<Vec<_>>().join("/")
    }
}

/// `sniffTextFile` の戻り値(`"text" | "binary" | "unreadable"`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SniffResult {
    Text,
    Binary,
    Unreadable,
}

impl SniffResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Binary => "binary",
            Self::Unreadable => "unreadable",
        }
    }
}

/// `sniffTextFile`: 先頭 `SNIFF_BYTES` の NUL 走査による text/binary 判定
/// (DESIGN-0019)。bounded 判定で size 判定は持たない。open/read 失敗は
/// `Unreadable` を返す(fail closed だが理由を区別する)。`observe_access`
/// の拒否は `Err` として呼出し側へ伝播する(TS の throw parity)。
pub fn sniff_text_file(
    real: &str,
    observe_access: Option<&AccessObserver<'_>>,
) -> Result<SniffResult, ToolError> {
    if let Some(observe) = observe_access {
        observe(real, ClaimMode::Shared, None, false)?;
    }
    let mut fh = match fs::File::open(real) {
        Ok(fh) => fh,
        Err(_) => return Ok(SniffResult::Unreadable),
    };
    let mut buf = vec![0u8; SNIFF_BYTES];
    // TS は `fh.read(buf, 0, SNIFF_BYTES, 0)` の単発 read。partial read は
    // そのまま読み取り分のみで判定する(同じく単発 read)。
    let bytes_read = match fh.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return Ok(SniffResult::Unreadable),
    };
    Ok(if buf[..bytes_read].contains(&0) {
        SniffResult::Binary
    } else {
        SniffResult::Text
    })
}

/// win32 の FS 大小文字非依存を boundary 比較に反映する。POSIX では厳密比較。
const CASE_INSENSITIVE_FS: bool = cfg!(windows);

/// `walkDir` 呼出し 1 回の結果。
enum Descend {
    /// stat が通常 file であったため `WalkedFile` を emit した。
    File(WalkedFile),
    /// dir frame を stack に push した。
    Descended,
    /// deny/stat 失敗/cycle/非 dir で何もしない。
    Skipped,
}

/// opendir 中の 1 dir(保持は `ReadDir` iterator 1 件 = O(1) entries)。
struct DirFrame {
    entries: fs::ReadDir,
    fs_path: String,
    logical_path: String,
    /// `ancestors` に登録した canonical path。frame 解放時に除去する
    /// (TS の try/finally `ancestors.delete(canonical)`)。
    canonical: String,
}

/// 自前 grep/find 共通の sandboxed walker(DESIGN-0002 / DESIGN-0019)の
/// pull ベース再現。許可された通常 file を size・内容に関係なく path 列挙
/// する。text eligibility は呼び出し側が [`sniff_text_file`] で判定する。
/// `.git`/`node_modules` skip、symlink policy、denylist、祖先チェーン
/// による cycle 防止を維持し、列挙失敗(stat/readdir/realpath 失敗・
/// dangling symlink)は `on_issue` に流す。deny 対象は結果にも
/// `on_issue` にも出さない(存在秘匿)。
/// 開始 path は `resolve_workspace_path` で boundary 判定し、失敗は
/// 従来どおり `Err(WorkspaceEscapeError)` で返す。
/// cycle 防止は祖先チェーン(canonical dir path)のみで判定し、同一実 dir の
/// 別名(symlink)経由の重複結果は許容する。
pub struct WalkFiles<'o, 'i> {
    opts: &'o WalkOptions<'o>,
    on_issue: Option<&'i mut WalkIssueHandler<'i>>,
    ancestors: HashSet<String>,
    stack: Vec<DirFrame>,
    /// 次に処理する `walkDir` 呼出し相当 `(fs_path, logical_path)`。
    /// entry が dir/symlink のときここへ積んで depth-first に潜る。
    pending: Option<(String, String)>,
    /// error item を返した後は fuse する(generator の throw = 列挙終了)。
    done: bool,
}

/// `walkFiles`。`resolve_workspace_path` 失敗は
/// `Err(WorkspaceEscapeError)`(`rethrow_as_denied` 変換は呼出し側の責務)。
/// deny 済みの開始 path は何も列挙しない iterator となる(存在秘匿)。
pub fn walk_files<'o, 'i>(
    opts: &'o WalkOptions<'o>,
    start_abs: &str,
    on_issue: Option<&'i mut WalkIssueHandler<'i>>,
) -> Result<WalkFiles<'o, 'i>, WorkspaceEscapeError> {
    let start = resolve_workspace_path(opts.root, start_abs, opts.symlink_policy)?;
    let pending = if opts.denylist.is_some_and(|d| d.is_denied(&start.display)) {
        None
    } else {
        Some((start.real.clone(), start.real))
    };
    Ok(WalkFiles {
        opts,
        on_issue,
        ancestors: HashSet::new(),
        stack: Vec::new(),
        pending,
        done: false,
    })
}

impl<'o, 'i> WalkFiles<'o, 'i> {
    fn observe(
        &self,
        path: &str,
        mode: ClaimMode,
        scope: Option<ResourceScope>,
    ) -> Result<(), ToolError> {
        match self.opts.observe_access {
            Some(observe) => observe(path, mode, scope, false),
            None => Ok(()),
        }
    }

    fn issue(&mut self, display: String) {
        if let Some(on_issue) = self.on_issue.as_deref_mut() {
            on_issue(WalkIssue {
                display,
                reason: WalkIssueReason::Unreadable,
            });
        }
    }

    fn is_denied(&self, display: &str) -> bool {
        self.opts.denylist.is_some_and(|d| d.is_denied(display))
    }

    /// `walkDir(fsPath, logicalPath)` の本体。dir なら frame を push し、
    /// 通常 file なら `WalkedFile` を返す。
    fn descend(&mut self, fs_path: &str, logical_path: &str) -> Result<Descend, ToolError> {
        let display = || to_display(self.opts.root, logical_path);
        // alias 経由でも実 path の deny entry は観測・I/O・存在通知の前に
        // 除外する。
        if self.is_denied(&to_display(self.opts.root, fs_path)) {
            return Ok(Descend::Skipped);
        }
        self.observe(fs_path, ClaimMode::Shared, None)?;
        let st = match fs::metadata(fs_path) {
            Ok(st) => st,
            Err(_) => {
                self.issue(display());
                return Ok(Descend::Skipped);
            }
        };
        if st.is_file() {
            if self.is_denied(&display()) {
                return Ok(Descend::Skipped);
            }
            return Ok(Descend::File(WalkedFile {
                real: fs_path.to_string(),
                display: display(),
            }));
        }
        if !st.is_dir() {
            return Ok(Descend::Skipped);
        }

        // 生の realpath を cycle キーに使う。POSIX では大小文字違いは別 dir
        // なので toLowerCase すると誤って同一視して walk 漏れが起きる。
        let canonical = match realpath(fs_path) {
            Ok(canonical) => canonical,
            Err(_) => {
                self.issue(display());
                return Ok(Descend::Skipped);
            }
        };
        if self.ancestors.contains(&canonical) {
            return Ok(Descend::Skipped); // symlink cycle 防止
        }
        self.ancestors.insert(canonical.clone());
        self.observe(fs_path, ClaimMode::Shared, Some(ResourceScope::Subtree))?;
        match fs::read_dir(fs_path) {
            Err(_) => {
                // opendir 失敗(TS の try/finally 相当に ancestors を戻す)
                self.ancestors.remove(&canonical);
                self.issue(display());
                Ok(Descend::Skipped)
            }
            Ok(entries) => {
                self.stack.push(DirFrame {
                    entries,
                    fs_path: fs_path.to_string(),
                    logical_path: logical_path.to_string(),
                    canonical,
                });
                Ok(Descend::Descended)
            }
        }
    }

    fn next_inner(&mut self) -> Result<Option<WalkedFile>, ToolError> {
        loop {
            if let Some((fs_path, logical_path)) = self.pending.take() {
                match self.descend(&fs_path, &logical_path)? {
                    Descend::File(file) => return Ok(Some(file)),
                    Descend::Descended | Descend::Skipped => continue,
                }
            }
            let Some(frame) = self.stack.last_mut() else {
                return Ok(None);
            };
            let entry = frame.entries.next();
            let frame_fs = frame.fs_path.clone();
            let frame_logical = frame.logical_path.clone();
            match entry {
                None => {
                    let frame = self.stack.pop().expect("top frame");
                    self.ancestors.remove(&frame.canonical);
                }
                Some(Err(_)) => {
                    // `dir.read()` 失敗 → 当該 dir の列挙を中断して閉じる
                    let frame = self.stack.pop().expect("top frame");
                    self.ancestors.remove(&frame.canonical);
                    self.issue(to_display(self.opts.root, &frame.logical_path));
                }
                Some(Ok(entry)) => {
                    // Dirent.name は Node が UTF-8 lossy に変換する。
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if SKIP_DIRS.contains(&name.as_str()) {
                        continue;
                    }
                    let child_fs = nodepath::join(&[&frame_fs, &name]);
                    let child_logical = nodepath::join(&[&frame_logical, &name]);
                    // deny path(file/dir 両方)は walk 結果にも潜行にも
                    // onIssue にも出さない。
                    if self.is_denied(&to_display(self.opts.root, &child_logical)) {
                        continue;
                    }
                    let file_type = match entry.file_type() {
                        Ok(ft) => ft,
                        Err(_) => {
                            // Dirent は readdir 時点で型を保持するので TS には
                            // 対応する失敗経路がない。判定不能は列挙失敗として
                            // 通知する。
                            self.issue(to_display(self.opts.root, &child_logical));
                            continue;
                        }
                    };
                    if file_type.is_symlink() {
                        if self.opts.symlink_policy == SymlinkPolicy::Deny {
                            continue;
                        }
                        let target = match realpath(&child_fs) {
                            Ok(target) => target,
                            Err(_) => {
                                // dangling symlink → 列挙失敗として通知
                                self.issue(to_display(self.opts.root, &child_logical));
                                continue;
                            }
                        };
                        if !is_inside_workspace_root(self.opts.root, &target, CASE_INSENSITIVE_FS) {
                            continue;
                        }
                        // realpath 側を辿り、表示は論理 path(symlink 名)を維持
                        self.pending = Some((target, child_logical));
                        continue;
                    }
                    if file_type.is_dir() {
                        self.pending = Some((child_fs, child_logical));
                        continue;
                    }
                    if file_type.is_file() {
                        if self.is_denied(&to_display(self.opts.root, &child_fs)) {
                            continue;
                        }
                        self.observe(&child_fs, ClaimMode::Shared, None)?;
                        return Ok(Some(WalkedFile {
                            real: child_fs,
                            display: to_display(self.opts.root, &child_logical),
                        }));
                    }
                    // fifo/socket 等の特殊 file は列挙しない(Dirent が
                    // file/dir/symlink のいずれでもない entry)。
                }
            }
        }
    }
}

impl Iterator for WalkFiles<'_, '_> {
    /// 1 file の列挙、または観測拒否(`observe` の throw)の error。
    /// `Err` item は 1 度返すと iterator が終了する(generator の throw
    /// parity)。
    type Item = Result<WalkedFile, ToolError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.next_inner() {
            Ok(Some(file)) => Some(Ok(file)),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(e) => {
                self.done = true;
                Some(Err(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cheaper_core::js::code_unit_compare;
    use cheaper_core::workspace::deny_paths::create_path_denylist;

    use super::*;

    /// `mkdtemp` 相当: `temp_dir()/cheaper-walk-<rand>` を作成する。
    fn tmpdir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn root_of(dir: &std::path::Path) -> String {
        realpath(dir.to_str().unwrap()).unwrap()
    }

    fn opts_for<'a>(
        root: &'a str,
        policy: SymlinkPolicy,
        denylist: Option<&'a PathDenylist>,
    ) -> WalkOptions<'a> {
        WalkOptions {
            observe_access: None,
            root,
            symlink_policy: policy,
            denylist,
        }
    }

    /// `collect` helper: display path を集めて JS `Array#sort`
    /// (UTF-16 code unit 順) で並べる。
    fn collect(opts: &WalkOptions<'_>, start: &str) -> Vec<String> {
        collect_issues(opts, start, None)
    }

    fn collect_issues<'i>(
        opts: &WalkOptions<'_>,
        start: &str,
        on_issue: Option<&'i mut WalkIssueHandler<'i>>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for f in walk_files(opts, start, on_issue).expect("walk start") {
            out.push(f.expect("walked file").display);
        }
        out.sort_by(|a, b| code_unit_compare(a, b));
        out
    }

    #[test]
    fn yields_files_recursively_with_posix_display() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        fs::create_dir_all(dir.join("sub").join("deep")).unwrap();
        fs::write(dir.join("top.txt"), "x").unwrap();
        fs::write(dir.join("sub").join("mid.txt"), "x").unwrap();
        fs::write(dir.join("sub").join("deep").join("low.txt"), "x").unwrap();
        let files = collect(&opts_for(&root, SymlinkPolicy::Deny, None), &root);
        assert_eq!(files, ["sub/deep/low.txt", "sub/mid.txt", "top.txt"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn yields_single_file_when_start_is_file() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        fs::write(dir.join("one.txt"), "x").unwrap();
        let files = collect(
            &opts_for(&root, SymlinkPolicy::Deny, None),
            dir.join("one.txt").to_str().unwrap(),
        );
        assert_eq!(files, ["one.txt"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_start_path_outside_workspace() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        let parent = nodepath::join(&[&root, ".."]);
        let err = walk_files(&opts_for(&root, SymlinkPolicy::Deny, None), &parent, None)
            .err()
            .expect("walk must fail");
        // /outside|escape/i と同等: WorkspaceEscapeError がそのまま返る
        assert!(err.to_string().contains("escape"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skips_git_and_node_modules_dirs() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        fs::create_dir_all(dir.join(".git")).unwrap();
        fs::create_dir_all(dir.join("node_modules").join("pkg")).unwrap();
        fs::write(dir.join(".git").join("config"), "x").unwrap();
        fs::write(dir.join("node_modules").join("pkg").join("index.js"), "x").unwrap();
        fs::write(dir.join("src.txt"), "x").unwrap();
        let files = collect(&opts_for(&root, SymlinkPolicy::Deny, None), &root);
        assert_eq!(files, ["src.txt"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn yields_files_regardless_of_size() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        fs::write(dir.join("big.bin"), vec![0x61u8; 1024 * 1024 + 1]).unwrap();
        fs::write(dir.join("small.txt"), "x").unwrap();
        let files = collect(&opts_for(&root, SymlinkPolicy::Deny, None), &root);
        assert_eq!(files, ["big.bin", "small.txt"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn yields_binary_files_path_only() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        let mut bin = vec![0x61u8; 64];
        bin[10] = 0;
        fs::write(dir.join("bin.dat"), bin).unwrap();
        fs::write(dir.join("text.txt"), "no nul here").unwrap();
        let files = collect(&opts_for(&root, SymlinkPolicy::Deny, None), &root);
        assert_eq!(files, ["bin.dat", "text.txt"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    /// symlink 作成(失敗時は test skip と同じく early return)。
    fn try_symlink_dir(orig: &std::path::Path, link: &std::path::Path) -> bool {
        #[cfg(unix)]
        let r = std::os::unix::fs::symlink(orig, link);
        #[cfg(windows)]
        let r = std::os::windows::fs::symlink_dir(orig, link);
        if let Err(e) = r {
            eprintln!("symlink tests skipped: {e}");
            return false;
        }
        true
    }

    fn try_symlink_file(orig: &std::path::Path, link: &std::path::Path) -> bool {
        #[cfg(unix)]
        let r = std::os::unix::fs::symlink(orig, link);
        #[cfg(windows)]
        let r = std::os::windows::fs::symlink_file(orig, link);
        if let Err(e) = r {
            eprintln!("symlink tests skipped: {e}");
            return false;
        }
        true
    }

    #[test]
    fn reports_dangling_symlinks_via_on_issue_under_within_workspace() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        if !try_symlink_dir(&dir.join("no-such-target"), &dir.join("dangling")) {
            fs::remove_dir_all(&dir).unwrap();
            return;
        }
        fs::write(dir.join("f.txt"), "x").unwrap();
        let mut issues = Vec::new();
        let mut handler = |i: WalkIssue| issues.push(i);
        let files = collect_issues(
            &opts_for(&root, SymlinkPolicy::WithinWorkspace, None),
            &root,
            Some(&mut handler),
        );
        assert_eq!(files, ["f.txt"]);
        assert_eq!(
            issues,
            [WalkIssue {
                display: "dangling".to_string(),
                reason: WalkIssueReason::Unreadable
            }]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn never_surfaces_denied_paths() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        // built-in deny pattern(.env)に match する file を置く
        fs::write(dir.join(".env"), "SECRET=1\n").unwrap();
        fs::write(dir.join("ok.txt"), "x").unwrap();
        let denylist = create_path_denylist(&[]);
        let opts = opts_for(&root, SymlinkPolicy::Deny, Some(&denylist));
        let mut issues = Vec::new();
        let mut handler = |i: WalkIssue| issues.push(i);
        let out = collect_issues(&opts, &root, Some(&mut handler));
        assert_eq!(out, ["ok.txt"]);
        assert!(issues.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn does_not_follow_symlinks_under_deny_policy() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        fs::create_dir_all(dir.join("real-dir")).unwrap();
        fs::write(dir.join("real-dir").join("f.txt"), "x").unwrap();
        if !try_symlink_dir(&dir.join("real-dir"), &dir.join("link-dir"))
            || !try_symlink_file(
                &dir.join("real-dir").join("f.txt"),
                &dir.join("link-file.txt"),
            )
        {
            fs::remove_dir_all(&dir).unwrap();
            return;
        }
        let files = collect(&opts_for(&root, SymlinkPolicy::Deny, None), &root);
        assert_eq!(files, ["real-dir/f.txt"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn follows_inside_and_skips_outside_symlinks_under_within_workspace() {
        let dir = tmpdir("cheaper-walk-");
        let outside = tmpdir("cheaper-walk-out-");
        let root = root_of(&dir);
        fs::write(outside.join("out.txt"), "x").unwrap();
        fs::create_dir_all(dir.join("real-dir")).unwrap();
        fs::write(dir.join("real-dir").join("f.txt"), "x").unwrap();
        if !try_symlink_dir(&dir.join("real-dir"), &dir.join("link-in"))
            || !try_symlink_dir(&outside, &dir.join("link-out"))
        {
            fs::remove_dir_all(&dir).unwrap();
            fs::remove_dir_all(&outside).unwrap();
            return;
        }
        let files = collect(
            &opts_for(&root, SymlinkPolicy::WithinWorkspace, None),
            &root,
        );
        assert!(files.contains(&"real-dir/f.txt".to_string()));
        // inside symlink dir は辿られる(display は symlink 経由 path)
        assert!(files.contains(&"link-in/f.txt".to_string()));
        // outside 指しは skip
        assert!(!files.iter().any(|f| f.starts_with("link-out")));
        fs::remove_dir_all(&dir).unwrap();
        fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn does_not_loop_on_symlink_cycles_under_within_workspace() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        if !try_symlink_dir(&dir, &dir.join("self")) {
            fs::remove_dir_all(&dir).unwrap();
            return;
        }
        fs::write(dir.join("f.txt"), "x").unwrap();
        let files = collect(
            &opts_for(&root, SymlinkPolicy::WithinWorkspace, None),
            &root,
        );
        assert_eq!(files, ["f.txt"]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sniff_text_file_text_binary_unreadable() {
        let dir = tmpdir("cheaper-walk-");
        fs::write(dir.join("t.txt"), "hello\n").unwrap();
        assert_eq!(
            sniff_text_file(dir.join("t.txt").to_str().unwrap(), None).unwrap(),
            SniffResult::Text
        );
        let mut bin = vec![0x61u8; 64];
        bin[10] = 0;
        fs::write(dir.join("b.dat"), bin).unwrap();
        assert_eq!(
            sniff_text_file(dir.join("b.dat").to_str().unwrap(), None).unwrap(),
            SniffResult::Binary
        );
        // NUL が先頭 8KiB を超えて初めて現れる場合は text(bounded sniff)
        let mut buf = vec![0x61u8; 10 * 1024];
        buf[9 * 1024] = 0;
        fs::write(dir.join("late-nul.bin"), buf).unwrap();
        assert_eq!(
            sniff_text_file(dir.join("late-nul.bin").to_str().unwrap(), None).unwrap(),
            SniffResult::Text
        );
        assert_eq!(
            sniff_text_file(dir.join("missing.txt").to_str().unwrap(), None).unwrap(),
            SniffResult::Unreadable
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn deny_classification() {
        let dir = tmpdir("cheaper-walk-");
        let root = root_of(&dir);
        // 実際の WorkspaceEscapeError を `rethrow_as_denied` で変換する
        let escape: ToolError =
            Box::new(resolve_workspace_path(&root, "..", SymlinkPolicy::Deny).unwrap_err());
        let err = rethrow_as_denied(escape);
        assert!(is_access_denied_error(&*err));
        assert_eq!(err.to_string(), ACCESS_DENIED_MESSAGE);
        assert!(is_access_denied_error(&DeniedError::policy()));
        assert!(is_access_denied_error(&DeniedError::scope()));
        // deny message 以外の error は classified されない
        let other: ToolError = Box::new(std::io::Error::other("boom"));
        assert!(!is_access_denied_error(&*other));
        // WorkspaceEscapeError でない error はそのまま passthrough される
        let passthrough = rethrow_as_denied(other);
        assert!(!is_access_denied_error(&*passthrough));
        fs::remove_dir_all(&dir).unwrap();
    }
}
