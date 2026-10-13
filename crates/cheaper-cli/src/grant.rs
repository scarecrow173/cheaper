//! `--grant` flag 値 → 合成 `invocation` policy layer の組立
//! (DESIGN-034FntkIZGiffhdU4caSkN §(c))。
//!
//! run 経路(`run.rs`)と `workflow run` 経路(`workflow_run.rs`)の 2 caller
//! が共有する単一 builder。flag 値の `name` key を strip して
//! `{"version":1,"command":{"grants":{<name>:<spec>}}}` へ畳み、
//! 宣言順の name 一覧を diagnostic/span field 用に返す。

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Map, Value};

/// grant `propertyNames` pattern — `command.rs` の
/// `^[a-z][a-z0-9_-]{0,63}$` と同一規則を byte 走査で再現する
/// (`args.rs::is_valid_worker_name` と同じ手法)。`--grant` flag 値と
/// `policy grant add|remove <name>` positional で同一規則を共有する。
pub(crate) fn is_valid_grant_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-' || *b == b'_')
}

/// error attribution 規約 — invocation 入力の構造的失敗(malformed 値・
/// `name` 欠落/pattern 違反・grants 以外の key・同名重複・policy との
/// collision)は `InvalidRequest` + `details.layer = "invocation"` で返す。
/// grant name が関与する場合は `details.grant` に載せる。
/// (`load.rs` の `invocation_invalid_request` と同一文言 — shared
/// validation 由来の失敗は load 側が inner code 保持で畳み込む)。
fn invocation_invalid_request(
    message: impl Into<String>,
    mut details: Map<String, Value>,
) -> CheaperError {
    details.insert("layer".to_string(), Value::from("invocation"));
    CheaperError::new(ErrorCode::InvalidRequest, message).with_details(details)
}

