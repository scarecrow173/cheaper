//! `cheaper-bench` — Rust-native benchmark harness(issue #183)。
//!
//! `benchmarks/run.mjs` + `lib/*.mjs`(TS/Node harness)を置き換える
//! scenario manifest v2 runner。design §1.1 に従い、全責務を lib 側に
//! 持たせ、`main.rs` は fixture-child dispatch → CLI parse → run の薄い
//! entry とする。`tests/` は本 lib API を直接検証する。
//!
//! module 対応:
//! - `cli` — arg parse・binary 解決・scenario×arm×(warmups+reps)ループ
//! - `manifest` — manifest v2 serde 型 + load(§2.1。fail-fast
//!   validation 本体は T2)
//! - `workspace` — observation 毎の temp workspace/cfgDir
//!   materialize(§2.4・files・models.yaml・policy.yaml section merge・
//!   routes.yaml・routed-exec copy)
//! - `env` — spawn env 組立(v1 `benchmarkEnvironment` parity)
//! - `spawn` — product binary / fixture child の blocking spawn
//!   (stdout/stderr capture・wall-time・exit code。nonzero exit で
//!   throw しない — §7)
//! - `fixture` — `CHEAPER_TEST_FAKE_STREAM` 用 RunFixture JSON 生成
//!   (`json!` 組立て + `RunFixture::parse` round-trip、§2.5)+ emit
//!   child の seeded 高エントロピ byte 生成器(§1.4)
//! - `child` — `--fixture-child <mode>` 自己再実行 mode(§1.4)
//! - `measure` — 観測 parser/計測(§3.3 — utility envelope・route
//!   trailer・hook JSON・usage.jsonl・armExpect 照合)
//! - `arms` — arm 実装語彙(§2.3 — 13 impl)の dispatch + spawn/観測

pub mod arms;
pub mod child;
pub mod cli;
pub mod env;
pub mod fixture;
pub mod manifest;
pub mod measure;
pub mod spawn;
pub mod workspace;
