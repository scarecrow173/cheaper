//! `envelope.rs` の byte 互換 contract test。
//!
//! 期待値は `packages/cli/dist/envelope.js` の `JSON.stringify(env) +
//! "\n"` 出力と一致する hand-verified literal(key 順・number format・
//! null 意味論を含む)。

use cheaper_cli::envelope::{
    envelope_json, error_envelope, success_envelope, utility_success_envelope, workflow_envelope,
};
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::classify::TierSource;
use cheaper_core::model::resolve::ModelSelection;
use cheaper_core::model::tier::Tier;
use cheaper_runtime::meta::{ModelMeta, WorkerRunMeta, WorkerSavings, WorkerUsage};
use cheaper_runtime::{PartialRunMeta, WorkerRunError};
use cheaper_tools::execution_metrics::ExecutionMetrics;
use cheaper_tools::journal::ChangesSummary;
use serde_json::{json, Map, Value};

fn map(entries: &[(&str, Value)]) -> Map<String, Value> {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert((*k).to_string(), v.clone());
    }
    m
}

fn run_meta() -> WorkerRunMeta {
    WorkerRunMeta {
        model: ModelMeta {
            provider: "openai".to_string(),
            id: "gpt-x".to_string(),
            tier: Tier::Low,
            selection: ModelSelection::Tier,
            tier_source: TierSource::Auto,
            effective_cost: Some(0.5),
            effective_cost_percentile: Some(12.3456),
            workload: Some(cheaper_runtime::meta::WorkloadMeta {
                output_per_input: 0.4,
            }),
        },
        usage: WorkerUsage {
            input: 100,
            output: 40,
            cache_read: 3,
            cache_write: 0,
            total_tokens: 143,
            cost_usd: 0.000123,
        },
        turns: 2,
        tool_calls: 5,
        rejected_tool_calls: 1,
        repairs: 0,
        transient_retries: 0,
        changes: ChangesSummary::default(),
        execution: ExecutionMetrics::default(),
        savings: WorkerSavings {
            source_tokens: 900,
            returned_tokens: 40,
            avoided_context_tokens: 860,
            directly_comparable: true,
        },
        duration_ms: None,
    }
}

// --- 1. utility success ------------------------------------------------------

#[test]
fn utility_success_byte_vector() {
    let env =
        utility_success_envelope(map(&[("path", json!("a.txt")), ("tokens", json!(3))]), 12.0);
    assert_eq!(
        envelope_json(&env),
        "{\"schemaVersion\":1,\"ok\":true,\"worker\":null,\"result\":{\"path\":\"a.txt\",\"tokens\":3},\"meta\":{\"durationMs\":12}}\n"
    );
}

// --- 2. error envelope: 詳細なし + null meta ---------------------------------

#[test]
fn error_envelope_minimal_byte_vector() {
    let err = CheaperError::new(ErrorCode::InvalidRequest, "command required: run");
    let env = error_envelope(&err, None, Some(5.0));
    assert_eq!(
        envelope_json(&env),
        "{\"schemaVersion\":1,\"ok\":false,\"worker\":null,\"error\":{\"code\":\"INVALID_REQUEST\",\"message\":\"command required: run\",\"exitCode\":2,\"retryable\":false},\"meta\":{\"durationMs\":5,\"model\":null,\"usage\":null,\"turns\":null,\"toolCalls\":null,\"rejectedToolCalls\":null,\"repairs\":null,\"transientRetries\":null,\"changes\":null,\"savings\":null,\"execution\":null}}\n"
    );
}

// --- 3. error envelope: worker + details + CheaperError cause ----------------

#[test]
fn error_envelope_worker_details_cause() {
    let cause = CheaperError::new(ErrorCode::Timeout, "timed out");
    let mut details = Map::new();
    details.insert("name".to_string(), json!("bad-name"));
    let err = CheaperError::new(ErrorCode::WorkerNotFound, "worker not found: bad-name")
        .with_details(details)
        .with_cause(cause);
    let env = error_envelope(&err, Some("bad-name"), Some(7.0));
    assert_eq!(
        envelope_json(&env),
        "{\"schemaVersion\":1,\"ok\":false,\"worker\":\"bad-name\",\"error\":{\"code\":\"WORKER_NOT_FOUND\",\"message\":\"worker not found: bad-name\",\"exitCode\":3,\"retryable\":false,\"details\":{\"name\":\"bad-name\"},\"cause\":{\"code\":\"TIMEOUT\",\"message\":\"timed out\"}},\"meta\":{\"durationMs\":7,\"model\":null,\"usage\":null,\"turns\":null,\"toolCalls\":null,\"rejectedToolCalls\":null,\"repairs\":null,\"transientRetries\":null,\"changes\":null,\"savings\":null,\"execution\":null}}\n"
    );
}

// --- 4. non-CheaperError → INTERNAL_ERROR(70) --------------------------------

