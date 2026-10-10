//! `cheaper-cli` — `packages/cli` の Rust port。
//!
//! 構成(TS 対応):
//! - `cli.rs` — `runCli` dispatch 本体 + `CliIo`/`CliHooks`(`cli.ts`)
//! - `args.rs` — `parseCliArgs`(`args.ts`)
//! - `envelope.rs` — 4 形態の envelope serialize(`envelope.ts`)
//! - `input.rs` — `readTaskInput`(`input.ts`)
//! - `count.rs` — `countWorkspaceFileTokens`(`count.ts`)
//! - `embedded.rs` — builtin resource 埋込 + version 解決
//!   (`count.ts`/`version.ts`/`sea-main.ts`)
//! - `fake_stream.rs` — `CHEAPER_TEST_FAKE_STREAM` seam
//!   (`cli.ts::loadTestStreamFn`)。`dev-seam` cargo feature でのみ
//!   compile 対象(ADR-034DyGyITd0YaATg8fkld7)
//! - `usage_log.rs` — `usage.jsonl` record(`usage-log.ts`)
//! - `usage_report.rs` / `pricing.rs` — `cheaper usage` 集計
//!   (`usage-report.ts`/`pricing.ts`, T4)
//! - `text_format.rs` — `--format text`(`text-format.ts`)
//! - `run.rs` / `models.rs` / `auth.rs` — command 本体(T3/T4/T5)
//! - `hook/` — hook entry(T6)
//! - `workflow_context.rs` / `workflow_run.rs` — workflow command(T7)
//!
//! process entry(`main.rs`)は panic 境界・`__grep-eval` 早期分岐・
//! tokio runtime のみを持ち、command 実装は全てこちらに置く。

pub mod args;
pub mod auth;
pub mod cli;
pub mod count;
pub mod embedded;
pub mod envelope;
#[cfg(feature = "dev-seam")]
pub mod fake_stream;
pub mod hook;
pub mod input;
pub mod models;
pub mod pricing;
pub mod run;
pub mod text_format;
pub mod usage_log;
pub mod usage_report;
pub mod workflow_context;
pub mod workflow_run;
