//! `cheaper-tools`: port of `packages/tool-runtime` +
//! `packages/runtime-pi/src/tools`.
//!
//! Canonical tool runtime boundary per the design doc crate table: the
//! canonical 8 tools, tool scheduler, budget/accounting, mutation
//! journal, and artifact ports. Depends on `cheaper-core` only; tokio is
//! allowed for process control and scheduling. Module layout mirrors the
//! Phase 2 plan File Map.
//!
//! `ToolError` is the crate-wide erased error for thrown values — the
//! Rust stand-in for TS `unknown` throws (`new Error(...)` / `CheaperError` /
//! `WorkspaceEscapeError`). `walker::is_access_denied_error` classifies
//! boundary/policy/scope denials for `rejectedToolCalls` accounting by
//! message equality, matching the TS contract.

/// tool 層で throw される error の型消去表現(TS の `unknown` throw 相当)。
pub type ToolError = Box<dyn std::error::Error + Send + Sync>;

pub mod access;
pub mod accounting;
pub mod artifact;
pub mod catalog;
pub mod definition;
pub mod execution_metrics;
pub mod glob;
pub mod invocation;
pub mod journal;
pub mod js16;
pub mod limits;
pub mod line_framer;
pub mod operation;
pub mod registry;
pub mod resources;
pub mod scheduler;
pub mod tools;
pub mod walker;
