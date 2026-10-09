//! Port of `packages/cli/src/cli.ts` — `runCli` dispatch 本体。
//!
//! T2 で骨組み + count-tokens/help/version を実装。run/models/usage/
//! auth/workflow/hook の各 command module は T3–T7 が本体を埋める
//! seam(`commands::*::execute`)として切り出してある。
//!
//! dispatch の契約:
//! - `argv[0] === "hook"` は envelope/recordUsage/汎用 catch の対象外
//!   (fail-open の別 contract)。
//! - 通常 command は stdout に envelope 1 枚のみ(`--format text` 時は
//!   text output)。
//! - 汎用 catch は `errorEnvelope` + stderr diagnostic + best-effort
//!   recordUsage を再現する。
//! - `__grep-eval` はここでは扱わない — `main.rs` の argv 先頭一致で
//!   envelope 外に早期分岐する。

use std::error::Error;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirsOptions, EnvLookup};
use cheaper_core::errors::{CheaperError, ErrorCode, NodeIoError};
use cheaper_core::token::count::FileTooLargeError;
use cheaper_core::workspace::paths::WorkspaceEscapeError;
use serde_json::Value;

use crate::args::{parse_cli_args, CliFormat, ParsedCli};
use crate::count::count_workspace_file_tokens;
use crate::embedded;
use crate::envelope::{envelope_json, error_envelope, utility_success_envelope};
use crate::text_format::format_text;

/// 全 command 経路の error 畳み込み型(`Box<dyn Error + Send + Sync>`)。
pub type BoxError = Box<dyn Error + Send + Sync + 'static>;

/// `StdinLike` — TTY 判定 + 全 bytes 読取り。
pub trait CliStdin {
    fn is_tty(&self) -> bool;
    /// stdin を EOF まで読む。TTY 時の呼出しは呼び出し側が避ける。
    fn read_all(&mut self) -> io::Result<Vec<u8>>;
}

/// `stdout`/`stderr` の `write(s)` 相当。
pub trait CliWrite {
    fn write_str(&mut self, s: &str);
}

/// `CliIo` — command 実装が触れる process 境界。
pub struct CliIo<'a> {
    pub stdin: &'a mut dyn CliStdin,
    pub stdout: &'a mut dyn CliWrite,
    pub stderr: &'a mut dyn CliWrite,
    pub env: EnvLookup<'a>,
    pub cwd: String,
}

/// `CliHooks` — 単体テスト用の差し替え seam。
/// `run_worker`/`create_model_runtime`/`open_url` は T3/T5 が組み立てる
/// adapter 経由の型と一緒に追加する。
#[derive(Default)]
pub struct CliHooks {
    /// built-in worker/policy の親 dir(既定: `embedded::builtin_dir`)。
    pub builtin_dir: Option<PathBuf>,
    /// `Date.now` 差し替え(ms epoch)。`Arc` — workflow run の
    /// engine `now` option が `Arc<dyn Fn>` を要求するため(T7)。
    pub now: Option<Arc<dyn Fn() -> f64 + Send + Sync>>,
    /// 外部 abort(テスト用。本番は SIGINT/SIGTERM 中継)。
    pub signal: Option<cheaper_tools::operation::AbortSignal>,
}

/// `hooks.now` 未指定時の時刻源(workflow engine の `now` option
/// 既定値としても参照 — T7)。
pub(crate) fn system_now() -> f64 {
    // `Date.now()` は整数 ms — sub-ms を保持すると envelope の
    // `durationMs` が小数化し byte parity を崩すため truncate する。
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// `Date.now` hook 経由の時刻源(command module 共有 — T3 以降も利用)。
pub(crate) fn now(hooks: &CliHooks) -> f64 {
    hooks.now.as_deref().map(|f| f()).unwrap_or_else(system_now)
}

fn now_fn<'h>(hooks: &'h CliHooks) -> impl Fn() -> f64 + 'h {
    move || now(hooks)
}

/// hooks 未指定時の builtinDir(embedded resource の materialize root)。
/// command module(run/models/…)も同一経路で解決する。
pub(crate) fn resolve_builtin_dir(hooks: &CliHooks) -> Result<PathBuf, BoxError> {
    match &hooks.builtin_dir {
        Some(d) => Ok(d.clone()),
        None => embedded::builtin_dir().map_err(BoxError::from),
    }
}

