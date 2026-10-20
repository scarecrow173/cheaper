//! Port of `packages/workflow/test/engine.test.ts`。
//!
//! fake timer 系は `#[tokio::test(start_paused = true)]` +
//! `common::advance` で再現する。`vi.advanceTimersByTime` を abort
//! callback 内で呼ぶケースは Rust の paused clock が sync callback から
//! 進められないため、「first abort 起点の grace 境界」を同等に検証する
//! 形に置き換えている(分割理由は IR doc に記載)。

mod common;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::resources::types::{ClaimMode, Resource, ResourceManifest, ResourceScope};
use cheaper_tools::execution_metrics::{ExecutionMetrics, OperationStateCounts};
use cheaper_tools::operation::{AbortController, AbortReason};
use cheaper_tools::resources::{prepared, LeaseKind, LeasePreparation, ResourceRequest};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

use cheaper_workflow::{
    run_workflow, JobExecutor, JobLeaseRequest, JobStatus, WorkflowEnginePorts,
    WorkflowExecutionContext, WorkflowExecutorError, WorkflowExecutorResult, WorkflowRunOptions,
};

use common::{
    advance, definition, flush, harness, independent, tool_result, Deferred, FixtureSet, FnExecutor,
};

/// deferred を await する executor。drop は reject(late settle)相当。
fn gated(d: &Arc<Deferred<WorkflowExecutorResult>>) -> Arc<dyn JobExecutor> {
    let d = Arc::clone(d);
    Arc::new(FnExecutor(move |_ctx: WorkflowExecutionContext| {
        let d = Arc::clone(&d);
        async move { d.wait().await }
    }))
}

fn options<'a>(
    workflow: &'a cheaper_workflow::CompiledWorkflow,
    ports: WorkflowEnginePorts,
) -> WorkflowRunOptions<'a> {
    WorkflowRunOptions {
        workflow,
        ports,
        max_jobs: 1.0,
        timeout_ms: 10_000.0,
        signal: None,
        abort_grace_ms: None,
        now: None,
    }
}

fn outcome_serialized(outcome: &cheaper_workflow::WorkflowOutcome) -> String {
    serde_json::to_string(outcome).unwrap()
}

// ---- one-shot scheduling and public projection ----

#[tokio::test]
async fn invalid_timeout_rejected_before_admission() {
    for timeout_ms in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        let workflow = independent(&["a"]).await;
        let h = harness(
            Arc::new(FnExecutor(|_: WorkflowExecutionContext| async {
                Ok(WorkflowExecutorResult {
                    result: json!("worker"),
                    meta: None,
                })
            })),
            Arc::new(FnExecutor(|ctx: WorkflowExecutionContext| async move {
                Ok(tool_result(&ctx.job_id))
            })),
        );
        let mut o = options(&workflow, h.ports.clone());
        o.timeout_ms = timeout_ms;
        let err = run_workflow(o).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRequest);
        assert!(h.acquire_calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn invalid_grace_rejected_before_admission() {
    for abort_grace_ms in [-1.0, f64::INFINITY, f64::NAN] {
        let workflow = independent(&["a"]).await;
        let h = common::default_harness();
        let mut o = options(&workflow, h.ports.clone());
        o.timeout_ms = 100.0;
        o.abort_grace_ms = Some(abort_grace_ms);
        let err = run_workflow(o).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRequest);
        assert!(h.acquire_calls.lock().unwrap().is_empty());
    }
}

#[tokio::test(start_paused = true)]
async fn empty_workflow_validates_outputs_and_cleans_timers() {
    let mut raw = definition();
    raw["outputs"]["ok"] =
        json!({ "schema": { "type": "boolean" }, "value": { "$literal": true } });
    let fixtures = FixtureSet::new();
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
    let h = common::default_harness();
    let mut o = options(&workflow, h.ports);
    o.timeout_ms = 100.0;
    let outcome = run_workflow(o).await.unwrap();
    assert!(outcome.ok);
    assert_eq!(outcome.result["ok"], json!(true));
    assert!(outcome.jobs.is_empty());
    assert!(h.acquire_calls.lock().unwrap().is_empty());
    // timer は cleanup 済み — 大量に進めても何も起きない。
    advance(60_000).await;
}

#[tokio::test]
async fn inferred_edges_and_from_follow_compiled_edges() {
    struct Injected;
    impl cheaper_workflow::ResourceInference for Injected {
        fn infer<'a>(
            &'a self,
            jobs: &'a [(String, cheaper_workflow::ResourceInferenceJob)],
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<cheaper_workflow::ResourceInferenceResult, ToolError>,
                    > + Send
                    + 'a,
            >,
        > {
            let manifests = jobs
                .iter()
                .map(|(id, _)| (id.clone(), Default::default()))
                .collect();
            Box::pin(async move {
                Ok(cheaper_workflow::ResourceInferenceResult {
                    manifests,
                    edges: vec![cheaper_workflow::DependencyEdge {
                        from: "a".into(),
                        to: "b".into(),
                    }],
                    conflicts: None,
                    diagnostics: None,
                })
            })
        }
    }
    let port = Injected;
    let fixtures = FixtureSet::new().with_resources(&port);
    let mut raw = definition();
    raw["jobs"]["a"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["b"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["c"] = json!({
        "kind": "tool", "tool": "grep",
        "input": { "value": { "$from": "b", "path": "/content/0/text" } }
    });
    raw["outputs"]["ok"] =
        json!({ "schema": {}, "value": { "$from": "c", "path": "/content/0/text" } });
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();

    let a = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let starts = Arc::new(Mutex::new(Vec::<String>::new()));
    let starts2 = Arc::clone(&starts);
    let a2 = Arc::clone(&a);
    let tool = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
        let starts = Arc::clone(&starts2);
        let a = Arc::clone(&a2);
        async move {
            starts.lock().unwrap().push(ctx.job_id.clone());
            if ctx.job_id == "a" {
                return a.wait().await;
            }
            if ctx.job_id == "c" {
                assert_eq!(ctx.input, json!({ "value": "b" }));
            }
            Ok(tool_result(&ctx.job_id))
        }
    }));
    let h = harness(
        Arc::new(FnExecutor(|_: WorkflowExecutionContext| async {
            Ok(WorkflowExecutorResult {
                result: json!("worker"),
                meta: None,
            })
        })),
        tool,
    );
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        async move {
            let mut o = options(&workflow, ports);
            o.max_jobs = 3.0;
            run_workflow(o).await
        }
    });
    flush().await;
    assert_eq!(*starts.lock().unwrap(), vec!["a"]);
    a.resolve(tool_result("a"));
    let outcome = done.await.unwrap().unwrap();
    assert_eq!(outcome.result["ok"], json!("c"));
    assert_eq!(*starts.lock().unwrap(), vec!["a", "b", "c"]);
}

