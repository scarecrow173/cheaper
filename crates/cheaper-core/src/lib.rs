//! `cheaper-core`: pure-logic port of `packages/core`.
//!
//! Rig-independent and tokio-independent (sync, `std::fs`-based) by design;
//! see `docs/designs/rust-rig-runtime-migration-architecture.md` crate table.
//! Module layout mirrors `packages/core/src/`.

pub mod config;
pub mod duration;
pub mod errors;
pub mod js;
pub mod json;
pub mod model;
mod nodepath;
pub mod policy;
pub mod resources;
pub mod token;
pub mod tools;
pub mod worker;
pub mod workspace;
pub(crate) mod yaml;
