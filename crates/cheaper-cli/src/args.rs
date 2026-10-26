//! Port of `packages/cli/src/args.ts` — `node:util.parseArgs`
//! (strict + allowPositionals + tokens) 相当の argv 解析。
//!
//! parseArgs 意味論(Node v26 で実測済み):
//! - `--name value` / `--name=value`。`--` は option terminator(以降は
//!   全て positional)。`-` 単独は positional。
//! - string option に option-like な次 arg(`-` 始まりで長さ>1。`--`
//!   terminator や未知 option を含む)が続くと ambiguous error、次 arg
//!   が無ければ missing error。
//! - boolean option に `=value` を付けると "does not take an argument"。
//! - 未知 long option → `Unknown option '--x' ...`、未知 short は
//!   group 先頭 char で `Unknown option '-a' ...` を報告
//!   (defined short option は存在しない)。
//! - 同一 option の重複は last-wins。
//! - error は parse 順に最初に発生したものが surface する。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::model::model_ref::parse_model_ref;
use cheaper_core::model::tier::{Tier, TIERS};
use serde_json::{json, Map};

use crate::errors::invalid_request;

/// `NAME_PATTERN` = `/^[a-z0-9][a-z0-9-]*$/`(JS `\w` 非依存の ASCII
/// 限定 — byte 走査で正確に再現)。
fn is_valid_worker_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let first_ok = bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit();
    first_ok
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

/// `ALIAS_WORKERS`: `bulk-read` → `bulk-reader` 等の run alias 表。
pub(crate) fn alias_worker(command: &str) -> Option<&'static str> {
    match command {
        "bulk-read" => Some("bulk-reader"),
        "code-write" => Some("code-writer"),
        "doc-write" => Some("doc-writer"),
        "tool-operate" => Some("tool-operator"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliFormat {
    Json,
    Text,
}

impl CliFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Text => "text",
        }
    }
}

