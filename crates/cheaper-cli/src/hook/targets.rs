//! Port of `packages/cli/src/hook/targets.ts` — `cheaper hook` の
//! target adapter (DESIGN-034z1EhuztlLSb2x2XXxgD)。
//!
//! 各 adapter は agent native の stdin payload を canonical routing
//! request へ正規化し、共有 policy の verdict を target の response
//! contract で render する。出力は `JSON.stringify` byte 互換が
//! contract (`cheaper_core::json::stringify` + `"\n"`)。

use std::path::PathBuf;

use cheaper_core::config::dirs::EnvLookup;
use cheaper_core::json::stringify::stringify;
use cheaper_core::nodepath;
use serde_json::{Map, Value};

use super::eval_cache;
use super::routing_policy::{
    EvaluateOptions, ReadRange, RoutingRequest, SearchOutputMode, Verdict,
};

/// `HookTarget` — `HOOK_TARGETS` の 6 値。
/// `Pretooluse` は claude 専用、`Codex` は codex 専用 — payload
/// shape は同一だが render contract が分かれる(C7 — discriminator
/// は `--target` 値)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookTarget {
    Windsurf,
    Copilot,
    Pretooluse,
    Cursor,
    Devin,
    Codex,
}

/// `HOOK_TARGETS` — argv の `--target` 値 → target。
impl HookTarget {
    pub const ALL: [Self; 6] = [
        Self::Windsurf,
        Self::Copilot,
        Self::Pretooluse,
        Self::Cursor,
        Self::Devin,
        Self::Codex,
    ];

    pub fn parse(s: &str) -> Option<HookTarget> {
        Self::ALL.iter().find(|t| t.as_str() == s).copied()
    }

    /// `--target` 値の canonical 表現(`cheaper.hook` span の `target`
    /// field 用 — `parse` が受理する値の逆写像)。
    pub fn as_str(&self) -> &'static str {
        match self {
            HookTarget::Windsurf => "windsurf",
            HookTarget::Copilot => "copilot",
            HookTarget::Pretooluse => "pretooluse",
            HookTarget::Cursor => "cursor",
            HookTarget::Devin => "devin",
            HookTarget::Codex => "codex",
        }
    }
}

/// `HookResponse` — stdout/stderr text は optional(無ければ書かない)。
pub struct HookResponse {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit: i32,
}

/// `HookDeps`。
#[derive(Clone, Copy)]
pub struct HookDeps<'a> {
    pub env: EnvLookup<'a>,
    /// `CliHooks.builtin_dir`(未指定なら計測時に embedded へ解決)。
    pub builtin_dir: Option<&'a PathBuf>,
    /// hook 呼出し側の cwd(payload に cwd が無い場合の fallback)。
    pub cwd: &'a str,
}

impl<'a> HookDeps<'a> {
    fn eval_opts(&self) -> EvaluateOptions<'a> {
        EvaluateOptions {
            env: self.env,
            builtin_dir: self.builtin_dir,
            // routes/command policy の供給は `evaluate_with_routes`
            // が firing 単位で注入する(C1)。
            route: None,
        }
    }
}

fn silent_allow() -> HookResponse {
    HookResponse {
        stdout: None,
        stderr: None,
        exit: 0,
    }
}

/// JS truthiness (`payload.tool_name || ...` の判定用): null/false/
/// 0/-0/NaN/"" が falsy、それ以外(object/array 含む)は truthy。
fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `{k: v, ...}` の object literal を insertion order のまま
/// `JSON.stringify` byte 互換で compact 化する。
fn stringify_ordered(pairs: &[(&str, Value)]) -> String {
    let mut map = Map::with_capacity(pairs.len());
    for (k, v) in pairs {
        map.insert((*k).to_string(), v.clone());
    }
    stringify(&Value::Object(map))
}

pub fn targets_json_line() -> String {
    let targets: Vec<Value> = HookTarget::ALL
        .iter()
        .map(|t| Value::from(t.as_str()))
        .collect();
    format!(
        "{}\n",
        stringify_ordered(&[
            ("schemaVersion", Value::from(1)),
            ("targets", Value::Array(targets)),
            // `cheaper hook-config` 所有境界の capability signal —
            // installer/lifecycle wrapper はこの field で対応 CLI を
            // 判別し、無い(旧 CLI)場合は silent 成功とみなさない。
            ("hookConfig", Value::from(true)),
        ])
    )
}

/// `reason.split("\n").join(" ")` — multi-line guidance を 1 行へ。
fn flatten_reason(reason: &str) -> String {
    reason.split('\n').collect::<Vec<_>>().join(" ")
}

// --- windsurf: deny = stderr guidance + exit 2, allow = silent exit 0 ---

fn windsurf_expected_event(event: &str) -> Option<&'static str> {
    match event {
        "pre-read-code" => Some("pre_read_code"),
        "pre-run-command" => Some("pre_run_command"),
        _ => None,
    }
}

fn windsurf_normalize(event: &str, input: &Value, cwd: &str) -> Option<RoutingRequest> {
    let expected = windsurf_expected_event(event)?;
    let payload = input.as_object()?;
    if payload.get("agent_action_name").and_then(Value::as_str) != Some(expected) {
        return None;
    }
    let tool_info = payload.get("tool_info").and_then(Value::as_object);
    if expected == "pre_read_code" {
        // `pattern` 非空 string の search 形 payload は kind 判定を
        // 最優先する — ただし `file_path` が file を指す併記形は
        // FileRead(全文 eval、「全文 read が最悪ケース」)を優先し、
        // 欠落・空文字・非 string・directory 解決の場合に限り
        // `Search` へ倒す 2 条件 gate(issue #161)。空 `pattern` の
        // fail-open・一致行 ⊆ file 内容実体化による過少評価を
        // normalize 段で塞ぐ。
        if let Some(t) = tool_info {
            if t.get("pattern")
                .and_then(Value::as_str)
                .is_some_and(|p| !p.is_empty())
                && windsurf_pattern_targets_search(t, cwd)
            {
                return normalize_search(t, cwd, Some("windsurf:pre_read_code".to_string()));
            }
        }
        let file_path = tool_info
            .and_then(|t| t.get("file_path"))
            .and_then(Value::as_str)?;
        if file_path.is_empty() {
            return None;
        }
        return Some(RoutingRequest::FileRead {
            path: file_path.to_string(),
            cwd: cwd.to_string(),
            source_tool: Some("windsurf:pre_read_code".to_string()),
            range: tool_info.and_then(windsurf_read_range),
        });
    }
    let command = tool_info
        .and_then(|t| t.get("command_line"))
        .and_then(Value::as_str)?;
    if command.is_empty() {
        return None;
    }
    Some(RoutingRequest::Shell {
        command: command.to_string(),
        cwd: cwd.to_string(),
        source_tool: Some("windsurf:pre_run_command".to_string()),
    })
}

/// `pattern` 併記 `pre_read_code` payload が `Search` 経路を取るかの
/// 判定 — `file_path` が欠落・空文字・非 string、または cwd 相対
/// resolve(`resolve_target` と同一の「resolve → metadata 分類」
/// 規約、normalize 層では workspace containment は見ない)先が
/// directory の場合のみ true。file 解決・metadata 失敗は FileRead
/// 優先(false)。normalize 層の fs 参照はこの metadata 1 回に
/// 限定される(呼出し側が `pattern` 非空を先に gate する)。
fn windsurf_pattern_targets_search(tool_info: &Map<String, Value>, cwd: &str) -> bool {
    match tool_info.get("file_path").and_then(Value::as_str) {
        // 欠落・非 string・空文字 → search root は `normalize_search`
        // の `path`/`file_path`/`"."` fallback 側で解決する。
        None => true,
        Some("") => true,
        Some(fp) => std::fs::metadata(nodepath::resolve(cwd, fp))
            .map(|m| m.is_dir())
            .unwrap_or(false),
    }
}

