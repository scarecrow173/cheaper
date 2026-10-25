//! 観測値の parser・metrics 抽出・usage.jsonl scan・payload mint・
//! `armExpect` 照合(design §2.3/§3.3)。
//!
//! - envelope parse は **best-effort** — 解釈不能な stdout は harness
//!   error ではなく `envelope_ok: Some(false)` の観測値(§3.3 — nonzero
//!   exit / envelopeOk:false は throw しない)。
//! - byte 計測は必ず `cheaper_core::json::stringify::stringify`
//!   (`JSON.stringify` 規則)経由 — `serde_json::to_string` 系は計測
//!   値に使わない(file 書き出し等の非計測用途は除く)。
//! - mode → parser 写像は本 module が担う — envelope を emit しない
//!   arm(route-run 生 text/hook JSON/harness 内 read)は
//!   `envelopeOk`/`errorCode` が `None`(null 意味論 — §2.3)。

use std::collections::BTreeMap;
use std::path::Path;

use cheaper_core::errors::CheaperError;
use cheaper_core::json::stringify::stringify;
use serde_json::{json, Map, Value};

use crate::fixture::bench_error;
use crate::manifest::{RouteSpec, Scenario};
use crate::spawn::Spawned;

/// 観測 record の `mode` 語彙(§3.3 — v1 mode 名を継承する)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Workflow,
    DirectToolOperation,
    SingleDirectWorkerWithOneToolCall,
    ScriptedDirectCliSequence,
    FullDefinitionRead,
    PublicInterfaceDiscovery,
    RouteResolve,
    RouteRewrite,
    RouteRun,
    RouteLogs,
    DirectChildExec,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::DirectToolOperation => "direct-tool-operation",
            Self::SingleDirectWorkerWithOneToolCall => "single-direct-worker-with-one-tool-call",
            Self::ScriptedDirectCliSequence => "scripted-direct-cli-sequence",
            Self::FullDefinitionRead => "full-definition-read",
            Self::PublicInterfaceDiscovery => "public-interface-discovery",
            Self::RouteResolve => "route-resolve",
            Self::RouteRewrite => "route-rewrite",
            Self::RouteRun => "route-run",
            Self::RouteLogs => "route-logs",
            Self::DirectChildExec => "direct-child-exec",
        }
    }

    /// envelope parser を持つ arm か(`envelopeOk`/`errorCode` が
    /// 意味を持つ arm — §2.3/§3.3 null 意味論)。
    pub fn emits_envelope(self) -> bool {
        matches!(
            self,
            Self::Workflow
                | Self::SingleDirectWorkerWithOneToolCall
                | Self::ScriptedDirectCliSequence
                | Self::PublicInterfaceDiscovery
                | Self::RouteResolve
                | Self::RouteLogs
        )
    }
}

/// `route` block(§3.3)— route 系 arm の観測値。未観測 field は
/// `None`(record では null)。
#[derive(Debug, Default)]
pub struct RouteObservation {
    /// mint/rewrite 観測した payload token の byte 数。
    pub payload_bytes: Option<u64>,
    /// route-rewrite: hook response `hookSpecificOutput.updatedInput.
    /// command` の有無(hook JSON 由来 — envelope ではない)。
    pub rewritten: Option<bool>,
    /// trailer `\[cheaper route log: <id>; …]`(or `kind:"route"`
    /// record `logRef` fallback)。
    pub run_id: Option<String>,
    /// `kind:"route"` record `worker` 非空 = analyze job meta あり。
    pub condense_fired: Option<bool>,
    /// trailer `truncated: <bool>`。
    pub truncated: Option<bool>,
    /// `kind:"route"` record — captured log の o200k 実計数。
    pub original_tokens: Option<f64>,
    /// `kind:"route"` record — 親へ返す text の実計数。
    pub returned_tokens: Option<f64>,
    /// captured exec 出力 byte — `<cfg>/route-logs/<runId>.log`
    /// file size(direct-child-exec では生 stdout byte)。
    pub child_output_bytes: Option<u64>,
    /// route-run stdout の byte 数(body + trailer)。
    pub returned_body_bytes: Option<u64>,
    /// route-logs arm: 返却 `data` の byte(or refetch record
    /// `bytesRead`)。
    pub log_refetch_bytes: Option<u64>,
}

/// `resolve` block(route-resolve arm の verdict 観測値 —
/// `armExpect.resolve` の照合対象)。
#[derive(Debug, Default)]
pub struct ResolveObservation {
    pub matched: Option<bool>,
    pub route: Option<String>,
    pub viability: Option<String>,
    pub executable: Option<String>,
    pub suppressed: Option<String>,
}

/// observation 1 件分(§3.3 — T3 record.rs が wire schema へ投影する
/// までの harness 内部表現。`success` は `armExpect` 全項目一致
/// (§2.3)— expect 未宣言は fallback 規約)。
#[derive(Debug)]
pub struct Observation {
    pub scenario_id: String,
    pub scenario_title: String,
    pub arm: String,
    pub repetition: u32,
    pub warmup: bool,
    pub mode: Mode,
    /// `command` 表示(repo 相対 binary + args — v1 `cliCommandLine`
    /// parity。in-process arm は `std::fs::read(…)` 表記)。
    pub command: String,
    pub wall_time_ms: f64,
    pub success: bool,
    /// `armExpect` 不一致項目の詳細(空 = 全一致/未宣言)。
    pub expect_mismatches: Vec<String>,
    /// spawn arm のみ(in-process arm は `None`)。
    pub exit_code: Option<i32>,
    /// envelope arm のみ(非 envelope arm は `None` — null 意味論)。
    pub envelope_ok: Option<bool>,
    pub error_code: Option<String>,
    /// 当該 observation cfgDir の usage.jsonl 行数(全 kind・
    /// 計測外 setup spawn 分を含む)。
    pub direct_usage_record_count: u64,
    pub direct_invocation_turns_proxy: Option<u64>,
    pub workflow_definition_bytes: Option<u64>,
    pub parent_visible_interface_bytes: Option<u64>,
    pub interface_discovery_wall_time_ms: Option<f64>,
    /// `generated-definition|saved-definition|private-dag-read|
    /// public-interface-only|no-workflow-definition`。
    pub discovery_mode: &'static str,
    pub definition_generation_ms: Option<f64>,
    pub route: Option<RouteObservation>,
    pub resolve: Option<ResolveObservation>,
    /// v1 comparable dims + 追加 metrics(§3.3)。`usage` は
    /// `source:"scripted"` marker 必須。
    pub metrics: Map<String, Value>,
}

