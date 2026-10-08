//! Port of `packages/cli/src/hook/routing-policy.ts` — cheaper shared
//! routing policy (DESIGN-034rA79Uog7JOa7evw6rOL)。
//!
//! target adapter が native payload を canonical request へ正規化し、
//! `evaluate` が target 非依存の判定を行う: workspace containment、
//! byte-size fast path、in-process token 計測、token threshold、
//! FILE_TOO_LARGE fail-closed verdict。
//!
//! canonical request:
//!   `kind:"file-read"` → [`RoutingRequest::FileRead`]
//!   `kind:"shell"`     → [`RoutingRequest::Shell`]
//!
//! verdict は [`Verdict::Allow`] または [`Verdict::Deny{reason}`]
//! (multi-line guidance text)。
//!
//! `shell` request は保守的 tokenizer で read candidate (cat/head/tail/
//! less/more の file operand) を抽出し、各々を同じ file-read policy で
//! 評価する。

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use cheaper_core::config::dirs::EnvLookup;
use cheaper_core::json::number::format_number;
use cheaper_core::nodepath;
use cheaper_core::token::count::FileTooLargeError;

use crate::count::count_workspace_file_tokens;
use crate::embedded;

/// `DEFAULT_THRESHOLD` (8192)。
pub const DEFAULT_THRESHOLD: f64 = 8192.0;

/// Legacy subprocess timeout: JS policy は `cheaper count-tokens` を 30s
/// limit の subprocess で呼んでいた。in-process 版は計測を blocking
/// thread へ逃がし、hang した I/O (FIFO・dead network FS) でも verdict
/// が resolve するよう同じ 30s bound を掛ける。
const MEASURE_TIMEOUT: Duration = Duration::from_millis(30_000);

/// canonical routing request (`RoutingRequest`)。
///
/// `source_tool` は TS request shape の一部(`{kind,path,cwd,sourceTool}`)
/// として adapter が埋める provenance — policy 本体は読まない。
/// `cwd` は `request.cwd || process.cwd()` 規則に合わせ、空文字列を
/// 「未指定」とみなす。
pub enum RoutingRequest {
    FileRead {
        path: String,
        cwd: String,
        source_tool: Option<String>,
    },
    Shell {
        command: String,
        cwd: String,
        source_tool: Option<String>,
    },
}

/// routing verdict (`Verdict`)。
pub enum Verdict {
    Allow,
    Deny { reason: String },
}

/// `EvaluateOptions` — `env` は stdin stub と同様に `CliIo.env` を
/// そのまま渡し、`builtin_dir` は `CliHooks.builtin_dir`(未指定時は
/// `embedded::builtin_dir()` を計測時に遅延解決)。
#[derive(Clone, Copy)]
pub struct EvaluateOptions<'a> {
    pub env: EnvLookup<'a>,
    pub builtin_dir: Option<&'a PathBuf>,
}

/// `process.cwd()` 相当(計測 fail-open なので取得失敗時は空文字列)。
fn process_cwd() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `threshold()` — `CHEAPER_READ_TOKEN_THRESHOLD` を JS
/// `Number.parseInt(raw, 10)` 規則(部分 parse 許容)で読み、正の整数
/// のみ採用する。`st.size <= limit`・`tokens <= limit` の比較で f64
/// のまま使う(TS が number をそのまま比較するのと同じ)。
pub fn threshold(env: EnvLookup<'_>) -> f64 {
    let Some(raw) = env("CHEAPER_READ_TOKEN_THRESHOLD") else {
        return DEFAULT_THRESHOLD;
    };
    match js_parse_int_10(&raw) {
        Some(n) if n > 0.0 => n,
        _ => DEFAULT_THRESHOLD,
    }
}