fn windsurf_render(verdict: Verdict) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => HookResponse {
            stdout: None,
            stderr: Some(format!("{reason}\n")),
            exit: 2,
        },
        Verdict::Allow => silent_allow(),
        // rewrite 非対応 target — fallback verdict を render する
        // (C6 — deny 素通し防止)。
        Verdict::Rewrite { fallback, .. } => windsurf_render(*fallback),
    }
}

// --- copilot: deny = permissionDecision JSON + exit 0, never nonzero ---

const COPILOT_READ_TOOLS: &[&str] = &["view"];
const COPILOT_SHELL_TOOLS: &[&str] = &["bash", "powershell", "shell"];

/// `normalizeToolArgs` — `toolArgs` は object または JSON string。
fn normalize_tool_args(tool_args: Option<&Value>) -> Option<Map<String, Value>> {
    let args = tool_args?;
    if let Some(s) = args.as_str() {
        match serde_json::from_str::<Value>(s) {
            Ok(v) => return v.as_object().cloned(),
            Err(_) => return None,
        }
    }
    args.as_object().cloned()
}

/// `pickString` — keys を順に見て最初の非空 string を返す。
fn pick_string<'a>(args: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(v) = args.get(*key).and_then(Value::as_str) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// keys を順に見て最初に存在する値を返す(型は問わない)。
fn pick_value<'a>(args: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|k| args.get(*k))
}

/// 行数系 integer field を `u64` へ正規化する。正の整数値のみ —
/// 非数・非整数・`0`・負値は `None`(= 無効値)。
fn json_line_number(v: &Value) -> Option<u64> {
    let n = v.as_f64()?;
    if n.fract() != 0.0 || n < 1.0 {
        return None;
    }
    Some(n as u64)
}

/// 非負 integer field(`context`/`-A`/`-B`)を `u64` へ。無効は `None`。
fn json_count(v: &Value) -> Option<u64> {
    let n = v.as_f64()?;
    if n.fract() != 0.0 || n < 0.0 {
        return None;
    }
    Some(n as u64)
}

/// `bool` field を JS truthiness ではなく bool-as-bool で読む
/// (hook payload の `-i`/`case_insensitive` は bool)。
fn json_bool(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
}

/// `ReadRange` 正規化 — `offset`/`limit` 系 key を読み、無効値は
/// `None`(全文評価へ縮退)。両方欠落でも `None`。
/// `offset` 欠落 + `limit` ありは `offset = 1` 扱い。
/// `offset == 1 && limit None` は全文と同値なので `None` へ畳む。
fn normalize_read_range(
    args: &Map<String, Value>,
    offset_keys: &[&str],
    limit_keys: &[&str],
) -> Option<ReadRange> {
    let offset_v = pick_value(args, offset_keys);
    let limit_v = pick_value(args, limit_keys);
    if offset_v.is_none() && limit_v.is_none() {
        return None;
    }
    let offset = match offset_v {
        None => 1,
        Some(v) => json_line_number(v)?,
    };
    let limit = match limit_v {
        None => None,
        Some(v) => Some(json_line_number(v)?),
    };
    if offset == 1 && limit.is_none() {
        return None;
    }
    Some(ReadRange { offset, limit })
}

/// `view_range`(`[startLine, endLine]` array、1-based・inclusive)の
/// 正規化結果を 3 値で表す内部表現 — windsurf caller が「矛盾
/// (helper 全体 `None` short-circuit)」と「不完全(次判定へ
/// fall-through)」を区別するための型。copilot caller は non-`Valid`
/// を全て `None` へ写像する(従来の `Option` 意味論と同値)。
enum ViewRange {
    /// `[start, end]`(`end >= start`)、または `[start, -1]` の
    /// "to EOF" sentinel から `ReadRange` へ写像できた。
    Valid(ReadRange),
    /// `end < start` の complete array — emitter の意図範囲より狭い
    /// 評価へ縮退させないため矛盾として扱う。
    Contradiction,
    /// array 非形成・`start`/`end` 受理不能(欠落・非整数・0・
    /// `-1` 以外の負値) — 後続判定へ進む。
    Incomplete,
}

/// `view_range`(`[startLine, endLine]`、1-based・inclusive)を
/// `ReadRange` へ正規化する — copilot `view`・windsurf
/// `pre_read_code` 共有の変換規約(旧 `copilot_view_range` の共有化)。
/// `end == -1` は "to EOF" sentinel として `{offset:start,
/// limit:None}` へ写像する(copilot wire では未観測だが同一規約と
/// して許容 — `to EOF` 解釈は通用する)。`end < start` は
/// `Contradiction`、それ以外の不受理は `Incomplete` を返す。
fn normalize_view_range(args: &Map<String, Value>) -> ViewRange {
    let Some(arr) = args.get("view_range").and_then(Value::as_array) else {
        return ViewRange::Incomplete;
    };
    let Some(start) = arr.first().and_then(json_line_number) else {
        return ViewRange::Incomplete;
    };
    let Some(end_v) = arr.get(1) else {
        return ViewRange::Incomplete;
    };
    // `end == -1` は `view_range` 系の "to EOF" sentinel 慣例値 —
    // `json_line_number` が負値を受理しないため先に raw 値で見る。
    if end_v.as_f64() == Some(-1.0) {
        return ViewRange::Valid(ReadRange {
            offset: start,
            limit: None,
        });
    }
    let Some(end) = json_line_number(end_v) else {
        return ViewRange::Incomplete;
    };
    if end < start {
        return ViewRange::Contradiction;
    }
    ViewRange::Valid(ReadRange {
        offset: start,
        limit: Some(end - start + 1),
    })
}

/// windsurf `pre_read_code` `tool_info` の `ReadRange` 抽出 —
/// 共有 `tool_use_normalize`(`offset`/`line_offset`/`start_line` +
/// `limit`/`line_limit`/`max_lines` の key 集合)と parity の受理面
/// (issue #161)。受理優先順位:
///
/// - **Step A — 明示 alias 経路**: `offset`/`line_offset`/`limit`/
///   `line_limit`/`max_lines` のいずれかの key が存在すれば
///   `normalize_read_range`(上記 key 集合)をそのまま返し、
///   pair/array/`end_line` 形式は参照しない(曖昧な混在 payload は
///   明示語彙側へ deterministic 解決)。
/// - **Step B — inclusive pair/array 形式**(Step A の key が 1 つも
///   無い場合のみ): `start_line`+`end_line` が共に受理できるなら
///   `{offset:s, limit:e-s+1}`(1-based・inclusive)。`e < s` の矛盾
///   pair は helper 全体 `None` を即座に返す — Step C の lone
///   `start_line` 規約で `{s,None}` へ倒すと emitter 意図範囲より
///   狭い過少評価になり得るため(後続判定・Step C へも進まない)。
///   complete pair 不成立の場合のみ `view_range`(`Contradiction`
///   も同上で helper 全体 `None`)→ `end_line` 単独
///   (`{offset:1, limit:e}`)を評価する。
/// - **Step C — fallback**: 上記いずれも不成立なら
///   `normalize_read_range`(Step A と同一 key 集合)を呼ぶ — lone
///   `start_line` → `{s,None}`(s 行〜EOF)を受理する経路。矛盾に
///   よる short-circuit `None` はここへ落ちない。
///
/// 単一語彙の入力では無効・矛盾・非整数値が `None`(全文評価)または
/// より広い window 以外へ縮退しない(fail-closed)。混在語彙
/// payload は本 precedence による deterministic 解決。
fn windsurf_read_range(tool_info: &Map<String, Value>) -> Option<ReadRange> {
    const OFFSET_KEYS: &[&str] = &["offset", "line_offset", "start_line"];
    const LIMIT_KEYS: &[&str] = &["limit", "line_limit", "max_lines"];
    // Step A — 明示 offset/limit 系 key が 1 つでも存在する。
    // `start_line` は pair 語彙でもあるため trigger に含めない
    // (`{start_line, end_line}` pair 判定を潰さない)。
    if ["offset", "line_offset", "limit", "line_limit", "max_lines"]
        .iter()
        .any(|k| tool_info.contains_key(*k))
    {
        return normalize_read_range(tool_info, OFFSET_KEYS, LIMIT_KEYS);
    }
    // Step B — `start_line`+`end_line` inclusive pair → `view_range`
    // array → `end_line` 単独の順に評価する。
    let start_line = tool_info.get("start_line").and_then(json_line_number);
    let end_line = tool_info.get("end_line").and_then(json_line_number);
    if let (Some(s), Some(e)) = (start_line, end_line) {
        return if e < s {
            // 矛盾 pair — helper 全体 `None` で全文評価へ倒す。
            None
        } else {
            Some(ReadRange {
                offset: s,
                limit: Some(e - s + 1),
            })
        };
    }
    match normalize_view_range(tool_info) {
        ViewRange::Valid(range) => return Some(range),
        ViewRange::Contradiction => return None,
        ViewRange::Incomplete => {}
    }
    // `end_line` 単独(`start_line` 欠落・無効値)→ 1〜`end_line` 行
    // (より広い評価 = 安全側)。
    if let Some(e) = end_line {
        return Some(ReadRange {
            offset: 1,
            limit: Some(e),
        });
    }
    // Step C — fallback。`None` は全文評価縮退を意味する。
    normalize_read_range(tool_info, OFFSET_KEYS, LIMIT_KEYS)
}

