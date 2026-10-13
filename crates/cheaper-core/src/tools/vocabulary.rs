//! Port of `packages/core/src/tools/vocabulary.ts`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};

/// `TOOL_NAMES`: the 9 canonical tool names, in declaration order.
pub const TOOL_NAMES: [&str; 9] = [
    "read",
    "grep",
    "find",
    "ls",
    "write",
    "edit",
    "count_tokens",
    "process",
    "command",
];

/// Canonical tool name. Serialized as the lowercase string form
/// (`count_tokens` → `"count_tokens"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolName {
    Read,
    Grep,
    Find,
    Ls,
    Write,
    Edit,
    CountTokens,
    Process,
    Command,
}

impl ToolName {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Grep => "grep",
            Self::Find => "find",
            Self::Ls => "ls",
            Self::Write => "write",
            Self::Edit => "edit",
            Self::CountTokens => "count_tokens",
            Self::Process => "process",
            Self::Command => "command",
        }
    }

    /// Exact canonical-name match (the `CANONICAL` set lookup in TS).
    pub fn from_canonical(name: &str) -> Option<Self> {
        Some(match name {
            "read" => Self::Read,
            "grep" => Self::Grep,
            "find" => Self::Find,
            "ls" => Self::Ls,
            "write" => Self::Write,
            "edit" => Self::Edit,
            "count_tokens" => Self::CountTokens,
            "process" => Self::Process,
            "command" => Self::Command,
            _ => return None,
        })
    }

    /// Single-sourced name resolver — canonical name first, then the
    /// `tool_alias` alias table. `tools` (`normalize_tool_names`) and
    /// `cheaper.optionalTools` share this so their resolution rules
    /// cannot drift.
    pub fn resolve(name: &str) -> Option<Self> {
        Self::from_canonical(name).or_else(|| tool_alias(name))
    }
}

/// `TOOL_ALIASES`: exact-match alias → canonical name (declaration order kept
/// so `Object.keys(TOOL_ALIASES)` order checks port directly).
pub static TOOL_ALIASES: &[(&str, ToolName)] = &[
    ("Read", ToolName::Read),
    ("Grep", ToolName::Grep),
    ("Glob", ToolName::Find),
    ("Write", ToolName::Write),
    ("Edit", ToolName::Edit),
];

fn tool_alias(name: &str) -> Option<ToolName> {
    TOOL_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, tool)| *tool)
}

/// scope 構築時に content-gate(nonempty tasks/grants)を持つ tool の
/// 一権威集合。`cheaper.optionalTools` の validate 検査が使う —
/// gated 集合を二重定義しないための唯一の predicate。
pub fn is_scope_gated_tool(name: ToolName) -> bool {
    matches!(name, ToolName::Process | ToolName::Command)
}

fn with_reason(message: &str, reason: &str) -> CheaperError {
    let mut details = Map::new();
    details.insert("reason".to_string(), Value::from(reason));
    CheaperError::new(ErrorCode::InvalidWorkerDefinition, message).with_details(details)
}

