//! Port of Pi `createWriteToolDefinition`(`dist/core/tools/write.js`)
//! on canonical boundary operations(`operations.ts`)。
//!
//! `resolveToCwd(path, cwd)` は canonical 側では `nodepath::resolve` +
//! `resolve_workspace_path` が同等の意味を持つ(cwd = workspace root、
//! absolute input はそのまま boundary 判定される)。Pi の
//! `normalizePath` 拡張(`~`・`@` prefix・unicode space 正規化)は
//! canonical contract には含めない(read.ts 等と同じ判断)。
//! `withFileMutationQueue` は同一 file の mutation を直列化する。

use std::sync::Arc;

use cheaper_core::nodepath;
use cheaper_core::tools::vocabulary::ToolName;
use serde_json::Value;

use crate::catalog::{self, CanonicalToolResult};
use crate::definition::{define_canonical_tool, CanonicalToolDefinition};
use crate::operation::AbortSignal;
use crate::tools::boundary::{mutation_queue_key, with_file_mutation_queue, WriteOperations};
use crate::ToolError;

/// `createWriteTool` の返り値。
pub struct WriteTool<'a> {
    ops: WriteOperations<'a>,
}

/// `createWriteTool`。
pub fn create_write_tool(ops: WriteOperations<'_>) -> WriteTool<'_> {
    WriteTool { ops }
}

/// `signal.aborted` → `throw new Error("Operation aborted")`。
fn throw_if_aborted(signal: Option<&AbortSignal>) -> Result<(), ToolError> {
    if signal.is_some_and(AbortSignal::is_aborted) {
        return Err(Box::new(std::io::Error::other("Operation aborted")));
    }
    Ok(())
}

impl WriteTool<'_> {
    /// `definition.execute` 相当。`path`/`content` は schema 検証済み。
    pub fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let path = input["path"].as_str().unwrap_or_default();
        let content = input["content"].as_str().unwrap_or_default();
        let absolute_path = nodepath::resolve(self.ops.root(), path);
        let dir = nodepath::dirname(&absolute_path);
        let queue_key = mutation_queue_key(self.ops.root(), &absolute_path);
        with_file_mutation_queue(&queue_key, || {
            // abort event listener ではなく各 await 後の `signal.aborted`
            // 観測に揃える(in-flight FS op が settle するまで queue lock を
            // 保持するため — Pi 本体のコメントと同じ意図)。
            throw_if_aborted(signal)?;
            // Create parent directories if needed.
            self.ops.mkdir(&dir)?;
            throw_if_aborted(signal)?;
            self.ops.write_file(&absolute_path, content)?;
            throw_if_aborted(signal)?;
            Ok(CanonicalToolResult::text(format!(
                "Successfully wrote to {path}"
            )))
        })
    }
}

/// `write` tool の canonical definition(label/description は Pi 由来)。
pub fn write_tool_definition(
    ops: WriteOperations<'_>,
) -> Result<CanonicalToolDefinition<'_>, ToolError> {
    let tool = Arc::new(create_write_tool(ops));
    Ok(define_canonical_tool(CanonicalToolDefinition {
        name: ToolName::Write,
        label: "write".to_string(),
        description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories.".to_string(),
        parameters: catalog::write_schema()
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
    use crate::tools::boundary::{create_write_operations, BoundaryOperationsOptions};

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-write-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn boundary_opts<'a>(
        root: &'a str,
        denylist: Option<&'a cheaper_core::workspace::deny_paths::PathDenylist>,
    ) -> BoundaryOperationsOptions<'a> {
        BoundaryOperationsOptions {
            observe_access: None,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            journal: None,
            denylist,
            scope: ResourceScopeOptions::default(),
        }
    }

    #[test]
    fn write_creates_parent_dirs_and_reports_input_path() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let ops = create_write_operations(boundary_opts(&root, None)).unwrap();
        let tool = create_write_tool(ops);
        let result = tool
            .execute(&json!({"path": "a/b/c.txt", "content": "hello"}), None)
            .unwrap();
        assert_eq!(result.content[0].text, "Successfully wrote to a/b/c.txt");
        assert_eq!(
            fs::read_to_string(dir.join("a").join("b").join("c.txt")).unwrap(),
            "hello"
        );
        // tmp file は残らない(atomic rename + cleanup)。
        let tmp_left = fs::read_dir(dir.join("a").join("b")).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .contains(".cheaper-tmp-")
        });
        assert!(!tmp_left);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_rejects_workspace_escape_with_denied_message() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let ops = create_write_operations(boundary_opts(&root, None)).unwrap();
        let tool = create_write_tool(ops);
        let err = tool
            .execute(&json!({"path": "../outside.txt", "content": "x"}), None)
            .unwrap_err();
        assert_eq!(err.to_string(), crate::walker::ACCESS_DENIED_MESSAGE);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_rejects_denylisted_path() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let denylist = create_path_denylist(&[]);
        let ops = create_write_operations(boundary_opts(&root, Some(&denylist))).unwrap();
        let tool = create_write_tool(ops);
        let err = tool
            .execute(&json!({"path": ".env", "content": "SECRET=1"}), None)
            .unwrap_err();
        assert_eq!(err.to_string(), crate::walker::POLICY_DENIED_MESSAGE);
        assert!(!dir.join(".env").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_scope_exclusive_observation_via_injected_observer() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let calls = std::sync::Mutex::new(Vec::<(String, String)>::new());
        let observer: &crate::access::AccessObserver = &|p, mode, _scope, _support| {
            calls
                .lock()
                .unwrap()
                .push((p.to_string(), mode.as_str().to_string()));
            Ok(())
        };
        let mut opts = boundary_opts(&root, None);
        opts.observe_access = Some(observer);
        let ops = create_write_operations(opts).unwrap();
        let tool = create_write_tool(ops);
        tool.execute(&json!({"path": "out.txt", "content": "x"}), None)
            .unwrap();
        let calls = calls.lock().unwrap();
        assert!(
            calls.iter().any(|(_, m)| m == "exclusive"),
            "exclusive observation expected: {calls:?}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_propagates_observer_scope_denial() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let observer: &crate::access::AccessObserver = &|_p, mode, _scope, _support| {
            if mode == cheaper_core::resources::types::ClaimMode::Exclusive {
                Err(Box::new(crate::walker::DeniedError::scope()))
            } else {
                Ok(())
            }
        };
        let mut opts = boundary_opts(&root, None);
        opts.observe_access = Some(observer);
        let ops = create_write_operations(opts).unwrap();
        let tool = create_write_tool(ops);
        let err = tool
            .execute(&json!({"path": "out.txt", "content": "x"}), None)
            .unwrap_err();
        assert_eq!(err.to_string(), crate::walker::SCOPE_DENIED_MESSAGE);
        assert!(!dir.join("out.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