/// JS `Number.parseInt(s, 10)`: leading whitespace + optional sign +
/// 連続する 10 進 digit のみ読む。digit が 1 つも無ければ `NaN`
/// (`None`)。戻り値は f64(指数 overflow も `Number` と同じ無限大)。
fn js_parse_int_10(s: &str) -> Option<f64> {
    // parseInt は leading whitespace を skip する(JS whitespace)。
    let trimmed = s.trim_start_matches(is_js_space);
    let bytes = trimmed.as_bytes();
    let mut idx = 0usize;
    let mut negative = false;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        negative = bytes[0] == b'-';
        idx = 1;
    }
    let digit_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == digit_start {
        return None; // NaN
    }
    let n: f64 = trimmed[digit_start..idx].parse().ok()?;
    Some(if negative { -n } else { n })
}

/// JS `\s`(regex)に相当する空白判定 — Unicode White_Space + U+FEFF
/// (ES2018 までの `\s` 集合。`js::is_js_trim_char` と同じ定義)。
fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

/// `count_workspace_file_tokens` が transitive に読む env key。
/// 計測は `spawn_blocking` へ送るため `EnvLookup`(`&dyn Fn`、非 Send)
/// を snapshot して閉包に写す — 読まれる key は
/// `resolve_user_config_dir`(`CHEAPER_CONFIG_DIR`/`XDG_CONFIG_HOME`/
/// `USERPROFILE`/`HOME`)と `count_max_bytes`(`CHEAPER_COUNT_MAX_BYTES`)
/// の閉じた集合(TS `opts.env` 経由と同じ値が渡る)。
const MEASURE_ENV_KEYS: &[&str] = &[
    "CHEAPER_CONFIG_DIR",
    "XDG_CONFIG_HOME",
    "USERPROFILE",
    "HOME",
    "CHEAPER_COUNT_MAX_BYTES",
];

/// `measureTokens` の結果。
enum Measured {
    Tokens(f64),
    FileTooLarge {
        bytes: u64,
        max_bytes: u64,
    },
    /// import 失敗・計測 error・timeout — fail-open(allow)。
    Failed,
}

/// `measureTokens` — byte fast path 通過後にのみ呼ばれる。
/// TS は dynamic import で tokenizer/policy 層の import cost をここに
/// 閉じ込めていた; Rust 版は `builtin_dir` 解決と policy/tokenizer 初期
/// 化を同じ境界(計測直前)へ遅延させる。error は生のまま呼び出し側へ
/// 畳み、`FileTooLargeError` だけ downcast で区別する(TS `instanceof`
/// 相当)。
async fn measure_tokens(resolved_path: &str, cwd: &str, opts: &EvaluateOptions<'_>) -> Measured {
    let builtin_dir = match opts.builtin_dir {
        Some(d) => d.clone(),
        // `opts?.builtinDir ?? defaultBuiltinDir` — 埋込 resource の
        // materialize に失敗しても計測失敗 → allow(fail-open)。
        None => match embedded::builtin_dir() {
            Ok(d) => d,
            Err(_) => return Measured::Failed,
        },
    };
    let resolved = resolved_path.to_string();
    let cwd = cwd.to_string();
    // `&dyn Fn` は Send でないため、計測経路が読む env をここで
    // snapshot して blocking thread へ受け渡す。
    let mut env_map: HashMap<String, String> = HashMap::new();
    for key in MEASURE_ENV_KEYS {
        if let Some(v) = (opts.env)(key) {
            env_map.insert((*key).to_string(), v);
        }
    }
    let measure = tokio::task::spawn_blocking(move || {
        let env_lookup = move |name: &str| env_map.get(name).cloned();
        count_workspace_file_tokens(&cwd, &resolved, &builtin_dir, &env_lookup)
    });
    match tokio::time::timeout(MEASURE_TIMEOUT, measure).await {
        Ok(Ok(Ok(counted))) => Measured::Tokens(counted.count.tokens as f64),
        Ok(Ok(Err(e))) => match e.downcast_ref::<FileTooLargeError>() {
            Some(ftl) => Measured::FileTooLarge {
                bytes: ftl.bytes(),
                max_bytes: ftl.max_bytes(),
            },
            None => Measured::Failed,
        },
        // JoinError(panic/cancel)も 30s timeout も計測失敗 → allow。
        Ok(Err(_)) | Err(_) => Measured::Failed,
    }
}