#[tokio::test]
async fn failure_keeps_adapter_selected_metadata_no_private_payload() {
    let workflow = independent(&["a"]).await;
    let tool = Arc::new(FnExecutor(|_: WorkflowExecutionContext| async {
        let mut meta = Map::new();
        meta.insert("model".to_string(), json!("selected"));
        meta.insert("usage".to_string(), json!({ "input": 10 }));
        meta.insert("durationMs".to_string(), json!(9000));
        Err(Box::new(
            WorkflowExecutorError::new(ErrorCode::ToolExecutionFailure, "failed", meta)
                .unwrap()
                .with_details({
                    let mut d = Map::new();
                    d.insert("private".to_string(), json!("PRIVATE DETAILS"));
                    d
                })
                .with_cause(std::io::Error::other("PRIVATE CAUSE")),
        ) as ToolError)
    }));
    let h = harness(
        Arc::new(FnExecutor(|_: WorkflowExecutionContext| async {
            Ok(WorkflowExecutorResult {
                result: json!("worker"),
                meta: None,
            })
        })),
        tool,
    );
    let mut o = options(&workflow, h.ports);
    o.now = Some(Arc::new(|| 1.0));
    let outcome = run_workflow(o).await.unwrap();
    let meta = serde_json::to_value(&outcome.jobs[0].meta).unwrap();
    assert_eq!(meta["model"], json!("selected"));
    assert_eq!(meta["usage"], json!({ "input": 10 }));
    assert_eq!(meta["durationMs"], json!(0));
    assert_eq!(meta["execution"], Value::Null);
    let serialized = outcome_serialized(&outcome);
    assert!(!serialized.contains("PRIVATE DETAILS"));
    assert!(!serialized.contains("PRIVATE CAUSE"));
}

#[tokio::test]
async fn owner_metrics_snapshot_overrides_meta_duration_and_execution() {
    let workflow = independent(&["a", "b"]).await;
    let metric = |operations: u64| ExecutionMetrics {
        operations,
        by_state: OperationStateCounts {
            completed: operations,
            ..Default::default()
        },
        by_tool: Default::default(),
        max_concurrency: 1,
        operation_ms: 4.0,
        active_ms: 3.0,
        overlap_ms: 1.0,
        queue_ms: 0.0,
    };
    let mut h = common::default_harness();
    let global = metric(7);
    h.ports.execution_metrics = Arc::new(move |id| {
        Some(match id {
            None => global.clone(),
            Some("a") => metric(2),
            Some(_) => metric(3),
        })
    });
    h.ports.tool = Arc::new(FnExecutor(|ctx: WorkflowExecutionContext| async move {
        Ok(WorkflowExecutorResult {
            result: json!({ "content": [{ "type": "text", "text": ctx.job_id }] }),
            meta: Some({
                let mut m = Map::new();
                m.insert("durationMs".to_string(), json!(9000));
                m.insert("execution".to_string(), json!({ "forged": true }));
                m.insert("turns".to_string(), json!(1));
                m
            }),
        })
    }));
    let mut o = options(&workflow, h.ports);
    o.max_jobs = 2.0;
    o.now = Some(Arc::new(|| 10.0));
    let outcome = run_workflow(o).await.unwrap();
    assert_eq!(outcome.meta.execution.as_ref().unwrap().operations, 7);
    assert_eq!(
        outcome
            .jobs
            .iter()
            .map(|j| j.meta.execution.as_ref().unwrap().operations)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    let meta0 = serde_json::to_value(&outcome.jobs[0].meta).unwrap();
    assert_eq!(meta0["durationMs"], json!(0));
    assert_eq!(meta0["turns"], json!(1));
    // forged executor "execution" は owner snapshot で置き換わる。
    assert_eq!(meta0["execution"]["operations"], json!(2));
}

#[tokio::test]
async fn invalid_max_jobs_rejected_before_execution() {
    for max_jobs in [0.0, -1.0, 1.5, f64::INFINITY, 9_007_199_254_740_992.0] {
        let workflow = independent(&["a"]).await;
        let h = common::default_harness();
        let mut o = options(&workflow, h.ports.clone());
        o.max_jobs = max_jobs;
        let err = run_workflow(o).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidRequest);
        assert!(h.acquire_calls.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn admission_wait_consumes_no_slot_and_keeps_semantic_edges() {
    let mut workflow = independent(&["a", "b", "c"]).await;
    let shared = std::env::current_dir()
        .unwrap()
        .join("resource")
        .to_string_lossy()
        .to_string();
    let claim = cheaper_core::resources::types::ResourceClaim {
        resource: Resource::workspace(shared.clone(), ResourceScope::Exact),
        mode: ClaimMode::Exclusive,
    };
    {
        let wf = Arc::get_mut(&mut workflow).unwrap();
        for id in ["a", "b"] {
            wf.job_mut(id).unwrap().manifest = Some(ResourceManifest {
                reads: vec![],
                writes: vec![claim.clone()],
                produces: vec![],
            });
        }
    }
    let a = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let starts = Arc::new(Mutex::new(Vec::<String>::new()));
    let starts2 = Arc::clone(&starts);
    let a2 = Arc::clone(&a);
    let tool = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
        let starts = Arc::clone(&starts2);
        let a = Arc::clone(&a2);
        async move {
            starts.lock().unwrap().push(ctx.job_id.clone());
            if ctx.job_id == "a" {
                a.wait().await
            } else {
                Ok(tool_result(&ctx.job_id))
            }
        }
    }));
    let mut h = common::default_harness();
    h.ports.tool = tool;
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        async move {
            let mut o = options(&workflow, ports);
            o.max_jobs = 2.0;
            run_workflow(o).await
        }
    });
    flush().await;
    assert_eq!(*starts.lock().unwrap(), vec!["a", "c"]);
    assert!(workflow.edges.is_empty());
    a.resolve(tool_result("a"));
    let outcome = done.await.unwrap().unwrap();
    assert!(outcome.ok);
    assert_eq!(*starts.lock().unwrap(), vec!["a", "c", "b"]);
}

