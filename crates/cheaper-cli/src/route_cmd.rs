//! `cheaper route resolve` — route match + viability の dry-run
//! utility command(design §route-resolve CLI、R11)。workflow・
//! 外部 process は一切実行しない — `routes::load`/`load_policy`
//! の file read と `route::resolve` 純粋評価のみ。
//!
//! - utility command 規約: `record_usage` は dispatch 側も含め非呼
//!   出し(`policy`/`hook-config` と同一)。
//! - `--command`/`--argv` は相互排他(parse 層で保証)。`--command`
//!   は `tokenize_shell` 済み token 列を argv とし、複数 segment・
//!   unsafe segment/token は `unsafe_command` flag として resolve
//!   へ渡す(unsafe-command viability へ写像)。`--argv` は JSON
//!   string array で、同一の env-prefix 正規化は `route::resolve`
//!   側が担う。

use cheaper_core::errors::CheaperError;
use serde_json::{json, Map, Value};
use tracing::Instrument;

use crate::args::{RouteAction, RouteCliArgs};
use crate::cli::{BoxError, CliHooks, CliIo, CommandOut};
use crate::envelope::utility_success_envelope;
use crate::errors::invalid_request;
use crate::hook::routing_policy::tokenize_shell;
use crate::route::resolve::{self, ResolveInput};
use crate::route::supply;

/// `route` command 本体。`cheaper.cli.command` span(utility
/// command instrumentation 規約)。
pub enum RouteOutput {
    Utility(CommandOut),
    Run(crate::route::run::RouteRunOutput),
}

pub async fn execute(
    args: &RouteCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<RouteOutput, BoxError> {
    execute_inner(args, io, hooks, started_at)
        .instrument(tracing::info_span!(
            "cheaper.cli.command",
            command = "route",
        ))
        .await
}

async fn execute_inner(
    args: &RouteCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<RouteOutput, BoxError> {
    match &args.action {
        RouteAction::Resolve { command, argv, cwd } => {
            execute_resolve(command, argv, cwd, io, hooks, started_at)
                .await
                .map(RouteOutput::Utility)
        }
        RouteAction::Logs { .. } => crate::route::logs::execute(args, io, hooks, started_at)
            .await
            .map(RouteOutput::Utility),
        RouteAction::Run { payload, cwd } => {
            crate::route::run::execute(payload, cwd.as_deref(), args.quiet, io, hooks, started_at)
                .await
                .map(RouteOutput::Run)
        }
    }
}

async fn execute_resolve(
    command: &Option<String>,
    argv: &Option<String>,
    cwd: &Option<String>,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    // 入力 argv 決定 — `--command` は tokenize、 `--argv` は JSON
    // array parse。env-prefix の strip は `route::resolve` 側で
    // 行う(2 入力形で規約が乖離しない)。
    let (argv_tokens, unsafe_command) = match (command, argv) {
        (Some(command), None) => parse_command(command)?,
        (None, Some(argv)) => (parse_argv(argv)?, false),
        _ => unreachable!("parse 層で相互排他を保証済み"),
    };

    // effective routes + command policy — hook eval・route run と
    // 同一 load 経路(`route::supply` が builtin 解決・root
    // canonicalize・config dir 解決・denylist 構築を一括で担う)。
    let supply = supply::load(
        cwd.as_deref().unwrap_or(&io.cwd),
        io.env,
        hooks.builtin_dir.as_deref(),
    )?;
    let view = supply.command_view();
    let resolution = resolve::resolve(&ResolveInput {
        argv: &argv_tokens,
        cwd: &supply.root,
        routes: &supply.routes,
        command_policy: &view,
        unsafe_command,
    });

    // R11 output shape — match 無しは route/target/grant/viability
    // を載せない。
    let mut result = Map::new();
    result.insert("matched".to_string(), json!(resolution.matched));
    if let Some(route) = &resolution.route {
        result.insert("route".to_string(), json!(route));
    }
    if let Some(target) = &resolution.target {
        result.insert("target".to_string(), json!(target));
    }
    if let Some(grant) = &resolution.grant {
        result.insert("grant".to_string(), json!(grant));
    }
    if let Some(executable) = &resolution.executable {
        result.insert("executable".to_string(), json!(executable));
    }
    result.insert("argv".to_string(), json!(resolution.argv));
    result.insert("args".to_string(), json!(resolution.args));
    if let Some(viability) = resolution.viability {
        result.insert("viability".to_string(), json!(viability.as_str()));
    }
    if let Some(suppressed) = &resolution.suppressed {
        result.insert("suppressed".to_string(), json!(suppressed));
    }

    Ok(CommandOut {
        envelope: utility_success_envelope(result, crate::cli::now(hooks) - started_at),
        exit_code: 0,
        model_line: None,
    })
}

/// `--command` — shell tokenize → first segment token 列 +
/// unsafe flag。tokenize 失敗(quote 不整合等)は `unsafe_command`
/// として resolve へ渡し fail-safe に倒す(argv は空 →
/// 分類自体は no-match にならないよう token は尽力抽出する)。
fn parse_command(command: &str) -> Result<(Vec<String>, bool), CheaperError> {
    match tokenize_shell(command) {
        Some(segments) => {
            let unsafe_command = segments.len() != 1
                || segments
                    .iter()
                    .any(|s| s.unsafe_segment || s.tokens.iter().any(|t| t.unsafe_token));
            let argv = segments
                .first()
                .map(|s| s.tokens.iter().map(|t| t.text.clone()).collect())
                .unwrap_or_default();
            Ok((argv, unsafe_command))
        }
        // tokenize 不能 — token 抽出不能でも unsafe として報告できる
        // よう raw 文字列 1 token を渡す(match しないため結果は
        // no-match + unsafe flag は viability 未到達で落ちるが、
        // parse error で拒否するより dry-run の report 性を優先)。
        None => Ok((vec![command.to_string()], true)),
    }
}

/// `--argv` — JSON string array のみ受理。
fn parse_argv(raw: &str) -> Result<Vec<String>, CheaperError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|_| invalid_request("invalid --argv: malformed JSON", None))?;
    let Some(items) = value.as_array() else {
        return Err(invalid_request(
            "invalid --argv: expected a JSON array of strings",
            None,
        ));
    };
    items
        .iter()
        .map(|v| {
            v.as_str().map(str::to_string).ok_or_else(|| {
                invalid_request("invalid --argv: expected a JSON array of strings", None)
            })
        })
        .collect()
}
