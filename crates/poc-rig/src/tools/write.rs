use rig::tool::{DynamicTool, ToolErrorKind, ToolExecutionError, ToolOutput};

fn tool_err(message: String) -> ToolExecutionError {
    ToolExecutionError::new(ToolErrorKind::Other, message)
}

/// Minimal write: truncates/creates a file.
pub fn write_tool() -> DynamicTool {
    DynamicTool::new(
        "write",
        "Write text to a file, creating or truncating it.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"],
            "additionalProperties": false
        }),
        |args| {
            Box::pin(async move {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tool_err("missing `path`".into()))?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tool_err("missing `content`".into()))?;
                std::fs::write(path, content)
                    .map_err(|e| tool_err(format!("write {path}: {e}")))?;
                Ok(ToolOutput::text(format!("wrote {} bytes", content.len())))
            })
        },
    )
}