#[tokio::test]
async fn output_only_schema_failure_keeps_independent_output() {
    let mut fixtures = FixtureSet::new();
    fixtures.worker_mut().output_schema = Arc::new(
        cheaper_core::worker::schema::compile_schema(
            Map::new(),
            cheaper_core::worker::schema::SchemaKind::Output,
        )
        .unwrap(),
    );
    let mut raw = definition();
    raw["jobs"]["a"] = json!({
        "kind": "worker", "worker": "bulk-reader", "input": { "task": "x" }
    });
    raw["outputs"]["bad"] =
        json!({ "schema": { "type": "number" }, "value": { "$from": "a", "path": "" } });
    raw["outputs"]["good"] = json!({ "schema": {}, "value": { "$literal": "kept" } });
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
    let h = common::default_harness();
    let outcome = run_workflow(options(&workflow, h.ports)).await.unwrap();
    assert!(!outcome.ok);
    assert_eq!(outcome.result["good"], json!("kept"));
    assert_eq!(
        outcome.error.as_ref().unwrap().code,
        ErrorCode::InvalidOutput
    );
    assert_eq!(
        outcome
            .meta
            .output_errors
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        vec!["bad"]
    );
    assert_eq!(outcome.jobs[0].status, JobStatus::Succeeded);
}

#[tokio::test]
async fn diagnostics_utf8_codepoint_safe_cap_no_private() {
    let workflow = independent(&["a"]).await;
    let tool = Arc::new(FnExecutor(|_: WorkflowExecutionContext| async {
        Err(Box::new(
            CheaperError::new(ErrorCode::PartialMutation, "あ😀".repeat(1000))
                .with_details({
                    let mut d = Map::new();
                    d.insert("private".to_string(), json!("PRIVATE DETAILS"));
                    d
                })
                .with_cause(std::io::Error::other("PRIVATE CAUSE")),
        ) as ToolError)
    }));
    let h = harness(
        Arc::new(FnExecutor(|_: WorkflowExecutionContext| async {
            Ok(WorkflowExecutorResult {
                result: json!("worker"),
                meta: None,
            })
        })),
        tool,
    );
    let outcome = run_workflow(options(&workflow, h.ports)).await.unwrap();
    let message = outcome.jobs[0].error.as_ref().unwrap().message.clone();
    assert!(message.len() <= 2048);
    assert!(!message.contains('\u{FFFD}'));
    assert_eq!(
        outcome.jobs[0].error.as_ref().unwrap().code,
        "PARTIAL_MUTATION"
    );
    let serialized = outcome_serialized(&outcome);
    assert!(!serialized.contains("PRIVATE DETAILS"));
    assert!(!serialized.contains("PRIVATE CAUSE"));
}

