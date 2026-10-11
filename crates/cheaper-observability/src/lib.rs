//! `cheaper-observability`: single ownership boundary for the
//! observability foundation (ADR-034DUFzrXJxOwfl1qveXWR).
//!
//! Owns activation interpretation (`CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`
//! env vars + `--log`/`--log-format` flags), sink construction (daily
//! rolling file + stderr), format selection, level filter wiring,
//! subscriber install, and the flush guard. Depends only on
//! `tracing`/`tracing-subscriber`/`tracing-appender` + `std` — the
//! dependency direction is `everything -> cheaper-observability ->
//! tracing-subscriber`, so this crate must never depend on a
//! workspace-internal crate.

mod config;
mod init;
mod sink;

use std::path::PathBuf;

pub use config::{resolve, LogFormat, LogLevel, ResolvedConfig};
pub use init::init;

/// Activation input — the caller (cli) passes raw flag/env values;
/// interpretation, precedence, and validation are this crate's
/// responsibility.
pub struct Activation {
    /// `--log <level>` value (`None` when the flag is absent).
    pub flag_level: Option<String>,
    /// `--log-format <text|json>` value.
    pub flag_format: Option<String>,
    /// `CHEAPER_LOG` env value.
    pub env_level: Option<String>,
    /// `CHEAPER_LOG_FORMAT` env value.
    pub env_format: Option<String>,
    /// Anchor for `.cheaper/logs` — the workspace root, resolved by the
    /// caller (`--cwd`/process cwd already applied).
    pub log_root: PathBuf,
    /// `false` on the hook path — the stderr sink is then omitted
    /// entirely so hook stderr (part of the target native contract)
    /// cannot be polluted.
    pub stderr_enabled: bool,
}

/// Guard returned by a successful [`init`]. Dropping it flushes the
/// non-blocking file writer (via the held `WorkerGuard`). `init`
/// returns `None` whenever no subscriber was installed, so there is no
/// code path that needs this type without a live subscriber.
pub struct ObservabilityGuard {
    // Held only for its `Drop` side effect — `WorkerGuard` flushes the
    // non_blocking writer and joins its worker thread.
    pub(crate) _file_writer_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}
