//! Port of `packages/tool-runtime/src/accounting.ts`.
//!
//! `createToolBudget` / `createToolMeter`。計数写像の contract:
//! 「実行到達 = `toolCalls`・boundary/limit 拒否 = `rejectedToolCalls`・
//! runtime 未到達の invalid call = 不計上」(design canonical-tool-runtime
//! 「budget 外側/meter 内側 1 回適用・拒否計数規則」)。
//!
//! `run` は `FnOnce() -> Future` を取る — 上限超過時に closure 自体を
//! 呼ばないことで、TS の「promise 開始前に admission を判定する」意味論
//! (Rust の future は lazy のため開始=first poll)を保存する。
//! 返り値の `budget`/`meter` は getter view で、`Arc` 共有の counter を
//! 読む(TS の getter が live state を読むのと同じ)。

use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cheaper_core::token::count::count_text_tokens;
use serde_json::Value;

use crate::walker::is_access_denied_error;
use crate::ToolError;

/// `ToolBudgetOptions`。`onLimit` は TS と同じく必須。
pub struct ToolBudgetOptions {
    pub max_tool_calls: u64,
    pub on_limit: Box<dyn Fn() + Send + Sync>,
}

/// `ToolBudget["limitHit"]`: `"maxToolCalls" | null`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitHit {
    MaxToolCalls,
}

impl LimitHit {
    /// `"maxToolCalls"`。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaxToolCalls => "maxToolCalls",
        }
    }
}

#[derive(Default)]
struct BudgetState {
    tool_calls: u64,
    rejected_tool_calls: u64,
    limit_hit: Option<LimitHit>,
}

struct BudgetInner {
    options: ToolBudgetOptions,
    state: Arc<Mutex<BudgetState>>,
}

/// `ToolBudget`(readonly counter view。getter は live state を読む)。
#[derive(Clone)]
pub struct ToolBudget {
    state: Arc<Mutex<BudgetState>>,
}

impl ToolBudget {
    /// `budget.toolCalls`。
    pub fn tool_calls(&self) -> u64 {
        self.state.lock().expect("budget state poisoned").tool_calls
    }

    /// `budget.rejectedToolCalls`。
    pub fn rejected_tool_calls(&self) -> u64 {
        self.state
            .lock()
            .expect("budget state poisoned")
            .rejected_tool_calls
    }

    /// `budget.limitHit`。
    pub fn limit_hit(&self) -> Option<LimitHit> {
        self.state.lock().expect("budget state poisoned").limit_hit
    }
}

/// `createToolBudget` の戻り値側(`run` を持つ runner)。
/// `Clone` は execute closure への共有用(internal state は Arc)。
#[derive(Clone)]
pub struct ToolBudgetRunner {
    inner: Arc<BudgetInner>,
}

impl ToolBudgetRunner {
    /// `accounting.budget`。
    pub fn budget(&self) -> ToolBudget {
        ToolBudget {
            state: Arc::clone(&self.inner.state),
        }
    }

    /// `run(execute)`。同期 admission が sibling call の await より先に
    /// 上限を予約する(`toolCalls++` は `execute` 呼出し前)。上限超過は
    /// `rejectedToolCalls++` + `limitHit` + `onLimit()` + throw。
    /// 実行中の throw が access denied error なら `rejectedToolCalls++`。
    pub async fn run<T, Fut>(&self, execute: impl FnOnce() -> Fut) -> Result<T, ToolError>
    where
        Fut: Future<Output = Result<T, ToolError>>,
    {
        {
            let mut state = self.inner.state.lock().expect("budget state poisoned");
            if state.tool_calls >= self.inner.options.max_tool_calls {
                state.limit_hit = Some(LimitHit::MaxToolCalls);
                state.rejected_tool_calls += 1;
                drop(state);
                (self.inner.options.on_limit)();
                return Err(format!(
                    "tool call rejected: maxToolCalls limit reached ({})",
                    self.inner.options.max_tool_calls
                )
                .into());
            }
            state.tool_calls += 1;
        }
        match execute().await {
            Err(e) => {
                if is_access_denied_error(e.as_ref()) {
                    self.inner
                        .state
                        .lock()
                        .expect("budget state poisoned")
                        .rejected_tool_calls += 1;
                }
                Err(e)
            }
            ok => ok,
        }
    }
}

/// `createToolBudget(opts)`。
/// 同期 admission が sibling call の await より先に上限を予約する。
pub fn create_tool_budget(options: ToolBudgetOptions) -> ToolBudgetRunner {
    ToolBudgetRunner {
        inner: Arc::new(BudgetInner {
            options,
            state: Arc::new(Mutex::new(BudgetState::default())),
        }),
    }
}

