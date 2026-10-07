//! Port of Pi `createEditToolDefinition`(`dist/core/tools/edit.js`) +
//! `edit-diff.js` on canonical `EditOperations`(`operations.ts`)。
//!
//! 全 edit は normalized(LF) content に対して元の位置で match し、
//! 逆順適用で offset を保つ。BOM は strip して最後に復元、line ending
//! は元の CRLF/LF を検出して復元する。fuzzy match(NFKC + trailing
//! whitespace + smart quote/dash/space 正規化)が使われた場合は
//! normalized-space で位置を求め、行単位 overlay で元の byte を保持する。
//! diff(`generateDiffString`)と unified patch(`generateUnifiedPatch` =
//! diff@8 `createTwoFilesPatch` FILE_HEADERS_ONLY, context 4)は
//! `similar` RawMyers + jsdiff 互換 tokenizer で再現する。

use std::sync::Arc;

use cheaper_core::nodepath;
use cheaper_core::tools::vocabulary::ToolName;
use serde_json::{json, Map, Value};
use similar::algorithms::{diff_slices, DiffHook};
use similar::Algorithm;
use unicode_normalization::UnicodeNormalization;

use crate::catalog::{self, CanonicalToolResult, ToolContent};
use crate::definition::{define_canonical_tool, CanonicalToolDefinition};
use crate::journal::tokenize_lines;
use crate::operation::AbortSignal;
use crate::tools::boundary::{mutation_queue_key, with_file_mutation_queue, EditOperations};
use crate::ToolError;

const ABORTED_MESSAGE: &str = "Operation aborted";
/// `generateUnifiedPatch`/`generateDiffString` の contextLines 既定値。
const CONTEXT_LINES: usize = 4;

/// `EditOperations` の tool struct。
pub struct EditTool<'a> {
    ops: EditOperations<'a>,
}

