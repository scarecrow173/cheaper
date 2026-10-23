//! T3 run orchestration 統合テスト — `runWorker` + fake provider seam。
//!
//! `dev-seam` cargo feature でのみ compile される seam 依存 test
//! (`fake_provider` の `FakeSessionFactory`/`FakeResponse`/`ScriptedSession`
//! を使う全 case)。`required-features = ["dev-seam"]` で gate 済み —
//! feature 無しの `cargo test --workspace` ではこの target は skip
//! される。non-seam test(`join_error_normalizes_to_internal_error`)は
//! `tests/run.rs` に残る。
//!
//! 対象(run-semantics spec):
//! - success + `meta`(usage/turns/transientRetries/costUsd/repairs)
//! - structured output 抽出 + bounded repair + shared usage 畳込
//! - abort 分類 4 系統(timeout/signal/maxTurns/maxToolCalls)+ 先勝ち
//! - prompt abort grace(signal を無視する stream の待ち切り)
//! - PARTIAL_MUTATION wrap(journal 非空)
//! - panic / JoinError → INTERNAL_ERROR
//! - usage-bearing error(provider failure の runMeta に model+usage)

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::config::ModelConfig;
use cheaper_core::policy::process::ResolvedProcessPolicy;
use cheaper_core::policy::schema::{
    ExecutionPolicy, FilesystemPolicy, ModelPolicy, OutputPolicy, RetryPolicy, RoutePolicy,
    RuntimePolicy, TierFallback, ToolsPolicy,
};
use cheaper_core::policy::security::SecurityPolicy;
use cheaper_core::policy::workflow::{WorkflowArtifactPolicy, WorkflowPolicy};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::parse::parse_worker_file;
use cheaper_core::worker::validate::{validate_worker_definition, WorkerOrigin};
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_rig_adapter::{RunEvent, UsageSnapshot};
use cheaper_runtime::auth::api_key::ProviderEnv;
use cheaper_runtime::catalog::schema::{ModelCostJson, ModelDefinitionJson};
use cheaper_runtime::dev::fake_provider::{FakeResponse, FakeSessionFactory, ToolCallSpec};
use cheaper_runtime::plugin::{
    CheaperProviderPlugin, PluginProviderRegistration, ProviderPluginContext, ProviderRegistry,
};
use cheaper_runtime::run::{run_worker, RunWorkerDeps, RunWorkerOk, SessionFactory, WorkerSession};
use cheaper_runtime::Runtime;
use cheaper_tools::operation::{AbortController, AbortReason};
use futures::StreamExt;
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

