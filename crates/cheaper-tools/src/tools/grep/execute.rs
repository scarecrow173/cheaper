//! `execute_grep`: 入力解決・eval worker 起動・scan loop 呼び出し・
//! footer 組立の orchestration。

use std::cell::RefCell;

use cheaper_core::js16::utf16_len;

use crate::limits::{
    GREP_BATCH_MAX_BYTES, GREP_BATCH_MAX_LINES, GREP_EVAL_PER_FILE_MS, GREP_EVAL_TOTAL_MS,
    GREP_MAX_LINE_BYTES, GREP_OUTPUT_LINE_MAX_CHARS, GREP_OUTPUT_MAX_BYTES, GREP_PATTERN_MAX_CHARS,
    GREP_SCAN_DEADLINE_MS, IO_CHUNK_BYTES, SKIP_FOOTER_MAX_BYTES, SKIP_FOOTER_MAX_ENTRIES,
    SKIP_PATH_MAX_CHARS,
};
use crate::walker::WalkOptions;
use crate::ToolError;

use super::super::grep_eval::{AbortCheck, GrepEvalWorker};
use super::scan::Scan;
use super::*;

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
        let inputs = scan::ScanInputs {
            walk: WalkOptions {
                observe_access: opts.observe_access,
                root: opts.root,
                symlink_policy: opts.symlink_policy,
                denylist: opts.denylist,
            },
            start: start_abs,
            glob: glob_matcher,
            effective_pattern,
            flags,
            per_file_ms,
            total_ms,
            deadline_at,
            chunk_bytes,
            max_line_bytes,
            context,
            max_results,
            output_line_max_chars,
            footer_max_entries,
            path_max_chars,
            batch_max_lines,
            batch_max_bytes,
        };
        scan::scan_workspace(
            &inputs,
            &mut scan,
            &skipped,
            &mut evaluator,
            &tool.eval_opts,
            now,
            aborted,
        )?;
    }
    // evaluator は Drop で kill される(TS finally `evaluator.dispose()`)。

    Ok(assemble_body(
        scan.out_lines,
        scan.stop_reason,
        skipped.into_inner(),
        footer_max_bytes,
    ))
}

/// `[skipped: ...]`/`[truncated: ...]` footer を組み立て、本文と結合する
/// (TS の走査後 footer 組立相当)。
fn assemble_body(
    out_lines: Vec<String>,
    stop_reason: Option<StopReason>,
    skipped: Skipped,
    max_bytes: usize,
) -> String {
    // [skipped: ...] 行。1 entry ≤ pathMaxChars、skipped 部全体は
    // footerMaxBytes byte で締め、超過は [skipped-report: truncated] で示す。
    let mut footer: Vec<String> = Vec::new();
    let mut skipped_bytes = 0usize;
    let mut skipped_report_truncated = false;
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
        if skipped_bytes + need > max_bytes {
            skipped_report_truncated = true;
            break;
        }
        footer.push(line);
        skipped_bytes += need;
    }
    match stop_reason {
        Some(StopReason::Eval) => footer.push("[truncated: evaluation time limit reached]".into()),
        Some(StopReason::Deadline) => footer.push("[truncated: scan deadline reached]".into()),
        Some(StopReason::ResultCap) => footer.push("[truncated: more matches]".into()),
        Some(StopReason::Output) => footer.push("[truncated: output limit reached]".into()),
        None => {}
    }
    if skipped_report_truncated {
        footer.push("[skipped-report: truncated]".into());
    }

    let mut body = if out_lines.is_empty() {
        "(no matches)".to_string()
    } else {
        out_lines.join("\n")
    };
    if !footer.is_empty() {
        body.push('\n');
        body.push_str(&footer.join("\n"));
    }
    body
}
