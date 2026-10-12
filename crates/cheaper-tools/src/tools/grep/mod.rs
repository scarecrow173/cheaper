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

use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::Value;

use crate::access::AccessObserver;
use crate::catalog::{self, CanonicalToolResult};
use crate::limits::{GREP_BATCH_MAX_BYTES, GREP_MAX_LINE_BYTES};
use crate::operation::AbortSignal;
use crate::ToolError;
use cheaper_core::js16::slice_utf16;

use super::grep_eval::{AbortCheck, EvalOptions, EVAL_REPLY_MAX_BYTES};

mod execute;
mod scan;
#[cfg(test)]
mod tests;

pub use execute::execute_grep;
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