#[tokio::test]
async fn mixed_fan_out_fan_in_waits_for_producers() {
    let mut raw = definition();
    raw["jobs"]["a"] = json!({
        "kind": "worker", "worker": "bulk-reader", "input": { "task": "PRIVATE INPUT" }
    });
    raw["jobs"]["b"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["c"] = json!({
        "kind": "tool", "tool": "grep", "needs": ["a", "b"],
        "input": { "value": { "$from": "a", "path": "/payload" } }
    });
    raw["outputs"]["result"] =
        json!({ "schema": {}, "value": { "$from": "c", "path": "/content/0/text" } });
    let fixtures = FixtureSet::new();
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();

    let a = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let b = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let starts = Arc::new(Mutex::new(Vec::<String>::new()));
    let a2 = Arc::clone(&a);
    let starts2 = Arc::clone(&starts);
    let worker = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
        let starts = Arc::clone(&starts2);
        let a = Arc::clone(&a2);
        async move {
            starts.lock().unwrap().push(ctx.job_id.clone());
            a.wait().await
        }
    }));
    let b2 = Arc::clone(&b);
    let starts3 = Arc::clone(&starts);
    let tool = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
        let starts = Arc::clone(&starts3);
        let b = Arc::clone(&b2);
        async move {
            starts.lock().unwrap().push(ctx.job_id.clone());
            if ctx.job_id == "b" {
                return b.wait().await;
            }
            assert_eq!(ctx.input, json!({ "value": "PRIVATE RESULT" }));
            Ok(tool_result("public"))
        }
    }));
    let h = harness(worker, tool);
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        async move {
            let mut o = options(&workflow, ports);
            o.max_jobs = 2.0;
            run_workflow(o).await
        }
    });
    flush().await;
    assert_eq!(*starts.lock().unwrap(), vec!["a", "b"]);
    // producer result は materialize 時点で snapshot される(後 mutation
    // は反映しない)。
    a.resolve(WorkflowExecutorResult {
        result: json!({ "payload": "PRIVATE RESULT" }),
        meta: Some({
            let mut m = Map::new();
            m.insert("model".to_string(), json!("test"));
            m
        }),
    });
    flush().await;
    assert_eq!(*starts.lock().unwrap(), vec!["a", "b"]);
    b.resolve(tool_result("private b"));
    let outcome = done.await.unwrap().unwrap();
    assert_eq!(*starts.lock().unwrap(), vec!["a", "b", "c"]);
    assert!(outcome.ok);
    assert_eq!(outcome.result["result"], json!("public"));
    assert_eq!(
        outcome
            .jobs
            .iter()
            .map(|j| j.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    let serialized = outcome_serialized(&outcome);
    assert!(!serialized.contains("PRIVATE INPUT"));
    assert!(!serialized.contains("PRIVATE RESULT"));
    assert!(!serialized.contains("private b"));
    assert_eq!(outcome.jobs[0].meta.extra["model"], json!("test"));
    assert_eq!(h.release_calls.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn max_jobs_1_serializes_and_integer_like_ids_are_code_unit_order() {
    let workflow = independent(&["2", "10", "a"]).await;
    let holds: Arc<Mutex<HashMap<String, Arc<Deferred<WorkflowExecutorResult>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let starts = Arc::new(Mutex::new(Vec::<String>::new()));
    let holds2 = Arc::clone(&holds);
    let starts2 = Arc::clone(&starts);
    let tool = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
        let holds = Arc::clone(&holds2);
        let starts = Arc::clone(&starts2);
        async move {
            starts.lock().unwrap().push(ctx.job_id.clone());
            let d = Arc::new(Deferred::new());
            holds
                .lock()
                .unwrap()
                .insert(ctx.job_id.clone(), Arc::clone(&d));
            d.wait().await
        }
    }));
    let mut h = common::default_harness();
    h.ports.tool = tool;
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        async move {
            run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 1.0,
                timeout_ms: 10_000.0,
                signal: None,
                abort_grace_ms: None,
                now: None,
            })
            .await
        }
    });
    flush().await;
    assert_eq!(*starts.lock().unwrap(), vec!["10"]);
    holds.lock().unwrap()["10"].resolve(tool_result("x"));
    flush().await;
    assert_eq!(*starts.lock().unwrap(), vec!["10", "2"]);
    holds.lock().unwrap()["2"].resolve(tool_result("x"));
    flush().await;
    assert_eq!(*starts.lock().unwrap(), vec!["10", "2", "a"]);
    holds.lock().unwrap()["a"].resolve(tool_result("x"));
    let outcome = done.await.unwrap().unwrap();
    assert_eq!(
        outcome
            .jobs
            .iter()
            .map(|j| j.id.as_str())
            .collect::<Vec<_>>(),
        vec!["10", "2", "a"]
    );
}

#[tokio::test]
async fn boundary_errors_skip_dependents_and_keep_independent_outputs() {
    for boundary in ["input", "admission", "executor", "output"] {
        let mut raw = definition();
        // `input` boundary: `a` の materialize が runtime で失敗するよう
        // worker(open output schema)の存在しない property を参照させる。
        raw["jobs"]["src"] =
            json!({ "kind": "worker", "worker": "bulk-reader", "input": { "task": "x" } });
        raw["jobs"]["a"] = if boundary == "input" {
            json!({
                "kind": "tool", "tool": "grep", "needs": ["src"],
                "input": { "x": { "$from": "src", "path": "/absent" } }
            })
        } else {
            json!({ "kind": "tool", "tool": "grep", "input": {} })
        };
        raw["jobs"]["b"] = json!({ "kind": "tool", "tool": "grep", "needs": ["a"], "input": {} });
        raw["jobs"]["c"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
        raw["outputs"]["bad"] = json!({ "schema": {}, "value": { "$from": "a", "path": "" } });
        raw["outputs"]["good"] =
            json!({ "schema": {}, "value": { "$from": "c", "path": "/content/0/text" } });
        let fixtures = FixtureSet::new();
        let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
        let mut h = common::default_harness();
        let error: ToolError = Box::new(
            CheaperError::new(ErrorCode::InvalidInput, "boundary failure").with_details({
                let mut d = Map::new();
                d.insert("hidden".to_string(), json!("PRIVATE DETAILS"));
                d
            }),
        );
        if boundary == "admission" {
            let acquire = h.ports.acquire_job_lease.clone();
            let error = error_clone(&error);
            h.ports.acquire_job_lease = Arc::new(move |request: JobLeaseRequest| {
                let acquire = Arc::clone(&acquire);
                let error = error_clone(&error);
                Box::pin(async move {
                    if request.job_id == "a" {
                        return Err(error);
                    }
                    acquire(request).await
                })
            });
        }
        {
            let error = error_clone(&error);
            h.ports.tool = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
                let error = error_clone(&error);
                async move {
                    if ctx.job_id == "a" {
                        if boundary == "output" {
                            return Ok(WorkflowExecutorResult {
                                result: json!(42),
                                meta: None,
                            });
                        }
                        return Err(error);
                    }
                    Ok(tool_result("kept"))
                }
            }));
        }
        let mut o = options(&workflow, h.ports);
        o.max_jobs = 2.0;
        let outcome = run_workflow(o).await.unwrap();
        assert!(!outcome.ok, "boundary: {boundary}");
        assert_eq!(
            outcome.result["good"],
            json!("kept"),
            "boundary: {boundary}"
        );
        assert_eq!(
            outcome.error.as_ref().unwrap().code,
            ErrorCode::ToolExecutionFailure,
            "boundary: {boundary}"
        );
        let by_id: HashMap<&str, &cheaper_workflow::WorkflowJobOutcome> =
            outcome.jobs.iter().map(|j| (j.id.as_str(), j)).collect();
        assert_eq!(by_id["a"].status, JobStatus::Failed, "boundary: {boundary}");
        assert_eq!(
            by_id["b"].status,
            JobStatus::Skipped,
            "boundary: {boundary}"
        );
        assert_eq!(
            by_id["c"].status,
            JobStatus::Succeeded,
            "boundary: {boundary}"
        );
        assert_eq!(
            by_id["b"].error.as_ref().unwrap().code,
            "DEPENDENCY_FAILED",
            "boundary: {boundary}"
        );
        assert_eq!(
            outcome
                .meta
                .output_errors
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
            vec!["bad"],
            "boundary: {boundary}"
        );
        let serialized = outcome_serialized(&outcome);
        assert!(!serialized.contains("PRIVATE DETAILS"));
        assert!(!serialized.contains("PRIVATE CAUSE"));
    }
}

