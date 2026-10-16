//! Port of `packages/workflow/test/compiler.test.ts`。

use crate::common;

use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::worker::schema::{compile_schema, SchemaKind};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use cheaper_workflow::{
    DependencyEdge, ResourceInference, ResourceInferenceJob, ResourceInferenceResult,
    WorkflowJobState,
};

use common::{definition, FixtureSet};

fn code(err: &ToolError) -> Option<ErrorCode> {
    err.downcast_ref::<CheaperError>().map(|e| e.code())
}

fn err_code(
    result: Result<Arc<cheaper_workflow::CompiledWorkflow>, ToolError>,
) -> Option<ErrorCode> {
    // `unwrap_err` は `T: Debug` を要求するため `err()` 経由。
    code(&result.err().unwrap())
}

fn job<'a>(
    compiled: &'a cheaper_workflow::CompiledWorkflow,
    id: &str,
) -> &'a cheaper_workflow::CompiledJob {
    compiled.job(id).expect("job present")
}

// compiled job output boundary
#[tokio::test]
async fn canonical_schema_validates_and_returns_snapshot() {
    let fixtures = FixtureSet::new();
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "read", "input": { "path": "a" } });
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    let value =
        json!({ "content": [{ "type": "text", "text": "result" }], "details": { "nested": [1] } });
    let result = compiled.validate_job_output("a", &value).unwrap();
    assert_eq!(
        result,
        json!({ "content": [{ "type": "text", "text": "result" }], "details": { "nested": [1] } })
    );
    assert_eq!(
        compiled
            .validate_job_output("a", &json!(42))
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
    // `undefined` — Rust 側は Null が JSON 境界上の最近傍。
    assert_eq!(
        compiled
            .validate_job_output("a", &Value::Null)
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
    assert_eq!(
        compiled
            .validate_job_output("missing", &value)
            .unwrap_err()
            .code(),
        ErrorCode::InvalidRequest
    );
}

#[tokio::test]
async fn worker_schema_uses_same_compiler_seam() {
    let mut fixtures = FixtureSet::new();
    fixtures.worker_mut().output_schema = Arc::new(
        compile_schema(
            json!({ "type": "integer" }).as_object().unwrap().clone(),
            SchemaKind::Output,
        )
        .unwrap(),
    );
    let mut raw = definition();
    let worker_name = fixtures.worker().name.clone();
    raw["jobs"]["a"] =
        json!({ "kind": "worker", "worker": worker_name, "input": { "task": "read" } });
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert_eq!(
        compiled.validate_job_output("a", &json!(1)).unwrap(),
        json!(1)
    );
    assert_eq!(
        compiled
            .validate_job_output("a", &json!("1"))
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
}

// public inputs/defaults
#[tokio::test]
async fn root_defaults_preserve_json_types_no_invocation_sharing() {
    let mut raw = definition();
    raw["inputs"] = json!({
        "object": { "type": "object", "default": { "nested": [1] } },
        "flag": { "type": "boolean", "default": true },
        "number": { "type": "number", "default": 3 },
        "nil": { "type": ["string", "null"], "default": null },
        "provided": { "type": "array" }
    });
    raw["jobs"]["a"] = json!({
        "kind": "tool",
        "tool": "grep",
        "input": {
            "object": { "$input": "object" },
            "flag": { "$input": "flag" },
            "number": { "$input": "number" },
            "nil": { "$input": "nil" },
            "provided": { "$input": "provided" }
        }
    });
    let input = json!({ "provided": ["caller"] });
    let fixtures = FixtureSet::new();
    let a = fixtures
        .compiler()
        .compile(&raw, Some(&input))
        .await
        .unwrap();
    let b = fixtures
        .compiler()
        .compile(&raw, Some(&input))
        .await
        .unwrap();
    assert_eq!(
        job(&a, "a").input,
        json!({
            "object": { "nested": [1] },
            "flag": true,
            "number": 3,
            "nil": null,
            "provided": ["caller"]
        })
    );
    // 呼び出し後の mutation が別 compile の snapshot に漏れない。
    assert_eq!(b.inputs["object"], json!({ "nested": [1] }));
    assert_eq!(raw["inputs"]["object"]["default"], json!({ "nested": [1] }));
    assert_eq!(input, json!({ "provided": ["caller"] }));
    assert_eq!(
        a.materialize_job_input("a", &HashMap::new()).unwrap(),
        job(&b, "a").input
    );
}

#[tokio::test]
async fn missing_invalid_unknown_non_object_inputs_are_invalid() {
    for input in [
        json!({}),
        json!({ "x": "wrong" }),
        json!({ "x": 1, "unknown": true }),
        Value::Null,
        json!([]),
        json!("x"),
    ] {
        let mut raw = definition();
        raw["inputs"]["x"] = json!({ "type": "number" });
        let fixtures = FixtureSet::new();
        assert_eq!(
            err_code(fixtures.compiler().compile(&raw, Some(&input)).await),
            Some(ErrorCode::InvalidInput),
            "input: {input}"
        );
    }
}

#[test]
fn invalid_default_is_definition_error_without_invocation() {
    let mut raw = definition();
    raw["inputs"]["x"] = json!({ "type": "number", "default": "bad" });
    let fixtures = FixtureSet::new();
    let err = fixtures.compiler().project_interface(&raw).unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::InvalidRequest));
}

