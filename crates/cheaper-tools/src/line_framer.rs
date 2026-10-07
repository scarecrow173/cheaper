//! Port of `packages/tool-runtime/src/line-framer.ts`.
//!
//! 上限付き行フレーマー(DESIGN-0019)。raw byte 列上で `0x0A` を走査して
//! 行を切り出す。`0x0A` は UTF-8 multibyte sequence に現れないため byte
//! 走査は UTF-8 安全であり、行 byte 長は decode 前の raw byte で計測する。
//! `max_line_bytes` を超えた行は蓄積を止めて次の `\n` まで破棄モードに
//! 入り、`on_oversized` を発行して行番号を消費する(保持量は chunk +
//! max_line_bytes 以下で、入力総 byte 数 N に依存しない)。`\r` は行内容に
//! 残す(既存 `split("\n")` との parity)。空入力・LF 終端入力では EOF の
//! `flush()` が末尾の空行 `""` を 1 行 emit する(同じく split parity)。
//! 行テキストは WHATWG UTF-8 decode(`String::from_utf8_lossy` と同じ
//! U+FFFD 置換)で返す。

const LF: u8 = 0x0a;

/// `onLine` callback 型。`line` は decode 後テキスト、`line_no` は
/// 1-indexed、`raw_bytes` は decode 前 byte 長。
pub type OnLine<'a> = Box<dyn FnMut(&str, usize, usize) + Send + 'a>;
/// `onOversized` callback 型。破棄した行の 1-indexed 行番号を受け取る。
pub type OnOversized<'a> = Box<dyn FnMut(usize) + Send + 'a>;

/// `LineFramerOptions`。
pub struct LineFramerOptions<'a> {
    /// 評価対象にする 1 行の raw byte 上限(`GREP_MAX_LINE_BYTES`)。
    pub max_line_bytes: usize,
    /// 行確定時の callback。
    pub on_line: OnLine<'a>,
    /// `max_line_bytes` 超過で破棄した行の callback。行番号は消費される。
    pub on_oversized: OnOversized<'a>,
}

/// `createLineFramer` が返す stateful framer。
pub struct LineFramer<'a> {
    max_line_bytes: usize,
    on_line: OnLine<'a>,
    on_oversized: OnOversized<'a>,
    /// 未確定行の蓄積 byte(TS は `Buffer[]` を保持して確定時に concat する
    /// が、同じ byte 列を返せばよいので grow する `Vec<u8>` 一本に畳む)。
    pending: Vec<u8>,
    discarding: bool,
    line_no: usize,
    // split("\n") parity 用: 空入力・LF 終端入力では末尾に空行が 1 行続く。
    // flush でその空行を emit するため、最後に fed された byte を追跡する。
    saw_any: bool,
    last_byte_was_lf: bool,
}

impl<'a> LineFramer<'a> {
    /// 確定した行を emit し、蓄積を空にする(TS `takePending` + `onLine`)。
    fn emit_pending(&mut self) {
        let raw = std::mem::take(&mut self.pending);
        let line = String::from_utf8_lossy(&raw);
        (self.on_line)(&line, self.line_no, raw.len());
        self.line_no += 1;
    }

    /// `feed(buf)`。
    pub fn feed(&mut self, buf: &[u8]) {
        if !buf.is_empty() {
            self.saw_any = true;
            self.last_byte_was_lf = buf[buf.len() - 1] == LF;
        }
        let mut i = 0;
        while i < buf.len() {
            // `buf.indexOf(LF, i)`
            let nl = buf[i..].iter().position(|&b| b == LF).map(|p| i + p);
            if self.discarding {
                // oversized 行の残りを次の \n まで捨てる(行番号は超過時点で
                // 消費済み)
                let Some(nl) = nl else { return };
                self.discarding = false;
                i = nl + 1;
                continue;
            }
            let end = nl.unwrap_or(buf.len());
            if self.pending.len() + (end - i) > self.max_line_bytes {
                self.pending.clear();
                (self.on_oversized)(self.line_no);
                self.line_no += 1;
                match nl {
                    None => {
                        self.discarding = true;
                        return;
                    }
                    Some(nl) => {
                        i = nl + 1;
                        continue;
                    }
                }
            }
            if end > i {
                self.pending.extend_from_slice(&buf[i..end]);
            }
            let Some(nl) = nl else { return };
            self.emit_pending();
            i = nl + 1;
        }
    }

