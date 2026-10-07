//! `cheaper-rig-adapter` の integration test。
//!
//! Phase 0 PoC (`crates/poc-rig` の scripted probes)を `stream_run` の
//! adapter 公開面上で再現する。`MockCompletionModel` の streaming turn
//! (`MockStreamEvent`)で deterministic に turn/error/abort を script する。
//!
//! ここでの Rig 型使用は adapter 境界の内側(test)なので isolation
//! contract に反しない — 検証対象は公開 API が `RunEvent`/`RunSnapshot`/
//! `CheaperError` で応答すること。

use std::path::PathBuf;

use futures::StreamExt;
use http::StatusCode;
use serde_json::{json, Value};

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_rig_adapter::{
    abort_reason_to_cheaper, build_system_prompt, build_user_prompt, classify, Accumulator,
    CompletionModel, ErrorClass, RigSession, RunEvent, UsageSnapshot, WorkerSessionSpec,
};
use cheaper_tools::operation::{AbortController, AbortReason};
use cheaper_tools::registry::{create_tool_runtime, ToolRuntimeOptions};

use rig::completion::request::FinishReason;
use rig::completion::Usage;
use rig::operation::Finish;
use rig::test_utils::{MockCompletionModel, MockError, MockStreamEvent};
use rig::ProviderResponseError;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn usage(input: u64, output: u64) -> Usage {
    Usage {
        input_tokens: Some(input),
        output_tokens: Some(output),
        total_tokens: Some(input + output),
        ..Default::default()
    }
}

fn text_turn(text: &str, input: u64, output: u64) -> Vec<MockStreamEvent> {
    vec![
        MockStreamEvent::text(text),
        MockStreamEvent::FinalResponse(Finish {
            usage: usage(input, output),
            reason: Some(FinishReason::Stop),
            ..Default::default()
        }),
    ]
}

fn session(model: MockCompletionModel, transient_retries: usize) -> RigSession {
    RigSession::build(WorkerSessionSpec {
        model: CompletionModel::from_rig(model.erase()),
        system_prompt: "test worker".to_string(),
        tool_runtime: None,
        max_output_tokens: 1024,
        max_turns: cheaper_rig_adapter::DEFAULT_MAX_TURNS,
        invalid_tool_retries: cheaper_rig_adapter::DEFAULT_INVALID_TOOL_RETRIES,
        transient_retries,
    })
}

async fn collect(stream: cheaper_rig_adapter::RunStream) -> Vec<Result<RunEvent, CheaperError>> {
    stream.collect::<Vec<_>>().await
}

// ---------------------------------------------------------------------------
// accumulator / classification unit-level checks
// ---------------------------------------------------------------------------

#[test]
fn accumulator_counts_and_reconciles() {
    let acc = Accumulator::default();
    acc.record_completion(&UsageSnapshot::from(&usage(100, 10)));
    acc.record_turn(false);
    acc.record_turn(true);
    acc.record_invalid_tool_call();
    acc.record_transient_retry();
    // rollback で hook を抜けた turn の課金分を settle 時に reconcile。
    acc.reconcile_usage(&UsageSnapshot::from(&usage(250, 15)));

    let snap = acc.snapshot();
    assert_eq!(snap.usage.input_tokens, 250);
    assert_eq!(snap.usage.output_tokens, 15);
    assert_eq!(snap.usage.total_tokens, 265);
    assert_eq!(snap.turns, 2);
    assert_eq!(snap.invalid_tool_calls, 1);
    assert_eq!(snap.transient_retries, 1);
    assert!(snap.last_turn_tool_calls);
}

