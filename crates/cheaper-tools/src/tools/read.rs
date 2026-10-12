//! Port of `packages/tool-runtime/src/read.ts`.
//!
//! bounded read tool(DESIGN-0019)。`IO_CHUNK_BYTES` 単位の positioned
//! read で prefix を破棄しながら `offset` 行まで走査し、最大 `limit` 行・
//! `READ_MAX_BYTES` を返す。`byteInLine` で oversized line の byte
//! 継続読み取りを行う。decode は行単位で WHATWG UTF-8(不正 byte は
//! U+FFFD)。file handle は呼び出しごとに open し cache しない
//! (`Drop` = `finally close`)。

use std::fs::{File, OpenOptions};
use std::io;

use std::sync::Arc;

use cheaper_core::errors::{node_io_error, CheaperError, ErrorCode};
use cheaper_core::js::MAX_SAFE_INTEGER_U64;
use cheaper_core::nodepath;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::{resolve_workspace_path, SymlinkPolicy};
use serde_json::{json, Map, Value};

use crate::access::AccessObserver;
use crate::artifact::{ArtifactData, ArtifactPorts, ArtifactProducer, ArtifactPut, ReadSource};
use crate::catalog::{self, CanonicalToolResult, ToolContent};
use crate::definition::{define_canonical_tool, CanonicalToolDefinition};
use crate::limits::{
    IO_CHUNK_BYTES, READ_DEFAULT_LIMIT, READ_MAX_BYTES, READ_MAX_LIMIT, SNIFF_BYTES,
};
use crate::operation::AbortSignal;
use crate::walker::{rethrow_as_denied, DeniedError};
use crate::ToolError;

const LF: u8 = 0x0a;

/// read が明示 error で拒否する画像拡張子(大小文字非依存、DESIGN-0019)。
const IMAGE_EXTENSIONS: [&str; 6] = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"];

/// `ReadToolOptions`。
pub struct ReadToolOptions<'a> {
    /// artifact ports(reader/publisher)。無しでも workspace read は動く。
    pub artifacts: Option<&'a ArtifactPorts>,
    /// publication producer の `jobId`(既定 `"tool-runtime"`)。
    pub owner_id: Option<&'a str>,
    pub observe_access: Option<&'a AccessObserver<'a>>,
    /// canonical 化済み workspace root。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    pub denylist: Option<&'a PathDenylist>,
    /// 本文 byte 上限の縮退注入(既定 `READ_MAX_BYTES`)。テスト用。
    pub max_bytes: Option<usize>,
    /// positioned-read chunk byte 数の縮退注入(既定 `IO_CHUNK_BYTES`)。
    /// テスト用。
    pub chunk_bytes: Option<usize>,
}

/// `createReadTool` の返り値。
pub struct ReadTool<'a> {
    opts: ReadToolOptions<'a>,
}

/// `createReadTool`。
pub fn create_read_tool(opts: ReadToolOptions<'_>) -> ReadTool<'_> {
    ReadTool { opts }
}

fn err(msg: impl Into<String>) -> ToolError {
    io::Error::other(msg.into()).into()
}

/// `signal.throwIfAborted()` — abort 済みなら reason を投げる。
fn throw_if_aborted(signal: Option<&AbortSignal>) -> Result<(), ToolError> {
    if let Some(s) = signal {
        if s.is_aborted() {
            let msg = s
                .reason()
                .map(|r| r.message().to_string())
                .unwrap_or_else(|| "Operation aborted".to_string());
            return Err(err(msg));
        }
    }
    Ok(())
}

/// positioned read(`fh.read(buf, 0, len, pos)`)。unix/windows 双方の
/// `read_at`/`seek_read` を包む。
#[cfg(unix)]
fn read_at(fh: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt as _;
    fh.read_at(buf, pos)
}

#[cfg(windows)]
fn read_at(fh: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    use std::os::windows::fs::FileExt as _;
    fh.seek_read(buf, pos)
}

#[cfg(not(any(unix, windows)))]
fn read_at(fh: &File, buf: &mut [u8], pos: u64) -> io::Result<usize> {
    // 最小 fallback: clone handle で seek+read(positioned read 相当)。
    use std::io::{Read as _, Seek as _, SeekFrom};
    let mut f = fh.try_clone()?;
    f.seek(SeekFrom::Start(pos))?;
    f.read(buf)
}

/// `StringDecoder("utf8")` 相当: `write` で未完 sequence を pending に
/// 保持し、`end` で残存を U+FFFD へ flush する。Rust の `Utf8Error`
/// (`valid_up_to`/`error_len`)は WHATWG の maximal-subpart 置換と一致する。
#[derive(Default)]
struct StringDecoder {
    pending: Vec<u8>,
}

impl StringDecoder {
    fn write(&mut self, piece: &[u8]) -> String {
        let mut buf = std::mem::take(&mut self.pending);
        buf.extend_from_slice(piece);
        let mut out = String::new();
        let mut rest: &[u8] = &buf;
        loop {
            match std::str::from_utf8(rest) {
                Ok(s) => {
                    out.push_str(s);
                    break;
                }
                Err(e) => {
                    let up_to = e.valid_up_to();
                    // prefix は valid であることが保証される。
                    out.push_str(std::str::from_utf8(&rest[..up_to]).expect("valid prefix"));
                    match e.error_len() {
                        Some(n) => {
                            out.push('\u{FFFD}');
                            rest = &rest[up_to + n..];
                        }
                        None => {
                            self.pending = rest[up_to..].to_vec();
                            break;
                        }
                    }
                }
            }
        }
        out
    }

    fn end(&mut self) -> &'static str {
        if self.pending.is_empty() {
            ""
        } else {
            self.pending.clear();
            "\u{FFFD}"
        }
    }
}

