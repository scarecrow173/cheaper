//! T3 最小 fake provider seam(testsupport.ts `createFakePi` 相当) +
//! T4 fixture DSL の scripted stream 層 + `cheaper-fake` catalog/auth 層。
//!
//! `SessionFactory`/`WorkerSession` を scripted response で実装する。
//! adapter(T4)の `stream_fn`-級 fake が来るまで、run orchestration の
//! abort/usage/meta/repair 分岐を決定的に検証するための test 用実装。
//!
//! fake session は `RigSession::build` と同じ計上経路を辿る —
//! session 固有の `Accumulator` を持ち、`tool_runtime` があれば
//! `budget()` を attach する(toolCalls/rejectedToolCalls が run snapshot
//! から読める)。
//!
//! T4 の 3 層 seam(G7):
//! - catalog 層: [`CheaperFakePlugin`] が `cheaper-fake/fake-model` を
//!   plugin registration 経由で catalog `get_available` に露出する。
//! - keyless credential 層: plugin `api_key` は常に解決する値(TS
//!   testsupport の `apiKey.resolve → {auth:{}}` 相当)。fixture は env
//!   template(`"$VAR"`)に変えて login→configured 経路も検証できる。
//! - scripted stream 層: [`ScriptedSession`]/[`ScriptStep`] が fixture DSL
//!   の primitive(`delay_ms`/`error{status,retryable}`/`hang_until_abort`/
//!   `abort_response`/`usage_delta`/tool calls/turns/output)を replay
//!   する。session seam は adapter 内の transient retry wrapper も含めて
//!   差し替えるため、scripted `error` の retry 分類(status/retryable →
//!   transient/permanent)は adapter `classify` の parity mirror を fake
//!   側で担う。

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_rig_adapter::{
    abort_reason_to_cheaper, Accumulator, RunEvent, RunSnapshot, RunStream, UsageSnapshot,
};
use cheaper_tools::operation::AbortSignal;
use cheaper_tools::registry::{ToolCallContext, ToolRuntime};
use futures::stream;
use serde_json::{json, Map, Value};

use crate::catalog::schema::{ModelCostJson, ModelDefinitionJson};
use crate::plugin::{
    CheaperProviderPlugin, PluginProviderRegistration, ProviderPluginContext, ProviderRegistry,
};
use crate::run::{SessionFactory, SessionSpec, WorkerSession};

/// scripted response 1 本(session 1 つ分の挙動)。
#[derive(Debug)]
pub enum FakeResponse {
    /// 正常完了 — usage/turns を計上し `Finished{output}` を emit。
    Complete {
        text: String,
        usage: UsageSnapshot,
        turns: u64,
        /// 最終 turn が tool 継続終了か(maxTurns 判定用)。
        last_turn_tool_calls: bool,
        transient_retries: u64,
    },
    /// raw event script — item 間で abort 判定を行い、abort 済なら
    /// code 付き abort error を終端 emit する(adapter と同じ shape)。
    Events {
        events: Vec<Result<RunEvent, CheaperError>>,
        usage: UsageSnapshot,
        turns: u64,
        last_turn_tool_calls: bool,
        transient_retries: u64,
    },
    /// 即終端 error(provider error / transient retry 尽き 相当)。
    Fail {
        error: CheaperError,
        usage: UsageSnapshot,
        turns: u64,
    },
    /// signal abort を待って code 付き abort error で終端
    /// (session が signal で cancel される通常経路)。
    WaitAbort { usage: UsageSnapshot, turns: u64 },
    /// signal を観測しない永久 pending — abort grace 経路の検証用。
    Hang,
    /// `tool_runtime.execute` を逐次実行してから `next` へ。
    /// tool 計上・journal・`onLimit` の実経路を辿る。stream event は
    /// emit しない簡易形 — model/result event 列まで再現する必要が
    /// ある検証(F1 pending batch 等)は [`ScriptStep::ToolCalls`] を使う。
    ToolCalls {
        calls: Vec<ToolCallSpec>,
        next: Box<FakeResponse>,
    },
    /// `stream_run` 内部で panic(panic 境界の検証用)。
    Panic { message: String },
    /// T4 fixture DSL の step 列を逐次 replay する scripted session。
    /// 終端 step(`finish`/`error` 非 retry/`hang_until_abort`/`hang`/
    /// `abort_response`/`panic`)まで step を消化する。
    Scripted(ScriptedSession),
}