#[test]
fn classify_maps_provider_errors() {
    // HTTP 429 → rate limit (transient)。
    let rate_limited = rig::error::ProviderError::ProviderResponse(ProviderResponseError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "slow down",
    ));
    assert_eq!(classify(&rate_limited), ErrorClass::TransientRateLimit);

    // HTTP 5xx → server (transient)。
    let server = rig::error::ProviderError::ProviderResponse(ProviderResponseError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "boom",
    ));
    assert_eq!(classify(&server), ErrorClass::TransientServer);

    // HTTP 4xx(429 以外) → permanent。
    let bad_request = rig::error::ProviderError::ProviderResponse(ProviderResponseError::new(
        StatusCode::BAD_REQUEST,
        "bad",
    ));
    assert_eq!(classify(&bad_request), ErrorClass::Permanent);

    // 応答を持たない transport 失敗 → transport (transient)。
    let transport =
        rig::error::ProviderError::Http(std::sync::Arc::new(rig::http_client::Error::StreamEnded));
    assert_eq!(classify(&transport), ErrorClass::TransientTransport);

    // request 構築失敗 → permanent。
    let request = rig::error::ProviderError::request("malformed request");
    assert_eq!(classify(&request), ErrorClass::Permanent);
}

#[test]
fn abort_reason_carries_cheaper_code() {
    // T3 の policy abort 経路: reason が CheaperError 由来 code を保持し、
    // adapter が元の code へ復元する(maxTurns → MAX_TURNS_EXCEEDED)。
    let reason = AbortReason::from_error(Box::new(CheaperError::new(
        ErrorCode::MaxTurnsExceeded,
        "worker hit policy maxTurns",
    )));
    let error = abort_reason_to_cheaper(&reason);
    assert_eq!(error.code(), ErrorCode::MaxTurnsExceeded);

    // code を持たない汎用 reason は ABORTED。
    let plain = abort_reason_to_cheaper(&AbortReason::aborted());
    assert_eq!(plain.code(), ErrorCode::Aborted);
}

// ---------------------------------------------------------------------------
// streaming run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_run_emits_deltas_and_final() {
    let session = session(
        MockCompletionModel::from_stream_turns([text_turn("hello world", 10, 5)]),
        0,
    );
    let events = collect(session.stream_run("hi", None)).await;

    let texts: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            Ok(RunEvent::TextDelta { text }) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec!["hello world".to_string()]);

    let finished = events.iter().find_map(|e| match e {
        Ok(RunEvent::Finished { output }) => Some(output.clone()),
        _ => None,
    });
    assert_eq!(finished.as_deref(), Some("hello world"));

    // usage は per-call report + settle reconcile で run total に一致する。
    assert!(events.iter().all(|e| e.is_ok()));
    let snap = session.snapshot();
    assert_eq!(snap.turns, 1);
    assert_eq!(snap.usage.input_tokens, 10);
    assert_eq!(snap.usage.output_tokens, 5);
    assert_eq!(snap.tool_calls, 0);
}

#[tokio::test]
async fn invalid_tool_call_recovers_and_reconciles_usage() {
    let model = MockCompletionModel::from_stream_turns([
        // turn 1: 未登録 tool call(usage 100/10 が課金されるが turn は
        // rollback され ModelTurnFinished が発火しない)。
        vec![
            MockStreamEvent::tool_call("call-1", "does_not_exist", json!({})),
            MockStreamEvent::FinalResponse(Finish {
                usage: usage(100, 10),
                reason: Some(FinishReason::ToolCalls),
                ..Default::default()
            }),
        ],
        // turn 2: corrective feedback を受けて text で回復。
        text_turn("recovered without tools", 150, 5),
    ]);
    let session = session(model.clone(), 0);
    let events = collect(session.stream_run("use a tool", None)).await;

    assert!(events.iter().all(|e| e.is_ok()), "events: {events:?}");
    let snap = session.snapshot();
    // recovery: invalid call を観測し retry feedback で 2 回目が受理。
    assert_eq!(snap.invalid_tool_calls, 1);
    assert_eq!(model.request_count(), 2);
    // rolled-back turn は accepted turn に数えないが課金は残る。
    assert_eq!(snap.turns, 1);
    assert_eq!(snap.tool_calls, 0);
    assert_eq!(snap.rejected_tool_calls, 0);
    assert_eq!(snap.usage.input_tokens, 250);
    let finished = events.iter().find_map(|e| match e {
        Ok(RunEvent::Finished { output }) => Some(output.clone()),
        _ => None,
    });
    assert_eq!(finished.as_deref(), Some("recovered without tools"));
}

