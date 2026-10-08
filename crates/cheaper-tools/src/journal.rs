//! Port of `packages/tool-runtime/src/journal.ts`.
//!
//! mutation journal(DESIGN-0002 / ADR-0006)。write/edit operations が初回
//! 書き込み前に `captureBefore` を呼び、実行終了時に `finalize` が
//! before と disk 上の最終内容を比較して `meta.changes` を生成する。
//! 差分なしのエントリは除外する。
//!
//! FS アクセスは core と同じく `std::fs` 同期実装。entries は tool handler
//! が走る間に共有されるため `Mutex` 内包(`&self` メソッドで同期取る)。
//!
//! 行差分は npm `diff`(`diffLines`, diff@9)ではなく `similar` で再現する:
//! - jsdiff の line tokenizer は `value.split(/(\n|\r\n)/)` + 末尾 empty
//!   token drop + separator merge。bare `\r` は separator ではない。
//!   `similar` の `tokenize_lines`(`from_lines` 内部)は bare `\r` でも
//!   分割するため `from_lines` は使わず、jsdiff 互換 token 列を自前で
//!   作って `algorithms::diff_slices` に渡す。
//! - algorithm は `Algorithm::RawMyers` — deadline/heuristic 無しで最短
//!   edit script を計算する。jsdiff の Myers(maxEditLength/timeout 未指定
//!   = 最短探索)と同じ minimal diff を返すため added/removed の token
//!   count 和(`linesAdded`/`linesDeleted`)は一致する。heuristic 版の
//!   `Algorithm::Myers` は大入力で最短性を捨てるため採用しない。
//!   (差分 probe: `#[cfg(test)]` の `journal_diff_counts_match_jsdiff`
//!   が diff@9.0.0 の出力 golden と一致を pin している)

use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;

use cheaper_core::js::locale_compare;
use serde::Serialize;
use similar::algorithms::{diff_slices, DiffHook};
use similar::Algorithm;

/// `FileChange["kind"]`: `"created" | "modified"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileChangeKind {
    /// before が無かった(新規作成)。
    Created,
    /// before が存在した(上書き/変更)。
    Modified,
}

impl FileChangeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Modified => "modified",
        }
    }
}

/// `meta.changes.files` の 1 エントリ(DESIGN-0005 envelope)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    /// workspace 相対 POSIX 区切り path。
    pub path: String,
    pub kind: FileChangeKind,
    pub lines_added: usize,
    pub lines_deleted: usize,
}

/// `ChangesSummary["totals"]`。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeTotals {
    pub files: usize,
    pub lines_added: usize,
    pub lines_deleted: usize,
}

/// `meta.changes` の shape。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ChangesSummary {
    pub files: Vec<FileChange>,
    pub totals: ChangeTotals,
}

struct JournalEntry {
    display: String,
    /// 初回書き込み前の内容。存在しなかったファイルは `None`。
    before: Option<Vec<u8>>,
}

/// `MutationJournal`(module doc 参照)。
#[derive(Default)]
pub struct MutationJournal {
    entries: Mutex<HashMap<String, JournalEntry>>,
}

impl MutationJournal {
    pub fn new() -> Self {
        Self::default()
    }

    /// real path をキーに初回書き込み前の内容を記録する。
    /// 同一 path の 2 回目以降の呼び出しは before を保持する(冪等)。
    /// 不存在は `None`(created)。その他の読み取り失敗も `None` 扱いとし、
    /// 後続の書き込み自体が失敗するか finalize の disk 比較に委ねる。
    pub fn capture_before(&self, real: &str, display: &str) {
        let mut entries = self.entries.lock().expect("journal entries poisoned");
        if entries.contains_key(real) {
            return;
        }
        let before = fs::read(real).ok();
        entries.insert(
            real.to_string(),
            JournalEntry {
                display: display.to_string(),
                before,
            },
        );
    }

    /// journal に記録があるか(abort 分類の PARTIAL_MUTATION 判定に使用)。
    pub fn is_empty(&self) -> bool {
        self.entries
            .lock()
            .expect("journal entries poisoned")
            .is_empty()
    }