/// `Search` 正規化 — `grep` 系 tool の bounded 検索 request。
/// `pattern` が非 string/欠落なら `None`(normalize 失敗 → allow と
/// 同等)。search root は `path` 優先(非 string は `"."`)、`path`
/// 欠落時は `file_path` の非空 string(windsurf `pre_read_code` は
/// `file_path` 語彙)、両者欠落・`file_path` 空/非 string は `"."`。
/// `max_results`/`head_limit` は小さい方へ畳み込む。`context` は
/// `context`/`-C` と `-A`/`-B` の最大値。
fn normalize_search(
    tool_input: &Map<String, Value>,
    cwd: &str,
    source_tool: Option<String>,
) -> Option<RoutingRequest> {
    let pattern = tool_input.get("pattern").and_then(Value::as_str)?;
    let path = match tool_input.get("path") {
        Some(v) => v.as_str().unwrap_or(".").to_string(),
        None => match tool_input.get("file_path").and_then(Value::as_str) {
            Some(fp) if !fp.is_empty() => fp.to_string(),
            _ => ".".to_string(),
        },
    };
    let pick_u64 =
        |keys: &[&str]| -> Option<u64> { pick_value(tool_input, keys).and_then(json_count) };
    let max_results = [pick_u64(&["max_results"]), pick_u64(&["head_limit"])]
        .into_iter()
        .flatten()
        .min();
    let context = [
        pick_u64(&["context"]),
        pick_u64(&["-C"]),
        pick_u64(&["-A"]),
        pick_u64(&["-B"]),
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or(0);
    let glob = ["glob", "glob_pattern"]
        .iter()
        .find_map(|k| tool_input.get(*k).and_then(Value::as_str))
        .map(str::to_string);
    let output_mode = match tool_input.get("output_mode").and_then(Value::as_str) {
        Some("files_with_matches") => SearchOutputMode::FilesWithMatches,
        Some("count") => SearchOutputMode::Count,
        Some("content") => SearchOutputMode::Content,
        Some(_) => SearchOutputMode::Unknown,
        None => SearchOutputMode::Content,
    };
    let case_insensitive = ["-i", "--ignore-case", "case_insensitive"]
        .iter()
        .any(|k| tool_input.get(*k).is_some_and(json_bool));
    Some(RoutingRequest::Search {
        path,
        cwd: cwd.to_string(),
        source_tool,
        pattern: pattern.to_string(),
        max_results,
        context,
        glob,
        output_mode,
        case_insensitive,
    })
}

/// `copilotHandle` の 3 値結果: normalize 失敗(None)・即時応答・
/// routing request。
enum CopilotOutcome {
    None,
    Respond(HookResponse),
    Request(RoutingRequest),
}

fn copilot_handle(input: &Value, cwd: &str) -> CopilotOutcome {
    let Some(payload) = input.as_object() else {
        return CopilotOutcome::None;
    };
    let tool_name = payload.get("toolName").and_then(Value::as_str);
    let Some(tool_name) = tool_name else {
        // Foreign payload shape (この hook entry が snake_case field を
        // 持つ他 target の preToolUse へ cross-fire した場合)。Cursor
        // 系 permission hook が silent exit 0 を block と解釈しない
        // よう dual schema の allow を emit する — Copilot は
        // `permissionDecision` を読んで明示 allow と見なす。
        // JS `||` は truthiness — falsy 値(null/false/0/"")は検出しない。
        if ["tool_name", "command", "file_path"]
            .iter()
            .any(|k| payload.get(*k).is_some_and(js_truthy))
        {
            return CopilotOutcome::Respond(HookResponse {
                stdout: Some(format!(
                    "{}\n",
                    stringify_ordered(&[
                        ("permission", Value::from("allow")),
                        ("permissionDecision", Value::from("allow")),
                    ])
                )),
                stderr: None,
                exit: 0,
            });
        }
        return CopilotOutcome::None;
    };
    let args = normalize_tool_args(payload.get("toolArgs"));
    if COPILOT_READ_TOOLS.contains(&tool_name) {
        let Some(args) = args else {
            return CopilotOutcome::None;
        };
        return match pick_string(&args, &["path", "file_path", "filePath"]) {
            Some(file_path) => CopilotOutcome::Request(RoutingRequest::FileRead {
                path: file_path.to_string(),
                cwd: cwd.to_string(),
                source_tool: Some(format!("copilot:{tool_name}")),
                // copilot は non-`Valid` を全て `None` へ写像する
                // plain `Option` 扱い(従来意味論と同値)。
                range: match normalize_view_range(&args) {
                    ViewRange::Valid(range) => Some(range),
                    ViewRange::Contradiction | ViewRange::Incomplete => None,
                },
            }),
            None => CopilotOutcome::None,
        };
    }
    if COPILOT_SHELL_TOOLS.contains(&tool_name) {
        if let Some(args) = &args {
            if let Some(command) =
                pick_string(args, &["command", "command_line", "commandLine", "cmd"])
            {
                return CopilotOutcome::Request(RoutingRequest::Shell {
                    command: command.to_string(),
                    cwd: cwd.to_string(),
                    source_tool: Some(format!("copilot:{tool_name}")),
                });
            }
        }
    }
    CopilotOutcome::None
}

fn copilot_render(verdict: Verdict) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => HookResponse {
            stdout: Some(format!(
                "{}\n",
                stringify_ordered(&[
                    ("permissionDecision", Value::from("deny")),
                    (
                        "permissionDecisionReason",
                        Value::from(flatten_reason(&reason)),
                    ),
                ])
            )),
            stderr: None,
            exit: 0,
        },
        Verdict::Allow => silent_allow(),
        Verdict::Rewrite { fallback, .. } => copilot_render(*fallback),
    }
}

