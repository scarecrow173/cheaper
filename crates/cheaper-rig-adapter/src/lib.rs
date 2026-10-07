//! `cheaper-rig-adapter` — Rig 0.43 への runtime adapter(Phase 3 T1)。
//!
//! 役割: `packages/runtime-pi` の session/run 構成を Rig `Agent` 上に再現
//! し、`cheaper-tools` の worker scope を tool として露出する。外部契約
//! (usage/turns/toolCalls/transientRetries の集計・error code・cancel
//! semantics)は TS runtime と一致させる。
//!
//! # Rig isolation
//!
//! Rig 型を import するのはこの crate と provider crate(例:
//! `cheaper-provider-devin`)のみ。公開 API の signature に Rig 型は出ない
//! — 唯一の例外は [`CompletionModel::from_rig`] で、これは provider→adapter
//! 境界の provider 側入口として明示的に存在する。
//!
//! # 所有権
//!
//! - tool の sandbox/policy/budget/scheduler/journal は `cheaper-tools` が
//!   担い、adapter は `ToolRuntime::execute` へ委譲するだけ。
//! - usage 集計は hook→[`Accumulator`] が唯一の収集源
//!   (`PromptResponse` は error/cancel 経路で得られないため)。
//! - run 全体の orchestration(abort grace・repair・output 抽出・
//!   `WorkerRunMeta` 構築)は T3(`cheaper-runtime`)の責務 — adapter は
//!   streaming run の primitive を提供するだけ。
//!
//! # 公開面
//!
//! - [`WorkerSessionSpec`] + [`RigSession::build`] で session 構築
//! - [`RigSession::stream_run`] が唯一の実行面(streaming のみ)
//! - [`RunEvent`]/[`RunStream`]/[`RunSnapshot`]/[`UsageSnapshot`] が
//!   外部に見える型
//! - [`build_system_prompt`]/[`build_user_prompt`] は prompt 契約の
//!   唯一の実装(DRY — T3 はここを呼ぶ)
//! - [`builtin_completion_model`] — catalog `api` 種別 → Rig provider
//!   client の builtin 組立(Phase 3 T7)。runtime 側の
//!   `CompletionModelFactory` 実装(Phase 4 CLI composition root も
//!   再利用)が呼ぶ。

mod accumulator;
mod agent;
mod builtin;
mod error;
mod hooks;
pub mod js_json;
mod retry;
mod run_stream;
mod tools;

pub use accumulator::{Accumulator, RunSnapshot, UsageSnapshot};
pub use agent::{
    build_system_prompt, build_user_prompt, CompletionModel, RigSession, WorkerSessionSpec,
    DEFAULT_INVALID_TOOL_RETRIES, DEFAULT_MAX_TURNS,
};
pub use builtin::{builtin_completion_model, BuiltinAuth, BuiltinModelSpec};
pub use error::{
    classify, classify_prompt, classify_report, prompt_error_to_cheaper, provider_error_to_cheaper,
    report_to_cheaper, streaming_error_to_cheaper, ErrorClass,
};
pub use retry::RetryPolicy;
pub use run_stream::{RunEvent, RunStream};

// `abort_reason_to_cheaper` は内部利用のみだが cancel 分類を一箇所に集約
// するため export(run_stream 経由で T3 が間接的に使う)。
pub use error::abort_reason_to_cheaper;