/// `createEditTool`。
pub fn create_edit_tool(ops: EditOperations<'_>) -> EditTool<'_> {
    EditTool { ops }
}

fn err(msg: impl Into<String>) -> ToolError {
    std::io::Error::other(msg.into()).into()
}

fn throw_if_aborted(signal: Option<&AbortSignal>) -> Result<(), ToolError> {
    if signal.is_some_and(AbortSignal::is_aborted) {
        return Err(err(ABORTED_MESSAGE));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// prepareArguments(`prepareEditArguments`)
// ---------------------------------------------------------------------------

/// `isSingleEditInput`: `{oldText: string, newText: string}` の object。
fn is_single_edit_input(v: &Value) -> bool {
    v.is_object() && v["oldText"].is_string() && v["newText"].is_string()
}

/// `prepareEditArguments`: `edits` の JSON string 形 / 単一 edit object 形 /
/// legacy トップレベル `oldText`/`newText` を吸収する。
pub fn prepare_edit_arguments(input: &Value) -> Value {
    let Value::Object(obj) = input else {
        return input.clone();
    };
    let mut args = obj.clone();
    // `edits` が string → JSON.parse して array なら採用、単一 edit object
    // なら wrap。parse 失敗はそのまま(schema 検証で落ちる)。
    if let Some(edits_str) = args.get("edits").and_then(Value::as_str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(edits_str) {
            if parsed.is_array() {
                args.insert("edits".to_string(), parsed);
            } else if is_single_edit_input(&parsed) {
                args.insert("edits".to_string(), json!([parsed]));
            }
        }
    } else if args.get("edits").is_some_and(is_single_edit_input) {
        let single = args["edits"].clone();
        args.insert("edits".to_string(), json!([single]));
    }
    // legacy `oldText`/`newText` top-level。
    let legacy = args.get("oldText").is_some_and(Value::is_string)
        && args.get("newText").is_some_and(Value::is_string);
    if !legacy {
        return Value::Object(args);
    }
    let mut edits = match args.get("edits") {
        Some(Value::Array(arr)) => arr.clone(),
        _ => Vec::new(),
    };
    edits.push(json!({
        "oldText": args["oldText"].clone(),
        "newText": args["newText"].clone(),
    }));
    args.remove("oldText");
    args.remove("newText");
    args.insert("edits".to_string(), Value::Array(edits));
    Value::Object(args)
}

/// `validateEditInput`: `edits` は非空 array(schema は empty array を
/// 通すため runtime 側で再度確認する)。
fn validate_edit_input(input: &Value) -> Result<(&str, &Vec<Value>), ToolError> {
    let edits = input["edits"].as_array();
    match edits {
        Some(e) if !e.is_empty() => Ok((input["path"].as_str().unwrap_or_default(), e)),
        _ => Err(err(
            "Edit tool input is invalid. edits must contain at least one replacement.",
        )),
    }
}

// ---------------------------------------------------------------------------
// edit-diff.js の port
// ---------------------------------------------------------------------------

/// `splitBom`(utils/text.js): 先頭 BOM(`\u{FEFF}`)を分離する。
fn split_bom(text: &str) -> (String, String) {
    if let Some(rest) = text.strip_prefix('\u{FEFF}') {
        ("\u{FEFF}".to_string(), rest.to_string())
    } else {
        (String::new(), text.to_string())
    }
}

/// `detectLineEnding`: 最初の newline が CRLF なら `"\r\n"`。
fn detect_line_ending(content: &str) -> &'static str {
    let crlf_idx = content.find("\r\n");
    let lf_idx = content.find('\n');
    match (crlf_idx, lf_idx) {
        (None, _) => "\n",
        (_, None) => "\n",
        (Some(c), Some(l)) if c < l => "\r\n",
        _ => "\n",
    }
}

/// `normalizeToLF`。
fn normalize_to_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// `restoreLineEndings`。
fn restore_line_endings(text: &str, ending: &str) -> String {
    if ending == "\r\n" {
        text.replace('\n', "\r\n")
    } else {
        text.to_string()
    }
}

/// `normalizeForFuzzyMatch`: NFKC → 行末 whitespace strip → smart
/// quote/dash/space の ASCII 正規化。
fn normalize_for_fuzzy_match(text: &str) -> String {
    let nfkc: String = text.nfkc().collect();
    nfkc.split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        // Smart single quotes → '
        .replace(['\u{2018}', '\u{2019}', '\u{201A}', '\u{201B}'], "'")
        // Smart double quotes → "
        .replace(['\u{201C}', '\u{201D}', '\u{201E}', '\u{201F}'], "\"")
        // U+2010..=U+2015, U+2212 → -
        .replace(
            [
                '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2015}', '\u{2212}',
            ],
            "-",
        )
        // U+00A0, U+2002..=U+200A, U+202F, U+205F, U+3000 → ' '
        .replace(
            [
                '\u{00A0}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}',
                '\u{2008}', '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}',
            ],
            " ",
        )
}

/// `splitLinesWithEndings`(`/[^\n]*\n|[^\n]+/g` 相当)。
fn split_lines_with_endings(content: &str) -> Vec<String> {
    if content.is_empty() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for part in content.split_inclusive('\n') {
        lines.push(part.to_string());
    }
    lines
}

/// `getLineSpans`。
#[derive(Debug, Clone, Copy)]
struct LineSpan {
    start: usize,
    end: usize,
}

fn get_line_spans(content: &str) -> Vec<LineSpan> {
    let mut offset = 0usize;
    split_lines_with_endings(content)
        .iter()
        .map(|line| {
            let span = LineSpan {
                start: offset,
                end: offset + line.len(),
            };
            offset = span.end;
            span
        })
        .collect()
}

/// `MatchedEdit`(edit-diff.js の `MatchedEdit`)。
#[derive(Debug, Clone)]
struct MatchedEdit {
    edit_index: usize,
    match_index: usize,
    match_length: usize,
    new_text: String,
}

/// `getReplacementLineRange`。
fn get_replacement_line_range(
    lines: &[LineSpan],
    replacement: &MatchedEdit,
) -> Result<(usize, usize), ToolError> {
    let replacement_start = replacement.match_index;
    let replacement_end = replacement.match_index + replacement.match_length;
    let mut start_line = None;
    for (i, line) in lines.iter().enumerate() {
        if replacement_start >= line.start && replacement_start < line.end {
            start_line = Some(i);
            break;
        }
    }
    let Some(start) = start_line else {
        return Err(err("Replacement range is outside the base content."));
    };
    let mut end = start;
    while end < lines.len() && lines[end].end < replacement_end {
        end += 1;
    }
    if end >= lines.len() {
        return Err(err("Replacement range is outside the base content."));
    }
    Ok((start, end + 1))
}

/// `applyReplacements`: 逆順適用で元 offset を保つ。
fn apply_replacements(content: &str, replacements: &[MatchedEdit], offset: usize) -> String {
    let mut result = content.to_string();
    for replacement in replacements.iter().rev() {
        let match_index = replacement.match_index - offset;
        result.replace_range(
            match_index..match_index + replacement.match_length,
            &replacement.new_text,
        );
    }
    result
}

/// `applyReplacementsPreservingUnchangedLines`。
fn apply_replacements_preserving_unchanged_lines(
    original_content: &str,
    base_content: &str,
    replacements: &[MatchedEdit],
) -> Result<String, ToolError> {
    let original_lines = split_lines_with_endings(original_content);
    let base_lines = get_line_spans(base_content);
    if original_lines.len() != base_lines.len() {
        return Err(err(
            "Cannot preserve unchanged lines because the base content has a different line count.",
        ));
    }
    struct Group {
        start_line: usize,
        end_line: usize,
        replacements: Vec<MatchedEdit>,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut sorted: Vec<&MatchedEdit> = replacements.iter().collect();
    sorted.sort_by_key(|r| r.match_index);
    for replacement in sorted {
        let (start_line, end_line) = get_replacement_line_range(&base_lines, replacement)?;
        if let Some(current) = groups.last_mut() {
            if start_line < current.end_line {
                current.end_line = current.end_line.max(end_line);
                current.replacements.push(replacement.clone());
                continue;
            }
        }
        groups.push(Group {
            start_line,
            end_line,
            replacements: vec![replacement.clone()],
        });
    }
    let mut original_line_index = 0usize;
    let mut result = String::new();
    for group in &groups {
        result.push_str(&original_lines[original_line_index..group.start_line].concat());
        let group_start_offset = base_lines[group.start_line].start;
        let group_end_offset = base_lines[group.end_line - 1].end;
        result.push_str(&apply_replacements(
            &base_content[group_start_offset..group_end_offset],
            &group.replacements,
            group_start_offset,
        ));
        original_line_index = group.end_line;
    }
    result.push_str(&original_lines[original_line_index..].concat());
    Ok(result)
}

/// `fuzzyFindText` の戻り値。TS の `contentForReplacement` は caller 側で
/// `replacementBaseContent` を別途計算するため実使用されず、ここでは省く。
struct FuzzyMatch {
    found: bool,
    index: usize,
    match_length: usize,
    used_fuzzy_match: bool,
}

/// `fuzzyFindText`。`indexOf` は byte index(Rust 内で offset 整合が
/// 取れれば UTF-16 index との差異は外部から観測されない)。
fn fuzzy_find_text(content: &str, old_text: &str) -> FuzzyMatch {
    if let Some(exact_index) = content.find(old_text) {
        return FuzzyMatch {
            found: true,
            index: exact_index,
            match_length: old_text.len(),
            used_fuzzy_match: false,
        };
    }
    let fuzzy_content = normalize_for_fuzzy_match(content);
    let fuzzy_old_text = normalize_for_fuzzy_match(old_text);
    match fuzzy_content.find(&fuzzy_old_text) {
        None => FuzzyMatch {
            found: false,
            index: 0,
            match_length: 0,
            used_fuzzy_match: false,
        },
        Some(fuzzy_index) => FuzzyMatch {
            found: true,
            index: fuzzy_index,
            match_length: fuzzy_old_text.len(),
            used_fuzzy_match: true,
        },
    }
}

/// `countOccurrences`(non-overlapping、`split().length - 1` 相当)。
/// 空 needle の `split("")` は JS で UTF-16 unit 数+1 の array を返す
/// (astral char は 2 unit)ため、UTF-16 unit 数で再現する。
fn count_occurrences(content: &str, old_text: &str) -> usize {
    let fuzzy_content = normalize_for_fuzzy_match(content);
    let fuzzy_old_text = normalize_for_fuzzy_match(old_text);
    if fuzzy_old_text.is_empty() {
        return crate::js16::utf16_len(&fuzzy_content).saturating_sub(1);
    }
    fuzzy_content.matches(&fuzzy_old_text).count()
}

fn get_not_found_error(path: &str, edit_index: usize, total_edits: usize) -> ToolError {
    if total_edits == 1 {
        err(format!(
            "Could not find the exact text in {path}. The old text must match exactly including all whitespace and newlines."
        ))
    } else {
        err(format!(
            "Could not find edits[{edit_index}] in {path}. The oldText must match exactly including all whitespace and newlines."
        ))
    }
}

fn get_duplicate_error(
    path: &str,
    edit_index: usize,
    total_edits: usize,
    occurrences: usize,
) -> ToolError {
    if total_edits == 1 {
        err(format!(
            "Found {occurrences} occurrences of the text in {path}. The text must be unique. Please provide more context to make it unique."
        ))
    } else {
        err(format!(
            "Found {occurrences} occurrences of edits[{edit_index}] in {path}. Each oldText must be unique. Please provide more context to make it unique."
        ))
    }
}

fn get_empty_old_text_error(path: &str, edit_index: usize, total_edits: usize) -> ToolError {
    if total_edits == 1 {
        err(format!("oldText must not be empty in {path}."))
    } else {
        err(format!(
            "edits[{edit_index}].oldText must not be empty in {path}."
        ))
    }
}

fn get_no_change_error(path: &str, total_edits: usize) -> ToolError {
    if total_edits == 1 {
        err(format!(
            "No changes made to {path}. The replacement produced identical content. This might indicate an issue with special characters or the text not existing as expected."
        ))
    } else {
        err(format!(
            "No changes made to {path}. The replacements produced identical content."
        ))
    }
}

/// `applyEditsToNormalizedContent`。
/// 戻り値 `(base_content, new_content)`。`base_content` は常に元の
/// normalized content(diff の before 側)。
fn apply_edits_to_normalized_content(
    normalized_content: &str,
    edits: &[Value],
    path: &str,
) -> Result<(String, String), ToolError> {
    let normalized_edits: Vec<(String, String)> = edits
        .iter()
        .map(|edit| {
            (
                normalize_to_lf(edit["oldText"].as_str().unwrap_or_default()),
                normalize_to_lf(edit["newText"].as_str().unwrap_or_default()),
            )
        })
        .collect();
    for (i, (old_text, _)) in normalized_edits.iter().enumerate() {
        if old_text.is_empty() {
            return Err(get_empty_old_text_error(path, i, normalized_edits.len()));
        }
    }
    let initial_matches: Vec<FuzzyMatch> = normalized_edits
        .iter()
        .map(|(old_text, _)| fuzzy_find_text(normalized_content, old_text))
        .collect();
    let used_fuzzy_match = initial_matches.iter().any(|m| m.used_fuzzy_match);
    let replacement_base_content = if used_fuzzy_match {
        normalize_for_fuzzy_match(normalized_content)
    } else {
        normalized_content.to_string()
    };
    let mut matched_edits: Vec<MatchedEdit> = Vec::new();
    for (i, (old_text, new_text)) in normalized_edits.iter().enumerate() {
        let match_result = fuzzy_find_text(&replacement_base_content, old_text);
        if !match_result.found {
            return Err(get_not_found_error(path, i, normalized_edits.len()));
        }
        let occurrences = count_occurrences(&replacement_base_content, old_text);
        if occurrences > 1 {
            return Err(get_duplicate_error(
                path,
                i,
                normalized_edits.len(),
                occurrences,
            ));
        }
        matched_edits.push(MatchedEdit {
            edit_index: i,
            match_index: match_result.index,
            match_length: match_result.match_length,
            new_text: new_text.clone(),
        });
    }
    matched_edits.sort_by_key(|e| e.match_index);
    for i in 1..matched_edits.len() {
        let previous = &matched_edits[i - 1];
        let current = &matched_edits[i];
        if previous.match_index + previous.match_length > current.match_index {
            return Err(err(format!(
                "edits[{}] and edits[{}] overlap in {path}. Merge them into one edit or target disjoint regions.",
                previous.edit_index, current.edit_index,
            )));
        }
    }
    let base_content = normalized_content.to_string();
    let new_content = if used_fuzzy_match {
        apply_replacements_preserving_unchanged_lines(
            normalized_content,
            &replacement_base_content,
            &matched_edits,
        )?
    } else {
        apply_replacements(&replacement_base_content, &matched_edits, 0)
    };
    if base_content == new_content {
        return Err(get_no_change_error(path, normalized_edits.len()));
    }
    Ok((base_content, new_content))
}

// ---------------------------------------------------------------------------
// diff 生成(diff@8 `diffLines` + `structuredPatch`/`formatPatch`)
// ---------------------------------------------------------------------------

/// `diffLines` の part 相当。`lines` は terminator 付き token(最終 token
/// は newline 無しになり得る)。
struct DiffPart {
    added: bool,
    removed: bool,
    lines: Vec<String>,
}

/// `diffLines` を `similar` RawMyers + jsdiff 互換 tokenizer で再現する。
/// `DiffHook` の contiguous run がそのまま part になる。
fn diff_lines(old: &str, new: &str) -> Vec<DiffPart> {
    let old_tokens = tokenize_lines(old);
    let new_tokens = tokenize_lines(new);

    struct Collect<'a> {
        old: &'a [&'a str],
        new: &'a [&'a str],
        parts: Vec<DiffPart>,
    }
    impl DiffHook for Collect<'_> {
        type Error = std::convert::Infallible;

        fn equal(
            &mut self,
            old_index: usize,
            _new_index: usize,
            len: usize,
        ) -> Result<(), Self::Error> {
            self.parts.push(DiffPart {
                added: false,
                removed: false,
                lines: self.old[old_index..old_index + len]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            });
            Ok(())
        }
        fn delete(
            &mut self,
            old_index: usize,
            old_len: usize,
            _new_index: usize,
        ) -> Result<(), Self::Error> {
            self.parts.push(DiffPart {
                added: false,
                removed: true,
                lines: self.old[old_index..old_index + old_len]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            });
            Ok(())
        }
        fn insert(
            &mut self,
            _old_index: usize,
            new_index: usize,
            new_len: usize,
        ) -> Result<(), Self::Error> {
            self.parts.push(DiffPart {
                added: true,
                removed: false,
                lines: self.new[new_index..new_index + new_len]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            });
            Ok(())
        }
        // `replace` は default(delete + insert)。
    }

    let mut collect = Collect {
        old: &old_tokens,
        new: &new_tokens,
        parts: Vec::new(),
    };
    match diff_slices(Algorithm::RawMyers, &mut collect, &old_tokens, &new_tokens) {
        Ok(()) => {}
        Err(e) => match e {},
    }
    // jsdiff は replacement を removed → added の順で emit するが、
    // similar の Myers hook 呼び出し順は insert → delete になり得る。
    // 隣接した added→removed 対を removed→added に正規化して jsdiff の
    // part 順と揃える。
    let mut parts = collect.parts;
    let mut i = 0usize;
    while i + 1 < parts.len() {
        if parts[i].added && parts[i + 1].removed {
            parts.swap(i, i + 1);
            i += 1;
        }
        i += 1;
    }
    parts
}

