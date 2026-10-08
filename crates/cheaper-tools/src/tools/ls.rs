//! Port of Pi `createLsToolDefinition`(`dist/core/tools/ls.js`) +
//! `truncate.js` の `truncateHead`/`formatSize` on canonical
//! `LsOperations`(`operations.ts`)。
//!
//! entry は dotfile を含み、`toLowerCase().localeCompare` で case-
//! insensitive sort される。directory entry には `/` を付ける。
//! entry 数上限(既定 500)と byte 上限(既定 50KB、部分行なし)は
//! どちらが先でも打ち切り、actionable notice を末尾に付ける。

use std::sync::Arc;

use cheaper_core::js::locale_compare;
use cheaper_core::nodepath;
use cheaper_core::tools::vocabulary::ToolName;
use serde_json::{json, Value};

use crate::catalog::{self, CanonicalToolResult, ToolContent};
use crate::definition::{define_canonical_tool, CanonicalToolDefinition};
use crate::operation::AbortSignal;
use crate::tools::boundary::LsOperations;
use crate::ToolError;

/// `DEFAULT_LIMIT`。
const DEFAULT_LIMIT: f64 = 500.0;
/// `DEFAULT_MAX_BYTES`(truncate.js)= 50KB。
const DEFAULT_MAX_BYTES: usize = 50 * 1024;
/// `Number.MAX_SAFE_INTEGER`。
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
const ABORTED_MESSAGE: &str = "Operation aborted";

/// `createLsTool` の返り値。
pub struct LsTool<'a> {
    ops: LsOperations<'a>,
}

/// `createLsTool`。
pub fn create_ls_tool(ops: LsOperations<'_>) -> LsTool<'_> {
    LsTool { ops }
}

fn aborted_err() -> ToolError {
    Box::new(std::io::Error::other(ABORTED_MESSAGE))
}

/// `formatSize(bytes)`。
fn format_size(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{}B", js_number(bytes))
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1}KB", bytes / 1024.0)
    } else {
        format!("{:.1}MB", bytes / (1024.0 * 1024.0))
    }
}

/// JS template 埋め込みの number→string(整数は `.0` なし)。
fn js_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 9.0e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// `truncateHead(content, {maxLines, maxBytes})`。`lastLinePartial` は
/// head では常に false。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TruncationResult {
    pub content: String,
    pub truncated: bool,
    pub truncated_by: Option<&'static str>,
    pub total_lines: usize,
    pub total_bytes: usize,
    pub output_lines: usize,
    pub output_bytes: usize,
    pub last_line_partial: bool,
    pub first_line_exceeds_limit: bool,
    /// JS number — `maxLines` は `MAX_SAFE_INTEGER` まで取るため
    /// serde_json integer に cast する。
    #[serde(serialize_with = "ser_max_lines")]
    pub max_lines: f64,
    pub max_bytes: usize,
}

fn ser_max_lines<S: serde::Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(*v as u64)
}

/// `splitLinesForCounting`(`""` → `[]`、末尾 `\n` は空要素を作らない)。
fn split_lines_for_counting(content: &str) -> Vec<&str> {
    if content.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = content.split('\n').collect();
    if content.ends_with('\n') {
        lines.pop();
    }
    lines
}

/// `truncateHead`。`max_lines` は JS number(小数可 — `i < maxLines`
/// 比較を f64 で行う)。
fn truncate_head(content: &str, max_lines: f64, max_bytes: usize) -> TruncationResult {
    let total_bytes = content.len();
    let lines = split_lines_for_counting(content);
    let total_lines = lines.len();
    if (total_lines as f64) <= max_lines && total_bytes <= max_bytes {
        return TruncationResult {
            content: content.to_string(),
            truncated: false,
            truncated_by: None,
            total_lines,
            total_bytes,
            output_lines: total_lines,
            output_bytes: total_bytes,
            last_line_partial: false,
            first_line_exceeds_limit: false,
            max_lines,
            max_bytes,
        };
    }
    // 先頭行単体で byte 上限超過。
    let first_line_bytes = lines.first().map_or(0, |l| l.len());
    if first_line_bytes > max_bytes {
        return TruncationResult {
            content: String::new(),
            truncated: true,
            truncated_by: Some("bytes"),
            total_lines,
            total_bytes,
            output_lines: 0,
            output_bytes: 0,
            last_line_partial: false,
            first_line_exceeds_limit: true,
            max_lines,
            max_bytes,
        };
    }
    let mut output_lines: Vec<&str> = Vec::new();
    let mut output_bytes_count = 0usize;
    let mut truncated_by = "lines";
    for (i, line) in lines.iter().enumerate() {
        if (i as f64) >= max_lines {
            break;
        }
        let line_bytes = line.len() + usize::from(i > 0); // +1 for newline
        if output_bytes_count + line_bytes > max_bytes {
            truncated_by = "bytes";
            break;
        }
        output_lines.push(line);
        output_bytes_count += line_bytes;
    }
    if (output_lines.len() as f64) >= max_lines && output_bytes_count <= max_bytes {
        truncated_by = "lines";
    }
    let output_content = output_lines.join("\n");
    TruncationResult {
        output_bytes: output_content.len(),
        output_lines: output_lines.len(),
        content: output_content,
        truncated: true,
        truncated_by: Some(truncated_by),
        total_lines,
        total_bytes,
        last_line_partial: false,
        first_line_exceeds_limit: false,
        max_lines,
        max_bytes,
    }
}