#[tokio::test]
async fn nested_defaults_are_not_completed() {
    let mut raw = definition();
    raw["inputs"]["x"] = json!({
        "type": "object",
        "required": ["nested"],
        "properties": { "nested": { "type": "number", "default": 1 } }
    });
    let fixtures = FixtureSet::new();
    assert_eq!(
        err_code(
            fixtures
                .compiler()
                .compile(&raw, Some(&json!({ "x": {} })))
                .await
        ),
        Some(ErrorCode::InvalidInput)
    );
}

#[tokio::test]
async fn local_defs_use_core_2020_12_semantics() {
    let mut raw = definition();
    raw["inputs"]["x"] = json!({
        "$defs": { "value": { "type": "integer", "minimum": 2 } },
        "$ref": "#/$defs/value",
        "default": 2
    });
    let fixtures = FixtureSet::new();
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert_eq!(compiled.inputs["x"], json!(2));
    assert_eq!(
        err_code(
            fixtures
                .compiler()
                .compile(&raw, Some(&json!({ "x": 1 })))
                .await
        ),
        Some(ErrorCode::InvalidInput)
    );
}

#[test]
fn external_ref_is_not_fetched_cause_preserved() {
    let mut raw = definition();
    raw["inputs"]["x"] = json!({ "$ref": "https://example.invalid/schema.json" });
    let fixtures = FixtureSet::new();
    let err = fixtures.compiler().validate_definition(&raw).unwrap_err();
    let cheaper = err.downcast_ref::<CheaperError>().expect("CheaperError");
    assert_eq!(cheaper.code(), ErrorCode::InvalidRequest);
    // cause chain 内の INVALID_INPUT_SCHEMA を検査。
    let mut source = cheaper.source();
    let mut found = false;
    while let Some(e) = source {
        if let Some(inner) = e.downcast_ref::<CheaperError>() {
            if inner.code() == ErrorCode::InvalidInputSchema {
                found = true;
                break;
            }
        }
        source = e.source();
    }
    assert!(found, "expected INVALID_INPUT_SCHEMA in cause chain");
}

#[tokio::test]
async fn prototype_collision_keys_are_safe() {
    let mut raw = definition();
    raw["inputs"] = serde_json::from_str(
        r#"{"__proto__":{"type":"number","default":1},"constructor":{"type":"number","default":2}}"#,
    )
    .unwrap();
    raw["jobs"]["a"] = json!({
        "kind": "tool", "tool": "grep", "input": { "$input": "__proto__" }
    });
    let fixtures = FixtureSet::new();
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert!(compiled.inputs.contains_key("__proto__"));
    assert_eq!(
        compiled
            .materialize_job_input("a", &HashMap::new())
            .unwrap(),
        json!(1)
    );
}