/// `evaluateFileRead` — measured verdict が唯一の fail-closed。
/// 全ての infrastructure 失敗は allow へ倒す。
async fn evaluate_file_read(path: &str, cwd: &str, opts: &EvaluateOptions<'_>) -> Verdict {
    let cwd = if cwd.is_empty() {
        process_cwd()
    } else {
        cwd.to_string()
    };
    // Node `path.resolve` は相対 base を process.cwd() に anchor する —
    // hook payload 由来の cwd が相対でも lexical 判定が壊れないよう
    // 同じ anchor を掛ける。
    let cwd_abs = if nodepath::is_absolute(&cwd) {
        cwd.clone()
    } else {
        nodepath::resolve(&process_cwd(), &cwd)
    };
    let resolved = nodepath::resolve(&cwd_abs, path);
    let rel = nodepath::relative(&cwd_abs, &resolved);
    // ".." 単独・"../" prefix は outside。"..foo.ts" のような
    // `..` 始まりの名前は workspace 内。
    if rel.is_empty()
        || rel == ".."
        || rel.starts_with(&format!("..{}", nodepath::SEP))
        || nodepath::is_absolute(&rel)
    {
        return Verdict::Allow;
    }
    let rel_posix = rel.replace(nodepath::SEP, "/");
    let md = match std::fs::metadata(&resolved) {
        Ok(md) => {
            if !md.is_file() {
                return Verdict::Allow;
            }
            md
        }
        Err(_) => return Verdict::Allow,
    };
    // Fast path (DESIGN-0017): tokens <= bytes が常に成り立つため、
    // byte size が threshold に収まる file は計測を skip できる
    // (builtin dir 解決・policy/tokenizer 初期化も走らない)。
    let limit = threshold(opts.env);
    if (md.len() as f64) <= limit {
        return Verdict::Allow;
    }

    match measure_tokens(&resolved, &cwd, opts).await {
        Measured::Failed => Verdict::Allow,
        Measured::FileTooLarge { bytes, max_bytes } => Verdict::Deny {
            reason: format!(
                "cheaper: '{rel_posix}' is {bytes} bytes (exceeds the {max_bytes}-byte measurement cap).\n\
                 Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n\
                 workspace-relative path: {rel_posix}"
            ),
        },
        Measured::Tokens(tokens) => {
            if tokens <= limit {
                return Verdict::Allow;
            }
            // Non-executable guidance only: file path を埋め込んだ
            // shell command は出さない(quoting が敵対的 filename で
            // 壊れる)。skill 側が JSON/stdin を組み立てるので
            // workspace-relative path のみ渡す。
            Verdict::Deny {
                reason: format!(
                    "cheaper: '{rel_posix}' has {} tokens (threshold {}).\n\
                     Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n\
                     workspace-relative path: {rel_posix}",
                    format_number(tokens),
                    format_number(limit),
                ),
            }
        }
    }
}

// --- shell tokenizer (issue #42, ADR-034rBhRB7QyWkAxRiTDXXq) -------------
// 実行・展開は一切しない。quote は token の区切りにのみ使い、shell
// metachar を含む token は `unsafe` として segment 全体を skip する
// (fail-open — 検出漏れはあっても誤 block はしない)。unterminated
// quote 等の unrecoverable input では `None` を返す。

/// `SEGMENT_UNSAFE` (`/[<>]/`)。
fn is_segment_unsafe(c: char) -> bool {
    c == '<' || c == '>'
}

/// `TOKEN_UNSAFE` (`/[$`*?[\]~(){}!]/`)。
fn is_token_unsafe(c: char) -> bool {
    matches!(
        c,
        '$' | '`' | '*' | '?' | '[' | ']' | '~' | '(' | ')' | '{' | '}' | '!'
    )
}