#[tokio::test]
async fn transient_server_error_retries_completion_call() {
    let model = MockCompletionModel::from_stream_turns([
        // turn 1: provider が reply を開く前に 500 — retry 可能。
        vec![MockStreamEvent::Error(MockError::ProviderResponse(
            ProviderResponseError::new(StatusCode::INTERNAL_SERVER_ERROR, "boom"),
        ))],
        text_turn("recovered", 20, 4),
    ]);
    let session = session(model.clone(), 2);
    let events = collect(session.stream_run("hi", None)).await;

    assert!(events.iter().all(|e| e.is_ok()), "events: {events:?}");
    let snap = session.snapshot();
    // completion call 単位の retry が 1 回発火し run は成功する。
    assert_eq!(snap.transient_retries, 1);
    assert_eq!(model.request_count(), 2);
    assert_eq!(snap.turns, 1);
    assert_eq!(snap.usage.input_tokens, 20);
}

#[tokio::test]
async fn transient_retry_budget_exhaustion_surfaces_last_error() {
    let fail = || {
        MockStreamEvent::Error(MockError::ProviderResponse(ProviderResponseError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "boom",
        )))
    };
    let model = MockCompletionModel::from_stream_turns([vec![fail()], vec![fail()], vec![fail()]]);
    let session = session(model.clone(), 2);
    let events = collect(session.stream_run("hi", None)).await;

    // retry 2 回 + 初回 = 3 呼出しで尽き、最後の provider error が残る。
    assert_eq!(model.request_count(), 3);
    let error = events.iter().find_map(|e| e.as_ref().err());
    let error = error.expect("run must fail");
    assert_eq!(error.code(), ErrorCode::ProviderFailure);
    let snap = session.snapshot();
    assert_eq!(snap.transient_retries, 2);
}

#[tokio::test]
async fn permanent_error_fails_without_retry() {
    let model = MockCompletionModel::from_stream_turns([vec![MockStreamEvent::Error(
        MockError::ProviderResponse(ProviderResponseError::new(
            StatusCode::BAD_REQUEST,
            "bad request",
        )),
    )]]);
    let session = session(model.clone(), 4);
    let events = collect(session.stream_run("hi", None)).await;

    assert_eq!(model.request_count(), 1);
    let error = events.iter().find_map(|e| e.as_ref().err());
    let error = error.expect("run must fail");
    assert_eq!(error.code(), ErrorCode::ProviderFailure);
    assert_eq!(session.snapshot().transient_retries, 0);
}

// ---------------------------------------------------------------------------
// tool bridge(実 ToolRuntime 経由 — tempdir の read scope)
// ---------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-rig-adapter-it-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }

    fn root(&self) -> String {
        canonicalize_workspace_root(self.0.to_str().unwrap()).unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn read_scope(root: &str) -> cheaper_tools::registry::ToolRuntime {
    create_tool_runtime(
        ToolRuntimeOptions {
            resource_manifest: None,
            write_scope: None,
            read_strictness: None,
            artifacts: None,
            process: None,
            root: root.to_string(),
            symlink_policy: SymlinkPolicy::Deny,
            names: vec![ToolName::Read],
            max_tool_calls: 20,
            on_limit: Box::new(|| {}),
            journal: None,
            denylist: None,
            parent_lease: None,
            signal: None,
            abort_grace_ms: None,
            concurrency: None,
            eval_spawn: None,
        },
        None,
    )
    .expect("create_tool_runtime")
}

fn read_call_turn(calls: &[(&str, Value)], input: u64, output: u64) -> Vec<MockStreamEvent> {
    let mut events: Vec<MockStreamEvent> = calls
        .iter()
        .map(|(id, arguments)| MockStreamEvent::tool_call(*id, "read", arguments.clone()))
        .collect();
    events.push(MockStreamEvent::FinalResponse(Finish {
        usage: usage(input, output),
        reason: Some(FinishReason::ToolCalls),
        ..Default::default()
    }));
    events
}

#[tokio::test]
async fn tool_call_executes_through_cheaper_tools() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("hello.txt"), "hello cheaper").unwrap();
    let runtime = read_scope(&dir.root());

    let model = MockCompletionModel::from_stream_turns([
        read_call_turn(&[("c1", json!({"path": "hello.txt"}))], 30, 8),
        text_turn("done", 40, 6),
    ]);
    let session = RigSession::build(WorkerSessionSpec {
        model: CompletionModel::from_rig(model.erase()),
        system_prompt: build_system_prompt(
            "test worker",
            &json!({"type": "object"}),
            &["read".to_string()],
        ),
        tool_runtime: Some(runtime),
        max_output_tokens: 1024,
        max_turns: cheaper_rig_adapter::DEFAULT_MAX_TURNS,
        invalid_tool_retries: cheaper_rig_adapter::DEFAULT_INVALID_TOOL_RETRIES,
        transient_retries: 0,
    });
    let events = collect(session.stream_run("read the file", None)).await;
    assert!(events.iter().all(|e| e.is_ok()), "events: {events:?}");

    // ToolCall → ToolCommitted → ToolResult の順序で観測され、結果には
    // file の中身が載る。
    let tool_call = events
        .iter()
        .any(|e| matches!(e, Ok(RunEvent::ToolCall { name, .. }) if name == "read"));
    let committed = events
        .iter()
        .any(|e| matches!(e, Ok(RunEvent::ToolCommitted { name, .. }) if name == "read"));
    let result_output = events.iter().find_map(|e| match e {
        Ok(RunEvent::ToolResult { name, output, .. }) if name == "read" => Some(output.clone()),
        _ => None,
    });
    assert!(tool_call && committed);
    let output = result_output.expect("read tool result");
    assert!(
        output.to_string().contains("hello cheaper"),
        "tool output: {output}"
    );

    let snap = session.snapshot();
    assert_eq!(snap.tool_calls, 1);
    assert_eq!(snap.rejected_tool_calls, 0);
    assert_eq!(snap.turns, 2);
    assert_eq!(snap.usage.input_tokens, 70);
    assert!(!snap.last_turn_tool_calls);
}

