//! `route` — PreToolUse hook・`cheaper route {resolve,run}` が共有する
//! route 宣言・解決・実行層(design §route module)。
//!
//! - `routes`: routes.yaml schema 検証・builtin/user/project の 3 層
//!   load・effective route set 構築。

pub mod log_store;
pub mod routes;