impl Observation {
    /// scenario/arm 識別子を持つ空 observation(field は各 arm
    /// impl が埋める — 未観測は None/null のまま残す)。
    pub fn new(
        scenario: &Scenario,
        arm: &str,
        repetition: u32,
        warmup: bool,
        mode: Mode,
        discovery_mode: &'static str,
    ) -> Self {
        Self {
            scenario_id: scenario.id.clone(),
            scenario_title: scenario.title.clone(),
            arm: arm.to_string(),
            repetition,
            warmup,
            mode,
            command: String::new(),
            wall_time_ms: 0.0,
            success: false,
            expect_mismatches: Vec::new(),
            exit_code: None,
            envelope_ok: None,
            error_code: None,
            direct_usage_record_count: 0,
            direct_invocation_turns_proxy: None,
            workflow_definition_bytes: None,
            parent_visible_interface_bytes: None,
            interface_discovery_wall_time_ms: None,
            discovery_mode,
            definition_generation_ms: None,
            route: None,
            resolve: None,
            metrics: Map::new(),
        }
    }

    /// record への JSON 投影(観測 local 形 — run-level field
    /// `runId`/`parentUsage`/`fixtureRevision`/`environment` は
    /// `record::observation_record` が挿入する。field 名は §3.3 と
    /// 一致。`evaluate_expect` の照合対象もこの形)。
    pub fn to_json(&self) -> Value {
        let mut record = Map::new();
        record.insert("schemaVersion".to_string(), json!(2));
        record.insert("scenarioId".to_string(), json!(self.scenario_id));
        record.insert("scenarioTitle".to_string(), json!(self.scenario_title));
        record.insert("arm".to_string(), json!(self.arm));
        record.insert("repetition".to_string(), json!(self.repetition));
        record.insert("warmup".to_string(), json!(self.warmup));
        record.insert("mode".to_string(), json!(self.mode.as_str()));
        record.insert("command".to_string(), json!(self.command));
        record.insert("wallTimeMs".to_string(), json!(self.wall_time_ms));
        record.insert("success".to_string(), json!(self.success));
        if !self.expect_mismatches.is_empty() {
            record.insert(
                "expectMismatches".to_string(),
                json!(self.expect_mismatches),
            );
        }
        record.insert(
            "exitCode".to_string(),
            self.exit_code.map_or(Value::Null, |c| json!(c)),
        );
        record.insert(
            "envelopeOk".to_string(),
            self.envelope_ok.map_or(Value::Null, Value::from),
        );
        record.insert(
            "errorCode".to_string(),
            self.error_code.as_ref().map_or(Value::Null, |c| json!(c)),
        );
        record.insert(
            "directUsageRecordCount".to_string(),
            json!(self.direct_usage_record_count),
        );
        let opt = |v: Option<u64>| v.map_or(Value::Null, |n| json!(n));
        record.insert(
            "directInvocationTurnsProxy".to_string(),
            opt(self.direct_invocation_turns_proxy),
        );
        record.insert(
            "workflowDefinitionBytes".to_string(),
            opt(self.workflow_definition_bytes),
        );
        record.insert(
            "parentVisibleInterfaceBytes".to_string(),
            opt(self.parent_visible_interface_bytes),
        );
        record.insert(
            "interfaceDiscoveryWallTimeMs".to_string(),
            self.interface_discovery_wall_time_ms
                .map_or(Value::Null, |n| json!(n)),
        );
        record.insert("discoveryMode".to_string(), json!(self.discovery_mode));
        record.insert(
            "definitionGenerationMs".to_string(),
            self.definition_generation_ms
                .map_or(Value::Null, |n| json!(n)),
        );
        if let Some(route) = &self.route {
            let mut block = Map::new();
            let opt_f = |v: Option<f64>| v.map_or(Value::Null, |n| json!(n));
            block.insert("payloadBytes".to_string(), opt(route.payload_bytes));
            block.insert(
                "rewritten".to_string(),
                route.rewritten.map_or(Value::Null, Value::from),
            );
            block.insert(
                "runId".to_string(),
                route.run_id.as_ref().map_or(Value::Null, |s| json!(s)),
            );
            block.insert(
                "condenseFired".to_string(),
                route.condense_fired.map_or(Value::Null, Value::from),
            );
            block.insert(
                "truncated".to_string(),
                route.truncated.map_or(Value::Null, Value::from),
            );
            block.insert("originalTokens".to_string(), opt_f(route.original_tokens));
            block.insert("returnedTokens".to_string(), opt_f(route.returned_tokens));
            block.insert(
                "childOutputBytes".to_string(),
                opt(route.child_output_bytes),
            );
            block.insert(
                "returnedBodyBytes".to_string(),
                opt(route.returned_body_bytes),
            );
            block.insert("logRefetchBytes".to_string(), opt(route.log_refetch_bytes));
            record.insert("route".to_string(), Value::Object(block));
        }
        if let Some(resolve) = &self.resolve {
            let mut block = Map::new();
            block.insert(
                "matched".to_string(),
                resolve.matched.map_or(Value::Null, Value::from),
            );
            block.insert(
                "route".to_string(),
                resolve.route.as_ref().map_or(Value::Null, |s| json!(s)),
            );
            block.insert(
                "viability".to_string(),
                resolve.viability.as_ref().map_or(Value::Null, |s| json!(s)),
            );
            block.insert(
                "executable".to_string(),
                resolve
                    .executable
                    .as_ref()
                    .map_or(Value::Null, |s| json!(s)),
            );
            block.insert(
                "suppressed".to_string(),
                resolve
                    .suppressed
                    .as_ref()
                    .map_or(Value::Null, |s| json!(s)),
            );
            record.insert("resolve".to_string(), Value::Object(block));
        }
        record.insert("metrics".to_string(), Value::Object(self.metrics.clone()));
        Value::Object(record)
    }
}

// ---------------------------------------------------------------------------
// byte 計測・envelope 観測
// ---------------------------------------------------------------------------

/// `JSON.stringify` 規則の byte 数(全 byte 計測の唯一経路)。
pub fn json_bytes(value: &Value) -> u64 {
    stringify(value).len() as u64
}

/// stdout の envelope parse — best-effort(1 JSON + 改行の stdout
/// contract を期待するが、解釈不能は `None` で観測側へ潰す)。
pub fn parse_envelope(stdout: &[u8]) -> Option<Value> {
    let text = std::str::from_utf8(stdout).ok()?.trim().to_string();
    if text.is_empty() {
        return None;
    }
    serde_json::from_str(&text).ok()
}