/// `ESCAPABLE` (`/[\s'"\\|&;<>$`*?[\](){}!#~]/`)。
/// backslash が次の char を escape するのは shell metachar/whitespace
/// の場合のみ — それ以外は literal として残し、`src\dir\file.txt`
/// のような Windows-style path を生かす。
fn is_escapable(c: char) -> bool {
    is_js_space(c)
        || matches!(
            c,
            '\'' | '"'
                | '\\'
                | '|'
                | '&'
                | ';'
                | '<'
                | '>'
                | '$'
                | '`'
                | '*'
                | '?'
                | '['
                | ']'
                | '('
                | ')'
                | '{'
                | '}'
                | '!'
                | '#'
                | '~'
        )
}

/// `ENV_PREFIX` (`/^[A-Za-z_][A-Za-z0-9_]*=/`)— prefix match。
fn is_env_prefix(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    for c in chars {
        if c == '=' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    false
}

/// `ShellToken`。
#[derive(Debug)]
pub struct ShellToken {
    pub text: String,
    pub unsafe_token: bool,
}

/// `ShellSegment`。
#[derive(Debug)]
pub struct ShellSegment {
    pub tokens: Vec<ShellToken>,
    pub unsafe_segment: bool,
}

/// `tokenizeShell` — `ShellSegment[] | null`。
/// JS 版は UTF-16 code unit で走査するが、対象 regex は全て ASCII のみ
/// match し、astral char は 2 code unit 連続 append と `chars()` の
/// 1 code point append で `text` が一致するため `chars` で等価。
pub fn tokenize_shell(command: &str) -> Option<Vec<ShellSegment>> {
    let chars: Vec<char> = command.chars().collect();
    let mut segments: Vec<ShellSegment> = Vec::new();
    let mut tokens: Vec<ShellToken> = Vec::new();
    let mut unsafe_segment = false;
    let mut text = String::new();
    let mut token_unsafe = false;
    // quote: None / Some('\'') / Some('"')
    let mut quote: Option<char> = None;
    let mut has_token = false;

    macro_rules! end_token {
        () => {
            if has_token {
                tokens.push(ShellToken {
                    text: std::mem::take(&mut text),
                    unsafe_token: token_unsafe,
                });
                token_unsafe = false;
                has_token = false;
            }
        };
    }
    macro_rules! end_segment {
        () => {
            end_token!();
            segments.push(ShellSegment {
                tokens: std::mem::take(&mut tokens),
                unsafe_segment,
            });
            unsafe_segment = false;
        };
    }

    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if quote == Some('\'') {
            if ch == '\'' {
                quote = None;
            } else {
                text.push(ch);
                has_token = true;
            }
            i += 1;
            continue;
        }
        if quote == Some('"') {
            if ch == '"' {
                quote = None;
            } else if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
                i += 1;
                text.push(chars[i]);
                has_token = true;
            } else {
                text.push(ch);
                has_token = true;
                if ch == '$' || ch == '`' {
                    token_unsafe = true;
                }
            }
            i += 1;
            continue;
        }
        // Unquoted context.
        if ch == '#' && !has_token {
            // Comment: skip to the next line. JS の `continue` は
            // `i++` を通るため `\n` 自体も comment の一部として
            // 消費され、segment 終端にはならない点に注意。
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if ch == ' ' || ch == '\t' || ch == '\r' {
            end_token!();
            i += 1;
            continue;
        }
        if ch == '\n' || ch == ';' {
            end_segment!();
            i += 1;
            continue;
        }
        if ch == '|' || ch == '&' {
            if chars.get(i + 1) == Some(&ch) {
                i += 1; // || or &&
            }
            end_segment!();
            i += 1;
            continue;
        }
        if is_segment_unsafe(ch) {
            // Redirection は segment 全体を解釈不能にする。
            unsafe_segment = true;
            i += 1;
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            has_token = true;
            i += 1;
            continue;
        }
        if ch == '\\' {
            if i + 1 < chars.len() && is_escapable(chars[i + 1]) {
                i += 1;
                text.push(chars[i]);
            } else {
                text.push(ch);
            }
            has_token = true;
            i += 1;
            continue;
        }
        if is_token_unsafe(ch) {
            token_unsafe = true;
        }
        text.push(ch);
        has_token = true;
        i += 1;
    }
    if quote.is_some() {
        return None; // unterminated quote
    }
    // 末尾の endToken/endSegment — reset は後続で読まれないため
    // 不要(JS の同名 closure と同じ push のみ行う)。
    if has_token {
        tokens.push(ShellToken {
            text: std::mem::take(&mut text),
            unsafe_token: token_unsafe,
        });
    }
    segments.push(ShellSegment {
        tokens: std::mem::take(&mut tokens),
        unsafe_segment,
    });
    Some(segments)
}

