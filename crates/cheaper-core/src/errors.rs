//! Port of `packages/core/src/errors.ts`.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One `ERROR_TABLE` entry: process exit code and retryability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorTableEntry {
    pub exit_code: u8,
    pub retryable: bool,
}

const fn entry(exit_code: u8, retryable: bool) -> ErrorTableEntry {
    ErrorTableEntry {
        exit_code,
        retryable,
    }
}

/// Verbatim port of `ERROR_TABLE` from `errors.ts` (declaration order preserved).
pub static ERROR_TABLE: &[(ErrorCode, ErrorTableEntry)] = &[
    (ErrorCode::InvalidRequest, entry(2, false)),
    (ErrorCode::InvalidInput, entry(2, false)),
    (ErrorCode::FileTooLarge, entry(2, false)),
    (ErrorCode::WorkerNotFound, entry(3, false)),
    (ErrorCode::InvalidWorkerDefinition, entry(3, false)),
    (ErrorCode::InvalidWorkerName, entry(3, false)),
    (ErrorCode::WorkerNameMismatch, entry(3, false)),
    (ErrorCode::UnknownTool, entry(3, false)),
    (ErrorCode::InvalidRuntimePolicy, entry(3, false)),
    (ErrorCode::OutputSchemaNotFound, entry(3, false)),
    (ErrorCode::InvalidOutputSchema, entry(3, false)),
    (ErrorCode::InputSchemaNotFound, entry(3, false)),
    (ErrorCode::InvalidInputSchema, entry(3, false)),
    (ErrorCode::InvalidModelConfig, entry(3, false)),
    (ErrorCode::PolicyViolation, entry(4, false)),
    (ErrorCode::ModelTierUnavailable, entry(5, true)),
    (ErrorCode::ProviderFailure, entry(5, true)),
    (ErrorCode::ToolExecutionFailure, entry(6, false)),
    (ErrorCode::InvalidOutput, entry(6, false)),
    (ErrorCode::MaxTurnsExceeded, entry(6, false)),
    (ErrorCode::MaxToolCallsExceeded, entry(6, false)),
    (ErrorCode::Timeout, entry(7, true)),
    (ErrorCode::Aborted, entry(8, false)),
    (ErrorCode::PartialMutation, entry(9, false)),
    (ErrorCode::InternalError, entry(70, false)),
];

/// `keyof typeof ERROR_TABLE`. Serialized as the original SCREAMING_SNAKE name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidRequest,
    InvalidInput,
    FileTooLarge,
    WorkerNotFound,
    InvalidWorkerDefinition,
    InvalidWorkerName,
    WorkerNameMismatch,
    UnknownTool,
    InvalidRuntimePolicy,
    OutputSchemaNotFound,
    InvalidOutputSchema,
    InputSchemaNotFound,
    InvalidInputSchema,
    InvalidModelConfig,
    PolicyViolation,
    ModelTierUnavailable,
    ProviderFailure,
    ToolExecutionFailure,
    InvalidOutput,
    MaxTurnsExceeded,
    MaxToolCallsExceeded,
    Timeout,
    Aborted,
    PartialMutation,
    InternalError,
}