/// `--grant` 値列 → invocation layer + 宣言順の grant name 一覧。
/// 構造的失敗(malformed JSON・`name` 欠落/pattern 違反・同名重複)は
/// `InvalidRequest` + `details.layer = "invocation"` で返す
/// (`details.grant` は name が確定した場合のみ)。`grants` が空なら
/// `(None, vec![])`。
// signature は design §(c) の固定 — tuple return を type alias 化
// しない(2 要素は layer と names で対称ではなく、alias 化の意義が薄い)。
#[allow(clippy::type_complexity)]
pub(crate) fn build_invocation_layer(
    grants: &[String],
) -> Result<(Option<Map<String, Value>>, Vec<String>), CheaperError> {
    let mut specs = Map::new();
    let mut names = Vec::new();
    for raw in grants {
        // flag 値は complete JSON object 必須。cmdline 由来値を error
        // message/details へ echo しない(先頭断片も載せない)。
        let value: Value = serde_json::from_str(raw).map_err(|_| {
            invocation_invalid_request("invalid --grant value: malformed JSON", Map::new())
        })?;
        let Value::Object(mut spec) = value else {
            return Err(invocation_invalid_request(
                "invalid --grant value: expected a JSON object",
                Map::new(),
            ));
        };
        // `name` は spec から strip する — grant spec schema の
        // `additionalProperties: false` が `name` key を必ず reject
        // するため、残したまま畳む実装は常に失敗する(strip は不変条件)。
        let name = match spec.remove("name") {
            Some(Value::String(name)) => name,
            // 欠落・非 string — name 未確定のため `details.grant` は省略。
            _ => {
                return Err(invocation_invalid_request(
                    "invalid --grant value: `name` is required and must be a string",
                    Map::new(),
                ))
            }
        };
        if !is_valid_grant_name(&name) {
            return Err(invocation_invalid_request(
                format!("invalid --grant value: invalid grant name: {name}"),
                Map::from_iter([("grant".to_string(), json!(name))]),
            ));
        }
        // intra-invocation duplicate — 宣言順 push の暗黙 last-wins を
        // 許さない。
        if specs.contains_key(&name) {
            return Err(invocation_invalid_request(
                format!("invalid --grant value: duplicate grant name: {name}"),
                Map::from_iter([("grant".to_string(), json!(name))]),
            ));
        }
        specs.insert(name.clone(), Value::Object(spec));
        names.push(name);
    }
    if names.is_empty() {
        return Ok((None, names));
    }
    let layer = json!({"version": 1, "command": {"grants": specs}});
    let Value::Object(layer) = layer else {
        unreachable!("object literal");
    };
    Ok((Some(layer), names))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer_detail(e: &CheaperError) -> Option<&Value> {
        e.details().and_then(|d| d.get("layer"))
    }

    /// flag JSON → `{"version":1,"command":{"grants":{...}}}` 組立・
    /// `name` strip・宣言順 names の pin(Risk #2 — `name` を残したまま
    /// schema へ流すと `additionalProperties: false` で必ず失敗する)。
    #[test]
    fn builds_layer_and_strips_name() {
        let grants = vec![
            r#"{"name":"self","executable":"/bin/self","argv":["--x"]}"#.to_string(),
            r#"{"name":"other","executable":"/bin/other","env":{"A":"1"}}"#.to_string(),
        ];
        let (layer, names) = build_invocation_layer(&grants).unwrap();
        assert_eq!(names, vec!["self".to_string(), "other".to_string()]);
        let layer = layer.expect("non-empty grants produce a layer");
        assert_eq!(layer["version"], json!(1));
        let grants_map = layer["command"]["grants"].as_object().unwrap();
        assert_eq!(grants_map.len(), 2);
        // `name` key は spec 側へ残らない。
        assert_eq!(
            grants_map["self"],
            json!({"executable": "/bin/self", "argv": ["--x"]})
        );
        assert_eq!(
            grants_map["other"],
            json!({"executable": "/bin/other", "env": {"A": "1"}})
        );
    }

    /// malformed JSON / 非 object → `InvalidRequest` + layer(name 未確定
    /// のため `details.grant` 無し・flag 値断片を echo しない)。
    #[test]
    fn rejects_malformed_json() {
        for raw in ["{not json", "[1,2]", "\"str\"", "42"] {
            let e = build_invocation_layer(&[raw.to_string()]).unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidRequest, "{raw}");
            assert_eq!(layer_detail(&e), Some(&json!("invocation")), "{raw}");
            assert!(
                !e.details().unwrap().contains_key("grant"),
                "{raw}: {:?}",
                e.details()
            );
        }
    }

    /// `name` 欠落・非 string・pattern 違反の reject。pattern 違反のみ
    /// `details.grant` を持つ(name 確定済み)。
    #[test]
    fn rejects_missing_or_invalid_name() {
        // 欠落 / 非 string — details.grant 無し。
        for raw in [
            r#"{"executable":"/bin/x"}"#,
            r#"{"name":3,"executable":"/bin/x"}"#,
        ] {
            let e = build_invocation_layer(&[raw.to_string()]).unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidRequest, "{raw}");
            assert_eq!(layer_detail(&e), Some(&json!("invocation")), "{raw}");
            assert!(!e.details().unwrap().contains_key("grant"), "{raw}");
        }
        // pattern 違反(先頭大文字・space・65 文字超)— details.grant 付き。
        let too_long = "a".repeat(65);
        for (raw, expected_name) in [
            (
                r#"{"name":"Self","executable":"/bin/x"}"#.to_string(),
                "Self",
            ),
            (r#"{"name":"a b","executable":"/bin/x"}"#.to_string(), "a b"),
            (
                format!("{{\"name\":\"{too_long}\",\"executable\":\"/bin/x\"}}"),
                too_long.as_str(),
            ),
        ] {
            let e = build_invocation_layer(std::slice::from_ref(&raw)).unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidRequest, "{raw}");
            assert_eq!(layer_detail(&e), Some(&json!("invocation")), "{raw}");
            assert_eq!(e.details().unwrap()["grant"], json!(expected_name), "{raw}");
        }
    }

    /// intra-invocation 同名重複は `InvalidRequest` + `details.grant`
    /// (宣言順 push の暗黙 last-wins を許さない — AC13)。
    #[test]
    fn rejects_duplicate_names() {
        let grants = vec![
            r#"{"name":"self","executable":"/bin/a"}"#.to_string(),
            r#"{"name":"other","executable":"/bin/b"}"#.to_string(),
            r#"{"name":"self","executable":"/bin/c"}"#.to_string(),
        ];
        let e = build_invocation_layer(&grants).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRequest);
        assert_eq!(e.details().unwrap()["grant"], json!("self"));
        assert_eq!(layer_detail(&e), Some(&json!("invocation")));
    }

    /// 空入力 → `(None, vec![])`(layer 非生成 — load 経路に差分無し)。
    #[test]
    fn empty_returns_none() {
        let (layer, names) = build_invocation_layer(&[]).unwrap();
        assert!(layer.is_none());
        assert!(names.is_empty());
    }
}