// definition と deterministic DAG
#[tokio::test]
async fn needs_from_edge_dedupe_definition_order_independent() {
    let mut raw = definition();
    raw["jobs"] = json!({
        "z": {
            "kind": "tool", "tool": "grep",
            "input": { "$from": "a", "path": "/content" },
            "needs": ["b", "a", "a"]
        },
        "b": { "kind": "tool", "tool": "grep", "input": {} },
        "a": { "kind": "tool", "tool": "grep", "input": {} }
    });
    let mut reversed = definition();
    reversed["jobs"] = json!({
        "a": { "kind": "tool", "tool": "grep", "input": {} },
        "b": { "kind": "tool", "tool": "grep", "input": {} },
        "z": {
            "kind": "tool", "tool": "grep",
            "input": { "$from": "a", "path": "/content" },
            "needs": ["a", "b"]
        }
    });
    let fixtures = FixtureSet::new();
    let a = fixtures.compiler().compile(&raw, None).await.unwrap();
    let b = fixtures.compiler().compile(&reversed, None).await.unwrap();
    assert_eq!(a.order, vec!["a", "b", "z"]);
    assert_eq!(
        a.edges,
        vec![
            DependencyEdge {
                from: "a".into(),
                to: "z".into()
            },
            DependencyEdge {
                from: "b".into(),
                to: "z".into()
            },
        ]
    );
    // JSON 表現は definition 順に依存しない(serializable subset)。
    let surface = |w: &cheaper_workflow::CompiledWorkflow| {
        json!({
            "inputs": w.inputs.clone(),
            "jobs": w.jobs.iter().map(|(id, j)| (id.clone(), serde_json::to_value(j).unwrap())).collect::<Map<String, Value>>(),
            "outputs": w.outputs.clone(),
            "order": w.order.clone(),
            "edges": w.edges.clone(),
        })
    };
    assert_eq!(surface(&a), surface(&b));
    // (deref) Arc<CompiledWorkflow> は &CompiledWorkflow に coerce される。
    assert_eq!(
        a.jobs.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "z"]
    );
}

#[tokio::test]
async fn literal_defaults_do_not_create_from_edges() {
    let mut raw = definition();
    raw["inputs"]["x"] = json!({
        "type": "object", "default": { "$from": "ghost", "path": "" }
    });
    raw["jobs"]["a"] = json!({
        "kind": "tool", "tool": "grep",
        "input": {
            "x": { "$input": "x" },
            "y": { "$literal": { "$from": "ghost", "path": "" } }
        }
    });
    let fixtures = FixtureSet::new();
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert!(compiled.edges.is_empty());
    assert_eq!(
        compiled
            .materialize_job_input("a", &HashMap::new())
            .unwrap(),
        json!({
            "x": { "$from": "ghost", "path": "" },
            "y": { "$from": "ghost", "path": "" }
        })
    );
}

#[tokio::test]
async fn invalid_job_reference_resource_is_invalid_request() {
    let cases = [
        json!({ "kind": "tool", "tool": "ghost", "input": {} }),
        json!({ "kind": "worker", "worker": "ghost", "input": {} }),
        json!({ "kind": "tool", "tool": "grep", "input": { "$input": "ghost" } }),
        json!({ "kind": "tool", "tool": "grep", "input": { "$from": "ghost", "path": "" } }),
        json!({ "kind": "tool", "tool": "grep", "input": {}, "needs": ["a"] }),
        json!({ "kind": "tool", "tool": "grep", "input": {}, "needs": ["unknown"] }),
        json!({ "kind": "tool", "tool": "grep", "input": {}, "extra": 1 }),
        json!({
            "kind": "tool", "tool": "grep", "input": {},
            "resources": { "reads": [{ "path": { "$from": "a", "path": "" }, "scope": "exact" }] }
        }),
    ];
    for job in cases {
        let mut raw = definition();
        raw["jobs"]["a"] = job.clone();
        let fixtures = FixtureSet::new();
        let err = fixtures.compiler().validate_definition(&raw).unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::InvalidRequest), "job: {job}");
    }
}

#[test]
fn unknown_top_level_and_output_fields_rejected() {
    for raw in [
        {
            let mut d = definition();
            d["extra"] = json!(true);
            d
        },
        {
            let mut d = definition();
            d["outputs"]["x"] = json!({ "schema": {}, "value": {}, "extra": 1 });
            d
        },
    ] {
        let fixtures = FixtureSet::new();
        let err = fixtures.compiler().validate_definition(&raw).unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::InvalidRequest));
    }
}

#[test]
fn cycle_rejected_by_definition_validation_without_invocation() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {}, "needs": ["b"] });
    raw["jobs"]["b"] = json!({ "kind": "tool", "tool": "grep", "input": {}, "needs": ["a"] });
    let fixtures = FixtureSet::new();
    let err = fixtures.compiler().validate_definition(&raw).unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::InvalidRequest));
}