/// `envelope.ok === true` の観測値 — parse 不能・ok 非 true は
/// `Some(false)`(envelope arm では malformed も観測値)。
pub fn envelope_ok(envelope: Option<&Value>) -> bool {
    envelope
        .and_then(|e| e.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// `envelope.error.code` の観測値(error envelope 以外は None)。
pub fn envelope_error_code(envelope: Option<&Value>) -> Option<String> {
    envelope?
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// `envelope.meta.durationMs`(cliReportedDurationMs)。
pub fn envelope_duration_ms(envelope: Option<&Value>) -> Option<f64> {
    envelope?
        .get("meta")
        .and_then(|m| m.get("durationMs"))
        .and_then(Value::as_f64)
}

/// `envelope.result`(`workflow interface`/`route resolve`/`route
/// logs` の utility envelope payload — envelope.result の byte は
/// `parentVisibleInterfaceBytes`/`logRefetchBytes` の素材)。
pub fn envelope_result(envelope: &Value) -> Option<&Map<String, Value>> {
    envelope.get("result").and_then(Value::as_object)
}

// ---------------------------------------------------------------------------
// envelope → metrics(v1 `summarizeWorkerEnvelope`/
// `measureWorkflowEnvelope` port)
// ---------------------------------------------------------------------------

fn finite_count(meta: Option<&Map<String, Value>>, key: &str) -> f64 {
    meta.and_then(|m| m.get(key))
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .unwrap_or(0.0)
}

/// `usage` object の集計(`{input,output,cachedInput←cacheRead,
/// cachedOutput←cacheWrite,total←totalTokens}` — v1 field 写像)。
fn usage_sum(usages: Vec<Option<Map<String, Value>>>) -> Map<String, Value> {
    let mut sum = Map::new();
    for (target, source) in [
        ("input", "input"),
        ("output", "output"),
        ("cachedInput", "cacheRead"),
        ("cachedOutput", "cacheWrite"),
        ("total", "totalTokens"),
    ] {
        let total: f64 = usages
            .iter()
            .map(|u| finite_count(u.as_ref(), source))
            .sum();
        sum.insert(target.to_string(), json!(total));
    }
    sum
}

/// usage map(観測値)→ `source:"scripted"` marker 付き metrics.usage
/// (§3.3 spec invariant — 実料金・実モデル効率の観測値ではない)。
fn scripted_usage(mut usage: Map<String, Value>) -> Value {
    usage.insert("source".to_string(), json!("scripted"));
    Value::Object(usage)
}

/// worker run envelope(`run bulk-reader`)→ metrics — v1
/// `summarizeWorkerEnvelope` + result byte 相当。
pub fn worker_envelope_metrics(envelope: Option<&Value>) -> WorkerMetrics {
    let meta = envelope
        .and_then(|e| e.get("meta"))
        .and_then(Value::as_object);
    let usage = meta
        .and_then(|m| m.get("usage"))
        .and_then(Value::as_object)
        .cloned();
    WorkerMetrics {
        worker_turns: finite_count(meta, "turns"),
        tool_calls: finite_count(meta, "toolCalls"),
        usage,
        repairs: finite_count(meta, "repairs"),
        transient_retries: finite_count(meta, "transientRetries"),
        duration_ms: envelope_duration_ms(envelope),
    }
}

/// `run bulk-reader` 1 spawn 分の metrics(v1 `runDirectWorker` の
/// 返却相当)。
#[derive(Debug)]
pub struct WorkerMetrics {
    pub worker_turns: f64,
    pub tool_calls: f64,
    /// `meta.usage`(raw — scripted marker は集計側で付ける)。
    pub usage: Option<Map<String, Value>>,
    pub repairs: f64,
    pub transient_retries: f64,
    pub duration_ms: Option<f64>,
}

/// workflow envelope → comparable metrics — v1
/// `measureWorkflowEnvelope` port(`criticalPathMs` は別関数)。
pub fn workflow_envelope_metrics(
    envelope: &Value,
    workflow: Option<&Value>,
    metrics: &mut Map<String, Value>,
) {
    let jobs = envelope
        .get("jobs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut worker_jobs = 0.0f64;
    let mut tool_jobs = 0.0f64;
    let mut turns = 0.0f64;
    let mut tool_calls = 0.0f64;
    let mut repairs = 0.0f64;
    let mut transient_retries = 0.0f64;
    let mut usages: Vec<Option<Map<String, Value>>> = Vec::with_capacity(jobs.len());
    for job in &jobs {
        let meta = job.get("meta").and_then(Value::as_object);
        let is_worker = meta.is_some_and(|m| {
            m.get("usage").is_some_and(Value::is_object) || m.contains_key("turns")
        });
        if is_worker {
            worker_jobs += 1.0;
        } else {
            tool_jobs += 1.0;
        }
        turns += finite_count(meta, "turns");
        tool_calls += finite_count(meta, "toolCalls");
        repairs += finite_count(meta, "repairs");
        transient_retries += finite_count(meta, "transientRetries");
        usages.push(
            meta.and_then(|m| m.get("usage"))
                .and_then(Value::as_object)
                .cloned(),
        );
    }
    let job_count = jobs.len() as f64;
    metrics.insert("jobCount".to_string(), json!(job_count));
    metrics.insert("workerJobCount".to_string(), json!(worker_jobs));
    metrics.insert("toolOnlyJobCount".to_string(), json!(tool_jobs));
    metrics.insert("workerTurns".to_string(), json!(turns));
    metrics.insert("toolCalls".to_string(), json!(tool_calls));
    metrics.insert("usage".to_string(), scripted_usage(usage_sum(usages)));
    metrics.insert("repairs".to_string(), json!(repairs));
    metrics.insert("transientRetries".to_string(), json!(transient_retries));

    // artifacts / retained bytes。
    let artifacts = envelope
        .get("meta")
        .and_then(|m| m.get("artifacts"))
        .and_then(Value::as_array);
    let artifact_bytes: u64 = artifacts
        .map(|list| {
            list.iter()
                .filter_map(|a| a.get("bytes").and_then(Value::as_u64))
                .sum()
        })
        .unwrap_or(0);
    let result_bytes = envelope.get("result").map(json_bytes).unwrap_or(0);
    metrics.insert(
        "retainedBytes".to_string(),
        json!(result_bytes + artifact_bytes),
    );
    metrics.insert("retainedResultBytes".to_string(), json!(result_bytes));
    metrics.insert("artifactBytes".to_string(), json!(artifact_bytes));
    metrics.insert("materializedPromptBytes".to_string(), Value::Null);
    metrics.insert(
        "criticalPathMs".to_string(),
        // duration 欠落/非有限 → None、cycle/未知参照 → Err — いずれ
        // も観測不能として null(§3.3 — 未知 dimension を 0 に潰さない)。
        critical_path_ms(envelope, workflow)
            .ok()
            .flatten()
            .map_or(Value::Null, |v| json!(v)),
    );
    let concurrency = envelope
        .get("meta")
        .and_then(|m| m.get("execution"))
        .and_then(|e| e.get("maxConcurrency"))
        .and_then(Value::as_f64);
    metrics.insert(
        "maxConcurrency".to_string(),
        concurrency.map_or(Value::Null, |v| json!(v)),
    );
    metrics.insert("workerStreamConcurrency".to_string(), Value::Null);
    metrics.insert(
        "toolOperationConcurrency".to_string(),
        concurrency.map_or(Value::Null, |v| json!(v)),
    );
    metrics.insert("nodeCount".to_string(), json!(job_count));
    metrics.insert(
        "cliReportedDurationMs".to_string(),
        envelope_duration_ms(Some(envelope)).map_or(Value::Null, |v| json!(v)),
    );
    // `postprocessFailures` — INVALID_OUTPUT で終了した job の
    // repair round 和(§3.3 — 観測不能経路は null)。
    let post_failures: Option<f64> = {
        let mut found = false;
        let sum: f64 = jobs
            .iter()
            .filter(|job| {
                job.get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(Value::as_str)
                    == Some("INVALID_OUTPUT")
            })
            .map(|job| {
                found = true;
                finite_count(job.get("meta").and_then(Value::as_object), "repairs")
            })
            .sum();
        found.then_some(sum)
    };
    metrics.insert(
        "postprocessFailures".to_string(),
        post_failures.map_or(Value::Null, |v| json!(v)),
    );
}

/// `job.input` 内の `{"$from": "<jobId>"}` binding 参照を集める(v1
/// `bindingDependencies` port — needs 宣言と併せて DAG edge を導く)。
fn binding_dependencies<'a>(value: &'a Value, out: &mut std::collections::BTreeSet<&'a str>) {
    match value {
        Value::Array(items) => {
            for item in items {
                binding_dependencies(item, out);
            }
        }
        Value::Object(map) => {
            if let Some(from) = map.get("$from").and_then(Value::as_str) {
                out.insert(from);
            } else {
                for child in map.values() {
                    binding_dependencies(child, out);
                }
            }
        }
        _ => {}
    }
}

/// envelope.jobs[i].meta.durationMs + workflow.jobs の依存 DAG から
/// critical path ms を導く(v1 `criticalPathMs` port — duration 欠落・
/// 非有限・負数は `Ok(None)`、cycle/未知参照は `Err`)。
pub fn critical_path_ms(
    envelope: &Value,
    workflow: Option<&Value>,
) -> Result<Option<f64>, CheaperError> {
    let (Some(jobs), Some(def_jobs)) = (
        envelope.get("jobs").and_then(Value::as_array),
        workflow
            .and_then(|w| w.get("jobs"))
            .and_then(Value::as_object),
    ) else {
        return Ok(None);
    };
    let outcomes: BTreeMap<&str, &Value> = jobs
        .iter()
        .filter_map(|j| j.get("id").and_then(Value::as_str).map(|id| (id, j)))
        .collect();
    let mut durations: BTreeMap<&str, f64> = BTreeMap::new();
    for id in def_jobs.keys() {
        let duration = outcomes
            .get(id.as_str())
            .and_then(|j| j.get("meta"))
            .and_then(|m| m.get("durationMs"))
            .and_then(Value::as_f64);
        match duration {
            Some(d) if d.is_finite() && d >= 0.0 => {
                durations.insert(id.as_str(), d);
            }
            _ => return Ok(None),
        }
    }
    let mut remaining: std::collections::BTreeSet<&str> = durations.keys().copied().collect();
    let mut path_ms: BTreeMap<&str, f64> = BTreeMap::new();
    while !remaining.is_empty() {
        let mut progressed = false;
        let ids: Vec<&str> = remaining.iter().copied().collect();
        for id in ids {
            let job = &def_jobs[id];
            let mut deps: std::collections::BTreeSet<&str> = job
                .get("needs")
                .and_then(Value::as_array)
                .map(|needs| {
                    needs
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<std::collections::BTreeSet<&str>>()
                })
                .unwrap_or_default();
            if let Some(input) = job.get("input") {
                binding_dependencies(input, &mut deps);
            }
            if deps.iter().any(|dep| !path_ms.contains_key(dep)) {
                continue;
            }
            let parent_max = deps
                .iter()
                .filter_map(|dep| path_ms.get(dep))
                .fold(0.0f64, |acc, v| acc.max(*v));
            path_ms.insert(id, parent_max + durations[id]);
            remaining.remove(id);
            progressed = true;
        }
        if !progressed {
            return Err(bench_error(
                "workflow job dependency graph is cyclic or references an unknown job",
            ));
        }
    }
    Ok(Some(path_ms.values().fold(0.0f64, |acc, v| acc.max(*v))))
}

// ---------------------------------------------------------------------------
// usage.jsonl scan(§3.3 `directUsageRecordCount`・route record 観測)
// ---------------------------------------------------------------------------

/// `<cfg>/usage.jsonl` の scan 結果。
#[derive(Debug, Default)]
pub struct UsageScan {
    /// 全 record 行数(全 kind・計測外 setup 分を含む)。
    pub record_count: u64,
    /// `kind:"route"` record(観測値 — 複数あれば最後の 1 件)。
    pub route: Option<RouteRecord>,
    /// `kind:"route-refetch"` record 件数。
    pub log_refetches: u64,
    /// `kind:"route-refetch"` record の `bytesRead` 和。
    pub refetch_bytes_read: u64,
}

/// `kind:"route"` record の観測 field。
#[derive(Debug)]
pub struct RouteRecord {
    /// `worker` object 非空 = analyze job meta あり = condenseFired。
    pub worker_nonempty: bool,
    pub original_tokens: Option<f64>,
    pub returned_tokens: Option<f64>,
    pub exit_code: Option<i32>,
    pub ok: Option<bool>,
    pub log_ref: Option<String>,
    pub postprocess_failures: Option<f64>,
    /// `durationMs.{exec, analyze, total}`。
    pub duration_ms: Option<Map<String, Value>>,
}

/// usage.jsonl を読み record 集計する(ENOENT = 0 件)。
pub fn scan_usage_log(config_dir: &Path) -> Result<UsageScan, CheaperError> {
    let path = config_dir.join("usage.jsonl");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(UsageScan::default());
        }
        Err(e) => {
            return Err(bench_error(format!(
                "usage.jsonl read failed {}: {e}",
                path.display()
            )));
        }
    };
    let mut scan = UsageScan::default();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        scan.record_count += 1;
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match record.get("kind").and_then(Value::as_str) {
            Some("route") => {
                let worker_nonempty = record
                    .get("worker")
                    .and_then(Value::as_object)
                    .is_some_and(|w| !w.is_empty());
                scan.route = Some(RouteRecord {
                    worker_nonempty,
                    original_tokens: record.get("originalTokens").and_then(Value::as_f64),
                    returned_tokens: record.get("returnedTokens").and_then(Value::as_f64),
                    exit_code: record
                        .get("exitCode")
                        .and_then(Value::as_i64)
                        .and_then(|v| i32::try_from(v).ok()),
                    ok: record.get("ok").and_then(Value::as_bool),
                    log_ref: record
                        .get("logRef")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    postprocess_failures: record.get("postprocessFailures").and_then(Value::as_f64),
                    duration_ms: record.get("durationMs").and_then(Value::as_object).cloned(),
                });
            }
            Some("route-refetch") => {
                scan.log_refetches += 1;
                scan.refetch_bytes_read +=
                    record.get("bytesRead").and_then(Value::as_u64).unwrap_or(0);
            }
            _ => {}
        }
    }
    Ok(scan)
}