/// `part.value.split("\n")` + 末尾空 drop 相当: token 末尾の `\n` を除く。
fn part_raw_lines(part: &DiffPart) -> Vec<String> {
    part.lines
        .iter()
        .map(|l| l.strip_suffix('\n').unwrap_or(l).to_string())
        .collect()
}

/// `generateDiffString` の戻り値。
pub struct DiffStringResult {
    pub diff: String,
    /// `firstChangedLine`(new file 側、1-based)。変更無しは `None`。
    pub first_changed_line: Option<usize>,
}

/// `generateDiffString`(contextLines = 4)。
pub fn generate_diff_string(old_content: &str, new_content: &str) -> DiffStringResult {
    generate_diff_string_ctx(old_content, new_content, CONTEXT_LINES)
}

fn generate_diff_string_ctx(
    old_content: &str,
    new_content: &str,
    context_lines: usize,
) -> DiffStringResult {
    let parts = diff_lines(old_content, new_content);
    let mut output: Vec<String> = Vec::new();
    let old_lines: Vec<&str> = old_content.split('\n').collect();
    let new_lines: Vec<&str> = new_content.split('\n').collect();
    let max_line_num = old_lines.len().max(new_lines.len());
    let line_num_width = max_line_num.to_string().len();
    let pad = |n: usize| format!("{n:>width$}", width = line_num_width);
    let mut old_line_num = 1usize;
    let mut new_line_num = 1usize;
    let mut last_was_change = false;
    let mut first_changed_line: Option<usize> = None;

    for (i, part) in parts.iter().enumerate() {
        let raw = part_raw_lines(part);
        if part.added || part.removed {
            if first_changed_line.is_none() {
                first_changed_line = Some(new_line_num);
            }
            for line in &raw {
                if part.added {
                    output.push(format!("+{} {line}", pad(new_line_num)));
                    new_line_num += 1;
                } else {
                    output.push(format!("-{} {line}", pad(old_line_num)));
                    old_line_num += 1;
                }
            }
            last_was_change = true;
        } else {
            let next_part_is_change = parts.get(i + 1).is_some_and(|p| p.added || p.removed);
            let has_leading_change = last_was_change;
            let has_trailing_change = next_part_is_change;
            if has_leading_change && has_trailing_change {
                if raw.len() <= context_lines * 2 {
                    for line in &raw {
                        output.push(format!(" {} {line}", pad(old_line_num)));
                        old_line_num += 1;
                        new_line_num += 1;
                    }
                } else {
                    let leading = &raw[..context_lines];
                    let trailing = &raw[raw.len() - context_lines..];
                    let skipped = raw.len() - leading.len() - trailing.len();
                    for line in leading {
                        output.push(format!(" {} {line}", pad(old_line_num)));
                        old_line_num += 1;
                        new_line_num += 1;
                    }
                    output.push(format!(" {} ...", " ".repeat(line_num_width)));
                    old_line_num += skipped;
                    new_line_num += skipped;
                    for line in trailing {
                        output.push(format!(" {} {line}", pad(old_line_num)));
                        old_line_num += 1;
                        new_line_num += 1;
                    }
                }
            } else if has_leading_change {
                let shown = &raw[..context_lines.min(raw.len())];
                let skipped = raw.len() - shown.len();
                for line in shown {
                    output.push(format!(" {} {line}", pad(old_line_num)));
                    old_line_num += 1;
                    new_line_num += 1;
                }
                if skipped > 0 {
                    output.push(format!(" {} ...", " ".repeat(line_num_width)));
                    old_line_num += skipped;
                    new_line_num += skipped;
                }
            } else if has_trailing_change {
                let skipped = raw.len().saturating_sub(context_lines);
                if skipped > 0 {
                    output.push(format!(" {} ...", " ".repeat(line_num_width)));
                    old_line_num += skipped;
                    new_line_num += skipped;
                }
                for line in &raw[skipped..] {
                    output.push(format!(" {} {line}", pad(old_line_num)));
                    old_line_num += 1;
                    new_line_num += 1;
                }
            } else {
                old_line_num += raw.len();
                new_line_num += raw.len();
            }
            last_was_change = false;
        }
    }
    DiffStringResult {
        diff: output.join("\n"),
        first_changed_line,
    }
}