/// catalog composition/persistence error の user-facing surface
/// (spec Req 4)。各 error を `warning: <msg>\n` 行として sink へ流す
/// (`usage-log:`/`model:` と同じ stderr diagnostic 系)。`--quiet` で
/// 抑制。envelope `result` shape・exit code は不変。
///
/// sink は `warning: <msg>\n` の行を受け取る closure —
/// `auth.rs::execute_with` は `CliWrite` を持たず
/// `terminal: &mut dyn AuthTerminal` しか受け取らないため、
/// `io.stderr.write_str` / `AuthTerminal::write` の双方を同一 helper
/// で流せるよう `&mut dyn FnMut(&str)` とする。
pub(crate) fn emit_catalog_warnings(errors: &[String], sink: &mut dyn FnMut(&str), quiet: bool) {
    if quiet {
        return;
    }
    for msg in errors {
        sink(&format!("warning: {msg}\n"));
    }
}

/// `resolvedUserConfigDir` — executeRun 途中 throw 時の recordUsage
/// 用 fallback(resolveConfigDirs は純粋関数で userConfigDir は env
/// のみに依存)。
fn resolved_user_config_dir(
    io: &CliIo<'_>,
    hooks: &CliHooks,
    run_root: Option<&str>,
    run_user_config_dir: Option<&str>,
) -> String {
    if let Some(d) = run_user_config_dir {
        return d.to_string();
    }
    let cwd = run_root.unwrap_or(&io.cwd).to_string();
    resolve_builtin_dir(hooks)
        .map(|builtin| {
            resolve_config_dirs(&ConfigDirsOptions {
                cwd: std::path::Path::new(&cwd),
                builtin_dir: &builtin,
                env: Some(io.env),
                homedir: None,
            })
            .user_config_dir
            .to_string_lossy()
            .into_owned()
        })
        .unwrap_or_default()
}

/// command module の戻り値 — envelope + exit code + optional stderr
/// model line。
pub struct CommandOut {
    pub envelope: Value,
    pub exit_code: i32,
    /// run 成功時の `model: <prov>/<id> (tier <t>)` stderr line。
    pub model_line: Option<String>,
}

/// `runCli`(DESIGN-0005)。戻り値は process exit code。
pub async fn run_cli(argv: &[String], io: &mut CliIo<'_>, hooks: &CliHooks) -> i32 {
    let started_at = now(hooks);
    // `hook` は envelope / recordUsage / 汎用 catch の対象外:
    // target 別ネイティブ contract + fail-open(DESIGN-034z1EhuztlLSb2x2XXxgD)。
    if argv.first().map(String::as_str) == Some("hook") {
        return crate::hook::execute_hook(&argv[1..], io, hooks).await;
    }

    // dispatch が mutably borrow する state — error path の
    // recordUsage / worker 名補完で参照する。
    let mut ctx = RunCtx::default();
    match dispatch(argv, io, hooks, started_at, &mut ctx).await {
        Ok(code) => code,
        Err(err) => {
            let env = error_envelope(
                err.as_ref(),
                ctx.worker_name.as_deref(),
                Some(now(hooks) - started_at),
            );
            let (mut format, mut quiet, mut suppress) = (CliFormat::Json, false, false);
            if let Ok(p) = parse_cli_args(argv) {
                if let Some(f) = p.format() {
                    format = f;
                }
                if let Some(q) = p.quiet() {
                    quiet = q;
                }
                suppress = matches!(p, ParsedCli::WorkflowRun(ref a) if a.quiet);
            }
            if !suppress {
                let code = env
                    .get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(Value::as_str)
                    .unwrap_or("INTERNAL_ERROR");
                let message = env
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                io.stderr.write_str(&format!("error {code}: {message}\n"));
            }
            let out = match format {
                CliFormat::Text => format_text(&env),
                CliFormat::Json => envelope_json(&env),
            };
            io.stdout.write_str(&out);
            let root = ctx.run_root.clone().unwrap_or_else(|| io.cwd.clone());
            let ucd = resolved_user_config_dir(
                io,
                hooks,
                ctx.run_root.as_deref(),
                ctx.run_user_config_dir.as_deref(),
            );
            crate::usage_log::record_usage(&env, &root, &ucd, io.stderr, quiet, &now_fn(hooks));
            env.get("error")
                .and_then(|e| e.get("exitCode"))
                .and_then(Value::as_i64)
                .unwrap_or(70) as i32
        }
    }
}

/// dispatch 内で run 経路が埋める context(error path の記録用)。
#[derive(Default)]
pub struct RunCtx {
    pub worker_name: Option<String>,
    pub run_root: Option<String>,
    pub run_user_config_dir: Option<String>,
}