#[test]
fn worker_capability_missing_tool_rejected_by_catalog() {
    let mut fixtures = FixtureSet::new();
    fixtures.worker_mut().tools = vec![cheaper_core::tools::vocabulary::ToolName::Write];
    let mut raw = definition();
    raw["jobs"]["a"] =
        json!({ "kind": "worker", "worker": "bulk-reader", "input": { "task": "x" } });
    let err = fixtures.compiler().validate_definition(&raw).unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::InvalidRequest));
}

#[tokio::test]
async fn impossible_pointer_static_rejection_open_schema_runtime() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "read", "input": { "path": "src/a" } });
    raw["outputs"]["x"] =
        json!({ "schema": {}, "value": { "$from": "a", "path": "/no-such-property" } });
    let fixtures = FixtureSet::new();
    let err = fixtures.compiler().validate_definition(&raw).unwrap_err();
    assert_eq!(code(&err), Some(ErrorCode::InvalidRequest));
    raw["outputs"]["x"]["value"] = json!({ "$from": "a", "path": "/details/not-statically-known" });
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert!(compiled.outputs.contains_key("x"));
}

// job input の最終 validation と snapshot
#[tokio::test]
async fn from_free_input_full_validation_at_compile() {
    for job in [
        json!({ "kind": "tool", "tool": "read", "input": { "path": 7 } }),
        json!({ "kind": "worker", "worker": "bulk-reader", "input": { "task": 7 } }),
    ] {
        let mut raw = definition();
        raw["jobs"]["a"] = job.clone();
        let fixtures = FixtureSet::new();
        assert_eq!(
            err_code(fixtures.compiler().compile(&raw, None).await),
            Some(ErrorCode::InvalidInput),
            "job: {job}"
        );
    }
}