/// fixture DSL session 1 本分(`create_session` 1 回に対応する script)。
#[derive(Debug, Clone)]
pub struct ScriptedSession {
    pub steps: Vec<ScriptStep>,
}

/// fixture DSL の step 単位(scripted stream 層の replay primitive)。
///
/// `error` step の retry 意味論は adapter `RetryingCompletion` と同じ:
/// 現在の completion call がまだ item を emit していない
/// (`emit`/`text_delta` 無し — `turn`/`tool_calls` step が call 境界を
/// 切る)・transient 分類・retry 予算残(`SessionSpec.transient_retries`)
/// の 3 条件を全て満たすとき `transientRetries` を計上して次 step へ
/// 進み、それ以外は終端 `PROVIDER_FAILURE` を emit する。
#[derive(Debug, Clone)]
pub enum ScriptStep {
    /// `RunEvent` を 1 件 emit(text delta 等)。現 call が item を
    /// 産んだ記録になり、以後の `error` は retry 対象外(partial turn)。
    Emit { event: RunEvent },
    /// `delay_ms` — 指定 ms 待つ。abort signal は待ちを中断して
    /// 終端 abort error になる(timeout/signal abort の再現)。
    Delay { ms: u64 },
    /// model tool-call event emit → `tool_runtime.execute` 逐次実行
    /// (計上・journal・onLimit の実経路)→ `ToolResult` event emit。
    /// Rig `drive_tool_calls` と同じ emit 順序(F1)。完了した call な
    /// ので次 step は新しい completion call。
    ToolCalls { calls: Vec<ToolCallSpec> },
    /// `usage_delta` — 1 completion call 分の token usage を加算する。
    UsageDelta { usage: UsageSnapshot },
    /// 受理 turn を 1 件計上(`record_turn`)。call 境界を切るので
    /// 次の `error` は新しい call の pre-item 失敗として retry 判定される。
    Turn { tool_calls: bool },
    /// invalid-tool recovery で rollback された model call を 1 件計上
    /// (`record_rolled_back_turn` — adapter hook と同じ計上点、F6b)。
    /// Pi は error tool result で turn を閉じ `turn_end` 計上するため
    /// `turns`/`last_turn_tool_calls` に載る。call 境界を切る。
    RolledBackTurn,
    /// `error { status?, retryable?, message? }` — provider failure。
    /// retry 条件を満たせば retry bookkeeping + 次 step、尽きるか
    /// permanent なら終端 `PROVIDER_FAILURE`(details.class/retryable/
    /// httpStatus は adapter `status_details` と同形)。
    ProviderError {
        status: Option<u16>,
        retryable: Option<bool>,
        message: String,
    },
    /// `hang_until_abort` — signal abort を待って終端 abort error。
    WaitAbort,
    /// signal を観測しない永久 pending(abort grace 経路)。
    Hang,
    /// `abort_response` — provider 側が cancel を応答として返す形
    /// (adapter の `PromptCancelled` → `ABORTED` 写像相当)。
    AbortResponse,
    /// stream 内部 panic(panic 境界の検証用)。
    Panic { message: String },
    /// 最終 assistant text で終了(`Finished` emit)。
    Finish { text: String },
}

/// `ToolCalls` の 1 実行。
#[derive(Debug, Clone)]
pub struct ToolCallSpec {
    pub name: String,
    pub input: Value,
}

/// `create_session` の記録(assert 用)。
#[derive(Debug, Clone)]
pub struct RecordedSpec {
    pub system_prompt: String,
    pub has_tool_runtime: bool,
    pub has_output_schema: bool,
    pub max_output_tokens: u64,
    pub transient_retries: usize,
    /// `policy.execution.maxTurns` — factory が内部 guard
    /// (`max(1024, this)`)を算出する材料として渡される値(F7)。
    pub policy_max_turns: u64,
}

