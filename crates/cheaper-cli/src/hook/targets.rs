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
            range: tool_info.and_then(|t| {
                normalize_read_range(t, &["offset", "line_offset"], &["limit", "line_limit"])
            }),
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

/// `Search` 正規化 — `grep` 系 tool の bounded 検索 request。
/// `pattern` が非 string/欠落なら `None`(normalize 失敗 → allow と
/// 同等)。`path` 欠落・非 string は `"."`。
/// `max_results`/`head_limit` は小さい方へ畳み込む。`context` は
/// `context`/`-C` と `-A`/`-B` の最大値。
fn normalize_search(
    tool_input: &Map<String, Value>,
    cwd: &str,
    source_tool: Option<String>,
) -> Option<RoutingRequest> {
    let pattern = tool_input.get("pattern").and_then(Value::as_str)?;
    let path = match tool_input.get("path") {
        None => ".".to_string(),
        Some(v) => v.as_str().unwrap_or(".").to_string(),
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
                range: copilot_view_range(&args),
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

/// Copilot `view` の `view_range`(`[startLine, endLine]`、1-based・
/// inclusive)を `ReadRange` へ正規化する。無効 shape は `None`
/// (全文評価へ縮退)。
fn copilot_view_range(args: &Map<String, Value>) -> Option<ReadRange> {
    let arr = args.get("view_range")?.as_array()?;
    let start = arr.first().and_then(json_line_number)?;
    let end = arr.get(1).and_then(json_line_number)?;
    if end < start {
        return None;
    }
    Some(ReadRange {
        offset: start,
        limit: Some(end - start + 1),
    })
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

    /// cursor/copilot — fallback のみ。
    #[test]
    fn cursor_and_copilot_rewrite_render_fallback() {
        let r = cursor_render(rewrite("cmd", Verdict::Allow));
        assert_eq!(r.exit, 0);
        let v: Value = serde_json::from_str(&r.stdout.unwrap()).unwrap();
        assert_eq!(v.get("permission").and_then(Value::as_str), Some("allow"));
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
}
