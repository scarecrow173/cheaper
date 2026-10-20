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
use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};
use cheaper_core::workspace::deny_paths::create_path_denylist;
use serde_json::{json, Map, Value};
use tracing::Instrument;

use crate::args::{RouteAction, RouteCliArgs};
use crate::cli::{BoxError, CliHooks, CliIo, CommandOut};
use crate::compose::resolve_workspace_layout;
use crate::envelope::utility_success_envelope;
use crate::errors::invalid_request;
use crate::hook::routing_policy::tokenize_shell;
use crate::route::resolve::{self, CommandPolicyView, ResolveInput};
use crate::route::routes::{self, RouteLoadOptions};

/// `route` command 本体。`cheaper.cli.command` span(utility
/// command instrumentation 規約)。
pub async fn execute(
    args: &RouteCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
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
) -> Result<CommandOut, BoxError> {
    let RouteAction::Resolve { command, argv, cwd } = &args.action;

    // 入力 argv 決定 — `--command` は tokenize、 `--argv` は JSON
    // array parse。env-prefix の strip は `route::resolve` 側で
    // 行う(2 入力形で規約が乖離しない)。
    let (argv_tokens, unsafe_command) = match (command, argv) {
        (Some(command), None) => parse_command(command)?,
        (None, Some(argv)) => (parse_argv(argv)?, false),
        _ => unreachable!("parse 層で相互排他を保証済み"),
    };

    // workspace preamble — builtin dir 解決 + root canonicalize +
    // config dir 解決(run/workflow と同一 sequence)。
    let layout = resolve_workspace_layout(cwd.as_deref().or(Some(&io.cwd)), io, hooks)?;

    // effective route set — hook eval・route run と同一 loader。
    let effective = routes::load(&RouteLoadOptions {
        builtin_dir: &layout.builtin_dir,
        user_config_dir: &layout.dirs.user_config_dir,
        project_dir: &layout.dirs.project_dir,
    })?;

    // effective command policy — run と同一 load 経路(invocation
    // layer 無し。route resolve は hook 評価と同じ policy 面を見る)。
    let builtin_policy_path = layout.builtin_dir.join("policy.yaml");
    let ucd = layout.dirs.user_config_dir.to_string_lossy();
    let pdir = layout.dirs.project_dir.to_string_lossy();
    let policy = load_policy(&LoadPolicyOptions {
        builtin_policy_path: &builtin_policy_path.to_string_lossy(),
        user_config_dir: &ucd,
        project_dir: &pdir,
        invocation: None,
    })?;
    let deny: Vec<&str> = policy
        .filesystem
        .deny_paths
        .iter()
        .map(String::as_str)
        .collect();
    let denylist = create_path_denylist(&deny);
    let view = CommandPolicyView {
        policy: &policy.command,
        root: &layout.root,
        symlink_policy: policy.filesystem.symlinks,
        denylist: Some(&denylist),
    };
    let resolution = resolve::resolve(&ResolveInput {
        argv: &argv_tokens,
        cwd: &layout.root,
        routes: &effective,
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