// ---- job `when` condition ----

/// `when` false は CONDITION_SKIPPED で skip し、dependent への
/// `$from` は `null` を materialize、workflow error には計上しない。
#[tokio::test]
async fn when_false_skips_without_failure_and_materializes_null() {
    let mut raw = definition();
    raw["jobs"]["exec"] = json!({
        "kind": "tool", "tool": "grep",
        "input": {},
    });
    raw["jobs"]["analyze"] = json!({
        "kind": "tool", "tool": "grep",
        "when": {
            "left": { "$from": "exec", "path": "/content/0/text" },
            "op": "eq",
            "right": { "$literal": "never" },
        },
        "input": {},
    });
    raw["jobs"]["report"] = json!({
        "kind": "tool", "tool": "grep",
        "needs": ["analyze"],
        "input": { "x": { "$from": "analyze", "path": "" } },
    });
    raw["outputs"]["x"] = json!({ "schema": {}, "value": { "$from": "analyze", "path": "" } });
    let fixtures = FixtureSet::new();
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
    let mut h = common::default_harness();
    let ran = Arc::new(Mutex::new(Vec::new()));
    {
        let ran = Arc::clone(&ran);
        h.ports.tool = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
            ran.lock().unwrap().push(ctx.job_id.clone());
            async move { Ok(tool_result("out")) }
        }));
    }
    let outcome = run_workflow(options(&workflow, h.ports)).await.unwrap();
    assert!(outcome.ok, "condition skip must not fail the workflow");
    let by_id: HashMap<&str, &cheaper_workflow::WorkflowJobOutcome> =
        outcome.jobs.iter().map(|j| (j.id.as_str(), j)).collect();
    assert_eq!(by_id["analyze"].status, JobStatus::Skipped);
    assert_eq!(
        by_id["analyze"].error.as_ref().unwrap().code,
        "CONDITION_SKIPPED"
    );
    // dependent は skip を伝播せず `null` binding で実行される。
    assert_eq!(by_id["report"].status, JobStatus::Succeeded);
    assert!(ran.lock().unwrap().contains(&"report".to_string()));
    // outputs 側の `$from` も `null`。
    assert_eq!(outcome.result["x"], Value::Null);
}