    /// before と disk 上の最終内容を比較し、差分のあるファイルのみを
    /// `ChangesSummary` として返す。行差分は jsdiff `diffLines` 互換の
    /// tokenizer + `similar` RawMyers で算出する。
    pub fn finalize(&self) -> ChangesSummary {
        let entries = self.entries.lock().expect("journal entries poisoned");
        let mut files: Vec<FileChange> = Vec::new();
        for (real, entry) in entries.iter() {
            let after = fs::read(real).ok();
            // `Buffer.toString("utf8")` は WHATWG UTF-8 置換(U+FFFD)と
            // 同じ — `String::from_utf8_lossy` に対応する。
            let before_str = entry
                .before
                .as_deref()
                .map(|b| String::from_utf8_lossy(b).into_owned());
            let after_str = after
                .as_deref()
                .map(|b| String::from_utf8_lossy(b).into_owned());
            if before_str == after_str {
                continue; // 差分なし → 除外
            }
            let (lines_added, lines_deleted) = diff_line_counts(
                before_str.as_deref().unwrap_or(""),
                after_str.as_deref().unwrap_or(""),
            );
            files.push(FileChange {
                path: entry.display.clone(),
                kind: if entry.before.is_none() {
                    FileChangeKind::Created
                } else {
                    FileChangeKind::Modified
                },
                lines_added,
                lines_deleted,
            });
        }
        // `files.sort((a, b) => a.path.localeCompare(b.path))`。
        files.sort_by(|a, b| locale_compare(&a.path, &b.path));
        let mut totals = ChangeTotals::default();
        for f in &files {
            totals.files += 1;
            totals.lines_added += f.lines_added;
            totals.lines_deleted += f.lines_deleted;
        }
        ChangesSummary { files, totals }
    }
}

/// jsdiff `diffLines` の tokenizer(`value.split(/(\n|\r\n)/)` + 末尾 empty
/// drop + separator merge)。各 token は行内容 + terminator。`""` は 0 token。
/// bare `\r` は separator にしない(jsdiff parity)。
/// `tools/edit` の diff 生成(`generateDiffString`/`generateUnifiedPatch`)
/// でも同じ tokenize contract を使うため `pub(crate)`。
pub(crate) fn tokenize_lines(value: &str) -> Vec<&str> {
    let bytes = value.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&value[start..i + 1]);
                i += 1;
                start = i;
            }
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                lines.push(&value[start..i + 2]);
                i += 2;
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < bytes.len() {
        lines.push(&value[start..]);
    }
    lines
}

/// `diffSlices` の added/removed token count を積む hook。
/// `replace` は default 実装(delete + insert)を使う。
struct LineCounts {
    added: usize,
    deleted: usize,
}

impl DiffHook for LineCounts {
    type Error = std::convert::Infallible;

    fn delete(
        &mut self,
        _old_index: usize,
        old_len: usize,
        _new_index: usize,
    ) -> Result<(), Self::Error> {
        self.deleted += old_len;
        Ok(())
    }

    fn insert(
        &mut self,
        _old_index: usize,
        _new_index: usize,
        new_len: usize,
    ) -> Result<(), Self::Error> {
        self.added += new_len;
        Ok(())
    }
}

