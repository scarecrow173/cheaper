//! Port of `packages/workflow/test/bindings.test.ts`。

mod common;

use std::collections::HashMap;

use cheaper_core::errors::ErrorCode;
use serde_json::{json, Map, Value};

use cheaper_workflow::{
    compile_dag, materialize_bindings, resolve_pointer, scan_bindings, validate_pointer_schema,
    DependencyEdge, MaterializeOptions,
};

fn code_of<T>(result: Result<T, cheaper_core::errors::CheaperError>) -> ErrorCode {
    // `unwrap_err` は `T: Debug` を要求するため `err()` 経由。
    result.err().unwrap().code()
}

fn inputs(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn results(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn materialize(
    value: &Value,
    inputs: &Map<String, Value>,
) -> Result<Value, cheaper_core::errors::CheaperError> {
    materialize_bindings(
        value,
        inputs,
        &HashMap::new(),
        MaterializeOptions::default(),
    )
}

fn materialize_with(
    value: &Value,
    inputs: &Map<String, Value>,
    results: &HashMap<String, Value>,
) -> Result<Value, cheaper_core::errors::CheaperError> {
    materialize_bindings(value, inputs, results, MaterializeOptions::default())
}

// binding の単一解釈と result-root Pointer
#[test]
fn input_json_types_literal_preserved_no_rescan() {
    let inserted = json!({ "$from": "ghost", "path": "" });
    let template = json!({
        "input": { "$input": "data" },
        "literal": { "$literal": { "$input": "missing", "extra": true } },
        "array": [{ "$input": "bool" }, { "$input": "number" }, { "$input": "nil" }]
    });
    let scanned = scan_bindings(&template).unwrap();
    assert_eq!(scanned.inputs, vec!["bool", "data", "nil", "number"]);
    assert!(scanned.from.is_empty());
    let resolved = materialize(
        &template,
        &inputs(&[
            ("data", inserted.clone()),
            ("bool", json!(true)),
            ("number", json!(7)),
            ("nil", Value::Null),
        ]),
    )
    .unwrap();
    assert_eq!(
        resolved,
        json!({
            "input": inserted,
            "literal": { "$input": "missing", "extra": true },
            "array": [true, 7, null]
        })
    );
}

#[test]
fn literal_stripped_once_from_inserted_not_rescanned() {
    let result = json!({ "$input": "ghost" });
    assert_eq!(
        materialize_with(
            &json!({ "$from": "a", "path": "" }),
            &Map::new(),
            &results(&[("a", result.clone())]),
        )
        .unwrap(),
        result
    );
    assert_eq!(
        materialize(&json!({ "$literal": { "$literal": 42 } }), &Map::new()).unwrap(),
        json!({ "$literal": 42 })
    );
}

#[test]
fn invalid_reserved_wrapper_is_definition_error() {
    for value in [
        json!({ "$input": "x", "extra": 1 }),
        json!({ "$input": 7 }),
        json!({ "$from": "a" }),
        json!({ "$from": "a", "path": "result" }),
        json!({ "$from": "a", "path": "/bad~2" }),
        json!({ "$from": "a", "path": "", "extra": true }),
        json!({ "$literal": null, "other": 1 }),
        json!({ "$input": "x", "$from": "a", "path": "" }),
    ] {
        assert_eq!(
            code_of(scan_bindings(&value)),
            ErrorCode::InvalidRequest,
            "value: {value}"
        );
    }
}

#[test]
fn empty_escape_array_null_resolved_from_result() {
    let result = json!({ "details": { "a/b": { "~": [null, 9] } }, "result": "nested" });
    assert_eq!(
        resolve_pointer(&result, "", ErrorCode::InvalidInput).unwrap(),
        result
    );
    assert_eq!(
        resolve_pointer(&result, "/details/a~1b/~0/0", ErrorCode::InvalidInput).unwrap(),
        Value::Null
    );
    assert_eq!(
        resolve_pointer(&result, "/details/a~1b/~0/1", ErrorCode::InvalidInput).unwrap(),
        json!(9)
    );
    assert_eq!(
        resolve_pointer(&result, "/result", ErrorCode::InvalidInput).unwrap(),
        json!("nested")
    );
    assert_eq!(
        resolve_pointer(&json!({ "": 5 }), "/", ErrorCode::InvalidInput).unwrap(),
        json!(5)
    );
}

#[test]
fn missing_invalid_index_inherited_key_rejected() {
    let result = json!({ "details": { "a/b": { "~": [null, 9] } } });
    for pointer in [
        "/missing",
        "/details/a~1b/~0/2",
        "/details/a~1b/~0/01",
        "/details/a~1b/~0/-",
        "/toString",
    ] {
        assert!(
            resolve_pointer(&result, pointer, ErrorCode::InvalidInput).is_err(),
            "pointer: {pointer}"
        );
    }
}

#[test]
fn missing_input_or_result_is_explicit_invalid_input() {
    assert_eq!(
        code_of(materialize(&json!({ "$input": "absent" }), &Map::new())),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code_of(materialize_with(
            &json!({ "$from": "absent", "path": "" }),
            &Map::new(),
            &HashMap::new(),
        )),
        ErrorCode::InvalidInput
    );
}

// 静的 Pointer 検査の確実性
#[test]
fn provably_impossible_schema_path_is_definition_error() {
    for (schema, path) in [
        (json!({ "type": "string" }), "/child"),
        (
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
            "/absent",
        ),
        (json!({ "type": "array", "items": {} }), "/01"),
        (json!({ "type": "array", "maxItems": 2, "items": {} }), "/2"),
        (
            json!({ "type": "array", "prefixItems": [{ "type": "string" }], "items": false }),
            "/1",
        ),
    ] {
        assert_eq!(
            code_of(validate_pointer_schema(schema.as_object().unwrap(), path)),
            ErrorCode::InvalidRequest,
            "schema: {schema} path: {path}"
        );
    }
}

#[test]
fn ref_union_open_schema_not_rejected_by_guess() {
    for (schema, path) in [
        (
            json!({ "$ref": "#/$defs/value", "$defs": { "value": {} } }),
            "/unresolved",
        ),
        (json!({ "type": ["object", "null"] }), "/unresolved"),
        (
            json!({ "anyOf": [{ "type": "object" }, { "type": "array" }] }),
            "/unresolved",
        ),
        (
            json!({ "type": "object", "additionalProperties": true }),
            "/unresolved",
        ),
    ] {
        assert!(
            validate_pointer_schema(schema.as_object().unwrap(), path).is_ok(),
            "schema: {schema} path: {path}"
        );
    }
}

// DAG の正本 order/edges
#[test]
fn dedup_edges_locale_independent_order() {
    let result = compile_dag(
        &["z", "2", "10", "a"].map(String::from),
        &[
            DependencyEdge {
                from: "a".into(),
                to: "z".into(),
            },
            DependencyEdge {
                from: "a".into(),
                to: "z".into(),
            },
        ],
    )
    .unwrap();
    assert_eq!(result.order, vec!["10", "2", "a", "z"]);
    assert_eq!(
        result.edges,
        vec![DependencyEdge {
            from: "a".into(),
            to: "z".into()
        }]
    );
}

#[test]
fn self_unknown_cycle_are_definition_errors() {
    let cases: Vec<Vec<DependencyEdge>> = vec![
        vec![DependencyEdge {
            from: "a".into(),
            to: "a".into(),
        }],
        vec![DependencyEdge {
            from: "unknown".into(),
            to: "b".into(),
        }],
        vec![
            DependencyEdge {
                from: "a".into(),
                to: "b".into(),
            },
            DependencyEdge {
                from: "b".into(),
                to: "a".into(),
            },
        ],
    ];
    for edges in cases {
        assert_eq!(
            compile_dag(&["a", "b"].map(String::from), &edges)
                .unwrap_err()
                .code(),
            ErrorCode::InvalidRequest
        );
    }
}
