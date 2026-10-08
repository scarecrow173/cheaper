use rig::tool::{DynamicTool, ToolErrorKind, ToolExecutionError, ToolOutput};

fn tool_err(message: String) -> ToolExecutionError {
    ToolExecutionError::new(ToolErrorKind::Other, message)
}

/// Minimal grep: substring match over a file, line numbers included.
pub fn grep_tool() -> DynamicTool {
    DynamicTool::new(
        "grep",
        "Search a text file for lines containing a substring.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string" }
            },
            "required": ["pattern", "path"],
            "additionalProperties": false
        }),
        |args| {
            Box::pin(async move {
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tool_err("missing `pattern`".into()))?;
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| tool_err("missing `path`".into()))?;
                let text = std::fs::read_to_string(path)
                    .map_err(|e| tool_err(format!("read {path}: {e}")))?;
                let hits: Vec<String> = text
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.contains(pattern))
                    .map(|(n, line)| format!("{}: {}", n + 1, line))
                    .collect();
                Ok(ToolOutput::text(hits.join("\n")))
            })
        },
    )
}