/// `meter.run` の result から `content[].text` を引く duck-typing seam。
/// TS は `(result as {content?})?.content` が配列なら各 item の
/// `text`(string のみ)を `countTextTokens` に掛ける。
/// `CanonicalToolResult`(catalog.rs / T2-4)がこの trait を実装する想定。
/// 同 shape の `serde_json::Value` にはここで既定 impl を提供する。
pub trait ResultContent {
    /// `content` 配列内の string `text` item を列挙する。
    fn content_texts(&self) -> Vec<&str>;
}

impl ResultContent for Value {
    fn content_texts(&self) -> Vec<&str> {
        self.get("content")
            .and_then(Value::as_array)
            .map(|content| {
                content
                    .iter()
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// `ToolMeter`(readonly counter view)。
#[derive(Clone)]
pub struct ToolMeter {
    source_tokens: Arc<AtomicUsize>,
}

impl ToolMeter {
    /// `meter.sourceTokens`。
    pub fn source_tokens(&self) -> usize {
        self.source_tokens.load(Ordering::Relaxed)
    }
}

/// `createToolMeter` の戻り値側(`run` を持つ runner)。
/// `Clone` は execute closure への共有用(counter は Arc)。
#[derive(Clone)]
pub struct ToolMeterRunner {
    source_tokens: Arc<AtomicUsize>,
}

impl ToolMeterRunner {
    /// `metering.meter`。
    pub fn meter(&self) -> ToolMeter {
        ToolMeter {
            source_tokens: Arc::clone(&self.source_tokens),
        }
    }

    /// `run(execute)`。成功 result の `content[].text` だけを
    /// `countTextTokens` で集計する。`execute` の throw は計上せずそのまま
    /// 伝播し、`countTextTokens` の throw(special token)もそのまま伝播する
    /// (それまでの部分加算は残る — TS と同じ逐次 `+=` 意味論)。
    pub async fn run<T, Fut>(&self, execute: impl FnOnce() -> Fut) -> Result<T, ToolError>
    where
        T: ResultContent,
        Fut: Future<Output = Result<T, ToolError>>,
    {
        let result = execute().await?;
        for text in result.content_texts() {
            let tokens = count_text_tokens(text).map_err(|e| Box::new(e) as ToolError)?;
            self.source_tokens.fetch_add(tokens, Ordering::Relaxed);
        }
        Ok(result)
    }
}

/// `createToolMeter()`。
pub fn create_tool_meter() -> ToolMeterRunner {
    ToolMeterRunner {
        source_tokens: Arc::new(AtomicUsize::new(0)),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio::sync::oneshot;

    use super::*;
    use crate::walker::DeniedError;

    fn budget_opts(max_tool_calls: u64) -> (ToolBudgetOptions, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls2 = Arc::clone(&calls);
        (
            ToolBudgetOptions {
                max_tool_calls,
                on_limit: Box::new(move || {
                    calls2.fetch_add(1, Ordering::Relaxed);
                }),
            },
            calls,
        )
    }

    /// 計数写像の contract: 実行到達 = toolCalls、boundary/limit 拒否 =
    /// rejectedToolCalls、runtime 未到達の invalid call = 不計上。
    /// (registry 側の invalid call は `run` を一度も呼ばない —
    /// 4 番目の写像は「run が呼ばれない」ことで成立する。)
    #[tokio::test]
    async fn acct_budget_counting_contract() {
        let (opts, on_limit) = budget_opts(2);
        let runner = create_tool_budget(opts);
        let budget = runner.budget();

        // 1. invalid call(runtime 未到達)— run を呼ばない → 不計上。
        assert_eq!(budget.tool_calls(), 0);
        assert_eq!(budget.rejected_tool_calls(), 0);

        // 2. 実行到達 + 成功 → toolCalls++ のみ。
        runner
            .run(|| async { Ok::<_, ToolError>(json!("ok")) })
            .await
            .unwrap();
        assert_eq!(budget.tool_calls(), 1);
        assert_eq!(budget.rejected_tool_calls(), 0);

        // 3. 実行到達 + access denied throw → toolCalls++ AND
        //    rejectedToolCalls++(runtime.test.ts と同じ両方計上)。
        runner
            .run(|| async { Err::<Value, _>(Box::new(DeniedError::scope()) as ToolError) })
            .await
            .unwrap_err();
        assert_eq!(budget.tool_calls(), 2);
        assert_eq!(budget.rejected_tool_calls(), 1);
        assert!(budget.limit_hit().is_none());

        // 4. 上限超過 → toolCalls 不増 + rejectedToolCalls++ + limitHit +
        //    onLimit + throw。execute closure 自体が呼ばれない。
        let executed = Arc::new(AtomicUsize::new(0));
        let e = {
            let executed = Arc::clone(&executed);
            runner
                .run(|| async move {
                    executed.fetch_add(1, Ordering::Relaxed);
                    Ok::<_, ToolError>(json!("never"))
                })
                .await
                .unwrap_err()
        };
        assert_eq!(
            e.to_string(),
            "tool call rejected: maxToolCalls limit reached (2)"
        );
        assert_eq!(executed.load(Ordering::Relaxed), 0);
        assert_eq!(budget.tool_calls(), 2);
        assert_eq!(budget.rejected_tool_calls(), 2);
        assert_eq!(budget.limit_hit(), Some(LimitHit::MaxToolCalls));
        assert_eq!(on_limit.load(Ordering::Relaxed), 1);
    }

    /// 非 access-denied の実行失敗は rejectedToolCalls に計上しない。
    #[tokio::test]
    async fn acct_budget_non_denied_error_not_rejected() {
        let (opts, _) = budget_opts(4);
        let runner = create_tool_budget(opts);
        let budget = runner.budget();
        let e = runner
            .run(|| async { Err::<Value, _>(Box::new(std::io::Error::other("boom")) as ToolError) })
            .await
            .unwrap_err();
        assert_eq!(e.to_string(), "boom");
        assert_eq!(budget.tool_calls(), 1);
        assert_eq!(budget.rejected_tool_calls(), 0);
        // deny 3 定数のいずれかなら rejected 計上。
        for denied in [DeniedError::access(), DeniedError::policy()] {
            runner
                .run(move || async move { Err::<Value, _>(Box::new(denied) as ToolError) })
                .await
                .unwrap_err();
        }
        assert_eq!(budget.tool_calls(), 3);
        assert_eq!(budget.rejected_tool_calls(), 2);
    }

    /// sibling admission は await 前に同期予約される —
    /// runtime.test.ts の「2 つ目の並行 call は拒否」を再現する。
    #[tokio::test]
    async fn acct_budget_admission_reserves_slot_before_await() {
        let (opts, _) = budget_opts(1);
        let runner = create_tool_budget(opts);
        let budget = runner.budget();
        let (tx, rx) = oneshot::channel::<()>();
        let first = runner.run(|| async move {
            rx.await.unwrap();
            Ok::<_, ToolError>(json!("first"))
        });
        tokio::pin!(first);
        // 1 回 poll して admission(toolCalls=1)だけ済ませ、execute は pending。
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut first)
                .await
                .is_err()
        );
        assert_eq!(budget.tool_calls(), 1);
        // sibling は即座に拒否される。
        let second = runner
            .run(|| async { Ok::<_, ToolError>(json!("second")) })
            .await;
        assert!(second.is_err());
        assert_eq!(budget.rejected_tool_calls(), 1);
        tx.send(()).unwrap();
        assert!(first.await.is_ok());
    }

    /// `meter.run` は成功 result の `content[].text`(string のみ)を
    /// `countTextTokens` で集計する。token 数は tokenizer で実測する。
    #[tokio::test]
    async fn acct_meter_sums_content_text_tokens() {
        let runner = create_tool_meter();
        let meter = runner.meter();
        let result = runner
            .run(|| async {
                Ok::<_, ToolError>(json!({
                    "content": [
                        {"type": "text", "text": "hello world"},
                        {"type": "text", "text": "another chunk"}
                    ]
                }))
            })
            .await
            .unwrap();
        assert_eq!(result["content"].as_array().unwrap().len(), 2);
        let expected =
            count_text_tokens("hello world").unwrap() + count_text_tokens("another chunk").unwrap();
        assert_eq!(meter.source_tokens(), expected);
    }

    /// `content` 非配列・`text` 非 string・content 欠落は計上しない
    /// (TS の `Array.isArray` + `typeof item?.text === "string"` guard)。
    /// 拒否/失敗した実行の output は計上しない。
    #[tokio::test]
    async fn acct_meter_skips_non_text_and_errors() {
        let runner = create_tool_meter();
        let meter = runner.meter();
        for result in [
            json!({}),
            json!({"content": "not-array"}),
            json!({"content": [{"type": "text", "text": 7}, {"type": "image"}]}),
        ] {
            runner
                .run(|| async { Ok::<_, ToolError>(result.clone()) })
                .await
                .unwrap();
        }
        assert_eq!(meter.source_tokens(), 0);
        // execute の throw は計上なしで伝播する。
        runner
            .run(|| async { Err::<Value, _>(Box::new(DeniedError::scope()) as ToolError) })
            .await
            .unwrap_err();
        assert_eq!(meter.source_tokens(), 0);
    }

    /// `countTextTokens` の throw(special token)は meter.run から伝播する。
    #[tokio::test]
    async fn acct_meter_propagates_tokenizer_error() {
        let runner = create_tool_meter();
        let meter = runner.meter();
        let e = runner
            .run(|| async {
                Ok::<_, ToolError>(json!({
                    "content": [{"type": "text", "text": "<|endoftext|>"}]
                }))
            })
            .await
            .unwrap_err();
        assert!(e.to_string().contains("special token"));
        assert_eq!(meter.source_tokens(), 0);
    }
}