/// `(linesAdded, linesDeleted)`。jsdiff `diffLines` の
/// `part.added/removed` + `count` 集計と同等。
fn diff_line_counts(before: &str, after: &str) -> (usize, usize) {
    let old = tokenize_lines(before);
    let new = tokenize_lines(after);
    let mut counts = LineCounts {
        added: 0,
        deleted: 0,
    };
    match diff_slices(Algorithm::RawMyers, &mut counts, &old, &new) {
        Ok(()) => {}
        Err(e) => match e {}, // `Infallible` — hook は失敗しない
    }
    (counts.added, counts.deleted)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    /// `mkdtemp` 相当: `temp_dir()/cheaper-journal-<rand>` を作成する。
    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-journal-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn real_of(dir: &Path, name: &str) -> String {
        dir.join(name).to_str().unwrap().to_string()
    }

    /// jsdiff `diffLines` tokenizer の分岐 pin。bare `\r` は分割しない
    /// (`similar` の `tokenize_lines` との差異 — `from_lines` を使わない
    /// 理由の regression guard)。
    #[test]
    fn journal_tokenize_lines_jsdiff_compatible() {
        assert_eq!(tokenize_lines(""), Vec::<&str>::new());
        assert_eq!(tokenize_lines("a\n"), vec!["a\n"]);
        assert_eq!(tokenize_lines("a"), vec!["a"]);
        assert_eq!(tokenize_lines("a\r\nb"), vec!["a\r\n", "b"]);
        // bare `\r` は separator ではない(jsdiff `split(/(\n|\r\n)/)` 互換)。
        assert_eq!(tokenize_lines("a\rb\n"), vec!["a\rb\n"]);
        assert_eq!(tokenize_lines("a\n\nb\n"), vec!["a\n", "\n", "b\n"]);
    }

    /// 差分 probe: npm `diff`@9.0.0 の `diffLines` 実出力を golden として
    /// pin する。値は `node -e` + `require('diff')` (packages/tool-runtime
    /// と同じ `^9.0.0`)で生成した `(added, deleted)` count。
    /// `similar` + RawMyers + jsdiff 互換 tokenizer が全ケースで一致する
    /// ことを検証する(一致 → `similar` 採用)。
    #[test]
    fn journal_diff_counts_match_jsdiff() {
        // (name, before, after, added, deleted) — diff@9.0.0 の実測値。
        let cases: &[(&str, &str, &str, usize, usize)] = &[
            (
                "basic change",
                "line1\nline2\nline3\n",
                "line1\nmodified\nline3\n",
                1,
                1,
            ),
            ("create from empty", "", "created line\n", 1, 0),
            ("delete to empty", "a\n", "", 0, 1),
            ("both empty", "", "", 0, 0),
            ("add newline at end", "a", "a\n", 1, 1),
            ("remove newline at end", "a\n", "a", 1, 1),
            ("reorder", "a\nb\n", "b\na\n", 1, 1),
            ("reversed three", "a\nb\nc\n", "c\nb\na\n", 2, 2),
            ("duplicate lines", "a\na\nb\n", "a\nb\na\n", 1, 1),
            ("duplicate lines swap", "x\nx\n", "x\nx\nx\n", 1, 0),
            ("crlf vs lf", "a\r\nb\r\n", "a\nb\n", 2, 2),
            ("bare cr", "a\rb\n", "a\nb\n", 2, 1),
            ("bare cr insert", "a\nb\n", "a\rb\n", 1, 2),
            ("identical crlf", "a\r\nb\r\n", "a\r\nb\r\n", 0, 0),
            ("append line", "a\n", "a\nb\n", 1, 0),
            ("prepend line", "b\n", "a\nb\n", 1, 0),
            (
                "duplicate anchors",
                "keep\nend\nkeep\n",
                "keep\nmid\nend\nkeep\n",
                1,
                0,
            ),
            ("unicode", "日本語\nok\n", "日本語\nng\n", 1, 1),
            ("blank lines", "a\n\nb\n", "a\n\n\nb\n", 1, 0),
            ("insert at eof no newline", "a\nb", "a\nb\nc\n", 2, 1),
            ("insert before eof newline", "a\nb\n", "a\nb\nc\n", 1, 0),
            (
                "duplicate identical",
                "x\ny\nx\ny\n",
                "x\ny\ny\nx\ny\n",
                1,
                0,
            ),
            ("long middle insert", "a\nb\nc\n", "a\nx\nc\n", 1, 1),
        ];
        for (name, before, after, added, deleted) in cases {
            assert_eq!(
                diff_line_counts(before, after),
                (*added, *deleted),
                "case `{name}`: before={before:?} after={after:?}"
            );
        }
    }

    /// `isEmpty` は capture 0 件で true、capture 後 false。
    #[test]
    fn journal_is_empty_reflects_captures() {
        let dir = tmpdir();
        let journal = MutationJournal::new();
        assert!(journal.is_empty());
        journal.capture_before(&real_of(&dir, "a.txt"), "a.txt");
        assert!(!journal.is_empty());
    }

    /// `captureBefore` は初回の内容を保持する(冪等)。2 回目以降の
    /// capture は before を上書きしない。
    #[test]
    fn journal_capture_before_is_idempotent() {
        let dir = tmpdir();
        let real = real_of(&dir, "a.txt");
        let journal = MutationJournal::new();
        // 存在しない file → before=null(created)。
        journal.capture_before(&real, "a.txt");
        fs::write(&real, "line1\n").unwrap();
        // 書き込み後の再 capture は no-op — before=null のまま。
        journal.capture_before(&real, "a.txt");
        fs::write(&real, "line1\nline2\n").unwrap();
        let summary = journal.finalize();
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].kind, FileChangeKind::Created);
        assert_eq!(summary.files[0].lines_added, 2);
        assert_eq!(summary.files[0].lines_deleted, 0);
    }

    /// existing file の変更は `modified` + added/deleted count。
    #[test]
    fn journal_finalize_modified_counts_lines() {
        let dir = tmpdir();
        let real = real_of(&dir, "a.txt");
        fs::write(&real, "a\nb\nc\n").unwrap();
        let journal = MutationJournal::new();
        journal.capture_before(&real, "a.txt");
        fs::write(&real, "a\nx\nc\n").unwrap();
        let summary = journal.finalize();
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].path, "a.txt");
        assert_eq!(summary.files[0].kind, FileChangeKind::Modified);
        assert_eq!(summary.files[0].lines_added, 1);
        assert_eq!(summary.files[0].lines_deleted, 1);
        assert_eq!(
            summary.totals,
            ChangeTotals {
                files: 1,
                lines_added: 1,
                lines_deleted: 1
            }
        );
    }

    /// 差分なしのエントリは `meta.changes` から除外する。
    #[test]
    fn journal_finalize_excludes_unchanged() {
        let dir = tmpdir();
        let real = real_of(&dir, "a.txt");
        fs::write(&real, "same\n").unwrap();
        let journal = MutationJournal::new();
        journal.capture_before(&real, "a.txt");
        fs::write(&real, "same\n").unwrap();
        let summary = journal.finalize();
        assert!(summary.files.is_empty());
        assert_eq!(summary.totals, ChangeTotals::default());
    }

    /// after が読めない(削除済み)場合は `""` との差分 — 全行 deleted。
    #[test]
    fn journal_finalize_deleted_file_counts_all_lines_removed() {
        let dir = tmpdir();
        let real = real_of(&dir, "a.txt");
        fs::write(&real, "a\nb\n").unwrap();
        let journal = MutationJournal::new();
        journal.capture_before(&real, "a.txt");
        fs::remove_file(&real).unwrap();
        let summary = journal.finalize();
        assert_eq!(summary.files.len(), 1);
        // before が存在したので kind は modified(TS も `before === null`
        // のみを created にする)。
        assert_eq!(summary.files[0].kind, FileChangeKind::Modified);
        assert_eq!(summary.files[0].lines_added, 0);
        assert_eq!(summary.files[0].lines_deleted, 2);
    }

    /// files は `path.localeCompare` 順にソートされ、totals は全 entry の和。
    #[test]
    fn journal_finalize_sorts_paths_and_sums_totals() {
        let dir = tmpdir();
        let journal = MutationJournal::new();
        // 意図的に辞書逆順で capture する。
        for (name, content) in [("z.txt", "z\n"), ("b.txt", "b\n"), ("a.txt", "a\n")] {
            let real = real_of(&dir, name);
            fs::write(&real, content).unwrap();
            journal.capture_before(&real, name);
            fs::write(&real, format!("{content}extra\n")).unwrap();
        }
        let summary = journal.finalize();
        let paths: Vec<&str> = summary.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["a.txt", "b.txt", "z.txt"]);
        assert_eq!(
            summary.totals,
            ChangeTotals {
                files: 3,
                lines_added: 3,
                lines_deleted: 0
            }
        );
    }

    /// before の読み取り失敗(不存在)は `null` と同じく created 扱い。
    /// CRLF → LF の書き換えは全行差分。
    #[test]
    fn journal_finalize_crlf_rewrite() {
        let dir = tmpdir();
        let real = real_of(&dir, "a.txt");
        fs::write(&real, "a\r\nb\r\n").unwrap();
        let journal = MutationJournal::new();
        journal.capture_before(&real, "a.txt");
        fs::write(&real, "a\nb\n").unwrap();
        let summary = journal.finalize();
        assert_eq!(summary.files[0].lines_added, 2);
        assert_eq!(summary.files[0].lines_deleted, 2);
    }
}
