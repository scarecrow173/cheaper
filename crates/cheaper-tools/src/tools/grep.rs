//! Port of `packages/tool-runtime/src/grep.ts` — sandboxed streaming grep
//! (T2-5)。
//!
//! 外部プロセス(rg 等)を spawn せず、path-only [`walk_files`] →
//! [`sniff_text_file`] → `IO_CHUNK_BYTES` chunk 読み取り →
//! [`create_line_framer`] → batch protocol(K 行 / B byte 締め・未応答 1)
//! で eval subprocess([`crate::tools::grep_eval`])へ流す streaming 構成。
//! 全文 `read` はしない。eval 予算は per-file(batch 間累積・超過で
//! timeout skip + worker 再生成)と total(超過で走査停止)を持ち、scan
//! deadline は部分結果 + footer で返す。
//!
//! 出力 contract(TS と同一): match は `path:line: text`、context 行は
//! `path-line- text`、非連続 block 間に `--`。binary/unreadable/
//! oversized-line/timeout は有界 footer(`[skipped: <reason> — <list>]`)
//! で報告する(deny 対象は walker filter 段で消えるため一切出ない)。
//! 走査停止理由は `[truncated: ...]` marker で示す。
//!
//! regex 方言は JS `new RegExp`(非 unicode)+ `i` flag で、eval engine
//! は subprocess 内の `regress::Regex`。ReDoS 隔離・watch timeout kill・
//! reply byte cap は [`super::grep_eval`] が担う。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::sync::mpsc::{channel, Receiver};

use cheaper_core::resources::types::ClaimMode;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::Value;

use crate::access::AccessObserver;
use crate::catalog::{self, CanonicalToolResult};
use crate::limits::{
    GREP_BATCH_MAX_BYTES, GREP_BATCH_MAX_LINES, GREP_EVAL_PER_FILE_MS, GREP_EVAL_TOTAL_MS,
    GREP_MAX_LINE_BYTES, GREP_OUTPUT_LINE_MAX_CHARS, GREP_OUTPUT_MAX_BYTES, GREP_PATTERN_MAX_CHARS,
    GREP_SCAN_DEADLINE_MS, IO_CHUNK_BYTES, SKIP_FOOTER_MAX_BYTES, SKIP_FOOTER_MAX_ENTRIES,
    SKIP_PATH_MAX_CHARS,
};
use crate::line_framer::{create_line_framer, LineFramerOptions};
use crate::operation::AbortSignal;
use crate::walker::{rethrow_as_denied, sniff_text_file, walk_files, SniffResult, WalkOptions};
use crate::ToolError;
use cheaper_core::js16::{slice_utf16, utf16_len};

use super::grep_eval::{
    AbortCheck, EvalError, EvalOptions, GrepEvalWorker, ABORTED_MESSAGE, EVAL_REPLY_MAX_BYTES,
};

const DEFAULT_MAX_RESULTS: usize = 100;
const MAX_MAX_RESULTS: usize = 500;

/// `GrepToolOptions`。`WalkOptions` の field を平坦化して持ち、既定値は
/// `limits` 定数から解決する(TS と同じ注入パターン)。
pub struct GrepToolOptions<'a> {
    /// canonical 化済み workspace root(`WalkOptions.root`)。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    /// privacy deny list。deny file は walk 結果にも footer にも出さない。
    pub denylist: Option<&'a PathDenylist>,
    /// `observeAccess`(access 層の観測 callback)。
    pub observe_access: Option<&'a AccessObserver<'a>>,
    /// 同一 file の全 batch に累積される regex 評価予算 ms(既定
    /// `GREP_EVAL_PER_FILE_MS`)。超過で file を timeout skip。
    pub eval_per_file_ms: Option<u64>,
    /// 1 回の grep 呼出しの累計 regex 評価上限 ms(既定
    /// `GREP_EVAL_TOTAL_MS`)。超過で走査停止。
    pub eval_total_ms: Option<u64>,
    /// walk+I/O+eval 待機を含む走査全体の wall-clock deadline ms(既定
    /// `GREP_SCAN_DEADLINE_MS`)。
    pub scan_deadline_ms: Option<u64>,
    /// `File::read` chunk byte 数の縮退注入(既定 `IO_CHUNK_BYTES`)。
    pub chunk_bytes: Option<usize>,
    /// 評価対象にする 1 行の raw byte 上限の縮退注入(既定
    /// `GREP_MAX_LINE_BYTES`)。raw byte の行は UTF-8 decode で最大 3 倍へ
    /// 展開しうる(invalid byte → U+FFFD 3 byte)ため、decoded payload は
    /// 最大 `3 * max_line_bytes`。
    pub max_line_bytes: Option<usize>,
    /// 1 batch の行数上限の縮退注入(既定 `GREP_BATCH_MAX_LINES`)。
    pub batch_max_lines: Option<usize>,
    /// 1 batch の行テキスト合計 byte 上限の縮退注入(既定
    /// `GREP_BATCH_MAX_BYTES`)。invariant: `batch_max_bytes >=
    /// 3 * max_line_bytes`(1 行の最大 decoded payload が空 batch に
    /// 収まり、「1 batch ≤ B」が厳密に保たれる)。違反する組合せは
    /// [`create_grep_tool`] 構築時に reject される。
    pub batch_max_bytes: Option<usize>,
    /// 出力本文の総 byte 上限の縮退注入(既定 `GREP_OUTPUT_MAX_BYTES`)。
    pub output_max_bytes: Option<usize>,
    /// 出力 1 行の表示文字数(UTF-16 code unit)上限の縮退注入(既定
    /// `GREP_OUTPUT_LINE_MAX_CHARS`)。
    pub output_line_max_chars: Option<usize>,
    /// skip 理由ごとの代表 path 件数の縮退注入(既定
    /// `SKIP_FOOTER_MAX_ENTRIES`)。
    pub skip_footer_max_entries: Option<usize>,
    /// footer 内 1 path の表示文字数の縮退注入(既定
    /// `SKIP_PATH_MAX_CHARS`)。
    pub skip_path_max_chars: Option<usize>,
    /// skip footer 全体の byte 上限の縮退注入(既定
    /// `SKIP_FOOTER_MAX_BYTES`)。
    pub skip_footer_max_bytes: Option<usize>,
    /// eval subprocess の spawn 差替(既定 `current_exe` + `__grep-eval`)。
    /// test は test binary を helper として差し替える。
    pub eval_spawn: Option<super::grep_eval::EvalSpawn>,
    /// eval reply frame の byte 上限の縮退注入(既定
    /// `EVAL_REPLY_MAX_BYTES`)。
    pub eval_reply_max_bytes: Option<usize>,
    /// eval 予算・deadline の時間源の差し替え(既定
    /// `Instant` 基準の monotonic ms)。test 用。
    pub now: Option<Box<dyn Fn() -> f64 + Send + Sync + 'a>>,
}