#[test]
fn error_envelope_plain_error_maps_internal() {
    let err = std::io::Error::other("disk broke");
    let env = error_envelope(&err, Some("w"), Some(0.0));
    assert_eq!(
        envelope_json(&env),
        "{\"schemaVersion\":1,\"ok\":false,\"worker\":\"w\",\"error\":{\"code\":\"INTERNAL_ERROR\",\"message\":\"disk broke\",\"exitCode\":70,\"retryable\":false},\"meta\":{\"durationMs\":0,\"model\":null,\"usage\":null,\"turns\":null,\"toolCalls\":null,\"rejectedToolCalls\":null,\"repairs\":null,\"transientRetries\":null,\"changes\":null,\"savings\":null,\"execution\":null}}\n"
    );
}

// --- 5. success envelope: meta key 順(durationMs 先頭) -----------------------

#[test]
fn success_envelope_byte_vector() {
    let env = success_envelope(
        "bulk-reader",
        map(&[("files", json!(["a.rs"]))]),
        &run_meta(),
        42.0,
    );
    let s = envelope_json(&env);
    assert!(s.starts_with(
        "{\"schemaVersion\":1,\"ok\":true,\"worker\":\"bulk-reader\",\"result\":{\"files\":[\"a.rs\"]},\"meta\":{\"durationMs\":42,\"model\":{\"provider\":\"openai\",\"id\":\"gpt-x\",\"tier\":\"low\",\"selection\":\"tier\",\"tierSource\":\"auto\",\"effectiveCost\":0.5,\"effectiveCostPercentile\":12.3456,\"workload\":{\"outputPerInput\":0.4}},\"usage\":{\"input\":100,\"output\":40,\"cacheRead\":3,\"cacheWrite\":0,\"totalTokens\":143,\"costUsd\":0.000123},\"turns\":2,\"toolCalls\":5,\"rejectedToolCalls\":1,\"repairs\":0,\"transientRetries\":0,"
    ));
    assert!(s.ends_with("}\n"));
    assert_eq!(s.chars().filter(|c| *c == '\n').count(), 1);
}

// --- 6. WorkerRunError の runMeta が error meta に埋まる ---------------------

#[test]
fn error_envelope_worker_run_error_meta() {
    let partial = PartialRunMeta {
        turns: Some(3),
        ..Default::default()
    };
    let inner = CheaperError::new(ErrorCode::InvalidOutput, "bad output");
    let err = WorkerRunError::new(inner, Some(partial));
    let env = error_envelope(&err, Some("w"), Some(9.0));
    assert_eq!(
        envelope_json(&env),
        "{\"schemaVersion\":1,\"ok\":false,\"worker\":\"w\",\"error\":{\"code\":\"INVALID_OUTPUT\",\"message\":\"bad output\",\"exitCode\":6,\"retryable\":false},\"meta\":{\"durationMs\":9,\"model\":null,\"usage\":null,\"turns\":3,\"toolCalls\":null,\"rejectedToolCalls\":null,\"repairs\":null,\"transientRetries\":null,\"changes\":null,\"savings\":null,\"execution\":null}}\n"
    );
}

// --- 7. workflow envelope ---------------------------------------------------

#[test]
fn workflow_envelope_byte_vector() {
    let env = workflow_envelope(
        true,
        map(&[("out", json!(1))]),
        vec![json!({"id":"j1"})],
        map(&[("durationMs", json!(8))]),
        None,
    );
    assert_eq!(
        envelope_json(&env),
        "{\"schemaVersion\":1,\"workflow\":true,\"ok\":true,\"worker\":null,\"result\":{\"out\":1},\"jobs\":[{\"id\":\"j1\"}],\"meta\":{\"durationMs\":8}}\n"
    );
    let err_env = workflow_envelope(
        false,
        Map::new(),
        vec![],
        map(&[("durationMs", json!(0))]),
        Some(json!({"code":"TIMEOUT","message":"t","exitCode":7,"retryable":true})),
    );
    assert_eq!(
        envelope_json(&err_env),
        "{\"schemaVersion\":1,\"workflow\":true,\"ok\":false,\"worker\":null,\"result\":{},\"jobs\":[],\"meta\":{\"durationMs\":0},\"error\":{\"code\":\"TIMEOUT\",\"message\":\"t\",\"exitCode\":7,\"retryable\":true}}\n"
    );
}

// --- canonical number format 経由 -------------------------------------------

#[test]
fn envelope_canonical_numbers() {
    // JS `JSON.stringify` 規則: 整数は `.0` なし、非有限は null。
    let env = utility_success_envelope(map(&[("n", json!(1.0))]), 1e21);
    let s = envelope_json(&env);
    assert!(s.contains("\"n\":1"));
    assert!(s.contains("\"durationMs\":1e+21"));

    // 小さな絶対値は JS の `1e-7` 形(Rust 既定の `0.0000001` ではない)。
    let env_small = utility_success_envelope(map(&[("n", json!(1e-7))]), 9.9e-7);
    let s_small = envelope_json(&env_small);
    assert!(s_small.contains("\"n\":1e-7"));
    assert!(s_small.contains("\"durationMs\":9.9e-7"));

    let env2 = utility_success_envelope(map(&[("n", json!(f64::NAN))]), f64::INFINITY);
    let s2 = envelope_json(&env2);
    assert!(s2.contains("\"n\":null"));
    assert!(s2.contains("\"durationMs\":null"));
}
