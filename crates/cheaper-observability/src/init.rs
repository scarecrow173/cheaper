//! Subscriber install: `Registry` + file/stderr `fmt::Layer`s +
//! target-scoped level filter + `set_global_default`.
//!
//! Fail-open contract (design): `init` never panics and never fails
//! the caller — every problem is folded into the returned `warnings`
//! and logging either degrades or stays off.

use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::{self, writer::MakeWriter};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::{LookupSpan, Registry};
use tracing_subscriber::Layer;

use crate::config::{resolve, LogFormat, LogLevel};
use crate::sink;
use crate::{Activation, ObservabilityGuard};

/// Interpret `activation`, build the sinks, and install the global
/// subscriber.
///
/// Returns `(guard, warnings)`:
/// - resolved level `Off` → `(None, warnings)` — no dir creation, no
///   subscriber; all emit sites stay on tracing's no-subscriber fast
///   path.
/// - file sink construction failure → stderr-only (or file-only
///   inverted) + a warning entry. If no sink at all is available
///   (file failure AND `stderr_enabled == false`) → `(None,
///   warnings)` — installing a sinkless subscriber would only lock
///   out a later successful `init` (first subscriber wins) for no
///   benefit.
/// - `set_global_default` failure (a subscriber is already installed)
///   → **first subscriber wins**: `(None, warnings + "subscriber
///   already installed")`, keeping the first activation's
///   configuration. Not an error — in-process paths that call `init`
///   more than once (e.g. a test binary driving `run_cli` twice)
///   depend on this contract.
///
/// The `Vec<String>` holds warning message bodies; the caller renders
/// them with a `warning:` prefix on its diagnostics channel.
pub fn init(activation: &Activation) -> (Option<ObservabilityGuard>, Vec<String>) {
    let resolved = resolve(activation);
    let mut warnings = resolved.warnings;

    if resolved.level == LogLevel::Off {
        return (None, warnings);
    }

    // File sink — fail-open: construction failure degrades to
    // stderr-only instead of failing the command over a logging
    // concern.
    let mut file_writer_guard: Option<WorkerGuard> = None;
    let file_layer = match sink::build_file_sink(&activation.log_root) {
        Ok(file_sink) => {
            file_writer_guard = Some(file_sink.guard);
            Some(fmt_layer(file_sink.writer, resolved.format))
        }
        Err(err) => {
            warnings.push(format!(
                "cannot initialize file log sink at {:?}: {err} — continuing without the file sink",
                sink::log_dir(&activation.log_root)
            ));
            None
        }
    };

    // stderr sink — omitted entirely when disabled (hook path). A
    // layer that only discards output must not be installed: the
    // hook-path stderr non-contamination contract relies on this
    // structural omission.
    let stderr_layer = activation
        .stderr_enabled
        .then(|| fmt_layer(std::io::stderr, resolved.format));

    if file_layer.is_none() && stderr_layer.is_none() {
        warnings.push("no log sinks available — logging disabled".to_string());
        return (None, warnings);
    }

    let subscriber = Registry::default()
        .with(scoped_filter(resolved.level))
        .with(file_layer)
        .with(stderr_layer);

    // `set_global_default` succeeds at most once per process.
    // `fmt::try_init()` bundles this with `LogTracer::init()` — install
    // explicitly instead so a `log`-bridge failure cannot strand the
    // subscriber while this function reports failure and drops the
    // non-blocking writer's flush guard.
    match tracing::subscriber::set_global_default(subscriber) {
        Ok(()) => {
            // Best-effort `log` -> `tracing` bridge: a logger that is
            // already installed is not a reason to give up the sinks.
            // Forwarded `log` records keep their emitter's target
            // (reqwest, ureq, …), so the `cheaper`-scoped filter above
            // keeps them disabled — the bridge is harmless for
            // third-party payload leakage.
            if let Err(err) = tracing_log::LogTracer::init() {
                warnings.push(format!(
                    "log-crate bridge unavailable ({err}) — `log` records are not forwarded to the sinks"
                ));
            }
            (
                Some(ObservabilityGuard {
                    _file_writer_guard: file_writer_guard,
                }),
                warnings,
            )
        }
        Err(_) => {
            warnings.push(
                "subscriber already installed — keeping the first activation's configuration"
                    .to_string(),
            );
            (None, warnings)
        }
    }
}

/// Build the target-scoped `EnvFilter` for the resolved level.
///
/// All cheaper emit sites use either module-path targets
/// (`cheaper_cli::*`, `cheaper_runtime::*`, …) or explicit dot targets
/// (`cheaper.cli.*`, `cheaper.tool.*`, …) — the common string prefix
/// is `cheaper`. `EnvFilter` matches directive targets by string
/// prefix, so the single `cheaper=<level>` directive covers every
/// cheaper emit site while third-party targets (reqwest, ureq, hyper,
/// h2, rig-core, `log`-crate records bridged by `LogTracer`, …) match
/// no directive at all. This filter has no default directive —
/// `EnvFilter::default()` is built from zero directives — so unmatched
/// targets are disabled on BOTH sinks and the metadata-only contract
/// holds in practice.
///
/// `EnvFilter::new` is lossy (never panics); `as_directive` emits a
/// lowercase level word, so `cheaper=<level>` always parses.
fn scoped_filter(level: LogLevel) -> EnvFilter {
    EnvFilter::new(format!("cheaper={}", level.as_directive()))
}

