//! `cheaper-provider` — bundled provider plugin 実装の集約 crate。
//!
//! 各 bundled provider は provider 名の module として内包する
//! （ADR-034r05iHrZUE6WoaNDNjvX 参照）。
//!
//! - [`devin`] — Devin Local provider plugin（Devin CLI delegated auth +
//!   isolated data dir）

pub mod devin;
