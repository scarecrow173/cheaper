//! Port of `packages/tool-runtime/src/find.ts`.
//!
//! 自前 sandboxed find(DESIGN-0002 / DESIGN-0019)。`matchesGlob` 相当の
//! POSIX glob を workspace 相対 display path に適用し、外部 process は
//! spawn しない。path 列挙のみで内容の open/read/sniff は行わず、列挙
//! 失敗は有界 footer(`skippedFooter`)で報告する。`.git`/
//! `node_modules` skip と denylist 排除・存在秘匿は walker の責務。

use std::sync::Arc;

use cheaper_core::tools::vocabulary::ToolName;
use serde_json::Value;

use crate::catalog::{self, CanonicalToolResult, ToolContent};
use crate::definition::{define_canonical_tool, CanonicalToolDefinition};
use crate::glob::matches_glob;
use crate::operation::AbortSignal;
use crate::walker::{rethrow_as_denied, walk_files, WalkOptions};
use crate::ToolError;
use cheaper_core::js16::slice_utf16;

const DEFAULT_MAX_RESULTS: u64 = 200;
const MAX_MAX_RESULTS: u64 = 1000;
const SKIP_FOOTER_MAX_ENTRIES: usize = 20;
const SKIP_PATH_MAX_CHARS: usize = 200;
const SKIP_FOOTER_MAX_BYTES: usize = 4 * 1024;
const ABORTED_MESSAGE: &str = "Operation aborted";

/// `SkipBucket`: 代表 path は 20 件まで保持し、残りは total で
/// `and N more` に集計する(列挙失敗が大量にあっても蓄積は有界)。
#[derive(Default)]
struct SkipBucket {
    shown: Vec<String>,
    total: usize,
}

fn record_issue(bucket: &mut SkipBucket, display: &str) {
    bucket.total += 1;
    if bucket.shown.len() < SKIP_FOOTER_MAX_ENTRIES {
        // TS `display.length > SKIP_PATH_MAX_CHARS` + `slice(0, N)` は
        // UTF-16 code unit 単位(astral char は 2 unit)。
        bucket
            .shown
            .push(slice_utf16(display, SKIP_PATH_MAX_CHARS).to_string());
    }
}

/// `skippedFooter(reason, bucket)` — `reason` は `"unreadable"` 固定。
fn skipped_footer(reason: &str, bucket: &SkipBucket) -> String {
    let more = bucket.total - bucket.shown.len();
    let list = if more > 0 {
        format!(
            "{}and {more} more",
            if bucket.shown.is_empty() {
                String::new()
            } else {
                format!("{}, ", bucket.shown.join(", "))
            },
        )
    } else {
        bucket.shown.join(", ")
    };
    let line = format!("[skipped: {reason} — {list}]");
    if line.len() > SKIP_FOOTER_MAX_BYTES {
        "[skipped-report: truncated]".to_string()
    } else {
        line
    }
}

fn aborted_err() -> ToolError {
    Box::new(std::io::Error::other(ABORTED_MESSAGE))
}

/// `createFindTool` の返り値。`WalkOptions` を借用する。
pub struct FindTool<'o, 'a> {
    opts: &'o WalkOptions<'a>,
}

/// `createFindTool`。
pub fn create_find_tool<'o, 'a>(opts: &'o WalkOptions<'a>) -> FindTool<'o, 'a> {
    FindTool { opts }
}

