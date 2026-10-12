//! workspace bounded read: positioned read で `offset` 行まで前方走査し
//! `limit` 行 / `max_bytes` まで返す。decode・継続読み取り・details
//! 組立を含む。

use std::fs::File;

use cheaper_core::errors::node_io_error;
use serde_json::{json, Map, Value};

use crate::catalog::{CanonicalToolResult, ToolContent};
use crate::limits::SNIFF_BYTES;
use crate::operation::AbortSignal;
use crate::ToolError;

use super::{err, read_at};

const LF: u8 = 0x0a;

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
pub(super) struct ScanParams<'a> {
    pub(super) offset: usize,
    pub(super) limit: usize,
    pub(super) byte_in_line: Option<usize>,
    pub(super) max_bytes: usize,
    pub(super) chunk_bytes: usize,
    pub(super) signal: Option<&'a AbortSignal>,
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

/// `readBounded`。`O(C + R)` 保持量の前方走査。詳細は TS コメントの通り。
pub(super) fn read_bounded(
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