// ---------------------------------------------------------------------------
// route-run trailer parser(envelope 非出力 arm の raw-text 観測 — §2.3)
// ---------------------------------------------------------------------------

/// trailer 形式 `\[cheaper route log: <runId>; truncated: <bool>]\n`
/// — `\S+` の runId は `]`・`;`・空白を含まない token。
/// stdout 末尾から最後の trailer を取り出す(regex 非依存 — 固定
/// format の手書き parse で dep を増やさない)。
pub fn parse_route_trailer(stdout: &[u8]) -> Option<(String, bool)> {
    let text = String::from_utf8_lossy(stdout);
    const PREFIX: &str = "[cheaper route log: ";
    let start = text.rfind(PREFIX)? + PREFIX.len();
    let rest = &text[start..];
    let end = rest.find(']')?;
    let inner = &rest[..end];
    let (run_id, flag) = inner.split_once("; truncated:")?;
    let run_id = run_id.trim();
    if run_id.is_empty() {
        return None;
    }
    match flag.trim() {
        "true" => Some((run_id.to_string(), true)),
        "false" => Some((run_id.to_string(), false)),
        _ => None,
    }
}

/// `<cfg>/route-logs/<runId>.log` の file size
/// (`route.childOutputBytes` — captured exec 出力 byte)。
pub fn route_log_bytes(config_dir: &Path, run_id: &str) -> Option<u64> {
    let path = config_dir.join("route-logs").join(format!("{run_id}.log"));
    std::fs::metadata(path).ok().map(|m| m.len())
}

