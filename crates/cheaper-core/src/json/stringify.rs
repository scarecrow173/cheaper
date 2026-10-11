//! JavaScript `JSON.stringify` 互換 serializer(F10)。
//!
//! `serde_json::to_string(_pretty)` の number 出力は Rust float 表現
//! (`1.0` → `"1.0"`、指数閾値も JS と異なる)で、`JSON.stringify`
//! 規則(整数は `.0` 省略、`|x| >= 1e21` または `0 < |x| < 1e-6` で
//! 指数表記、`-0` → `0`、非有限 → `null`)と一致しない。JS 規則の
//! JSON text が contract となる経路(provider へ渡す prompt embed・
//! token 計量の元 text・CLI envelope / usage.jsonl)は全てこの module
//! を通す。number 部は [`crate::json::number::format_number`] が正源で
//! object/array layout のみここで実装する(元々 `cheaper-rig-adapter`
//! にあったものを、CLI envelope 層も使うため core へ移設した)。
//!
//! - indent は `JSON.stringify(v, null, 2)` 相当の 2 space
//! - key 順は insertion order(serde_json `preserve_order` 前提) —
//!   JS の object key 列挙順(非 index key は挿入順)と一致
//! - 文字列 escaping は serde_json に委譲(`"` `\` 制御文字のみ
//!   escape、non-ASCII は生 UTF-8 — ECMA-262 と一致)
//! - JS で `undefined`/function/symbol 相当の値は `serde_json::Value`
//!   では表現不能なので考慮不要。number 内の NaN/Infinity は
//!   `format_number` が `null` に写像する。

use serde_json::Value;

use super::number::format_number;

/// `JSON.stringify(value)` 相当(compact — separator 無し)。
pub fn stringify(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0, 0);
    out
}

/// `JSON.stringify(value, null, 2)` 相当(2-space indent)。
pub fn stringify_pretty(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0, 2);
    out
}

/// `indent` は 1 level あたりの space 数(0 = compact)。
fn write_value(out: &mut String, value: &Value, depth: usize, indent: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => {
            // JS の number は常に f64 — serde_json::Number の整数も
            // f64 へ写してから format(JS `JSON.stringify` が parse
            // 済み number に掛ける規則と一致する)。非有限は
            // `format_number` 側で `null`。
            out.push_str(&format_number(n.as_f64().unwrap_or(f64::NAN)));
        }
        Value::String(s) => {
            out.push_str(&serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()));
        }
        Value::Array(items) => {
            // JS は空 container でも indent を展開しない(`[]` / `{}`)。
            if items.is_empty() {
                out.push_str("[]");
            } else {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    push_indent(out, depth + 1, indent);
                    write_value(out, item, depth + 1, indent);
                }
                push_indent(out, depth, indent);
                out.push(']');
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
            } else {
                out.push('{');
                for (i, (key, item)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    push_indent(out, depth + 1, indent);
                    out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".into()));
                    out.push(':');
                    if indent > 0 {
                        out.push(' ');
                    }
                    write_value(out, item, depth + 1, indent);
                }
                push_indent(out, depth, indent);
                out.push('}');
            }
        }
    }
}

/// pretty 時は改行 + `depth * indent` spaces、compact 時は何もしない。
fn push_indent(out: &mut String, depth: usize, indent: usize) {
    if indent == 0 {
        return;
    }
    out.push('\n');
    for _ in 0..depth * indent {
        out.push(' ');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn number_boundaries_match_json_stringify() {
        // F10: JS `JSON.stringify` の number 規則。
        assert_eq!(stringify(&json!(1.0)), "1");
        assert_eq!(stringify(&json!(-0.0)), "0");
        assert_eq!(stringify(&json!(1e21)), "1e+21");
        assert_eq!(stringify(&json!(0.5)), "0.5");
        assert_eq!(stringify(&json!(1e-6)), "0.000001");
        assert_eq!(stringify(&json!(1e-7)), "1e-7");
        assert_eq!(stringify(&json!(-1234567890123.5)), "-1234567890123.5");
        // serde_json::Value は非有限数を保持できない(json! が Null に
        // 落ちる)ため `null` 化は format_number の責務として確認済み。
        assert_eq!(stringify(&json!(null)), "null");
    }

    #[test]
    fn pretty_layout_matches_json_stringify_2() {
        let value = json!({"a": 1, "b": [2.5, "x"], "c": {"d": true}, "e": [], "f": {}});
        assert_eq!(
            stringify_pretty(&value),
            "{\n  \"a\": 1,\n  \"b\": [\n    2.5,\n    \"x\"\n  ],\n  \"c\": {\n    \"d\": true\n  },\n  \"e\": [],\n  \"f\": {}\n}"
        );
    }

    #[test]
    fn compact_matches_json_stringify() {
        let value = json!({"a": [1, "x"], "b": null});
        assert_eq!(stringify(&value), "{\"a\":[1,\"x\"],\"b\":null}");
    }

    #[test]
    fn string_escaping_matches_serde() {
        // `"` `\` 制御文字のみ escape — non-ASCII は生 UTF-8。
        assert_eq!(stringify(&json!("a\"b\\c\nd")), "\"a\\\"b\\\\c\\nd\"");
        assert_eq!(stringify(&json!("あ")), "\"あ\"");
    }
}