#[tokio::test]
async fn access_denied_tool_call_counts_rejected() {
    let dir = TempDir::new();
    let runtime = read_scope(&dir.root());

    // workspace 外参照は cheaper-tools が access denied で拒否する —
    // TS contract では `toolCalls`(admission)と `rejectedToolCalls` の両方
    // に計上される。
    let model = MockCompletionModel::from_stream_turns([
        read_call_turn(&[("c1", json!({"path": "../escape.txt"}))], 30, 8),
        text_turn("denied, answering directly", 40, 6),
    ]);
    let session = RigSession::build(WorkerSessionSpec {
        model: CompletionModel::from_rig(model.erase()),
        system_prompt: "test worker".to_string(),
        tool_runtime: Some(runtime),
        max_output_tokens: 1024,
        max_turns: cheaper_rig_adapter::DEFAULT_MAX_TURNS,
        invalid_tool_retries: cheaper_rig_adapter::DEFAULT_INVALID_TOOL_RETRIES,
        transient_retries: 0,
    });
    let events = collect(session.stream_run("escape", None)).await;
    assert!(events.iter().all(|e| e.is_ok()), "events: {events:?}");

    let snap = session.snapshot();
    assert_eq!(snap.tool_calls, 1);
    assert_eq!(snap.rejected_tool_calls, 1);
}

#[tokio::test]
async fn same_turn_tool_calls_run_in_call_order() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "alpha").unwrap();
    std::fs::write(dir.0.join("b.txt"), "beta").unwrap();
    let runtime = read_scope(&dir.root());

    // tool_concurrency(1) — same-turn batch は call 順に結果が commit される。
    let model = MockCompletionModel::from_stream_turns([
        read_call_turn(
            &[
                ("c1", json!({"path": "a.txt"})),
                ("c2", json!({"path": "b.txt"})),
            ],
            30,
            8,
        ),
        text_turn("both read", 40, 6),
    ]);
    let session = RigSession::build(WorkerSessionSpec {
        model: CompletionModel::from_rig(model.erase()),
        system_prompt: "test worker".to_string(),
        tool_runtime: Some(runtime),
        max_output_tokens: 1024,
        max_turns: cheaper_rig_adapter::DEFAULT_MAX_TURNS,
        invalid_tool_retries: cheaper_rig_adapter::DEFAULT_INVALID_TOOL_RETRIES,
        transient_retries: 0,
    });
    let events = collect(session.stream_run("read both", None)).await;
    assert!(events.iter().all(|e| e.is_ok()), "events: {events:?}");

    let results: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            Ok(RunEvent::ToolResult { name, output, .. }) if name == "read" => {
                Some(output.to_string())
            }
            _ => None,
        })
        .collect();
    assert_eq!(results.len(), 2);
    assert!(results[0].contains("alpha"));
    assert!(results[1].contains("beta"));
    assert_eq!(session.snapshot().tool_calls, 2);
}

