//! Port of `packages/workflow/src/bindings.ts`.

use std::collections::{HashMap, HashSet};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::code_unit_compare;
use serde_json::{json, Map, Value};

use crate::json::{clone_json, has_own};
use crate::types::{Binding, FromBinding, JsonSchema};

/// JS の JSON Pointer segment 判定(`/^(0|[1-9][0-9]*)$/`)。
fn is_array_index(segment: &str) -> bool {
    if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // 先頭 0 は "0" 単独のみ許可。
    segment == "0" || !segment.starts_with('0')
}

/// `pointerSegments`: `""` → `[]`、非 `/` 始まり・不正 `~` escape は
/// `INVALID_REQUEST`。`~1`→`/`・`~0`→`~` の順で unescape する。
pub fn pointer_segments(pointer: &str) -> Result<Vec<String>, CheaperError> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    if !pointer.starts_with('/') || has_bad_escape(pointer) {
        return Err(CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("invalid JSON Pointer: {pointer}"),
        ));
    }
    Ok(pointer[1..]
        .split('/')
        .map(|part| part.replace("~1", "/").replace("~0", "~"))
        .collect())
}

/// `/~(?:[^01]|$)/`。
fn has_bad_escape(pointer: &str) -> bool {
    let bytes = pointer.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'~' {
            match bytes.get(i + 1) {
                Some(b'0') | Some(b'1') => {}
                _ => return true,
            }
        }
    }
    false
}

/// `parseBinding`: reserved key を含む object は必ず exact wrapper として
/// 解釈する。wrapper 形状を満たさなければ `INVALID_REQUEST`。
pub fn parse_binding(value: &Value) -> Result<Option<Binding>, CheaperError> {
    let Value::Object(map) = value else {
        return Ok(None);
    };
    if !["$input", "$from", "$literal"]
        .iter()
        .any(|k| map.contains_key(*k))
    {
        return Ok(None);
    }
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by(|a, b| code_unit_compare(a, b));
    let joined = keys
        .iter()
        .map(|k| k.as_str())
        .collect::<Vec<_>>()
        .join(",");
    if joined == "$input" {
        if let Some(name) = map["$input"].as_str() {
            if !name.is_empty() {
                return Ok(Some(Binding::Input(name.to_string())));
            }
        }
    } else if joined == "$literal" {
        return Ok(Some(Binding::Literal(map["$literal"].clone())));
    } else if joined == "$from,path" {
        let from = map["$from"].as_str();
        let path = map["path"].as_str();
        if let (Some(from), Some(path)) = (from, path) {
            if !from.is_empty() {
                // pointer の validity はここで確認する(throw を伝播)。
                pointer_segments(path)?;
                return Ok(Some(Binding::From(FromBinding {
                    from: from.to_string(),
                    path: path.to_string(),
                })));
            }
        }
    }
    Err(
        CheaperError::new(ErrorCode::InvalidRequest, "invalid binding wrapper").with_details({
            let mut details = Map::new();
            details.insert(
                "keys".to_string(),
                Value::Array(map.keys().cloned().map(Value::String).collect()),
            );
            details
        }),
    )
}

/// `scanBindings` の結果。`inputs` は dedupe 済み code-unit sort、
/// `from` は document 順。
pub struct ScannedBindings {
    pub inputs: Vec<String>,
    pub from: Vec<FromBinding>,
}