impl ErrorCode {
    /// Canonical SCREAMING_SNAKE name (identical to the serde representation).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::InvalidInput => "INVALID_INPUT",
            Self::FileTooLarge => "FILE_TOO_LARGE",
            Self::WorkerNotFound => "WORKER_NOT_FOUND",
            Self::InvalidWorkerDefinition => "INVALID_WORKER_DEFINITION",
            Self::InvalidWorkerName => "INVALID_WORKER_NAME",
            Self::WorkerNameMismatch => "WORKER_NAME_MISMATCH",
            Self::UnknownTool => "UNKNOWN_TOOL",
            Self::InvalidRuntimePolicy => "INVALID_RUNTIME_POLICY",
            Self::OutputSchemaNotFound => "OUTPUT_SCHEMA_NOT_FOUND",
            Self::InvalidOutputSchema => "INVALID_OUTPUT_SCHEMA",
            Self::InputSchemaNotFound => "INPUT_SCHEMA_NOT_FOUND",
            Self::InvalidInputSchema => "INVALID_INPUT_SCHEMA",
            Self::InvalidModelConfig => "INVALID_MODEL_CONFIG",
            Self::PolicyViolation => "POLICY_VIOLATION",
            Self::ModelTierUnavailable => "MODEL_TIER_UNAVAILABLE",
            Self::ProviderFailure => "PROVIDER_FAILURE",
            Self::ToolExecutionFailure => "TOOL_EXECUTION_FAILURE",
            Self::InvalidOutput => "INVALID_OUTPUT",
            Self::MaxTurnsExceeded => "MAX_TURNS_EXCEEDED",
            Self::MaxToolCallsExceeded => "MAX_TOOL_CALLS_EXCEEDED",
            Self::Timeout => "TIMEOUT",
            Self::Aborted => "ABORTED",
            Self::PartialMutation => "PARTIAL_MUTATION",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }

    fn table_entry(self) -> ErrorTableEntry {
        ERROR_TABLE
            .iter()
            .find(|(code, _)| *code == self)
            .map(|(_, entry)| *entry)
            .expect("ERROR_TABLE covers every ErrorCode")
    }

    pub fn exit_code(self) -> u8 {
        self.table_entry().exit_code
    }

    pub fn retryable(self) -> bool {
        self.table_entry().retryable
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Port of `CheaperError`. `exitCode`/`retryable` are derived from
/// `ERROR_TABLE` at access time instead of being stored, which preserves the
/// same invariant as the TS constructor.
#[derive(Debug)]
pub struct CheaperError {
    code: ErrorCode,
    message: String,
    details: Option<Map<String, Value>>,
    cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl CheaperError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            cause: None,
        }
    }

    pub fn with_details(mut self, details: Map<String, Value>) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_cause(mut self, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn exit_code(&self) -> u8 {
        self.code.exit_code()
    }

    pub fn retryable(&self) -> bool {
        self.code.retryable()
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn details(&self) -> Option<&Map<String, Value>> {
        self.details.as_ref()
    }
}

impl fmt::Display for CheaperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CheaperError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Port of `isCheaperError` (downcast on a `dyn Error`).
pub fn is_cheaper_error(e: &(dyn std::error::Error + 'static)) -> bool {
    e.downcast_ref::<CheaperError>().is_some()
}

/// Map an `io::Error` to a Node/libuv errno code name for
/// `details.errno` and error Display. Shared by `policy::load`,
/// `model::config` and `worker::resolve`; `ENOENT`/`ENOTDIR` are
/// handled by the callers' absent-file checks — this renders the rest.
///
/// The mapping is driven by `raw_os_error` (POSIX errno values on unix,
/// Win32 error codes on Windows → the libuv `uv_translate_sys_error`
/// names), falling back to `ErrorKind` for constructed errors.
/// Residual: the Win32 table covers the fs-error codes reachable from
/// this code base; unmapped values fall back to the numeric string.
pub(crate) fn errno_string(e: &std::io::Error) -> String {
    #[cfg(unix)]
    let name = e.raw_os_error().and_then(|c| {
        Some(
            match c {
                1 => "EPERM",
                2 => "ENOENT",
                4 => "EINTR",
                5 => "EIO",
                9 => "EBADF",
                11 => "EAGAIN",
                13 => "EACCES",
                16 => "EBUSY",
                17 => "EEXIST",
                18 => "EXDEV",
                19 => "ENODEV",
                20 => "ENOTDIR",
                21 => "EISDIR",
                22 => "EINVAL",
                23 => "ENFILE",
                24 => "EMFILE",
                26 => "ETXTBSY",
                27 => "EFBIG",
                28 => "ENOSPC",
                29 => "ESPIPE",
                30 => "EROFS",
                31 => "EMLINK",
                32 => "EPIPE",
                36 => "ENAMETOOLONG",
                39 => "ENOTEMPTY",
                40 => "ELOOP",
                _ => return None,
            }
            .to_string(),
        )
    });
    #[cfg(windows)]
    let name = e.raw_os_error().and_then(|c| {
        Some(
            match c {
                // libuv `uv_translate_sys_error` names for the fs-relevant
                // Win32 codes (reading a directory is special-cased by
                // `io_errno_display`, which reports EISDIR).
                2 => "ENOENT",         // ERROR_FILE_NOT_FOUND
                3 => "ENOENT",         // ERROR_PATH_NOT_FOUND
                4 => "EMFILE",         // ERROR_TOO_MANY_OPEN_FILES
                5 => "EACCES",         // ERROR_ACCESS_DENIED
                6 => "EBADF",          // ERROR_INVALID_HANDLE
                8 => "ENOMEM",         // ERROR_NOT_ENOUGH_MEMORY
                17 => "EXDEV",         // ERROR_NOT_SAME_DEVICE
                19 => "EROFS",         // ERROR_WRITE_PROTECT
                32 => "EBUSY",         // ERROR_SHARING_VIOLATION
                80 => "EEXIST",        // ERROR_FILE_EXISTS
                82 => "EACCES",        // ERROR_CANNOT_MAKE
                112 => "ENOSPC",       // ERROR_DISK_FULL
                123 => "EINVAL",       // ERROR_INVALID_NAME
                145 => "ENOTEMPTY",    // ERROR_DIR_NOT_EMPTY
                183 => "EEXIST",       // ERROR_ALREADY_EXISTS
                206 => "ENAMETOOLONG", // ERROR_FILENAME_EXCED_RANGE
                267 => "ENOTDIR",      // ERROR_DIRECTORY
                1921 => "ELOOP",       // ERROR_CANT_RESOLVE_FILENAME
                _ => return None,
            }
            .to_string(),
        )
    });
    name.unwrap_or_else(|| match e.kind() {
        std::io::ErrorKind::PermissionDenied => "EACCES".to_string(),
        std::io::ErrorKind::IsADirectory => "EISDIR".to_string(),
        _ => e
            .raw_os_error()
            .map(|c| c.to_string())
            .unwrap_or_else(|| format!("{:?}", e.kind())),
    })
}

/// Node code name for a read/open failure on `path`. On Windows,
/// `read` on a directory surfaces `ERROR_ACCESS_DENIED` where libuv
/// reports `EISDIR` for Node's `fs.readFile` — check the path kind and
/// special-case it. (POSIX `read` yields `EISDIR` directly, and a
/// permission-denied directory must still report `EACCES`.)
pub(crate) fn io_errno_display(path: &str, e: &std::io::Error) -> String {
    #[cfg(windows)]
    if e.kind() == std::io::ErrorKind::PermissionDenied && std::path::Path::new(path).is_dir() {
        return "EISDIR".to_string();
    }
    let _ = path;
    errno_string(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: &[(ErrorCode, u8, bool)] = &[
        (ErrorCode::InvalidRequest, 2, false),
        (ErrorCode::InvalidInput, 2, false),
        (ErrorCode::FileTooLarge, 2, false),
        (ErrorCode::WorkerNotFound, 3, false),
        (ErrorCode::InvalidWorkerDefinition, 3, false),
        (ErrorCode::InvalidWorkerName, 3, false),
        (ErrorCode::WorkerNameMismatch, 3, false),
        (ErrorCode::UnknownTool, 3, false),
        (ErrorCode::InvalidRuntimePolicy, 3, false),
        (ErrorCode::OutputSchemaNotFound, 3, false),
        (ErrorCode::InvalidOutputSchema, 3, false),
        (ErrorCode::InputSchemaNotFound, 3, false),
        (ErrorCode::InvalidInputSchema, 3, false),
        (ErrorCode::InvalidModelConfig, 3, false),
        (ErrorCode::PolicyViolation, 4, false),
        (ErrorCode::ModelTierUnavailable, 5, true),
        (ErrorCode::ProviderFailure, 5, true),
        (ErrorCode::ToolExecutionFailure, 6, false),
        (ErrorCode::InvalidOutput, 6, false),
        (ErrorCode::MaxTurnsExceeded, 6, false),
        (ErrorCode::MaxToolCallsExceeded, 6, false),
        (ErrorCode::Timeout, 7, true),
        (ErrorCode::Aborted, 8, false),
        (ErrorCode::PartialMutation, 9, false),
        (ErrorCode::InternalError, 70, false),
    ];

    #[test]
    fn error_table_covers_every_expected_code_exactly_once() {
        assert_eq!(ERROR_TABLE.len(), EXPECTED.len());
        let mut table: Vec<_> = ERROR_TABLE.iter().map(|(c, _)| *c).collect();
        let mut expected: Vec<_> = EXPECTED.iter().map(|(c, _, _)| *c).collect();
        table.sort_by_key(|c| c.as_str());
        expected.sort_by_key(|c| c.as_str());
        assert_eq!(table, expected);
    }

    #[test]
    fn error_table_maps_each_code() {
        for (code, exit_code, retryable) in EXPECTED {
            let entry = ERROR_TABLE
                .iter()
                .find(|(c, _)| c == code)
                .map(|(_, e)| *e)
                .expect("code present");
            assert_eq!(entry.exit_code, *exit_code, "{code}");
            assert_eq!(entry.retryable, *retryable, "{code}");
        }
    }

    #[test]
    fn serde_name_matches_as_str() {
        for (code, _, _) in EXPECTED {
            let v = serde_json::to_value(code).unwrap();
            assert_eq!(v.as_str().unwrap(), code.as_str());
            let back: ErrorCode = serde_json::from_value(v).unwrap();
            assert_eq!(back, *code);
        }
    }

    #[test]
    fn cheaper_error_sets_code_exit_code_retryable_from_table() {
        let e = CheaperError::new(ErrorCode::Timeout, "timed out");
        assert_eq!(e.code(), ErrorCode::Timeout);
        assert_eq!(e.exit_code(), 7);
        assert!(e.retryable());
        assert_eq!(e.message(), "timed out");
        assert_eq!(e.to_string(), "timed out");
    }

    #[test]
    fn cheaper_error_carries_details_and_cause() {
        let mut details = Map::new();
        details.insert("a".to_string(), Value::from(1));
        let e = CheaperError::new(ErrorCode::InternalError, "x")
            .with_details(details)
            .with_cause(std::io::Error::other("root"));
        assert_eq!(e.details().unwrap()["a"], Value::from(1));
        assert_eq!(std::error::Error::source(&e).unwrap().to_string(), "root");
    }

    #[test]
    fn cheaper_error_omits_details_and_cause_when_not_given() {
        let e = CheaperError::new(ErrorCode::Aborted, "x");
        assert!(e.details().is_none());
        assert!(std::error::Error::source(&e).is_none());
    }

    #[test]
    fn is_cheaper_error_discriminates() {
        let e: Box<dyn std::error::Error> = Box::new(CheaperError::new(ErrorCode::Aborted, "x"));
        assert!(is_cheaper_error(e.as_ref()));
        let plain: Box<dyn std::error::Error> = Box::new(std::io::Error::other("x"));
        assert!(!is_cheaper_error(plain.as_ref()));
    }
}
