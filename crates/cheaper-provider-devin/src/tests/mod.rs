//! Unit tests — TS `packages/runtime-pi` devin provider tests の
//! 振る舞いを Rust 側で再確認する。byte-level parity（wire）、
//! Connect framing、context map、catalog 変換、credentials/CLI、
//! JWT cache、plugin registration をカバーする。

mod cli_credentials;
mod context_map;
mod jwt;
mod models;
mod plugin;
mod rig_provider;
mod stream;
mod wire;