/// grep tool の parameter(grepSchema 検証済み入力)。
#[derive(Debug, Default)]
pub struct GrepParams<'a> {
    /// JavaScript regular expression。
    pub pattern: &'a str,
    /// 検索 root(workspace 相対、既定 `.`)。
    pub path: Option<&'a str>,
    /// workspace 相対 path に対する glob filter。
    pub glob: Option<&'a str>,
    /// `i` flag。
    pub ignore_case: bool,
    /// 1..=500(既定 100)。schema 側で clamp 済み想定だが内部でも min を取る。
    pub max_results: Option<usize>,
    /// context 行数 0..=3(既定 0)。
    pub context: Option<usize>,
}

fn err(msg: impl Into<String>) -> ToolError {
    std::io::Error::other(msg.into()).into()
}

/// 走査停止理由(TS `stopReason`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopReason {
    ResultCap,
    Output,
    Eval,
    Deadline,
}

/// `[skipped: ...]` footer の 1 バケツ。
#[derive(Default)]
struct SkipBucket {
    shown: Vec<String>,
    total: usize,
}

/// 4 分類の skip 集計。
#[derive(Default)]
struct Skipped {
    binary: SkipBucket,
    unreadable: SkipBucket,
    oversized: SkipBucket,
    timeout: SkipBucket,
}

fn record_skip(bucket: &mut SkipBucket, entry: String, max_entries: usize, max_chars: usize) {
    bucket.total += 1;
    if bucket.shown.len() < max_entries {
        bucket
            .shown
            .push(slice_utf16(&entry, max_chars).to_string());
    }
}

/// `createGrepTool` の返り値。構築時に `B >= 3 * L` invariant を検証する
/// (TS は `createGrepTool` 呼出し時に throw)。
pub struct GrepTool<'a> {
    opts: GrepToolOptions<'a>,
    eval_opts: EvalOptions,
}

/// `createGrepTool`。invariant 違反はここで reject する。
pub fn create_grep_tool(opts: GrepToolOptions<'_>) -> Result<GrepTool<'_>, ToolError> {
    let eff_max_line_bytes = opts.max_line_bytes.unwrap_or(GREP_MAX_LINE_BYTES);
    let eff_batch_max_bytes = opts.batch_max_bytes.unwrap_or(GREP_BATCH_MAX_BYTES);
    if eff_batch_max_bytes < 3 * eff_max_line_bytes {
        return Err(err(format!(
            "grep: batchMaxBytes ({eff_batch_max_bytes}) must be >= 3 * maxLineBytes \
             ({}); a single decoded line payload must fit in one batch",
            3 * eff_max_line_bytes
        )));
    }
    let eval_opts = EvalOptions {
        spawn: match &opts.eval_spawn {
            Some(s) => s.clone(),
            None => super::grep_eval::EvalSpawn::current_exe().map_err(err_io)?,
        },
        reply_max_bytes: opts.eval_reply_max_bytes.unwrap_or(EVAL_REPLY_MAX_BYTES),
    };
    Ok(GrepTool { opts, eval_opts })
}

fn err_io(e: std::io::Error) -> ToolError {
    Box::new(e)
}

impl GrepTool<'_> {
    /// canonical `execute(input, signal)` — raw `Value` params を
    /// `GrepParams` へ写し、`AbortSignal` を `AbortCheck` callback へ
    /// adapt して [`execute_grep`] を呼ぶ(TS `createGrepTool().execute`
    /// 相当。`input` の検証は schema/registry 側責務)。
    pub fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let params = GrepParams {
            pattern: input
                .get("pattern")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            path: input.get("path").and_then(Value::as_str),
            glob: input.get("glob").and_then(Value::as_str),
            ignore_case: input
                .get("ignoreCase")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            max_results: input
                .get("maxResults")
                .and_then(Value::as_u64)
                .map(|n| n as usize),
            context: input
                .get("context")
                .and_then(Value::as_u64)
                .map(|n| n as usize),
        };
        // `signal.aborted` → `AbortCheck`(poll callback)へ adapt。
        let check;
        let aborted = match signal {
            Some(signal) => {
                check = move || signal.is_aborted();
                Some(&check as &AbortCheck)
            }
            None => None,
        };
        let text = execute_grep(self, &params, aborted)?;
        Ok(CanonicalToolResult::text(text))
    }
}

/// `grep` tool の canonical definition(TS `createGrepTool` の
/// `defineCanonicalTool` 相当)。
pub fn grep_tool_definition(
    opts: GrepToolOptions<'_>,
) -> Result<crate::definition::CanonicalToolDefinition<'_>, ToolError> {
    let tool = std::sync::Arc::new(create_grep_tool(opts)?);
    Ok(crate::definition::define_canonical_tool(
        crate::definition::CanonicalToolDefinition {
            name: cheaper_core::tools::vocabulary::ToolName::Grep,
            label: "grep".to_string(),
            description: "Search file contents with a JavaScript regular expression. Returns matching lines as path:line: text. Searches allowed files of any size; binary files, unreadable files, oversized lines, and per-file evaluation timeouts are skipped and reported in a bounded footer. Skips .git and node_modules.".to_string(),
            parameters: catalog::grep_schema()
                .as_object()
                .expect("object schema")
                .clone(),
            prepare_arguments: None,
            execute: Box::new(move |input, signal| {
                let tool = std::sync::Arc::clone(&tool);
                Box::pin(async move { tool.execute(&input, signal.as_ref()) })
            }),
        },
    ))
}

/// framer callback → scan loop への受け渡し(`Send` 制約のため
/// `mpsc::channel` 経由で戻す)。
enum FramerEvent {
    Line {
        line_no: usize,
        text: String,
        payload_bytes: usize,
    },
    Oversized {
        line_no: usize,
    },
}

/// batch 送信用に集めた 1 行。
struct PendingLine {
    line_no: usize,
    text: String,
    /// worker へ送る decoded payload の UTF-8 byte 長(batch byte 計上用)。
    payload_bytes: usize,
}

/// 走査全体の共有状態(TS の execute 内ローカル変数群)。
struct Scan {
    out_lines: Vec<String>,
    out_bytes: usize,
    matches: usize,
    eval_total_ms: f64,
    stop_reason: Option<StopReason>,
    output_max_bytes: usize,
}

impl Scan {
    /// TS `emitRaw`。`out_bytes + need > output_max_bytes` で
    /// `output` stop を立てて emit を拒否する。
    fn emit_raw(&mut self, s: &str) -> bool {
        let need = s.len() + usize::from(!self.out_lines.is_empty());
        if self.out_bytes + need > self.output_max_bytes {
            self.stop_reason.get_or_insert(StopReason::Output);
            return false;
        }
        self.out_lines.push(s.to_string());
        self.out_bytes += need;
        true
    }
}

