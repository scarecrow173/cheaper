//! Port of `packages/core/src/worker/write-scope.ts`.
//!
//! `extractWriteScope` only owns literal-path extraction; canonical
//! identity is `ResourceAccess`'s job (Phase 2).

use std::collections::HashSet;

use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::worker::validate::WorkerDefinition;

/// `extractWriteScope`: `input[field]` must be `string[]` or `{path}[]`.
/// `Ok(None)` = worker has no `writeScope`.
pub fn extract_write_scope(
    worker: &WorkerDefinition,
    input: &Value,
) -> Result<Option<HashSet<String>>, CheaperError> {
    let Some(field) = &worker.cheaper.write_scope else {
        return Ok(None);
    };
    let value = input.as_object().and_then(|o| o.get(field));
    let invalid = |reason: &str| {
        let mut details = Map::new();
        details.insert("field".to_string(), json!(field));
        details.insert("reason".to_string(), json!(reason));
        CheaperError::new(
            ErrorCode::InvalidInput,
            format!("writeScope field invalid: {reason}"),
        )
        .with_details(details)
    };
    let Some(Value::Array(items)) = value else {
        return Err(invalid("missing-or-not-array"));
    };
    let mut allowed = HashSet::with_capacity(items.len());
    for item in items {
        let target = match item {
            Value::String(s) => Some(s.as_str()),
            Value::Object(o) => o.get("path").and_then(Value::as_str),
            _ => None,
        };
        match target {
            Some(t) => {
                allowed.insert(t.to_string());
            }
            None => return Err(invalid("item-lacks-path")),
        }
    }
    Ok(Some(allowed))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::worker::parse::parse_worker_file;
    use crate::worker::validate::{validate_worker_definition, WorkerOrigin};

    fn worker(write_scope: Option<&str>) -> WorkerDefinition {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("w.md");
        let ws = write_scope
            .map(|f| format!("  writeScope: {f}\n"))
            .unwrap_or_default();
        let content = format!(
            "---\nname: w\ndescription: d\ntools: [read]\ncheaper:\n  model: {{tier: low}}\n  output: {{schema: {{inline: {{type: string}}}}}}\n{ws}---\nbody\n"
        );
        let parsed = parse_worker_file(&content, p.to_str().unwrap()).unwrap();
        validate_worker_definition(&parsed, p.to_str().unwrap(), WorkerOrigin::Project).unwrap()
    }

    #[test]
    fn none_without_write_scope() {
        let w = worker(None);
        assert_eq!(extract_write_scope(&w, &json!({})).unwrap(), None);
    }

    #[test]
    fn collects_string_and_path_object_items() {
        let w = worker(Some("targets"));
        let scope = extract_write_scope(
            &w,
            &json!({"targets": ["a.txt", {"path": "b.txt"}, {"path": "a.txt"}]}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(scope.len(), 2);
        assert!(scope.contains("a.txt") && scope.contains("b.txt"));
    }

    #[test]
    fn rejects_missing_or_non_array() {
        let w = worker(Some("targets"));
        let e = extract_write_scope(&w, &json!({})).unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("missing-or-not-array")
        );
        let e = extract_write_scope(&w, &json!({"targets": "x"})).unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("missing-or-not-array")
        );
        // Non-object input also fails as not-an-array (TS same).
        let e = extract_write_scope(&w, &json!("x")).unwrap_err();
        assert_eq!(
            e.details().unwrap()["reason"],
            json!("missing-or-not-array")
        );
    }

    #[test]
    fn rejects_items_without_path() {
        let w = worker(Some("targets"));
        for item in [json!(3), json!({"p": "x"}), json!(null)] {
            let e = extract_write_scope(&w, &json!({"targets": [item]})).unwrap_err();
            assert_eq!(e.details().unwrap()["reason"], json!("item-lacks-path"));
        }
    }
}