fn tool_use_normalize(
    input: &Value,
    cwd: &str,
    source_prefix: &str,
    shell_tools: &[&str],
) -> Option<RoutingRequest> {
    let payload = input.as_object()?;
    let tool_name = payload.get("tool_name").and_then(Value::as_str)?;
    let tool_input = payload.get("tool_input").and_then(Value::as_object)?;
    let normalized = tool_name.to_ascii_lowercase();
    let source_tool = Some(format!("{source_prefix}:{tool_name}"));
    match normalized.as_str() {
        "read" => {
            let file_path = tool_input.get("file_path").and_then(Value::as_str)?;
            Some(RoutingRequest::FileRead {
                path: file_path.to_string(),
                cwd: cwd.to_string(),
                source_tool,
                // Devin `read` は `offset`/`limit`(#107)。範囲を
                // 落とすと全文評価で short read まで deny される
                // ため正規化で保持する。
                range: normalize_read_range(
                    tool_input,
                    &["offset", "line_offset", "start_line"],
                    &["limit", "line_limit", "max_lines"],
                ),
            })
        }
        "grep" => normalize_search(tool_input, cwd, source_tool),
        name if shell_tools.contains(&name) => {
            let command = tool_input.get("command").and_then(Value::as_str)?;
            Some(RoutingRequest::Shell {
                command: command.to_string(),
                cwd: cwd.to_string(),
                source_tool,
            })
        }
        _ => None,
    }
}

// --- pretooluse (claude) / codex: deny = hookSpecificOutput JSON + exit 0 ---

const PRETOOLUSE_SHELL_TOOLS: &[&str] = &["bash", "powershell"];

fn pretooluse_normalize(input: &Value, cwd: &str) -> Option<RoutingRequest> {
    tool_use_normalize(input, cwd, "pretooluse", PRETOOLUSE_SHELL_TOOLS)
}

/// `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
/// "permissionDecision":"deny",...}}` — claude/codex 共有の deny
/// 応答 shape。
fn hook_specific_output_deny(reason: &str) -> HookResponse {
    HookResponse {
        stdout: Some(format!(
            "{}\n",
            stringify_ordered(&[(
                "hookSpecificOutput",
                Value::Object({
                    let mut inner = Map::new();
                    inner.insert("hookEventName".to_string(), Value::from("PreToolUse"));
                    inner.insert("permissionDecision".to_string(), Value::from("deny"));
                    inner.insert(
                        "permissionDecisionReason".to_string(),
                        Value::from(flatten_reason(reason)),
                    );
                    inner
                }),
            )])
        )),
        stderr: None,
        exit: 0,
    }
}

/// `updatedInput` object — rewrite 対象 input の差し替え command。
fn updated_input(command: &str) -> Value {
    let mut inner = Map::new();
    inner.insert("command".to_string(), Value::from(command));
    Value::Object(inner)
}

/// `hookSpecificOutput` rewrite emit — `approval_bypass` 時のみ
/// `permissionDecision:"allow"` を hookEventName と updatedInput の
/// 間に挿入する(field 挿入順は probe 観測順、`stringify_ordered`
/// byte 互換規約)。flag 非対称の claude(`pretooluse`)/devin は常に
/// `false` で emit byte を現行と完全一致に保つ(D3)。
fn hook_specific_output_rewrite(command: &str, approval_bypass: bool) -> HookResponse {
    HookResponse {
        stdout: Some(format!(
            "{}\n",
            stringify_ordered(&[(
                "hookSpecificOutput",
                Value::Object({
                    let mut inner = Map::new();
                    inner.insert("hookEventName".to_string(), Value::from("PreToolUse"));
                    if approval_bypass {
                        inner.insert("permissionDecision".to_string(), Value::from("allow"));
                    }
                    inner.insert("updatedInput".to_string(), updated_input(command));
                    inner
                }),
            )])
        )),
        stderr: None,
        exit: 0,
    }
}

/// claude(`--target pretooluse`)— Rewrite は
/// `hookSpecificOutput.updatedInput` で emit する
/// (`permissionDecision` は emit しない、D3)。
fn pretooluse_render(verdict: Verdict) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => hook_specific_output_deny(&reason),
        Verdict::Allow => silent_allow(),
        Verdict::Rewrite { command, .. } => hook_specific_output_rewrite(&command, false),
    }
}

/// codex(`--target codex`)— `route.rewrite.approvalBypass` に
/// `"codex"` が含まれる場合のみ `permissionDecision:"allow"` +
/// `updatedInput` の bypassing emit を発行する(R6/D2)。flag 無し
/// (未指定・空・codex 非含有・route supply load 失敗)では従来どおり
/// fallback verdict を render し `updatedInput` を emit しない。
/// `updatedInput` 単独・`"ask"` 併記は codex が無視する silent
/// no-op(実機確認済み)のため、いかなる flag 状態でも発行しない
/// (R7/D2)。
fn codex_render(verdict: Verdict, approval_bypass: bool) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => hook_specific_output_deny(&reason),
        Verdict::Allow => silent_allow(),
        Verdict::Rewrite { command, .. } if approval_bypass => {
            hook_specific_output_rewrite(&command, true)
        }
        Verdict::Rewrite { fallback, .. } => codex_render(*fallback, approval_bypass),
    }
}

/// Devin の検証済み shell tool は `exec`(`tool_input.command` —
/// devin.ai docs の PreToolUse matcher 一覧)。`bash` は未検証のため
/// 登録しない(誤名を matcher に入れると hook が発火しないまま
/// 無評価になる)。
const DEVIN_SHELL_TOOLS: &[&str] = &["exec"];

/// Devin の cwd 解決: payload `cwd` → `DEVIN_PROJECT_DIR` →
/// hook process cwd。dir 走査が workspace 外の hook cwd に
/// anchor されないよう、この順序で workspace root を取る。
fn devin_normalize(event: &str, input: &Value, deps: &HookDeps<'_>) -> Option<RoutingRequest> {
    if event != "pre-tool-use" {
        return None;
    }
    let payload = input.as_object()?;
    if payload.get("hook_event_name").and_then(Value::as_str) != Some("PreToolUse") {
        return None;
    }
    let cwd = payload
        .get("cwd")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| (deps.env)("DEVIN_PROJECT_DIR").filter(|s| !s.is_empty()))
        .unwrap_or_else(|| deps.cwd.to_string());
    tool_use_normalize(input, &cwd, "devin", DEVIN_SHELL_TOOLS)
}

fn devin_render(verdict: Verdict) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => HookResponse {
            stdout: Some(format!(
                "{}\n",
                stringify_ordered(&[
                    ("decision", Value::from("block")),
                    ("reason", Value::from(reason)),
                ])
            )),
            stderr: None,
            exit: 0,
        },
        Verdict::Allow => silent_allow(),
        // devin 応答 envelope は `hookSpecificOutput` 規約 — Rewrite
        // は `hookSpecificOutput.updatedInput.command` で input
        // rewrite を通知する(C4)。
        Verdict::Rewrite { command, .. } => hook_specific_output_rewrite(&command, false),
    }
}

// --- cursor: always emit schema-valid permission JSON ---

fn cursor_normalize(input: &Value, cwd: &str) -> Option<RoutingRequest> {
    let payload = input.as_object()?;
    let payload_cwd = match payload.get("cwd").and_then(Value::as_str) {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => cwd.to_string(),
    };
    if let Some(file_path) = payload.get("file_path").and_then(Value::as_str) {
        if !file_path.is_empty() {
            return Some(RoutingRequest::FileRead {
                path: file_path.to_string(),
                cwd: payload_cwd,
                source_tool: Some("cursor:beforeReadFile".to_string()),
                // cursor `beforeReadFile` payload には検証済みの
                // 範囲 field が無いため全文評価のまま。
                range: None,
            });
        }
    }
    if let Some(command) = payload.get("command").and_then(Value::as_str) {
        if !command.is_empty() {
            return Some(RoutingRequest::Shell {
                command: command.to_string(),
                cwd: payload_cwd,
                source_tool: Some("cursor:beforeShellExecution".to_string()),
            });
        }
    }
    // `preToolUse` payload hypothesis — nested `tool_input` を field
    // 駆動で読む。`tool_name` domain(`"Shell"`/`"Read"` 等)は未検証
    // のため name matching ではなく field の有無で判別し、claude 型
    // nested / cursor 固有 flat の両 shape hypothesis を吸収する
    // (top-level 判定を先に試す順序は beforeXxx payload との誤判別
    // 防止)。field 駆動判別は over-match し得る(read 以外の tool が
    // `file_path` input を持つ場合に FileRead として評価される) —
    // `tool_name` domain 確定までの既知残 risk(design §Risks)。
    if let Some(tool_input) = payload.get("tool_input").and_then(Value::as_object) {
        if let Some(command) = tool_input.get("command").and_then(Value::as_str) {
            if !command.is_empty() {
                return Some(RoutingRequest::Shell {
                    command: command.to_string(),
                    cwd: payload_cwd,
                    source_tool: Some("cursor:preToolUse".to_string()),
                });
            }
        }
        if let Some(file_path) = tool_input.get("file_path").and_then(Value::as_str) {
            if !file_path.is_empty() {
                return Some(RoutingRequest::FileRead {
                    path: file_path.to_string(),
                    cwd: payload_cwd,
                    source_tool: Some("cursor:preToolUse".to_string()),
                    // 範囲 field は未検証のため全文評価のまま
                    // (beforeReadFile と同じ扱い)。
                    range: None,
                });
            }
        }
    }
    None
}