impl LsTool<'_> {
    /// `definition.execute` 相当。
    pub fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        if signal.is_some_and(AbortSignal::is_aborted) {
            return Err(aborted_err());
        }
        let path = input["path"].as_str();
        let limit = input["limit"].as_f64();
        let dir_path = nodepath::resolve(self.ops.root(), path.unwrap_or("."));
        let effective_limit = limit.unwrap_or(DEFAULT_LIMIT);
        // Check if path exists.
        if !self.ops.exists(&dir_path)? {
            return Err(Box::new(std::io::Error::other(format!(
                "Path not found: {dir_path}"
            ))));
        }
        if signal.is_some_and(AbortSignal::is_aborted) {
            return Err(aborted_err());
        }
        // Check if path is a directory.
        let stat = self.ops.stat(&dir_path)?;
        if !stat.is_directory() {
            return Err(Box::new(std::io::Error::other(format!(
                "Not a directory: {dir_path}"
            ))));
        }
        if signal.is_some_and(AbortSignal::is_aborted) {
            return Err(aborted_err());
        }
        // Read directory entries.
        let mut entries = match self.ops.readdir(&dir_path) {
            Ok(e) => e,
            Err(e) => {
                return Err(Box::new(std::io::Error::other(format!(
                    "Cannot read directory: {e}"
                ))));
            }
        };
        // Sort alphabetically, case-insensitive.
        entries.sort_by(|a, b| locale_compare(&a.to_lowercase(), &b.to_lowercase()));
        // Format entries with directory indicators.
        let mut results: Vec<String> = Vec::new();
        let mut entry_limit_reached = false;
        for entry in &entries {
            if (results.len() as f64) >= effective_limit {
                entry_limit_reached = true;
                break;
            }
            if signal.is_some_and(AbortSignal::is_aborted) {
                return Err(aborted_err());
            }
            let full_path = nodepath::join(&[&dir_path, entry]);
            let suffix = match self.ops.stat(&full_path) {
                Ok(st) if st.is_directory() => "/",
                Ok(_) => "",
                // Skip entries we cannot stat.
                Err(_) => continue,
            };
            results.push(format!("{entry}{suffix}"));
        }
        if results.is_empty() {
            return Ok(CanonicalToolResult::text("(empty directory)"));
        }
        let raw_output = results.join("\n");
        // entry count が既に cap されるため行数制限はない。
        let truncation = truncate_head(&raw_output, MAX_SAFE_INTEGER, DEFAULT_MAX_BYTES);
        let mut output = truncation.content.clone();
        let mut notices: Vec<String> = Vec::new();
        let mut details = serde_json::Map::new();
        if entry_limit_reached {
            notices.push(format!(
                "{} entries limit reached. Use limit={} for more",
                js_number(effective_limit),
                js_number(effective_limit * 2.0),
            ));
            let v = if effective_limit.fract() == 0.0
                && effective_limit.is_finite()
                && effective_limit.abs() < 9.0e15
            {
                json!(effective_limit as i64)
            } else {
                json!(effective_limit)
            };
            details.insert("entryLimitReached".to_string(), v);
        }
        if truncation.truncated {
            notices.push(format!(
                "{} limit reached",
                format_size(DEFAULT_MAX_BYTES as f64)
            ));
            details.insert(
                "truncation".to_string(),
                serde_json::to_value(&truncation).expect("truncation serializes"),
            );
        }
        if !notices.is_empty() {
            output.push_str(&format!("\n\n[{}]", notices.join(". ")));
        }
        Ok(CanonicalToolResult {
            content: vec![ToolContent::text(output)],
            details: if details.is_empty() {
                None
            } else {
                Some(Value::Object(details))
            },
        })
    }
}