/// `auth login --type` の値。既定 `oauth`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthTypeArg {
    Oauth,
    ApiKey,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunCliArgs {
    /// named Worker。`--worker-file` 指定時は `None`。
    pub worker_name: Option<String>,
    /// `run` 専用の明示 Worker file。named resolution を bypass する。
    pub worker_file: Option<String>,
    /// `--input` 値。`-` は stdin。
    pub input: String,
    pub cwd: Option<String>,
    /// `--timeout` の生文字列。cli 層で `parse_duration` する。
    pub timeout: Option<String>,
    /// `--model <provider>/<id>` 直接指定。
    pub model: Option<String>,
    /// repeatable `--grant <json>` の宣言順値列 — invocation-scoped
    /// command grant(invoker 供給の非永続 grant)。
    pub grants: Vec<String>,
    /// `--resume <path>` — budget 枯渇 checkpoint からの再開
    /// (issue #111)。run/alias のみ受理。
    pub resume: Option<String>,
    /// `--input` が明示指定されたか(値 `-` の stdin 指定を含む)。
    /// resume 時の input 解決で「省略 → checkpoint stored input」を
    /// 区別するために `option_tokens` から採る。
    pub input_explicit: bool,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelsCliArgs {
    /// `--provider <id>` filter。
    pub provider: Option<String>,
    /// `--tier <low|medium|high>` filter(effective tier 基準)。
    pub tier: Option<Tier>,
    /// `--worker <name>`: auto 分類の基準 workload profile。
    pub worker: Option<String>,
    pub cwd: Option<String>,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CountTokensCliArgs {
    /// 測定対象 file(workspace 相対または絶対 path)。
    pub path: String,
    pub cwd: Option<String>,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowInterfaceCliArgs {
    pub path: String,
    pub cwd: Option<String>,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowRunCliArgs {
    pub path: String,
    /// `None` means `{}`; `-` means explicit stdin。
    pub input: Option<String>,
    pub cwd: Option<String>,
    pub timeout: Option<String>,
    /// repeatable `--grant <json>` の宣言順値列(workflow arm の
    /// `run` 側 allowlist のみが受理する)。
    pub grants: Vec<String>,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthCliArgs {
    /// `auth status|login|logout` の action。
    pub action: AuthAction,
    /// login/logout 必須、status では optional filter。
    pub provider: Option<String>,
    /// login の `select` prompt へ即答する option id(browser/device 等)。
    pub method: Option<String>,
    /// login の auth type。既定 oauth。
    pub auth_type: AuthTypeArg,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAction {
    Status,
    Login,
    Logout,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyCliArgs {
    /// `policy grant` の action。
    pub action: PolicyAction,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyAction {
    /// `policy grant add <name> <spec-json>` — spec は生 JSON 文字列
    /// (`command.grants` の map value shape。`name` key は含まない)。
    GrantAdd { name: String, spec: String },
    /// `policy grant list`。
    GrantList,
    /// `policy grant remove <name>`。
    GrantRemove { name: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct HookConfigCliArgs {
    /// `hook-config sync|verify|remove` の action。`--event-stdin`
    /// 指定時は positional action 不要(`None`)で、stdin の lifecycle
    /// event payload から action を決める。
    pub action: Option<HookConfigAction>,
    /// `project`(既定)|`user`。その他の値は実行時 error。
    pub scope: Option<String>,
    /// `--project-root <path>` — project scope の対象 root
    /// (既定は process cwd)。
    pub project_root: Option<String>,
    /// APM lifecycle event JSON を stdin から読む。
    pub event_stdin: bool,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookConfigAction {
    Sync,
    Verify,
    Remove,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteCliArgs {
    /// `route resolve` の action。
    pub action: RouteAction,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteAction {
    /// `route resolve` — dry-run のみ(workflow・外部 process は
    /// 実行しない)。入力は `--command`/`--argv` のどちらか 1 つ。
    Resolve {
        /// `--command <string>` — shell command 文字列(tokenize 後
        /// env-prefix strip で正規化)。
        command: Option<String>,
        /// `--argv <json-array>` — argv 列の JSON array
        /// (string のみ。同一の env-prefix 正規化を受ける)。
        argv: Option<String>,
        /// `--cwd` — workspace root 解決の base。
        cwd: Option<String>,
    },
    Logs {
        id: String,
        offset: u64,
        max_bytes: Option<u64>,
        cwd: Option<String>,
    },
    Run {
        payload: String,
        cwd: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageCliArgs {
    /// `--since` の生文字列。cli 層で `parse_duration` する。
    pub since: Option<String>,
    pub worker: Option<String>,
    pub provider: Option<String>,
    /// `<provider>/<id>` 形式。
    pub model: Option<String>,
    /// `<provider>/<id>` 形式。estimated 算出の comparison model。
    pub compare_model: Option<String>,
    /// `--group-by` の未検証値。validation は usage dispatch が担う。
    pub group_by: Option<String>,
    /// route aggregation のみを絞る route 名。
    pub route: Option<String>,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryCliArgs {
    pub action: HistoryAction,
    pub format: CliFormat,
    pub quiet: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryAction {
    List {
        since: Option<String>,
        worker: Option<String>,
        kind: Option<String>,
        status: Option<String>,
        limit: Option<String>,
        cursor: Option<String>,
    },
    Show {
        execution_id: String,
    },
    Trace {
        trace_id: String,
        limit: Option<String>,
        cursor: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedCli {
    Help,
    Version,
    Run(RunCliArgs),
    CountTokens(CountTokensCliArgs),
    WorkflowInterface(WorkflowInterfaceCliArgs),
    WorkflowRun(WorkflowRunCliArgs),
    Auth(AuthCliArgs),
    Models(ModelsCliArgs),
    Usage(UsageCliArgs),
    History(HistoryCliArgs),
    Policy(PolicyCliArgs),
    /// `hook-config` — `hook` が早期 dispatch されるため別 top-level
    /// command として分離する(ADR-034H2WBNtC4We8dyC5lQZJ)。
    HookConfig(HookConfigCliArgs),
    /// `route` — route resolve/run の route 層 utility command。
    Route(RouteCliArgs),
}

impl ParsedCli {
    /// error path での `--format`/`--quiet` 再読み用(TS: catch 内の
    /// `parseCliArgs` 再試行 → `p.format`/`p.quiet`/`workflow-run &&
    /// quiet` の抑止判定)。`--format text` が有効な kind のみ値を返す。
    pub fn format(&self) -> Option<CliFormat> {
        match self {
            Self::Run(a) => Some(a.format),
            Self::CountTokens(a) => Some(a.format),
            Self::Usage(a) => Some(a.format),
            Self::History(a) => Some(a.format),
            Self::WorkflowInterface(a) => Some(a.format),
            Self::WorkflowRun(a) => Some(a.format),
            Self::Auth(a) => Some(a.format),
            Self::Policy(a) => Some(a.format),
            Self::HookConfig(a) => Some(a.format),
            Self::Route(a) => Some(a.format),
            Self::Help | Self::Version | Self::Models(_) => None,
        }
    }

    /// `p.quiet`(format() と同じ kind 条件)。
    pub fn quiet(&self) -> Option<bool> {
        match self {
            Self::Run(a) => Some(a.quiet),
            Self::CountTokens(a) => Some(a.quiet),
            Self::Usage(a) => Some(a.quiet),
            Self::History(a) => Some(a.quiet),
            Self::WorkflowInterface(a) => Some(a.quiet),
            Self::WorkflowRun(a) => Some(a.quiet),
            Self::Auth(a) => Some(a.quiet),
            Self::Policy(a) => Some(a.quiet),
            Self::HookConfig(a) => Some(a.quiet),
            Self::Route(a) => Some(a.quiet),
            Self::Help | Self::Version | Self::Models(_) => None,
        }
    }
}

fn invalid_worker_name(name: &str) -> CheaperError {
    let mut details = Map::new();
    details.insert("name".to_string(), json!(name));
    CheaperError::new(
        ErrorCode::InvalidWorkerName,
        format!("invalid worker name: {name}"),
    )
    .with_details(details)
}

/// `policy grant add|remove <name>` の parse 段検査 — grant
/// `propertyNames`(`^[a-z][a-z0-9_-]{0,63}$`)と同一規則を
/// `grant::is_valid_grant_name` で共有する。
fn check_policy_grant_name(name: &str) -> Result<(), CheaperError> {
    if crate::grant::is_valid_grant_name(name) {
        return Ok(());
    }
    Err(invalid_request(
        format!("invalid grant name: {name}"),
        Some(Map::from_iter([("grant".to_string(), json!(name))])),
    ))
}

// ---------------------------------------------------------------------------
// parseArgs 相当層
// ---------------------------------------------------------------------------

/// `parsed.tokens` の option token(name のみ契約に使用)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct OptionToken {
    name: String,
}

/// string option の parseArgs `options` 表(TS declaration 順)。
const STRING_OPTIONS: &[&str] = &[
    "input",
    "worker-file",
    "cwd",
    "timeout",
    "method",
    "type",
    "model",
    "provider",
    "tier",
    "since",
    "worker",
    "compare-model",
    "group-by",
    "route",
    "kind",
    "status",
    "limit",
    "cursor",
    "format",
    // repeatable — last-wins 規約からの deliberate deviation
    // (`set_string` の `"grant"` arm 参照)。
    "grant",
    // `hook-config` 専用(`hook-config` arm 以外は reject)。
    "scope",
    "project-root",
    // `run`/worker alias 専用 — budget 枯渇 checkpoint からの再開
    // (issue #111)。非 run command arm は field 検査で reject する。
    "resume",
    // `route resolve` 専用 — 入力は `--command`/`--argv` の
    // どちらか 1 つ。他 command arm は reject。
    "command",
    "argv",
    "offset",
    "max-bytes",
    "payload",
    // observability activation(T2)の global option。値は `Values`
    // field に保持せず、消費は `global_log_flags` が argv scan で
    // 担う(parse 済み値と activation 値の二重表現を避ける)。
    "log",
    "log-format",
];

/// boolean option(`type: "boolean"`)。
const BOOL_OPTIONS: &[&str] = &["quiet", "help", "version", "event-stdin"];

/// `values` — parseArgs の `values` object 相当。string option は
/// `Option`、default 付き(`input`-`"-"`・`format`-`"json"`)は
/// 解決済み値を持つ。
#[derive(Debug)]
struct Values {
    input: String,
    worker_file: Option<String>,
    cwd: Option<String>,
    timeout: Option<String>,
    method: Option<String>,
    auth_type: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    tier: Option<String>,
    since: Option<String>,
    worker: Option<String>,
    compare_model: Option<String>,
    group_by: Option<String>,
    route: Option<String>,
    kind: Option<String>,
    status: Option<String>,
    limit: Option<String>,
    cursor: Option<String>,
    format: String,
    /// repeatable `--grant` の宣言順蓄積。`Vec` は default-empty の
    /// 受け皿なので、値を消費しない非 run command arm は
    /// `!values.grants.is_empty()` の reject 検査を持つ — silent
    /// drop 防止規約。新規 command arm を追加する場合も同じ検査を
    /// 置くこと(run / workflow run 以外は全て reject)。
    grants: Vec<String>,
    resume: Option<String>,
    scope: Option<String>,
    project_root: Option<String>,
    /// `route resolve` の `--command`/`--argv` 値。
    command: Option<String>,
    argv: Option<String>,
    offset: Option<String>,
    max_bytes: Option<String>,
    payload: Option<String>,
    event_stdin: bool,
    quiet: bool,
    help: bool,
    version: bool,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            input: "-".to_string(),
            worker_file: None,
            cwd: None,
            timeout: None,
            method: None,
            auth_type: None,
            model: None,
            provider: None,
            tier: None,
            since: None,
            worker: None,
            compare_model: None,
            group_by: None,
            route: None,
            kind: None,
            status: None,
            limit: None,
            cursor: None,
            format: "json".to_string(),
            grants: Vec::new(),
            resume: None,
            scope: None,
            project_root: None,
            command: None,
            argv: None,
            offset: None,
            max_bytes: None,
            payload: None,
            event_stdin: false,
            quiet: false,
            help: false,
            version: false,
        }
    }
}

impl Values {
    fn set_string(&mut self, name: &str, value: String) {
        match name {
            "input" => self.input = value,
            "worker-file" => self.worker_file = Some(value),
            "cwd" => self.cwd = Some(value),
            "timeout" => self.timeout = Some(value),
            "method" => self.method = Some(value),
            "type" => self.auth_type = Some(value),
            "model" => self.model = Some(value),
            "provider" => self.provider = Some(value),
            "tier" => self.tier = Some(value),
            "since" => self.since = Some(value),
            "worker" => self.worker = Some(value),
            "compare-model" => self.compare_model = Some(value),
            "group-by" => self.group_by = Some(value),
            "route" => self.route = Some(value),
            "kind" => self.kind = Some(value),
            "status" => self.status = Some(value),
            "limit" => self.limit = Some(value),
            "cursor" => self.cursor = Some(value),
            "format" => self.format = value,
            // `--grant` は repeatable — 本 file の冒頭に文書化された
            // last-wins 規約からの deliberate deviation(Node `parseArgs`
            // の `multiple: true` 相当)。宣言順を保持するため push で畳む。
            "grant" => self.grants.push(value),
            "resume" => self.resume = Some(value),
            "scope" => self.scope = Some(value),
            "project-root" => self.project_root = Some(value),
            "command" => self.command = Some(value),
            "argv" => self.argv = Some(value),
            "offset" => self.offset = Some(value),
            "max-bytes" => self.max_bytes = Some(value),
            "payload" => self.payload = Some(value),
            // 受理のみで値は捨てる — activation 入力は
            // `global_log_flags` が argv から直接 scan する。
            "log" | "log-format" => {}
            _ => unreachable!("set_string called with non-string option"),
        }
    }

    fn set_bool(&mut self, name: &str) {
        match name {
            "quiet" => self.quiet = true,
            "help" => self.help = true,
            "version" => self.version = true,
            "event-stdin" => self.event_stdin = true,
            _ => unreachable!("set_bool called with non-bool option"),
        }
    }
}

/// Node `isOptionLikeValue`: `-` 始まりで長さ>1(`-` 単独・空文字は
/// positional/value として扱われる)。`--` terminator も option-like。
fn is_option_like(value: &str) -> bool {
    value.len() > 1 && value.starts_with('-')
}

fn parse_args_error(message: String) -> CheaperError {
    invalid_request(
        format!("invalid arguments: {message}"),
        Some(Map::from_iter([(
            "reason".to_string(),
            json!("parse-args"),
        )])),
    )
}

fn unknown_option(raw: &str) -> CheaperError {
    parse_args_error(format!(
        "Unknown option '{raw}'. To specify a positional argument starting with a '-', \
place it at the end of the command after '--', as in '-- \"{raw}\"'"
    ))
}

/// `--grant` flag 値の byte 上限(64KiB)— cmdline 長制限と無制限
/// 入力への sanity bound。populate 段で enforce する(非 run command
/// では `!values.grants.is_empty()` の reject が先に発火するため、
/// 「その command では使えない」旨の error が size error より優先
/// される)。
fn check_grant_value_bounds(grants: &[String]) -> Result<(), CheaperError> {
    const MAX_GRANT_VALUE_BYTES: usize = 64 * 1024;
    if grants.iter().any(|g| g.len() > MAX_GRANT_VALUE_BYTES) {
        return Err(invalid_request(
            "--grant value exceeds the 64KiB limit",
            None,
        ));
    }
    Ok(())
}

/// 非 run command arm 共通の `--grant` reject — `Values.grants` の
/// 規約 comment(「非 run command は grants 非空検査を持つ」)を満たす
/// 各 arm の呼出し点。
fn reject_grant_flag() -> CheaperError {
    invalid_request("--grant is only valid for `run` and `workflow run`", None)
}

fn parse_js_safe_u64(raw: &str, flag: &str) -> Result<u64, CheaperError> {
    let parsed = (!raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()))
        .then(|| raw.parse::<u64>().ok())
        .flatten();
    match parsed {
        Some(value) if value <= cheaper_core::js::MAX_SAFE_INTEGER_I64 as u64 => Ok(value),
        _ => Err(invalid_request(
            format!("invalid --{flag}: expected a non-negative integer"),
            None,
        )),
    }
}

struct ParseOutcome {
    values: Values,
    positionals: Vec<String>,
    option_tokens: Vec<OptionToken>,
}

/// `parseArgs({strict: true, allowPositionals: true, tokens: true, options})`
/// 相当。error message は Node の文言を再現する(`invalid arguments:` prefix
/// は呼び出し側の catch 相当としてここで付ける)。
fn parse_args_safe(argv: &[String]) -> Result<ParseOutcome, CheaperError> {
    let mut values = Values::default();
    let mut positionals = Vec::new();
    let mut option_tokens = Vec::new();
    let mut i = 0;
    let mut terminated = false;
    while i < argv.len() {
        let arg = &argv[i];
        if terminated {
            positionals.push(arg.clone());
            i += 1;
            continue;
        }
        if arg == "--" {
            terminated = true;
            i += 1;
            continue;
        }
        if arg.len() > 2 && arg.starts_with("--") {
            // long option: `--name` / `--name=value`。
            let body = &arg[2..];
            let (name, inline_value) = match body.find('=') {
                Some(eq) => (&body[..eq], Some(body[eq + 1..].to_string())),
                None => (body, None),
            };
            if BOOL_OPTIONS.contains(&name) {
                if inline_value.is_some() {
                    return Err(parse_args_error(format!(
                        "Option '--{name}' does not take an argument"
                    )));
                }
                values.set_bool(name);
                option_tokens.push(OptionToken {
                    name: name.to_string(),
                });
                i += 1;
                continue;
            }
            if !STRING_OPTIONS.contains(&name) {
                return Err(unknown_option(arg));
            }
            let value = match inline_value {
                Some(v) => v,
                None => match argv.get(i + 1) {
                    None => {
                        return Err(parse_args_error(format!(
                            "Option '--{name} <value>' argument missing"
                        )));
                    }
                    Some(next) if is_option_like(next) => {
                        return Err(parse_args_error(format!(
                            "Option '--{name}' argument is ambiguous.\n\
Did you forget to specify the option argument for '--{name}'?\n\
To specify an option argument starting with a dash use '--{name}=-XYZ'."
                        )));
                    }
                    Some(next) => {
                        i += 1;
                        next.clone()
                    }
                },
            };
            values.set_string(name, value);
            option_tokens.push(OptionToken {
                name: name.to_string(),
            });
            i += 1;
            continue;
        }
        if arg.len() > 1 && arg.starts_with('-') {
            // short option group — defined short は存在しないため先頭
            // char で unknown error。JS は UTF-16 unit を取るが、astral
            // char の surrogate 分割は非契約 message の隅なので char を使う。
            let first = arg[1..].chars().next().expect("len > 1");
            return Err(unknown_option(&format!("-{first}")));
        }
        positionals.push(arg.clone());
        i += 1;
    }
    Ok(ParseOutcome {
        values,
        positionals,
        option_tokens,
    })
}

// ---------------------------------------------------------------------------
// parseCliArgs 本体
// ---------------------------------------------------------------------------

/// `parseCliArgs`(DESIGN-0005)。command 別の option 許可表・alias・
/// worker name 検証を `args.ts` の分類・文言規則で port する。
pub fn parse_cli_args(argv: &[String]) -> Result<ParsedCli, CheaperError> {
    let parsed = parse_args_safe(argv)?;
    let ParseOutcome {
        values,
        positionals,
        option_tokens,
    } = parsed;

    if values.help {
        return Ok(ParsedCli::Help);
    }
    if values.version {
        return Ok(ParsedCli::Version);
    }

    let format = match values.format.as_str() {
        "json" => CliFormat::Json,
        "text" => CliFormat::Text,
        other => {
            return Err(invalid_request(
                format!("invalid --format: {other}"),
                Some(Map::from_iter([("format".to_string(), json!(other))])),
            ));
        }
    };
    let quiet = values.quiet;

    let Some(command) = positionals.first() else {
        return Err(invalid_request(
            "command required: run | bulk-read | code-write | doc-write | tool-operate | count-tokens | usage | history | models | auth | workflow | policy | hook-config | route",
            None,
        ));
    };

    let worker_file = values.worker_file.clone();
    if command == "workflow" {
        let action = positionals.get(1).map(String::as_str);
        if (action != Some("interface") && action != Some("run"))
            || positionals.len() != 3
            || positionals[2].is_empty()
        {
            return Err(invalid_request(
                "workflow requires interface|run <path>",
                None,
            ));
        }
        let allowed: &[&str] = if action == Some("run") {
            &[
                "input",
                "cwd",
                "timeout",
                "grant",
                "format",
                "quiet",
                "log",
                "log-format",
            ]
        } else {
            &["cwd", "format", "quiet", "log", "log-format"]
        };
        for token in &option_tokens {
            if !allowed.contains(&token.name.as_str()) {
                return Err(invalid_request(
                    format!(
                        "--{} is not valid for workflow {}",
                        token.name,
                        action.unwrap_or("")
                    ),
                    None,
                ));
            }
        }
        if action == Some("run") {
            check_grant_value_bounds(&values.grants)?;
            return Ok(ParsedCli::WorkflowRun(WorkflowRunCliArgs {
                path: positionals[2].clone(),
                input: if option_tokens.iter().any(|t| t.name == "input") {
                    Some(values.input.clone())
                } else {
                    None
                },
                cwd: values.cwd.clone(),
                timeout: values.timeout.clone(),
                grants: values.grants.clone(),
                format,
                quiet,
            }));
        }
        return Ok(ParsedCli::WorkflowInterface(WorkflowInterfaceCliArgs {
            path: positionals[2].clone(),
            cwd: values.cwd.clone(),
            format,
            quiet,
        }));
    }

    if command == "history" {
        let action = positionals.get(1).map(String::as_str);
        let valid_shape = match action {
            Some("list") => positionals.len() == 2,
            Some("show" | "trace") => positionals.len() == 3 && !positionals[2].is_empty(),
            _ => false,
        };
        if !valid_shape {
            return Err(invalid_request(
                "history requires list | show <execution-uuid> | trace <trace-uuid>",
                None,
            ));
        }
        let allowed: &[&str] = match action {
            Some("list") => &[
                "since",
                "worker",
                "kind",
                "status",
                "limit",
                "cursor",
                "format",
                "quiet",
                "log",
                "log-format",
            ],
            Some("show") => &["format", "quiet", "log", "log-format"],
            Some("trace") => &["limit", "cursor", "format", "quiet", "log", "log-format"],
            _ => unreachable!("valid history action checked above"),
        };
        for token in &option_tokens {
            if !allowed.contains(&token.name.as_str()) {
                return Err(invalid_request(
                    format!(
                        "--{} is not valid for `history {}`",
                        token.name,
                        action.unwrap()
                    ),
                    None,
                ));
            }
        }
        if !values.grants.is_empty() {
            return Err(reject_grant_flag());
        }
        let history_format = if option_tokens.iter().any(|token| token.name == "format") {
            format
        } else {
            CliFormat::Text
        };
        let action = match action {
            Some("list") => HistoryAction::List {
                since: values.since.clone(),
                worker: values.worker.clone(),
                kind: values.kind.clone(),
                status: values.status.clone(),
                limit: values.limit.clone(),
                cursor: values.cursor.clone(),
            },
            Some("show") => HistoryAction::Show {
                execution_id: positionals[2].clone(),
            },
            Some("trace") => HistoryAction::Trace {
                trace_id: positionals[2].clone(),
                limit: values.limit.clone(),
                cursor: values.cursor.clone(),
            },
            _ => unreachable!("valid history action checked above"),
        };
        return Ok(ParsedCli::History(HistoryCliArgs {
            action,
            format: history_format,
            quiet,
        }));
    }

    // --model/--provider/--worker は run/models と意味を共有するため
    // usage 専用には含めない(TS コメント通り)。
    let usage_only_option_used = values.since.is_some()
        || values.compare_model.is_some()
        || values.group_by.is_some()
        || values.route.is_some();
    // `route resolve` 専用 option — `route` arm 以外で値を持つ
    // 場合は reject する(field 検査規約。`workflow`/`hook-config`/
    // `policy` の allowlist 走査が先に発火するため、この guard は
    // field 検査式の arm — usage/count-tokens/models/auth/run —
    // に対して効く)。
    let route_only_option_used = values.command.is_some()
        || values.argv.is_some()
        || values.offset.is_some()
        || values.max_bytes.is_some()
        || values.payload.is_some();

    if command == "route" {
        // `route resolve` のみ受理する utility arm(B2)。shape は
        // `route resolve` の 2 positional に限定し、将来 arm
        // (`run`/`logs`)は別 task で追加する — 未実装 action 名は
        // unknown として reject する。
        let action = positionals.get(1).map(String::as_str);
        let valid_shape = match action {
            Some("resolve") | Some("run") => positionals.len() == 2,
            Some("logs") => positionals.len() == 3 && !positionals[2].is_empty(),
            _ => false,
        };
        if !valid_shape {
            return Err(invalid_request(
                "route requires resolve | run | logs <id>",
                None,
            ));
        }
        // flag allowlist — `--command`/`--argv`/`--cwd` と共通の
        // format/quiet/log のみ受理する。
        let allowed: &[&str] = match action {
            Some("run") => &["payload", "cwd", "format", "quiet", "log", "log-format"],
            Some("logs") => &[
                "offset",
                "max-bytes",
                "cwd",
                "format",
                "quiet",
                "log",
                "log-format",
            ],
            _ => &[
                "command",
                "argv",
                "cwd",
                "format",
                "quiet",
                "log",
                "log-format",
            ],
        };
        for token in &option_tokens {
            if !allowed.contains(&token.name.as_str()) {
                return Err(invalid_request(
                    format!(
                        "--{} is not valid for `route {}`",
                        token.name,
                        action.unwrap_or("")
                    ),
                    None,
                ));
            }
        }
        if !values.grants.is_empty() {
            return Err(reject_grant_flag());
        }
        if action == Some("run") {
            let payload = values.payload.clone().unwrap_or_default();
            if payload.is_empty() {
                return Err(invalid_request("route run requires --payload", None));
            }
            return Ok(ParsedCli::Route(RouteCliArgs {
                action: RouteAction::Run {
                    payload,
                    cwd: values.cwd.clone(),
                },
                format,
                quiet,
            }));
        }
        if action == Some("logs") {
            let offset = match values.offset.as_deref() {
                Some(raw) => parse_js_safe_u64(raw, "offset")?,
                None => 0,
            };
            let max_bytes = match values.max_bytes.as_deref() {
                Some(raw) => {
                    let value = parse_js_safe_u64(raw, "max-bytes")?;
                    if value == 0 {
                        return Err(invalid_request(
                            "invalid --max-bytes: expected a positive integer",
                            None,
                        ));
                    }
                    Some(value)
                }
                None => None,
            };
            return Ok(ParsedCli::Route(RouteCliArgs {
                action: RouteAction::Logs {
                    id: positionals[2].clone(),
                    offset,
                    max_bytes,
                    cwd: values.cwd.clone(),
                },
                format,
                quiet,
            }));
        }
        // `--command`/`--argv` は相互排他でどちらか 1 つ必須。
        if values.command.is_some() == values.argv.is_some() {
            return Err(invalid_request(
                "route resolve requires exactly one of --command or --argv",
                None,
            ));
        }
        return Ok(ParsedCli::Route(RouteCliArgs {
            action: RouteAction::Resolve {
                command: values.command.clone(),
                argv: values.argv.clone(),
                cwd: values.cwd.clone(),
            },
            format,
            quiet,
        }));
    }

    if route_only_option_used {
        return Err(invalid_request(
            "--command/--argv/--offset/--max-bytes/--payload are only valid for `route` subcommands",
            None,
        ));
    }

    if command == "usage" {
        if positionals.len() > 1 {
            return Err(invalid_request(
                format!("unexpected arguments: {}", positionals[1..].join(" ")),
                None,
            ));
        }
        if worker_file.is_some() {
            return Err(invalid_request(
                "--worker-file is only valid for `run`",
                None,
            ));
        }
        // 規約: 非 run command は grants 非空検査を持つ(silent drop 防止)。
        if !values.grants.is_empty() {
            return Err(reject_grant_flag());
        }
        if values.kind.is_some()
            || values.status.is_some()
            || values.limit.is_some()
            || values.cursor.is_some()
        {
            return Err(invalid_request(
                "--kind/--status/--limit/--cursor are only valid for `history`",
                None,
            ));
        }
        if values.timeout.is_some() || values.input != "-" {
            return Err(invalid_request(
                "--timeout/--input are only valid for worker runs",
                None,
            ));
        }
        if values.resume.is_some() {
            return Err(invalid_request(
                "--resume is only valid for worker runs",
                None,
            ));
        }
        if values.method.is_some() || values.auth_type.is_some() {
            return Err(invalid_request(
                "--method/--type are only valid for `auth login`",
                None,
            ));
        }
        if values.tier.is_some() {
            return Err(invalid_request("--tier is only valid for `models`", None));
        }
        return Ok(ParsedCli::Usage(UsageCliArgs {
            since: values.since.clone(),
            worker: values.worker.clone(),
            provider: values.provider.clone(),
            model: values.model.clone(),
            compare_model: values.compare_model.clone(),
            group_by: values.group_by.clone(),
            route: values.route.clone(),
            format,
            quiet,
        }));
    }

    if values.kind.is_some()
        || values.status.is_some()
        || values.limit.is_some()
        || values.cursor.is_some()
    {
        return Err(invalid_request(
            "--kind/--status/--limit/--cursor are only valid for `history`",
            None,
        ));
    }

    if usage_only_option_used {
        return Err(invalid_request(
            "--since/--compare-model/--group-by/--route are only valid for `usage`",
            None,
        ));
    }

    if command == "count-tokens" {
        if positionals.len() > 2 {
            return Err(invalid_request(
                format!("unexpected arguments: {}", positionals[2..].join(" ")),
                None,
            ));
        }
        let Some(target) = positionals.get(1) else {
            return Err(invalid_request("count-tokens requires <path>", None));
        };
        if worker_file.is_some() {
            return Err(invalid_request(
                "--worker-file is only valid for `run`",
                None,
            ));
        }
        // 規約: 非 run command は grants 非空検査を持つ(silent drop 防止)。
        if !values.grants.is_empty() {
            return Err(reject_grant_flag());
        }
        if values.timeout.is_some() {
            return Err(invalid_request(
                "--timeout is only valid for worker runs",
                None,
            ));
        }
        if values.input != "-" {
            return Err(invalid_request(
                "--input is only valid for worker runs",
                None,
            ));
        }
        if values.resume.is_some() {
            return Err(invalid_request(
                "--resume is only valid for worker runs",
                None,
            ));
        }
        if values.method.is_some() || values.auth_type.is_some() {
            return Err(invalid_request(
                "--method/--type are only valid for `auth login`",
                None,
            ));
        }
        if values.model.is_some() {
            return Err(invalid_request(
                "--model is only valid for worker runs or `usage`",
                None,
            ));
        }
        if values.provider.is_some() || values.worker.is_some() {
            return Err(invalid_request(
                "--provider/--worker are only valid for `models` or `usage`",
                None,
            ));
        }
        if values.tier.is_some() {
            return Err(invalid_request("--tier is only valid for `models`", None));
        }
        return Ok(ParsedCli::CountTokens(CountTokensCliArgs {
            path: target.clone(),
            cwd: values.cwd.clone(),
            format,
            quiet,
        }));
    }

    if command == "models" {
        if positionals.len() > 1 {
            return Err(invalid_request(
                format!("unexpected arguments: {}", positionals[1..].join(" ")),
                None,
            ));
        }
        if worker_file.is_some() {
            return Err(invalid_request(
                "--worker-file is only valid for `run`",
                None,
            ));
        }
        // 規約: 非 run command は grants 非空検査を持つ(silent drop 防止)。
        if !values.grants.is_empty() {
            return Err(reject_grant_flag());
        }
        if values.model.is_some() {
            return Err(invalid_request(
                "--model is only valid for worker runs or `usage`",
                None,
            ));
        }
        if values.timeout.is_some() {
            return Err(invalid_request(
                "--timeout is only valid for worker runs",
                None,
            ));
        }
        if values.input != "-" {
            return Err(invalid_request(
                "--input is only valid for worker runs",
                None,
            ));
        }
        if values.resume.is_some() {
            return Err(invalid_request(
                "--resume is only valid for worker runs",
                None,
            ));
        }
        if values.method.is_some() || values.auth_type.is_some() {
            return Err(invalid_request(
                "--method/--type are only valid for `auth login`",
                None,
            ));
        }
        let tier = match values.tier.as_deref() {
            None => None,
            Some(t) if TIERS.iter().any(|tier| tier.as_str() == t) => Some(match t {
                "low" => Tier::Low,
                "medium" => Tier::Medium,
                _ => Tier::High,
            }),
            Some(t) => {
                return Err(invalid_request(
                    format!(
                        "invalid --tier: {t} (expected {})",
                        TIERS
                            .iter()
                            .map(|t| t.as_str())
                            .collect::<Vec<_>>()
                            .join("|")
                    ),
                    Some(Map::from_iter([("tier".to_string(), json!(t))])),
                ));
            }
        };
        if let Some(worker) = values.worker.as_deref() {
            if !is_valid_worker_name(worker) {
                return Err(invalid_worker_name(worker));
            }
        }
        return Ok(ParsedCli::Models(ModelsCliArgs {
            provider: values.provider.clone(),
            tier,
            worker: values.worker.clone(),
            cwd: values.cwd.clone(),
            format,
            quiet,
        }));
    }

    if command == "auth" {
        if worker_file.is_some() {
            return Err(invalid_request(
                "--worker-file is only valid for `run`",
                None,
            ));
        }
        // 規約: 非 run command は grants 非空検査を持つ(silent drop 防止)。
        if !values.grants.is_empty() {
            return Err(reject_grant_flag());
        }
        if values.timeout.is_some() {
            return Err(invalid_request(
                "--timeout is only valid for worker runs",
                None,
            ));
        }
        if values.input != "-" {
            return Err(invalid_request(
                "--input is only valid for worker runs",
                None,
            ));
        }
        if values.resume.is_some() {
            return Err(invalid_request(
                "--resume is only valid for worker runs",
                None,
            ));
        }
        let action = match positionals.get(1).map(String::as_str) {
            Some("status") => AuthAction::Status,
            Some("login") => AuthAction::Login,
            Some("logout") => AuthAction::Logout,
            other => {
                return Err(invalid_request(
                    format!(
                        "unknown auth action: {} (expected status|login|logout)",
                        other.unwrap_or("undefined")
                    ),
                    None,
                ));
            }
        };
        if positionals.len() > 3 {
            return Err(invalid_request(
                format!("unexpected arguments: {}", positionals[3..].join(" ")),
                None,
            ));
        }
        let provider = positionals.get(2).cloned();
        if provider.is_none() && action != AuthAction::Status {
            return Err(invalid_request(
                format!(
                    "auth {} requires <provider>",
                    match action {
                        AuthAction::Status => "status",
                        AuthAction::Login => "login",
                        AuthAction::Logout => "logout",
                    }
                ),
                None,
            ));
        }
        if values.method.is_some() && action != AuthAction::Login {
            return Err(invalid_request(
                "--method is only valid for `auth login`",
                None,
            ));
        }
        if let Some(t) = values.auth_type.as_deref() {
            if action != AuthAction::Login {
                return Err(invalid_request(
                    "--type is only valid for `auth login`",
                    None,
                ));
            }
            if t != "oauth" && t != "api_key" {
                return Err(invalid_request(
                    format!("invalid --type: {t} (expected oauth|api_key)"),
                    Some(Map::from_iter([("type".to_string(), json!(t))])),
                ));
            }
        }
        if values.model.is_some() {
            return Err(invalid_request(
                "--model is only valid for worker runs or `usage`",
                None,
            ));
        }
        if values.provider.is_some() || values.worker.is_some() {
            return Err(invalid_request(
                "--provider/--worker are only valid for `models` or `usage`",
                None,
            ));
        }
        if values.tier.is_some() {
            return Err(invalid_request("--tier is only valid for `models`", None));
        }
        return Ok(ParsedCli::Auth(AuthCliArgs {
            action,
            provider,
            method: values.method.clone(),
            auth_type: match values.auth_type.as_deref() {
                Some("api_key") => AuthTypeArg::ApiKey,
                _ => AuthTypeArg::Oauth,
            },
            format,
            quiet,
        }));
    }

    if command == "hook-config" {
        // utility command の flag allowlist — `hook-config` 専用
        // option(`scope`/`project-root`/`event-stdin`)と共通の
        // format/quiet/log のみ受理する。
        for token in &option_tokens {
            if ![
                "scope",
                "project-root",
                "event-stdin",
                "format",
                "quiet",
                "log",
                "log-format",
            ]
            .contains(&token.name.as_str())
            {
                return Err(invalid_request(
                    format!("--{} is not valid for `hook-config`", token.name),
                    None,
                ));
            }
        }
        // 規約: 非 run command は grants 非空検査を持つ
        // (`--grant` は上の allowlist で既に reject されるが、
        // 規約の explicit 検査として残す)。
        if !values.grants.is_empty() {
            return Err(reject_grant_flag());
        }
        let action = match positionals.get(1).map(String::as_str) {
            Some("sync") => Some(HookConfigAction::Sync),
            Some("verify") => Some(HookConfigAction::Verify),
            Some("remove") => Some(HookConfigAction::Remove),
            Some(other) => {
                return Err(invalid_request(
                    format!("unknown hook-config action: {other} (expected sync|verify|remove)"),
                    None,
                ));
            }
            None => None,
        };
        if positionals.len() > 2 {
            return Err(invalid_request(
                format!("unexpected arguments: {}", positionals[2..].join(" ")),
                None,
            ));
        }
        if action.is_none() && !values.event_stdin {
            return Err(invalid_request(
                "hook-config requires sync|verify|remove (or --event-stdin)",
                None,
            ));
        }
        if action.is_some() && values.event_stdin {
            return Err(invalid_request(
                "--event-stdin cannot be combined with an action \
                 (the lifecycle event selects the action)",
                None,
            ));
        }
        return Ok(ParsedCli::HookConfig(HookConfigCliArgs {
            action,
            scope: values.scope.clone(),
            project_root: values.project_root.clone(),
            event_stdin: values.event_stdin,
            format,
            quiet,
        }));
    }

    if command == "policy" {
        // `policy grant add <name> <spec-json>`(4)/`list`(2)/
        // `remove <name>`(3)以外の形状は受理しない。grant name は
        // add/remove とも `^[a-z][a-z0-9_-]{0,63}$`(grant
        // `propertyNames` と同一)を parse 段で検査する。
        let sub = &positionals[1..];
        let action = match sub {
            [cmd, verb, name, spec] if cmd == "grant" && verb == "add" => {
                check_policy_grant_name(name)?;
                PolicyAction::GrantAdd {
                    name: name.clone(),
                    spec: spec.clone(),
                }
            }
            [cmd, verb] if cmd == "grant" && verb == "list" => PolicyAction::GrantList,
            [cmd, verb, name] if cmd == "grant" && verb == "remove" => {
                check_policy_grant_name(name)?;
                PolicyAction::GrantRemove { name: name.clone() }
            }
            _ => {
                return Err(invalid_request(
                    "policy requires grant add <name> <spec>|list|remove <name>",
                    None,
                ))
            }
        };
        // utility command の flag allowlist — `--grant`・`--worker-file`・
        // `--cwd`・`--input`・`--timeout` 等他 option は全て reject
        // する(workflow arm と同型の option_tokens 走査。`Values.grants`
        // の「非 run command は grants 非空検査を持つ」規約はこの
        // allowlist が包含する)。
        for token in &option_tokens {
            if !["format", "quiet", "log", "log-format"].contains(&token.name.as_str()) {
                return Err(invalid_request(
                    format!("--{} is not valid for `policy`", token.name),
                    None,
                ));
            }
        }
        return Ok(ParsedCli::Policy(PolicyCliArgs {
            action,
            format,
            quiet,
        }));
    }

    let worker_name: Option<String> = if command == "run" {
        if positionals.len() > 2 {
            return Err(invalid_request(
                format!("unexpected arguments: {}", positionals[2..].join(" ")),
                None,
            ));
        }
        let name = positionals.get(1).cloned();
        if name.is_some() && worker_file.is_some() {
            return Err(invalid_request(
                "--worker-file cannot be combined with a worker name",
                None,
            ));
        }
        if name.is_none() && worker_file.is_none() {
            return Err(invalid_request(
                "run requires <worker> or --worker-file",
                None,
            ));
        }
        name
    } else if let Some(alias) = alias_worker(command) {
        if positionals.len() > 1 {
            return Err(invalid_request(
                format!("unexpected arguments: {}", positionals[1..].join(" ")),
                None,
            ));
        }
        if worker_file.is_some() {
            return Err(invalid_request(
                "--worker-file is only valid for `run`",
                None,
            ));
        }
        Some(alias.to_string())
    } else {
        return Err(invalid_request(
            format!("unknown command: {command}"),
            Some(Map::from_iter([("command".to_string(), json!(command))])),
        ));
    };

    if values.method.is_some() || values.auth_type.is_some() {
        return Err(invalid_request(
            "--method/--type are only valid for `auth login`",
            None,
        ));
    }
    if values.provider.is_some() || values.worker.is_some() {
        return Err(invalid_request(
            "--provider/--worker are only valid for `models` or `usage`",
            None,
        ));
    }
    if values.tier.is_some() {
        return Err(invalid_request("--tier is only valid for `models`", None));
    }

    if let Some(name) = worker_name.as_deref() {
        if !is_valid_worker_name(name) {
            return Err(invalid_worker_name(name));
        }
    }

    let model = values.model.clone();
    if let Some(m) = model.as_deref() {
        if parse_model_ref(m).is_none() {
            return Err(invalid_request(
                format!("invalid --model reference: {m} (expected <provider>/<id>)"),
                Some(Map::from_iter([("model".to_string(), json!(m))])),
            ));
        }
    }

    check_grant_value_bounds(&values.grants)?;
    Ok(ParsedCli::Run(RunCliArgs {
        worker_name,
        worker_file,
        input: values.input,
        cwd: values.cwd,
        timeout: values.timeout,
        model,
        grants: values.grants,
        resume: values.resume,
        // `--input`(値 `-` の stdin 指定を含む)の明示有無 — resume
        // 時の「checkpoint stored input へ fallback するか」の判定に
        // 使う(workflow arm と同じ option_tokens presence 規約)。
        input_explicit: option_tokens.iter().any(|t| t.name == "input"),
        format,
        quiet,
    }))
}

// ---------------------------------------------------------------------------
// lenient global-option scan(init 前の activation 入力用)
// ---------------------------------------------------------------------------
//
// `run_cli` が `parse_cli_args` より前に呼ぶ軽量 accessor 群。全て同一の
// scan 規約を共有する:
// - 先頭の `--` terminator で打ち切り(以降の positional は flag と
//   して扱わない)
// - string option は `--name value` / `--name=value` の両形を受理し、
//   `--name` の次 arg は option-like でも無条件に値として消費する
// - 同一 option の重複は last-wins
//
// 値の検証・missing/ambiguous 判定は後続の strict parse が担うため、
// ここでは best-effort の取得のみ行い error は返さない。

/// string option 用の lenient scan 実体。`global_log_flags`・
/// `global_string_flag`・`resolve_log_root` の `--cwd` 解決が共有する
/// 単一の scan 規約。
fn scan_string_flag(argv: &[String], name: &str) -> Option<String> {
    let mut value = None;
    let mut iter = argv.iter().map(String::as_str).take_while(|a| *a != "--");
    while let Some(arg) = iter.next() {
        let Some(body) = arg.strip_prefix("--") else {
            continue;
        };
        let matched = match body.split_once('=') {
            Some((n, v)) if n == name => Some(v.to_string()),
            Some(_) => continue,
            None if body == name => iter.next().map(str::to_string),
            None => continue,
        };
        if let Some(v) = matched {
            value = Some(v);
        }
    }
    value
}

/// `--log`/`--log-format` の lenient scan → `(level, format)`。
/// `run_cli` が observability init 前に呼び、値は生のまま
/// `Activation` に載せる(解釈は observability crate の責務)。
pub fn global_log_flags(argv: &[String]) -> (Option<String>, Option<String>) {
    (
        scan_string_flag(argv, "log"),
        scan_string_flag(argv, "log-format"),
    )
}

/// 任意の string option 用 lenient scan — `resolve_log_root` の
/// `--cwd` 解決など、strict parse 前に値が必要な global 用途向け。
/// scan 規約は `global_log_flags` と同一。
pub fn global_string_flag(argv: &[String], name: &str) -> Option<String> {
    scan_string_flag(argv, name)
}

/// bool option 版の lenient scan — `--<name>` 単独形のみ受理
/// (`--name=…` は strict parse が "does not take an argument" で
/// 弾くため値解釈は不要)。`argv.iter().any(== "--quiet")` の素朴
/// scan は `--` 以降の positional を誤検出するため、同じ `--`
/// terminator 規約で打ち切る。
pub fn global_flag_present(argv: &[String], name: &str) -> bool {
    let flag = format!("--{name}");
    argv.iter()
        .take_while(|a| a.as_str() != "--")
        .any(|a| a == &flag)
}