/// dispatch 本体 — parsed command 別に command module へ委譲し、
/// stdout 出力 + recordUsage を処理する。
async fn dispatch(
    argv: &[String],
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
    ctx: &mut RunCtx,
) -> Result<i32, BoxError> {
    let parsed = parse_cli_args(argv)?;
    match parsed {
        ParsedCli::Help => {
            io.stdout.write_str(HELP);
            Ok(0)
        }
        ParsedCli::Version => {
            let builtin = resolve_builtin_dir(hooks)?;
            let v = embedded::package_version(&builtin).map_err(BoxError::from)?;
            io.stdout.write_str(&format!("{v}\n"));
            Ok(0)
        }
        ParsedCli::CountTokens(args) => {
            // LLM/model/worker 解決を伴わない測定専用経路(DESIGN-0012)。
            // effective policy から filesystem.symlinks のみ使う。
            let builtin = resolve_builtin_dir(hooks)?;
            let counted = count_workspace_file_tokens(
                args.cwd.as_deref().unwrap_or(&io.cwd),
                &args.path,
                &builtin,
                io.env,
            )
            .map_err(|e| map_count_error(e, &args.path))?;
            // FileTokenCount → `{path, tokens, bytes, tokenizer:
            // {name, implementation, version}}`(declaration 順)。
            let c = &counted.count;
            let mut tokenizer = serde_json::Map::new();
            tokenizer.insert("name".to_string(), Value::from(c.tokenizer.name));
            tokenizer.insert(
                "implementation".to_string(),
                Value::from(c.tokenizer.implementation),
            );
            tokenizer.insert("version".to_string(), Value::from(c.tokenizer.version));
            let mut result = serde_json::Map::new();
            result.insert("path".to_string(), Value::from(c.path.clone()));
            result.insert("tokens".to_string(), Value::from(c.tokens));
            result.insert("bytes".to_string(), Value::from(c.bytes));
            result.insert("tokenizer".to_string(), Value::Object(tokenizer));
            let envelope = utility_success_envelope(result, now(hooks) - started_at);
            write_output(io, &envelope, args.format);
            crate::usage_log::record_usage(
                &envelope,
                &counted.root,
                &counted.user_config_dir.to_string_lossy(),
                io.stderr,
                args.quiet,
                &now_fn(hooks),
            );
            Ok(0)
        }
        ParsedCli::Usage(args) => {
            let builtin = resolve_builtin_dir(hooks)?;
            let out =
                crate::usage_report::execute_usage(&args, io, hooks, &builtin, started_at).await?;
            write_output(io, &out.envelope, args.format);
            Ok(out.exit_code)
        }
        ParsedCli::Models(args) => {
            let out = crate::models::execute(&args, io, hooks, started_at).await?;
            write_output(io, &out.envelope, args.format);
            Ok(out.exit_code)
        }
        ParsedCli::WorkflowInterface(args) => {
            let out = crate::workflow_run::execute_interface(&args, io, hooks, started_at).await?;
            write_output(io, &out.envelope, args.format);
            Ok(out.exit_code)
        }
        ParsedCli::WorkflowRun(args) => {
            let out = crate::workflow_run::execute_run(&args, io, hooks, started_at).await?;
            write_output(io, &out.envelope, args.format);
            // workflow-run は envelope.ok / error.exitCode が exit code
            // を決める(TS `envelope.ok ? 0 : (envelope.error?.exitCode ?? 70)`)。
            let code = if out.envelope.get("ok").and_then(Value::as_bool) == Some(true) {
                0
            } else {
                out.envelope
                    .get("error")
                    .and_then(|e| e.get("exitCode"))
                    .and_then(Value::as_i64)
                    .unwrap_or(70) as i32
            };
            Ok(code)
        }
        ParsedCli::Auth(args) => {
            let out = crate::auth::execute(&args, io, hooks, started_at).await?;
            write_output(io, &out.envelope, args.format);
            Ok(out.exit_code)
        }
        ParsedCli::Run(args) => {
            let out = crate::run::execute(&args, io, hooks, started_at, ctx).await?;
            if let Some(line) = &out.model_line {
                if !args.quiet {
                    io.stderr.write_str(&format!("{line}\n"));
                }
            }
            write_output(io, &out.envelope, args.format);
            let root = ctx
                .run_root
                .clone()
                .or_else(|| args.cwd.clone())
                .unwrap_or_else(|| io.cwd.clone());
            let ucd = resolved_user_config_dir(
                io,
                hooks,
                ctx.run_root.as_deref(),
                ctx.run_user_config_dir.as_deref(),
            );
            crate::usage_log::record_usage(
                &out.envelope,
                &root,
                &ucd,
                io.stderr,
                args.quiet,
                &now_fn(hooks),
            );
            Ok(out.exit_code)
        }
    }
}

