//! Port of `packages/core/src/token/count.ts`.
//!
//! `o200k_base` token counting via `tiktoken-rs` (js-tiktoken
//! replacement); the 16 MiB byte cap, utf8-lossy decode, and
//! `FileTooLargeError` are preserved. js-tiktoken's `encode(text)`
//! rejects inputs containing an o200k special-token string
//! (`disallowedSpecial: "all"`), so `encode_ordinary` is preceded by a
//! literal special-token scan producing [`SpecialTokenError`].

use crate::errors::{node_io_error, NodeIoError};
use crate::workspace::paths::{resolve_workspace_path, SymlinkPolicy, WorkspaceEscapeError};

/// `DEFAULT_COUNT_MAX_BYTES` (16 MiB, ADR-0015). Exceeding it fails
/// before read/encode.
pub const DEFAULT_COUNT_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// `FileTooLargeError` (ADR-0015 / DESIGN-0017): a plain error (not
/// `CheaperError`); callers (CLI/tool) convert it to a dedicated code or
/// a tool error.
#[derive(Debug)]
pub struct FileTooLargeError {
    path: String,
    bytes: u64,
    max_bytes: u64,
}

impl FileTooLargeError {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }
}

impl std::fmt::Display for FileTooLargeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "file exceeds byte cap for token counting: {} ({} bytes > {})",
            self.path, self.bytes, self.max_bytes
        )
    }
}

impl std::error::Error for FileTooLargeError {}

/// js-tiktoken `encode(text)` failure for disallowed special tokens: a
/// plain `Error` (not a `CheaperError`) — callers (CLI/tool) map it to
/// INTERNAL_ERROR. `Display` reproduces the js-tiktoken message.
#[derive(Debug)]
pub struct SpecialTokenError {
    token: String,
}

impl SpecialTokenError {
    /// The special-token string found in the input.
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl std::fmt::Display for SpecialTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The text contains a special token that is not allowed: {}",
            self.token
        )
    }
}

impl std::error::Error for SpecialTokenError {}

/// `TOKENIZER_NAME`.
pub const TOKENIZER_NAME: &str = "o200k_base";
/// `TOKENIZER_IMPLEMENTATION` — the migration swaps `js-tiktoken` for
/// `tiktoken-rs` (token-count parity is proven by golden fixtures).
pub const TOKENIZER_IMPLEMENTATION: &str = "tiktoken-rs";
/// `TOKENIZER_VERSION` — the vendored `tiktoken-rs` version
/// (Cargo.toml pin).
pub const TOKENIZER_VERSION: &str = "0.12.1";

/// `TokenizerIdentity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct TokenizerIdentity {
    pub name: &'static str,
    pub implementation: &'static str,
    pub version: &'static str,
}

/// `TOKENIZER_IDENTITY`.
pub const TOKENIZER_IDENTITY: TokenizerIdentity = TokenizerIdentity {
    name: TOKENIZER_NAME,
    implementation: TOKENIZER_IMPLEMENTATION,
    version: TOKENIZER_VERSION,
};

/// `FileTokenCount`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTokenCount {
    /// Workspace-relative POSIX display path.
    pub path: String,
    pub tokens: usize,
    pub bytes: u64,
    pub tokenizer: TokenizerIdentity,
}

fn encoding() -> &'static tiktoken_rs::CoreBPE {
    // BPE rank table is loaded once at module scope in TS; a singleton
    // here keeps the same lifecycle.
    static ENC: std::sync::OnceLock<tiktoken_rs::CoreBPE> = std::sync::OnceLock::new();
    ENC.get_or_init(|| tiktoken_rs::o200k_base().expect("o200k_base loads"))
}

/// js-tiktoken `disallowedSpecial: "all"` check: the literal
/// special-token strings of the encoding (`<|endoftext|>`,
/// `<|endofprompt|>` for o200k_base) may not occur in the input. The
/// reported token is the leftmost match, as with the JS regex scan.
fn reject_special_tokens(text: &str) -> Result<(), SpecialTokenError> {
    let found = encoding()
        .special_tokens()
        .iter()
        .filter_map(|tok| text.find(*tok).map(|i| (i, *tok)))
        .min_by_key(|(i, _)| *i)
        .map(|(_, tok)| tok);
    match found {
        Some(tok) => Err(SpecialTokenError {
            token: tok.to_string(),
        }),
        None => Ok(()),
    }
}

/// `countTextTokens` — canonical token count of any string
/// (`o200k_base`), for non-file measurement (tool output, result JSON).
/// Like the TS function this fails (plain error, not `CheaperError`)
/// when the text contains an o200k special-token string.
pub fn count_text_tokens(text: &str) -> Result<usize, SpecialTokenError> {
    reject_special_tokens(text)?;
    Ok(encoding().encode_ordinary(text).len())
}

/// `CountFileTokensOptions`.
pub struct CountFileTokensOptions<'a> {
    /// Canonicalized workspace root.
    pub root: &'a str,
    /// Workspace-relative or absolute path (boundary-resolved).
    pub path: &'a str,
    pub symlink_policy: SymlinkPolicy,
    /// Byte cap (`None` = `DEFAULT_COUNT_MAX_BYTES`).
    pub max_bytes: Option<u64>,
}