/// scripted `SessionFactory` — queue から response を取り出し session
/// を返す。生成 spec・全 prompt・未消化数を記録する(TS `calls`/
/// `pendingResponses` 相当)。
#[derive(Debug, Default)]
pub struct FakeSessionFactory {
    responses: Mutex<VecDeque<FakeResponse>>,
    created: Mutex<Vec<RecordedSpec>>,
    prompts: Arc<Mutex<Vec<String>>>,
}

impl FakeSessionFactory {
    pub fn new(responses: Vec<FakeResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            created: Mutex::new(Vec::new()),
            prompts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 生成済み session の spec 記録。
    pub fn created(&self) -> Vec<RecordedSpec> {
        self.created
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 全 session に渡された prompt(repair prompt の検証用)。
    pub fn prompts(&self) -> Vec<String> {
        self.prompts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// queue に残った未消化 response 数(TS `pendingResponses` 相当)。
    pub fn pending_responses(&self) -> usize {
        self.responses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }
}

impl SessionFactory for FakeSessionFactory {
    fn create_session(
        &self,
        spec: &SessionSpec<'_>,
    ) -> Result<Arc<dyn WorkerSession>, CheaperError> {
        self.created
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(RecordedSpec {
                system_prompt: spec.system_prompt.clone(),
                has_tool_runtime: spec.tool_runtime.is_some(),
                has_output_schema: spec.output_schema.is_some(),
                max_output_tokens: spec.max_output_tokens,
                transient_retries: spec.transient_retries,
                policy_max_turns: spec.policy_max_turns,
            });
        let response = self
            .responses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
            .ok_or_else(|| {
                CheaperError::new(
                    ErrorCode::InternalError,
                    "fake provider: no scripted response for session",
                )
            })?;
        Ok(Arc::new(FakeSession::new(
            response,
            spec.tool_runtime.clone(),
            Arc::clone(&self.prompts),
            spec.transient_retries,
        )))
    }
}

struct FakeSession {
    acc: Arc<Accumulator>,
    tool_runtime: Option<ToolRuntime>,
    prompts: Arc<Mutex<Vec<String>>>,
    retry_budget: usize,
    response: Mutex<Option<FakeResponse>>,
}

impl FakeSession {
    fn new(
        response: FakeResponse,
        tool_runtime: Option<ToolRuntime>,
        prompts: Arc<Mutex<Vec<String>>>,
        retry_budget: usize,
    ) -> Self {
        let mut acc = Accumulator::default();
        if let Some(rt) = &tool_runtime {
            acc.attach_budget(rt.budget());
        }
        Self {
            acc: Arc::new(acc),
            tool_runtime,
            prompts,
            retry_budget,
            response: Mutex::new(Some(response)),
        }
    }
}

impl WorkerSession for FakeSession {
    fn stream_run(&self, prompt: String, signal: Option<AbortSignal>) -> RunStream {
        let response = self
            .response
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .expect("fake session stream_run called twice");
        self.prompts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(prompt);
        fake_stream(
            response,
            signal,
            Arc::clone(&self.acc),
            self.tool_runtime.clone(),
            self.retry_budget,
        )
    }

    /// signal 駆動で cancel されるため no-op(grace trigger)。
    fn abort(&self) {}

    fn snapshot(&self) -> RunSnapshot {
        self.acc.snapshot()
    }
}

// ---------------------------------------------------------------------------
// scripted stream — unfold state machine
// ---------------------------------------------------------------------------

/// `ToolCalls` step が貼る実行待ち item。Rig `drive_tool_calls` の emit
/// 順序(model `ToolCall` event 群 → batch 実行 → `ToolResult` event 群)を
/// 再現するための内部 queue — fixture DSL には露出しない(F1: run 側の
/// pending-tool-call 追跡が ToolCall/ToolResult item 対に依存するため、
/// step は emit → 実行 → emit を item 単位で進める必要がある)。
enum QueuedWork {
    /// そのまま emit する stream item。
    Event(RunEvent),
    /// emit 済み `ToolCall` event 群に対応する実実行。結果は
    /// `ToolResult` event として queue 先頭へ差し戻す。
    Execute(Vec<ToolCallSpec>),
}

enum Step {
    /// 未解釈の response。
    Pending(Box<FakeResponse>),
    /// event queue 消化中。item 間で abort 判定。
    Events(VecDeque<Result<RunEvent, CheaperError>>),
    /// scripted session の step 消化中。
    /// - `queued`: `ToolCalls` step が積んだ未処理 item(ToolCall event /
    ///   実行 marker / ToolResult event)。非空なら step より先に 1 件消化。
    /// - `emitted_in_call`: 現 completion call が既に item を emit したか
    ///   (emit 後の `error` は retry しない — adapter の partial-turn 規則)。
    /// - `remaining_retries`: `SessionSpec.transient_retries` 由来の retry 予算。
    Scripted {
        steps: VecDeque<ScriptStep>,
        queued: VecDeque<QueuedWork>,
        emitted_in_call: bool,
        remaining_retries: usize,
    },
    /// signal abort 待ち。
    WaitAbort,
    /// 永久 pending。
    Hang,
    Done,
}

struct FakeState {
    step: Step,
    signal: Option<AbortSignal>,
    acc: Arc<Accumulator>,
    tool_runtime: Option<ToolRuntime>,
    /// `SessionSpec.transient_retries` — scripted `error` step の retry 予算。
    retry_budget: usize,
}

impl FakeState {
    /// script 宣言の usage/turns/transient を acc に計上する。
    fn apply_usage(
        acc: &Accumulator,
        usage: &UsageSnapshot,
        turns: u64,
        last_turn_tool_calls: bool,
        transient_retries: u64,
    ) {
        acc.record_completion(usage);
        for i in 0..turns {
            acc.record_turn(last_turn_tool_calls && i + 1 == turns);
        }
        for _ in 0..transient_retries {
            acc.record_transient_retry();
        }
    }

    /// abort 済なら終端 error item を返す(adapter stream と同じ形:
    /// `abort_reason_to_cheaper` で reason の code を復元する)。
    fn abort_item(&self) -> Option<Result<RunEvent, CheaperError>> {
        let signal = self.signal.as_ref()?;
        if !signal.is_aborted() {
            return None;
        }
        let reason = signal.reason().expect("is_aborted implies reason recorded");
        Some(Err(abort_reason_to_cheaper(&reason)))
    }
}

// ---------------------------------------------------------------------------
// scripted `error` step の分類 — adapter `classify`/`status_details` の
// parity mirror。adapter の分類関数は Rig `ProviderError`/`ErrorReport`
// を引数に取り runtime からは呼べないため、同じ分類表を scripted
// `{status,retryable}` から再構成する(`Debug` 名も `ErrorClass` と一致)。
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptedErrorClass {
    TransientRateLimit,
    TransientServer,
    TransientTransport,
    Permanent,
}

impl ScriptedErrorClass {
    fn is_transient(self) -> bool {
        !matches!(self, Self::Permanent)
    }
}

/// adapter `classify_status` + `retryable` fallback と同じ表:
/// 429 → RateLimit、status >= 500 → Server、その他 status → Permanent、
/// status 無しは provider の `retryable` 主張 → Transport、else Permanent。
fn classify_scripted_error(status: Option<u16>, retryable: Option<bool>) -> ScriptedErrorClass {
    match status {
        Some(429) => ScriptedErrorClass::TransientRateLimit,
        Some(s) if s >= 500 => ScriptedErrorClass::TransientServer,
        Some(_) => ScriptedErrorClass::Permanent,
        None => {
            if retryable.unwrap_or(false) {
                ScriptedErrorClass::TransientTransport
            } else {
                ScriptedErrorClass::Permanent
            }
        }
    }
}

/// adapter `provider_error_to_cheaper` + `status_details` と同形の
/// 終端 `PROVIDER_FAILURE`(details.class/retryable/httpStatus)。
fn scripted_provider_error(
    class: ScriptedErrorClass,
    status: Option<u16>,
    message: &str,
) -> CheaperError {
    let mut details = Map::new();
    details.insert("class".to_string(), Value::from(format!("{class:?}")));
    details.insert("retryable".to_string(), Value::from(class.is_transient()));
    if let Some(status) = status {
        details.insert("httpStatus".to_string(), Value::from(status));
    }
    CheaperError::new(
        ErrorCode::ProviderFailure,
        format!("provider completion failed: {message}"),
    )
    .with_details(details)
}

/// scripted step 1 件の処理結果。
enum StepOutcome {
    /// side-effect 済み — loop 継続。`self.step` が `Done` のままなら
    /// 呼び出し側が `Scripted` を復元する(WaitAbort/Hang へ遷移した
    /// 場合は既に設定済み)。
    Continue,
    /// emit すべき item(step は遷移済み — `self.step` は Done か
    /// Emit の場合のみ次の `Scripted`)。
    Item(Result<RunEvent, CheaperError>),
}

impl FakeState {
    /// `Scripted` step を 1 件消化する。呼び出し側は `st.step` を `Done`
    /// に置き換えてから呼ぶ — この関数が必要に応じて `st.step` を再設定する。
    /// `queued` は `ToolCalls` step が積む emit/実行 item の吐き先。
    async fn run_script_step(
        &mut self,
        steps: &mut VecDeque<ScriptStep>,
        queued: &mut VecDeque<QueuedWork>,
        emitted_in_call: &mut bool,
        remaining_retries: &mut usize,
    ) -> StepOutcome {
        let Some(step) = steps.pop_front() else {
            return StepOutcome::Item(Err(CheaperError::new(
                ErrorCode::ProviderFailure,
                "fake provider: scripted session ended without a terminal step",
            )));
        };
        match step {
            ScriptStep::Emit { event } => {
                *emitted_in_call = true;
                self.step = Step::Scripted {
                    steps: std::mem::take(steps),
                    queued: std::mem::take(queued),
                    emitted_in_call: *emitted_in_call,
                    remaining_retries: *remaining_retries,
                };
                StepOutcome::Item(Ok(event))
            }
            ScriptStep::Delay { ms } => {
                // abort signal は delay を中断する — 中断後は loop-top の
                // abort_item check が終端 error を emit する。
                let sleep = tokio::time::sleep(Duration::from_millis(ms));
                if let Some(signal) = &self.signal {
                    tokio::select! {
                        biased;
                        _ = signal.cancelled() => {}
                        _ = sleep => {}
                    }
                } else {
                    sleep.await;
                }
                StepOutcome::Continue
            }
            ScriptStep::ToolCalls { calls } => {
                // Rig `drive_tool_calls` の emit 順序を再現する(F1):
                // model の `ToolCall` event を全件 emit → batch 実行
                // (`QueuedWork::Execute`)→ `ToolResult` event を emit。
                // 即時実行せず queue へ積むのは、run 側の
                // pending-tool-call 追跡(maxTurns の deferred abort)が
                // `ToolCall`/`ToolResult` item 対を見るため — 実行が
                // event 間に挟まる実 stream と同じ形状にする。
                for (i, call) in calls.iter().enumerate() {
                    queued.push_back(QueuedWork::Event(RunEvent::ToolCall {
                        call_id: format!("call-{i}"),
                        name: call.name.clone(),
                        arguments: call.input.clone(),
                    }));
                }
                queued.push_back(QueuedWork::Execute(calls));
                // tool call を産んだ completion call は完了 — 次 step は
                // 新しい call(`error` の pre-item retry 判定を reset)。
                *emitted_in_call = false;
                StepOutcome::Continue
            }
            ScriptStep::UsageDelta { usage } => {
                self.acc.record_completion(&usage);
                StepOutcome::Continue
            }
            ScriptStep::Turn { tool_calls } => {
                self.acc.record_turn(tool_calls);
                *emitted_in_call = false;
                StepOutcome::Continue
            }
            ScriptStep::RolledBackTurn => {
                self.acc.record_rolled_back_turn();
                *emitted_in_call = false;
                StepOutcome::Continue
            }
            ScriptStep::ProviderError {
                status,
                retryable,
                message,
            } => {
                let class = classify_scripted_error(status, retryable);
                if !*emitted_in_call && class.is_transient() && *remaining_retries > 0 {
                    *remaining_retries -= 1;
                    self.acc.record_transient_retry();
                    return StepOutcome::Continue;
                }
                // `st.step` は既に Done — 終端 error を emit する。
                StepOutcome::Item(Err(scripted_provider_error(class, status, &message)))
            }
            ScriptStep::WaitAbort => {
                self.step = Step::WaitAbort;
                StepOutcome::Continue
            }
            ScriptStep::Hang => {
                self.step = Step::Hang;
                StepOutcome::Continue
            }
            ScriptStep::AbortResponse => {
                // `PromptError::PromptCancelled` → ABORTED と同じ写像。
                StepOutcome::Item(Err(CheaperError::new(
                    ErrorCode::Aborted,
                    "worker run cancelled: scripted abort response",
                )))
            }
            ScriptStep::Panic { message } => {
                panic!("fake provider panic: {message}");
            }
            ScriptStep::Finish { text } => {
                StepOutcome::Item(Ok(RunEvent::Finished { output: text }))
            }
        }
    }
}

fn fake_stream(
    response: FakeResponse,
    signal: Option<AbortSignal>,
    acc: Arc<Accumulator>,
    tool_runtime: Option<ToolRuntime>,
    retry_budget: usize,
) -> RunStream {
    let state = FakeState {
        step: Step::Pending(Box::new(response)),
        signal,
        acc,
        tool_runtime,
        retry_budget,
    };
    Box::pin(stream::unfold(state, |mut st| async move {
        loop {
            // per-item abort 判定 — adapter stream の check と同じ位置。
            if let Some(item) = st.abort_item() {
                st.step = Step::Done;
                return Some((item, st));
            }
            // scripted session — step 処理は `run_script_step` へ。
            // `match` scrutinee の借用を避けるため先に `Done` へ置き換える。
            if matches!(st.step, Step::Scripted { .. }) {
                let Step::Scripted {
                    mut steps,
                    mut queued,
                    mut emitted_in_call,
                    mut remaining_retries,
                } = std::mem::replace(&mut st.step, Step::Done)
                else {
                    unreachable!()
                };
                // `ToolCalls` が積んだ item を step より先に 1 件消化する
                // (item 毎に emit — run 側の per-item abort 判定が各
                // `ToolCall`/`ToolResult` を観測できる)。
                if let Some(work) = queued.pop_front() {
                    match work {
                        QueuedWork::Event(event) => {
                            st.step = Step::Scripted {
                                steps,
                                queued,
                                emitted_in_call,
                                remaining_retries,
                            };
                            return Some((Ok(event), st));
                        }
                        QueuedWork::Execute(calls) => {
                            // emit 済み `ToolCall` event 群に対応する実
                            // 実行 — rig は batch 全件の実行後に result を
                            // surface する。計上・journal・onLimit は
                            // `ToolRuntime::execute` の実経路を辿る。
                            let mut results = VecDeque::with_capacity(calls.len());
                            for (i, call) in calls.iter().enumerate() {
                                let output = match &st.tool_runtime {
                                    Some(rt) => match rt
                                        .execute(
                                            &call.name,
                                            &call.input,
                                            Some(ToolCallContext {
                                                signal: st.signal.clone(),
                                            }),
                                        )
                                        .await
                                    {
                                        Ok(result) => serde_json::to_value(&result.content)
                                            .unwrap_or(Value::Null),
                                        Err(err) => json!({ "error": err.to_string() }),
                                    },
                                    None => Value::Null,
                                };
                                results.push_back(QueuedWork::Event(RunEvent::ToolResult {
                                    call_id: format!("call-{i}"),
                                    name: call.name.clone(),
                                    output,
                                    details: None,
                                }));
                            }
                            // 実行結果は queue 先頭へ — emit 済み ToolCall
                            // に続く ToolResult の順序を保つ。
                            while let Some(item) = results.pop_back() {
                                queued.push_front(item);
                            }
                            st.step = Step::Scripted {
                                steps,
                                queued,
                                emitted_in_call,
                                remaining_retries,
                            };
                            continue;
                        }
                    }
                }
                match st
                    .run_script_step(
                        &mut steps,
                        &mut queued,
                        &mut emitted_in_call,
                        &mut remaining_retries,
                    )
                    .await
                {
                    StepOutcome::Item(item) => return Some((item, st)),
                    StepOutcome::Continue => {
                        // run_script_step が WaitAbort/Hang へ遷移させた
                        // 場合はそのまま。Done のままなら残 step を戻す。
                        if matches!(st.step, Step::Done) {
                            st.step = Step::Scripted {
                                steps,
                                queued,
                                emitted_in_call,
                                remaining_retries,
                            };
                        }
                        continue;
                    }
                }
            }
            match &mut st.step {
                Step::Done => return None,
                // Scripted は上の pre-match block で処理する — ここには来ない。
                Step::Scripted { .. } => unreachable!(),
                Step::Hang => {
                    std::future::pending::<()>().await;
                }
                Step::WaitAbort => match &st.signal {
                    Some(signal) => {
                        signal.cancelled().await;
                        let reason = signal.reason().expect("cancelled implies reason");
                        st.step = Step::Done;
                        return Some((Err(abort_reason_to_cheaper(&reason)), st));
                    }
                    None => {
                        st.step = Step::Done;
                        continue;
                    }
                },
                Step::Events(queue) => match queue.pop_front() {
                    Some(item) => {
                        if item.is_err() {
                            st.step = Step::Done;
                        }
                        return Some((item, st));
                    }
                    None => {
                        st.step = Step::Done;
                        continue;
                    }
                },
                Step::Pending(response) => {
                    let owned = std::mem::replace(response, Box::new(FakeResponse::Hang));
                    match *owned {
                        FakeResponse::Complete {
                            text,
                            usage,
                            turns,
                            last_turn_tool_calls,
                            transient_retries,
                        } => {
                            FakeState::apply_usage(
                                &st.acc,
                                &usage,
                                turns,
                                last_turn_tool_calls,
                                transient_retries,
                            );
                            st.step = Step::Done;
                            return Some((Ok(RunEvent::Finished { output: text }), st));
                        }
                        FakeResponse::Events {
                            events,
                            usage,
                            turns,
                            last_turn_tool_calls,
                            transient_retries,
                        } => {
                            FakeState::apply_usage(
                                &st.acc,
                                &usage,
                                turns,
                                last_turn_tool_calls,
                                transient_retries,
                            );
                            st.step = Step::Events(events.into());
                            continue;
                        }
                        FakeResponse::Fail {
                            error,
                            usage,
                            turns,
                        } => {
                            FakeState::apply_usage(&st.acc, &usage, turns, false, 0);
                            st.step = Step::Done;
                            return Some((Err(error), st));
                        }
                        FakeResponse::WaitAbort { usage, turns } => {
                            FakeState::apply_usage(&st.acc, &usage, turns, false, 0);
                            st.step = Step::WaitAbort;
                            continue;
                        }
                        FakeResponse::Hang => {
                            st.step = Step::Hang;
                            continue;
                        }
                        FakeResponse::ToolCalls { calls, next } => {
                            for call in calls {
                                if let Some(rt) = &st.tool_runtime {
                                    // 結果(成功/拒否)は問わない — 計上と
                                    // journal 副作用が目的。onLimit は
                                    // execute 内部で fire する。
                                    let _ = rt
                                        .execute(
                                            &call.name,
                                            &call.input,
                                            Some(ToolCallContext {
                                                signal: st.signal.clone(),
                                            }),
                                        )
                                        .await;
                                }
                            }
                            st.step = Step::Pending(next);
                            continue;
                        }
                        FakeResponse::Panic { message } => {
                            panic!("fake provider panic: {message}");
                        }
                        FakeResponse::Scripted(session) => {
                            st.step = Step::Scripted {
                                steps: session.steps.into(),
                                queued: VecDeque::new(),
                                emitted_in_call: false,
                                // session 毎の retry 予算(main/repair で
                                // `SessionSpec.transient_retries` 由来)。
                                remaining_retries: st.retry_budget,
                            };
                            continue;
                        }
                    }
                }
            }
        }
    }))
}

// ---------------------------------------------------------------------------
// CheaperFakePlugin — catalog + keyless credential 層
// ---------------------------------------------------------------------------

/// `cheaper-fake` provider の plugin registration(TS testsupport
/// `createFakePi` 相当 — `provider: "cheaper-fake"`・`api:
/// "cheaper-fake-api"`・model `fake-model`・keyless `apiKey.resolve`)。
///
/// keyless credential: `api_key` default は literal `"fake-key"` —
/// env/credential 無しで configured + resolve 済みになる。fixture は
/// `with_api_key("$VAR")` で env template 解決経路、stored credential
/// 経路は `AuthService::login`(api_key method は `api_key` 有りで常に
/// advertise される)で辿れる。
pub struct CheaperFakePlugin {
    /// `apiKey` config 値(literal or `"$VAR"`/`"!cmd"` template)。
    api_key: String,
    /// bootstrap model list(fixture の model/api/cost 差し替え用)。
    models: Vec<ModelDefinitionJson>,
}

impl CheaperFakePlugin {
    pub const PROVIDER_ID: &'static str = "cheaper-fake";
    pub const API_KIND: &'static str = "cheaper-fake-api";
    pub const MODEL_ID: &'static str = "fake-model";

    pub fn new() -> Self {
        Self {
            api_key: "fake-key".to_string(),
            models: vec![Self::default_model()],
        }
    }

    /// `fake-model` の cost — `tests/run.rs` の fake provider と同じ
    /// rates(input=1.0 output=2.0 cache*=0.5 /1M tokens)。fixture の
    /// `costUsd` 期待値はこの pricing から導かれる。
    pub fn default_cost() -> ModelCostJson {
        ModelCostJson {
            input: 1.0,
            output: 2.0,
            cache_read: 0.5,
            cache_write: 0.5,
            tiers: None,
        }
    }

    /// bootstrap `fake-model` 定義(api は provider `cheaper-fake-api` を
    /// 継承 — model `api` 明示で unsupported api 検証も可能)。
    pub fn default_model() -> ModelDefinitionJson {
        ModelDefinitionJson {
            id: Self::MODEL_ID.to_string(),
            name: Some("Fake Model".to_string()),
            api: None,
            base_url: None,
            reasoning: None,
            thinking_level_map: None,
            input: None,
            cost: Some(Self::default_cost()),
            context_window: Some(128_000.0),
            max_tokens: Some(8192.0),
            sampling_params: None,
            headers: None,
            compat: None,
        }
    }

    /// `apiKey` config 値の差し替え(`"$CHEAPER_FAKE_API_KEY"` 等の
    /// template で env 解決経路を検証する)。
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = api_key.into();
        self
    }

    /// bootstrap model list の差し替え(fixture `model` 設定用)。
    pub fn with_models(mut self, models: Vec<ModelDefinitionJson>) -> Self {
        self.models = models;
        self
    }
}

impl Default for CheaperFakePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CheaperProviderPlugin for CheaperFakePlugin {
    fn id(&self) -> &str {
        Self::PROVIDER_ID
    }

    fn register(&self, registry: &mut ProviderRegistry, _ctx: &ProviderPluginContext) {
        registry.register_provider(PluginProviderRegistration {
            id: Self::PROVIDER_ID.to_string(),
            name: "Cheaper Fake".to_string(),
            api: Self::API_KIND.to_string(),
            base_url: "http://cheaper-fake.local".to_string(),
            api_key: Some(self.api_key.clone()),
            auth_header: None,
            headers: None,
            models: self.models.clone(),
            auth: None,
            subscription: false,
            catalog: None,
        });
    }
}