// Read command と separate value token を消費する option。
// `plus_options` は `+` prefix token も option とみなす command
// (less `+F`/`+123`、more `+/<pattern>`/`+<line>`)。
struct ReadCommandSpec {
    value_options: &'static [&'static str],
    plus_options: bool,
}

fn read_command_spec(name: &str) -> Option<ReadCommandSpec> {
    match name {
        "cat" => Some(ReadCommandSpec {
            value_options: &[],
            plus_options: false,
        }),
        "head" => Some(ReadCommandSpec {
            value_options: &["-n", "-c", "--lines", "--bytes"],
            plus_options: false,
        }),
        "tail" => Some(ReadCommandSpec {
            value_options: &[
                "-n",
                "-c",
                "-s",
                "--lines",
                "--bytes",
                "--sleep-interval",
                "--pid",
                "--max-unchanged-stats",
            ],
            plus_options: false,
        }),
        "less" => Some(ReadCommandSpec {
            value_options: &[
                "-o",
                "-O",
                "-p",
                "-k",
                "-P",
                "-t",
                "-T",
                "-D",
                "--log-file",
                "--pattern",
                "--tags",
                "--tag-file",
                "--prompt",
            ],
            plus_options: true,
        }),
        "more" => Some(ReadCommandSpec {
            value_options: &["-n", "-p", "-t"],
            plus_options: true,
        }),
        _ => None,
    }
}

/// `commandName` — dir prefix(`/`・`\` のどちらでも)を剥がし、
/// `.exe`/`.bat`/`.cmd` suffix (case-insensitive) を除去する。
/// lowercase 化はしない(`CAT` は read command と認識しない TS 規則)。
fn command_name(token: &str) -> String {
    let base = token.rsplit(['/', '\\']).next().unwrap_or(token);
    for ext in [".exe", ".bat", ".cmd"] {
        // JS `\.(exe|bat|cmd)$` — ".exe" 単体でも match し "" になる。
        if base.len() >= ext.len() && base.to_ascii_lowercase().ends_with(ext) {
            return base[..base.len() - ext.len()].to_string();
        }
    }
    base.to_string()
}

// 後続 segment の実効 cwd を変える command (POSIX builtin + cmd/
// PowerShell alias)。移動先は追跡しない — その後の read candidate は
// hook の cwd と異なる base で解決されるため ambiguous であり、誤
// block・誤 pass を避けて fail-open に倒す。
const CWD_CHANGING_COMMANDS: &[&str] = &["cd", "chdir", "pushd", "popd", "set-location", "sl"];