/// Build one `fmt::Layer` for `writer` in the resolved format. Text
/// (`Format<Full>`) and JSON (`Format<Json>`) produce different
/// `fmt::Layer` types, so each branch is boxed into
/// `Box<dyn Layer<S> + Send + Sync>` for a single registration path.
/// `S` is generic because each `.with` call nests the subscriber type
/// further; the call sites let type inference pin it. The format is
/// shared by both sinks.
///
/// - ANSI is off on every sink: emitted lines stay plain text and
///   control characters can never leak into hook/pipe consumers.
/// - `FmtSpan::CLOSE` records stage timing (`time.busy`/`time.idle`)
///   at span close — one instrumentation site instead of manual
///   `Instant`/`elapsed` fields at every stage.
fn fmt_layer<S, W>(writer: W, format: LogFormat) -> Box<dyn Layer<S> + Send + Sync>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    match format {
        LogFormat::Text => fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .with_span_events(FmtSpan::CLOSE)
            .boxed(),
        LogFormat::Json => fmt::layer()
            .json()
            .with_writer(writer)
            .with_ansi(false)
            .with_span_events(FmtSpan::CLOSE)
            .boxed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tracing::callsite::{Callsite, Identifier};
    use tracing::field::FieldSet;
    use tracing::metadata::Kind;
    use tracing::{Level, Metadata};

    fn activation(log_root: PathBuf, level: Option<&str>) -> Activation {
        Activation {
            flag_level: level.map(str::to_string),
            flag_format: None,
            env_level: None,
            env_format: None,
            log_root,
            stderr_enabled: false,
        }
    }

    /// The ONLY test that calls `init` — `set_global_default` succeeds
    /// at most once per process, so no other test may install a
    /// subscriber. Covers the OFF path (no dir, no install), the first
    /// install, and the repeat-init contract in one ordered sequence.
    #[test]
    fn repeat_init_is_first_subscriber_wins() {
        let temp = tempfile::tempdir().unwrap();
        let log_root = temp.path().join("ws");

        // 1) OFF resolution installs nothing and creates no log dir.
        let (guard, warnings) = init(&activation(log_root.clone(), None));
        assert!(guard.is_none());
        assert!(warnings.is_empty());
        assert!(!sink::log_dir(&log_root).exists());

        // 2) First non-OFF activation installs the subscriber and
        //    creates the log dir — reachable after an earlier OFF
        //    resolution because OFF never installs.
        let (guard, warnings) = init(&activation(log_root.clone(), Some("debug")));
        let first_guard = guard.expect("first init must install the subscriber");
        assert!(warnings.is_empty());
        assert!(sink::log_dir(&log_root).exists());

        // 3) Repeat init: first subscriber wins — no new guard, and a
        //    warning explains that the existing configuration stays.
        let (guard, warnings) = init(&activation(log_root, Some("debug")));
        assert!(guard.is_none());
        assert!(warnings
            .iter()
            .any(|w| w.contains("subscriber already installed")));

        // Dropping the guard flushes the non_blocking file writer.
        drop(first_guard);
    }

    /// Minimal `Callsite` for building `Metadata` outside a macro
    /// callsite — the filter test only needs per-callsite `Interest`.
    struct TestCallsite;
    impl Callsite for TestCallsite {
        fn set_interest(&self, _: tracing::subscriber::Interest) {}
        fn metadata(&self) -> &Metadata<'_> {
            unreachable!("register_callsite only reads the passed metadata")
        }
    }
    static TEST_CALLSITE: TestCallsite = TestCallsite;

    /// The scoped filter must admit cheaper-owned targets and reject
    /// every third-party target — that is what keeps reqwest/hyper/
    /// rig-core payloads (and `log`-bridged records) out of the sinks.
    /// `register_callsite` exercises the static-directive path that
    /// real emit sites hit, without needing a `Context`.
    #[test]
    fn scoped_filter_admits_only_cheaper_targets() {
        let filter = scoped_filter(LogLevel::Debug);
        let interest = |target: &'static str, level: Level| {
            let meta: &'static Metadata<'static> = Box::leak(Box::new(Metadata::new(
                "test-event",
                target,
                level,
                None,
                None,
                None,
                FieldSet::new(&[], Identifier(&TEST_CALLSITE)),
                Kind::EVENT,
            )));
            Layer::<Registry>::register_callsite(&filter, meta)
        };

        // Module-path targets (`cheaper_*::*`) and explicit dot
        // targets (`cheaper.*`) share the `cheaper` prefix.
        assert!(interest("cheaper_runtime::run", Level::DEBUG).is_always());
        assert!(interest("cheaper.cli.run", Level::INFO).is_always());
        assert!(interest("cheaper_observability::init", Level::WARN).is_always());

        // The level gate still applies within cheaper targets.
        assert!(interest("cheaper_runtime::run", Level::ERROR).is_always());
        let trace_filter = scoped_filter(LogLevel::Error);
        let trace_meta: &'static Metadata<'static> = Box::leak(Box::new(Metadata::new(
            "test-event",
            "cheaper_runtime::run",
            Level::DEBUG,
            None,
            None,
            None,
            FieldSet::new(&[], Identifier(&TEST_CALLSITE)),
            Kind::EVENT,
        )));
        assert!(Layer::<Registry>::register_callsite(&trace_filter, trace_meta).is_never());

        // Third-party targets match no directive and the filter has
        // no default directive — disabled at ANY level.
        for target in [
            "reqwest::connect",
            "ureq",
            "hyper",
            "h2",
            "rig_core::agent",
            "log",
        ] {
            assert!(
                interest(target, Level::ERROR).is_never(),
                "third-party target {target:?} must be disabled"
            );
        }
    }
}