fn cursor_respond(permission: &str, reason: Option<&str>) -> HookResponse {
    let mut map = Map::new();
    map.insert("permission".to_string(), Value::from(permission));
    if let Some(reason) = reason {
        let text = flatten_reason(reason);
        map.insert("user_message".to_string(), Value::from(text.clone()));
        map.insert("agent_message".to_string(), Value::from(text));
    }
    HookResponse {
        stdout: Some(format!("{}\n", stringify(&Value::Object(map)))),
        stderr: None,
        exit: 0,
    }
}

fn cursor_render(verdict: Verdict) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => cursor_respond("deny", Some(&reason)),
        Verdict::Allow => cursor_respond("allow", None),
        Verdict::Rewrite { fallback, .. } => cursor_render(*fallback),
    }
}

/// verdict の分類値(`cheaper.hook` 完了 event の `verdict` field 用)。
/// normalize 失敗・foreign payload 等の即時 allow 経路は verdict を
/// 通っていないが応答 semantics 上は allow なので `"allow"` を返す。
fn verdict_name(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "allow",
        Verdict::Deny { .. } => "deny",
        Verdict::Rewrite { .. } => "rewrite",
    }
}

/// request の cwd — 各 variant は `cwd` field を持つ(normalize が
/// payload・`deps.cwd` fallback で確定済み)。route supply の project
/// 層起点・resolve cwd に使う。
fn request_cwd(request: &RoutingRequest) -> &str {
    match request {
        RoutingRequest::FileRead { cwd, .. }
        | RoutingRequest::Search { cwd, .. }
        | RoutingRequest::Shell { cwd, .. } => cwd,
    }
}

/// request 単位で routes + command policy を 1 回 load し eval へ
/// 供給する(C1 — firing あたり 1 回の load に集約。per-candidate
/// reload はしない)。load 失敗は `route: None`(route arm 不活性)
/// へ畳み eval 自体は従来どおり走る — hook の fail-open 規約。
/// load 済み `RouteSupply` を verdict と共に返す — codex arm が
/// `route.rewrite.approvalBypass` flag を render 層へ供給する経路
/// (`load_or_none` → `None` は flag 未供給 = bypass 無効の
/// conservative default)。
async fn evaluate_with_routes(
    request: &RoutingRequest,
    deps: &HookDeps<'_>,
) -> (Verdict, Option<crate::route::supply::RouteSupply>) {
    let supply =
        crate::route::supply::load_or_none(request_cwd(request), deps.env, deps.builtin_dir);
    let mut opts = deps.eval_opts();
    opts.route = supply.as_ref().map(|s| s.eval_input());
    (eval_cache::evaluate_dedup(request, &opts).await, supply)
}

/// target 別の normalize → evaluate → render。
/// normalize/render のどの段でも失敗した場合は呼び出し側の catch-all
/// が fallback へ倒す。
pub async fn handle_hook(
    target: HookTarget,
    event: &str,
    input: &Value,
    deps: &HookDeps<'_>,
) -> HookResponse {
    let (verdict, bypass, response) = match target {
        HookTarget::Windsurf => match windsurf_normalize(event, input, deps.cwd) {
            None => ("allow", false, silent_allow()),
            Some(request) => {
                let (verdict, _supply) = evaluate_with_routes(&request, deps).await;
                (verdict_name(&verdict), false, windsurf_render(verdict))
            }
        },
        HookTarget::Copilot => match copilot_handle(input, deps.cwd) {
            CopilotOutcome::None => ("allow", false, silent_allow()),
            CopilotOutcome::Respond(r) => ("allow", false, r),
            CopilotOutcome::Request(request) => {
                let (verdict, _supply) = evaluate_with_routes(&request, deps).await;
                (verdict_name(&verdict), false, copilot_render(verdict))
            }
        },
        HookTarget::Pretooluse => match pretooluse_normalize(input, deps.cwd) {
            None => ("allow", false, silent_allow()),
            Some(request) => {
                let (verdict, _supply) = evaluate_with_routes(&request, deps).await;
                (verdict_name(&verdict), false, pretooluse_render(verdict))
            }
        },
        HookTarget::Devin => match devin_normalize(event, input, deps) {
            None => ("allow", false, silent_allow()),
            Some(request) => {
                let (verdict, _supply) = evaluate_with_routes(&request, deps).await;
                (verdict_name(&verdict), false, devin_render(verdict))
            }
        },
        HookTarget::Cursor => match cursor_normalize(input, deps.cwd) {
            None => ("allow", false, cursor_respond("allow", None)),
            Some(request) => {
                // Rust の `evaluate` は error を外部へ伝播しない
                // (計測失敗は内部で allow)が、TS の try/catch allow
                // 経路と同じく verdict に関わらず schema-valid JSON を
                // 返すことは変えない。
                let (verdict, _supply) = evaluate_with_routes(&request, deps).await;
                (verdict_name(&verdict), false, cursor_render(verdict))
            }
        },
        HookTarget::Codex => match pretooluse_normalize(input, deps.cwd) {
            None => ("allow", false, silent_allow()),
            Some(request) => {
                let (verdict, supply) = evaluate_with_routes(&request, deps).await;
                // bypassing emit の許可は codex arm のみで読む —
                // `load_or_none` → `None` は flag 未供給 = bypass 無効
                // (conservative default、R15)。
                let bypass = supply
                    .as_ref()
                    .is_some_and(|s| s.policy.route.rewrite.approval_bypass_enabled("codex"));
                (
                    verdict_name(&verdict),
                    bypass,
                    codex_render(verdict, bypass),
                )
            }
        },
    };
    // `cheaper.hook` 完了 event(design §instrumentation 配置)—
    // `verdict` 分類値 + 出力 byte 長のみ(deny reason・payload 本文は
    // metadata-only 規約で載せない)。`bypass` は approval-bypass
    // emit の観測面 — bypassing emit は silent exit 0 で stderr 痕跡を
    // 持たないためこの field が唯一の観測経路(codex arm でのみ
    // `true` になり得る)。
    tracing::debug!(
        verdict = verdict,
        bypass = bypass,
        stdout.len = response.stdout.as_deref().map_or(0, str::len),
        stderr.len = response.stderr.as_deref().map_or(0, str::len),
        "hook response"
    );
    response
}