struct MapEnv;
impl ProviderEnv for MapEnv {
    fn env(&self, _name: &str) -> Option<String> {
        None
    }
    fn file_exists(&self, _path: &str) -> bool {
        false
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
}

/// fake provider plugin — `api_key` 設定済みで configured 扱い、model
/// `fake/fake-model`(cost input=1.0 output=2.0 cache*=0.5 /1M)を登録する。
struct FakeProviderPlugin;
impl CheaperProviderPlugin for FakeProviderPlugin {
    fn id(&self) -> &str {
        "fake"
    }
    fn register(&self, registry: &mut ProviderRegistry, _ctx: &ProviderPluginContext) {
        registry.register_provider(PluginProviderRegistration {
            id: "fake".to_string(),
            name: "Fake Provider".to_string(),
            api: "fake-api".to_string(),
            base_url: "http://fake.local".to_string(),
            api_key: Some("test-key".to_string()),
            auth_header: None,
            headers: None,
            models: vec![ModelDefinitionJson {
                id: "fake-model".to_string(),
                name: None,
                api: None,
                base_url: None,
                reasoning: None,
                thinking_level_map: None,
                input: None,
                cost: Some(ModelCostJson {
                    input: 1.0,
                    output: 2.0,
                    cache_read: 0.5,
                    cache_write: 0.5,
                    tiers: None,
                }),
                context_window: Some(128_000.0),
                max_tokens: Some(8192.0),
                sampling_params: None,
                headers: None,
                compat: None,
            }],
            auth: None,
            subscription: false,
            catalog: None,
        });
    }
}

/// `cheaper.optionalTools` opt-in worker — `tools: [read, process,
/// command]` で両 gated tool を opt-in 宣言し、builtin default policy
/// 相当(tasks/grants 空)で両方が縮退する。
fn opt_in_worker_definition(
    root: &std::path::Path,
    blocked_status: bool,
) -> cheaper_core::worker::validate::WorkerDefinition {
    let path = root.join("opt-worker.md");
    let status_enum = if blocked_status {
        "[success, partial, failed, blocked]"
    } else {
        "[success, failed]"
    };
    let content = format!(
        r#"---
name: opt-worker
description: opt-in gated worker
tools: [read, process, command]
cheaper:
  optionalTools: [process, command]
  model:
    tier: low
  output:
    schema:
      inline:
        type: object
        required: [status]
        properties:
          status:
            type: string
            enum: {status_enum}
---
Do the thing.
"#
    );
    std::fs::write(&path, &content).unwrap();
    let parsed = parse_worker_file(content.as_str(), path.to_str().unwrap()).unwrap();
    validate_worker_definition(&parsed, path.to_str().unwrap(), WorkerOrigin::Explicit).unwrap()
}

fn worker_definition(root: &std::path::Path) -> cheaper_core::worker::validate::WorkerDefinition {
    let path = root.join("test-worker.md");
    let content = r#"---
name: test-worker
description: test worker
tools: [read, write]
cheaper:
  model:
    tier: low
  writeScope: files
  output:
    schema:
      inline:
        type: object
        required: [ok]
        properties:
          ok:
            type: boolean
---
Do the thing.
"#;
    std::fs::write(&path, content).unwrap();
    let parsed = parse_worker_file(content, path.to_str().unwrap()).unwrap();
    validate_worker_definition(&parsed, path.to_str().unwrap(), WorkerOrigin::Explicit).unwrap()
}

fn policy() -> RuntimePolicy {
    policy_with(ExecutionPolicy {
        timeout_ms: 60_000,
        // 既定は `maxTurns: null` 相当(policy 上限なし)— bounded が
        // 必要な test は個別に `Some(N)` を立てる。
        max_turns: None,
        max_tool_calls: 50,
        concurrency: None,
    })
}

fn policy_with(execution: ExecutionPolicy) -> RuntimePolicy {
    RuntimePolicy {
        workflow: WorkflowPolicy {
            max_jobs: 4,
            artifacts: WorkflowArtifactPolicy {
                max_artifact_bytes: 4096,
                max_total_bytes: 16384,
                max_preview_bytes: 4096,
                max_read_bytes: 4096,
            },
        },
        process: ResolvedProcessPolicy {
            max_concurrent: 4,
            tasks: Map::new(),
            timeout: "2m".into(),
            max_output_bytes: 4096,
            kill_grace: "2s".into(),
            heartbeat: cheaper_core::policy::process::ProcessHeartbeat {
                interval: "30s".into(),
            },
        },
        command: cheaper_core::policy::command::ResolvedCommandPolicy {
            max_concurrent: 4,
            grants: std::collections::BTreeMap::new(),
            timeout: "2m".into(),
            max_output_bytes: 4096,
            kill_grace: "2s".into(),
            heartbeat: cheaper_core::policy::process::ProcessHeartbeat {
                interval: "30s".into(),
            },
        },
        route: RoutePolicy::default(),
        security: SecurityPolicy::default(),
        execution,
        tools: ToolsPolicy {
            allow: vec![ToolName::Read, ToolName::Write],
        },
        filesystem: FilesystemPolicy {
            symlinks: SymlinkPolicy::Deny,
            deny_paths: vec![],
        },
        model: ModelPolicy {
            max_tier: cheaper_core::model::tier::Tier::High,
            tier_fallback: TierFallback::Higher,
        },
        output: OutputPolicy {
            max_tokens: 4096,
            repair_attempts: 2,
        },
        retry: RetryPolicy {
            transient_attempts: 2,
        },
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    runtime: Runtime,
    worker: cheaper_core::worker::validate::WorkerDefinition,
    policy: RuntimePolicy,
    model_config: ModelConfig,
    input: Value,
}

fn fixture(policy: RuntimePolicy) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let user_config = dir.path().join("config");
    std::fs::create_dir_all(&user_config).unwrap();
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![Arc::new(FakeProviderPlugin)];
    let runtime =
        Runtime::create(user_config, Arc::new(MapEnv), &plugins, BTreeMap::new()).unwrap();
    let worker = worker_definition(&root);
    Fixture {
        _dir: dir,
        root,
        runtime,
        worker,
        policy,
        model_config: ModelConfig::default(),
        input: json!({"files": ["a.txt"]}),
    }
}

fn deps<'a>(fx: &'a Fixture, factory: &'a FakeSessionFactory) -> RunWorkerDeps<'a> {
    RunWorkerDeps {
        worker: &fx.worker,
        input: &fx.input,
        root: fx.root.to_str().unwrap().to_string(),
        policy: &fx.policy,
        model_config: &fx.model_config,
        runtime: &fx.runtime,
        session_factory: factory,
        explicit_model: Some("fake/fake-model"),
        signal: None,
        abort_grace_ms: Some(80.0),
        execution: None,
        operation_clock: None,
        invocation_grants: &[],
        resume: None,
    }
}

fn usage(input: u64, output: u64) -> UsageSnapshot {
    UsageSnapshot {
        input_tokens: input,
        output_tokens: output,
        total_tokens: input + output,
        ..UsageSnapshot::default()
    }
}

const OK_OUTPUT: &str = r#"{"ok":true}"#;

// ---------------------------------------------------------------------------
// success + meta
// ---------------------------------------------------------------------------

#[tokio::test]
async fn success_extracts_output_and_builds_meta() {
    let fx = fixture(policy());
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: format!("```json\n{OK_OUTPUT}\n```"),
        usage: UsageSnapshot {
            input_tokens: 1000,
            output_tokens: 2000,
            cache_creation_input_tokens: 50,
            cached_input_tokens: 100,
            total_tokens: 3150,
            ..UsageSnapshot::default()
        },
        turns: 2,
        last_turn_tool_calls: false,
        transient_retries: 1,
    }]);
    let RunWorkerOk { result, meta } = run_worker(&deps(&fx, &factory)).await.unwrap();

    assert_eq!(result["ok"], json!(true));
    assert_eq!(meta.model.provider, "fake");
    assert_eq!(meta.model.id, "fake-model");
    assert_eq!(meta.usage.input, 1000);
    assert_eq!(meta.usage.output, 2000);
    assert_eq!(meta.usage.cache_read, 100);
    assert_eq!(meta.usage.cache_write, 50);
    assert_eq!(meta.usage.total_tokens, 3150);
    // cheaper pricing(1.0/2.0/0.5/0.5 per 1M)× token 数:
    // (1000*1.0 + 2000*2.0 + 100*0.5 + 50*0.5) / 1e6 = 0.005075
    assert!((meta.usage.cost_usd - 0.005_075).abs() < 1e-9);
    assert_eq!(meta.turns, 2);
    assert_eq!(meta.transient_retries, 1);
    assert_eq!(meta.tool_calls, 0);
    assert_eq!(meta.repairs, 0);
    assert!(meta.savings.returned_tokens > 0);
    assert_eq!(factory.pending_responses(), 0);
}

// ---------------------------------------------------------------------------
// repair
// ---------------------------------------------------------------------------

