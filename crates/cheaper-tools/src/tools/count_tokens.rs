//! Port of `packages/tool-runtime/src/count-tokens.ts`.
//!
//! canonical Token Counter の tool 入口(DESIGN-0012)。core
//! `count_file_tokens` と同一実装を使い、CLI `count-tokens` と同じ
//! count を返す。file 本文は返さない。

use std::sync::Arc;

use cheaper_core::token::count::{count_file_tokens, CountFileTokensOptions, CountTokensError};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{resolve_workspace_path, SymlinkPolicy};
use serde_json::{json, Value};

use crate::access::AccessObserver;
use crate::catalog::{self, CanonicalToolResult, ToolContent};
use crate::definition::{define_canonical_tool, CanonicalToolDefinition};
use crate::walker::{rethrow_as_denied, DeniedError};
use crate::ToolError;

/// `CountTokensToolOptions`。
pub struct CountTokensToolOptions<'a> {
    pub observe_access: Option<&'a AccessObserver<'a>>,
    /// canonical 化済み workspace root。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    pub denylist: Option<&'a PathDenylist>,
}

/// `createCountTokensTool` の返り値。
pub struct CountTokensTool<'a> {
    opts: CountTokensToolOptions<'a>,
}

/// `createCountTokensTool`。
pub fn create_count_tokens_tool(opts: CountTokensToolOptions<'_>) -> CountTokensTool<'_> {
    CountTokensTool { opts }
}

impl CountTokensTool<'_> {
    /// `definition.execute` 相当。
    pub fn execute(&self, input: &Value) -> Result<CanonicalToolResult, ToolError> {
        let path = input["path"].as_str().unwrap_or_default();
        let opts = &self.opts;
        // try/catch → `rethrowAsDenied` を末尾で適用する形に揃える。
        let result = (|| -> Result<CanonicalToolResult, ToolError> {
            if let Some(denylist) = opts.denylist {
                let resolved = resolve_workspace_path(opts.root, path, opts.symlink_policy)
                    .map_err(|e| Box::new(e) as ToolError)?;
                if denylist.is_denied(&resolved.display) {
                    return Err(Box::new(DeniedError::policy()));
                }
            }
            if let Some(observe) = opts.observe_access {
                observe(
                    path,
                    cheaper_core::resources::types::ClaimMode::Shared,
                    None,
                    false,
                )?;
            }
            let result = count_file_tokens(&CountFileTokensOptions {
                root: opts.root,
                path,
                symlink_policy: opts.symlink_policy,
                max_bytes: None,
            })
            .map_err(|e| -> ToolError {
                // `Escape` 内包の `WorkspaceEscapeError` を剥がして
                // `rethrow_as_denied` が拾える形にする(TS では
                // countFileTokens が escape error を直接投げる)。
                match e {
                    CountTokensError::Escape(esc) => Box::new(esc),
                    other => Box::new(other),
                }
            })?;
            let details = json!({
                "path": result.path,
                "tokens": result.tokens,
                "bytes": result.bytes,
                "tokenizer": result.tokenizer,
            });
            Ok(CanonicalToolResult {
                content: vec![ToolContent::text(
                    serde_json::to_string(&details).expect("count result serializes"),
                )],
                details: Some(details),
            })
        })();
        result.map_err(rethrow_as_denied)
    }
}

/// `count_tokens` tool の canonical definition。
pub fn count_tokens_tool_definition(
    opts: CountTokensToolOptions<'_>,
) -> Result<CanonicalToolDefinition<'_>, ToolError> {
    let tool = Arc::new(create_count_tokens_tool(opts));
    Ok(define_canonical_tool(CanonicalToolDefinition {
        name: ToolName::CountTokens,
        label: "count_tokens".to_string(),
        description: "Count the canonical o200k_base tokens of a workspace file. Returns path, tokens, bytes, and tokenizer identity. Does not return file contents.".to_string(),
        parameters: catalog::count_tokens_schema()
            .as_object()
            .expect("object schema")
            .clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, _signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input) })
        }),
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use cheaper_core::resources::types::ClaimMode;
    use cheaper_core::workspace::deny_paths::create_path_denylist;
    use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
    use serde_json::json;

    use super::*;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-count-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn opts<'a>(root: &'a str, denylist: Option<&'a PathDenylist>) -> CountTokensToolOptions<'a> {
        CountTokensToolOptions {
            observe_access: None,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            denylist,
        }
    }

    #[test]
    fn fs_count_tokens_returns_details_without_content() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "hello world hello world").unwrap();
        let tool = create_count_tokens_tool(opts(&root, None));
        let result = tool.execute(&json!({"path": "f.txt"})).unwrap();
        let d = result.details.as_ref().unwrap();
        assert_eq!(d["path"], "f.txt");
        assert_eq!(d["bytes"], 23);
        assert_eq!(d["tokenizer"]["name"], "o200k_base");
        assert!(d["tokens"].as_u64().unwrap() > 0);
        // file 本文は content に含まれない。
        assert!(!result.content[0].text.contains("hello world"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_count_tokens_denied_and_escape() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join(".env"), "SECRET=1").unwrap();
        let denylist = create_path_denylist(&[]);
        let tool = create_count_tokens_tool(opts(&root, Some(&denylist)));
        let err = tool.execute(&json!({"path": ".env"})).unwrap_err();
        assert_eq!(err.to_string(), crate::walker::POLICY_DENIED_MESSAGE);
        let tool = create_count_tokens_tool(opts(&root, None));
        let err = tool.execute(&json!({"path": "../x.txt"})).unwrap_err();
        assert_eq!(err.to_string(), crate::walker::ACCESS_DENIED_MESSAGE);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_count_tokens_observes_shared_access() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "x").unwrap();
        let calls = Mutex::new(Vec::<(String, ClaimMode)>::new());
        let observer: &AccessObserver = &|p, mode, _scope, _support| {
            calls.lock().unwrap().push((p.to_string(), mode));
            Ok(())
        };
        let mut o = opts(&root, None);
        o.observe_access = Some(observer);
        let tool = create_count_tokens_tool(o);
        tool.execute(&json!({"path": "f.txt"})).unwrap();
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "f.txt");
        assert_eq!(calls[0].1, ClaimMode::Shared);
        fs::remove_dir_all(&dir).unwrap();
    }
}