/// `extractReadCandidates` — shell command string から read command の
/// file operand を抽出する。parse 不能・安全に読める参照が無い場合は
/// `[]`(caller は fail-open として扱う)。重複は first-occurrence
/// order で dedupe (`[...new Set(candidates)]`)。
pub fn extract_read_candidates(command: &str) -> Vec<String> {
    let Some(segments) = tokenize_shell(command) else {
        return Vec::new();
    };
    let mut candidates: Vec<String> = Vec::new();
    let mut cwd_ambiguous = false;
    for segment in &segments {
        let tokens = &segment.tokens;
        let mut i = 0usize;
        while i < tokens.len() {
            if !is_env_prefix(&tokens[i].text) {
                break;
            }
            i += 1;
        }
        let Some(head) = tokens.get(i) else {
            continue;
        };
        if head.unsafe_token {
            continue;
        }
        if CWD_CHANGING_COMMANDS.contains(&command_name(&head.text).to_lowercase().as_str()) {
            cwd_ambiguous = true;
            continue;
        }
        if cwd_ambiguous || segment.unsafe_segment {
            continue;
        }
        let Some(spec) = read_command_spec(&command_name(&head.text)) else {
            continue;
        };
        let mut operands: Vec<String> = Vec::new();
        let mut options_ended = false;
        let mut skip_value = false;
        let mut abandoned = false;
        for tok in &tokens[i + 1..] {
            if skip_value {
                skip_value = false;
                continue;
            }
            if tok.unsafe_token {
                abandoned = true;
                break;
            }
            let t = &tok.text;
            if !options_ended {
                if t == "--" {
                    options_ended = true;
                    continue;
                }
                if t.starts_with('-') || (spec.plus_options && t.starts_with('+')) {
                    if spec.value_options.contains(&t.as_str()) {
                        skip_value = true;
                    }
                    continue;
                }
            }
            operands.push(t.clone());
        }
        if !abandoned {
            candidates.extend(operands);
        }
    }
    // `[...new Set(candidates)]` — insertion order dedupe。
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|c| seen.insert(c.clone()));
    candidates
}

/// `command.replace(/\s+/g, " ").trim()` — JS `\s` run を 1 space へ
/// 畳んで JS whitespace を trim。
fn collapse_command(command: &str) -> String {
    // `\s+` run を 1 space へ畳む(space は次の非空白文字が来た時点で
    // flush — leading/trailing run は自然に消えて JS trim と等価)。
    let mut out = String::new();
    let mut pending_space = false;
    for c in command.chars() {
        if is_js_space(c) {
            pending_space = true;
        } else {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            out.push(c);
            pending_space = false;
        }
    }
    out
}

/// `evaluateShell` — read candidate を file-read policy で順に評価し、
/// 最初の deny で止める。
async fn evaluate_shell(command: &str, cwd: &str, opts: &EvaluateOptions<'_>) -> Verdict {
    for candidate in extract_read_candidates(command) {
        let verdict = evaluate_file_read(&candidate, cwd, opts).await;
        if let Verdict::Deny { reason } = verdict {
            return Verdict::Deny {
                reason: format!(
                    "cheaper: blocked shell command '{}'.\n{reason}",
                    collapse_command(command)
                ),
            };
        }
    }
    Verdict::Allow
}