/// `countFileTokens` error: boundary escape, byte-cap, disallowed
/// special token, or IO failure.
#[derive(Debug)]
pub enum CountTokensError {
    Escape(WorkspaceEscapeError),
    TooLarge(FileTooLargeError),
    SpecialToken(SpecialTokenError),
    /// Node fs `UVException` 相当(`{code, message}` を保持する —
    /// `describeError` parity のため raw `io::Error` ではなく
    /// [`NodeIoError`] を保持する)。
    Io(NodeIoError),
}

impl std::fmt::Display for CountTokensError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Escape(e) => e.fmt(f),
            Self::TooLarge(e) => e.fmt(f),
            Self::SpecialToken(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for CountTokensError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Escape(e) => Some(e),
            Self::TooLarge(e) => Some(e),
            Self::SpecialToken(e) => Some(e),
            Self::Io(e) => Some(e),
        }
    }
}

impl From<WorkspaceEscapeError> for CountTokensError {
    fn from(e: WorkspaceEscapeError) -> Self {
        Self::Escape(e)
    }
}

impl From<FileTooLargeError> for CountTokensError {
    fn from(e: FileTooLargeError) -> Self {
        Self::TooLarge(e)
    }
}

impl From<SpecialTokenError> for CountTokensError {
    fn from(e: SpecialTokenError) -> Self {
        Self::SpecialToken(e)
    }
}

impl From<NodeIoError> for CountTokensError {
    fn from(e: NodeIoError) -> Self {
        Self::Io(e)
    }
}

/// `countFileTokens` (DESIGN-0012 / ADR-0011): boundary + symlink policy
/// via `resolveWorkspacePath`, utf8-lossy decode, `o200k_base` encode.
/// The file body is not returned.
pub fn count_file_tokens(
    opts: &CountFileTokensOptions<'_>,
) -> Result<FileTokenCount, CountTokensError> {
    let resolved = resolve_workspace_path(opts.root, opts.path, opts.symlink_policy)?;
    let max_bytes = opts.max_bytes.unwrap_or(DEFAULT_COUNT_MAX_BYTES);
    // Node `fs.stat`/`fs.readFile` の UVException shape を再現するため
    // `node_io_error` で wrap する(TS は raw fs error をそのまま投げる)。
    let info =
        std::fs::metadata(&resolved.real).map_err(|e| node_io_error("stat", &resolved.real, e))?;
    if info.len() > max_bytes {
        return Err(FileTooLargeError {
            path: resolved.display,
            bytes: info.len(),
            max_bytes,
        }
        .into());
    }
    let buf =
        std::fs::read(&resolved.real).map_err(|e| node_io_error("open", &resolved.real, e))?;
    let text = String::from_utf8_lossy(&buf);
    reject_special_tokens(&text)?;
    let tokens = encoding().encode_ordinary(&text).len();
    Ok(FileTokenCount {
        path: resolved.display,
        tokens,
        bytes: buf.len() as u64,
        tokenizer: TOKENIZER_IDENTITY,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_known_strings() {
        // Values verified against js-tiktoken o200k_base (pinned in the
        // golden fixtures slice; these are regression anchors).
        assert_eq!(count_text_tokens("").unwrap(), 0);
        assert_eq!(count_text_tokens("hello").unwrap(), 1);
        assert!(count_text_tokens("こんにちは").unwrap() > 0);
    }

    #[test]
    fn rejects_special_tokens_like_js_tiktoken() {
        // js-tiktoken `encode` throws on disallowed special tokens.
        let e = count_text_tokens("a <|endoftext|> b").unwrap_err();
        assert_eq!(e.token(), "<|endoftext|>");
        assert_eq!(
            e.to_string(),
            "The text contains a special token that is not allowed: <|endoftext|>"
        );
        let e = count_text_tokens("<|endofprompt|>").unwrap_err();
        assert_eq!(e.token(), "<|endofprompt|>");
        // Similar-looking but not-special text still counts.
        assert!(count_text_tokens("<|endoftext|").is_ok());
    }

    #[test]
    fn file_count_uses_display_path_and_byte_cap() {
        let dir = tempfile::tempdir().unwrap();
        let root =
            crate::workspace::paths::canonicalize_workspace_root(dir.path().to_str().unwrap())
                .unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello world").unwrap();
        let out = count_file_tokens(&CountFileTokensOptions {
            root: &root,
            path: "a.txt",
            symlink_policy: SymlinkPolicy::Deny,
            max_bytes: None,
        })
        .unwrap();
        assert_eq!(out.path, "a.txt");
        assert_eq!(out.bytes, 11);
        assert_eq!(out.tokens, count_text_tokens("hello world").unwrap());
        assert_eq!(out.tokenizer.name, "o200k_base");

        // Cap exceeded → FileTooLargeError.
        let err = count_file_tokens(&CountFileTokensOptions {
            root: &root,
            path: "a.txt",
            symlink_policy: SymlinkPolicy::Deny,
            max_bytes: Some(5),
        })
        .unwrap_err();
        match err {
            CountTokensError::TooLarge(e) => {
                assert_eq!(e.path(), "a.txt");
                assert_eq!(e.bytes(), 11);
                assert_eq!(e.max_bytes(), 5);
                assert!(e.to_string().contains("a.txt (11 bytes > 5)"));
            }
            other => panic!("expected TooLarge, got {other:?}"),
        }
        // Escape propagates.
        assert!(matches!(
            count_file_tokens(&CountFileTokensOptions {
                root: &root,
                path: "../x",
                symlink_policy: SymlinkPolicy::Deny,
                max_bytes: None,
            }),
            Err(CountTokensError::Escape(_))
        ));
    }
}
