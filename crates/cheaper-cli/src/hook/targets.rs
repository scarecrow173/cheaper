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

use super::routing_policy::{
    evaluate, EvaluateOptions, ReadRange, RoutingRequest, SearchOutputMode, Verdict,
};

/// `HookTarget` — `HOOK_TARGETS` の 5 値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookTarget {
    Windsurf,
    Copilot,
    Pretooluse,
    Cursor,
    Devin,
}

/// `HOOK_TARGETS` — argv の `--target` 値 → target。
impl HookTarget {
    pub const ALL: [Self; 5] = [
        Self::Windsurf,
        Self::Copilot,
        Self::Pretooluse,
        Self::Cursor,
        Self::Devin,
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

// --- pretooluse (claude / codex): deny = hookSpecificOutput JSON + exit 0 ---

const PRETOOLUSE_SHELL_TOOLS: &[&str] = &["bash", "powershell"];

fn pretooluse_normalize(input: &Value, cwd: &str) -> Option<RoutingRequest> {
    tool_use_normalize(input, cwd, "pretooluse", PRETOOLUSE_SHELL_TOOLS)
}

fn pretooluse_render(verdict: Verdict) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => HookResponse {
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
                            Value::from(flatten_reason(&reason)),
                        );
                        inner
                    }),
                )])
            )),
            stderr: None,
            exit: 0,
        },
        Verdict::Allow => silent_allow(),
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
    }
}

/// verdict の分類値(`cheaper.hook` 完了 event の `verdict` field 用)。
/// normalize 失敗・foreign payload 等の即時 allow 経路は verdict を
/// 通っていないが応答 semantics 上は allow なので `"allow"` を返す。
fn verdict_name(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "allow",
        Verdict::Deny { .. } => "deny",
    }
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
    let (verdict, response) = match target {
        HookTarget::Windsurf => match windsurf_normalize(event, input, deps.cwd) {
            None => ("allow", silent_allow()),
            Some(request) => {
                let verdict = evaluate(&request, &deps.eval_opts()).await;
                (verdict_name(&verdict), windsurf_render(verdict))
            }
        },
        HookTarget::Copilot => match copilot_handle(input, deps.cwd) {
            CopilotOutcome::None => ("allow", silent_allow()),
            CopilotOutcome::Respond(r) => ("allow", r),
            CopilotOutcome::Request(request) => {
                let verdict = evaluate(&request, &deps.eval_opts()).await;
                (verdict_name(&verdict), copilot_render(verdict))
            }
        },
        HookTarget::Pretooluse => match pretooluse_normalize(input, deps.cwd) {
            None => ("allow", silent_allow()),
            Some(request) => {
                let verdict = evaluate(&request, &deps.eval_opts()).await;
                (verdict_name(&verdict), pretooluse_render(verdict))
            }
        },
        HookTarget::Devin => match devin_normalize(event, input, deps) {
            None => ("allow", silent_allow()),
            Some(request) => {
                let verdict = evaluate(&request, &deps.eval_opts()).await;
                (verdict_name(&verdict), devin_render(verdict))
            }
        },
        HookTarget::Cursor => match cursor_normalize(input, deps.cwd) {
            None => ("allow", cursor_respond("allow", None)),
            Some(request) => {
                // Rust の `evaluate` は error を外部へ伝播しない
                // (計測失敗は内部で allow)が、TS の try/catch allow
                // 経路と同じく verdict に関わらず schema-valid JSON を
                // 返すことは変えない。
                let verdict = evaluate(&request, &deps.eval_opts()).await;
                (verdict_name(&verdict), cursor_render(verdict))
            }
        },
    };
    // `cheaper.hook` 完了 event(design §instrumentation 配置)—
    // `verdict` 分類値 + 出力 byte 長のみ(deny reason・payload 本文は
    // metadata-only 規約で載せない)。
    tracing::debug!(
        verdict = verdict,
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