/// `parsed.format === "text" ? formatText : envelopeJson` の書出し。
fn write_output(io: &mut CliIo<'_>, env: &Value, format: CliFormat) {
    let s = match format {
        CliFormat::Text => format_text(env),
        CliFormat::Json => envelope_json(env),
    };
    io.stdout.write_str(&s);
}

/// count-tokens 分岐の error 写像(cli.ts の `.catch` 相当)。
/// `FileTooLargeError` → FILE_TOO_LARGE、`WorkspaceEscapeError` →
/// INVALID_REQUEST、`ENOENT` → INVALID_REQUEST "file not found"。
fn map_count_error(e: BoxError, arg_path: &str) -> BoxError {
    if let Some(ftl) = e.downcast_ref::<FileTooLargeError>() {
        let mut details = serde_json::Map::new();
        details.insert("path".to_string(), serde_json::json!(ftl.path()));
        details.insert("bytes".to_string(), serde_json::json!(ftl.bytes()));
        details.insert("maxBytes".to_string(), serde_json::json!(ftl.max_bytes()));
        let err = CheaperError::new(ErrorCode::FileTooLarge, ftl.to_string()).with_details(details);
        // TS は `cause: e` で元 error を保持 — envelope には
        // CheaperError cause しか出ないので emit はされないが chain は保持。
        return Box::new(err);
    }
    if let Some(esc) = e.downcast_ref::<WorkspaceEscapeError>() {
        let mut details = serde_json::Map::new();
        details.insert("path".to_string(), serde_json::json!(arg_path));
        return Box::new(
            CheaperError::new(ErrorCode::InvalidRequest, esc.to_string()).with_details(details),
        );
    }
    if let Some(io_err) = e.downcast_ref::<NodeIoError>() {
        if io_err.code() == "ENOENT" {
            let mut details = serde_json::Map::new();
            details.insert("path".to_string(), serde_json::json!(arg_path));
            return Box::new(
                CheaperError::new(
                    ErrorCode::InvalidRequest,
                    format!("file not found: {arg_path}"),
                )
                .with_details(details),
            );
        }
    }
    e
}

/// HELP — cli.ts の HELP literal をそのまま移植(indent 保存のため
/// raw string)。
pub const HELP: &str = r"Usage: cheaper <command> [options]

Commands:
  run <worker>           Run a named worker (or --worker-file <path>)
  bulk-read              Alias for: run bulk-reader
  code-write             Alias for: run code-writer
  doc-write              Alias for: run doc-writer
  count-tokens <path>    Count canonical tokens of a workspace file
  workflow interface <path> Show the public workflow inputs and outputs
  workflow run <path>       Run a one-shot workflow
  usage                  Aggregate usage.jsonl (worker cost / measured savings)
  models                 List available models with tier and pricing info
  auth status [provider] Show stored credentials and provider readiness
  hook <event>          Agent hook entry (internal; --target <agent>)
  auth login <provider>  Sign in to a provider (OAuth; e.g. openai-codex)
  auth logout <provider> Remove a provider credential

Options:
  --input <file|->       Task input JSON (default: stdin)
  --worker-file <path>   Run an explicit worker file (run only)
  --cwd <path>           Workspace root (default: process.cwd())
  --timeout <duration>   Narrow policy timeout (e.g. 30s, 5m)
  --model <prov>/<id>    run: explicit model; usage: filter by model
  --provider <id>        models/usage: filter by provider
  --tier <low|med|high>  Filter 'models' output by effective tier
  --worker <name>        Rank 'models' auto tiers by a worker's workload
  --method <id>          Auto-answer auth login select prompts (e.g. device)
  --type <oauth|api_key> Auth method for 'auth login' (default: oauth)
  --since <duration>     usage: only records within the period (e.g. 24h)
  --worker <name>        usage: filter by worker name
  --compare-model <p/id> usage: estimate savings vs this model
  --format <json|text>   Output format (default: json)
  --quiet                Suppress stderr diagnostics
  --help                 Show this help
  --version              Show version
";