/// `when` true の job は通常通り実行される(eq/`$from` 数値比較)。
#[tokio::test]
async fn when_true_runs_job() {
    let mut raw = definition();
    raw["jobs"]["exec"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["analyze"] = json!({
        "kind": "tool", "tool": "grep",
        "when": {
            "left": { "$from": "exec", "path": "/content/0/text" },
            "op": "ne",
            "right": { "$literal": "never" },
        },
        "input": {},
    });
    let fixtures = FixtureSet::new();
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
    let outcome = run_workflow(options(&workflow, common::default_harness().ports))
        .await
        .unwrap();
    assert!(outcome.ok);
    let by_id: HashMap<&str, &cheaper_workflow::WorkflowJobOutcome> =
        outcome.jobs.iter().map(|j| (j.id.as_str(), j)).collect();
    assert_eq!(by_id["analyze"].status, JobStatus::Succeeded);
}

/// `when` 内の `$from` は dep edge として推論され、producer 完了後に
/// 条件評価される(needs 宣言無しでも producer 待ちになる)。
#[tokio::test]
async fn when_from_creates_dependency_edge() {
    let mut raw = definition();
    raw["jobs"]["producer"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["gated"] = json!({
        "kind": "tool", "tool": "grep",
        "when": { "left": { "$from": "producer", "path": "/content/0/text" }, "op": "exists" },
        "input": {},
    });
    let fixtures = FixtureSet::new();
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
    // dep edge 推論 — order で producer が gated より前。
    let order = &workflow.order;
    assert!(
        order.iter().position(|id| id == "producer") < order.iter().position(|id| id == "gated"),
        "order: {order:?}"
    );
    let outcome = run_workflow(options(&workflow, common::default_harness().ports))
        .await
        .unwrap();
    assert!(outcome.ok);
    let by_id: HashMap<&str, &cheaper_workflow::WorkflowJobOutcome> =
        outcome.jobs.iter().map(|j| (j.id.as_str(), j)).collect();
    // producer の text は non-null — exists は true。
    assert_eq!(by_id["gated"].status, JobStatus::Succeeded);
}

/// `gt`/`lt` は数値比較のみ — 非数値 operand は false(=skip)に
/// 安全評価される。数値比較は通常通り真偽を返す。
#[tokio::test]
async fn when_numeric_operators_safely_evaluate() {
    for (op, left, right, expect_skipped) in [
        (
            "gt",
            json!({ "$literal": "text" }),
            json!({ "$literal": 1 }),
            true,
        ), // 非数値 → false
        (
            "gt",
            json!({ "$literal": 5 }),
            json!({ "$literal": 1 }),
            false,
        ),
        (
            "lt",
            json!({ "$literal": 5 }),
            json!({ "$literal": 1 }),
            true,
        ),
        (
            "lt",
            json!({ "$literal": null }),
            json!({ "$literal": 1 }),
            true,
        ), // null → false
    ] {
        let mut raw = definition();
        raw["jobs"]["a"] = json!({
            "kind": "tool", "tool": "grep",
            "when": { "left": left, "op": op, "right": right },
            "input": {},
        });
        let fixtures = FixtureSet::new();
        let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
        let outcome = run_workflow(options(&workflow, common::default_harness().ports))
            .await
            .unwrap();
        let by_id: HashMap<&str, &cheaper_workflow::WorkflowJobOutcome> =
            outcome.jobs.iter().map(|j| (j.id.as_str(), j)).collect();
        assert_eq!(
            by_id["a"].status,
            if expect_skipped {
                JobStatus::Skipped
            } else {
                JobStatus::Succeeded
            },
            "op {op} left {left} right {right}"
        );
        assert!(outcome.ok);
    }
}

/// `when` の binding 解決失敗(producer result 上に存在しない
/// path — schema 上は不可能と証明できない open object 配下)は
/// silent skip でなく job failure。
#[tokio::test]
async fn when_binding_failure_is_job_failure() {
    let mut raw = definition();
    raw["jobs"]["exec"] = json!({ "kind": "tool", "tool": "grep", "input": {} });
    raw["jobs"]["a"] = json!({
        "kind": "tool", "tool": "grep",
        "needs": ["exec"],
        "when": {
            "left": { "$from": "exec", "path": "/details/absent" },
            "op": "eq",
            "right": { "$literal": true }
        },
        "input": {},
    });
    let fixtures = FixtureSet::new();
    let workflow = fixtures.compiler().compile(&raw, None).await.unwrap();
    let outcome = run_workflow(options(&workflow, common::default_harness().ports))
        .await
        .unwrap();
    let by_id: HashMap<&str, &cheaper_workflow::WorkflowJobOutcome> =
        outcome.jobs.iter().map(|j| (j.id.as_str(), j)).collect();
    assert_eq!(by_id["a"].status, JobStatus::Failed);
    assert!(!outcome.ok);
}

/// CheaperError を clone して新しい ToolError を作る(cause 非保持)。
fn error_clone(e: &ToolError) -> ToolError {
    match e.downcast_ref::<CheaperError>() {
        Some(ce) => {
            let mut rebuilt = CheaperError::new(ce.code(), ce.message());
            if let Some(d) = ce.details() {
                rebuilt = rebuilt.with_details(d.clone());
            }
            Box::new(rebuilt)
        }
        None => Box::new(std::io::Error::other(e.to_string())),
    }
}

// ---- outer cancellation and physical lease lifetime ----

#[tokio::test(start_paused = true)]
async fn timeout_origin_is_engine_start_not_arm_time() {
    let workflow = independent(&["a"]).await;
    let hold = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let mut h = common::default_harness();
    h.ports.tool = gated(&hold);
    let aborts = Arc::clone(&h.abort_calls);
    // engine 開始時点から timeout が測られる。paused clock 上で
    // `run_workflow` entry → spawn → advance で観測する。
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        async move {
            run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 1.0,
                timeout_ms: 10.0,
                signal: None,
                abort_grace_ms: Some(0.0),
                now: None,
            })
            .await
        }
    });
    flush().await;
    advance(9).await;
    assert!(aborts.lock().unwrap().is_empty());
    advance(1).await;
    assert_eq!(aborts.lock().unwrap().len(), 1);
    hold.resolve(tool_result("late"));
    let outcome = done.await.unwrap().unwrap();
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Timeout);
}

#[tokio::test(start_paused = true)]
async fn grace_origin_is_first_outer_abort() {
    let workflow = independent(&["a"]).await;
    let hold = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let mut h = common::default_harness();
    h.ports.tool = gated(&hold);
    let outer = AbortController::new();
    let completed = Arc::new(AtomicUsize::new(0));
    let completed2 = Arc::clone(&completed);
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        let signal = outer.signal();
        async move {
            let outcome = run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 1.0,
                timeout_ms: 100.0,
                signal: Some(signal),
                abort_grace_ms: Some(10.0),
                now: None,
            })
            .await
            .unwrap();
            completed2.fetch_add(1, Ordering::SeqCst);
            outcome
        }
    });
    flush().await;
    outer.abort(AbortReason::msg("stop"));
    // grace は first abort 起点(0ms)から 10ms — 9ms では未完了。
    advance(9).await;
    assert_eq!(completed.load(Ordering::SeqCst), 0);
    advance(1).await;
    assert_eq!(completed.load(Ordering::SeqCst), 1);
    hold.resolve(tool_result("late"));
    let outcome = done.await.unwrap();
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Aborted);
}

