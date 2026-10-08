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
fn alias_worker(command: &str) -> Option<&'static str> {
    match command {
        "bulk-read" => Some("bulk-reader"),
        "code-write" => Some("code-writer"),
        "doc-write" => Some("doc-writer"),
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
pub struct UsageCliArgs {
    /// `--since` の生文字列。cli 層で `parse_duration` する。
    pub since: Option<String>,
    pub worker: Option<String>,
    pub provider: Option<String>,
    /// `<provider>/<id>` 形式。
    pub model: Option<String>,
    /// `<provider>/<id>` 形式。estimated 算出の comparison model。
    pub compare_model: Option<String>,
    pub format: CliFormat,
    pub quiet: bool,
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
            Self::WorkflowInterface(a) => Some(a.format),
            Self::WorkflowRun(a) => Some(a.format),
            Self::Auth(a) => Some(a.format),
            Self::Help | Self::Version | Self::Models(_) => None,
        }
    }

    /// `p.quiet`(format() と同じ kind 条件)。
    pub fn quiet(&self) -> Option<bool> {
        match self {
            Self::Run(a) => Some(a.quiet),
            Self::CountTokens(a) => Some(a.quiet),
            Self::Usage(a) => Some(a.quiet),
            Self::WorkflowInterface(a) => Some(a.quiet),
            Self::WorkflowRun(a) => Some(a.quiet),
            Self::Auth(a) => Some(a.quiet),
            Self::Help | Self::Version | Self::Models(_) => None,
        }
    }
}

fn invalid_request(
    message: impl Into<String>,
    details: Option<Map<String, serde_json::Value>>,
) -> CheaperError {
    let e = CheaperError::new(ErrorCode::InvalidRequest, message.into());
    match details {
        Some(d) => e.with_details(d),
        None => e,
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
    "format",
];

/// boolean option(`type: "boolean"`)。
const BOOL_OPTIONS: &[&str] = &["quiet", "help", "version"];

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
    format: String,
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
            format: "json".to_string(),
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
            "format" => self.format = value,
            _ => unreachable!("set_string called with non-string option"),
        }
    }

    fn set_bool(&mut self, name: &str) {
        match name {
            "quiet" => self.quiet = true,
            "help" => self.help = true,
            "version" => self.version = true,
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
            "command required: run | bulk-read | code-write | doc-write | count-tokens | usage | models | auth | workflow",
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
            &["input", "cwd", "timeout", "format", "quiet"]
        } else {
            &["cwd", "format", "quiet"]
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
            return Ok(ParsedCli::WorkflowRun(WorkflowRunCliArgs {
                path: positionals[2].clone(),
                input: if option_tokens.iter().any(|t| t.name == "input") {
                    Some(values.input.clone())
                } else {
                    None
                },
                cwd: values.cwd.clone(),
                timeout: values.timeout.clone(),
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

    // --model/--provider/--worker は run/models と意味を共有するため
    // usage 専用には含めない(TS コメント通り)。
    let usage_only_option_used = values.since.is_some() || values.compare_model.is_some();

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
        if values.timeout.is_some() || values.input != "-" {
            return Err(invalid_request(
                "--timeout/--input are only valid for worker runs",
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
            format,
            quiet,
        }));
    }

    if usage_only_option_used {
        return Err(invalid_request(
            "--since/--compare-model are only valid for `usage`",
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

    Ok(ParsedCli::Run(RunCliArgs {
        worker_name,
        worker_file,
        input: values.input,
        cwd: values.cwd,
        timeout: values.timeout,
        model,
        format,
        quiet,
    }))
}
