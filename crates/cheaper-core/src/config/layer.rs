//! `readPolicyFile`/`readModelsConfig` の共通骨格 — read → lossy utf8 →
//! yaml parse → object check。error の reason 文字列(`read-failed`/
//! `yaml-parse`/`not-object`)・`errno` detail・trace event 名は各
//! caller の契約として `LayerKind` で保持する。

use serde_json::{json, Map, Value};

use crate::errors::{io_errno_display, is_absent_file_error, CheaperError, ErrorCode};

/// layer file の種別 — error code・message prefix・trace event 名。
pub(crate) struct LayerKind {
    pub code: ErrorCode,
    /// `"policy file"` / `"models config"` — `not found`/`read failed`
    /// message の prefix。
    pub file_label: &'static str,
    /// `"policy yaml"` / `"models yaml"` — parse error message の prefix。
    pub yaml_label: &'static str,
    /// `"policy"` / `"models config"` — `is not an object` message の
    /// prefix。
    pub object_label: &'static str,
    /// `cheaper-core` event 名(例: `"policy file read"`)。
    pub event: &'static str,
}

/// layer file を読み `Value::Object` だけを返す。absent
/// (ENOENT/ENOTDIR)は `Ok(None)`; `required` layer で absent の場合は
/// `{file_label} not found` error。read/parse 失敗・non-object は
/// `kind` の error code で返す。
pub(crate) fn read_yaml_layer(
    file_path: &str,
    kind: &LayerKind,
    required: bool,
) -> Result<Option<Value>, CheaperError> {
    let bytes = match std::fs::read(file_path) {
        Ok(b) => b,
        Err(e) => {
            if is_absent_file_error(&e) {
                if !required {
                    return Ok(None);
                }
                let mut details = Map::new();
                details.insert("path".to_string(), json!(file_path));
                return Err(CheaperError::new(
                    kind.code,
                    format!("{} not found: {file_path}", kind.file_label),
                )
                .with_details(details)
                .with_cause(e));
            }
            let mut details = Map::new();
            details.insert("path".to_string(), json!(file_path));
            details.insert("reason".to_string(), json!("read-failed"));
            details.insert("errno".to_string(), json!(io_errno_display(file_path, &e)));
            return Err(CheaperError::new(
                kind.code,
                format!("{} read failed: {file_path}", kind.file_label),
            )
            .with_details(details)
            .with_cause(e));
        }
    };
    // Node readFile("utf8") is lossy.
    let text = String::from_utf8_lossy(&bytes);
    // `cheaper-core` event(design §instrumentation 配置)— layer file
    // の read 境界。`file.name` は basename のみ(full path は載せない)。
    tracing::trace!(
        file.name = crate::nodepath::basename(file_path, None).as_str(),
        "{}",
        kind.event,
    );
    let parsed: Value = crate::yaml::parse_yaml(&text).map_err(|e| {
        let mut details = Map::new();
        details.insert("path".to_string(), json!(file_path));
        details.insert("reason".to_string(), json!("yaml-parse"));
        CheaperError::new(
            kind.code,
            format!("{} parse failed: {file_path}", kind.yaml_label),
        )
        .with_details(details)
        .with_cause(e)
    })?;
    match parsed {
        Value::Object(_) => Ok(Some(parsed)),
        _ => {
            let mut details = Map::new();
            details.insert("path".to_string(), json!(file_path));
            details.insert("reason".to_string(), json!("not-object"));
            Err(CheaperError::new(
                kind.code,
                format!("{} is not an object: {file_path}", kind.object_label),
            )
            .with_details(details))
        }
    }
}