// ---------------------------------------------------------------------------
// hook response parser(route-rewrite arm — envelope ではない JSON)
// ---------------------------------------------------------------------------

/// hook stdout JSON → `hookSpecificOutput.updatedInput.command`。
/// hook JSON の parse 失敗・key 不在は `None`(rewritten=false の
/// 観測値 — deny/allow の non-rewrite verdict を含む)。
pub fn parse_hook_rewrite(stdout: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(stdout).ok()?.trim().to_string();
    let value: Value = serde_json::from_str(&text).ok()?;
    value
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(Value::as_str)
        .map(str::to_string)
}

// ---------------------------------------------------------------------------
// route-run payload mint(§2.3 導出規則 — hook build_rewrite_command
// と同規約を harness 側で再現)
// ---------------------------------------------------------------------------

/// `command_name` — basename + `.exe`/`.bat`/`.cmd` 除去
/// (`routing_policy.rs` の crate-private 規約の harness 側複製 —
/// executable 名判定の leaf rule)。
fn command_name(token: &str) -> String {
    let base = token.rsplit(['/', '\\']).next().unwrap_or(token);
    for ext in [".exe", ".bat", ".cmd"] {
        if base.len() >= ext.len() && base.to_ascii_lowercase().ends_with(ext) {
            return base[..base.len() - ext.len()].to_string();
        }
    }
    base.to_string()
}

/// `route.command` → `(argv, args, executable)` — env-prefix strip +
/// match.command literal prefix 規約(validation 済み前提だが、
/// mint 側でも防御的に検査して argv-mismatch reject を未然に防ぐ)。
pub fn route_argv_parts(
    route: &RouteSpec,
) -> Result<(Vec<String>, Vec<String>, String), CheaperError> {
    let segments = cheaper_cli::hook::routing_policy::tokenize_shell(&route.command)
        .filter(|s| s.len() == 1)
        .ok_or_else(|| bench_error("route.command must tokenize to a single segment"))?;
    let segment = &segments[0];
    if segment.unsafe_segment || segment.tokens.iter().any(|t| t.unsafe_token) {
        return Err(bench_error("route.command contains unsafe tokens"));
    }
    let argv: Vec<String> = segment.tokens.iter().map(|t| t.text.clone()).collect();
    let (_, argv) = cheaper_cli::route::resolve::split_env_prefix(&argv);
    let argv = argv.to_vec();
    let match_len = route.route_match.command.len();
    if argv.len() < match_len || argv[..match_len] != route.route_match.command[..] {
        return Err(bench_error(
            "route.command argv lacks the declared match.command prefix",
        ));
    }
    let executable = command_name(&argv[0]);
    let args = argv[match_len..].to_vec();
    Ok((argv, args, executable))
}

/// `route run --payload <tok>` の token を mint する — §2.3 導出
/// 規約(`payload::encode` `EncodeInput` — ambient_env は
/// `passthroughEnv` 名の spawn env 値のみ、12 scenario は `[]` の
/// ため空)。
pub fn mint_route_payload(
    route: &RouteSpec,
    workspace_root: &Path,
    spawn_env: &BTreeMap<String, String>,
) -> Result<String, CheaperError> {
    let (argv, args, executable) = route_argv_parts(route)?;
    let cwd = cheaper_core::workspace::paths::canonicalize_workspace_root(
        &workspace_root.to_string_lossy(),
    )?;
    let ambient_env: BTreeMap<String, String> = route
        .passthrough_env
        .iter()
        .filter_map(|name| spawn_env.get(name).map(|v| (name.clone(), v.clone())))
        .collect();
    cheaper_cli::route::payload::encode(cheaper_cli::route::payload::EncodeInput {
        command: route.command.clone(),
        executable,
        argv,
        args,
        cwd,
        ambient_env,
        passthrough_env: route.passthrough_env.clone(),
    })
}

// ---------------------------------------------------------------------------
// `armExpect` 照合(§2.3 success 意味論 — 全項目一致が success。
// mismatch obs は harness error ではなく `success:false` 観測値)
// ---------------------------------------------------------------------------

/// 宣言 `armExpect` 値と observation の key 単位照合 — 不一致項目の
/// 人間可読な記述列を返す(空 = 全一致)。
pub fn evaluate_expect(expect: &Value, observation: &Observation) -> Vec<String> {
    let mut mismatches = Vec::new();
    let Some(obj) = expect.as_object() else {
        return mismatches;
    };
    let observed = observation.to_json();
    for (key, expected) in obj {
        match key.as_str() {
            "route" | "resolve" | "metrics" => {
                let Some(expected_obj) = expected.as_object() else {
                    continue;
                };
                let observed_obj = observed.get(key.as_str()).and_then(Value::as_object);
                for (field, expected_value) in expected_obj {
                    let actual = observed_obj
                        .and_then(|o| o.get(field))
                        .cloned()
                        .unwrap_or(Value::Null);
                    if &actual != expected_value {
                        mismatches.push(format!(
                            "{key}.{field}: expected {}, observed {}",
                            stringify(expected_value),
                            stringify(&actual)
                        ));
                    }
                }
            }
            _ => {
                let actual = observed.get(key.as_str()).cloned().unwrap_or(Value::Null);
                if &actual != expected {
                    mismatches.push(format!(
                        "{key}: expected {}, observed {}",
                        stringify(expected),
                        stringify(&actual)
                    ));
                }
            }
        }
    }
    mismatches
}