#[tokio::test]
async fn deferred_from_input_validates_after_resolution() {
    for kind in ["tool", "worker"] {
        let mut raw = definition();
        raw["jobs"]["source"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
        raw["jobs"]["consumer"] = if kind == "tool" {
            json!({
                "kind": "tool", "tool": "read",
                "input": { "path": { "$from": "source", "path": "/details/value" } }
            })
        } else {
            json!({
                "kind": "worker", "worker": "bulk-reader",
                "input": { "task": { "$from": "source", "path": "/details/value" } }
            })
        };
        let fixtures = FixtureSet::new();
        let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
        let mut results = HashMap::new();
        results.insert(
            "source".to_string(),
            json!({ "content": [], "details": { "value": 7 } }),
        );
        assert_eq!(
            compiled
                .materialize_job_input("consumer", &results)
                .unwrap_err()
                .code(),
            ErrorCode::InvalidInput
        );
        results.insert(
            "source".to_string(),
            json!({ "content": [], "details": { "value": "valid" } }),
        );
        assert_eq!(
            compiled
                .materialize_job_input("consumer", &results)
                .unwrap(),
            if kind == "tool" {
                json!({ "path": "valid" })
            } else {
                json!({ "task": "valid" })
            }
        );
        assert_eq!(
            compiled
                .materialize_job_input("consumer", &HashMap::new())
                .unwrap_err()
                .code(),
            ErrorCode::InvalidInput
        );
        assert_eq!(
            compiled
                .materialize_job_input("ghost", &results)
                .unwrap_err()
                .code(),
            ErrorCode::InvalidRequest
        );
    }
}

#[tokio::test]
async fn mutation_of_definition_invocation_catalog_keeps_snapshot() {
    let mut raw = definition();
    raw["inputs"]["x"] = json!({ "type": "string" });
    raw["jobs"]["a"] = json!({
        "kind": "worker", "worker": "bulk-reader",
        "input": { "task": { "$input": "x" } }
    });
    let caller = json!({ "x": "original" });
    let mut fixtures = FixtureSet::new();
    let compiled = fixtures
        .compiler()
        .compile(&raw, Some(&caller))
        .await
        .unwrap();
    // TS: caller.x/raw.jobs.a.input/compiled.inputs.x/compiled.jobs.a.input/
    // worker.inputSchema 全てを mutate しても snapshot が不変。
    raw["jobs"]["a"]["input"] = json!({ "task": 9 });
    fixtures.worker_mut().input_schema = Some(Arc::new(
        compile_schema(Map::new(), SchemaKind::Input).unwrap(),
    ));
    assert_eq!(
        compiled
            .materialize_job_input("a", &HashMap::new())
            .unwrap(),
        json!({ "task": "original" })
    );
}

// 全 terminal 後の output hooks
#[tokio::test]
async fn output_hooks_readiness_schema_missing_partial() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["failed"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["outputs"] = json!({
        "good": { "schema": { "type": "number" }, "value": { "$from": "a", "path": "/details/value" } },
        "bad": { "schema": {}, "value": { "$from": "failed", "path": "" } },
        "constant": { "schema": { "type": "string" }, "value": { "$literal": "public" } }
    });
    let fixtures = FixtureSet::new();
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    let mut states: HashMap<String, WorkflowJobState> = HashMap::from([
        (
            "a".to_string(),
            WorkflowJobState {
                terminal: true,
                result: Some(json!({ "content": [], "details": { "value": 5 } })),
            },
        ),
        (
            "failed".to_string(),
            WorkflowJobState {
                terminal: false,
                result: None,
            },
        ),
    ]);
    // 非 terminal job があると output materialize は INVALID_OUTPUT。
    assert_eq!(
        compiled
            .materialize_output("constant", &states)
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
    states.get_mut("failed").unwrap().terminal = true;
    assert_eq!(
        compiled.materialize_output("good", &states).unwrap(),
        json!(5)
    );
    assert_eq!(
        compiled.materialize_output("constant", &states).unwrap(),
        json!("public")
    );
    // `failed` は result を持たない → `bad` の $from は missing → error。
    assert_eq!(
        compiled
            .materialize_output("bad", &states)
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
    assert_eq!(
        compiled.materialize_outputs(&states).unwrap_err().code(),
        ErrorCode::InvalidOutput
    );
    states.get_mut("failed").unwrap().result = Some(Value::Null);
    assert_eq!(
        compiled.materialize_outputs(&states).unwrap(),
        serde_json::from_str::<Map<String, Value>>(r#"{"bad":null,"constant":"public","good":5}"#)
            .unwrap()
    );
    states.get_mut("a").unwrap().result =
        Some(json!({ "content": [], "details": { "value": "bad" } }));
    assert_eq!(
        compiled
            .materialize_output("good", &states)
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
    states.remove("a");
    assert_eq!(
        compiled.materialize_outputs(&states).unwrap_err().code(),
        ErrorCode::InvalidOutput
    );
}

#[tokio::test]
async fn output_schema_binding_snapshot_false_result_not_missing() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["outputs"]["x"] =
        json!({ "schema": { "type": "boolean" }, "value": { "$from": "a", "path": "" } });
    let fixtures = FixtureSet::new();
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    let states = HashMap::from([(
        "a".to_string(),
        WorkflowJobState {
            terminal: true,
            result: Some(json!(false)),
        },
    )]);
    assert_eq!(
        compiled.materialize_output("x", &states).unwrap(),
        json!(false)
    );
}

#[tokio::test]
async fn missing_pointer_is_output_error_empty_outputs_need_all_terminal() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    let fixtures = FixtureSet::new();
    let empty = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert_eq!(
        empty
            .materialize_outputs(&HashMap::new())
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
    assert_eq!(
        empty
            .materialize_outputs(&HashMap::from([(
                "a".to_string(),
                WorkflowJobState {
                    terminal: true,
                    result: None
                }
            )]))
            .unwrap(),
        Map::new()
    );
    raw["outputs"]["x"] =
        json!({ "schema": {}, "value": { "$from": "a", "path": "/details/missing" } });
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert_eq!(
        compiled
            .materialize_output(
                "x",
                &HashMap::from([(
                    "a".to_string(),
                    WorkflowJobState {
                        terminal: true,
                        result: Some(json!({ "content": [], "details": {} }))
                    }
                )])
            )
            .unwrap_err()
            .code(),
        ErrorCode::InvalidOutput
    );
}

// resource 推論 port は未実装を成功にしない
#[tokio::test]
async fn omitted_resources_no_manifest_empty_array_needs_port() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["b"] = json!({ "kind": "tool", "tool": "grep", "input": {}, "resources": {} });
    let fixtures = FixtureSet::new();
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert!(job(&compiled, "a").resources.is_none());
    assert!(job(&compiled, "b").resources.is_some());
    assert!(job(&compiled, "a").manifest.is_none());
    raw["jobs"]["b"]["resources"] = json!({ "reads": [] });
    assert_eq!(
        err_code(fixtures.compiler().compile(&raw, None).await),
        Some(ErrorCode::InvalidRequest)
    );
}

struct FnInference<F>(F);

/// closure の return は `Pin<Box<async block>>` から dyn へ自動 coerce
/// されないため、ここで `Pin<Box<dyn Future + 'a>>` に固定する。
fn infer_future<'a, Fut>(f: Fut) -> Pin<Box<dyn Future<Output = Fut::Output> + Send + 'a>>
where
    Fut: Future + Send + 'a,
{
    Box::pin(f)
}

impl<F> ResourceInference for FnInference<F>
where
    // closure は `for<'a> Fn(&'a T) -> Pin<Box<dyn Future + 'a>>` の
    // HRTB を満たせない(return が入力 lifetime に依存する型を
    // closure で表現できない)ため、port の future は 'static に限定し、
    // `jobs` の借用は closure の sync 部で閉じる。
    F: Fn(
            &[(String, ResourceInferenceJob)],
        )
            -> Pin<Box<dyn Future<Output = Result<ResourceInferenceResult, ToolError>> + Send>>
        + Send
        + Sync,
{
    fn infer<'a>(
        &'a self,
        jobs: &'a [(String, ResourceInferenceJob)],
    ) -> Pin<Box<dyn Future<Output = Result<ResourceInferenceResult, ToolError>> + Send + 'a>> {
        (self.0)(jobs)
    }
}

fn empty_manifest() -> cheaper_core::resources::types::ResourceManifest {
    Default::default()
}

#[tokio::test]
async fn async_port_receives_materialized_paths_and_worker_metadata() {
    let mut raw = definition();
    raw["inputs"]["path"] = json!({ "type": "string", "default": "src" });
    raw["jobs"]["a"] = json!({
        "kind": "tool", "tool": "grep", "input": {},
        "resources": { "produces": [{ "path": { "$input": "path" }, "scope": "subtree" }] }
    });
    raw["jobs"]["b"] = json!({
        "kind": "worker", "worker": "bulk-reader", "input": { "task": "inspect" },
        "resources": { "reads": [] }
    });
    let manifests = vec![
        ("a".to_string(), empty_manifest()),
        ("b".to_string(), empty_manifest()),
    ];
    let seen = Arc::new(std::sync::Mutex::new(false));
    let seen2 = Arc::clone(&seen);
    let port = FnInference(move |jobs: &[(String, ResourceInferenceJob)]| {
        let manifests = manifests.clone();
        {
            let mut guard = seen2.lock().unwrap();
            *guard = true;
        }
        // `jobs.a.job.resources.produces[0].path` は materialize 済み
        // `"src"`。`jobs` の借用は sync 部で閉じ、返す future は
        // 'static に保つ(closure の HRTB は 'static future を要求)。
        let a = jobs
            .iter()
            .find(|(id, _)| id == "a")
            .map(|(_, j)| j)
            .unwrap();
        let produces = a
            .job
            .resources
            .as_ref()
            .and_then(|r| r.produces.as_ref())
            .unwrap();
        assert!(matches!(
            &produces[0].path,
            cheaper_workflow::ScopePath::Literal(p) if p == "src"
        ));
        let b = jobs
            .iter()
            .find(|(id, _)| id == "b")
            .map(|(_, j)| j)
            .unwrap();
        assert_eq!(b.worker.as_ref().unwrap().name, "bulk-reader");
        infer_future(async move {
            Ok(ResourceInferenceResult {
                manifests,
                edges: vec![DependencyEdge {
                    from: "a".into(),
                    to: "b".into(),
                }],
                conflicts: None,
                diagnostics: None,
            })
        })
    });
    let fixtures = FixtureSet::new().with_resources(&port);
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert!(*seen.lock().unwrap());
    assert_eq!(
        compiled.edges,
        vec![DependencyEdge {
            from: "a".into(),
            to: "b".into()
        }]
    );
    assert_eq!(job(&compiled, "a").manifest, Some(empty_manifest()));
    assert_eq!(job(&compiled, "b").manifest, Some(empty_manifest()));
}

#[tokio::test]
async fn non_string_resource_path_is_invalid_input() {
    let mut raw = definition();
    raw["inputs"]["path"] = json!({ "type": "number", "default": 7 });
    raw["jobs"]["a"] = json!({
        "kind": "tool", "tool": "grep", "input": {},
        "resources": { "reads": [{ "path": { "$input": "path" }, "scope": "exact" }] }
    });
    let fixtures = FixtureSet::new();
    assert_eq!(
        err_code(fixtures.compiler().compile(&raw, None).await),
        Some(ErrorCode::InvalidInput)
    );
}

#[tokio::test]
async fn missing_manifest_port_throw_bad_edges_rejected() {
    let mut raw = definition();
    raw["jobs"]["a"] =
        json!({ "kind": "tool", "tool": "grep", "input": {}, "resources": { "reads": [] } });
    raw["jobs"]["b"] = json!({ "kind": "tool", "tool": "grep", "input": {}, "needs": ["a"] });

    // manifest 欠落。
    {
        let port = FnInference(|_jobs: &[(String, ResourceInferenceJob)]| {
            infer_future(async {
                Ok(ResourceInferenceResult {
                    manifests: vec![("a".to_string(), empty_manifest())],
                    edges: vec![],
                    conflicts: None,
                    diagnostics: None,
                })
            })
        });
        let fixtures = FixtureSet::new().with_resources(&port);
        assert_eq!(
            err_code(fixtures.compiler().compile(&raw, None).await),
            Some(ErrorCode::InvalidRequest)
        );
    }
    // port throw はそのまま伝播。
    {
        let port = FnInference(|_jobs: &[(String, ResourceInferenceJob)]| {
            infer_future(async {
                Err(Box::new(std::io::Error::other("normalization failed")) as ToolError)
            })
        });
        let fixtures = FixtureSet::new().with_resources(&port);
        let err = fixtures.compiler().compile(&raw, None).await.err().unwrap();
        assert!(err.to_string().contains("normalization failed"));
    }
    // self/unknown/cycle edge。
    for edges in [
        vec![DependencyEdge {
            from: "a".into(),
            to: "a".into(),
        }],
        vec![DependencyEdge {
            from: "ghost".into(),
            to: "b".into(),
        }],
        vec![DependencyEdge {
            from: "b".into(),
            to: "a".into(),
        }],
    ] {
        let port = FnInference(move |_jobs: &[(String, ResourceInferenceJob)]| {
            let edges = edges.clone();
            infer_future(async move {
                Ok(ResourceInferenceResult {
                    manifests: vec![
                        ("a".to_string(), empty_manifest()),
                        ("b".to_string(), empty_manifest()),
                    ],
                    edges,
                    conflicts: None,
                    diagnostics: None,
                })
            })
        });
        let fixtures = FixtureSet::new().with_resources(&port);
        assert_eq!(
            err_code(fixtures.compiler().compile(&raw, None).await),
            Some(ErrorCode::InvalidRequest),
            "bad edges rejected"
        );
    }
}

// compact fixture — bulk-reader.input.schema.json に適合する compact 経路
// (crates/cheaper-cli/resources/workers/schemas/bulk-reader.input.schema.json
// は workspace file を read する—Rust 側では repo root から相対で開く)。
#[tokio::test]
async fn compact_fixture_matches_real_bulk_reader_schema() {
    let schema_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../cheaper-cli/resources/workers/schemas/bulk-reader.input.schema.json"
    );
    let text = match std::fs::read_to_string(schema_path) {
        Ok(t) => t,
        // schema file が無い環境では skip(TS 側は存在前提だが crate 単体の
        // 検証可能性を保つ)。
        Err(_) => return,
    };
    let schema: Value = serde_json::from_str(&text).unwrap();
    let mut fixtures = FixtureSet::new();
    fixtures.worker_mut().input_schema = Some(Arc::new(
        compile_schema(schema.as_object().unwrap().clone(), SchemaKind::Input).unwrap(),
    ));
    let mut raw = definition();
    raw["inputs"]["path"] = json!({ "type": "string", "default": "src/index.ts" });
    raw["jobs"] = json!({
        "read": { "kind": "tool", "tool": "read", "input": { "path": { "$input": "path" } } },
        "analyze": {
            "kind": "worker", "worker": "bulk-reader",
            "input": { "task": { "$from": "read", "path": "/content/0/text" } }
        }
    });
    raw["outputs"]["summary"] = json!({ "schema": { "type": "string" }, "value": { "$from": "analyze", "path": "/summary" } });
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    let results = HashMap::from([(
        "read".to_string(),
        json!({ "content": [{ "type": "text", "text": "Explain the source" }] }),
    )]);
    assert_eq!(
        compiled.materialize_job_input("analyze", &results).unwrap(),
        json!({ "task": "Explain the source" })
    );
    let states = HashMap::from([
        (
            "read".to_string(),
            WorkflowJobState {
                terminal: true,
                result: None,
            },
        ),
        (
            "analyze".to_string(),
            WorkflowJobState {
                terminal: true,
                result: Some(json!({ "summary": "public" })),
            },
        ),
    ]);
    assert_eq!(
        compiled.materialize_output("summary", &states).unwrap(),
        json!("public")
    );
}