#[tokio::test(start_paused = true)]
async fn default_grace_holds_conflict_admission_until_late_settle() {
    let mut workflow = independent(&["a"]).await;
    let shared = std::env::current_dir()
        .unwrap()
        .join("held")
        .to_string_lossy()
        .to_string();
    let claim = cheaper_core::resources::types::ResourceClaim {
        resource: Resource::workspace(shared, ResourceScope::Exact),
        mode: ClaimMode::Exclusive,
    };
    Arc::get_mut(&mut workflow)
        .unwrap()
        .job_mut("a")
        .unwrap()
        .manifest = Some(ResourceManifest {
        reads: vec![],
        writes: vec![claim.clone()],
        produces: vec![],
    });
    let hold = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let mut h = common::default_harness();
    h.ports.tool = gated(&hold);
    let resources = h.resources.clone();
    let completed = Arc::new(AtomicUsize::new(0));
    let completed2 = Arc::clone(&completed);
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        async move {
            let outcome = run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 1.0,
                timeout_ms: 10.0,
                signal: None,
                abort_grace_ms: None,
                now: None,
            })
            .await
            .unwrap();
            completed2.fetch_add(1, Ordering::SeqCst);
            outcome
        }
    });
    flush().await;
    advance(10).await;
    advance(4999).await;
    assert_eq!(completed.load(Ordering::SeqCst), 0);
    advance(1).await;
    flush().await;
    let outcome = done.await.unwrap();
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Timeout);
    // conflict admission は actual late settle まで lease を保持する。
    let granted = Arc::new(AtomicUsize::new(0));
    let granted2 = Arc::clone(&granted);
    let foreign = tokio::spawn({
        let resources = resources.clone();
        async move {
            resources
                .request(ResourceRequest {
                    owner_id: "foreign".to_string(),
                    kind: LeaseKind::Job,
                    signal: None,
                    parent_lease: None,
                    access: None,
                    prepare: Box::new(move || {
                        prepared(LeasePreparation {
                            claims: vec![claim],
                            read_strictness: cheaper_tools::resources::ReadStrictness::Record,
                        })
                    }),
                    can_start: None,
                    start: move |lease| {
                        granted2.fetch_add(1, Ordering::SeqCst);
                        Ok(lease)
                    },
                })
                .await
        }
    });
    flush().await;
    assert_eq!(granted.load(Ordering::SeqCst), 0);
    hold.resolve(tool_result("late"));
    let foreign_lease = foreign.await.unwrap().unwrap();
    assert_eq!(granted.load(Ordering::SeqCst), 1);
    resources.release(foreign_lease);
}

#[tokio::test(start_paused = true)]
async fn first_timeout_wins_over_later_signal_grace_zero() {
    let workflow = independent(&["a"]).await;
    let hold = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let mut h = common::default_harness();
    h.ports.tool = gated(&hold);
    let outer = AbortController::new();
    let aborts = Arc::clone(&h.abort_calls);
    let releases = Arc::clone(&h.release_calls);
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        let signal = outer.signal();
        async move {
            run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 1.0,
                timeout_ms: 10.0,
                signal: Some(signal),
                abort_grace_ms: Some(0.0),
                now: None,
            })
            .await
        }
    });
    flush().await;
    advance(10).await;
    outer.abort(AbortReason::msg("later signal"));
    let outcome = done.await.unwrap().unwrap();
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Timeout);
    assert_eq!(aborts.lock().unwrap().len(), 1);
    // late rejection — lease は detached settle で release される。
    hold.reject();
    flush().await;
    assert_eq!(releases.lock().unwrap().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn large_finite_timeout_grace_do_not_early_cancel() {
    let workflow = independent(&["a"]).await;
    let hold = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let mut h = common::default_harness();
    h.ports.tool = gated(&hold);
    let outer = AbortController::new();
    let aborts = Arc::clone(&h.abort_calls);
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        let signal = outer.signal();
        async move {
            run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 1.0,
                timeout_ms: 2_147_483_650.0,
                signal: Some(signal),
                abort_grace_ms: Some(2_147_483_650.0),
                now: None,
            })
            .await
        }
    });
    flush().await;
    advance(100).await;
    assert!(aborts.lock().unwrap().is_empty());
    outer.abort(AbortReason::aborted());
    advance(100).await;
    // grace (2147483650ms) は未到達 — outcome はまだ返らない。
    assert!(!done.is_finished());
    hold.resolve(tool_result("cancelled"));
    flush().await;
    let outcome = done.await.unwrap().unwrap();
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Aborted);
}

#[tokio::test]
async fn preabort_cancels_all_jobs_with_first_reason() {
    let workflow = independent(&["a", "b"]).await;
    let h = common::default_harness();
    let outer = AbortController::new();
    outer.abort(AbortReason::msg("stop"));
    let outcome = run_workflow(WorkflowRunOptions {
        workflow: &workflow,
        ports: h.ports.clone(),
        max_jobs: 2.0,
        timeout_ms: 10_000.0,
        signal: Some(outer.signal()),
        abort_grace_ms: None,
        now: None,
    })
    .await
    .unwrap();
    assert!(!outcome.ok);
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Aborted);
    assert_eq!(outcome.error.as_ref().unwrap().exit_code, 8);
    assert!(outcome.meta.execution.is_none());
    assert_eq!(
        outcome.jobs.iter().map(|j| j.status).collect::<Vec<_>>(),
        vec![JobStatus::Cancelled, JobStatus::Cancelled]
    );
    assert!(outcome.jobs.iter().all(|j| j.meta.duration_ms.is_none()));
    assert!(h.acquire_calls.lock().unwrap().is_empty());
    let aborts = h.abort_calls.lock().unwrap();
    assert_eq!(aborts.len(), 1);
    assert_eq!(aborts[0].message(), "stop");
}