#[tokio::test]
async fn repair_merges_usage_and_returns_repaired_result() {
    let fx = fixture(policy());
    let factory = FakeSessionFactory::new(vec![
        FakeResponse::Complete {
            text: "not json at all".into(),
            usage: usage(100, 10),
            turns: 1,
            last_turn_tool_calls: false,
            transient_retries: 0,
        },
        FakeResponse::Complete {
            text: "still bad".into(),
            usage: usage(50, 5),
            turns: 1,
            last_turn_tool_calls: false,
            transient_retries: 1,
        },
        FakeResponse::Complete {
            text: OK_OUTPUT.into(),
            usage: usage(60, 6),
            turns: 1,
            last_turn_tool_calls: false,
            transient_retries: 0,
        },
    ]);
    let RunWorkerOk { result, meta } = run_worker(&deps(&fx, &factory)).await.unwrap();

    assert_eq!(result["ok"], json!(true));
    assert_eq!(meta.repairs, 2);
    // repair session の usage/turns/transientRetries は main に畳み込む。
    assert_eq!(meta.usage.input, 210);
    assert_eq!(meta.usage.output, 21);
    assert_eq!(meta.turns, 3);
    assert_eq!(meta.transient_retries, 1);

    let created = factory.created();
    assert_eq!(created.len(), 3);
    assert!(created[0].has_tool_runtime);
    // F8: main session にも worker output schema hint を渡す。
    assert!(created[0].has_output_schema);
    // repair session は tool-less + output_schema hint 付き。
    assert!(!created[1].has_tool_runtime);
    assert!(created[1].has_output_schema);
    // repair prompt に前回 raw output が載る。
    let prompts = factory.prompts();
    assert_eq!(prompts.len(), 3);
    assert!(prompts[1].contains("failed schema validation"));
    assert!(prompts[1].contains("not json at all"));
}

#[tokio::test]
async fn repair_exhausted_returns_invalid_output_with_usage_meta() {
    let mut p = policy();
    p.output.repair_attempts = 1;
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![
        FakeResponse::Complete {
            text: "bad".into(),
            usage: usage(100, 10),
            turns: 1,
            last_turn_tool_calls: false,
            transient_retries: 0,
        },
        FakeResponse::Complete {
            text: "also bad".into(),
            usage: usage(50, 5),
            turns: 1,
            last_turn_tool_calls: false,
            transient_retries: 0,
        },
    ]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidOutput);
    // details: errors 先頭 20 件・rawOutput 先頭 2000 文字。
    let details = err.details().expect("INVALID_OUTPUT has details");
    assert!(details["errors"].is_array());
    assert_eq!(details["rawOutput"], json!("also bad"));
    // usage-bearing error — meta.model + meta.usage が残る。
    let meta = err.run_meta().expect("run meta on failure");
    assert_eq!(meta.usage.as_ref().unwrap().input, 150);
    assert_eq!(meta.repairs, Some(1));
    assert_eq!(meta.model.as_ref().unwrap().id, "fake-model");
}

// ---------------------------------------------------------------------------
// abort 分類
// ---------------------------------------------------------------------------

#[tokio::test]
async fn timeout_abort_classifies_time_out() {
    let mut p = policy();
    p.execution.timeout_ms = 50;
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Hang]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Timeout);
    assert_eq!(err.details().unwrap()["reason"], json!("timeout"));
    // abort 分類でも full meta が付く(model+usage)。
    assert!(err.run_meta().unwrap().model.is_some());
}

/// T3(issue #111): run 経路の `timeoutMs` は stall window — 進捗
/// `RunEvent` が流れ続ける限り全体時間が window を超えても完走する。
/// 50ms 間隔の emit で合計 ~650ms > window(500ms)としても TIMEOUT
/// にならないことを pin する。
#[tokio::test]
async fn progress_events_reset_stall_timeout() {
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};

    let mut p = policy();
    p.execution.timeout_ms = 500;
    let fx = fixture(p);
    let mut steps = Vec::new();
    for i in 0..12 {
        steps.push(ScriptStep::Delay { ms: 50 });
        steps.push(ScriptStep::Emit {
            event: RunEvent::TextDelta {
                text: format!("delta-{i}"),
            },
        });
    }
    steps.push(ScriptStep::Finish {
        text: OK_OUTPUT.into(),
    });
    let factory = FakeSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession { steps })]);
    let start = std::time::Instant::now();
    run_worker(&deps(&fx, &factory)).await.unwrap();
    // 進捗駆動なので window 超過分(~600ms)掛かるが TIMEOUT にならない。
    assert!(start.elapsed() >= Duration::from_millis(500));
}

/// T4(issue #111): 最後の進捗から `timeoutMs` を超えて event が途絶えた
/// run は `TIMEOUT` で abort される(stall window の発火側)。
#[tokio::test]
async fn stall_timeout_fires_without_progress() {
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};

    let mut p = policy();
    p.execution.timeout_ms = 100;
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession {
        steps: vec![
            ScriptStep::Emit {
                event: RunEvent::TextDelta {
                    text: "first".into(),
                },
            },
            // 進捗が止まる — delay 中に stall window が発火する。
            ScriptStep::Delay { ms: 60_000 },
        ],
    })]);
    let start = std::time::Instant::now();
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Timeout);
    assert_eq!(err.details().unwrap()["reason"], json!("timeout"));
    // stall window ~100ms + abort 処理で返る(60s delay までは待たない)。
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn external_signal_classifies_aborted() {
    let fx = fixture(policy());
    let controller = AbortController::new();
    let signal = controller.signal();
    let factory = FakeSessionFactory::new(vec![FakeResponse::WaitAbort {
        usage: usage(10, 1),
        turns: 1,
    }]);
    let mut d = deps(&fx, &factory);
    d.signal = Some(&signal);
    // run 開始後に外部 abort。
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        controller.abort(AbortReason::new("test", "user abort", None));
    });
    let err = run_worker(&d).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Aborted);
    assert_eq!(err.details().unwrap()["reason"], json!("signal"));
    // WaitAbort までに計上された usage が meta に残る。
    assert_eq!(err.run_meta().unwrap().usage.as_ref().unwrap().input, 10);
}

