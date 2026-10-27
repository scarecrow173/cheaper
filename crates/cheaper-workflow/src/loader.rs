//! Port of `packages/workflow/src/loader.ts`。明示 file の I/O 境界
//! だけを所有し、定義検証は canonical compiler に委譲する。
//! workflow file は JSON のみ(YAML は parse しない)。

use std::future::Future;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{resolve_workspace_path, SymlinkPolicy};
use cheaper_tools::ToolError;
use serde_json::{Map, Value};

use crate::compiler::WorkflowCompiler;
use crate::types::WorkflowDefinition;

/// `WorkflowFileOptions`。
pub struct WorkflowFileOptions {
    /// canonical 化済み workspace root。
    pub root: String,
    pub path: String,
    pub symlink_policy: SymlinkPolicy,
    pub denylist: PathDenylist,
}

/// `LoadedWorkflow`。
pub struct LoadedWorkflow<'a> {
    pub definition: WorkflowDefinition,
    pub compiler: WorkflowCompiler<'a>,
}

/// `checkDenied(display, denylist)` — `display` を `/` で prefix 分割し
/// 各 prefix を照合する。
fn check_denied(display: &str, denylist: &PathDenylist) -> Result<(), CheaperError> {
    let segments: Vec<&str> = display.split('/').collect();
    for i in 1..=segments.len() {
        if denylist.is_denied(&segments[..i].join("/")) {
            return Err(CheaperError::new(
                ErrorCode::InvalidRequest,
                "workflow path is denied by policy",
            ));
        }
    }
    Ok(())
}

/// `withCause({cause})` 用 — box 済み `ToolError` を
/// `impl std::error::Error`(Sized 要求)へ渡すための器。`source()` は
/// 内側へ delegate する。
struct BoxedCause(ToolError);

impl std::fmt::Debug for BoxedCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl std::fmt::Display for BoxedCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for BoxedCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// `loadWorkflowFile`。`createCompiler` の失敗はそのまま伝播する。
pub async fn load_workflow_file<'a, F, Fut>(
    options: &WorkflowFileOptions,
    create_compiler: F,
) -> Result<LoadedWorkflow<'a>, ToolError>
where
    F: FnOnce(&Value) -> Fut,
    Fut: Future<Output = Result<WorkflowCompiler<'a>, ToolError>>,
{
    let source: String;
    {
        // I/O 境界内で発生する非 CheaperError(`WorkspaceEscapeError`・
        // fs error)は `cannot read workflow file` に wrap する。
        let read = async || -> Result<String, ToolError> {
            let resolved =
                resolve_workspace_path(&options.root, &options.path, options.symlink_policy)
                    .map_err(|e| -> ToolError { Box::new(e) })?;
            let logical = nodepath::relative(
                &options.root,
                &nodepath::resolve(&options.root, &options.path),
            );
            check_denied(
                &logical.split(nodepath::SEP).collect::<Vec<_>>().join("/"),
                &options.denylist,
            )
            .map_err(|e| -> ToolError { Box::new(e) })?;
            check_denied(&resolved.display, &options.denylist)
                .map_err(|e| -> ToolError { Box::new(e) })?;
            let metadata = tokio::fs::metadata(&resolved.real)
                .await
                .map_err(|e| -> ToolError { Box::new(e) })?;
            if !metadata.is_file() {
                return Err(Box::new(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    "workflow path must be a regular file",
                )) as ToolError);
            }
            // `readFile(path, "utf8")` — invalid UTF-8 は lossy decode。
            let bytes = tokio::fs::read(&resolved.real)
                .await
                .map_err(|e| -> ToolError { Box::new(e) })?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        };
        match read().await {
            Ok(text) => source = text,
            Err(cause) => {
                // `cause instanceof CheaperError` は direct check。
                if cause.downcast_ref::<CheaperError>().is_some() {
                    return Err(cause);
                }
                return Err(Box::new(
                    CheaperError::new(ErrorCode::InvalidRequest, "cannot read workflow file")
                        .with_details({
                            let mut details = Map::new();
                            details.insert("path".to_string(), Value::String(options.path.clone()));
                            details
                        })
                        .with_cause(BoxedCause(cause)),
                ));
            }
        }
    }
    load_workflow_document(&source, create_compiler).await
}

/// `loadWorkflowFile` の document-input 版。workflow source の取得は
/// caller が担い、JSON parse・compiler 作成・definition validation は
/// file loader と同一経路を共有する。route check など embedded/logical
/// source からの read-only validation に使う。
pub async fn load_workflow_document<'a, F, Fut>(
    source: &str,
    create_compiler: F,
) -> Result<LoadedWorkflow<'a>, ToolError>
where
    F: FnOnce(&Value) -> Fut,
    Fut: Future<Output = Result<WorkflowCompiler<'a>, ToolError>>,
{
    let raw: Value = serde_json::from_str(source).map_err(|_| {
        // JSON.parse の message は private 本文を含み得るため公開しない。
        Box::new(CheaperError::new(
            ErrorCode::InvalidInput,
            "workflow file must contain valid JSON",
        )) as ToolError
    })?;
    let compiler = create_compiler(&raw).await?;
    let definition = compiler.validate_definition(&raw)?;
    Ok(LoadedWorkflow {
        definition,
        compiler,
    })
}