#[tokio::test(start_paused = true)]
async fn outer_timeout_common_grace_lease_held_until_settle() {
    let workflow = independent(&["a", "b"]).await;
    let a = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let b = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let executor_signal: Arc<Mutex<Option<cheaper_tools::operation::AbortSignal>>> =
        Arc::new(Mutex::new(None));
    let sig2 = Arc::clone(&executor_signal);
    let mut h = common::default_harness();
    h.ports.tool = Arc::new(FnExecutor(move |ctx: WorkflowExecutionContext| {
        let a = Arc::clone(&a2);
        let b = Arc::clone(&b2);
        let sig = Arc::clone(&sig2);
        async move {
            *sig.lock().unwrap() = Some(ctx.signal.clone());
            if ctx.job_id == "a" {
                a.wait().await
            } else {
                b.wait().await
            }
        }
    }));
    let releases = Arc::clone(&h.release_calls);
    let aborts = Arc::clone(&h.abort_calls);
    let completed = Arc::new(AtomicUsize::new(0));
    let completed2 = Arc::clone(&completed);
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        async move {
            let outcome = run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 2.0,
                timeout_ms: 20.0,
                signal: None,
                abort_grace_ms: Some(10.0),
                now: None,
            })
            .await
            .unwrap();
            completed2.fetch_add(1, Ordering::SeqCst);
            outcome
        }
    });
    flush().await;
    advance(20).await;
    {
        let guard = executor_signal.lock().unwrap();
        let signal = guard.as_ref().expect("executor signal captured");
        assert!(signal.is_aborted());
        assert_eq!(signal.reason().unwrap().code(), Some("TIMEOUT"));
    }
    assert_eq!(completed.load(Ordering::SeqCst), 0);
    a.resolve(tool_result("late cooperative result"));
    flush().await;
    assert_eq!(releases.lock().unwrap().len(), 1);
    assert_eq!(completed.load(Ordering::SeqCst), 0);
    advance(10).await;
    let outcome = done.await.unwrap();
    assert!(!outcome.ok);
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Timeout);
    assert_eq!(outcome.error.as_ref().unwrap().exit_code, 7);
    assert_eq!(
        outcome.jobs.iter().map(|j| j.status).collect::<Vec<_>>(),
        vec![JobStatus::Cancelled, JobStatus::Cancelled]
    );
    let snapshot = outcome_serialized(&outcome);
    b.resolve(WorkflowExecutorResult {
        result: json!({ "content": [] }),
        meta: Some({
            let mut m = Map::new();
            m.insert("hidden".to_string(), json!("LATE META"));
            m
        }),
    });
    flush().await;
    assert_eq!(releases.lock().unwrap().len(), 2);
    assert_eq!(outcome_serialized(&outcome), snapshot);
    assert_eq!(aborts.lock().unwrap().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn abort_releases_ready_late_admission_and_detaches_running() {
    let workflow = independent(&["a", "b", "c"]).await;
    let a = Arc::new(Deferred::<WorkflowExecutorResult>::new());
    let admission = Arc::new(Deferred::<cheaper_tools::resources::ResourceLease>::new());
    let mut h = common::default_harness();
    let acquire = h.ports.acquire_job_lease.clone();
    let ad = Arc::clone(&admission);
    let late_lease: Arc<Mutex<Option<cheaper_tools::resources::ResourceLease>>> =
        Arc::new(Mutex::new(None));
    let ll = Arc::clone(&late_lease);
    h.ports.acquire_job_lease = Arc::new(move |request: JobLeaseRequest| {
        let acquire = Arc::clone(&acquire);
        let ad = Arc::clone(&ad);
        let ll = Arc::clone(&ll);
        let job_id = request.job_id.clone();
        Box::pin(async move {
            let lease = acquire(request).await?;
            if job_id != "c" {
                return Ok(lease);
            }
            *ll.lock().unwrap() = Some(lease);
            ad.wait().await
        })
    });
    h.ports.tool = gated(&a);
    let outer = AbortController::new();
    let releases = Arc::clone(&h.release_calls);
    let aborts = Arc::clone(&h.abort_calls);
    let done = tokio::spawn({
        let workflow = Arc::clone(&workflow);
        let ports = h.ports.clone();
        let signal = outer.signal();
        async move {
            run_workflow(WorkflowRunOptions {
                workflow: &workflow,
                ports,
                max_jobs: 1.0,
                timeout_ms: 20.0,
                signal: Some(signal),
                abort_grace_ms: Some(5.0),
                now: None,
            })
            .await
        }
    });
    flush().await;
    // a running、b は ready(lease 保持)、c は admission pending。
    outer.abort(AbortReason::msg("external stop"));
    flush().await;
    assert_eq!(releases.lock().unwrap().len(), 1, "ready lease released");
    advance(5).await;
    let outcome = done.await.unwrap().unwrap();
    assert_eq!(outcome.error.as_ref().unwrap().code, ErrorCode::Aborted);
    assert_eq!(aborts.lock().unwrap().len(), 1);
    assert_eq!(aborts.lock().unwrap()[0].message(), "external stop");
    let lease = late_lease.lock().unwrap().expect("late lease prepared");
    admission.resolve(lease);
    flush().await;
    assert_eq!(releases.lock().unwrap().len(), 2, "late grant released");
    // a の late settle(rejection)で running lease が解放される。
    a.reject();
    flush().await;
    assert_eq!(releases.lock().unwrap().len(), 3);
    assert_eq!(
        outcome.jobs.iter().map(|j| j.status).collect::<Vec<_>>(),
        vec![
            JobStatus::Cancelled,
            JobStatus::Cancelled,
            JobStatus::Cancelled
        ]
    );
}

#[tokio::test]
async fn child_timeout_fails_only_that_job_no_outer_cancel() {
    let workflow = independent(&["a", "b"]).await;
    let mut h = common::default_harness();
    h.ports.tool = Arc::new(FnExecutor(|ctx: WorkflowExecutionContext| async move {
        if ctx.job_id == "a" {
            return Err(
                Box::new(CheaperError::new(ErrorCode::Timeout, "child timeout")) as ToolError,
            );
        }
        Ok(tool_result("independent"))
    }));
    let mut o = options(&workflow, h.ports.clone());
    o.max_jobs = 2.0;
    let outcome = run_workflow(o).await.unwrap();
    assert_eq!(
        outcome.error.as_ref().unwrap().code,
        ErrorCode::ToolExecutionFailure
    );
    assert_eq!(
        outcome.jobs.iter().map(|j| j.status).collect::<Vec<_>>(),
        vec![JobStatus::Failed, JobStatus::Succeeded]
    );
    assert_eq!(outcome.jobs[0].error.as_ref().unwrap().code, "TIMEOUT");
    assert!(h.abort_calls.lock().unwrap().is_empty());
}