#[tokio::test]
async fn hanging_stream_stops_waiting_after_grace() {
    let fx = fixture(policy());
    let controller = AbortController::new();
    let signal = controller.signal();
    // signal を観測しない stream — grace(abort_grace_ms=80)で待ち切り。
    let factory = FakeSessionFactory::new(vec![FakeResponse::Hang]);
    let mut d = deps(&fx, &factory);
    d.signal = Some(&signal);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        controller.abort(AbortReason::new("test", "user abort", None));
    });
    let start = std::time::Instant::now();
    let err = run_worker(&d).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Aborted);
    // grace(80ms)経過で返る — 十分速い(ハングしない)ことを確認。
    assert!(start.elapsed() < Duration::from_secs(5));
}

/// T1(issue #111): `maxTurns: null`(既定 `None`)では policy レベルの
/// turn 上限が無く、低 tier model 相当の多 turn run も完走する。
/// 以前の既定(8)を超える turn 数で pinned する。
#[tokio::test]
async fn null_max_turns_allows_multi_turn_run_to_complete() {
    let fx = fixture(policy());
    assert_eq!(fx.policy.execution.max_turns, None);
    // `last_turn_tool_calls: true` + turns > 旧既定(8) — bounded
    // `Some(8)` なら `MAX_TURNS_EXCEEDED` になる組合せで、null 既定が
    // policy abort を発火させないことを pin する。
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: OK_OUTPUT.into(),
        usage: usage(10, 1),
        turns: 12,
        last_turn_tool_calls: true,
        transient_retries: 0,
    }]);
    run_worker(&deps(&fx, &factory)).await.unwrap();
}

#[tokio::test]
async fn max_turns_classifies_max_turns_exceeded() {
    let mut p = policy();
    p.execution.max_turns = Some(2);
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: OK_OUTPUT.into(),
        usage: usage(10, 1),
        turns: 2,
        last_turn_tool_calls: true,
        transient_retries: 0,
    }]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::MaxTurnsExceeded);
    assert_eq!(err.details().unwrap()["reason"], json!("maxTurns"));
    assert_eq!(err.run_meta().unwrap().turns, Some(2));
}

#[tokio::test]
async fn max_tool_calls_classifies_via_on_limit() {
    let mut p = policy();
    p.execution.max_tool_calls = 1;
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::ToolCalls {
        calls: vec![
            ToolCallSpec {
                name: "write".into(),
                input: json!({"path": "a.txt", "content": "x"}),
            },
            ToolCallSpec {
                name: "write".into(),
                input: json!({"path": "a.txt", "content": "y"}),
            },
        ],
        next: Box::new(FakeResponse::WaitAbort {
            usage: usage(10, 1),
            turns: 1,
        }),
    }]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    // 1 回目の write は実行済み(journal 非空)→ abort 系 error は
    // PARTIAL_MUTATION で wrap され、cause が MAX_TOOL_CALLS_EXCEEDED。
    assert_eq!(err.code(), ErrorCode::PartialMutation);
    // cause = MAX_TOOL_CALLS_EXCEEDED の abort error。
    let cause = err
        .error()
        .source()
        .and_then(|s| s.downcast_ref::<CheaperError>())
        .expect("cause is CheaperError");
    assert_eq!(cause.code(), ErrorCode::MaxToolCallsExceeded);
    // rejected tool call が meta に残る。
    let meta = err.run_meta().unwrap();
    assert_eq!(meta.tool_calls, Some(1));
    assert_eq!(meta.rejected_tool_calls, Some(1));
}

// ---------------------------------------------------------------------------
// PARTIAL_MUTATION / provider error / panic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn abort_after_mutation_wraps_partial_mutation() {
    let fx = fixture(policy());
    let controller = AbortController::new();
    let signal = controller.signal();
    let factory = FakeSessionFactory::new(vec![FakeResponse::ToolCalls {
        calls: vec![ToolCallSpec {
            name: "write".into(),
            input: json!({"path": "a.txt", "content": "x"}),
        }],
        next: Box::new(FakeResponse::WaitAbort {
            usage: usage(10, 1),
            turns: 1,
        }),
    }]);
    let mut d = deps(&fx, &factory);
    d.signal = Some(&signal);
    tokio::spawn(async move {
        // write が journal に乗る時間を確保してから abort。
        tokio::time::sleep(Duration::from_millis(120)).await;
        controller.abort(AbortReason::new("test", "user abort", None));
    });
    let err = run_worker(&d).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::PartialMutation);
    assert!(
        err.details().unwrap()["changes"]["totals"]["files"]
            .as_u64()
            .unwrap()
            > 0
    );
    let cause = err
        .error()
        .source()
        .and_then(|s| s.downcast_ref::<CheaperError>())
        .expect("cause is CheaperError");
    assert_eq!(cause.code(), ErrorCode::Aborted);
    // 実ファイルが書かれている(partial mutation の実効証拠)。
    assert_eq!(std::fs::read_to_string(fx.root.join("a.txt")).unwrap(), "x");
}

#[tokio::test]
async fn provider_failure_keeps_code_and_usage_meta() {
    let fx = fixture(policy());
    let factory = FakeSessionFactory::new(vec![FakeResponse::Fail {
        error: CheaperError::new(ErrorCode::ProviderFailure, "boom"),
        usage: usage(42, 4),
        turns: 1,
    }]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::ProviderFailure);
    let meta = err.run_meta().expect("usage-bearing failure");
    assert_eq!(meta.usage.as_ref().unwrap().input, 42);
    assert!(meta.model.is_some());
}