/// `formatLimit(maxBytes)`。
fn format_limit(max_bytes: usize) -> String {
    if max_bytes.is_multiple_of(1024) {
        format!("{}KB", max_bytes / 1024)
    } else {
        format!("{max_bytes}B")
    }
}

/// `ScanParams`。
struct ScanParams<'a> {
    offset: usize,
    limit: usize,
    byte_in_line: Option<usize>,
    max_bytes: usize,
    chunk_bytes: usize,
    signal: Option<&'a AbortSignal>,
}

/// `ReadToolDetails`。
#[derive(Debug)]
struct ReadToolDetails {
    start_line: usize,
    end_line: usize,
    eof: bool,
    truncated_by: Option<&'static str>,
    continuation: Option<Continuation>,
    total_lines: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct Continuation {
    offset: usize,
    byte_in_line: Option<usize>,
}

fn details_json(d: &ReadToolDetails) -> Value {
    let mut obj = Map::new();
    obj.insert("startLine".to_string(), json!(d.start_line));
    obj.insert("endLine".to_string(), json!(d.end_line));
    obj.insert("eof".to_string(), json!(d.eof));
    obj.insert(
        "truncatedBy".to_string(),
        d.truncated_by.map_or(Value::Null, |t| json!(t)),
    );
    if let Some(c) = d.continuation {
        let mut cont = Map::new();
        cont.insert("offset".to_string(), json!(c.offset));
        if let Some(b) = c.byte_in_line {
            cont.insert("byteInLine".to_string(), json!(b));
        }
        obj.insert("continuation".to_string(), Value::Object(cont));
    }
    if let Some(t) = d.total_lines {
        obj.insert("totalLines".to_string(), json!(t));
    }
    Value::Object(obj)
}

/// `readPublication`: stat size に依存せず cap+1 まで走査し、put 前に
/// overflow を拒否する。先頭 SNIFF_BYTES に NUL があれば binary error。
fn read_publication(
    fh: &File,
    path: &str,
    cap: u64,
    chunk_bytes: usize,
    signal: Option<&AbortSignal>,
) -> Result<Vec<u8>, ToolError> {
    // `Number.isSafeInteger(cap) && cap >= 0`。
    if cap > MAX_SAFE_INTEGER_U64 {
        return Err(Box::new(CheaperError::new(
            ErrorCode::InvalidRequest,
            "invalid artifact publication cap",
        )));
    }
    let mut chunks: Vec<u8> = Vec::new();
    let mut bytes: u64 = 0;
    let mut sniffed: u64 = 0;
    loop {
        throw_if_aborted(signal)?;
        let want = std::cmp::min(chunk_bytes as u64, cap - bytes + 1) as usize;
        let mut buf = vec![0u8; want];
        let n = read_at(fh, &mut buf, bytes)
            .map_err(|e| Box::new(node_io_error("read", path, e)) as ToolError)?;
        throw_if_aborted(signal)?;
        if n == 0 {
            return Ok(chunks);
        }
        let data = &buf[..n];
        let sniff_hit = data
            .iter()
            .take(SNIFF_BYTES.saturating_sub(sniffed as usize))
            .any(|&b| b == 0);
        if sniff_hit {
            return Err(err("read: binary file (NUL byte within first 8KiB)"));
        }
        sniffed += n as u64;
        bytes += n as u64;
        if bytes > cap {
            return Err(Box::new(CheaperError::new(
                ErrorCode::PolicyViolation,
                "artifact storage limit exceeded",
            )));
        }
        chunks.extend_from_slice(data);
    }
}

/// `readBounded`。`O(C + R)` 保持量の前方走査。詳細は TS コメントの通り。
fn read_bounded(
    fh: &File,
    path: &str,
    size: u64,
    p: &ScanParams<'_>,
) -> Result<CanonicalToolResult, ToolError> {
    let buf_size = p.chunk_bytes;
    let mut buf = vec![0u8; buf_size];
    let mut chunk_start: u64 = 0; // buf[0] のファイル byte 位置
    let mut next_read: u64 = 0; // 次に読むファイル byte 位置(単調増加)
    let mut file_end: u64 = size; // EOF 観測で確定する実効ファイル終端
    let mut sniffed = false;

    let check_abort = || -> Result<(), ToolError> {
        if p.signal.is_some_and(AbortSignal::is_aborted) {
            return Err(err("Operation aborted"));
        }
        Ok(())
    };

    // buf が pos を含むまで順方向に読み進める。EOF で false。
    // 最初の chunk はそのまま NUL sniff(先頭 SNIFF_BYTES 分)を兼ねる。
    let ensure = |buf: &mut Vec<u8>,
                  chunk_start: &mut u64,
                  next_read: &mut u64,
                  file_end: &mut u64,
                  sniffed: &mut bool,
                  pos: u64|
     -> Result<bool, ToolError> {
        check_abort()?;
        while pos >= *next_read {
            let n = read_at(fh, &mut buf[..], *next_read)
                .map_err(|e| Box::new(node_io_error("read", path, e)) as ToolError)?;
            if n == 0 {
                *file_end = *next_read;
                return Ok(false);
            }
            if !*sniffed {
                *sniffed = true;
                if buf[..std::cmp::min(SNIFF_BYTES, n)].contains(&0) {
                    return Err(err("read: binary file (NUL byte within first 8KiB)"));
                }
            }
            *chunk_start = *next_read;
            *next_read += n as u64;
            check_abort()?;
        }
        Ok(true)
    };

    // 現在の chunk 内で pos 以降の次の LF の絶対位置(無ければ -1)。
    macro_rules! lf_in_chunk {
        ($pos:expr) => {{
            let rel = $pos - chunk_start;
            let rel = rel as usize;
            buf[rel..]
                .iter()
                .position(|&b| b == LF)
                .and_then(|i| {
                    let abs = chunk_start + (rel + i) as u64;
                    if abs < next_read {
                        Some(abs)
                    } else {
                        None
                    }
                })
                .map(|x| x as i64)
                .unwrap_or(-1)
        }};
    }

    // [from, before) の次の LF の絶対位置。範囲走査完了で -2、EOF で -1。
    macro_rules! scan_lf_before {
        ($from:expr, $before:expr) => {{
            let mut cur: u64 = $from;
            let before: u64 = $before;
            loop {
                if cur >= before {
                    break -2i64;
                }
                if !ensure(
                    &mut buf,
                    &mut chunk_start,
                    &mut next_read,
                    &mut file_end,
                    &mut sniffed,
                    cur,
                )? {
                    break -1i64;
                }
                let nl = lf_in_chunk!(cur);
                if nl >= 0 {
                    break if (nl as u64) < before { nl } else { -2 };
                }
                cur = next_read;
            }
        }};
    }

    // 1. offset 行の開始 byte 位置を特定する(prefix は chunk 単位で破棄)。
    let mut line_start: u64 = 0;
    if p.offset > 1 {
        let mut cur: u64 = 0;
        let mut nls: u64 = 0;
        loop {
            if !ensure(
                &mut buf,
                &mut chunk_start,
                &mut next_read,
                &mut file_end,
                &mut sniffed,
                cur,
            )? {
                // EOF 到達時点で総行数は判明済み(split("\n") 意味論: LF 数 + 1)。
                return Err(err(format!(
                    "Offset {} is beyond end of file ({} lines total)",
                    p.offset,
                    nls + 1,
                )));
            }
            let nl = lf_in_chunk!(cur);
            if nl < 0 {
                cur = next_read;
                continue;
            }
            nls += 1;
            if nls == (p.offset - 1) as u64 {
                line_start = (nl + 1) as u64;
                break;
            }
            cur = (nl + 1) as u64;
        }
    }

    // 2. byteInLine の検証と UTF-8 境界スナップ。
    let mut emit_from = line_start;
    if let Some(k) = p.byte_in_line {
        let k = k as u64;
        // [lineStart, lineStart+k+1) に行末(LF or EOF)が来るなら行 byte 長
        // ≤ k で error。
        let r = scan_lf_before!(line_start, line_start + k + 1);
        if r != -2 {
            let line_len = (if r == -1 { file_end } else { r as u64 }) - line_start;
            return Err(err(format!(
                "byteInLine {k} is beyond end of line {} ({line_len} bytes)",
                p.offset,
            )));
        }
        emit_from = line_start + k;
        // UTF-8 文字境界へ前進スナップ(最大 3 byte)。
        for _ in 0..3 {
            if !ensure(
                &mut buf,
                &mut chunk_start,
                &mut next_read,
                &mut file_end,
                &mut sniffed,
                emit_from,
            )? {
                break;
            }
            let b = buf[(emit_from - chunk_start) as usize];
            if (b & 0xc0) != 0x80 {
                break;
            }
            emit_from += 1;
        }
    }

    // 3. emit ループ。
    let mut decoder = StringDecoder::default();
    let mut out_lines: Vec<String> = Vec::new();
    // join 後本文 byte 数(marker・行・`\n` 連結を含む、decode 後基準)。
    let mut emitted_bytes: usize = 0;
    let mut emitted_lines: usize = 0;
    let mut wrote_something = false;
    let mut end_line: usize = p.offset - 1;
    let mut eof = false;
    let mut truncated_by: Option<&'static str> = None;
    let mut continuation: Option<Continuation> = None;
    let mut trailer: Option<String> = None;

    // byteInLine 再開時は先頭に partial marker 行を付ける(本文 byte 予算に
    // 計上)。
    if emit_from > line_start {
        let marker = format!(
            "[partial line {}: continuing from byte {}]",
            p.offset,
            emit_from - line_start,
        );
        emitted_bytes += marker.len();
        out_lines.push(marker);
        wrote_something = true;
    }

    let mut line_no = p.offset;
    let mut cur_start = line_start;
    let mut cur_emit = emit_from;

    loop {
        let emit_begin_in_line = cur_emit - cur_start;
        let line_emit_start = cur_emit;
        let mut line_text = String::new();
        let mut line_emitted = false;
        let mut line_done = false;
        let mut terminated = false;
        let mut budget_stop = false;
        // region 末尾で未完だった UTF-8 sequence の prefix(≤3 byte、
        // decoder 未投入)。位置としては消費済み。
        let mut carry: Vec<u8> = Vec::new();

        while !line_done && !budget_stop {
            let region_end: u64;
            let mut term_here = false;
            let mut at_eof = false;
            if ensure(
                &mut buf,
                &mut chunk_start,
                &mut next_read,
                &mut file_end,
                &mut sniffed,
                cur_emit,
            )? {
                let nl = lf_in_chunk!(cur_emit);
                if nl >= 0 {
                    region_end = nl as u64;
                    term_here = true;
                } else {
                    region_end = next_read;
                }
            } else {
                region_end = file_end;
                at_eof = true;
            }
            let region_len = region_end.saturating_sub(cur_emit) as usize;
            let region_base = (cur_emit - chunk_start) as usize;
            let wlen = carry.len() + region_len;
            if wlen > 0 {
                // window = carry ++ region。WHATWG 規則で 1 文字ずつ
                // (消費 src byte 数, emit する decode 後 byte 数)を確定し、
                // 予算が尽きるか window 末尾で sequence が未完になる手前で
                // 止める。
                let wat = |j: usize| -> u8 {
                    if j < carry.len() {
                        carry[j]
                    } else {
                        buf[region_base + j - carry.len()]
                    }
                };
                let mut j = 0usize; // 消費確定済みの window 先頭からの byte 数
                let mut stop: Option<&'static str> = None;
                while j < wlen {
                    let b = wat(j);
                    let mut char_len = 0usize; // 消費する src byte 数(0 = window 末尾で未完)
                    let mut dec = 0usize; // emit する decode 後 byte 数
                    if b < 0x80 {
                        char_len = 1;
                        dec = 1;
                    } else if !(0xc2..=0xf4).contains(&b) {
                        // stray continuation・不正 lead(0x80-0xBF, 0xC0,
                        // 0xC1, 0xF5-0xFF)。
                        char_len = 1;
                        dec = 3; // U+FFFD
                    } else {
                        let need = if b < 0xe0 {
                            1
                        } else if b < 0xf0 {
                            2
                        } else {
                            3
                        };
                        let mut valid = 0usize;
                        let mut malformed = false;
                        for k in 1..=need {
                            if j + k >= wlen {
                                break;
                            }
                            let c = wat(j + k);
                            // WHATWG 第 1 continuation 範囲(E0: A0-BF,
                            // ED: 80-9F, F0: 90-BF, F4: 80-8F, 他: 80-BF)。
                            let (mut lo, mut hi) = (0x80u8, 0xbfu8);
                            if k == 1 {
                                match b {
                                    0xe0 => lo = 0xa0,
                                    0xed => hi = 0x9f,
                                    0xf0 => lo = 0x90,
                                    0xf4 => hi = 0x8f,
                                    _ => {}
                                }
                            }
                            if !(lo..=hi).contains(&c) {
                                malformed = true;
                                break;
                            }
                            valid += 1;
                        }
                        if malformed {
                            // malformed subpart = lead + 有効 continuation 分を
                            // U+FFFD 1 個で消費。
                            char_len = 1 + valid;
                            dec = 3;
                        } else if valid == need {
                            char_len = need + 1;
                            dec = need + 1;
                        }
                        // valid < need かつ不正 byte なし = window 末尾で未完。
                    }
                    if char_len == 0 {
                        stop = Some("incomplete");
                        break;
                    }
                    // join 用 `\n` は本文 byte 予算に計上するが separator 自体は
                    // `out_lines.join("\n")` が供給する。
                    let sep = usize::from(!line_emitted && wrote_something);
                    if emitted_bytes + sep + dec > p.max_bytes {
                        stop = Some("budget");
                        break;
                    }
                    emitted_bytes += sep + dec;
                    line_emitted = true;
                    j += char_len;
                }
                if stop == Some("incomplete") && (term_here || at_eof) {
                    // region 末尾が真の行末(LF or EOF): 未完 sequence は
                    // continuation 待ちではなく不正 byte なので、残り全部を
                    // 消費し U+FFFD 1 個(3 byte)として計上する。
                    let sep = usize::from(!line_emitted && wrote_something);
                    if emitted_bytes + sep + 3 > p.max_bytes {
                        stop = Some("budget");
                    } else {
                        emitted_bytes += sep + 3;
                        line_emitted = true;
                        j = wlen;
                        stop = None;
                    }
                }
                // 消費確定分(window 先頭 j byte = carry 残 + region 前方)を
                // decode して連結。
                if j > 0 {
                    let from_carry = j.min(carry.len());
                    let from_region = j.saturating_sub(carry.len());
                    let mut piece = Vec::with_capacity(from_carry + from_region);
                    piece.extend_from_slice(&carry[..from_carry]);
                    piece.extend_from_slice(&buf[region_base..region_base + from_region]);
                    line_text.push_str(&decoder.write(&piece));
                }
                if stop == Some("budget") {
                    // 未完 char(carry 分を含む)は未消費へ戻す → byteInLine
                    // は常に文字境界。`j - carry.len()` は負になり得る
                    // (budget が carry 内で尽きた場合)ため符号付きで加算。
                    cur_emit = (cur_emit as i64 + j as i64 - carry.len() as i64) as u64;
                    budget_stop = true;
                    break;
                }
                // region は位置として全消費。未完 prefix は carry が保持し
                // 次 region で再構成する。
                cur_emit += region_len as u64;
                if stop == Some("incomplete") {
                    let tail_start = region_base + j.saturating_sub(carry.len());
                    let tail = &buf[tail_start..region_base + region_len];
                    carry = if j < carry.len() {
                        let mut c = carry[j..].to_vec();
                        c.extend_from_slice(tail);
                        c
                    } else {
                        tail.to_vec()
                    };
                } else {
                    carry = Vec::new();
                }
                continue;
            }
            // wlen === 0: 行末(`\n` 直上 or EOF)。空行は join 用 `\n` 分だけ
            // 予算を見る。
            if !line_emitted && wrote_something {
                if emitted_bytes + 1 > p.max_bytes {
                    budget_stop = true;
                    break;
                }
                emitted_bytes += 1;
            }
            line_done = true;
            terminated = term_here;
        }

        if line_emitted || line_done {
            // 空行も join 要素として emit する(split("\n") 意味論の
            // verbatim 連結)。`decoder.end()` は emit 行ごとに 1 回。
            let flush = decoder.end();
            line_text.push_str(flush);
            out_lines.push(line_text);
            end_line = line_no;
            wrote_something = true;
        }

        if budget_stop {
            truncated_by = Some("bytes");
            if cur_emit == line_emit_start && emit_begin_in_line == 0 {
                // byte cap が行境界で到達。
                continuation = Some(Continuation {
                    offset: line_no,
                    byte_in_line: None,
                });
                trailer = Some(format!(
                    "[Reached {} limit. Use offset={line_no} to continue.]",
                    format_limit(p.max_bytes),
                ));
            } else {
                // byte cap が行途中で到達。byteInLine 継続を返す(常に
                // UTF-8 文字境界)。
                let c = cur_emit - cur_start;
                continuation = Some(Continuation {
                    offset: line_no,
                    byte_in_line: Some(c as usize),
                });
                trailer = Some(format!(
                    "[Line {line_no} emitted bytes {emit_begin_in_line}-{c} (reached {} limit). Use offset={line_no} byteInLine={c} to continue.]",
                    format_limit(p.max_bytes),
                ));
            }
            break;
        }

        emitted_lines += 1;
        if !terminated {
            // EOF の未終端行 = 最終行。総行数はこの時点で確定。
            eof = true;
            break;
        }
        if emitted_lines >= p.limit {
            truncated_by = Some("limit");
            continuation = Some(Continuation {
                offset: line_no + 1,
                byte_in_line: None,
            });
            trailer = Some(format!(
                "[Showing lines {}-{line_no}. Use offset={} to continue.]",
                p.offset,
                line_no + 1,
            ));
            break;
        }
        line_no += 1;
        cur_start = cur_emit + 1; // 行末 `\n` の次 byte から次行が始まる
        cur_emit = cur_start;
    }

    let details = ReadToolDetails {
        start_line: p.offset,
        end_line,
        eof,
        truncated_by,
        continuation,
        total_lines: eof.then_some(end_line),
    };
    let mut text = out_lines.join("\n");
    if let Some(t) = trailer {
        text.push('\n');
        text.push_str(&t);
    }
    Ok(CanonicalToolResult {
        content: vec![ToolContent::text(text)],
        details: Some(details_json(&details)),
    })
}

impl ReadTool<'_> {
    /// `definition.execute` 相当。
    pub fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let opts = &self.opts;
        if input.get("source").is_some() {
            throw_if_aborted(signal)?;
            let reader = opts
                .artifacts
                .and_then(|p| p.reader.as_ref())
                .ok_or_else(|| {
                    Box::new(CheaperError::new(
                        ErrorCode::InvalidRequest,
                        "artifact reader is unavailable",
                    )) as ToolError
                })?;
            let source: ReadSource = serde_json::from_value(input["source"].clone())
                .map_err(|e| err(format!("invalid artifact source: {e}")))?;
            let artifact_source = match &source {
                ReadSource::Artifact(s) => s.clone(),
                _ => return Err(err("invalid artifact source")),
            };
            let result = reader.read(
                &artifact_source,
                &crate::artifact::ArtifactReadRequest {
                    offset: input["offset"].as_u64().unwrap_or(0),
                    max_bytes: input["maxBytes"].as_u64().unwrap_or(1),
                },
            )?;
            throw_if_aborted(signal)?;
            return Ok(CanonicalToolResult {
                content: vec![ToolContent::text(String::from_utf8_lossy(&result.data))],
                details: Some(json!({
                    "offset": result.offset,
                    "nextOffset": result.next_offset,
                    "eof": result.eof,
                })),
            });
        }
        let has_artifact = input.get("artifact").is_some();
        let publisher = if has_artifact {
            opts.artifacts.and_then(|p| p.publisher.as_ref())
        } else {
            None
        };
        if has_artifact && publisher.is_none() {
            return Err(Box::new(CheaperError::new(
                ErrorCode::InvalidRequest,
                "artifact publisher is unavailable",
            )));
        }
        if has_artifact {
            throw_if_aborted(signal)?;
        }
        let workspace = if has_artifact { None } else { Some(input) };
        if workspace.and_then(|w| w.get("byteInLine")).is_some()
            && workspace.and_then(|w| w.get("offset")).is_none()
        {
            return Err(err("byteInLine requires offset"));
        }
        let path = input["path"].as_str().unwrap_or_default();
        let scan = ScanParams {
            offset: workspace.and_then(|w| w["offset"].as_u64()).unwrap_or(1) as usize,
            limit: std::cmp::min(
                workspace
                    .and_then(|w| w["limit"].as_u64())
                    .unwrap_or(READ_DEFAULT_LIMIT as u64) as usize,
                READ_MAX_LIMIT,
            ),
            byte_in_line: workspace
                .and_then(|w| w["byteInLine"].as_u64())
                .map(|v| v as usize),
            max_bytes: opts.max_bytes.unwrap_or(READ_MAX_BYTES),
            chunk_bytes: opts.chunk_bytes.unwrap_or(IO_CHUNK_BYTES),
            signal,
        };
        let result = (|| -> Result<CanonicalToolResult, ToolError> {
            let resolved = resolve_workspace_path(opts.root, path, opts.symlink_policy)
                .map_err(|e| Box::new(e) as ToolError)?;
            if opts
                .denylist
                .is_some_and(|d| d.is_denied(&resolved.display))
            {
                return Err(Box::new(DeniedError::policy()));
            }
            // stat を open 前に行い directory 等を platform 非依存の明示 error
            // にする。
            if let Some(observe) = opts.observe_access {
                observe(
                    &resolved.real,
                    cheaper_core::resources::types::ClaimMode::Shared,
                    None,
                    false,
                )?;
            }
            let st = std::fs::metadata(&resolved.real)
                .map_err(|e| Box::new(node_io_error("stat", &resolved.real, e)) as ToolError)?;
            if !st.is_file() {
                return Err(err("read: not a regular file"));
            }
            // 画像は走査前に明示 error。画像経路・全文 fallback は設けない。
            if IMAGE_EXTENSIONS.contains(&nodepath::extname(&resolved.real).to_lowercase().as_str())
            {
                return Err(err("read does not support image files"));
            }
            if let Some(observe) = opts.observe_access {
                observe(
                    &resolved.real,
                    cheaper_core::resources::types::ClaimMode::Shared,
                    None,
                    false,
                )?;
            }
            let fh = OpenOptions::new()
                .read(true)
                .open(&resolved.real)
                .map_err(|e| Box::new(node_io_error("open", &resolved.real, e)) as ToolError)?;
            if has_artifact {
                let publisher = publisher.expect("checked above");
                let data = read_publication(
                    &fh,
                    &resolved.real,
                    publisher.max_artifact_bytes(),
                    scan.chunk_bytes,
                    signal,
                )?;
                throw_if_aborted(signal)?;
                let artifact = publisher.put(ArtifactPut {
                    producer: ArtifactProducer {
                        job_id: opts.owner_id.unwrap_or("tool-runtime").to_string(),
                        name: input["artifact"].as_str().unwrap_or_default().to_string(),
                    },
                    data: ArtifactData::Bytes(data),
                })?;
                return Ok(CanonicalToolResult {
                    content: Vec::new(),
                    details: Some(json!({
                        "artifact": serde_json::to_value(&artifact)
                            .expect("artifact reference serializes"),
                    })),
                });
            }
            read_bounded(&fh, &resolved.real, st.len(), &scan)
        })();
        result.map_err(rethrow_as_denied)
    }
}