/// 1 file の走査状態 + TS closure 群(sendBatch/drain/emitText/lookup)。
struct FileScan<'s> {
    scan: &'s mut Scan,
    skipped: &'s RefCell<Skipped>,
    evaluator: &'s mut GrepEvalWorker,
    eval_opts: &'s EvalOptions,
    now: &'s (dyn Fn() -> f64 + Send + Sync),
    aborted: Option<&'s AbortCheck<'s>>,
    display: &'s str,
    pattern: &'s str,
    flags: &'s str,
    per_file_ms: f64,
    total_ms: f64,
    deadline_at: f64,
    context: usize,
    max_results: usize,
    output_line_max_chars: usize,
    footer_max_entries: usize,
    path_max_chars: usize,
    batch_max_lines: usize,
    batch_max_bytes: usize,
    // per-file 状態(TS と同名)
    pending: VecDeque<PendingLine>,
    ring: VecDeque<(usize, String)>,
    batch_lines: Vec<String>,
    batch_start: usize,
    batch_bytes: usize,
    file_eval_ms: f64,
    open_block_to: usize,
    last_emitted: usize,
    need_sep: bool,
    file_abandoned: bool,
    file_batches: usize,
}

impl<'s> FileScan<'s> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        scan: &'s mut Scan,
        skipped: &'s RefCell<Skipped>,
        evaluator: &'s mut GrepEvalWorker,
        eval_opts: &'s EvalOptions,
        now: &'s (dyn Fn() -> f64 + Send + Sync),
        aborted: Option<&'s AbortCheck<'s>>,
        display: &'s str,
        pattern: &'s str,
        flags: &'s str,
        budgets: (f64, f64, f64),
        caps: (usize, usize, usize, usize, usize, usize, usize),
    ) -> Self {
        Self {
            scan,
            skipped,
            evaluator,
            eval_opts,
            now,
            aborted,
            display,
            pattern,
            flags,
            per_file_ms: budgets.0,
            total_ms: budgets.1,
            deadline_at: budgets.2,
            context: caps.0,
            max_results: caps.1,
            output_line_max_chars: caps.2,
            footer_max_entries: caps.3,
            path_max_chars: caps.4,
            batch_max_lines: caps.5,
            batch_max_bytes: caps.6,
            pending: VecDeque::new(),
            ring: VecDeque::new(),
            batch_lines: Vec::new(),
            batch_start: 0,
            batch_bytes: 0,
            file_eval_ms: 0.0,
            open_block_to: 0,
            last_emitted: 0,
            need_sep: false,
            file_abandoned: false,
            file_batches: 0,
        }
    }

    fn now(&self) -> f64 {
        (self.now)()
    }

    fn aborted(&self) -> bool {
        self.aborted.is_some_and(|a| a())
    }

    fn record_timeout_skip(&self) {
        record_skip(
            &mut self.skipped.borrow_mut().timeout,
            self.display.to_string(),
            self.footer_max_entries,
            self.path_max_chars,
        );
    }

    /// TS `emitText`。
    fn emit_text(&mut self, is_match: bool, line_no: usize, text: &str) -> bool {
        let truncated = if utf16_len(text) > self.output_line_max_chars {
            format!(
                "{}... [truncated]",
                slice_utf16(text, self.output_line_max_chars)
            )
        } else {
            text.to_string()
        };
        if is_match {
            self.scan
                .emit_raw(&format!("{}:{}: {}", self.display, line_no, truncated))
        } else {
            self.scan
                .emit_raw(&format!("{}-{}- {}", self.display, line_no, truncated))
        }
    }

    /// batch 内 0-indexed 行 → 絶対行番号の写像用 lookup(TS `lookup`)。
    /// 戻り値は clone(`emit_text` が `&mut self` を取るため借用を分離する)。
    /// context 行は高々 `context`(≤3)件/match のため bounded。
    fn lookup(&self, line_no: usize, lines: &[String], start: usize) -> Option<String> {
        if line_no >= start {
            lines.get(line_no - start).cloned()
        } else {
            self.ring
                .iter()
                .find(|(n, _)| *n == line_no)
                .map(|(_, t)| t.clone())
        }
    }

    /// TS `sendBatch`。batch 送信 → result 待ち → context 付き出力。
    /// timeout は file skip 化、他 error は呼出し側へ伝播する。
    fn send_batch(&mut self) -> Result<(), ToolError> {
        let lines = std::mem::take(&mut self.batch_lines);
        let start = self.batch_start;
        self.batch_bytes = 0;
        self.file_batches += 1;
        if self.evaluator.is_dead() {
            *self.evaluator = GrepEvalWorker::spawn(self.eval_opts, self.pattern, self.flags)
                .map_err(|e| -> ToolError { Box::new(e) })?;
        }
        // watchMs = min(per-file 残・total 残・deadline 残)
        let watch_ms = (self.per_file_ms - self.file_eval_ms)
            .min(self.total_ms - self.scan.eval_total_ms)
            .min(self.deadline_at - self.now());
        let t0 = self.now();
        let matched =
            match self
                .evaluator
                .evaluate(self.display, start, &lines, watch_ms, self.aborted)
            {
                Ok(m) => m,
                Err(e) => {
                    if self.aborted() {
                        return Err(err(ABORTED_MESSAGE));
                    }
                    match e {
                        EvalError::Timeout => {
                            self.scan.eval_total_ms += self.now() - t0;
                            self.record_timeout_skip();
                            if self.now() >= self.deadline_at {
                                self.scan.stop_reason.get_or_insert(StopReason::Deadline);
                            } else if self.scan.eval_total_ms >= self.total_ms {
                                self.scan.stop_reason.get_or_insert(StopReason::Eval);
                            } else {
                                // per-file 予算超過 → 当該 file を捨てて次へ
                                self.file_abandoned = true;
                            }
                            return Ok(());
                        }
                        other => return Err(Box::new(other)),
                    }
                }
            };
        let dt = self.now() - t0;
        self.file_eval_ms += dt;
        self.scan.eval_total_ms += dt;

        // batch 内 0-indexed matched を startLine で絶対行番号へ写像し、
        // ring(前 context)と pending countdown(後 context)で block merge
        // しながら出力へ流す。全行配列・全一致配列は作らない。
        let mut mi = 0usize;
        let end = start + lines.len();
        for n in start..end {
            let is_match = mi < matched.len() && matched[mi] == n - start;
            if is_match {
                mi += 1;
                let from = n.saturating_sub(self.context).max(1);
                if from > self.open_block_to + 1
                    && self.need_sep
                    && self.context > 0
                    && !self.scan.emit_raw("--")
                {
                    break;
                }
                let mut i = from.max(self.last_emitted + 1);
                while i < n {
                    if let Some(t) = self.lookup(i, &lines, start) {
                        if !self.emit_text(false, i, &t) {
                            break;
                        }
                    }
                    i += 1;
                }
                if self.scan.stop_reason.is_some() {
                    break;
                }
                if !self.emit_text(true, n, &lines[n - start]) {
                    break;
                }
                self.need_sep = true;
                self.open_block_to = self.open_block_to.max(n + self.context);
                self.last_emitted = n;
                self.scan.matches += 1;
                if self.scan.matches >= self.max_results {
                    self.scan.stop_reason.get_or_insert(StopReason::ResultCap);
                    break;
                }
            } else if n <= self.open_block_to {
                if !self.emit_text(false, n, &lines[n - start]) {
                    break;
                }
                self.last_emitted = n;
            }
        }

        // ring を次 batch の pre-context 用に更新(直近 context 行のみ保持)
        for (i, line) in lines.iter().enumerate() {
            self.ring.push_back((start + i, line.clone()));
        }
        while self.ring.len() > self.context {
            self.ring.pop_front();
        }
        Ok(())
    }

    /// TS `drain`。batch を batchMaxLines 行・batchMaxBytes byte・
    /// oversized 行を挟む不連続のいずれかで締めて送る。B は hard cap。
    /// `force` 時は partial batch も送る。
    fn drain(&mut self, force: bool) -> Result<(), ToolError> {
        while !self.file_abandoned && self.scan.stop_reason.is_none() {
            let mut closed_by_bytes = false;
            while self.batch_lines.len() < self.batch_max_lines && !self.pending.is_empty() {
                let head = self.pending.front().expect("non-empty");
                if !self.batch_lines.is_empty()
                    && head.line_no != self.batch_start + self.batch_lines.len()
                {
                    break; // oversized 行を挟む不連続 → 現在 batch を締める
                }
                // B は hard cap: 次行を足すと超えるなら現在 batch を締める。
                // 空 batch には必ず積むため、1 行が B を超える場合(縮退した
                // test 注入 limit でのみ到達可能)も 1 行 batch として送り、
                // 進行は保証される。
                if !self.batch_lines.is_empty()
                    && self.batch_bytes + head.payload_bytes > self.batch_max_bytes
                {
                    closed_by_bytes = true;
                    break;
                }
                let head = self.pending.pop_front().expect("front");
                if self.batch_lines.is_empty() {
                    self.batch_start = head.line_no;
                }
                self.batch_bytes += head.payload_bytes;
                self.batch_lines.push(head.text);
            }
            if self.batch_lines.is_empty() {
                return Ok(());
            }
            let closed_by_gap = !self.pending.is_empty()
                && self.pending.front().expect("non-empty").line_no
                    != self.batch_start + self.batch_lines.len();
            if self.batch_lines.len() < self.batch_max_lines
                && !closed_by_bytes
                && !closed_by_gap
                && !force
            {
                return Ok(()); // partial batch は次の feed/EOF まで保留
            }
            // 送信前に残予算を検査(超過なら送らずに打ち切る)
            if self.file_eval_ms >= self.per_file_ms {
                self.record_timeout_skip();
                self.evaluator.kill();
                self.file_abandoned = true;
                return Ok(());
            }
            if self.scan.eval_total_ms >= self.total_ms {
                // file 未着手なら名指ししない(scan-level の停止は marker が担う)
                if self.file_batches > 0 {
                    self.record_timeout_skip();
                }
                self.scan.stop_reason.get_or_insert(StopReason::Eval);
                return Ok(());
            }
            if self.now() >= self.deadline_at {
                if self.file_batches > 0 {
                    self.record_timeout_skip();
                }
                self.scan.stop_reason.get_or_insert(StopReason::Deadline);
                return Ok(());
            }
            self.send_batch()?;
        }
        Ok(())
    }
}