/// `armExpect` 未宣言 arm の fallback success(§2.3 — `exitCode==0`
/// かつ envelope arm は `envelopeOk===true`。in-process arm は
/// exit code 非観測のため harness error 未到達なら true)。
pub fn fallback_success(mode: Mode, exit_code: Option<i32>, envelope_ok: Option<bool>) -> bool {
    let exit_ok = exit_code.is_none_or(|c| c == 0);
    let envelope_ok = if mode.emits_envelope() {
        envelope_ok.unwrap_or(false)
    } else {
        true
    };
    exit_ok && envelope_ok
}

/// spawn 結果の command 表示(repo 相対 binary + args — v1
/// `cliCommandLine` parity)。
pub fn display_command(binary: &Path, repo_root: &Path, args: &[String]) -> String {
    let shown = binary
        .strip_prefix(repo_root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| binary.to_string_lossy().into_owned());
    format!("{} {}", shown, args.join(" "))
}

/// in-process arm の command 表示(`std::fs::read` 等 — v1
/// `node:fs/promises.readFile` の Rust 置換表記)。
pub fn display_in_process(label: &str) -> String {
    format!("std::{label}")
}

/// spawn 観測値を observation へ畳む共通処理 — wall/exit/envelope
/// 系をまとめて設定する(arms 各 impl の重複を避ける集中点)。
pub fn observe_spawn(
    observation: &mut Observation,
    spawned: &Spawned,
    command: String,
) -> Option<Value> {
    observation.command = command;
    observation.wall_time_ms = spawned.wall_time.as_secs_f64() * 1000.0;
    observation.exit_code = spawned.exit_code;
    if observation.mode.emits_envelope() {
        let envelope = parse_envelope(&spawned.stdout);
        observation.envelope_ok = Some(envelope_ok(envelope.as_ref()));
        observation.error_code = envelope_error_code(envelope.as_ref());
        envelope
    } else {
        None
    }
}

/// `metrics.usage` へ scripted marker 付きで挿入する(単一 worker
/// spawn の観測 usage)。
pub fn insert_scripted_usage(metrics: &mut Map<String, Value>, usage: Option<Map<String, Value>>) {
    metrics.insert(
        "usage".to_string(),
        usage.map_or(Value::Null, scripted_usage),
    );
}