/// `read` tool の canonical definition。
pub fn read_tool_definition(
    opts: ReadToolOptions<'_>,
) -> Result<CanonicalToolDefinition<'_>, ToolError> {
    let tool = Arc::new(create_read_tool(opts));
    Ok(define_canonical_tool(CanonicalToolDefinition {
        name: ToolName::Read,
        label: "read".to_string(),
        description: "Choose one exclusive mode: workspace line read {path,offset?,limit?,byteInLine?} returns up to 2000 lines and 50KB; publication {path,artifact:name} returns only a bounded artifact reference; artifact byte read {source:{kind:artifact,reference},offset,maxBytes} returns a bounded slice and nextOffset/eof. Workspace offsets are 1-indexed lines; artifact offsets are 0-indexed source bytes. Binary/image workspace files are rejected. Invalid UTF-8 bytes are replaced with U+FFFD.".to_string(),
        parameters: catalog::read_schema()
            .as_object()
            .expect("object schema")
            .clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input, signal.as_ref()) })
        }),
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
    use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
    use serde_json::json;

    use super::*;
    use crate::artifact::{
        ArtifactData, ArtifactPut, ArtifactReadRequest, ArtifactReadResult, ArtifactReader,
        ArtifactReference, ArtifactSource,
    };

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-read-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn read_opts<'a>(root: &'a str, denylist: Option<&'a PathDenylist>) -> ReadToolOptions<'a> {
        ReadToolOptions {
            artifacts: None,
            owner_id: None,
            observe_access: None,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            denylist,
            max_bytes: None,
            chunk_bytes: None,
        }
    }

    fn text_of(result: &CanonicalToolResult) -> &str {
        &result.content[0].text
    }

    #[test]
    fn fs_read_lines_offset_limit() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let body = (1..=10)
            .map(|i| format!("line{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(dir.join("f.txt"), &body).unwrap();
        let tool = create_read_tool(read_opts(&root, None));
        let result = tool
            .execute(&json!({"path": "f.txt", "offset": 3, "limit": 2}), None)
            .unwrap();
        assert_eq!(
            text_of(&result),
            "line3\nline4\n[Showing lines 3-4. Use offset=5 to continue.]"
        );
        let d = result.details.as_ref().unwrap();
        assert_eq!(d["startLine"], 3);
        assert_eq!(d["endLine"], 4);
        assert_eq!(d["eof"], false);
        assert_eq!(d["truncatedBy"], "limit");
        assert_eq!(d["continuation"]["offset"], 5);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_eof_total_lines() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "a\nb\n").unwrap();
        let tool = create_read_tool(read_opts(&root, None));
        let result = tool.execute(&json!({"path": "f.txt"}), None).unwrap();
        // split("\n") 意味論: "a\nb\n" → ["a","b",""] の末尾 empty は 3 行目。
        assert_eq!(text_of(&result), "a\nb\n");
        let d = result.details.as_ref().unwrap();
        assert_eq!(d["eof"], true);
        assert_eq!(d["totalLines"], 3);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_offset_beyond_eof_reports_total() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "a\nb\n").unwrap();
        let tool = create_read_tool(read_opts(&root, None));
        let err = tool
            .execute(&json!({"path": "f.txt", "offset": 9}), None)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Offset 9 is beyond end of file (3 lines total)"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_oversized_line_continuation() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let long = "x".repeat(100);
        fs::write(dir.join("f.txt"), &long).unwrap();
        let mut opts = read_opts(&root, None);
        opts.max_bytes = Some(30);
        let tool = create_read_tool(opts);
        let result = tool.execute(&json!({"path": "f.txt"}), None).unwrap();
        let d = result.details.as_ref().unwrap();
        assert_eq!(d["truncatedBy"], "bytes");
        let byte_in_line = d["continuation"]["byteInLine"].as_u64().unwrap() as usize;
        assert!(byte_in_line > 0);
        let emitted = &result.content[0].text;
        let (body, trailer) = emitted.split_once('\n').unwrap();
        assert_eq!(body.len(), byte_in_line);
        assert!(trailer.contains(&format!("byteInLine={byte_in_line}")));
        // byteInLine 継続で残りを読む(marker も byte 予算に計上される
        // ため、十分な cap の tool を別建てる)。
        let mut opts2 = read_opts(&root, None);
        opts2.max_bytes = Some(4096);
        let tool2 = create_read_tool(opts2);
        let result2 = tool2
            .execute(
                &json!({"path": "f.txt", "offset": 1, "byteInLine": byte_in_line}),
                None,
            )
            .unwrap();
        let d2 = result2.details.as_ref().unwrap();
        assert_eq!(d2["eof"], true);
        assert_eq!(d2["totalLines"], 1);
        let text2 = text_of(&result2);
        let marker = format!("[partial line 1: continuing from byte {byte_in_line}]");
        let expected_rest = "x".repeat(100 - byte_in_line);
        assert_eq!(text2, format!("{marker}\n{expected_rest}"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_rejects_directory_and_image() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("img.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();
        let tool = create_read_tool(read_opts(&root, None));
        let err = tool.execute(&json!({"path": "sub"}), None).unwrap_err();
        assert_eq!(err.to_string(), "read: not a regular file");
        let err = tool.execute(&json!({"path": "img.png"}), None).unwrap_err();
        assert_eq!(err.to_string(), "read does not support image files");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_rejects_binary_nul_sniff() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("b.bin"), [0x61u8, 0x00, 0x62]).unwrap();
        let tool = create_read_tool(read_opts(&root, None));
        let err = tool.execute(&json!({"path": "b.bin"}), None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "read: binary file (NUL byte within first 8KiB)"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_invalid_utf8_becomes_replacement_char() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("bad.txt"), [0x66, 0x80, 0x67]).unwrap();
        let tool = create_read_tool(read_opts(&root, None));
        let result = tool.execute(&json!({"path": "bad.txt"}), None).unwrap();
        assert_eq!(text_of(&result), "f\u{FFFD}g");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_denied_and_escape_errors() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join(".env"), "SECRET=1").unwrap();
        let denylist = create_path_denylist(&[]);
        let tool = create_read_tool(read_opts(&root, Some(&denylist)));
        let err = tool.execute(&json!({"path": ".env"}), None).unwrap_err();
        assert_eq!(err.to_string(), crate::walker::POLICY_DENIED_MESSAGE);
        let tool2 = create_read_tool(read_opts(&root, None));
        let err = tool2.execute(&json!({"path": "../x"}), None).unwrap_err();
        assert_eq!(err.to_string(), crate::walker::ACCESS_DENIED_MESSAGE);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_aborted_signal() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "a").unwrap();
        let controller = crate::operation::AbortController::new();
        controller.abort(crate::operation::AbortReason::msg("stop"));
        let signal = controller.signal();
        let tool = create_read_tool(read_opts(&root, None));
        let err = tool
            .execute(&json!({"path": "f.txt"}), Some(&signal))
            .unwrap_err();
        assert_eq!(err.to_string(), "Operation aborted");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// 最小 publisher: bytes を Vec に保持する fake。
    struct FakePublisher {
        cap: u64,
    }

    impl crate::artifact::ArtifactPublisher for FakePublisher {
        fn max_artifact_bytes(&self) -> u64 {
            self.cap
        }
        fn put(&self, value: ArtifactPut) -> Result<ArtifactReference, ToolError> {
            let bytes = match &value.data {
                ArtifactData::Bytes(b) => b.len() as u64,
                ArtifactData::Text(t) => t.len() as u64,
            };
            Ok(ArtifactReference {
                artifact: "test-artifact".to_string(),
                run_id: "run-1".to_string(),
                producer: value.producer,
                bytes,
                preview: String::new(),
                truncated: false,
            })
        }
    }

    #[test]
    fn fs_read_artifact_publication() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("big.txt"), "data").unwrap();
        let publisher = Arc::new(FakePublisher { cap: 1024 });
        let ports = ArtifactPorts {
            reader: None,
            publisher: Some(publisher),
        };
        let mut opts = read_opts(&root, None);
        opts.artifacts = Some(&ports);
        let tool = create_read_tool(opts);
        let result = tool
            .execute(&json!({"path": "big.txt", "artifact": "blob"}), None)
            .unwrap();
        assert!(result.content.is_empty());
        let artifact = &result.details.as_ref().unwrap()["artifact"];
        assert_eq!(artifact["artifact"], "test-artifact");
        assert_eq!(artifact["producer"]["name"], "blob");
        assert_eq!(artifact["producer"]["jobId"], "tool-runtime");
        assert_eq!(artifact["bytes"], 4);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_read_artifact_publication_cap() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("big.txt"), "0123456789").unwrap();
        let publisher = Arc::new(FakePublisher { cap: 4 });
        let ports = ArtifactPorts {
            reader: None,
            publisher: Some(publisher),
        };
        let mut opts = read_opts(&root, None);
        opts.artifacts = Some(&ports);
        let tool = create_read_tool(opts);
        let err = tool
            .execute(&json!({"path": "big.txt", "artifact": "blob"}), None)
            .unwrap_err();
        assert_eq!(err.to_string(), "artifact storage limit exceeded");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// 最小 reader。
    struct FakeReader {
        data: Vec<u8>,
    }

    impl ArtifactReader for FakeReader {
        fn read(
            &self,
            _source: &ArtifactSource,
            request: &ArtifactReadRequest,
        ) -> Result<ArtifactReadResult, ToolError> {
            let offset = request.offset.min(self.data.len() as u64) as usize;
            let end = (offset + request.max_bytes as usize).min(self.data.len());
            Ok(ArtifactReadResult {
                data: self.data[offset..end].to_vec(),
                offset: offset as u64,
                next_offset: end as u64,
                eof: end >= self.data.len(),
            })
        }
    }

    #[test]
    fn fs_read_artifact_bytes() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let reader = Arc::new(FakeReader {
            data: b"hello world".to_vec(),
        });
        let ports = ArtifactPorts {
            reader: Some(reader),
            publisher: None,
        };
        let mut opts = read_opts(&root, None);
        opts.artifacts = Some(&ports);
        let tool = create_read_tool(opts);
        let source = json!({
            "kind": "artifact",
            "reference": {
                "artifact": "a",
                "runId": "r",
                "producer": {"jobId": "j", "name": "n"},
                "bytes": 11,
                "preview": "",
                "truncated": false,
            },
        });
        let result = tool
            .execute(&json!({"source": source, "offset": 6, "maxBytes": 5}), None)
            .unwrap();
        assert_eq!(text_of(&result), "world");
        let d = result.details.as_ref().unwrap();
        assert_eq!(d["offset"], 6);
        assert_eq!(d["nextOffset"], 11);
        assert_eq!(d["eof"], true);
        fs::remove_dir_all(&dir).unwrap();
    }
}