/// 最深部 catch-all が使う target 適合の allow 応答。
/// `target` が判別不能(None)のときは silent — cursor のみ schema
/// 必須のため `{"permission":"allow"}` を emit する。
pub fn fallback_allow(target: Option<HookTarget>) -> HookResponse {
    match target {
        Some(HookTarget::Cursor) => cursor_respond("allow", None),
        _ => silent_allow(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn rewrite(command: &str, fallback: Verdict) -> Verdict {
        Verdict::Rewrite {
            command: command.to_string(),
            fallback: Box::new(fallback),
        }
    }

    /// claude(`pretooluse`)— Rewrite は `updatedInput.command` を
    /// emit し `permissionDecision` を載せない(D3)。exit 0。
    #[test]
    fn pretooluse_rewrite_emits_updated_input_without_permission() {
        let r = pretooluse_render(rewrite(
            "\"/x/cheaper\" route run --payload aa",
            Verdict::Allow,
        ));
        assert_eq!(r.exit, 0);
        let stdout = r.stdout.unwrap();
        let v: Value = serde_json::from_str(&stdout).unwrap();
        let hso = v.get("hookSpecificOutput").unwrap();
        assert_eq!(
            hso.get("hookEventName").and_then(Value::as_str),
            Some("PreToolUse")
        );
        assert_eq!(
            hso.pointer("/updatedInput/command").and_then(Value::as_str),
            Some("\"/x/cheaper\" route run --payload aa")
        );
        assert!(hso.get("permissionDecision").is_none());
    }

    /// claude — Rewrite の render は fallback 内容に関わらず
    /// `updatedInput` のみ(R37)。fallback の保持は rewrite 非対応
    /// target 向けの verdict 構造であり、claude render は参照しない。
    #[test]
    fn pretooluse_rewrite_ignores_fallback_for_render() {
        let r = pretooluse_render(rewrite("cmd", Verdict::Deny { reason: "x".into() }));
        assert_eq!(r.exit, 0);
        let v: Value = serde_json::from_str(&r.stdout.unwrap()).unwrap();
        assert!(v
            .pointer("/hookSpecificOutput/updatedInput/command")
            .is_some());
    }

    /// devin — Rewrite は hookSpecificOutput.updatedInput.command
    /// のみを返す(承認判定 field は付けない)。
    #[test]
    fn devin_rewrite_emits_updated_input_without_permission() {
        let r = devin_render(rewrite("new-cmd", Verdict::Allow));
        assert_eq!(r.exit, 0);
        let v: Value = serde_json::from_str(&r.stdout.unwrap()).unwrap();
        assert!(v.get("decision").is_none());
        assert!(v.get("updatedInput").is_none());
        assert_eq!(
            v.pointer("/hookSpecificOutput/hookEventName")
                .and_then(Value::as_str),
            Some("PreToolUse")
        );
        assert_eq!(
            v.pointer("/hookSpecificOutput/updatedInput/command")
                .and_then(Value::as_str),
            Some("new-cmd")
        );
        assert!(v
            .pointer("/hookSpecificOutput/permissionDecision")
            .is_none());
    }

    /// codex — flag 無しの Rewrite は `updatedInput` を発行せず
    /// fallback verdict を render する(R7 — flag は emit 有効化で
    /// はなく許可。未指定・空・codex 非含有・supply load 失敗は全て
    /// この経路)。
    #[test]
    fn codex_rewrite_renders_fallback_only() {
        // fallback Deny → hookSpecificOutput deny。
        let r = codex_render(
            rewrite(
                "cmd",
                Verdict::Deny {
                    reason: "nope".into(),
                },
            ),
            false,
        );
        assert_eq!(r.exit, 0);
        let v: Value = serde_json::from_str(&r.stdout.unwrap()).unwrap();
        assert_eq!(
            v.pointer("/hookSpecificOutput/permissionDecision")
                .and_then(Value::as_str),
            Some("deny")
        );
        assert!(v.pointer("/hookSpecificOutput/updatedInput").is_none());
        // fallback Allow → silent。
        let r = codex_render(rewrite("cmd", Verdict::Allow), false);
        assert_eq!(r.exit, 0);
        assert!(r.stdout.is_none() && r.stderr.is_none());
    }

    /// codex — `route.rewrite.approvalBypass` に `"codex"` 含有時のみ
    /// `hookEventName:"PreToolUse"` → `permissionDecision:"allow"` →
    /// `updatedInput:{command}` の順(probe 観測順)で bypassing emit
    /// を発行し exit 0(R6/D2)。
    #[test]
    fn codex_rewrite_with_approval_bypass_emits_allow_and_updated_input() {
        let r = codex_render(
            rewrite("\"/x/cheaper\" route run --payload aa", Verdict::Allow),
            true,
        );
        assert_eq!(r.exit, 0);
        assert!(r.stderr.is_none());
        let stdout = r.stdout.unwrap();
        // field 挿入順を byte レベルに近い精度で pin —
        // hookEventName → permissionDecision → updatedInput。
        let event_pos = stdout.find("\"hookEventName\"").unwrap();
        let decision_pos = stdout.find("\"permissionDecision\"").unwrap();
        let input_pos = stdout.find("\"updatedInput\"").unwrap();
        assert!(
            event_pos < decision_pos && decision_pos < input_pos,
            "{stdout}"
        );
        let v: Value = serde_json::from_str(&stdout).unwrap();
        let hso = v.get("hookSpecificOutput").unwrap();
        assert_eq!(
            hso.get("hookEventName").and_then(Value::as_str),
            Some("PreToolUse")
        );
        assert_eq!(
            hso.get("permissionDecision").and_then(Value::as_str),
            Some("allow")
        );
        assert_eq!(
            hso.pointer("/updatedInput/command").and_then(Value::as_str),
            Some("\"/x/cheaper\" route run --payload aa")
        );
        // top-level `decision`/`updatedInput` は無い。
        assert!(v.get("decision").is_none() && v.get("updatedInput").is_none());
    }

    /// codex 負 shape — `updatedInput` 単独・`permissionDecision:"ask"`
    /// 併記の emit はいかなる flag 状態でも発行しない(codex が両者を
    /// 無視する silent no-op、R7/D2)。emit され得る shape は `allow`
    /// 併記形のみ。
    #[test]
    fn codex_rewrite_never_emits_unapplied_shapes() {
        for flag in [false, true] {
            for fallback in [Verdict::Allow, Verdict::Deny { reason: "r".into() }] {
                let r = codex_render(rewrite("cmd", fallback), flag);
                if let Some(stdout) = &r.stdout {
                    let v: Value = serde_json::from_str(stdout).unwrap();
                    let decision = v
                        .pointer("/hookSpecificOutput/permissionDecision")
                        .and_then(Value::as_str);
                    // `"ask"` を emit する経路は存在しない。
                    assert_ne!(decision, Some("ask"), "{stdout}");
                    // `updatedInput` を `permissionDecision` 無しで
                    // emit する経路は存在しない。
                    if v.pointer("/hookSpecificOutput/updatedInput").is_some() {
                        assert_eq!(decision, Some("allow"), "{stdout}");
                    }
                }
            }
        }
    }

    /// windsurf — Rewrite は fallback render(deny → stderr exit 2)。
    #[test]
    fn windsurf_rewrite_renders_fallback() {
        let r = windsurf_render(rewrite(
            "cmd",
            Verdict::Deny {
                reason: "blocked".into(),
            },
        ));
        assert_eq!(r.exit, 2);
        assert!(r.stderr.unwrap().contains("blocked"));
        let r = windsurf_render(rewrite("cmd", Verdict::Allow));
        assert_eq!(r.exit, 0);
        assert!(r.stdout.is_none());
    }

    /// cursor `preToolUse` — nested `tool_input` を field 駆動で読む
    /// (payload shape hypothesis — `tool_name` domain 未検証のため
    /// name matching ではなく field の有無で判別)。
    #[test]
    fn cursor_normalize_reads_nested_tool_input() {
        // tool_input.command → Shell(source_tool: cursor:preToolUse)。
        let req = cursor_normalize(
            &json!({"tool_input": {"command": "routed x"}, "cwd": "/w"}),
            "/fallback",
        )
        .expect("tool_input.command normalizes");
        match req {
            RoutingRequest::Shell {
                command,
                cwd,
                source_tool,
            } => {
                assert_eq!(command, "routed x");
                assert_eq!(cwd, "/w");
                assert_eq!(source_tool.as_deref(), Some("cursor:preToolUse"));
            }
            _ => panic!("expected Shell"),
        }
        // tool_input.file_path → FileRead(range: None)。
        let req = cursor_normalize(&json!({"tool_input": {"file_path": "/abs/big.rs"}}), "/w")
            .expect("tool_input.file_path normalizes");
        match req {
            RoutingRequest::FileRead {
                path,
                source_tool,
                range,
                ..
            } => {
                assert_eq!(path, "/abs/big.rs");
                assert_eq!(source_tool.as_deref(), Some("cursor:preToolUse"));
                assert!(range.is_none());
            }
            _ => panic!("expected FileRead"),
        }
        // top-level(beforeXxx payload)は従来 source_tool で先に評価
        // される — nested との誤判別を防ぐため flat 判定が先。
        let req = cursor_normalize(
            &json!({"command": "flat", "tool_input": {"command": "nested"}}),
            "/w",
        )
        .expect("flat command normalizes");
        match req {
            RoutingRequest::Shell {
                command,
                source_tool,
                ..
            } => {
                assert_eq!(command, "flat");
                assert_eq!(source_tool.as_deref(), Some("cursor:beforeShellExecution"));
            }
            _ => panic!("expected Shell"),
        }
        // malformed・field 欠落は None(fail-open — caller 側の
        // schema-valid `{"permission":"allow"}` 経路)。
        for input in [
            json!({}),
            json!(null),
            json!({"tool_input": {}}),
            json!({"tool_input": "x"}),
            json!({"tool_input": {"command": ""}}),
        ] {
            assert!(cursor_normalize(&input, "/w").is_none(), "{input}");
        }
    }

    /// cursor/copilot — fallback のみ。emit contract(`updated_input`/
    /// `updatedInput`・`permission`/`permissionDecision` 併記の approval
    /// 意味)が実機検証で確定するまでいかなる flag 状態でも emit しない
    /// (R10/R12 — `cursor_render`/`copilot_render` は flag を受け取らず
    /// 構造的に emit 不能)。
    #[test]
    fn cursor_and_copilot_rewrite_render_fallback() {
        let r = cursor_render(rewrite("cmd", Verdict::Allow));
        assert_eq!(r.exit, 0);
        let v: Value = serde_json::from_str(&r.stdout.unwrap()).unwrap();
        assert_eq!(v.get("permission").and_then(Value::as_str), Some("allow"));
        assert!(v.get("updatedInput").is_none() && v.get("updated_input").is_none());
        let r = copilot_render(rewrite(
            "cmd",
            Verdict::Deny {
                reason: "deny-reason".into(),
            },
        ));
        assert_eq!(r.exit, 0);
        let v: Value = serde_json::from_str(&r.stdout.unwrap()).unwrap();
        assert_eq!(
            v.get("permissionDecision").and_then(Value::as_str),
            Some("deny")
        );
    }

    /// `windsurf_read_range` — Step A(明示 alias)→ Step B(pair/
    /// array/`end_line` 単独)→ Step C(fallback)の受理順・矛盾
    /// short-circuit・`-1` sentinel・混在 precedence を pin する
    /// (issue #161)。`None` は全文評価縮退を意味し、矛盾 pair/array
    /// が Step C の lone `start_line` 規約(`{s,None}`)へ落ちない
    /// ことが fail-closed の肝。
    #[test]
    fn windsurf_read_range_table() {
        type Case = (&'static str, Option<(u64, Option<u64>)>);
        let cases: &[Case] = &[
            // Step A — 明示 alias 経路(`start_line` は offset alias
            // として受理 — 共有 `tool_use_normalize` と同一 key 集合)。
            ("{\"offset\":10,\"limit\":5}", Some((10, Some(5)))),
            ("{\"line_offset\":10,\"line_limit\":5}", Some((10, Some(5)))),
            ("{\"offset\":1,\"max_lines\":20}", Some((1, Some(20)))),
            ("{\"start_line\":5,\"limit\":20}", Some((5, Some(20)))),
            ("{\"start_line\":5,\"line_limit\":20}", Some((5, Some(20)))),
            ("{\"start_line\":5,\"max_lines\":20}", Some((5, Some(20)))),
            ("{\"limit\":20}", Some((1, Some(20)))),
            // explicit key が 1 つでもあれば pair/array/end_line は
            // 参照しない(混在語彙は deterministic 解決)。
            (
                "{\"offset\":10,\"limit\":5,\"view_range\":[1,1000]}",
                Some((10, Some(5))),
            ),
            (
                "{\"limit\":5,\"start_line\":10,\"end_line\":30}",
                Some((10, Some(5))),
            ),
            (
                "{\"line_limit\":5,\"start_line\":10,\"end_line\":30}",
                Some((10, Some(5))),
            ),
            // Step B — `start_line`+`end_line` inclusive pair。
            ("{\"start_line\":10,\"end_line\":30}", Some((10, Some(21)))),
            ("{\"start_line\":10,\"end_line\":10}", Some((10, Some(1)))),
            // `start_line` が無効でも `end_line` が有効なら
            // `end_line` 単独(`{1,e}` — より広い評価=安全側)へ縮退。
            ("{\"start_line\":0,\"end_line\":30}", Some((1, Some(30)))),
            // `view_range` array — pair 不成立時のみ評価。
            ("{\"view_range\":[10,30]}", Some((10, Some(21)))),
            // `[s,-1]` は "to EOF" sentinel → `{s,None}`。
            ("{\"view_range\":[10,-1]}", Some((10, None))),
            ("{\"view_range\":[1,-1]}", Some((1, None))),
            // `end_line` 単独 → `{1,e}`。
            ("{\"end_line\":30}", Some((1, Some(30)))),
            // Step C — lone `start_line` → `{s,None}`(s 行〜EOF)。
            ("{\"start_line\":5}", Some((5, None))),
            // 矛盾 short-circuit — helper 全体 `None` で Step C へ
            // 落ちない(`{500,None}` にはならない)。
            ("{\"start_line\":500,\"end_line\":20}", None),
            (
                "{\"start_line\":500,\"end_line\":20,\"view_range\":[1,10]}",
                None,
            ),
            ("{\"view_range\":[10,2]}", None),
            // 無効・非整数値 → `None`(全文評価縮退)。
            ("{}", None),
            ("{\"start_line\":0}", None),
            ("{\"start_line\":-3}", None),
            ("{\"start_line\":1.5}", None),
            ("{\"start_line\":\"5\"}", None),
            ("{\"view_range\":[10,0]}", None),
            ("{\"view_range\":[10,-5]}", None),
            ("{\"view_range\":[0,10]}", None),
            ("{\"view_range\":\"x\"}", None),
            ("{\"view_range\":[10]}", None),
            ("{\"end_line\":0}", None),
            ("{\"end_line\":\"x\"}", None),
            // `offset == 1 && limit == None` は全文と同値で
            // `normalize_read_range` が `None` へ畳む。
            ("{\"offset\":1}", None),
            ("{\"offset\":0,\"limit\":10}", None),
            ("{\"offset\":10,\"limit\":0}", None),
        ];
        for (json_str, expect) in cases {
            let v: Value = serde_json::from_str(json_str).unwrap();
            let map = v.as_object().unwrap();
            let got = windsurf_read_range(map).map(|r| (r.offset, r.limit));
            assert_eq!(got, *expect, "tool_info={json_str}");
        }
    }

    /// copilot `view` — 共有 `normalize_view_range` の non-`Valid` は
    /// 全て `None`(従来意味論と同値)。`[s,-1]` → `{s,None}` の
    /// "to EOF" sentinel は共有化に伴い copilot にも効く(design
    /// 受理済の差分 — copilot wire では `-1` 未観測)。
    #[test]
    fn copilot_view_range_maps_non_valid_to_none() {
        let range_of = |tool_args: &str| {
            let input: Value = serde_json::from_str(&format!(
                "{{\"toolName\":\"view\",\"toolArgs\":{tool_args}}}"
            ))
            .unwrap();
            match copilot_handle(&input, "/w") {
                CopilotOutcome::Request(RoutingRequest::FileRead { range, .. }) => {
                    range.map(|r| (r.offset, r.limit))
                }
                _ => panic!("expected FileRead request"),
            }
        };
        assert_eq!(
            range_of("{\"path\":\"f.txt\",\"view_range\":[10,-1]}"),
            Some((10, None))
        );
        // `end` 不受理(`0`)は `None` — 全文評価縮退(従来どおり)。
        assert_eq!(range_of("{\"path\":\"f.txt\",\"view_range\":[10,0]}"), None);
        // `end < start` の矛盾も `None`(従来どおり)。
        assert_eq!(range_of("{\"path\":\"f.txt\",\"view_range\":[10,2]}"), None);
        assert_eq!(
            range_of("{\"path\":\"f.txt\",\"view_range\":[10,30]}"),
            Some((10, Some(21)))
        );
        // `view_range` 欠落・非 array は `None`。
        assert_eq!(range_of("{\"path\":\"f.txt\"}"), None);
        assert_eq!(range_of("{\"path\":\"f.txt\",\"view_range\":\"x\"}"), None);
    }

    /// `windsurf_normalize` — `pre_read_code` payload の各 `tool_info`
    /// 形が期待 `ReadRange` を持つ `FileRead` request へ正規化される
    /// (adapter 接続の smoke — 受理規則本体は `windsurf_read_range`
    /// の table test で pin)。
    #[test]
    fn windsurf_normalize_maps_tool_info_range_vocabularies() {
        let normalize = |tool_info: &str| {
            let input: Value = serde_json::from_str(&format!(
                "{{\"agent_action_name\":\"pre_read_code\",\"tool_info\":{tool_info}}}"
            ))
            .unwrap();
            windsurf_normalize("pre-read-code", &input, "/w")
        };
        let assert_range =
            |tool_info: &str, expect: Option<(u64, Option<u64>)>| match normalize(tool_info) {
                Some(RoutingRequest::FileRead {
                    path,
                    range,
                    source_tool,
                    ..
                }) => {
                    assert_eq!(path, "big_lines.txt", "{tool_info}");
                    assert_eq!(source_tool.as_deref(), Some("windsurf:pre_read_code"));
                    assert_eq!(range.map(|r| (r.offset, r.limit)), expect, "{tool_info}");
                }
                _ => panic!("expected FileRead: {tool_info}"),
            };
        assert_range(
            "{\"file_path\":\"big_lines.txt\",\"start_line\":1,\"end_line\":20}",
            Some((1, Some(20))),
        );
        assert_range(
            "{\"file_path\":\"big_lines.txt\",\"view_range\":[1,20]}",
            Some((1, Some(20))),
        );
        assert_range(
            "{\"file_path\":\"big_lines.txt\",\"offset\":1,\"limit\":20}",
            Some((1, Some(20))),
        );
        assert_range(
            "{\"file_path\":\"big_lines.txt\",\"end_line\":20}",
            Some((1, Some(20))),
        );
        assert_range(
            "{\"file_path\":\"big_lines.txt\",\"start_line\":1995}",
            Some((1995, None)),
        );
        // 矛盾 pair → `range: None`(全文評価縮退)。
        assert_range(
            "{\"file_path\":\"big_lines.txt\",\"start_line\":1900,\"end_line\":20}",
            None,
        );
        assert_range("{\"file_path\":\"big_lines.txt\"}", None);
    }

    /// `windsurf_normalize` の `pattern`→`Search` gate 全分岐
    /// (issue #161 T2)。`pattern` 非空 string は kind 判定を最優先
    /// するが、`file_path` が file 解決・metadata 失敗の併記形は
    /// FileRead(全文 eval)を維持し、`pattern` 空/非 string も
    /// FileRead 判定へ進む(過少評価・fail-open を塞ぐ 2 条件 gate)。
    #[test]
    fn windsurf_normalize_pattern_gate_routes_search_or_fileread() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().to_string_lossy().to_string();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("f.txt"), "x").unwrap();

        let normalize = |tool_info: &str| {
            let input: Value = serde_json::from_str(&format!(
                "{{\"agent_action_name\":\"pre_read_code\",\"tool_info\":{tool_info}}}"
            ))
            .unwrap();
            windsurf_normalize("pre-read-code", &input, &cwd)
        };
        // `pattern` 非空 + `file_path` 欠落/空/非 string/dir 解決
        // → `Search`(search root は normalize_search の
        // `path`/`file_path`/`"."` fallback 側で解決)。
        for (tool_info, expect_root) in [
            ("{\"pattern\":\"lorem\"}", "."),
            ("{\"file_path\":\"\",\"pattern\":\"lorem\"}", "."),
            ("{\"file_path\":123,\"pattern\":\"lorem\"}", "."),
            ("{\"file_path\":\"sub\",\"pattern\":\"lorem\"}", "sub"),
            // `path` key は `file_path` より優先される。
            (
                "{\"file_path\":\"sub\",\"path\":\"alt\",\"pattern\":\"lorem\"}",
                "alt",
            ),
        ] {
            match normalize(tool_info) {
                Some(RoutingRequest::Search {
                    path,
                    pattern,
                    source_tool,
                    ..
                }) => {
                    assert_eq!(pattern, "lorem", "{tool_info}");
                    assert_eq!(path, expect_root, "{tool_info}");
                    assert_eq!(
                        source_tool.as_deref(),
                        Some("windsurf:pre_read_code"),
                        "{tool_info}"
                    );
                }
                _ => panic!("expected Search for {tool_info}"),
            }
        }
        // `file_path` が file 解決・metadata 失敗の併記形、および
        // `pattern` 空/非 string → FileRead 維持(gate で潰さない)。
        for tool_info in [
            "{\"file_path\":\"f.txt\",\"pattern\":\"lorem\"}",
            "{\"file_path\":\"nope.txt\",\"pattern\":\"lorem\"}",
            "{\"file_path\":\"f.txt\",\"pattern\":\"\"}",
            "{\"file_path\":\"f.txt\",\"pattern\":123}",
        ] {
            match normalize(tool_info) {
                Some(RoutingRequest::FileRead { path, .. }) => {
                    assert!(path == "f.txt" || path == "nope.txt", "{tool_info}");
                }
                _ => panic!("expected FileRead for {tool_info}"),
            }
        }
    }

    /// `normalize_search` の search root fallback — `path` 優先
    /// (非 string は `"."`)、`path` 欠落時は非空 `file_path`、
    /// 両者欠落/`file_path` 空・非 string は `"."`(issue #161 T2)。
    #[test]
    fn normalize_search_path_fallback_order() {
        let search_root = |tool_input: &str| {
            let v: Value = serde_json::from_str(tool_input).unwrap();
            match normalize_search(v.as_object().unwrap(), "/w", None) {
                Some(RoutingRequest::Search { path, .. }) => path,
                _ => panic!("expected Search: {tool_input}"),
            }
        };
        assert_eq!(search_root("{\"pattern\":\"p\",\"path\":\"x\"}"), "x");
        // `path` 非 string は従来どおり `"."`(`file_path` へは落ちない)。
        assert_eq!(
            search_root("{\"pattern\":\"p\",\"path\":123,\"file_path\":\"fp\"}"),
            "."
        );
        assert_eq!(
            search_root("{\"pattern\":\"p\",\"file_path\":\"fp\"}"),
            "fp"
        );
        assert_eq!(search_root("{\"pattern\":\"p\"}"), ".");
        assert_eq!(search_root("{\"pattern\":\"p\",\"file_path\":\"\"}"), ".");
        assert_eq!(search_root("{\"pattern\":\"p\",\"file_path\":123}"), ".");
    }
}
