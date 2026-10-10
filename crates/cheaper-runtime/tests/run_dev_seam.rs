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
    ExecutionPolicy, FilesystemPolicy, ModelPolicy, OutputPolicy, RetryPolicy, RuntimePolicy,
    TierFallback, ToolsPolicy,
};
use cheaper_core::policy::workflow::{WorkflowArtifactPolicy, WorkflowPolicy};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::parse::parse_worker_file;
use cheaper_core::worker::validate::{validate_worker_definition, WorkerOrigin};
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_rig_adapter::UsageSnapshot;
use cheaper_runtime::auth::api_key::ProviderEnv;
use cheaper_runtime::catalog::schema::{ModelCostJson, ModelDefinitionJson};
use cheaper_runtime::fake_provider::{FakeResponse, FakeSessionFactory, ToolCallSpec};
use cheaper_runtime::plugin::{
    CheaperProviderPlugin, PluginProviderRegistration, ProviderPluginContext, ProviderRegistry,
};
use cheaper_runtime::run::{run_worker, RunWorkerDeps, RunWorkerOk};
use cheaper_runtime::Runtime;
use cheaper_tools::operation::{AbortController, AbortReason};
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
        max_turns: 10,
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

#[tokio::test]
async fn max_turns_classifies_max_turns_exceeded() {
    let mut p = policy();
    p.execution.max_turns = 2;
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
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
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
    use cheaper_runtime::fake_provider::{ScriptStep, ScriptedSession};

    // F1: maxTurns に達した turn が tool call で終わった場合、その
    // pending batch の実行・`ToolResult` emit を待ってから abort する。
    // 即 abort だと Rig の `drive_tool_calls` が batch 途中で drop され、
    // 最終 turn の tool 副作用・結果 event・usage が欠落する。
    // `ScriptStep::ToolCalls` は Rig と同じ emit 順序を再現する —
    // `ToolCall` event → batch 実行 → `ToolResult` event。
    let mut p = policy();
    p.execution.max_turns = 1;
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

#[tokio::test]
async fn rolled_back_turn_counts_toward_max_turns() {
    use cheaper_rig_adapter::RunEvent;
    use cheaper_runtime::fake_provider::{ScriptStep, ScriptedSession};

    // F6: invalid-tool retry で rollback された turn も完了 model
    // call として `turns` に数える(Pi `turn_end` parity)。さもないと
    // invalid-call ループが `maxTurns` 分類を回避する。fake では
    // `RolledBackTurn` step が adapter hook と同じ計上点を replay
    // する — `turns` 計上 + `last_turn_tool_calls = true`。
    let mut p = policy();
    p.execution.max_turns = 1;
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
    p.execution.max_turns = 1;
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
    use cheaper_runtime::fake_provider::{ScriptStep, ScriptedSession};

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

#[tokio::test]
async fn policy_max_turns_flows_to_session_spec() {
    // F7: `policy.execution.maxTurns` が `SessionSpec.policy_max_turns`
    // として session factory へ届く(main + repair 双方)。内部 guard
    // `max(1024, policy)` の算出は `RigSessionFactory` 側(単体 test は
    // run.rs 内) — ここでは policy 値の wiring を検証する。
    let mut p = policy();
    p.execution.max_turns = 2048;
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
    assert_eq!(created[0].policy_max_turns, 2048);
    assert_eq!(created[1].policy_max_turns, 2048);
}
