//! Completion-call 単位の transient retry wrapper(G3)。
//!
//! TS `runtime-pi` では Pi session の `retry.maxRetries` が provider 呼出し
//! 内で transient 失敗を retry する。Rig 側に相当機能は無いため、adapter が
//! completion effect を serve する `Serve` handler(`ModelAdapter` と同じ
//! seam)として実装する。
//!
//! 境界は **completion call 1 回**だけ: run 全体や tool 実行を巻き込む
//! retry は非冪等な tool 副作用を二重実行し得るため絶対に行わない
//! (retry loop は dispatch 1 件の中に閉じ、tool dispatch は別 effect)。
//! retry 毎に `Accumulator::record_transient_retry` で計上する
//! (`meta.transientRetries` 相当 — Pi `auto_retry_start` の計上点)。
//!
//! streaming reply の retry は **最初の event が emit される前の失敗**に
//! 限る。1 件でも event を下流へ流した後の失敗は partial turn として engine
//! に見えているため、そのまま error として流す(勝手に再送すると provider
//! 応答の prefix が重複する)。

use std::time::Duration;

use futures::StreamExt;

use rig::completion::{CompletionRequest, CompletionResponse};
use rig::core::effect::{self, EffectKind, FamilyDescriptor, HandlerDescriptor, Outcome};
use rig::core::serve::{Dispatch, Reply, Serve};
use rig::core::{operation::Completion, DynModel};
use rig::error::ErrorReport;

use crate::accumulator::Accumulator;
use crate::error::{classify, classify_report, ErrorClass};

/// Transient retry の policy。`attempts` は再試行回数(初回を除く)で
/// `policy.retry.transientAttempts` 相当 — 0 で retry 無効。
/// backoff は `initial * 2^n` を `cap` で打ち切る指数 backoff。
/// (Pi 内部の backoff パラメータは contract ではないので adapter 選定値。)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// transient 失敗に対する最大 retry 回数。
    pub max_attempts: usize,
    /// 初回 retry の待機時間。
    pub initial_backoff: Duration,
    /// backoff の上限。
    pub max_backoff: Duration,
}

impl RetryPolicy {
    /// `transientRetries = 0` の既定(Pi `retry.enabled: false` 相当)。
    pub const DISABLED: Self = Self {
        max_attempts: 0,
        initial_backoff: Duration::from_millis(500),
        max_backoff: Duration::from_secs(8),
    };

    /// 再試行回数だけ指定した policy。
    pub const fn with_attempts(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            ..Self::DISABLED
        }
    }

    fn backoff(&self, attempt: usize) -> Duration {
        self.initial_backoff
            .saturating_mul(1u32 << attempt.min(20))
            .min(self.max_backoff)
    }
}

/// `ModelAdapter` と同じ責務の completion handler に transient retry を
/// 組み込んだもの。`AgentBuilder::model_route_handler` で登録し、
/// `Agent::set_model_label` で default model として選択する
/// (`named_model` は `Into<DynModel>` しか受けないため route 経由)。
pub(crate) struct RetryingCompletion {
    label: rig::completion::ModelRef,
    inner: DynModel<Completion>,
    policy: RetryPolicy,
    acc: std::sync::Arc<Accumulator>,
}

impl RetryingCompletion {
    pub(crate) fn new(
        label: impl Into<rig::completion::ModelRef>,
        inner: DynModel<Completion>,
        policy: RetryPolicy,
        acc: std::sync::Arc<Accumulator>,
    ) -> Self {
        Self {
            label: label.into(),
            inner,
            policy,
            acc,
        }
    }

    /// retry 判定: 分類が transient かつ retry 予算が残っているか。
    fn should_retry(&self, class: ErrorClass, attempt: usize) -> bool {
        class.is_transient() && attempt < self.policy.max_attempts
    }

    // `DynModel::call` の Err 型が `ProviderError`(大きい)で固定されている
    // ため、ここは rig の signature に合わせる(boxing は seam の外側の話)。
    #[allow(clippy::result_large_err)]
    async fn call_with_retry(
        &self,
        request: CompletionRequest,
        observation: Option<rig::observe::AdapterContext>,
    ) -> Result<CompletionResponse, rig::error::ProviderError> {
        let mut attempt = 0usize;
        loop {
            let result = match &observation {
                Some(context) => {
                    self.inner
                        .call_observed(request.clone(), context.clone())
                        .await
                }
                None => self.inner.call(request.clone()).await,
            };
            match result {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if !self.should_retry(classify(&error), attempt) {
                        return Err(error);
                    }
                    self.acc.record_transient_retry();
                    tokio::time::sleep(self.policy.backoff(attempt)).await;
                    attempt += 1;
                }
            }
        }
    }

    /// streaming open の retry + 最初の event までの早期失敗 retry。
    /// 最初の `Ok` item を見た後の失敗は partial turn なので retry しない。
    async fn stream_with_retry(
        &self,
        request: CompletionRequest,
        observation: Option<rig::observe::AdapterContext>,
    ) -> Reply {
        let mut attempt = 0usize;
        loop {
            let opened = match &observation {
                Some(context) => self.inner.stream_observed(request.clone(), context.clone()),
                None => self.inner.stream(request.clone()),
            };
            let mut relay = match opened {
                Ok(stream) => stream.into_relay(),
                Err(error) => {
                    if !self.should_retry(classify(&error), attempt) {
                        return Reply::Outcome(Err(ErrorReport::from(&error)));
                    }
                    self.acc.record_transient_retry();
                    tokio::time::sleep(self.policy.backoff(attempt)).await;
                    attempt += 1;
                    continue;
                }
            };
            match relay.next().await {
                // 最初の item が error = provider が reply を開く前に失敗した
                // のと等価 — まだ何も emit していないので retry 可能。
                Some(Err(report)) if self.should_retry(classify_report(&report), attempt) => {
                    self.acc.record_transient_retry();
                    tokio::time::sleep(self.policy.backoff(attempt)).await;
                    attempt += 1;
                }
                first => {
                    // 先読みした 1 item を繋ぎ戻して残りをそのまま流す。
                    let rest = futures::stream::iter(first).chain(relay);
                    return Reply::Stream(Box::pin(rest));
                }
            }
        }
    }
}

fn wrong_family(kind: &EffectKind) -> ErrorReport {
    ErrorReport::new(
        rig::error::ErrorKind::HandlerUnavailable,
        format!(
            "a completion handler cannot serve a `{}` effect",
            kind.name()
        ),
    )
}

impl Serve for RetryingCompletion {
    type Family = effect::family::Completion;

    fn descriptor(&self) -> HandlerDescriptor {
        HandlerDescriptor {
            key: effect::model_key(self.label.as_str()),
            family: FamilyDescriptor::Completion {
                model: self.label.clone(),
                capabilities: self.inner.capabilities().completion,
            },
            layers: Vec::new(),
        }
    }

    async fn serve(&self, kind: EffectKind, dispatch: Dispatch) -> Reply {
        let observation = dispatch.adapter_context();
        match kind {
            EffectKind::Completion {
                request,
                stream: false,
            } => Reply::Outcome(
                self.call_with_retry(request, observation)
                    .await
                    .map(Outcome::Completion)
                    .map_err(|error| ErrorReport::from(&error)),
            ),
            EffectKind::Completion {
                request,
                stream: true,
            } => self.stream_with_retry(request, observation).await,
            other => Reply::Outcome(Err(wrong_family(&other))),
        }
    }
}