// ---------------------------------------------------------------------------
// cancellation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn aborted_signal_yields_terminal_error() {
    let controller = AbortController::new();
    let signal = controller.signal();
    let session = session(
        MockCompletionModel::from_stream_turns([text_turn("never", 10, 5)]),
        0,
    );
    controller.abort(AbortReason::aborted());

    let events = collect(session.stream_run("hi", Some(signal))).await;
    assert_eq!(events.len(), 1);
    let error = events[0].as_ref().expect_err("aborted run must fail");
    assert_eq!(error.code(), ErrorCode::Aborted);
}

#[tokio::test]
async fn mid_run_abort_yields_terminal_error_and_partial_snapshot() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "alpha").unwrap();
    let runtime = read_scope(&dir.root());

    let model = MockCompletionModel::from_stream_turns([
        read_call_turn(&[("c1", json!({"path": "a.txt"}))], 30, 8),
        text_turn("second turn", 40, 6),
    ]);
    let session = RigSession::build(WorkerSessionSpec {
        model: CompletionModel::from_rig(model.erase()),
        system_prompt: "test worker".to_string(),
        tool_runtime: Some(runtime),
        max_output_tokens: 1024,
        max_turns: cheaper_rig_adapter::DEFAULT_MAX_TURNS,
        invalid_tool_retries: cheaper_rig_adapter::DEFAULT_INVALID_TOOL_RETRIES,
        transient_retries: 0,
    });

    let controller = AbortController::new();
    let signal = controller.signal();
    let mut stream = session.stream_run("work", Some(signal));

    // turn 1 の tool result まで消費してから abort(T3 の policy abort
    // 経路と同じ code 付き reason)。
    while let Some(item) = stream.next().await {
        if matches!(item, Ok(RunEvent::ToolResult { .. })) {
            break;
        }
        assert!(item.is_ok(), "unexpected error: {item:?}");
    }
    controller.abort(AbortReason::from_error(Box::new(CheaperError::new(
        ErrorCode::MaxTurnsExceeded,
        "worker hit policy maxTurns",
    ))));

    // 終端 error は reason の code を引き継ぎ、Finished は来ない。
    let mut terminal: Option<CheaperError> = None;
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            terminal = Some(error);
        }
    }
    let error = terminal.expect("aborted run must end with an error");
    assert_eq!(error.code(), ErrorCode::MaxTurnsExceeded);

    // partial snapshot: settle 済みの turn 1 分が残る。
    let snap = session.snapshot();
    assert_eq!(snap.turns, 1);
    assert_eq!(snap.tool_calls, 1);
    assert_eq!(snap.usage.input_tokens, 30);
}

// ---------------------------------------------------------------------------
// prompt builders — runtime-pi `buildSystemPrompt`/`buildUserPrompt` parity
// ---------------------------------------------------------------------------

#[test]
fn prompts_match_ts_shape() {
    let prompt = build_system_prompt(
        "You are a worker.\n",
        &json!({"type": "object", "properties": {"answer": {"type": "string"}}}),
        &["read".to_string(), "grep".to_string()],
    );
    assert!(prompt.starts_with("You are a worker.\n\n## Output contract\n"));
    assert!(prompt.contains("- Available tools: read, grep"));
    // batching guidance は tool 非空時のみ emit。
    assert!(prompt.contains("Independent tool calls must be issued together"));
    assert!(prompt.contains("Output schema:\n{"));
    assert!(prompt.ends_with("}\n"));

    let none = build_system_prompt("w", &json!({}), &[]);
    assert!(none.contains("- Available tools: none"));
    assert!(!none.contains("Independent tool calls"));

    let user = build_user_prompt(&json!({"path": "src"}));
    assert_eq!(user, "Task input (JSON):\n{\n  \"path\": \"src\"\n}");
}