/// `generateUnifiedPatch` = diff@8 `structuredPatch` + `formatPatch`
/// (`FILE_HEADERS_ONLY`, context 4)。
pub fn generate_unified_patch(path: &str, old_content: &str, new_content: &str) -> String {
    generate_unified_patch_ctx(path, old_content, new_content, CONTEXT_LINES)
}

fn generate_unified_patch_ctx(
    path: &str,
    old_content: &str,
    new_content: &str,
    context: usize,
) -> String {
    let mut diff = diff_lines(old_content, new_content);
    // `diff.push({value: '', lines: []})` sentinel。
    diff.push(DiffPart {
        added: false,
        removed: false,
        lines: Vec::new(),
    });
    struct Hunk {
        old_start: i64,
        old_lines: i64,
        new_start: i64,
        new_lines: i64,
        lines: Vec<String>,
    }
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut old_range_start = 0i64;
    let mut new_range_start = 0i64;
    let mut cur_range: Vec<String> = Vec::new();
    let mut old_line = 1i64;
    let mut new_line = 1i64;
    let n = diff.len();
    for i in 0..n {
        let current = &diff[i];
        let lines = &current.lines; // terminator 付き token
        if current.added || current.removed {
            if old_range_start == 0 {
                let prev = i.checked_sub(1).map(|p| &diff[p]);
                old_range_start = old_line;
                new_range_start = new_line;
                if let Some(prev) = prev {
                    let ctx = if context > 0 {
                        let start = prev.lines.len().saturating_sub(context);
                        prev.lines[start..]
                            .iter()
                            .map(|l| format!(" {l}"))
                            .collect::<Vec<String>>()
                    } else {
                        Vec::new()
                    };
                    old_range_start -= ctx.len() as i64;
                    new_range_start -= ctx.len() as i64;
                    cur_range = ctx;
                }
            }
            for line in lines {
                cur_range.push(format!("{}{line}", if current.added { '+' } else { '-' }));
            }
            if current.added {
                new_line += lines.len() as i64;
            } else {
                old_line += lines.len() as i64;
            }
        } else {
            if old_range_start != 0 {
                if lines.len() <= context * 2 && i + 2 < n {
                    // overlapping context — 次の変更まで丸ごと context として
                    // 連結する。
                    for line in lines {
                        cur_range.push(format!(" {line}"));
                    }
                } else {
                    let context_size = lines.len().min(context);
                    for line in &lines[..context_size] {
                        cur_range.push(format!(" {line}"));
                    }
                    hunks.push(Hunk {
                        old_start: old_range_start,
                        old_lines: old_line - old_range_start + context_size as i64,
                        new_start: new_range_start,
                        new_lines: new_line - new_range_start + context_size as i64,
                        lines: std::mem::take(&mut cur_range),
                    });
                    old_range_start = 0;
                    new_range_start = 0;
                }
            }
            old_line += lines.len() as i64;
            new_line += lines.len() as i64;
        }
    }
    // STEP 2: 各行の末尾 `\n` を除去し、無ければ `\ No newline at end of file` を挿入。
    for hunk in &mut hunks {
        let mut i = 0usize;
        while i < hunk.lines.len() {
            if hunk.lines[i].ends_with('\n') {
                hunk.lines[i].pop();
            } else {
                hunk.lines
                    .insert(i + 1, "\\ No newline at end of file".to_string());
                i += 1;
            }
            i += 1;
        }
    }
    // formatPatch(FILE_HEADERS_ONLY)。
    let mut ret: Vec<String> = Vec::new();
    ret.push(format!("--- {path}"));
    ret.push(format!("+++ {path}"));
    for hunk in &mut hunks {
        // Unified Diff quirk: chunk size 0 なら start を 1 減らす。
        if hunk.old_lines == 0 {
            hunk.old_start -= 1;
        }
        if hunk.new_lines == 0 {
            hunk.new_start -= 1;
        }
        ret.push(format!(
            "@@ -{},{} +{},{} @@",
            hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
        ));
        ret.extend(hunk.lines.iter().cloned());
    }
    let mut out = ret.join("\n");
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// tool execute
// ---------------------------------------------------------------------------

/// `access` 失敗の `Error code: ${code}` 形式用の Node errno 名近似。
/// `NodeIoError` は保持する `code` をそのまま使う(Node の `error.code`
/// 相当)。raw `io::Error` は `io::ErrorKind` → 代表的 POSIX code。
/// 未知は `String(error)` 側(`"{e}"`)に畳む。
fn error_code_string(e: &(dyn std::error::Error + 'static)) -> Option<String> {
    if let Some(node) = e.downcast_ref::<cheaper_core::errors::NodeIoError>() {
        return Some(node.code().to_string());
    }
    let io = e.downcast_ref::<std::io::Error>()?;
    let code = match io.kind() {
        std::io::ErrorKind::NotFound => "ENOENT",
        std::io::ErrorKind::PermissionDenied => "EACCES",
        std::io::ErrorKind::IsADirectory => "EISDIR",
        std::io::ErrorKind::NotADirectory => "ENOTDIR",
        std::io::ErrorKind::ReadOnlyFilesystem => "EROFS",
        _ => return io.raw_os_error().map(|errno| format!("errno {errno}")),
    };
    Some(code.to_string())
}

impl EditTool<'_> {
    /// `definition.execute` 相当(prepareArguments/validate は executor 側)。
    pub fn execute(
        &self,
        input: &Value,
        signal: Option<&AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        let (path, edits) = validate_edit_input(input)?;
        let absolute_path = nodepath::resolve(self.ops.root(), path);
        let queue_key = mutation_queue_key(self.ops.root(), &absolute_path);
        with_file_mutation_queue(&queue_key, || {
            // abort event listener で reject しない — 各 op settle 後の
            // `signal.aborted` 観測に揃える(Pi 本体のコメントと同じ意図)。
            throw_if_aborted(signal)?;
            // Check if file exists.
            if let Err(error) = self.ops.access(&absolute_path) {
                throw_if_aborted(signal)?;
                let error_message = error_code_string(error.as_ref())
                    .map(|code| format!("Error code: {code}"))
                    .unwrap_or_else(|| error.to_string());
                return Err(err(format!(
                    "Could not edit file: {path}. {error_message}."
                )));
            }
            throw_if_aborted(signal)?;
            // Read the file.
            let buffer = self.ops.read_file(&absolute_path)?;
            let raw_content = String::from_utf8_lossy(&buffer).into_owned();
            throw_if_aborted(signal)?;
            // BOM を strip して match(model は不可視 BOM を oldText に含めない)。
            let (bom, content) = split_bom(&raw_content);
            let original_ending = detect_line_ending(&content);
            let normalized_content = normalize_to_lf(&content);
            let (base_content, new_content) =
                apply_edits_to_normalized_content(&normalized_content, edits, path)?;
            throw_if_aborted(signal)?;
            let final_content = format!(
                "{bom}{}",
                restore_line_endings(&new_content, original_ending)
            );
            self.ops.write_file(&absolute_path, &final_content)?;
            throw_if_aborted(signal)?;
            let diff_result = generate_diff_string(&base_content, &new_content);
            let patch = generate_unified_patch(path, &base_content, &new_content);
            let mut details = Map::new();
            details.insert("diff".to_string(), json!(diff_result.diff));
            details.insert("patch".to_string(), json!(patch));
            if let Some(first) = diff_result.first_changed_line {
                details.insert("firstChangedLine".to_string(), json!(first));
            }
            Ok(CanonicalToolResult {
                content: vec![ToolContent::text(format!(
                    "Successfully replaced {} block(s) in {path}.",
                    edits.len()
                ))],
                details: Some(Value::Object(details)),
            })
        })
    }
}

/// `edit` tool の canonical definition。
pub fn edit_tool_definition(
    ops: EditOperations<'_>,
) -> Result<CanonicalToolDefinition<'_>, ToolError> {
    let tool = Arc::new(create_edit_tool(ops));
    Ok(define_canonical_tool(CanonicalToolDefinition {
        name: ToolName::Edit,
        label: "edit".to_string(),
        description: "Edit a single file using exact text replacement. Every edits[].oldText must match a unique, non-overlapping region of the original file. If two changes affect the same block or nearby lines, merge them into one edit instead of emitting overlapping edits. Do not include large unchanged regions just to connect distant changes.".to_string(),
        parameters: catalog::edit_schema()
            .as_object()
            .expect("object schema")
            .clone(),
        prepare_arguments: Some(std::sync::Arc::new(prepare_edit_arguments)),
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

    use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
    use serde_json::json;

    use super::*;
    use crate::access::ResourceScopeOptions;
    use crate::tools::boundary::{create_edit_operations, BoundaryOperationsOptions};

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cheaper-edit-{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).expect("tmpdir");
        dir
    }

    fn make_tool<'a>(root: &'a str) -> EditTool<'a> {
        let ops = create_edit_operations(BoundaryOperationsOptions {
            observe_access: None,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            journal: None,
            denylist: None,
            scope: ResourceScopeOptions::default(),
        })
        .unwrap();
        create_edit_tool(ops)
    }

    #[test]
    fn prepare_arguments_accepts_json_string_and_legacy_forms() {
        // JSON string edits array。
        let out = prepare_edit_arguments(&json!({
            "path": "f",
            "edits": "[{\"oldText\":\"a\",\"newText\":\"b\"}]",
        }));
        assert_eq!(out["edits"].as_array().unwrap().len(), 1);
        // 単一 edit object は wrap。
        let out = prepare_edit_arguments(&json!({
            "path": "f",
            "edits": {"oldText": "a", "newText": "b"},
        }));
        assert_eq!(out["edits"].as_array().unwrap().len(), 1);
        // legacy top-level。
        let out = prepare_edit_arguments(&json!({
            "path": "f",
            "oldText": "a",
            "newText": "b",
        }));
        assert_eq!(out["edits"].as_array().unwrap().len(), 1);
        assert!(out.get("oldText").is_none());
        // 既存 edits へ legacy を append。
        let out = prepare_edit_arguments(&json!({
            "path": "f",
            "edits": [{"oldText": "x", "newText": "y"}],
            "oldText": "a",
            "newText": "b",
        }));
        assert_eq!(out["edits"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn fs_edit_exact_replacement_success() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "hello world\nsecond\n").unwrap();
        let tool = make_tool(&root);
        let result = tool
            .execute(
                &json!({
                    "path": "f.txt",
                    "edits": [{"oldText": "world", "newText": "there"}],
                }),
                None,
            )
            .unwrap();
        assert_eq!(
            result.content[0].text,
            "Successfully replaced 1 block(s) in f.txt."
        );
        assert_eq!(
            fs::read_to_string(dir.join("f.txt")).unwrap(),
            "hello there\nsecond\n"
        );
        let details = result.details.unwrap();
        assert!(details["diff"].as_str().unwrap().contains("-1 hello world"));
        assert!(details["diff"].as_str().unwrap().contains("+1 hello there"));
        assert_eq!(details["firstChangedLine"], 1);
        let patch = details["patch"].as_str().unwrap();
        assert!(patch.starts_with("--- f.txt\n+++ f.txt\n"));
        assert!(patch.contains("@@ -1,"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_edit_disjoint_multiple_edits() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "a\nb\nc\n").unwrap();
        let tool = make_tool(&root);
        let result = tool
            .execute(
                &json!({
                    "path": "f.txt",
                    "edits": [
                        {"oldText": "c", "newText": "C"},
                        {"oldText": "a", "newText": "A"},
                    ],
                }),
                None,
            )
            .unwrap();
        assert_eq!(
            result.content[0].text,
            "Successfully replaced 2 block(s) in f.txt."
        );
        assert_eq!(fs::read_to_string(dir.join("f.txt")).unwrap(), "A\nb\nC\n");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_edit_unique_and_overlap_validation() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "foo bar foo\n").unwrap();
        let tool = make_tool(&root);
        // 2 occurrences → unique 違反。
        let err = tool
            .execute(
                &json!({"path": "f.txt", "edits": [{"oldText": "foo", "newText": "x"}]}),
                None,
            )
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Found 2 occurrences of the text in f.txt. The text must be unique. Please provide more context to make it unique."
        );
        // overlapping edits。
        let err = tool
            .execute(
                &json!({
                    "path": "f.txt",
                    "edits": [
                        {"oldText": "foo bar", "newText": "x"},
                        {"oldText": "bar foo", "newText": "y"},
                    ],
                }),
                None,
            )
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "edits[0] and edits[1] overlap in f.txt. Merge them into one edit or target disjoint regions."
        );
        // not found。
        let err = tool
            .execute(
                &json!({"path": "f.txt", "edits": [{"oldText": "zzz", "newText": "x"}]}),
                None,
            )
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Could not find the exact text in f.txt. The old text must match exactly including all whitespace and newlines."
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_edit_empty_edits_and_no_change() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "x\n").unwrap();
        let tool = make_tool(&root);
        let err = tool
            .execute(&json!({"path": "f.txt", "edits": []}), None)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Edit tool input is invalid. edits must contain at least one replacement."
        );
        let err = tool
            .execute(
                &json!({"path": "f.txt", "edits": [{"oldText": "x", "newText": "x"}]}),
                None,
            )
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "No changes made to f.txt. The replacement produced identical content. This might indicate an issue with special characters or the text not existing as expected."
        );
        let err = tool
            .execute(
                &json!({"path": "f.txt", "edits": [{"oldText": "", "newText": "x"}]}),
                None,
            )
            .unwrap_err();
        assert_eq!(err.to_string(), "oldText must not be empty in f.txt.");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_edit_preserves_crlf_and_bom() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        fs::write(dir.join("f.txt"), "\u{FEFF}a\r\nb\r\n").unwrap();
        let tool = make_tool(&root);
        tool.execute(
            &json!({
                "path": "f.txt",
                "edits": [{"oldText": "a\nb", "newText": "a\nB"}],
            }),
            None,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("f.txt")).unwrap(),
            "\u{FEFF}a\r\nB\r\n"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_edit_fuzzy_trailing_whitespace() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        // file 側は trailing space 付き、oldText は無し → fuzzy match。
        fs::write(dir.join("f.txt"), "hello   \nworld\n").unwrap();
        let tool = make_tool(&root);
        let result = tool
            .execute(
                &json!({
                    "path": "f.txt",
                    "edits": [{"oldText": "hello\nworld", "newText": "HELLO\nworld"}],
                }),
                None,
            )
            .unwrap();
        // 未変更行の byte を保持(行 overlay)。対象行のみ newText。
        assert_eq!(
            fs::read_to_string(dir.join("f.txt")).unwrap(),
            "HELLO\nworld\n"
        );
        assert!(result.details.is_some());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn fs_edit_missing_file_error_code() {
        let dir = tmpdir();
        let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
        let tool = make_tool(&root);
        let err = tool
            .execute(
                &json!({
                    "path": "missing.txt",
                    "edits": [{"oldText": "x", "newText": "y"}],
                }),
                None,
            )
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.starts_with("Could not edit file: missing.txt. "),
            "unexpected message: {msg}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unified_patch_golden() {
        let patch = generate_unified_patch("f.txt", "a\nb\nc\nd\ne\nf\n", "a\nb\nc\nD\ne\nf\n");
        assert_eq!(
            patch,
            "--- f.txt\n+++ f.txt\n@@ -1,6 +1,6 @@\n a\n b\n c\n-d\n+D\n e\n f\n"
        );
    }

    #[test]
    fn diff_string_skips_large_unchanged() {
        let old = (1..=20)
            .map(|i| format!("l{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let new = old.replace("l5", "L5").replace("l15", "L15");
        let d = generate_diff_string(&old, &new);
        assert_eq!(d.first_changed_line, Some(5));
        // 大きな unchanged block は " ... " で省略される。行番号は
        // 最大行数(20 → width 2)で右揃え pad される。
        assert!(d.diff.contains("  ..."));
        assert!(d.diff.contains("- 5 l5"));
        assert!(d.diff.contains("+ 5 L5"));
        assert!(d.diff.contains("-15 l15"));
        assert!(d.diff.contains("+15 L15"));
    }
}