#[tokio::test]
async fn panic_normalizes_to_internal_error() {
    let fx = fixture(policy());
    let factory = FakeSessionFactory::new(vec![FakeResponse::Panic {
        message: "kaboom".into(),
    }]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::InternalError);
    assert_eq!(err.exit_code(), 70);
    assert!(err.message().contains("kaboom"));
}

// ---------------------------------------------------------------------------
// model resolution errors
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invalid_model_ref_is_invalid_request() {
    let fx = fixture(policy());
    let factory = FakeSessionFactory::new(vec![]);
    let mut d = deps(&fx, &factory);
    d.explicit_model = Some("no-slash-here");
    let err = run_worker(&d).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
    assert_eq!(err.details().unwrap()["reason"], json!("invalid-model-ref"));
}

// ---------------------------------------------------------------------------
// first-reason-wins(単体は abort.rs 内、ここでは run 経路の複合確認)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn timeout_does_not_reclassify_existing_signal_reason() {
    // signal が先に落ちた後に timeout が来ても ABORTED を保つ。
    let mut p = policy();
    p.execution.timeout_ms = 60;
    let fx = fixture(p);
    let controller = AbortController::new();
    let signal = controller.signal();
    let factory = FakeSessionFactory::new(vec![FakeResponse::WaitAbort {
        usage: usage(1, 1),
        turns: 1,
    }]);
    let mut d = deps(&fx, &factory);
    d.signal = Some(&signal);
    // stall window(進捗起点の 60ms)と spawn 済み signal(20ms)の
    // 起点がずれる — 高負荷 runner で spawn の初回 poll が遅れると
    // timeout が先に fire し得るため、signal 側を即 abort にして
    // 確定的に先勝ちさせる。
    tokio::spawn(async move {
        controller.abort(AbortReason::new("test", "user abort", None));
    });
    let err = run_worker(&d).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Aborted);
    assert_eq!(err.details().unwrap()["reason"], json!("signal"));
}

// ---------------------------------------------------------------------------
// review cycle 1 — maxTurns の tool batch 順序 / symlink policy / 内部 guard
// ---------------------------------------------------------------------------

#[tokio::test]
async fn max_turns_waits_for_final_tool_batch_before_aborting() {
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};

    // F1: maxTurns に達した turn が tool call で終わった場合、その
    // pending batch の実行・`ToolResult` emit を待ってから abort する。
    // 即 abort だと Rig の `drive_tool_calls` が batch 途中で drop され、
    // 最終 turn の tool 副作用・結果 event・usage が欠落する。
    // `ScriptStep::ToolCalls` は Rig と同じ emit 順序を再現する —
    // `ToolCall` event → batch 実行 → `ToolResult` event。
    let mut p = policy();
    p.execution.max_turns = Some(1);
    let fx = fixture(p);
    let root = fx.root.clone();
    let factory = FakeSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession {
        steps: vec![
            // turn 受理(tool 継続)— `Turn` は snapshot だけを更新し
            // event は emit しない。続く `ToolCalls` の `ToolCall`
            // event emit 時点で pending=1 なので abort は発火しない。
            ScriptStep::Turn { tool_calls: true },
            // model tool-call event → 実実行(a.txt 書込)→
            // `ToolResult` event で pending=0 → deferred abort 発火。
            ScriptStep::ToolCalls {
                calls: vec![ToolCallSpec {
                    name: "write".into(),
                    input: json!({"path": "a.txt", "content": "x"}),
                }],
            },
            ScriptStep::WaitAbort,
        ],
    })]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    // tool が実際に走った(mutation landed)→ PARTIAL_MUTATION で wrap、
    // cause は MAX_TURNS_EXCEEDED。
    assert_eq!(err.code(), ErrorCode::PartialMutation);
    let cause = err
        .error()
        .source()
        .and_then(|s| s.downcast_ref::<CheaperError>())
        .expect("cause is CheaperError");
    assert_eq!(cause.code(), ErrorCode::MaxTurnsExceeded);
    assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "x");
    assert_eq!(err.run_meta().unwrap().tool_calls, Some(1));
}

/// T6(issue #111): budget 枯渇 abort は checkpoint を
/// `<userConfigDir>/checkpoints/` へ保存し、envelope 最外層 error
/// (mutation 経路では `PARTIAL_MUTATION` wrapper)の
/// `details.checkpoint = {path, observationsCount}` を付ける。
#[tokio::test]
async fn budget_abort_saves_checkpoint_and_surfaces_details() {
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};

    let mut p = policy();
    p.execution.max_turns = Some(1);
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession {
        steps: vec![
            ScriptStep::Turn { tool_calls: true },
            ScriptStep::ToolCalls {
                calls: vec![ToolCallSpec {
                    name: "write".into(),
                    input: json!({"path": "a.txt", "content": "x"}),
                }],
            },
            ScriptStep::WaitAbort,
        ],
    })]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::PartialMutation);
    // envelope 最外層(wrapper)に checkpoint details があること —
    // cause 側(inner MAX_TURNS_EXCEEDED)だけでは envelope 直列化で
    // 落ちてしまう(issue #111 review の P1 指摘)。
    let checkpoint = &err.details().unwrap()["checkpoint"];
    let path = checkpoint["path"].as_str().expect("checkpoint.path");
    assert_eq!(checkpoint["observationsCount"], json!(1));
    // 既存 keys(changes)は保持される。
    assert!(err.details().unwrap()["changes"].is_object());

    // checkpoint file の内容を検証する。
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved["schemaVersion"], json!(1));
    assert_eq!(saved["worker"], json!("test-worker"));
    assert_eq!(saved["abortReason"], json!("maxTurns"));
    assert_eq!(saved["turns"], json!(1));
    assert_eq!(saved["toolCalls"], json!(1));
    // input はそのまま保持される(resume 時の default input)。
    assert!(saved["input"].is_object());
    // ToolCommitted(hook patch 後 args) + ToolResult が join された
    // 観測 — write call の args と実行結果抜粋。
    let obs = &saved["observations"][0];
    assert_eq!(obs["tool"], json!("write"));
    assert_eq!(obs["args"], json!({"path": "a.txt", "content": "x"}));
    assert!(obs["resultExcerpt"].is_string());
}

