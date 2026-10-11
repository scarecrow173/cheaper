//! `cheaper-core`: pure-logic port of `packages/core`.
//!
//! Rig-independent and tokio-independent (sync, `std::fs`-based) by design;
//! see `docs/designs/rust-rig-runtime-migration-architecture.md` crate table.
//! Module layout mirrors `packages/core/src/`.

pub mod config;
pub mod duration;
pub mod errors;
pub mod js;
pub mod js16;
pub mod json;
pub mod model;
/// Node `path` 字句互換層。tool 層(`cheaper-tools` の walker 等)も
/// `path.join`/`path.relative`/`path.sep` の byte-level 意味論を必要とするため
/// 公開する(Windows path canonicalize 規則は core 責務の設計固定ルール)。
pub mod nodepath;
pub mod policy;
pub mod resources;
pub mod token;
pub mod tools;
pub mod worker;
pub mod workspace;
pub(crate) mod yaml;