    /// `flush()`: EOF の未終端行を 1 行として emit する。入力が空または
    /// LF 終端の場合は `split("\n")` parity として末尾の空行 `""` も
    /// emit する。
    pub fn flush(&mut self) {
        // oversized 行を破棄したまま EOF に達した場合は何も emit しない
        if self.discarding {
            return;
        }
        if !self.pending.is_empty() {
            self.emit_pending();
            return;
        }
        // split("\n") parity: 空入力または LF 終端なら末尾の空行を 1 行
        // emit する
        if !self.saw_any || self.last_byte_was_lf {
            (self.on_line)("", self.line_no, 0);
            self.line_no += 1;
        }
    }
}

/// `createLineFramer`。
pub fn create_line_framer(opts: LineFramerOptions<'_>) -> LineFramer<'_> {
    LineFramer {
        max_line_bytes: opts.max_line_bytes,
        on_line: opts.on_line,
        on_oversized: opts.on_oversized,
        pending: Vec::new(),
        discarding: false,
        line_no: 1,
        saw_any: false,
        last_byte_was_lf: false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Emitted {
        line: String,
        line_no: usize,
        raw_bytes: usize,
    }

    struct Collected {
        framer: LineFramer<'static>,
        lines: Arc<Mutex<Vec<Emitted>>>,
        oversized: Arc<Mutex<Vec<usize>>>,
    }

    fn collect(max_line_bytes: usize) -> Collected {
        let lines = Arc::new(Mutex::new(Vec::new()));
        let oversized = Arc::new(Mutex::new(Vec::new()));
        let lines2 = Arc::clone(&lines);
        let oversized2 = Arc::clone(&oversized);
        let framer = create_line_framer(LineFramerOptions {
            max_line_bytes,
            on_line: Box::new(move |line, line_no, raw_bytes| {
                lines2.lock().unwrap().push(Emitted {
                    line: line.to_string(),
                    line_no,
                    raw_bytes,
                });
            }),
            on_oversized: Box::new(move |line_no| oversized2.lock().unwrap().push(line_no)),
        });
        Collected {
            framer,
            lines,
            oversized,
        }
    }

    fn lines_of(c: &Collected) -> Vec<Emitted> {
        c.lines.lock().unwrap().clone()
    }

    #[test]
    fn emits_lines_split_on_raw_lf_with_1_indexed_numbers() {
        let mut c = collect(1024);
        c.framer.feed(b"a\nbb\nccc\n");
        c.framer.flush();
        let lines = lines_of(&c);
        assert_eq!(
            lines,
            vec![
                Emitted {
                    line: "a".into(),
                    line_no: 1,
                    raw_bytes: 1
                },
                Emitted {
                    line: "bb".into(),
                    line_no: 2,
                    raw_bytes: 2
                },
                Emitted {
                    line: "ccc".into(),
                    line_no: 3,
                    raw_bytes: 3
                },
                // LF 終端の末尾空行(split parity)
                Emitted {
                    line: "".into(),
                    line_no: 4,
                    raw_bytes: 0
                },
            ]
        );
        assert!(c.oversized.lock().unwrap().is_empty());
    }

    #[test]
    fn reassembles_line_split_across_chunk_boundaries() {
        let mut c = collect(1024);
        c.framer.feed(b"hel");
        c.framer.feed(b"lo w");
        c.framer.feed(b"orld\nnex");
        c.framer.feed(b"t\n");
        c.framer.flush();
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["hello world", "next", ""]);
        let n: Vec<usize> = c.lines.lock().unwrap().iter().map(|e| e.line_no).collect();
        assert_eq!(n, [1, 2, 3]);
    }

    #[test]
    fn keeps_multibyte_utf8_split_across_chunks() {
        let mut c = collect(1024);
        let buf = "あいう\n".as_bytes(); // 9 byte + LF
        c.framer.feed(&buf[..4]); // multibyte sequence の途中で分割
        c.framer.feed(&buf[4..]);
        c.framer.flush();
        let lines = lines_of(&c);
        assert_eq!(
            lines,
            vec![
                Emitted {
                    line: "あいう".into(),
                    line_no: 1,
                    raw_bytes: 9
                },
                Emitted {
                    line: "".into(),
                    line_no: 2,
                    raw_bytes: 0
                },
            ]
        );
    }

    #[test]
    fn discards_oversized_line_once() {
        let mut c = collect(4);
        c.framer.feed(b"ok\n");
        c.framer.feed(b"this line is way too long\n");
        c.framer.feed(b"tail\n");
        c.framer.flush();
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["ok", "tail", ""]);
        assert_eq!(*c.oversized.lock().unwrap(), vec![2]);
    }

    #[test]
    fn oversized_line_consumes_line_number() {
        let mut c = collect(4);
        c.framer.feed(b"a\n");
        c.framer.feed(b"xxxxx\n"); // line 2 oversized
        c.framer.feed(b"yy\n");
        c.framer.flush();
        let n: Vec<usize> = c.lines.lock().unwrap().iter().map(|e| e.line_no).collect();
        assert_eq!(n, [1, 3, 4]); // 末尾空行(split parity)
        assert_eq!(*c.oversized.lock().unwrap(), vec![2]);
    }

    #[test]
    fn detects_overflow_mid_chunk_and_resumes() {
        let mut c = collect(4);
        c.framer.feed(b"abc");
        c.framer.feed(b"defgh"); // pending 3 + 5 > 4 → oversized、\n まで破棄
        c.framer.feed(b"ij\nkl\n");
        c.framer.flush();
        let lines = lines_of(&c);
        assert_eq!(
            lines,
            vec![
                Emitted {
                    line: "kl".into(),
                    line_no: 2,
                    raw_bytes: 2
                },
                Emitted {
                    line: "".into(),
                    line_no: 3,
                    raw_bytes: 0
                },
            ]
        );
        assert_eq!(*c.oversized.lock().unwrap(), vec![1]);
    }

    #[test]
    fn emits_unterminated_trailing_line_on_flush() {
        let mut c = collect(1024);
        c.framer.feed(b"a\nlast");
        c.framer.flush();
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["a", "last"]);
        let n: Vec<usize> = c.lines.lock().unwrap().iter().map(|e| e.line_no).collect();
        assert_eq!(n, [1, 2]);
    }

    #[test]
    fn emits_trailing_empty_line_when_ends_with_newline() {
        let mut c = collect(1024);
        c.framer.feed(b"a\n");
        c.framer.flush();
        // "a\n".split("\n") === ["a", ""]
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["a", ""]);
        let n: Vec<usize> = c.lines.lock().unwrap().iter().map(|e| e.line_no).collect();
        assert_eq!(n, [1, 2]);
    }

    #[test]
    fn emits_single_empty_line_for_empty_input() {
        let mut c = collect(1024);
        c.framer.flush();
        // "".split("\n") === [""]
        let lines = lines_of(&c);
        assert_eq!(
            lines,
            vec![Emitted {
                line: "".into(),
                line_no: 1,
                raw_bytes: 0
            }]
        );
    }

    #[test]
    fn emits_every_trailing_empty_line_for_consecutive_newlines() {
        let mut c = collect(1024);
        c.framer.feed(b"a\n\n");
        c.framer.flush();
        // "a\n\n".split("\n") === ["a", "", ""]
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["a", "", ""]);
        let n: Vec<usize> = c.lines.lock().unwrap().iter().map(|e| e.line_no).collect();
        assert_eq!(n, [1, 2, 3]);
    }

    #[test]
    fn no_extra_empty_line_when_no_trailing_newline() {
        let mut c = collect(1024);
        c.framer.feed(b"a");
        c.framer.flush();
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["a"]);
    }

    #[test]
    fn no_trailing_empty_line_when_oversized_discarded_at_eof() {
        let mut c = collect(4);
        c.framer.feed(b"a\n");
        c.framer.feed(b"discarded-long-line");
        c.framer.flush();
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["a"]);
        assert_eq!(*c.oversized.lock().unwrap(), vec![2]);
    }

    #[test]
    fn oversized_unterminated_line_reported_at_flush() {
        let mut c = collect(4);
        c.framer.feed(b"a very long line without newline");
        c.framer.flush();
        assert!(c.lines.lock().unwrap().is_empty());
        assert_eq!(*c.oversized.lock().unwrap(), vec![1]);
    }

    #[test]
    fn keeps_cr_in_line_content() {
        let mut c = collect(1024);
        c.framer.feed(b"a\r\nb\r\n");
        c.framer.flush();
        let v: Vec<String> = c
            .lines
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.line.clone())
            .collect();
        assert_eq!(v, ["a\r", "b\r", ""]);
    }

    #[test]
    fn replaces_invalid_utf8_with_fffd() {
        let mut c = collect(1024);
        c.framer.feed(&[0x61, 0xff, 0x62, 0x0a]);
        c.framer.flush();
        let lines = lines_of(&c);
        assert_eq!(
            lines,
            vec![
                Emitted {
                    line: "a\u{FFFD}b".into(),
                    line_no: 1,
                    raw_bytes: 3
                },
                Emitted {
                    line: "".into(),
                    line_no: 2,
                    raw_bytes: 0
                },
            ]
        );
    }
}