/// T6b(issue #111): 再開可能な状態を持たない abort(timeout・signal)は
/// checkpoint を保存しない — `details.checkpoint` は付かない。
#[tokio::test]
async fn timeout_abort_saves_no_checkpoint() {
    let mut p = policy();
    p.execution.timeout_ms = 60;
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Hang]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Timeout);
    // timeout には部分進行を再開する材料が無い — checkpoint 非対象。
    assert!(err
        .details()
        .and_then(|d| d.get("checkpoint").cloned())
        .is_none());
}

// ---------------------------------------------------------------------------
// issue #111 — `--resume`(checkpoint 継続注入 / 連鎖 resume)
// ---------------------------------------------------------------------------

/// T7: `deps.resume` 指定の run は前回観測・counter を user prompt 末尾へ
/// 注入し(continuation section)、新規 session で完走する。
#[tokio::test]
async fn resume_injects_prior_observations_into_prompt() {
    use cheaper_runtime::run::checkpoint::ResumeCheckpoint;

    let fx = fixture(policy());
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: format!("```json\n{OK_OUTPUT}\n```"),
        usage: usage(10, 5),
        turns: 1,
        last_turn_tool_calls: false,
        transient_retries: 0,
    }]);
    let prior = ResumeCheckpoint {
        abort_reason: "maxTurns".to_string(),
        turns: 8,
        tool_calls: 3,
        observations: vec![json!({
            "tool": "read",
            "args": {"path": "a.txt"},
            "resultExcerpt": "alpha"
        })],
    };
    let mut d = deps(&fx, &factory);
    d.resume = Some(&prior);
    let ok = run_worker(&d).await.unwrap();
    assert_eq!(ok.result["ok"], json!(true));
    let prompt = &factory.prompts()[0];
    assert!(prompt.contains("interrupted by maxTurns after 8 turns and 3 tool calls"));
    assert!(prompt.contains("\"tool\": \"read\""));
    assert!(prompt.contains("Continue the task from this state"));
}

/// T8: 連鎖 resume — resume run が再度 `MaxTurns` で abort した場合、
/// 新 checkpoint は前回観測 + 新規観測を累積して保存する。
#[tokio::test]
async fn chained_resume_accumulates_observations() {
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};
    use cheaper_runtime::run::checkpoint::ResumeCheckpoint;

    let mut p = policy();
    p.execution.max_turns = Some(1);
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession {
        steps: vec![
            ScriptStep::Turn { tool_calls: true },
            ScriptStep::ToolCalls {
                calls: vec![ToolCallSpec {
                    name: "read".into(),
                    input: json!({"path": "a.txt"}),
                }],
            },
            ScriptStep::WaitAbort,
        ],
    })]);
    let prior = ResumeCheckpoint {
        abort_reason: "maxTurns".to_string(),
        turns: 8,
        tool_calls: 3,
        observations: vec![
            json!({"tool":"read","args":{"path":"old.txt"},"resultExcerpt":"o1"}),
            json!({"tool":"write","args":{"path":"w.txt"},"resultExcerpt":"o2"}),
        ],
    };
    let mut d = deps(&fx, &factory);
    d.resume = Some(&prior);
    let err = run_worker(&d).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::MaxTurnsExceeded);
    // observationsCount = prior(2) + 新規(1)。
    assert_eq!(
        err.details().unwrap()["checkpoint"]["observationsCount"],
        json!(3)
    );
    let path = err.details().unwrap()["checkpoint"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let obs = saved["observations"].as_array().unwrap();
    assert_eq!(obs.len(), 3);
    // 累積順 — 前回観測が先、今回の新規観測が後。
    assert_eq!(obs[0]["resultExcerpt"], json!("o1"));
    assert_eq!(obs[1]["resultExcerpt"], json!("o2"));
    assert_eq!(obs[2]["tool"], json!("read"));
}

#[tokio::test]
async fn rolled_back_turn_counts_toward_max_turns() {
    use cheaper_rig_adapter::RunEvent;
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};

    // F6: invalid-tool retry で rollback された turn も完了 model
    // call として `turns` に数える(Pi `turn_end` parity)。さもないと
    // invalid-call ループが `maxTurns` 分類を回避する。fake では
    // `RolledBackTurn` step が adapter hook と同じ計上点を replay
    // する — `turns` 計上 + `last_turn_tool_calls = true`。
    let mut p = policy();
    p.execution.max_turns = Some(1);
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession {
        steps: vec![
            // retry の feedback を受けた次の model call が item を emit
            // した時点で deferred check が走り MAX_TURNS_EXCEEDED。
            ScriptStep::RolledBackTurn,
            ScriptStep::Emit {
                event: RunEvent::TextDelta { text: "x".into() },
            },
            ScriptStep::WaitAbort,
        ],
    })]);
    let err = run_worker(&deps(&fx, &factory)).await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::MaxTurnsExceeded);
    assert_eq!(err.details().unwrap()["reason"], json!("maxTurns"));
    assert_eq!(err.run_meta().unwrap().turns, Some(1));
}

#[tokio::test]
async fn max_turns_still_aborts_without_tool_calls_in_final_turn() {
    // F1 の対称: 最終 turn が text 終了(last_turn_tool_calls=false)
    // なら tool batch は存在せず、従来通り即座に分類 — ただし
    // `stopReason === "toolUse"` 条件が必須なので text 終了で
    // maxTurns 到達は abort しない(TS: turn_end の toolUse check)。
    let mut p = policy();
    p.execution.max_turns = Some(1);
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: OK_OUTPUT.into(),
        usage: usage(10, 1),
        turns: 1,
        last_turn_tool_calls: false,
        transient_retries: 0,
    }]);
    // text 終了の turn は tool batch を持たない → run は成功する。
    let ok = run_worker(&deps(&fx, &factory)).await.unwrap();
    assert_eq!(ok.result["ok"], json!(true));
}

