//! Port of `packages/workflow/src/json.ts`.
//!
//! `serde_json::Value` は常に plain JSON(非有限 number・symbol・getter・
//! cyclic 参照を表現できない)なので、`clone_json` の残る責務は「object
//! key を code-unit 順に正規化した深い複製」のみ。getter/toJSON 非実行・
//! cyclic 拒否は型レベルで保証される。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::code_unit_compare;
use serde_json::{Map, Value};

/// `Object.hasOwn(value, key)`。
pub fn has_own(value: &Map<String, Value>, key: &str) -> bool {
    value.contains_key(key)
}

/// `isObject`(plain object 判定)。`serde_json::Value::Object` は常に
/// plain object に対応する。
pub fn is_object(value: &Value) -> bool {
    matches!(value, Value::Object(_))
}

/// `cloneJson(value, code)`。TS 側は object の own enumerable key を
/// code-unit sort して再構成するため、ここでも key を `code_unit_compare`
/// で sort した `Map` に詰め直す(key 順は serialization/pattern 一致に
/// 観測可能)。
///
/// `Value` が到達不能な失敗(非有限 number)だけは `code` で拒否する。
pub fn clone_json(value: &Value, code: ErrorCode) -> Result<Value, CheaperError> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(value.clone()),
        Value::Number(n) => {
            if n.as_f64().is_some_and(f64::is_finite) {
                Ok(value.clone())
            } else {
                Err(CheaperError::new(code, "value must be plain JSON"))
            }
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(clone_json(item, code)?);
            }
            Ok(Value::Array(out))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| code_unit_compare(a, b));
            let mut out = Map::with_capacity(map.len());
            for key in keys {
                out.insert(key.clone(), clone_json(&map[key], code)?);
            }
            Ok(Value::Object(out))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn clone_json_sorts_object_keys_by_code_unit() {
        let value = json!({"z": {"b": 1, "a": 2}, "a": [{"y": 1, "x": 2}]});
        let cloned = clone_json(&value, ErrorCode::InvalidRequest).unwrap();
        let keys: Vec<&String> = cloned.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["a", "z"]);
        let inner: Vec<&String> = cloned["z"].as_object().unwrap().keys().collect();
        assert_eq!(inner, ["a", "b"]);
        // 値は等しい。
        assert_eq!(cloned, value);
    }

    #[test]
    fn clone_json_preserves_scalars_and_arrays() {
        for value in [
            json!(null),
            json!(true),
            json!("x"),
            json!(7.5),
            json!([1, "a", null]),
        ] {
            assert_eq!(clone_json(&value, ErrorCode::InvalidInput).unwrap(), value);
        }
    }
}