/// `evaluate` — TS 版は `unknown` を受けて shape を runtime validate
/// するが、Rust 版は typed request を受けるため malformed request は
/// adapter の normalize(None → allow)で表現され、ここには到達しない。
pub async fn evaluate(request: &RoutingRequest, opts: &EvaluateOptions<'_>) -> Verdict {
    match request {
        RoutingRequest::FileRead { path, cwd, .. } => {
            if path.is_empty() {
                return Verdict::Allow;
            }
            evaluate_file_read(path, cwd, opts).await
        }
        RoutingRequest::Shell { command, cwd, .. } => {
            if command.is_empty() {
                return Verdict::Allow;
            }
            evaluate_shell(command, cwd, opts).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: HashMap<&str, &str> = pairs.iter().copied().collect();
        move |k: &str| map.get(k).map(|v| (*v).to_string())
    }

    // --- js_parse_int_10 / threshold ---------------------------------

    #[test]
    fn parse_int_matches_js_partial_parse() {
        assert_eq!(js_parse_int_10("1024"), Some(1024.0));
        assert_eq!(js_parse_int_10("1024abc"), Some(1024.0));
        assert_eq!(js_parse_int_10("  42"), Some(42.0));
        assert_eq!(js_parse_int_10("+7"), Some(7.0));
        assert_eq!(js_parse_int_10("-5"), Some(-5.0));
        assert_eq!(js_parse_int_10("4.5"), Some(4.0));
        // parseInt("0x10", 10) は "0" で止まる。
        assert_eq!(js_parse_int_10("0x10"), Some(0.0));
        assert_eq!(js_parse_int_10("abc"), None);
        assert_eq!(js_parse_int_10(""), None);
        assert_eq!(js_parse_int_10("+"), None);
    }

    #[test]
    fn threshold_defaults_and_env() {
        let e = env_of(&[]);
        assert_eq!(threshold(&e), 8192.0);
        let e = env_of(&[("CHEAPER_READ_TOKEN_THRESHOLD", "1024")]);
        assert_eq!(threshold(&e), 1024.0);
        // JS と同じく parseInt の部分 parse を許容。
        let e = env_of(&[("CHEAPER_READ_TOKEN_THRESHOLD", "1024abc")]);
        assert_eq!(threshold(&e), 1024.0);
        for bad in ["0", "-5", "abc"] {
            let pairs = [("CHEAPER_READ_TOKEN_THRESHOLD", bad)];
            let e = env_of(&pairs);
            assert_eq!(threshold(&e), 8192.0, "env={bad}");
        }
    }

    // --- tokenize_shell ----------------------------------------------

    fn segment_texts(command: &str) -> Option<Vec<Vec<String>>> {
        tokenize_shell(command).map(|segs| {
            segs.into_iter()
                .map(|s| s.tokens.into_iter().map(|t| t.text).collect())
                .collect()
        })
    }

    #[test]
    fn tokenize_basic_and_quotes() {
        assert_eq!(
            segment_texts("cat big.txt"),
            Some(vec![vec!["cat".to_string(), "big.txt".to_string()]])
        );
        assert_eq!(
            segment_texts("cat \"path with spaces/big.txt\""),
            Some(vec![vec![
                "cat".to_string(),
                "path with spaces/big.txt".to_string()
            ]])
        );
        // single quote は escape 無しの literal。
        assert_eq!(
            segment_texts("cat 'a\\nb'"),
            Some(vec![vec!["cat".to_string(), "a\\nb".to_string()]])
        );
        // unterminated quote → null。
        assert_eq!(segment_texts("cat \"unclosed"), None);
    }

    #[test]
    fn tokenize_comment_consumes_newline_without_ending_segment() {
        // JS 規則: comment の while が `\n` 直前で止まり、`continue` の
        // i++ で `\n` を消費するため segment は継続する。
        assert_eq!(
            segment_texts("cat a # c\nmore"),
            Some(vec![vec![
                "cat".to_string(),
                "a".to_string(),
                "more".to_string()
            ]])
        );
        // comment は token 途中(`hasToken`)では始まらない。
        assert_eq!(
            segment_texts("cat a#b"),
            Some(vec![vec!["cat".to_string(), "a#b".to_string()]])
        );
    }

    #[test]
    fn tokenize_separators_and_redirection() {
        // `|`/`&&`/`;`/`\n` は segment 終端。
        assert_eq!(
            segment_texts("cat a | grep x"),
            Some(vec![
                vec!["cat".to_string(), "a".to_string()],
                vec!["grep".to_string(), "x".to_string()]
            ])
        );
        assert_eq!(
            segment_texts("a&&b"),
            Some(vec![vec!["a".to_string()], vec!["b".to_string()]])
        );
        // redirection → segment unsafe(最終 segment は token 0 でも
        // push される)。
        let segs = tokenize_shell("cat < big.txt").unwrap();
        assert!(segs[0].unsafe_segment);
        assert_eq!(segs[0].tokens[0].text, "cat");
    }

    #[test]
    fn tokenize_unsafe_chars_and_windows_path() {
        let segs = tokenize_shell("cat $(ls)").unwrap();
        assert!(segs[0].tokens[1].unsafe_token);
        let segs = tokenize_shell("cat *.txt").unwrap();
        assert!(segs[0].tokens[1].unsafe_token);
        // `\` が escapable でない次字の前では literal — Windows path。
        let segs = tokenize_shell("cat src\\dir\\f.txt").unwrap();
        assert_eq!(segs[0].tokens[1].text, "src\\dir\\f.txt");
        // `\ ` は escaped space → 同一 token。
        let segs = tokenize_shell("cat a\\ b.txt").unwrap();
        assert_eq!(segs[0].tokens[1].text, "a b.txt");
    }

    // --- command_name / extract_read_candidates -----------------------

    #[test]
    fn command_name_strips_dir_and_ext() {
        assert_eq!(command_name("cat"), "cat");
        assert_eq!(command_name("/bin/cat"), "cat");
        assert_eq!(command_name("C:\\bin\\cat.exe"), "cat");
        assert_eq!(command_name("cat.BAT"), "cat");
        // lowercase 化はしない。
        assert_eq!(command_name("CAT"), "CAT");
    }

    #[test]
    fn extract_read_candidates_matches_js_table() {
        // routing-policy.test.ts の fixture と同一。
        assert_eq!(extract_read_candidates("cat big.txt"), ["big.txt"]);
        assert_eq!(extract_read_candidates("head -n 200 big.txt"), ["big.txt"]);
        assert_eq!(
            extract_read_candidates("cat small.txt big.txt"),
            ["small.txt", "big.txt"]
        );
        assert_eq!(
            extract_read_candidates("cat \"path with spaces/big.txt\""),
            ["path with spaces/big.txt"]
        );
        assert_eq!(
            extract_read_candidates("cat big.txt | grep foo"),
            ["big.txt"]
        );
        assert_eq!(
            extract_read_candidates("cat big.txt && cd src"),
            ["big.txt"]
        );
        assert!(extract_read_candidates("cat $(ls)").is_empty());
        assert!(extract_read_candidates("cat *.txt").is_empty());
        assert!(extract_read_candidates("cat < big.txt").is_empty());
        assert!(extract_read_candidates("cat \"unclosed").is_empty());
        assert_eq!(extract_read_candidates("cat.exe big.txt"), ["big.txt"]);
        // cwd 遷移後の read candidate は抽出しない(fail-open)。
        assert!(extract_read_candidates("cd src && cat big.txt").is_empty());
        assert_eq!(extract_read_candidates("FOO=1 cat big.txt"), ["big.txt"]);
    }

    #[test]
    fn extract_read_candidates_options_and_dedupe() {
        // value option が次 token を消費する。
        assert_eq!(extract_read_candidates("tail -n 100 f.txt"), ["f.txt"]);
        assert_eq!(extract_read_candidates("head --lines=5 f.txt"), ["f.txt"]);
        // `--` で option 終了 → `-n` も operand。
        assert_eq!(extract_read_candidates("cat -- -n.txt"), ["-n.txt"]);
        // dedupe は first-occurrence order。
        assert_eq!(extract_read_candidates("cat a.txt a.txt"), ["a.txt"]);
        // operand 側の unsafe token は segment を abandon。
        assert!(extract_read_candidates("cat a.txt $(x)").is_empty());
        assert_eq!(
            extract_read_candidates("cat a.txt $(x) 2>/dev/null"),
            Vec::<String>::new()
        );
        // plusOptions: less/more の `+` token は option。
        assert_eq!(extract_read_candidates("less +F f.txt"), ["f.txt"]);
        assert_eq!(extract_read_candidates("more +10 f.txt"), ["f.txt"]);
        assert_eq!(extract_read_candidates("cat +x f.txt"), ["+x", "f.txt"]);
        // 別 segment の cd 以降は全て skip。
        assert!(extract_read_candidates("cd a; cat f.txt; cat g.txt").is_empty());
        // unsafe segment 内の cd でも ambiguous は立つ。
        assert!(extract_read_candidates("cd a > out; cat f.txt").is_empty());
    }

    // --- collapse_command ---------------------------------------------

    #[test]
    fn collapse_command_matches_js_replace_trim() {
        assert_eq!(collapse_command("cat   big.txt"), "cat big.txt");
        assert_eq!(collapse_command("  cat\tbig.txt\n"), "cat big.txt");
        assert_eq!(collapse_command("cat big.txt"), "cat big.txt");
    }
}
