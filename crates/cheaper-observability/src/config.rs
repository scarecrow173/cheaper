//! Activation interpretation: `LogLevel`/`LogFormat` parsing and
//! flag > env > default precedence. This module never installs a
//! subscriber and never touches the filesystem.

use crate::Activation;

/// Minimal log level set accepted by `CHEAPER_LOG` / `--log`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogLevel {
    /// Logging disabled — no subscriber is installed and every emit
    /// site stays on tracing's no-subscriber fast path.
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    /// Parse a level value. `source` names where the value came from
    /// (`--log` or `CHEAPER_LOG`) for the warning message.
    fn parse(source: &str, raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(format!(
                "invalid {source} value {raw:?}: expected off|error|warn|info|debug|trace — logging disabled"
            )),
        }
    }

    /// `EnvFilter` directive for this level.
    pub(crate) fn as_directive(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

/// Output format accepted by `CHEAPER_LOG_FORMAT` / `--log-format`,
/// shared by both sinks (per-sink formats are out of scope — YAGNI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogFormat {
    /// `fmt::format::Format<Full>` human-readable single-line output.
    Text,
    /// `fmt::format::Format<Json>` single-line JSON.
    Json,
}

impl LogFormat {
    /// Parse a format value. `source` names where the value came from
    /// (`--log-format` or `CHEAPER_LOG_FORMAT`) for the warning
    /// message.
    fn parse(source: &str, raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err(format!(
                "invalid {source} value {raw:?}: expected text|json — using \"text\""
            )),
        }
    }
}

/// Fully interpreted activation — the non-installing view used by
/// [`init`](crate::init) for paths that need the resolved
/// configuration without side effects.
pub(crate) struct ResolvedConfig {
    /// Effective level. `Off` means logging is disabled.
    pub(crate) level: LogLevel,
    /// Effective format, shared by the file and stderr sinks.
    pub(crate) format: LogFormat,
    /// Interpretation warnings (message bodies — the caller adds the
    /// `warning:` prefix when rendering to a diagnostics channel).
    pub(crate) warnings: Vec<String>,
}

/// Interpret `activation` with **flag > env > default** precedence.
/// Never installs a subscriber and never touches the filesystem.
///
/// Fail-open rules:
/// - no level value anywhere → `Off` (no warning — logging is
///   opt-in).
/// - the winning level value is invalid → warning + `Off` (the
///   lower-precedence value is not retried).
/// - no format value → `Text` (no warning); the winning format value
///   is invalid → warning + `Text`.
pub(crate) fn resolve(activation: &Activation) -> ResolvedConfig {
    let mut warnings = Vec::new();

    let level_source = activation
        .flag_level
        .as_deref()
        .map(|raw| ("--log", raw))
        .or_else(|| {
            activation
                .env_level
                .as_deref()
                .map(|raw| ("CHEAPER_LOG", raw))
        });
    let level = match level_source {
        None => LogLevel::Off,
        Some((source, raw)) => match LogLevel::parse(source, raw) {
            Ok(level) => level,
            Err(warning) => {
                warnings.push(warning);
                LogLevel::Off
            }
        },
    };

    let format_source = activation
        .flag_format
        .as_deref()
        .map(|raw| ("--log-format", raw))
        .or_else(|| {
            activation
                .env_format
                .as_deref()
                .map(|raw| ("CHEAPER_LOG_FORMAT", raw))
        });
    let format = match format_source {
        None => LogFormat::Text,
        Some((source, raw)) => match LogFormat::parse(source, raw) {
            Ok(format) => format,
            Err(warning) => {
                warnings.push(warning);
                LogFormat::Text
            }
        },
    };

    ResolvedConfig {
        level,
        format,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn activation() -> Activation {
        Activation {
            flag_level: None,
            flag_format: None,
            env_level: None,
            env_format: None,
            log_root: PathBuf::from("/unused"),
            stderr_enabled: true,
        }
    }

    #[test]
    fn no_level_value_resolves_off_without_warning() {
        let resolved = resolve(&activation());
        assert_eq!(resolved.level, LogLevel::Off);
        assert_eq!(resolved.format, LogFormat::Text);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn env_level_activates_logging() {
        let mut activation = activation();
        activation.env_level = Some("debug".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.level, LogLevel::Debug);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn flag_level_overrides_env_level() {
        let mut activation = activation();
        activation.env_level = Some("debug".to_string());
        activation.flag_level = Some("error".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.level, LogLevel::Error);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn explicit_off_value_resolves_off() {
        let mut activation = activation();
        activation.flag_level = Some("off".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.level, LogLevel::Off);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn level_values_are_case_insensitive() {
        let mut activation = activation();
        activation.env_level = Some("DEBUG".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.level, LogLevel::Debug);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn invalid_env_level_warns_and_resolves_off() {
        let mut activation = activation();
        activation.env_level = Some("verbose".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.level, LogLevel::Off);
        assert_eq!(resolved.warnings.len(), 1);
        assert!(resolved.warnings[0].contains("CHEAPER_LOG"));
        assert!(resolved.warnings[0].contains("verbose"));
    }

    #[test]
    fn invalid_flag_level_warns_and_resolves_off() {
        let mut activation = activation();
        // flag > env even when the flag value is invalid — the
        // lower-precedence value is not retried (fail-open).
        activation.env_level = Some("debug".to_string());
        activation.flag_level = Some("bogus".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.level, LogLevel::Off);
        assert_eq!(resolved.warnings.len(), 1);
        assert!(resolved.warnings[0].contains("--log"));
        assert!(resolved.warnings[0].contains("bogus"));
    }

    #[test]
    fn env_format_selects_json() {
        let mut activation = activation();
        activation.env_format = Some("json".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.format, LogFormat::Json);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn flag_format_overrides_env_format() {
        let mut activation = activation();
        activation.env_format = Some("json".to_string());
        activation.flag_format = Some("text".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.format, LogFormat::Text);
        assert!(resolved.warnings.is_empty());
    }

    #[test]
    fn invalid_env_format_warns_and_resolves_text() {
        let mut activation = activation();
        activation.env_format = Some("yaml".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.format, LogFormat::Text);
        assert_eq!(resolved.warnings.len(), 1);
        assert!(resolved.warnings[0].contains("CHEAPER_LOG_FORMAT"));
        assert!(resolved.warnings[0].contains("yaml"));
    }

    #[test]
    fn invalid_flag_format_warns_and_resolves_text() {
        let mut activation = activation();
        activation.env_format = Some("json".to_string());
        activation.flag_format = Some("bogus".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.format, LogFormat::Text);
        assert_eq!(resolved.warnings.len(), 1);
        assert!(resolved.warnings[0].contains("--log-format"));
    }

    #[test]
    fn invalid_level_and_format_each_produce_a_warning() {
        let mut activation = activation();
        activation.env_level = Some("bogus".to_string());
        activation.env_format = Some("bogus".to_string());
        let resolved = resolve(&activation);
        assert_eq!(resolved.level, LogLevel::Off);
        assert_eq!(resolved.format, LogFormat::Text);
        assert_eq!(resolved.warnings.len(), 2);
    }
}