#[cfg(unix)]
#[tokio::test]
async fn filesystem_symlinks_policy_reaches_tool_scope() {
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};

    // F5: `policy.filesystem.symlinks` が worker scope の tool runtime
    // へ届くこと — Deny では workspace 内 symlink の read が拒否され、
    // WithinWorkspace では受理される。
    for (symlinks, expected_rejected) in [
        (SymlinkPolicy::Deny, 1u64),
        (SymlinkPolicy::WithinWorkspace, 0u64),
    ] {
        let mut p = policy();
        p.filesystem.symlinks = symlinks;
        let fx = fixture(p);
        std::fs::write(fx.root.join("real.txt"), "hello").unwrap();
        std::os::unix::fs::symlink(fx.root.join("real.txt"), fx.root.join("link.txt")).unwrap();
        let factory = FakeSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession {
            steps: vec![
                ScriptStep::ToolCalls {
                    calls: vec![ToolCallSpec {
                        name: "read".into(),
                        input: json!({"path": "link.txt"}),
                    }],
                },
                ScriptStep::Finish {
                    text: OK_OUTPUT.into(),
                },
            ],
        })]);
        let ok = run_worker(&deps(&fx, &factory)).await.unwrap();
        assert_eq!(
            ok.meta.rejected_tool_calls, expected_rejected,
            "symlinks={symlinks:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// optionalTools degrade — prompt surfacing(design §(f)、test-spec AC4/AC8)
// ---------------------------------------------------------------------------

/// empty tasks/grants で opt-in した `process`/`command` が縮退する
/// run は "Available tools" を実登録集合から組み立て、利用不可注記と
/// (`status` enum が受理する場合の) blocked 誘導行を prompt に載せる。
/// `FakeSessionFactory` の `RecordedSpec.system_prompt` で文言を固定する。
#[tokio::test]
async fn degraded_run_surfaces_unavailable_tools_and_blocked_hint() {
    let mut p = policy();
    p.tools.allow = vec![ToolName::Read, ToolName::Process, ToolName::Command];
    let mut fx = fixture(p);
    fx.worker = opt_in_worker_definition(&fx.root, true);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: r#"{"status":"success"}"#.into(),
        usage: usage(10, 1),
        turns: 1,
        last_turn_tool_calls: false,
        transient_retries: 0,
    }]);
    let ok = run_worker(&deps(&fx, &factory)).await.unwrap();
    assert_eq!(ok.result["status"], json!("success"));

    let prompt = &factory.created()[0].system_prompt;
    // 実登録集合 — 縮退した process/command は列挙されず、直後に
    // 宣言順(process → command)の利用不可注記が続く。
    assert!(
        prompt.contains(
            "- Available tools: read\n\
             - Declared but unavailable under current policy: process, command"
        ),
        "{prompt}"
    );
    // output schema が `status: "blocked"` を受理するため誘導行も emit。
    assert!(
        prompt.contains(
            "- If the task requires an unavailable tool, do not call it; \
             finish with status \"blocked\" and list the missing capability \
             in unresolved and nextActions."
        ),
        "{prompt}"
    );
}

/// `status` enum に `"blocked"` を持たない opt-in worker では利用
/// 不可列挙行のみ emit され blocked 誘導行は出ない — schema が
/// 受理しない終了状態を prompt が促さない(test-spec Example)。
#[tokio::test]
async fn degraded_run_omits_blocked_hint_when_schema_lacks_blocked() {
    let mut p = policy();
    p.tools.allow = vec![ToolName::Read, ToolName::Process, ToolName::Command];
    let mut fx = fixture(p);
    fx.worker = opt_in_worker_definition(&fx.root, false);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: r#"{"status":"success"}"#.into(),
        usage: usage(10, 1),
        turns: 1,
        last_turn_tool_calls: false,
        transient_retries: 0,
    }]);
    run_worker(&deps(&fx, &factory)).await.unwrap();

    let prompt = &factory.created()[0].system_prompt;
    assert!(
        prompt.contains("- Declared but unavailable under current policy: process, command"),
        "{prompt}"
    );
    assert!(!prompt.contains("status \"blocked\""), "{prompt}");
}

#[tokio::test]
async fn policy_max_turns_flows_to_session_spec() {
    // F7: `policy.execution.maxTurns` が `SessionSpec.policy_max_turns`
    // として session factory へ届く(main + repair 双方)。内部 guard
    // `max(1024, policy)` の算出は `RigSessionFactory` 側(単体 test は
    // run.rs 内) — ここでは policy 値の wiring を検証する。
    let mut p = policy();
    p.execution.max_turns = Some(2048);
    let fx = fixture(p);
    let factory = FakeSessionFactory::new(vec![
        // 1 本目を extract 失敗させて repair session も生成させる。
        FakeResponse::Complete {
            text: "bad".into(),
            usage: usage(10, 1),
            turns: 1,
            last_turn_tool_calls: false,
            transient_retries: 0,
        },
        FakeResponse::Complete {
            text: OK_OUTPUT.into(),
            usage: usage(10, 1),
            turns: 1,
            last_turn_tool_calls: false,
            transient_retries: 0,
        },
    ]);
    run_worker(&deps(&fx, &factory)).await.unwrap();
    let created = factory.created();
    assert_eq!(created.len(), 2);
    assert_eq!(created[0].policy_max_turns, Some(2048));
    assert_eq!(created[1].policy_max_turns, Some(2048));
}

