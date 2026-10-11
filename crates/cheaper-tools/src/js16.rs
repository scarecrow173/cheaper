//! JS UTF-16 helper の thin re-export。実装は `cheaper_core::js16`
//! (core 側の JS/JSON compat layer 責務)に集約済み。`crate::js16` path
//! 互換のため残置する。

pub use cheaper_core::js16::*;
