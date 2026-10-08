//! Re-export shim — 実装は Phase 4 で [`cheaper_core::json::stringify`]
//! へ移設された(JS `JSON.stringify` 互換 serializer を CLI envelope
//! 層も使うため、canonical formatter と同じ core に集約)。
//! 既存の `crate::js_json::*` / `cheaper_rig_adapter::js_json::*` 参照は
//! そのまま動く。

pub use cheaper_core::json::stringify::{stringify, stringify_pretty};