/// Port of `normalizeToolNames`.
pub fn normalize_tool_names(raw: &Value) -> Result<Vec<ToolName>, CheaperError> {
    let valid_array = raw
        .as_array()
        .filter(|arr| !arr.is_empty() && arr.iter().all(Value::is_string));
    let arr = match valid_array {
        Some(arr) => arr,
        None => {
            return Err(with_reason(
                "tools must be a non-empty string array",
                "tools-non-empty-string-array",
            ))
        }
    };
    let mut names = Vec::with_capacity(arr.len());
    for v in arr {
        let name = v.as_str().expect("checked is_string");
        let tool = ToolName::resolve(name);
        match tool {
            Some(tool) => names.push(tool),
            None => {
                let mut details = Map::new();
                details.insert("tool".to_string(), Value::from(name));
                details.insert("known".to_string(), json!(TOOL_NAMES));
                return Err(CheaperError::new(
                    ErrorCode::UnknownTool,
                    format!("unknown tool: {name}"),
                )
                .with_details(details));
            }
        }
    }
    let mut seen = HashSet::with_capacity(names.len());
    if !names.iter().all(|n| seen.insert(*n)) {
        return Err(with_reason("tools contain duplicates", "tools-duplicate"));
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn as_strings(names: &[ToolName]) -> Vec<&str> {
        names.iter().map(|n| n.as_str()).collect()
    }

    #[test]
    fn passes_canonical_names_through() {
        let names = normalize_tool_names(&json!(["read", "grep"])).unwrap();
        assert_eq!(as_strings(&names), ["read", "grep"]);
    }

    #[test]
    fn normalizes_allowed_aliases_to_canonical_names() {
        let names =
            normalize_tool_names(&json!(["Read", "Grep", "Glob", "Write", "Edit"])).unwrap();
        assert_eq!(
            as_strings(&names),
            ["read", "grep", "find", "write", "edit"]
        );
    }

    #[test]
    fn exposes_exactly_the_allowed_aliases() {
        let mut keys: Vec<_> = TOOL_ALIASES.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["Edit", "Glob", "Grep", "Read", "Write"]);
    }

    #[test]
    fn rejects_non_array_values() {
        // TS test also covers `undefined`; serde_json has no undefined value.
        for raw in [Value::Null, json!("read"), json!(5), json!({})] {
            let err = normalize_tool_names(&raw).unwrap_err();
            assert_eq!(err.code(), ErrorCode::InvalidWorkerDefinition, "{raw}");
        }
    }

    #[test]
    fn rejects_empty_array() {
        let err = normalize_tool_names(&json!([])).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidWorkerDefinition);
    }

    #[test]
    fn rejects_non_string_entries() {
        let err = normalize_tool_names(&json!(["read", 3])).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidWorkerDefinition);
    }

    #[test]
    fn rejects_unknown_tool_with_details() {
        let err = normalize_tool_names(&json!(["bash"])).unwrap_err();
        assert_eq!(err.code(), ErrorCode::UnknownTool);
        let details = err.details().unwrap();
        assert_eq!(details["tool"], json!("bash"));
        assert_eq!(details["known"], json!(TOOL_NAMES));
    }

    #[test]
    fn rejects_case_variants_not_in_the_alias_table() {
        let err = normalize_tool_names(&json!(["READ"])).unwrap_err();
        assert_eq!(err.code(), ErrorCode::UnknownTool);
    }

    #[test]
    fn rejects_duplicates_after_normalization() {
        let err = normalize_tool_names(&json!(["read", "Read"])).unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidWorkerDefinition);
        assert_eq!(err.details().unwrap()["reason"], json!("tools-duplicate"));
    }

    #[test]
    fn resolve_accepts_canonical_names_then_aliases() {
        assert_eq!(ToolName::resolve("read"), Some(ToolName::Read));
        assert_eq!(
            ToolName::resolve("count_tokens"),
            Some(ToolName::CountTokens)
        );
        assert_eq!(ToolName::resolve("command"), Some(ToolName::Command));
        assert_eq!(ToolName::resolve("Read"), Some(ToolName::Read));
        assert_eq!(ToolName::resolve("Glob"), Some(ToolName::Find));
        assert_eq!(ToolName::resolve("bash"), None);
        assert_eq!(ToolName::resolve("READ"), None);
    }

    #[test]
    fn is_scope_gated_tool_marks_only_process_and_command() {
        assert!(is_scope_gated_tool(ToolName::Process));
        assert!(is_scope_gated_tool(ToolName::Command));
        for t in [
            ToolName::Read,
            ToolName::Grep,
            ToolName::Find,
            ToolName::Ls,
            ToolName::Write,
            ToolName::Edit,
            ToolName::CountTokens,
        ] {
            assert!(!is_scope_gated_tool(t), "{t:?}");
        }
    }

    #[test]
    fn tool_name_serializes_to_canonical_string() {
        assert_eq!(
            serde_json::to_value(ToolName::CountTokens).unwrap(),
            json!("count_tokens")
        );
        let back: ToolName = serde_json::from_value(json!("process")).unwrap();
        assert_eq!(back, ToolName::Process);
    }
}