// canonical resource inference compiler integration
#[tokio::test]
async fn full_manifests_on_injected_port_cloned_annotations() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["b"] = json!({ "kind": "tool", "tool": "grep", "input": {}, "resources": {} });
    let claim = cheaper_core::resources::types::ResourceClaim {
        resource: cheaper_core::resources::types::Resource::workspace(
            "/canonical/state",
            cheaper_core::resources::types::ResourceScope::Exact,
        ),
        mode: cheaper_core::resources::types::ClaimMode::Exclusive,
    };
    let manifests = vec![
        (
            "a".to_string(),
            cheaper_core::resources::types::ResourceManifest {
                reads: vec![],
                writes: vec![claim.clone()],
                produces: vec![],
            },
        ),
        (
            "b".to_string(),
            cheaper_core::resources::types::ResourceManifest {
                reads: vec![],
                writes: vec![claim.clone()],
                produces: vec![],
            },
        ),
    ];
    let conflicts = vec![cheaper_workflow::ResourceConflict {
        jobs: ["a".into(), "b".into()],
        claims: [claim.clone(), claim.clone()],
    }];
    let diagnostics = vec![cheaper_workflow::ResourceInferenceDiagnostic::Fallback {
        job: "a".into(),
        reason: "unknown-access".into(),
    }];
    let calls = Arc::new(std::sync::Mutex::new(0u32));
    let calls2 = Arc::clone(&calls);
    let port = FnInference(move |jobs: &[(String, ResourceInferenceJob)]| {
        *calls2.lock().unwrap() += 1;
        assert_eq!(
            jobs.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        let manifests = manifests.clone();
        let conflicts = conflicts.clone();
        let diagnostics = diagnostics.clone();
        infer_future(async move {
            Ok(ResourceInferenceResult {
                manifests,
                edges: vec![],
                conflicts: Some(conflicts),
                diagnostics: Some(diagnostics),
            })
        })
    });
    let fixtures = FixtureSet::new().with_resources(&port);
    let compiled = fixtures.compiler().compile(&raw, None).await.unwrap();
    assert_eq!(*calls.lock().unwrap(), 1);
    assert_eq!(
        job(&compiled, "a").manifest.as_ref().unwrap().writes.len(),
        1
    );
    assert_eq!(compiled.conflicts.as_ref().unwrap().len(), 1);
    assert_eq!(
        compiled.diagnostics.as_ref().unwrap()[0],
        cheaper_workflow::ResourceInferenceDiagnostic::Fallback {
            job: "a".into(),
            reason: "unknown-access".into()
        }
    );
}

#[tokio::test]
async fn all_omitted_missing_manifest_and_port_failure() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    {
        let port = FnInference(|_jobs: &[(String, ResourceInferenceJob)]| {
            infer_future(async {
                Ok(ResourceInferenceResult {
                    manifests: vec![],
                    edges: vec![],
                    conflicts: None,
                    diagnostics: None,
                })
            })
        });
        let fixtures = FixtureSet::new().with_resources(&port);
        assert_eq!(
            err_code(fixtures.compiler().compile(&raw, None).await),
            Some(ErrorCode::InvalidRequest)
        );
    }
    {
        let port = FnInference(|_jobs: &[(String, ResourceInferenceJob)]| {
            infer_future(async {
                Err(Box::new(std::io::Error::other("inference failed")) as ToolError)
            })
        });
        let fixtures = FixtureSet::new().with_resources(&port);
        let err = fixtures.compiler().compile(&raw, None).await.err().unwrap();
        assert!(err.to_string().contains("inference failed"));
    }
}
