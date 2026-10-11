//! `WorkerRunError`(run.ts line 52-62 相当)— 分類済み error + `runMeta`。
//!
//! TS では `WorkerRunError extends Error` に `code`/`details`/`cause`/
//! `runMeta` を直接持たせる。Rust では内部に `CheaperError` を保持し
//! cause chain まで保全する(`cause` は `std::error::Error::source`
//! 経由で辿れる)。

use std::error::Error;
use std::fmt;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_tools::execution_metrics::ExecutionMetrics;
use cheaper_tools::journal::ChangesSummary;
use serde::Serialize;
use serde_json::{Map, Value};

use super::meta::{ModelMeta, WorkerRunMeta, WorkerSavings, WorkerUsage};

/// `Partial<WorkerRunMeta> & {durationMs?}` — error path の部分 meta。
/// full meta が構築できない段階(model 未解決等)では `execution` のみ
/// 入ることがある(TS outer catch が `{execution}` のみ付けるのと同じ)。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialRunMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<WorkerUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejected_tool_calls: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repairs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transient_retries: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<ChangesSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ExecutionMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub savings: Option<WorkerSavings>,
    /// CLI 側で付与される run 全体所要時間(run.ts では設定しない)。
    #[serde(rename = "durationMs", skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<f64>,
}

impl From<WorkerRunMeta> for PartialRunMeta {
    fn from(meta: WorkerRunMeta) -> Self {
        Self {
            model: Some(meta.model),
            usage: Some(meta.usage),
            turns: Some(meta.turns),
            tool_calls: Some(meta.tool_calls),
            rejected_tool_calls: Some(meta.rejected_tool_calls),
            repairs: Some(meta.repairs),
            transient_retries: Some(meta.transient_retries),
            changes: Some(meta.changes),
            execution: Some(meta.execution),
            savings: Some(meta.savings),
            duration_ms: meta.duration_ms,
        }
    }
}

/// `workerRunError(error, meta)` 相当。`error` は分類済みの
/// `CheaperError`(code・details・cause chain を保全)。
/// `runMeta` まで持つと大きくなるため内部は Box 化している
/// (`Result` の `Err` variant size を小さく保つ)。
#[derive(Debug)]
pub struct WorkerRunError {
    inner: Box<WorkerRunErrorInner>,
}

#[derive(Debug)]
struct WorkerRunErrorInner {
    error: CheaperError,
    run_meta: Option<PartialRunMeta>,
}

impl WorkerRunError {
    pub fn new(error: CheaperError, run_meta: Option<PartialRunMeta>) -> Self {
        Self {
            inner: Box::new(WorkerRunErrorInner { error, run_meta }),
        }
    }

    pub fn code(&self) -> ErrorCode {
        self.inner.error.code()
    }

    pub fn message(&self) -> &str {
        self.inner.error.message()
    }

    pub fn details(&self) -> Option<&Map<String, Value>> {
        self.inner.error.details()
    }

    /// `error.runMeta`。
    pub fn run_meta(&self) -> Option<&PartialRunMeta> {
        self.inner.run_meta.as_ref()
    }

    /// `error.exitCode` — CLI 層が exit code に使う。
    pub fn exit_code(&self) -> u8 {
        self.inner.error.exit_code()
    }

    /// 内部 `CheaperError`(cause chain 含む)への参照。
    pub fn error(&self) -> &CheaperError {
        &self.inner.error
    }

    /// `error` 本体を取り出す(envelope 変換等)。
    pub fn into_error(self) -> CheaperError {
        self.inner.error
    }
}

impl fmt::Display for WorkerRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner.error, f)
    }
}

impl Error for WorkerRunError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner.error)
    }
}
