use rig::tool::{DynamicTool, ToolErrorKind, ToolExecutionError, ToolOutput};

fn tool_err(message: String) -> ToolExecutionError {
    ToolExecutionError::new(ToolErrorKind::Other, message)
}

/// Minimal sandboxed read: absolute or cwd-relative path only.
pub fn read_tool() -> DynamicTool {
    DynamicTool::new(
        "read",
        "Read a text file and return its contents.",
        serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"],
            "additionalProperties": false
        }),
        |args| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tool_err("missing `path`".into()))?;
                let text = std::fs::read_to_string(path)
                    .map_err(|e| tool_err(format!("read {path}: {e}")))?;
                Ok(ToolOutput::text(text))
            })
        },
    )
}
