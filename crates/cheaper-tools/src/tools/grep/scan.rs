//! 走査責務: file walk -> sniff -> chunk read -> line framer -> batch eval
//! (`execute_grep` から分離した scan loop と per-file 状態機械)。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io::Read;
use std::sync::mpsc::{channel, Receiver};

use cheaper_core::js16::{slice_utf16, utf16_len};
use cheaper_core::resources::types::ClaimMode;

use crate::line_framer::{create_line_framer, LineFramerOptions};
use crate::walker::{rethrow_as_denied, sniff_text_file, walk_files, SniffResult, WalkOptions};
use crate::ToolError;

use super::super::grep_eval::{
    AbortCheck, EvalError, EvalOptions, GrepEvalWorker, ABORTED_MESSAGE,
};
use super::*;

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
pub(super) struct Scan {
    pub(super) out_lines: Vec<String>,
    pub(super) out_bytes: usize,
    pub(super) matches: usize,
    pub(super) eval_total_ms: f64,
    pub(super) stop_reason: Option<StopReason>,
    pub(super) output_max_bytes: usize,
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

/// `scan_workspace` の入力束(execute 側で解決済みの limit/opts)。
pub(super) struct ScanInputs<'a> {
    /// workspace walk の filter(root/deny/symlink/observe)。
    pub walk: WalkOptions<'a>,
    /// walk 起点(`params.path.unwrap_or(".")`)。
    pub start: &'a str,
    /// glob filter(`params.glob`)。
    pub glob: Option<&'a str>,
    /// eval へ送る pattern(empty は `"(?:)"` 正規化済み)。
    pub effective_pattern: &'a str,
    /// regress flags(`"i"` or `""`)。
    pub flags: &'a str,
    /// per-file eval 予算 ms。
    pub per_file_ms: f64,
    /// total eval 予算 ms。
    pub total_ms: f64,
    /// scan deadline(absolute ms)。
    pub deadline_at: f64,
    /// read chunk byte 数。
    pub chunk_bytes: usize,
    /// 1 行の最大 byte(oversized skip 判定)。
    pub max_line_bytes: usize,
    /// context 行数。
    pub context: usize,
    /// match 上限。
    pub max_results: usize,
    /// 出力 1 行の最大 UTF-16 unit。
    pub output_line_max_chars: usize,
    /// footer へ出す skip entry 上限。
    pub footer_max_entries: usize,
    /// footer entry の path 最大 char。
    pub path_max_chars: usize,
    /// batch 締め行数。
    pub batch_max_lines: usize,
    /// batch 締め byte 数。
    pub batch_max_bytes: usize,
}

/// abort/deadline/total-eval 予算の検査(TS `checkScan`)。
fn check_scan(
    now: &(dyn Fn() -> f64 + Send + Sync),
    deadline_at: f64,
    aborted: Option<&AbortCheck<'_>>,
    stop_reason: &mut Option<StopReason>,
) -> Result<(), ToolError> {
    if aborted.is_some_and(|a| a()) {
        return Err(err(ABORTED_MESSAGE));
    }
    if now() >= deadline_at {
        stop_reason.get_or_insert(StopReason::Deadline);
    }
    Ok(())
}

/// workspace walk -> per-file scan の loop(TS `execute` 内の for-await
/// 相当)。`Scan`/`Skipped` を in-place で積み、evaluator は呼び出し側
/// 所有(Drop kill)を維持する。
pub(super) fn scan_workspace(
    inputs: &ScanInputs<'_>,
    scan: &mut Scan,
    skipped: &RefCell<Skipped>,
    evaluator: &mut GrepEvalWorker,
    eval_opts: &EvalOptions,
    now: &(dyn Fn() -> f64 + Send + Sync),
    aborted: Option<&AbortCheck<'_>>,
) -> Result<(), ToolError> {
    let mut on_issue = |issue: crate::walker::WalkIssue| {
        record_skip(
            &mut skipped.borrow_mut().unreadable,
            issue.display,
            inputs.footer_max_entries,
            inputs.path_max_chars,
        );
    };
    let walk = walk_files(&inputs.walk, inputs.start, Some(&mut on_issue))
        .map_err(|e| rethrow_as_denied(Box::new(e)))?;
    for file in walk {
        // for-await の列挙 error は checkScan より先に伝播する。
        let file = match file {
            Ok(f) => f,
            Err(e) => return Err(rethrow_as_denied(e)),
        };
        check_scan(now, inputs.deadline_at, aborted, &mut scan.stop_reason)?;
        if scan.stop_reason.is_some() {
            break;
        }
        if let Some(g) = inputs.glob {
            // find と同様、over-long pattern の TypeError は file 毎の
            // lazy 評価で propagate する。
            if !crate::glob::matches_glob(&file.display, g)? {
                continue;
            }
        }
        // TS は for-await 全体を try/catch(rethrowAsDenied)で包む。
        // walk 中の escape error は denied 3 message に写像される。
        match sniff_text_file(&file.real, inputs.walk.observe_access).map_err(rethrow_as_denied)? {
            SniffResult::Binary => {
                record_skip(
                    &mut skipped.borrow_mut().binary,
                    file.display,
                    inputs.footer_max_entries,
                    inputs.path_max_chars,
                );
                continue;
            }
            SniffResult::Unreadable => {
                record_skip(
                    &mut skipped.borrow_mut().unreadable,
                    file.display,
                    inputs.footer_max_entries,
                    inputs.path_max_chars,
                );
                continue;
            }
            SniffResult::Text => {}
        }
        if let Some(observe) = inputs.walk.observe_access {
            observe(&file.real, ClaimMode::Shared, None, false).map_err(rethrow_as_denied)?;
        }
        let mut fh = match fs::File::open(&file.real) {
            Ok(fh) => fh,
            Err(_) => {
                record_skip(
                    &mut skipped.borrow_mut().unreadable,
                    file.display,
                    inputs.footer_max_entries,
                    inputs.path_max_chars,
                );
                continue;
            }
        };

        // per-file 状態。batch は常に連続行番号を保つ(oversized 行は
        // 行番号を消費するためそこで batch を締め、0-indexed ↔ 絶対行番号
        // の写像を維持する)。
        let mut fs_can = FileScan::new(
            scan,
            skipped,
            evaluator,
            eval_opts,
            now,
            aborted,
            &file.display,
            inputs.effective_pattern,
            inputs.flags,
            (inputs.per_file_ms, inputs.total_ms, inputs.deadline_at),
            (
                inputs.context,
                inputs.max_results,
                inputs.output_line_max_chars,
                inputs.footer_max_entries,
                inputs.path_max_chars,
                inputs.batch_max_lines,
                inputs.batch_max_bytes,
            ),
        );

        // framer callback は `Send` 制約があるため channel 経由で
        // pending/skip へ流す(feed 完了時に全 event を回収する)。
        let (ev_tx, ev_rx) = channel::<FramerEvent>();
        let mut framer = {
            let tx = ev_tx.clone();
            let tx2 = ev_tx;
            create_line_framer(LineFramerOptions {
                max_line_bytes: inputs.max_line_bytes,
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
            let mut buf = vec![0u8; inputs.chunk_bytes];
            loop {
                check_scan(
                    now,
                    inputs.deadline_at,
                    aborted,
                    &mut fs_can.scan.stop_reason,
                )?;
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
                            inputs.footer_max_entries,
                            inputs.path_max_chars,
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
    Ok(())
}