/// usage map 同士の加算(direct-worker-sequence 集計 — 欠落 key・
/// 非数は 0 として畳む)。
pub fn add_usage(acc: &mut Map<String, Value>, usage: Option<&Map<String, Value>>) {
    let Some(usage) = usage else { return };
    for (target, source) in [
        ("input", "input"),
        ("output", "output"),
        ("cachedInput", "cacheRead"),
        ("cachedOutput", "cacheWrite"),
        ("total", "totalTokens"),
    ] {
        let add = finite_count(Some(usage), source);
        let entry = acc.entry(target.to_string()).or_insert_with(|| json!(0.0));
        if let Some(current) = entry.as_f64() {
            *entry = json!(current + add);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn write_usage_file(lines: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut file = std::fs::File::create(dir.path().join("usage.jsonl")).unwrap();
        for line in lines {
            writeln!(file, "{line}").unwrap();
        }
        dir
    }

    // -- envelope parser ----------------------------------------------------

    #[test]
    fn parse_envelope_is_best_effort() {
        let ok = parse_envelope(br#"{"ok":true,"result":{"x":1}}"#);
        assert_eq!(ok.as_ref().unwrap()["ok"], json!(true));
        // 解釈不能(非 JSON・空・非 UTF-8)は例外ではなく None。
        assert!(parse_envelope(b"not json at all").is_none());
        assert!(parse_envelope(b"  \n").is_none());
        assert!(parse_envelope(&[0xff, 0xfe]).is_none());
    }

    #[test]
    fn envelope_field_accessors() {
        let ok_env = json!({
            "ok": true,
            "result": {"interface": "text"},
            "meta": {"durationMs": 12.5}
        });
        assert!(envelope_ok(Some(&ok_env)));
        assert_eq!(envelope_error_code(Some(&ok_env)), None);
        assert_eq!(envelope_duration_ms(Some(&ok_env)), Some(12.5));
        assert_eq!(
            envelope_result(&ok_env).unwrap()["interface"],
            json!("text")
        );

        let err_env = json!({
            "ok": false,
            "error": {"code": "TIMEOUT", "message": "t"}
        });
        assert!(!envelope_ok(Some(&err_env)));
        assert_eq!(
            envelope_error_code(Some(&err_env)).as_deref(),
            Some("TIMEOUT")
        );

        // parse 不能/ok 非 true は Some(false) の観測値(malformed
        // envelope も envelope arm では観測値 — §3.3)。
        assert!(!envelope_ok(None));
        assert!(!envelope_ok(Some(&json!({"ok": "yes"}))));
        assert_eq!(envelope_error_code(None), None);
    }

    // -- envelope → metrics -------------------------------------------------

    #[test]
    fn worker_metrics_extract_meta_fields() {
        let envelope = json!({
            "ok": true,
            "meta": {
                "turns": 2,
                "toolCalls": 1,
                "repairs": 0,
                "transientRetries": 0,
                "durationMs": 9.0,
                "usage": {"input": 10, "output": 20, "totalTokens": 30}
            }
        });
        let m = worker_envelope_metrics(Some(&envelope));
        assert_eq!(m.worker_turns, 2.0);
        assert_eq!(m.tool_calls, 1.0);
        assert_eq!(m.duration_ms, Some(9.0));
        assert_eq!(m.usage.as_ref().unwrap()["input"], json!(10));
        // parse 不能 envelope は全 field 既定値(0/None — 観測値として
        // 潰れる、throw しない)。
        let m = worker_envelope_metrics(None);
        assert_eq!(m.worker_turns, 0.0);
        assert!(m.usage.is_none());
        assert!(m.duration_ms.is_none());
    }

    #[test]
    fn workflow_metrics_aggregate_jobs_and_scripted_usage() {
        let workflow = json!({
            "jobs": {
                "read": {"kind": "tool"},
                "analyze": {"kind": "worker", "input": {"hints": [{"$from": "read"}]}}
            }
        });
        let envelope = json!({
            "ok": true,
            "result": {"summary": "done"},
            "jobs": [
                {"id": "read", "meta": {"durationMs": 3.0}},
                {"id": "analyze", "meta": {
                    "durationMs": 7.0,
                    "turns": 1, "toolCalls": 0,
                    "usage": {"input": 5, "output": 6, "totalTokens": 11}
                }}
            ],
            "meta": {"durationMs": 10.0, "artifacts": [{"bytes": 4}], "execution": {"maxConcurrency": 2}}
        });
        let mut metrics = Map::new();
        workflow_envelope_metrics(&envelope, Some(&workflow), &mut metrics);
        assert_eq!(metrics["jobCount"], json!(2.0));
        assert_eq!(metrics["workerJobCount"], json!(1.0));
        assert_eq!(metrics["toolOnlyJobCount"], json!(1.0));
        // usage 集計は `source:"scripted"` marker 必須(§3.3 invariant)。
        assert_eq!(metrics["usage"]["source"], json!("scripted"));
        assert_eq!(metrics["usage"]["input"], json!(5.0));
        assert_eq!(metrics["usage"]["total"], json!(11.0));
        // retainedBytes = result bytes + artifact bytes。
        let expected_result = json_bytes(envelope.get("result").unwrap());
        assert_eq!(metrics["retainedResultBytes"], json!(expected_result));
        assert_eq!(metrics["retainedBytes"], json!(expected_result + 4));
        assert_eq!(metrics["criticalPathMs"], json!(10.0));
        assert_eq!(metrics["cliReportedDurationMs"], json!(10.0));
        assert_eq!(metrics["maxConcurrency"], json!(2.0));
    }

    #[test]
    fn workflow_metrics_postprocess_failures_counts_invalid_output_repairs() {
        let envelope = json!({
            "ok": false,
            "jobs": [
                {"id": "a", "error": {"code": "INVALID_OUTPUT"}, "meta": {"repairs": 1}},
                {"id": "b", "error": {"code": "TIMEOUT"}, "meta": {"repairs": 0}}
            ],
        });
        let mut metrics = Map::new();
        workflow_envelope_metrics(&envelope, None, &mut metrics);
        assert_eq!(metrics["postprocessFailures"], json!(1.0));
    }

    #[test]
    fn critical_path_takes_dependency_max_and_handles_missing_durations() {
        // fan-in: analyze waits on max(left,right) = 5 → path 12。
        let workflow = json!({
            "jobs": {
                "left": {}, "right": {},
                "summary": {"needs": ["left", "right"]}
            }
        });
        let envelope = json!({
            "jobs": [
                {"id": "left", "meta": {"durationMs": 5.0}},
                {"id": "right", "meta": {"durationMs": 2.0}},
                {"id": "summary", "meta": {"durationMs": 7.0}}
            ]
        });
        assert_eq!(
            critical_path_ms(&envelope, Some(&workflow)).unwrap(),
            Some(12.0)
        );
        // `$from` binding も dependency として畳む(needs 無しでも)。
        let binding_workflow = json!({
            "jobs": {
                "a": {},
                "b": {"input": {"x": {"$from": "a"}}}
            }
        });
        let envelope = json!({
            "jobs": [
                {"id": "a", "meta": {"durationMs": 4.0}},
                {"id": "b", "meta": {"durationMs": 1.0}}
            ]
        });
        assert_eq!(
            critical_path_ms(&envelope, Some(&binding_workflow)).unwrap(),
            Some(5.0)
        );
        // duration 欠落 → Ok(None)(観測不能 = null、0 へ潰さない)。
        let envelope = json!({"jobs": [{"id": "a", "meta": {}}]});
        assert_eq!(
            critical_path_ms(&envelope, Some(&json!({"jobs": {"a": {}}}))).unwrap(),
            None
        );
        // cycle → Err(観測不能だが graph 自体が invalid — caller が
        // null へ潰す)。
        let cyclic = json!({
            "jobs": {
                "a": {"needs": ["b"]},
                "b": {"needs": ["a"]}
            }
        });
        let envelope = json!({
            "jobs": [
                {"id": "a", "meta": {"durationMs": 1.0}},
                {"id": "b", "meta": {"durationMs": 1.0}}
            ]
        });
        assert!(critical_path_ms(&envelope, Some(&cyclic)).is_err());
    }

    // -- usage.jsonl scan -----------------------------------------------------

    #[test]
    fn scan_usage_log_counts_records_and_extracts_route_fields() {
        let dir = write_usage_file(&[
            r#"{"kind":"run","worker":"bulk-reader"}"#,
            "",
            r#"{"kind":"route","worker":{"model":"m"},"originalTokens":100,"returnedTokens":8,"exitCode":7,"ok":true,"logRef":"r1","postprocessFailures":0,"durationMs":{"exec":1}}"#,
            r#"{"kind":"route-refetch","bytesRead":64}"#,
            r#"{"kind":"route-refetch","bytesRead":32}"#,
            r#"not-json-but-counted"#,
        ]);
        let scan = scan_usage_log(dir.path()).unwrap();
        // 非空行は全 kind 計数(JSON parse 不能行も含む — §3.3)。
        assert_eq!(scan.record_count, 5);
        assert_eq!(scan.log_refetches, 2);
        assert_eq!(scan.refetch_bytes_read, 96);
        let route = scan.route.expect("route record");
        assert!(route.worker_nonempty);
        assert_eq!(route.original_tokens, Some(100.0));
        assert_eq!(route.returned_tokens, Some(8.0));
        assert_eq!(route.exit_code, Some(7));
        assert_eq!(route.ok, Some(true));
        assert_eq!(route.log_ref.as_deref(), Some("r1"));
        assert_eq!(route.duration_ms.as_ref().unwrap()["exec"], json!(1));
    }

    #[test]
    fn scan_usage_log_keeps_last_route_record() {
        let dir = write_usage_file(&[
            r#"{"kind":"route","logRef":"first","exitCode":0}"#,
            r#"{"kind":"route","logRef":"last","exitCode":1}"#,
        ]);
        let scan = scan_usage_log(dir.path()).unwrap();
        let route = scan.route.unwrap();
        assert_eq!(route.log_ref.as_deref(), Some("last"));
        assert_eq!(route.exit_code, Some(1));
    }

    #[test]
    fn scan_usage_log_missing_file_is_zero_records() {
        let dir = tempfile::tempdir().unwrap();
        let scan = scan_usage_log(dir.path()).unwrap();
        assert_eq!(scan.record_count, 0);
        assert!(scan.route.is_none());
        assert_eq!(scan.log_refetches, 0);
    }

    // -- route-run trailer parser -------------------------------------------

    #[test]
    fn route_trailer_parses_canonical_form() {
        let stdout = b"some body output\n[cheaper route log: 01JXYZ; truncated: true]\n";
        assert_eq!(
            parse_route_trailer(stdout),
            Some(("01JXYZ".to_string(), true))
        );
        let stdout = b"[cheaper route log: run_2; truncated: false]";
        assert_eq!(
            parse_route_trailer(stdout),
            Some(("run_2".to_string(), false))
        );
    }

    #[test]
    fn route_trailer_missing_or_malformed_is_none() {
        assert_eq!(parse_route_trailer(b"no trailer here"), None);
        assert_eq!(
            parse_route_trailer(b"[cheaper route log: ; truncated: true]"),
            None
        );
        assert_eq!(
            parse_route_trailer(b"[cheaper route log: r1; truncated: maybe]"),
            None
        );
        // `rfind` — 末尾 trailer が優先される(body 内に酷似文字列が
        // あっても最後のものを取る)。
        let stdout = b"[cheaper route log: decoy; truncated: false]\nbody\n[cheaper route log: real; truncated: true]";
        assert_eq!(
            parse_route_trailer(stdout),
            Some(("real".to_string(), true))
        );
    }

    // -- hook response parser -------------------------------------------------

    #[test]
    fn hook_rewrite_extracts_updated_input_command() {
        let stdout = br#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":{"command":"<exe> route run --payload tok"},"permissionDecision":"allow"}}"#;
        assert_eq!(
            parse_hook_rewrite(stdout).as_deref(),
            Some("<exe> route run --payload tok")
        );
        // non-rewrite verdict(allow/deny without updatedInput)・parse
        // 失敗は None → rewritten=false の観測値。
        assert_eq!(
            parse_hook_rewrite(br#"{"hookSpecificOutput":{"permissionDecision":"allow"}}"#),
            None
        );
        assert_eq!(parse_hook_rewrite(b"garbage"), None);
    }

    // -- route argv / payload -------------------------------------------------

    fn route_spec(command: &str, match_cmd: &[&str]) -> RouteSpec {
        serde_json::from_value(json!({
            "name": "r",
            "match": {"command": match_cmd},
            "command": command,
            "grant": "g",
            "childMode": "emit"
        }))
        .unwrap()
    }

    #[test]
    fn route_argv_parts_strips_env_prefix_and_checks_match() {
        let route = route_spec("FOO=1 benchrun --fixture-child emit", &["benchrun"]);
        let (argv, args, executable) = route_argv_parts(&route).unwrap();
        assert_eq!(argv, vec!["benchrun", "--fixture-child", "emit"]);
        assert_eq!(args, vec!["--fixture-child", "emit"]);
        assert_eq!(executable, "benchrun");
    }

    #[test]
    fn route_argv_parts_rejects_prefix_mismatch() {
        let route = route_spec("other --fixture-child emit", &["benchrun"]);
        assert!(route_argv_parts(&route).is_err());
    }

    #[test]
    fn command_name_strips_path_and_ext() {
        assert_eq!(command_name("benchrun"), "benchrun");
        assert_eq!(command_name("C:\\bin\\tool.EXE"), "tool");
        assert_eq!(command_name("/usr/bin/runner.cmd"), "runner");
        assert_eq!(command_name("./rel/x.bat"), "x");
    }

    // -- armExpect 照合 / fallback success ------------------------------------

    fn observation_for_expect() -> Observation {
        let scenario: Scenario = serde_json::from_value(json!({
            "id": "s", "title": "t", "arms": ["route-run"]
        }))
        .unwrap();
        let mut obs = Observation::new(&scenario, "route-run", 0, false, Mode::RouteRun, "x");
        obs.exit_code = Some(0);
        obs.route = Some(RouteObservation {
            condense_fired: Some(false),
            truncated: Some(true),
            ..RouteObservation::default()
        });
        obs
    }

    #[test]
    fn evaluate_expect_reports_only_mismatched_fields() {
        let obs = observation_for_expect();
        let expect = json!({
            "exitCode": 0,
            "route": {"condenseFired": false, "truncated": true}
        });
        assert!(evaluate_expect(&expect, &obs).is_empty());

        let expect = json!({
            "exitCode": 0,
            "route": {"condenseFired": true},
            "metrics": {"logRefetches": 1}
        });
        let mismatches = evaluate_expect(&expect, &obs);
        assert_eq!(mismatches.len(), 2);
        assert!(mismatches[0].contains("route.condenseFired"));
        assert!(mismatches[1].contains("metrics.logRefetches"));
    }

    #[test]
    fn evaluate_expect_null_expected_matches_null_actual() {
        // null pin は null 観測値に一致する(非 envelope arm の
        // `envelopeOk: null` pin 等 — 宣言された null 意味論)。
        let scenario: Scenario = serde_json::from_value(json!({
            "id": "s", "title": "t", "arms": ["direct-child-exec"]
        }))
        .unwrap();
        let mut obs = Observation::new(
            &scenario,
            "direct-child-exec",
            0,
            false,
            Mode::DirectChildExec,
            "x",
        );
        obs.exit_code = Some(7);
        let expect = json!({"exitCode": 7, "envelopeOk": null});
        assert!(evaluate_expect(&expect, &obs).is_empty());
    }

    #[test]
    fn fallback_success_envelope_arms_require_ok_envelope() {
        // 非 envelope arm — exit 0 で true(envelopeOk 無観測)。
        assert!(fallback_success(Mode::RouteRun, Some(0), None));
        assert!(!fallback_success(Mode::RouteRun, Some(7), None));
        // envelope arm — envelopeOk===true が必要(parse 不能の
        // Some(false) で fail)。
        assert!(fallback_success(Mode::Workflow, Some(0), Some(true)));
        assert!(!fallback_success(Mode::Workflow, Some(0), Some(false)));
        assert!(!fallback_success(Mode::Workflow, Some(0), None));
        // in-process arm(exit_code None)— harness error 未到達なら true。
        assert!(fallback_success(Mode::DirectToolOperation, None, None));
    }

    // -- usage 集計 -----------------------------------------------------------

    #[test]
    fn insert_scripted_usage_marks_source_and_null_is_preserved() {
        let mut metrics = Map::new();
        let usage: Map<String, Value> = serde_json::from_value(json!({
            "input": 1, "output": 2
        }))
        .unwrap();
        insert_scripted_usage(&mut metrics, Some(usage));
        assert_eq!(metrics["usage"]["source"], json!("scripted"));
        let mut metrics = Map::new();
        insert_scripted_usage(&mut metrics, None);
        assert!(metrics["usage"].is_null());
    }

    #[test]
    fn add_usage_maps_cache_and_total_fields() {
        let mut acc = Map::new();
        let usage: Map<String, Value> = serde_json::from_value(json!({
            "input": 10, "output": 5,
            "cacheRead": 3, "cacheWrite": 1, "totalTokens": 18
        }))
        .unwrap();
        add_usage(&mut acc, Some(&usage));
        add_usage(&mut acc, Some(&usage));
        assert_eq!(acc["input"], json!(20.0));
        assert_eq!(acc["cachedInput"], json!(6.0));
        assert_eq!(acc["cachedOutput"], json!(2.0));
        assert_eq!(acc["total"], json!(36.0));
        // None は no-op(欠落 usage で 0 へ潰さない)。
        add_usage(&mut acc, None);
        assert_eq!(acc["input"], json!(20.0));
    }
}