/// `scanBindings`。
pub fn scan_bindings(value: &Value) -> Result<ScannedBindings, CheaperError> {
    let mut inputs = HashSet::new();
    let mut from = Vec::new();
    fn scan(
        node: &Value,
        inputs: &mut HashSet<String>,
        from: &mut Vec<FromBinding>,
    ) -> Result<(), CheaperError> {
        if let Some(binding) = parse_binding(node)? {
            match binding {
                Binding::Input(name) => {
                    inputs.insert(name);
                }
                Binding::From(b) => from.push(b),
                Binding::Literal(_) => {}
            }
            return Ok(());
        }
        match node {
            Value::Array(items) => {
                for child in items {
                    scan(child, inputs, from)?;
                }
            }
            Value::Object(map) => {
                for child in map.values() {
                    scan(child, inputs, from)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    scan(value, &mut inputs, &mut from)?;
    let mut inputs: Vec<String> = inputs.into_iter().collect();
    inputs.sort_by(|a, b| code_unit_compare(a, b));
    Ok(ScannedBindings { inputs, from })
}

/// `resolvePointer`: result 自身を root に JSON Pointer を辿る。
/// array index は `^(0|[1-9][0-9]*)$` + bounds、object は own key。
pub fn resolve_pointer(
    result: &Value,
    pointer: &str,
    code: ErrorCode,
) -> Result<Value, CheaperError> {
    let mut value = result;
    for segment in pointer_segments(pointer)? {
        let missing = || {
            CheaperError::new(code, format!("missing value at JSON Pointer: {pointer}"))
                .with_details({
                    let mut details = Map::new();
                    details.insert("pointer".to_string(), json!(pointer));
                    details.insert("segment".to_string(), json!(segment));
                    details
                })
        };
        match value {
            Value::Object(map) => {
                value = map.get(&segment).ok_or_else(missing)?;
            }
            Value::Array(items) => {
                if !is_array_index(&segment) {
                    return Err(missing());
                }
                let index = segment.parse::<usize>().map_err(|_| missing())?;
                value = items.get(index).ok_or_else(missing)?;
            }
            _ => return Err(missing()),
        }
    }
    clone_json(value, code)
}

/// `materializeBindings` の option。`defer_from` は compile 表示専用。
/// `skipped_from` は condition-skipped producer の job id 集合 —
/// `$from` が集合内の job を指し result が不在のとき、binding 式
/// 全体を `null` に materialize する(path 解決は行わない)。
#[derive(Debug, Clone, Copy, Default)]
pub struct MaterializeOptions<'a> {
    pub defer_from: bool,
    pub code: Option<ErrorCode>,
    pub skipped_from: Option<&'a HashSet<String>>,
}

/// `materializeBindings`。binding でない object は key を code-unit sort
/// して再構成する(TS `Object.keys(node).sort()` と同じ)。
pub fn materialize_bindings(
    value: &Value,
    inputs: &Map<String, Value>,
    results: &HashMap<String, Value>,
    options: MaterializeOptions<'_>,
) -> Result<Value, CheaperError> {
    let code = options.code.unwrap_or(ErrorCode::InvalidInput);
    fn materialize(
        node: &Value,
        inputs: &Map<String, Value>,
        results: &HashMap<String, Value>,
        code: ErrorCode,
        defer_from: bool,
        skipped_from: Option<&HashSet<String>>,
    ) -> Result<Value, CheaperError> {
        if let Some(binding) = parse_binding(node)? {
            return match binding {
                Binding::Literal(v) => clone_json(&v, code),
                Binding::Input(name) => {
                    if !has_own(inputs, &name) {
                        return Err(CheaperError::new(code, format!("missing input: {name}")));
                    }
                    clone_json(&inputs[&name], code)
                }
                Binding::From(b) => {
                    if defer_from {
                        return clone_json(node, code);
                    }
                    match results.get(&b.from) {
                        Some(result) => resolve_pointer(result, &b.path, code),
                        // condition-skipped producer への参照は
                        // binding 式全体が `null`。
                        None if skipped_from.is_some_and(|s| s.contains(&b.from)) => {
                            Ok(Value::Null)
                        }
                        None => Err(CheaperError::new(
                            code,
                            format!("missing job result: {}", b.from),
                        )),
                    }
                }
            };
        }
        match node {
            Value::Array(items) => items
                .iter()
                .map(|child| materialize(child, inputs, results, code, defer_from, skipped_from))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_by(|a, b| code_unit_compare(a, b));
                let mut out = Map::with_capacity(map.len());
                for key in keys {
                    out.insert(
                        key.clone(),
                        materialize(&map[key], inputs, results, code, defer_from, skipped_from)?,
                    );
                }
                Ok(Value::Object(out))
            }
            _ => Ok(node.clone()),
        }
    }
    materialize(
        value,
        inputs,
        results,
        code,
        options.defer_from,
        options.skipped_from,
    )
}

/// `validatePointerSchema` — 閉じた単純 schema で証明できる不可能性だけ
/// 拒否。ref/union/open schema は runtime に残す。
pub fn validate_pointer_schema(schema: &JsonSchema, pointer: &str) -> Result<(), CheaperError> {
    // `current` は降下のたびに所有 clone を保持する(TS は参照を
    // 再代入するだけだが、ここでは値 semantics で同じ結果になる)。
    let mut current = Value::Object(schema.clone());
    for segment in pointer_segments(pointer)? {
        // boolean schema(`true`/`false`)— TS の `current === true` /
        // `current === false` に対応。
        match current {
            Value::Bool(true) => return Ok(()),
            Value::Bool(false) => {
                return Err(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    format!("impossible output path: {pointer}"),
                ));
            }
            _ => {}
        }
        let Value::Object(map) = &current else {
            // 非 object(例: scalar / `items: undefined` 由来の Null)—
            // TS の `!isObject(current)` で return と同じ。
            return Ok(());
        };
        if [
            "$ref",
            "$dynamicRef",
            "anyOf",
            "oneOf",
            "allOf",
            "if",
            "not",
        ]
        .iter()
        .any(|key| map.contains_key(*key))
        {
            return Ok(());
        }
        let Some(ty) = map.get("type").and_then(Value::as_str) else {
            return Ok(());
        };
        match ty {
            "object" => {
                let next = map
                    .get("properties")
                    .and_then(Value::as_object)
                    .and_then(|properties| properties.get(&segment));
                if let Some(next) = next {
                    current = next.clone();
                    continue;
                }
                if map.get("additionalProperties") == Some(&Value::Bool(false))
                    && !map.contains_key("patternProperties")
                {
                    return Err(CheaperError::new(
                        ErrorCode::InvalidRequest,
                        format!("unknown closed output property: {pointer}"),
                    ));
                }
                return Ok(());
            }
            "array" => {
                if !is_array_index(&segment)
                    || map
                        .get("maxItems")
                        .and_then(Value::as_f64)
                        .is_some_and(|max| segment.parse::<f64>().unwrap_or(f64::MAX) >= max)
                {
                    return Err(CheaperError::new(
                        ErrorCode::InvalidRequest,
                        format!("impossible array output path: {pointer}"),
                    ));
                }
                let index = segment.parse::<usize>().unwrap_or(usize::MAX);
                if let Some(prefix) = map.get("prefixItems").and_then(Value::as_array) {
                    if index < prefix.len() {
                        current = prefix[index].clone();
                        continue;
                    }
                }
                match map.get("items") {
                    Some(Value::Bool(false)) => {
                        return Err(CheaperError::new(
                            ErrorCode::InvalidRequest,
                            format!("impossible array output path: {pointer}"),
                        ));
                    }
                    Some(items) => {
                        current = items.clone();
                        continue;
                    }
                    // `items` 欠落 → `current.items` === undefined →
                    // 次 iteration の `!isObject` で return。直接 return と
                    // 同じ結果。
                    None => return Ok(()),
                }
            }
            _ => {
                return Err(CheaperError::new(
                    ErrorCode::InvalidRequest,
                    format!("primitive output has no child: {pointer}"),
                ));
            }
        }
    }
    Ok(())
}