impl FindTool<'_, '_> {
    /// `definition.execute` 相当。
    pub fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let start_abs = input["path"].as_str().unwrap_or(".");
        let pattern = input["pattern"].as_str().unwrap_or_default();
        let max_results = std::cmp::min(
            input["maxResults"].as_u64().unwrap_or(DEFAULT_MAX_RESULTS),
            MAX_MAX_RESULTS,
        );
        let mut out: Vec<String> = Vec::new();
        let mut issues = SkipBucket::default();
        let check_abort = || -> Result<(), ToolError> {
            if signal.is_some_and(AbortSignal::is_aborted) {
                return Err(aborted_err());
            }
            Ok(())
        };
        let mut truncated = false;
        {
            let mut record = |issue: crate::walker::WalkIssue| {
                record_issue(&mut issues, &issue.display);
            };
            let result = (|| -> Result<(), ToolError> {
                check_abort()?;
                let files = walk_files(self.opts, start_abs, Some(&mut record))
                    .map_err(|e| rethrow_as_denied(Box::new(e) as ToolError))?;
                for file in files {
                    let file = file.map_err(rethrow_as_denied)?;
                    check_abort()?;
                    // `matchesGlob` は file 毎に lazy 評価される — file が
                    // 1 件でもあれば over-long pattern の TypeError がここで
                    // propagate する(file 0 件では throw しない)。
                    if !matches_glob(&file.display, pattern)? {
                        continue;
                    }
                    if out.len() as u64 >= max_results {
                        truncated = true;
                        break;
                    }
                    out.push(file.display);
                }
                Ok(())
            })();
            result?;
        }
        if truncated {
            out.push("[truncated: more results]".to_string());
        }
        let mut body = if out.is_empty() {
            "(no matches)".to_string()
        } else {
            out.join("\n")
        };
        if issues.total > 0 {
            body.push('\n');
            body.push_str(&skipped_footer("unreadable", &issues));
        }
        Ok(CanonicalToolResult {
            content: vec![ToolContent::text(body)],
            details: None,
        })
    }
}

