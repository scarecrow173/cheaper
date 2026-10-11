//! dev/test seam — `dev-seam` cargo feature でのみ compile 対象に
//! なる module(ADR-034DyGyITd0YaATg8fkld7)。consumer install(default
//! feature)の binary・公開 API には含まれない。
//!
//! - `fake_provider` — scripted fake provider/session(T3 seam)。
//! - `fixture` — `RunFixture` JSON DSL + loader(fixture replay harness
//!   と cli `fake_stream` seam の素材)。

pub mod fake_provider;
pub mod fixture;