/// `ls` tool の canonical definition。description は Pi 由来の
/// 書式(`DEFAULT_LIMIT`/`DEFAULT_MAX_BYTES/1024` を interpolate)。
pub fn ls_tool_definition(ops: LsOperations<'_>) -> Result<CanonicalToolDefinition<'_>, ToolError> {
    let tool = Arc::new(create_ls_tool(ops));
    Ok(define_canonical_tool(CanonicalToolDefinition {
        name: ToolName::Ls,
        label: "ls".to_string(),
        description: format!(
            "List directory contents. Returns entries sorted alphabetically, with '/' suffix for directories. Includes dotfiles. Output is truncated to {} entries or {}KB (whichever is hit first).",
            js_number(DEFAULT_LIMIT),
            DEFAULT_MAX_BYTES / 1024,
        ),
        parameters: catalog::ls_schema()
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

    use super::*;
    use crate::access::ResourceScopeOptions;
    use crate::tools::boundary::{create_ls_operations, BoundaryOperationsOptions};

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-ls-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn make_tool<'a>(
        root: &'a str,
        denylist: Option<&'a cheaper_core::workspace::deny_paths::PathDenylist>,
    ) -> LsTool<'a> {
        let ops = create_ls_operations(BoundaryOperationsOptions {
            observe_access: None,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            journal: None,
            denylist,
            scope: ResourceScopeOptions::default(),
        })
        .unwrap();
        create_ls_tool(ops)
    }

    #[test]
    fn fs_ls_sorted_dotfiles_and_dir_suffix() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("Zebra.txt"), "").unwrap();
        fs::write(dir.join("apple.txt"), "").unwrap();
        fs::write(dir.join(".hidden"), "").unwrap();
        fs::create_dir(dir.join("Subdir")).unwrap();
        let tool = make_tool(&root, None);
        let result = tool.execute(&json!({"path": "."}), None).unwrap();
        assert_eq!(
            result.content[0].text,
            ".hidden\napple.txt\nSubdir/\nZebra.txt"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_ls_empty_and_missing() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::create_dir(dir.join("empty")).unwrap();
        let tool = make_tool(&root, None);
        let result = tool.execute(&json!({"path": "empty"}), None).unwrap();
        assert_eq!(result.content[0].text, "(empty directory)");
        let err = tool.execute(&json!({"path": "nope"}), None).unwrap_err();
        assert!(err.to_string().starts_with("Path not found:"));
        let err = tool.execute(&json!({"path": "../x"}), None).unwrap_err();
        assert_eq!(err.to_string(), crate::walker::ACCESS_DENIED_MESSAGE);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_ls_entry_limit_notice() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        for i in 0..5 {
            fs::write(dir.join(format!("f{i}.txt")), "").unwrap();
        }
        let tool = make_tool(&root, None);
        let result = tool
            .execute(&json!({"path": ".", "limit": 2}), None)
            .unwrap();
        assert_eq!(
            result.content[0].text,
            "f0.txt\nf1.txt\n\n[2 entries limit reached. Use limit=4 for more]"
        );
        assert_eq!(result.details.unwrap()["entryLimitReached"], 2);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_ls_denied_entry_hidden() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("a.txt"), "").unwrap();
        fs::write(dir.join(".env"), "SECRET=1").unwrap();
        let denylist = create_path_denylist(&[]);
        let tool = make_tool(&root, Some(&denylist));
        let result = tool.execute(&json!({"path": "."}), None).unwrap();
        assert_eq!(result.content[0].text, "a.txt");
        // denied dir 自体を list すると policy denied。
        let err = tool.execute(&json!({"path": ".env"}), None).unwrap_err();
        assert_eq!(err.to_string(), crate::walker::POLICY_DENIED_MESSAGE);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn format_size_bounds() {
        assert_eq!(format_size(500.0), "500B");
        assert_eq!(format_size(50.0 * 1024.0), "50.0KB");
        assert_eq!(format_size(2.0 * 1024.0 * 1024.0), "2.0MB");
    }
}
