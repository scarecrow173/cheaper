//! Sink construction: the rolling file appender under
//! `<log_root>/.cheaper/logs` and the stderr writer.
//!
//! File sink defaults (design): `Rotation::DAILY` + `max_log_files(7)`
//! + `filename_prefix("cheaper.log")` -> `cheaper.log.YYYY-MM-DD`.
//!
//! `filename_prefix` is required, not cosmetic — `max_log_files`
//! retention only considers files whose name matches the prefix, so an
//! un-prefixed roller would never prune old generations.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

/// File sink directory: `<log_root>/.cheaper/logs`.
pub(crate) fn log_dir(log_root: &Path) -> PathBuf {
    log_root.join(".cheaper").join("logs")
}

/// The non-blocking file writer plus the `WorkerGuard` that flushes it
/// on drop.
pub(crate) struct FileSink {
    pub(crate) writer: NonBlocking,
    pub(crate) guard: WorkerGuard,
}

/// Build the daily rolling file sink at `<log_root>/.cheaper/logs`.
/// Fails if the directory cannot be created or the appender cannot be
/// built — the caller degrades to stderr-only (fail-open).
pub(crate) fn build_file_sink(log_root: &Path) -> Result<FileSink, Box<dyn Error + Send + Sync>> {
    let dir = log_dir(log_root);
    fs::create_dir_all(&dir)?;
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("cheaper.log")
        .max_log_files(7)
        .build(&dir)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    Ok(FileSink { writer, guard })
}