/// mixed policy — `process.tasks` 非空 × `command.grants` 空では
/// "Available tools" に `process` が残り、利用不可注記は failed-gate
/// の `command` のみ列挙する(縮退は per-tool — test-spec Rule)。
#[tokio::test]
async fn degraded_run_prompt_lists_only_the_failed_gate_tool() {
    let mut p = policy();
    p.tools.allow = vec![ToolName::Read, ToolName::Process, ToolName::Command];
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    p.process.tasks.insert(
        "probe".to_string(),
        json!({"command": [exe], "inputs": [], "outputs": [], "cwd": ".", "env": {}}),
    );
    let mut fx = fixture(p);
    fx.worker = opt_in_worker_definition(&fx.root, true);
    let factory = FakeSessionFactory::new(vec![FakeResponse::Complete {
        text: r#"{"status":"success"}"#.into(),
        usage: usage(10, 1),
        turns: 1,
        last_turn_tool_calls: false,
        transient_retries: 0,
    }]);
    let ok = run_worker(&deps(&fx, &factory)).await.unwrap();
    assert_eq!(ok.result["status"], json!("success"));

    let prompt = &factory.created()[0].system_prompt;
    // 実登録集合 — `process` は gate 通過で残る。
    assert!(
        prompt.contains("- Available tools: read, process"),
        "{prompt}"
    );
    // 利用不可注記は failed-gate の `command` のみ。
    assert!(
        prompt.contains("- Declared but unavailable under current policy: command"),
        "{prompt}"
    );
    assert!(
        !prompt.contains("unavailable under current policy: process"),
        "{prompt}"
    );
}

// ---------------------------------------------------------------------------
// degraded / undeclared call → UNKNOWN_TOOL — stream event での literal
// assert(envelope には載らないため session seam 経由で `ToolResult` を
// 記録する — test-spec「degraded/undeclared tool call は UNKNOWN_TOOL」)。
// ---------------------------------------------------------------------------

/// `RunEvent::ToolResult` の `output` を記録する `WorkerSession`
/// wrapper — fake provider が `tool_runtime.execute` 経由で返す error
/// result の literal content を検証する。
struct RecordingSession {
    inner: Arc<dyn WorkerSession>,
    tool_results: Arc<std::sync::Mutex<Vec<Value>>>,
}

impl WorkerSession for RecordingSession {
    fn stream_run(
        &self,
        prompt: String,
        signal: Option<cheaper_tools::operation::AbortSignal>,
    ) -> cheaper_rig_adapter::RunStream {
        let sink = Arc::clone(&self.tool_results);
        Box::pin(self.inner.stream_run(prompt, signal).inspect(move |item| {
            if let Ok(RunEvent::ToolResult { output, .. }) = item {
                sink.lock().unwrap().push(output.clone());
            }
        }))
    }
    fn abort(&self) {
        self.inner.abort();
    }
    fn snapshot(&self) -> cheaper_rig_adapter::RunSnapshot {
        self.inner.snapshot()
    }
}

struct RecordingSessionFactory {
    inner: FakeSessionFactory,
    tool_results: Arc<std::sync::Mutex<Vec<Value>>>,
}

impl RecordingSessionFactory {
    fn new(responses: Vec<FakeResponse>) -> Self {
        Self {
            inner: FakeSessionFactory::new(responses),
            tool_results: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

impl SessionFactory for RecordingSessionFactory {
    fn create_session(
        &self,
        spec: &cheaper_runtime::run::SessionSpec<'_>,
    ) -> Result<Arc<dyn WorkerSession>, CheaperError> {
        let inner = self.inner.create_session(spec)?;
        Ok(Arc::new(RecordingSession {
            inner,
            tool_results: Arc::clone(&self.tool_results),
        }))
    }
}

/// 縮退した opt-in tool(`command`/`process`)と worker が宣言して
/// いない tool(`write`)への call は、fake provider が `tool_runtime.
/// execute` 経由で流す `ToolResult` の `{"error": "unknown or
/// unregistered tool: <name>"}` として安全に失敗し、run 自体は完走
/// する。いずれも metering/accounting に到達しない(`toolCalls == 0`)。
#[tokio::test]
async fn degraded_and_undeclared_calls_fail_unknown_tool() {
    use cheaper_runtime::dev::fake_provider::{ScriptStep, ScriptedSession};

    let mut p = policy();
    p.tools.allow = vec![
        ToolName::Read,
        ToolName::Process,
        ToolName::Command,
        ToolName::Write,
    ];
    let mut fx = fixture(p);
    fx.worker = opt_in_worker_definition(&fx.root, true);
    let factory = RecordingSessionFactory::new(vec![FakeResponse::Scripted(ScriptedSession {
        steps: vec![
            ScriptStep::ToolCalls {
                calls: vec![
                    ToolCallSpec {
                        name: "command".into(),
                        input: json!({"name": "self", "args": []}),
                    },
                    ToolCallSpec {
                        name: "process".into(),
                        input: json!({"task": "probe"}),
                    },
                    ToolCallSpec {
                        name: "write".into(),
                        input: json!({"path": "x.txt", "content": "x"}),
                    },
                ],
            },
            ScriptStep::Finish {
                text: r#"{"status":"blocked"}"#.into(),
            },
        ],
    })]);
    let deps = RunWorkerDeps {
        session_factory: &factory,
        invocation_grants: &[],
        resume: None,
        ..deps(&fx, &factory.inner)
    };
    let ok = run_worker(&deps).await.unwrap();
    assert_eq!(ok.result["status"], json!("blocked"));
    assert_eq!(ok.meta.tool_calls, 0);

    let results = factory.tool_results.lock().unwrap();
    assert_eq!(results.len(), 3, "{results:?}");
    for (name, output) in [
        ("command", &results[0]),
        ("process", &results[1]),
        ("write", &results[2]),
    ] {
        assert_eq!(
            output["error"].as_str().unwrap_or(""),
            format!("unknown or unregistered tool: {name}"),
            "{name}: {output}"
        );
    }
    assert!(!fx.root.join("x.txt").exists());
}