/// `find` tool の canonical definition。
pub fn find_tool_definition<'a>(
    opts: &'a WalkOptions<'a>,
) -> Result<CanonicalToolDefinition<'a>, ToolError> {
    let tool = Arc::new(create_find_tool(opts));
    Ok(define_canonical_tool(CanonicalToolDefinition {
        name: ToolName::Find,
        label: "find".to_string(),
        description: "List files whose workspace-relative path matches a glob (e.g. **/*.ts). Lists all allowed file paths regardless of size or content. Skips .git and node_modules.".to_string(),
        parameters: catalog::find_schema()
            .as_object()
            .expect("object schema")
            .clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input, signal.as_ref()) })
        }),
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use cheaper_core::workspace::deny_paths::create_path_denylist;
    use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
    use serde_json::json;

    use super::{record_issue, SkipBucket, SKIP_PATH_MAX_CHARS};
    use crate::glob::matches_glob;
    use crate::walker::WalkOptions;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-find-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn walk_opts<'a>(
        root: &'a str,
        denylist: Option<&'a cheaper_core::workspace::deny_paths::PathDenylist>,
    ) -> WalkOptions<'a> {
        WalkOptions {
            observe_access: None,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            denylist,
        }
    }

    fn text(result: &super::CanonicalToolResult) -> &str {
        &result.content[0].text
    }

    #[test]
    fn fs_find_glob_and_skips() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        fs::create_dir_all(dir.join(".git/objects")).unwrap();
        fs::write(dir.join("src/main.ts"), "").unwrap();
        fs::write(dir.join("src/lib.rs"), "").unwrap();
        fs::write(dir.join("node_modules/pkg/index.ts"), "").unwrap();
        fs::write(dir.join(".git/objects/aa"), "").unwrap();
        let opts = walk_opts(&root, None);
        let tool = super::create_find_tool(&opts);
        let result = tool
            .execute(&json!({"path": ".", "pattern": "**/*.ts"}), None)
            .unwrap();
        // node_modules / .git は列挙 skip。
        assert_eq!(text(&result), "src/main.ts");
        let result = tool
            .execute(&json!({"path": ".", "pattern": "**/zzz.*"}), None)
            .unwrap();
        assert_eq!(text(&result), "(no matches)");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_find_max_results_truncated_marker() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        for i in 0..5 {
            fs::write(dir.join(format!("f{i}.txt")), "").unwrap();
        }
        let opts = walk_opts(&root, None);
        let tool = super::create_find_tool(&opts);
        let result = tool
            .execute(
                &json!({"path": ".", "pattern": "*.txt", "maxResults": 2}),
                None,
            )
            .unwrap();
        let t = text(&result);
        assert!(t.ends_with("[truncated: more results]"), "{t}");
        assert_eq!(t.lines().count(), 3);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn record_issue_truncates_at_utf16_units() {
        // TS `display.length > SKIP_PATH_MAX_CHARS` + `slice(0, N)` は
        // UTF-16 code unit 単位 — astral char は 2 unit を占める。
        // (半端 surrogate は表現不能のため手前で切る backoff は許容差異。)
        let display = format!("{}x", "😀".repeat(100)); // 100*2+1 = 201 units
        let mut bucket = SkipBucket::default();
        record_issue(&mut bucket, &display);
        assert_eq!(
            bucket.shown[0].chars().count(),
            100,
            "200 unit で 100 emoji だけ残る('x' は入らない)"
        );
        let mut ascii = SkipBucket::default();
        record_issue(&mut ascii, &"a".repeat(300));
        assert_eq!(ascii.shown[0].len(), SKIP_PATH_MAX_CHARS);
    }

    #[test]
    fn fs_find_denied_hidden_and_escape() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join(".env"), "SECRET=1").unwrap();
        fs::write(dir.join("a.txt"), "").unwrap();
        let denylist = create_path_denylist(&[]);
        let opts = walk_opts(&root, Some(&denylist));
        let tool = super::create_find_tool(&opts);
        let result = tool
            .execute(&json!({"path": ".", "pattern": "**"}), None)
            .unwrap();
        assert!(!text(&result).contains(".env"), "{}", text(&result));
        let err = tool
            .execute(&json!({"path": "../", "pattern": "*"}), None)
            .unwrap_err();
        assert_eq!(err.to_string(), crate::walker::ACCESS_DENIED_MESSAGE);
        fs::remove_dir_all(&dir).unwrap();
    }

    /// `path.posix.matchesGlob` の代表 fixture(Node の minimatch 意味論)。
    /// POSIX(nocase なし)基準で検証する — Windows では nocase なので
    /// case-sensitive fixture は `#[cfg(unix)]` 側で確かめる。
    #[test]
    fn glob_basics() {
        assert!(matches_glob("src/main.ts", "**/*.ts").unwrap());
        assert!(matches_glob("a.ts", "*.ts").unwrap());
        assert!(matches_glob("a/b/c.rs", "**/c.rs").unwrap());
        assert!(matches_glob("a/b", "a/**/b").unwrap());
        assert!(matches_glob("a/x/y/b", "a/**/b").unwrap());
        assert!(!matches_glob("a.ts/b", "*.ts").unwrap());
        assert!(matches_glob("foo1", "foo?").unwrap());
        assert!(!matches_glob("foo/x", "foo?").unwrap());
        assert!(matches_glob("a.js", "*.{ts,js}").unwrap());
        assert!(!matches_glob("a.rs", "*.{ts,js}").unwrap());
        assert!(matches_glob("b", "[abc]").unwrap());
        assert!(matches_glob("d", "[!abc]").unwrap());
        assert!(matches_glob("b", "[a-c]").unwrap());
        assert!(!matches_glob("d", "[a-c]").unwrap());
        // pattern 側の `\` は `/` に正規化される(path 側は正規化されない)。
        assert!(matches_glob("a/b", "a\\*b").unwrap());
        assert!(!matches_glob("a*b", "a\\*b").unwrap());
        assert!(!matches_glob("axb", "a\\*b").unwrap());
        // `.` 開始 segment は `*`/`?`/`[]`/`**` に match しない。
        assert!(!matches_glob(".env", "*").unwrap());
        assert!(!matches_glob("sub/.env", "*/*").unwrap());
        assert!(!matches_glob(".h/x", "**/x").unwrap());
        assert!(matches_glob(".env", ".*").unwrap());
        assert!(matches_glob("sub/.env", "*/.*").unwrap());
    }

    /// pattern > 65536 UTF-16 unit は file 毎の `matchesGlob` 評価で
    /// `TypeError("pattern is too long")` 相当として propagate する。
    /// 対象 file が 0 件なら throw しない(Node の lazy 評価と同じ)。
    #[test]
    fn overlong_pattern_errors_lazily() {
        let pattern = "a".repeat(65537);
        // file なし → matcher 未呼出 → error にならない。
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let opts = walk_opts(&root, None);
        let tool = super::create_find_tool(&opts);
        let result = tool
            .execute(&json!({"path": ".", "pattern": pattern}), None)
            .unwrap();
        assert_eq!(text(&result), "(no matches)");
        fs::remove_dir_all(&dir).unwrap();
        // file あり → matcher が throw 相当の error を返す。
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("a.txt"), "").unwrap();
        let opts = walk_opts(&root, None);
        let tool = super::create_find_tool(&opts);
        let err = tool
            .execute(&json!({"path": ".", "pattern": pattern}), None)
            .unwrap_err();
        assert_eq!(err.to_string(), "pattern is too long");
        fs::remove_dir_all(&dir).unwrap();
    }
}