/// grep tool の execute 相当。結果は tool result の text body
/// (`content[0].text`)。error は TS の throw に対応する `Err`。
/// `aborted` は `signal.aborted` 相当の観測 callback。
pub fn execute_grep(
    tool: &GrepTool<'_>,
    params: &GrepParams<'_>,
    aborted: Option<&AbortCheck<'_>>,
) -> Result<String, ToolError> {
    let opts = &tool.opts;
    if utf16_len(params.pattern) > GREP_PATTERN_MAX_CHARS {
        return Err(err(format!(
            "invalid grep pattern: pattern exceeds {GREP_PATTERN_MAX_CHARS} characters"
        )));
    }
    let flags: &str = if params.ignore_case { "i" } else { "" };
    // main 側で事前検証(TS `new RegExp(params.pattern, flags)`)。
    // helper 側 compile が失敗しても error reply として fail closed する。
    let effective_pattern: &str = if params.pattern.is_empty() {
        // JS `new RegExp("").source === "(?:)"`。regress が空 pattern を
        // 拒否する構成でも同じ意味を送る(worker には source を送る parity)。
        "(?:)"
    } else {
        params.pattern
    };
    regress::Regex::with_flags(effective_pattern, flags)
        .map_err(|e| err(format!("invalid grep pattern: {e}")))?;
    let start_abs = params.path.unwrap_or(".");
    let max_results = params
        .max_results
        .unwrap_or(DEFAULT_MAX_RESULTS)
        .min(MAX_MAX_RESULTS);
    let context = params.context.unwrap_or(0);
    let per_file_ms = opts.eval_per_file_ms.unwrap_or(GREP_EVAL_PER_FILE_MS) as f64;
    let total_ms = opts.eval_total_ms.unwrap_or(GREP_EVAL_TOTAL_MS) as f64;
    let default_now = || -> f64 {
        use std::sync::OnceLock;
        use std::time::Instant;
        static T0: OnceLock<Instant> = OnceLock::new();
        T0.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    };
    let now: &(dyn Fn() -> f64 + Send + Sync) = match &opts.now {
        Some(f) => f.as_ref(),
        None => &default_now,
    };
    let deadline_at = now() + opts.scan_deadline_ms.unwrap_or(GREP_SCAN_DEADLINE_MS) as f64;
    let chunk_bytes = opts.chunk_bytes.unwrap_or(IO_CHUNK_BYTES);
    let max_line_bytes = opts.max_line_bytes.unwrap_or(GREP_MAX_LINE_BYTES);
    let batch_max_lines = opts.batch_max_lines.unwrap_or(GREP_BATCH_MAX_LINES);
    let batch_max_bytes = opts.batch_max_bytes.unwrap_or(GREP_BATCH_MAX_BYTES);
    let output_max_bytes = opts.output_max_bytes.unwrap_or(GREP_OUTPUT_MAX_BYTES);
    let output_line_max_chars = opts
        .output_line_max_chars
        .unwrap_or(GREP_OUTPUT_LINE_MAX_CHARS);
    let footer_max_entries = opts
        .skip_footer_max_entries
        .unwrap_or(SKIP_FOOTER_MAX_ENTRIES);
    let path_max_chars = opts.skip_path_max_chars.unwrap_or(SKIP_PATH_MAX_CHARS);
    let footer_max_bytes = opts.skip_footer_max_bytes.unwrap_or(SKIP_FOOTER_MAX_BYTES);

    let mut evaluator = GrepEvalWorker::spawn(&tool.eval_opts, effective_pattern, flags)
        .map_err(|e| -> ToolError { Box::new(e) })?;

    // glob filter は `crate::glob::matches_glob`
    // (`path.posix.matchesGlob` の UTF-16 忠実 port)。display は POSIX
    // 区切り。minimatch が不正な正規表現を生成する pattern で Node が
    // SyntaxError を投げる case は Rust 側では false 相当として扱う
    // (accepted divergence)。over-long pattern (>65536 UTF-16 unit)は
    // Node の `TypeError("pattern is too long")` と同様に `Err` となり、
    // file 毎の評価時に propagate する。
    let glob_matcher = params.glob;

    let is_aborted = || aborted.is_some_and(|a| a());
    let check_scan = |stop_reason: &mut Option<StopReason>| -> Result<(), ToolError> {
        if is_aborted() {
            return Err(err(ABORTED_MESSAGE));
        }
        if now() >= deadline_at {
            stop_reason.get_or_insert(StopReason::Deadline);
        }
        Ok(())
    };

    let mut scan = Scan {
        out_lines: Vec::new(),
        out_bytes: 0,
        matches: 0,
        eval_total_ms: 0.0,
        stop_reason: None,
        output_max_bytes,
    };
    // 各 reason の代表 entry は footerMaxEntries 件まで保持し、残りは
    // total で `and N more` に集計する(oversized 行が大量にあっても
    // 蓄積は有界)。
    let skipped: RefCell<Skipped> = RefCell::new(Skipped::default());

    {
        let mut on_issue = |issue: crate::walker::WalkIssue| {
            record_skip(
                &mut skipped.borrow_mut().unreadable,
                issue.display,
                footer_max_entries,
                path_max_chars,
            );
        };
        let walk_opts = WalkOptions {
            observe_access: opts.observe_access,
            root: opts.root,
            symlink_policy: opts.symlink_policy,
            denylist: opts.denylist,
        };
        let walk = walk_files(&walk_opts, start_abs, Some(&mut on_issue))
            .map_err(|e| rethrow_as_denied(Box::new(e)))?;
        for file in walk {
            // for-await の列挙 error は checkScan より先に伝播する。
            let file = match file {
                Ok(f) => f,
                Err(e) => return Err(rethrow_as_denied(e)),
            };
            check_scan(&mut scan.stop_reason)?;
            if scan.stop_reason.is_some() {
                break;
            }
            if let Some(g) = glob_matcher {
                // find と同様、over-long pattern の TypeError は file 毎の
                // lazy 評価で propagate する。
                if !crate::glob::matches_glob(&file.display, g)? {
                    continue;
                }
            }
            // TS は for-await 全体を try/catch(rethrowAsDenied)で包む。
            // walk 中の escape error は denied 3 message に写像される。
            match sniff_text_file(&file.real, opts.observe_access).map_err(rethrow_as_denied)? {
                SniffResult::Binary => {
                    record_skip(
                        &mut skipped.borrow_mut().binary,
                        file.display,
                        footer_max_entries,
                        path_max_chars,
                    );
                    continue;
                }
                SniffResult::Unreadable => {
                    record_skip(
                        &mut skipped.borrow_mut().unreadable,
                        file.display,
                        footer_max_entries,
                        path_max_chars,
                    );
                    continue;
                }
                SniffResult::Text => {}
            }
            if let Some(observe) = opts.observe_access {
                observe(&file.real, ClaimMode::Shared, None, false).map_err(rethrow_as_denied)?;
            }
            let mut fh = match fs::File::open(&file.real) {
                Ok(fh) => fh,
                Err(_) => {
                    record_skip(
                        &mut skipped.borrow_mut().unreadable,
                        file.display,
                        footer_max_entries,
                        path_max_chars,
                    );
                    continue;
                }
            };

            // per-file 状態。batch は常に連続行番号を保つ(oversized 行は
            // 行番号を消費するためそこで batch を締め、0-indexed ↔ 絶対行番号
            // の写像を維持する)。
            let mut fs_can = FileScan::new(
                &mut scan,
                &skipped,
                &mut evaluator,
                &tool.eval_opts,
                now,
                aborted,
                &file.display,
                effective_pattern,
                flags,
                (per_file_ms, total_ms, deadline_at),
                (
                    context,
                    max_results,
                    output_line_max_chars,
                    footer_max_entries,
                    path_max_chars,
                    batch_max_lines,
                    batch_max_bytes,
                ),
            );

            // framer callback は `Send` 制約があるため channel 経由で
            // pending/skip へ流す(feed 完了時に全 event を回収する)。
            let (ev_tx, ev_rx) = channel::<FramerEvent>();
            let mut framer = {
                let tx = ev_tx.clone();
                let tx2 = ev_tx;
                create_line_framer(LineFramerOptions {
                    max_line_bytes,
                    on_line: Box::new(move |line, line_no, _raw| {
                        let _ = tx.send(FramerEvent::Line {
                            line_no,
                            text: line.to_string(),
                            payload_bytes: line.len(),
                        });
                    }),
                    on_oversized: Box::new(move |line_no| {
                        let _ = tx2.send(FramerEvent::Oversized { line_no });
                    }),
                })
            };
            let drain_events = |fs_can: &mut FileScan<'_>, ev_rx: &Receiver<FramerEvent>| {
                while let Ok(ev) = ev_rx.try_recv() {
                    match ev {
                        FramerEvent::Line {
                            line_no,
                            text,
                            payload_bytes,
                        } => fs_can.pending.push_back(PendingLine {
                            line_no,
                            text,
                            payload_bytes,
                        }),
                        FramerEvent::Oversized { line_no } => record_skip(
                            &mut fs_can.skipped.borrow_mut().oversized,
                            format!("{}:{line_no}", fs_can.display),
                            fs_can.footer_max_entries,
                            fs_can.path_max_chars,
                        ),
                    }
                }
            };

            {
                let mut buf = vec![0u8; chunk_bytes];
                loop {
                    check_scan(&mut fs_can.scan.stop_reason)?;
                    if fs_can.scan.stop_reason.is_some() || fs_can.file_abandoned {
                        break;
                    }
                    let n = match fh.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => n,
                        Err(_) => {
                            // 途中 I/O error → file は不完全。unreadable
                            // skip として記録し次へ。
                            record_skip(
                                &mut skipped.borrow_mut().unreadable,
                                file.display.clone(),
                                footer_max_entries,
                                path_max_chars,
                            );
                            fs_can.file_abandoned = true;
                            break;
                        }
                    };
                    framer.feed(&buf[..n]);
                    drain_events(&mut fs_can, &ev_rx);
                    fs_can.drain(false)?;
                }
                if !fs_can.file_abandoned && fs_can.scan.stop_reason.is_none() {
                    framer.flush();
                    drain_events(&mut fs_can, &ev_rx);
                    fs_can.drain(true)?;
                }
            }
            if scan.stop_reason.is_some() {
                break;
            }
        }
    }
    // evaluator は Drop で kill される(TS finally `evaluator.dispose()`)。

    // [skipped: ...] 行。1 entry ≤ pathMaxChars、skipped 部全体は
    // footerMaxBytes byte で締め、超過は [skipped-report: truncated] で示す。
    let mut footer: Vec<String> = Vec::new();
    let mut skipped_bytes = 0usize;
    let mut skipped_report_truncated = false;
    let skipped = skipped.into_inner();
    for (reason, bucket) in [
        ("binary", &skipped.binary),
        ("unreadable", &skipped.unreadable),
        ("oversized-line", &skipped.oversized),
        ("timeout", &skipped.timeout),
    ] {
        if bucket.total == 0 {
            continue;
        }
        let more = bucket.total - bucket.shown.len();
        let list = if more > 0 {
            if bucket.shown.is_empty() {
                format!("and {more} more")
            } else {
                format!("{}, and {more} more", bucket.shown.join(", "))
            }
        } else {
            bucket.shown.join(", ")
        };
        let line = format!("[skipped: {reason} — {list}]");
        let need = line.len() + usize::from(!footer.is_empty());
        if skipped_bytes + need > footer_max_bytes {
            skipped_report_truncated = true;
            break;
        }
        footer.push(line);
        skipped_bytes += need;
    }
    match scan.stop_reason {
        Some(StopReason::Eval) => footer.push("[truncated: evaluation time limit reached]".into()),
        Some(StopReason::Deadline) => footer.push("[truncated: scan deadline reached]".into()),
        Some(StopReason::ResultCap) => footer.push("[truncated: more matches]".into()),
        Some(StopReason::Output) => footer.push("[truncated: output limit reached]".into()),
        None => {}
    }
    if skipped_report_truncated {
        footer.push("[skipped-report: truncated]".into());
    }

    let mut body = if scan.out_lines.is_empty() {
        "(no matches)".to_string()
    } else {
        scan.out_lines.join("\n")
    };
    if !footer.is_empty() {
        body.push('\n');
        body.push_str(&footer.join("\n"));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use std::fs as stdfs;
    use std::path::Path;

    use cheaper_core::workspace::deny_paths::create_path_denylist;

    use super::super::grep_eval::test_support::test_spawn;
    use super::*;

    fn tmpdir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}{}", fastrand::u64(..)));
        stdfs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn root_of(dir: &Path) -> String {
        cheaper_core::workspace::paths::realpath(dir.to_str().unwrap()).unwrap()
    }

    fn opts<'a>(root: &'a str, o: GrepToolOptions<'a>) -> GrepToolOptions<'a> {
        GrepToolOptions {
            root,
            symlink_policy: SymlinkPolicy::Deny,
            eval_spawn: Some(test_spawn()),
            ..o
        }
    }

    fn default_options<'a>() -> GrepToolOptions<'a> {
        GrepToolOptions {
            root: "",
            symlink_policy: SymlinkPolicy::Deny,
            denylist: None,
            observe_access: None,
            eval_per_file_ms: None,
            eval_total_ms: None,
            scan_deadline_ms: None,
            chunk_bytes: None,
            max_line_bytes: None,
            batch_max_lines: None,
            batch_max_bytes: None,
            output_max_bytes: None,
            output_line_max_chars: None,
            skip_footer_max_entries: None,
            skip_path_max_chars: None,
            skip_footer_max_bytes: None,
            eval_spawn: None,
            eval_reply_max_bytes: None,
            now: None,
        }
    }

    fn tool(root: &str) -> GrepTool<'_> {
        create_grep_tool(opts(root, default_options())).expect("tool")
    }

    fn run(t: &GrepTool<'_>, params: GrepParams<'_>) -> Result<String, ToolError> {
        execute_grep(t, &params, None)
    }

    fn p(pattern: &str) -> GrepParams<'_> {
        GrepParams {
            pattern,
            ..Default::default()
        }
    }

    // ---- parity ----

    #[test]
    fn finds_matches_as_display_line_text() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "foo\nbar\nfoo bar\n").unwrap();
        let out = run(&tool(&root), p("foo")).unwrap();
        assert_eq!(out, "a.txt:1: foo\na.txt:3: foo bar");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn honors_ignore_case() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "Foo\nfOO\n").unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "foo",
                ignore_case: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "a.txt:1: Foo\na.txt:2: fOO");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn filters_by_glob() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
        stdfs::write(dir.join("a.md"), "hit\n").unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "hit",
                glob: Some("*.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "a.txt:1: hit");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn glob_star_does_not_cross_directories() {
        // `path.posix.matchesGlob("sub/x.txt", "*.txt")` === false
        // (`*` は `/` をまたがない)。`sub/*.txt` は一致する。
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::create_dir_all(dir.join("sub")).unwrap();
        stdfs::write(dir.join("sub").join("x.txt"), "hit\n").unwrap();
        stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "hit",
                glob: Some("*.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "a.txt:1: hit");
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "hit",
                glob: Some("sub/*.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "sub/x.txt:1: hit");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn searches_subdirectory_via_path() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::create_dir_all(dir.join("sub")).unwrap();
        stdfs::write(dir.join("sub").join("s.txt"), "needle\n").unwrap();
        stdfs::write(dir.join("top.txt"), "needle\n").unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "needle",
                path: Some("sub"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "sub/s.txt:1: needle");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn caps_results_at_max_results() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let lines: Vec<String> = (0..10).map(|i| format!("m{i}")).collect();
        stdfs::write(dir.join("many.txt"), format!("{}\n", lines.join("\n"))).unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "m\\d",
                max_results: Some(3),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            out,
            "many.txt:1: m0\nmany.txt:2: m1\nmany.txt:3: m2\n[truncated: more matches]"
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn emits_context_lines() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("c.txt"), "l1\nl2\nhit\nl4\nl5\n").unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "hit",
                context: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "c.txt-2- l2\nc.txt:3: hit\nc.txt-4- l4");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn emits_separator_between_disjoint_context_blocks() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let body: Vec<String> = (0..12)
            .map(|i| {
                if i == 2 || i == 9 {
                    "hit".to_string()
                } else {
                    format!("l{i}")
                }
            })
            .collect();
        stdfs::write(dir.join("sep.txt"), format!("{}\n", body.join("\n"))).unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "hit",
                context: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            out,
            "sep.txt-2- l1\nsep.txt:3: hit\nsep.txt-4- l3\n--\nsep.txt-9- l8\nsep.txt:10: hit\nsep.txt-11- l10"
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keeps_cr_verbatim_for_crlf() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("crlf.txt"), "a\r\nhit\r\n").unwrap();
        let out = run(&tool(&root), p("hit")).unwrap();
        assert_eq!(out, "crlf.txt:2: hit\r");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skips_node_modules() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::create_dir_all(dir.join("node_modules")).unwrap();
        stdfs::write(dir.join("node_modules").join("n.txt"), "hit\n").unwrap();
        stdfs::write(dir.join("t.txt"), "hit\n").unwrap();
        let out = run(&tool(&root), p("hit")).unwrap();
        assert_eq!(out, "t.txt:1: hit");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn matches_caret_dollar_on_line1_of_empty_file() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("empty.txt"), "").unwrap();
        let out = run(&tool(&root), p("^$")).unwrap();
        assert_eq!(out, "empty.txt:1: ");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn matches_caret_dollar_on_trailing_empty_line() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "a\n").unwrap();
        let out = run(&tool(&root), p("^$")).unwrap();
        assert_eq!(out, "a.txt:2: ");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_trailing_empty_line_without_final_newline() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "a").unwrap();
        let out = run(&tool(&root), p("^$")).unwrap();
        assert_eq!(out, "(no matches)");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_matches_marker() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "zzz\n").unwrap();
        let out = run(&tool(&root), p("qqq")).unwrap();
        assert_eq!(out, "(no matches)");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_invalid_regex() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let e = run(&tool(&root), p("(((")).expect_err("invalid");
        assert!(e.to_string().contains("invalid grep pattern"), "{e}");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_oversized_pattern() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let long = "a".repeat(GREP_PATTERN_MAX_CHARS + 1);
        let e = run(
            &tool(&root),
            GrepParams {
                pattern: long.as_str(),
                ..Default::default()
            },
        )
        .expect_err("too long");
        assert!(e.to_string().contains("invalid grep pattern"), "{e}");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn denies_path_outside_workspace() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let e = run(
            &tool(&root),
            GrepParams {
                pattern: "x",
                path: Some(".."),
                ..Default::default()
            },
        )
        .expect_err("denied");
        assert_eq!(
            e.to_string(),
            "Access denied: path is outside the workspace"
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    // ---- streaming / batch protocol ----

    #[test]
    fn finds_known_match_in_tail_of_large_file() {
        // 16MB 超 fixture は TS と同じ要領(ただし行数を落として CI を
        // 軽く保つ — streaming 証明には chunk/batch 境界を跨げば足りる)。
        let dir = tmpdir("cheaper-grep-big-");
        let root = root_of(&dir);
        let lines = 60_000usize;
        let mut buf = String::new();
        for i in 1..=lines {
            buf.push_str(&format!("line-{i:07} {}\n", "x".repeat(80)));
        }
        let needle_line = lines - 10;
        buf = buf.replacen(
            &format!("line-{needle_line:07} {}", "x".repeat(80)),
            &format!("line-{needle_line:07} UNIQUE_NEEDLE"),
            1,
        );
        assert!(buf.len() > 4 * 1024 * 1024);
        stdfs::write(dir.join("big.txt"), buf).unwrap();
        let out = run(
            &tool(&root),
            GrepParams {
                pattern: "UNIQUE_NEEDLE",
                glob: Some("big.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            out,
            format!("big.txt:{needle_line}: line-{needle_line:07} UNIQUE_NEEDLE")
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn matches_lines_spanning_chunk_and_batch_boundaries() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("x.txt"), "aaaaaaaa\nhit-target\nzz\n").unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                chunk_bytes: Some(5),
                batch_max_lines: Some(1),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "hit-target",
                glob: Some("x.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "x.txt:2: hit-target");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn merges_context_blocks_across_batch_boundaries() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("m.txt"), "l1\nhit\nl3\nhit\nl5\nhit\nl7\n").unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                batch_max_lines: Some(3),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "hit",
                context: Some(1),
                glob: Some("m.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            out,
            "m.txt-1- l1\nm.txt:2: hit\nm.txt-3- l3\nm.txt:4: hit\nm.txt-5- l5\nm.txt:6: hit\nm.txt-7- l7"
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn closes_batch_at_oversized_line_preserving_line_numbers() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(
            dir.join("o.txt"),
            "l1\nl2\nl3\nl4\nOVERSIZED-LINE-CONTENT\nhit6\n",
        )
        .unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                max_line_bytes: Some(10),
                batch_max_lines: Some(100),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "hit6",
                glob: Some("o.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "o.txt:6: hit6\n[skipped: oversized-line — o.txt:5]");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn closes_batch_before_exceeding_batch_max_bytes() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("b.txt"), "aaa\nbbb\ncccc\nddd\n").unwrap();
        // invariant: B=12 >= 3*L=12。"aaa"+"bbb"+"cccc"=10 byte で 1 batch、
        // "ddd" を足すと 13 > 12 なので次 batch に締める。
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                max_line_bytes: Some(4),
                batch_max_bytes: Some(12),
                batch_max_lines: Some(100),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "ddd",
                glob: Some("b.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "b.txt:4: ddd");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_construction_when_batch_bytes_below_3x_line_bytes() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let e = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                batch_max_bytes: Some(4),
                max_line_bytes: Some(100),
                ..default_options()
            },
        ))
        .err()
        .expect("invariant");
        assert!(e.to_string().contains("batchMaxBytes"), "{e}");
        let e = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                batch_max_bytes: Some(10),
                ..default_options()
            },
        ))
        .err()
        .expect("invariant");
        assert!(e.to_string().contains("batchMaxBytes"), "{e}");
        // 境界値 B = 3*L は受理される。
        create_grep_tool(opts(
            &root,
            GrepToolOptions {
                max_line_bytes: Some(4),
                batch_max_bytes: Some(12),
                ..default_options()
            },
        ))
        .expect("boundary ok");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    // ---- skip footer ----

    #[test]
    fn reports_binary_files_in_footer() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("b.dat"), [0x68u8, 0x69, 0x00, 0x74]).unwrap();
        stdfs::write(dir.join("t.txt"), "hit\n").unwrap();
        let out = run(&tool(&root), p("hit")).unwrap();
        assert_eq!(out, "t.txt:1: hit\n[skipped: binary — b.dat]");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reports_oversized_lines_as_path_line() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("o.txt"), "ok\nthis-is-a-very-long-line\nhit\n").unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                max_line_bytes: Some(10),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "hit",
                glob: Some("o.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "o.txt:3: hit\n[skipped: oversized-line — o.txt:2]");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn distinguishes_oversized_skip_from_no_matches() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("o.txt"), "a-very-very-long-line\n").unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                max_line_bytes: Some(10),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "hit",
                glob: Some("o.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out, "(no matches)\n[skipped: oversized-line — o.txt:1]");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn bounds_representative_entries_and_counts_rest() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        for i in 0..25 {
            stdfs::write(dir.join(format!("b{i:02}.dat")), [0x61u8, 0x00, 0x62]).unwrap();
        }
        let out = run(&tool(&root), p("hit")).unwrap();
        assert!(out.contains("[skipped: binary — "), "{out}");
        assert!(out.contains("and 5 more"), "{out}");
        let shown = out.matches(".dat").count();
        assert!(shown <= 20, "{out}");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn truncates_long_paths_at_skip_path_max_chars() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let long_name = format!("{}.dat", "d".repeat(200));
        stdfs::write(dir.join(&long_name), [0x61u8, 0x00]).unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                skip_path_max_chars: Some(20),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(&t, p("hit")).unwrap();
        assert_eq!(
            out,
            format!("(no matches)\n[skipped: binary — {}]", "d".repeat(20))
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn emits_skipped_report_truncated_over_footer_cap() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        for i in 0..4 {
            stdfs::write(
                dir.join(format!("long-name-{}-{i}.dat", "n".repeat(40))),
                [0x00u8],
            )
            .unwrap();
        }
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                skip_footer_max_bytes: Some(40),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(&t, p("hit")).unwrap();
        assert!(out.contains("[skipped-report: truncated]"), "{out}");
        assert!(!out.contains("[skipped: binary"), "{out}");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hides_denied_files_from_results_and_footer() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join(".env"), "SECRET=1\n").unwrap();
        stdfs::write(dir.join("normal.txt"), "hit\n").unwrap();
        let denylist = create_path_denylist(&[]);
        let t = create_grep_tool(GrepToolOptions {
            denylist: Some(&denylist),
            ..opts(&root, default_options())
        })
        .unwrap();
        let out = run(&t, p("SECRET")).unwrap();
        assert!(!out.contains(".env"), "{out}");
        assert!(!out.contains("SECRET=1"), "{out}");
        let out2 = run(&t, p("hit")).unwrap();
        assert_eq!(out2, "normal.txt:1: hit");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    // ---- eval budgets and deadline ----

    /// fake clock: 呼出ごとに `step` ms 進める(TS fakeClock 相当)。
    /// 1 batch の eval 計測は now() 複数呼出ぶんとして積算される。
    fn fake_clock(step: f64) -> Box<dyn Fn() -> f64 + Send + Sync> {
        let t = std::sync::Arc::new(std::sync::Mutex::new(0.0f64));
        Box::new(move || {
            let mut g = t.lock().unwrap();
            *g += step;
            *g
        })
    }

    #[test]
    fn per_file_budget_accumulates_skips_file_and_recovers() {
        // step=200ms → 1 batch の eval 計測(t0→dt で ~4 呼出)≈800ms。
        // perFile=1000 は 2 batch(1600ms)で累積超過する。watch は各 batch
        // で十分な残り(200ms〜)になるよう余裕を取る。
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let lines: Vec<String> = (0..30).map(|i| format!("l{i}")).collect();
        stdfs::write(dir.join("a.txt"), format!("{}\n", lines.join("\n"))).unwrap();
        stdfs::write(dir.join("b.txt"), "hit\n").unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                batch_max_lines: Some(3),
                eval_per_file_ms: Some(1000),
                now: Some(fake_clock(200.0)),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(&t, p("hit")).unwrap();
        assert!(out.contains("b.txt:1: hit"), "{out}");
        assert!(out.contains("[skipped: timeout — a.txt]"), "{out}");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn total_budget_stops_scan() {
        // 列挙順に依存しないよう両 file を同形にする。先に走査された
        // file が total 予算を使い切り、走査全体が止まる。
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let lines: Vec<String> = (0..30).map(|i| format!("l{i}")).collect();
        let body = format!("hit\n{}\n", lines.join("\n"));
        stdfs::write(dir.join("a.txt"), &body).unwrap();
        stdfs::write(dir.join("b.txt"), &body).unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                batch_max_lines: Some(3),
                eval_total_ms: Some(1600),
                now: Some(fake_clock(200.0)),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(&t, p("hit")).unwrap();
        assert!(
            out.contains("[truncated: evaluation time limit reached]"),
            "{out}"
        );
        let scanned: Vec<&str> = ["a.txt", "b.txt"]
            .iter()
            .copied()
            .filter(|n| out.contains(n))
            .collect();
        assert_eq!(scanned.len(), 1, "{out}");
        assert!(out.contains(&format!("{}:1: hit", scanned[0])), "{out}");
        assert!(
            out.contains(&format!("[skipped: timeout — {}]", scanned[0])),
            "{out}"
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn scan_deadline_returns_partial_results_with_marker() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                scan_deadline_ms: Some(0),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(&t, p("hit")).unwrap();
        assert!(out.contains("[truncated: scan deadline reached]"), "{out}");
        assert!(!out.contains("a.txt:1: hit"), "{out}");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_with_operation_aborted_on_signal() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
        let flag = AtomicBool::new(true);
        let aborted = || flag.load(Ordering::SeqCst);
        let e = execute_grep(&tool(&root), &p("hit"), Some(&aborted)).expect_err("abort");
        assert_eq!(e.to_string(), "Operation aborted");
        stdfs::remove_dir_all(&dir).unwrap();
    }

    // ---- output bounds ----

    #[test]
    fn truncates_long_displayed_line() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        stdfs::write(dir.join("long.txt"), format!("hit{}\n", "a".repeat(100))).unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                output_line_max_chars: Some(10),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "hit",
                glob: Some("long.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            out,
            format!("long.txt:1: hit{}... [truncated]", "a".repeat(7))
        );
        stdfs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn stops_output_at_output_max_bytes() {
        let dir = tmpdir("cheaper-grep-");
        let root = root_of(&dir);
        let lines: Vec<String> = (0..30)
            .map(|i| format!("match-line-{i}-xxxxxxxx"))
            .collect();
        stdfs::write(dir.join("o.txt"), format!("{}\n", lines.join("\n"))).unwrap();
        let t = create_grep_tool(opts(
            &root,
            GrepToolOptions {
                output_max_bytes: Some(100),
                ..default_options()
            },
        ))
        .unwrap();
        let out = run(
            &t,
            GrepParams {
                pattern: "match",
                glob: Some("o.txt"),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(out.contains("[truncated: output limit reached]"), "{out}");
        let body = out.split("\n[truncated").next().unwrap_or("");
        assert!(body.len() <= 100, "{body}");
        stdfs::remove_dir_all(&dir).unwrap();
    }
}
