//! `args.rs` の parse table test — `args.ts` の分類・文言規則に対応。

use cheaper_cli::args::{
    global_flag_present, global_log_flags, parse_cli_args, AuthAction, AuthTypeArg, CliFormat,
    ParsedCli,
};
use cheaper_core::errors::ErrorCode;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn err(argv: &[&str]) -> cheaper_core::errors::CheaperError {
    match parse_cli_args(&args(argv)) {
        Err(e) => e,
        Ok(p) => panic!("expected parse error, got {p:?}"),
    }
}

// --- help / version ----------------------------------------------------------

#[test]
fn help_and_version() {
    assert_eq!(parse_cli_args(&args(&["--help"])).unwrap(), ParsedCli::Help);
    assert_eq!(
        parse_cli_args(&args(&["--version"])).unwrap(),
        ParsedCli::Version
    );
    // help は他の option と同居しても help が勝つ
    assert_eq!(
        parse_cli_args(&args(&["--help", "run"])).unwrap(),
        ParsedCli::Help
    );
}

// --- command required / unknown ----------------------------------------------

#[test]
fn command_required() {
    let e = err(&[]);
    assert_eq!(e.code(), ErrorCode::InvalidRequest);
    assert!(e.message().starts_with("command required:"));
}

#[test]
fn unknown_command() {
    let e = err(&["frobnicate"]);
    assert_eq!(e.code(), ErrorCode::InvalidRequest);
    assert_eq!(e.message(), "unknown command: frobnicate");
    assert_eq!(
        e.details()
            .unwrap()
            .get("command")
            .unwrap()
            .as_str()
            .unwrap(),
        "frobnicate"
    );
}

// --- run --------------------------------------------------------------------

#[test]
fn run_named_worker() {
    let ParsedCli::Run(a) = parse_cli_args(&args(&["run", "bulk-reader"])).unwrap() else {
        panic!()
    };
    assert_eq!(a.worker_name.as_deref(), Some("bulk-reader"));
    assert_eq!(a.input, "-");
    assert_eq!(a.format, CliFormat::Json);
    assert!(!a.quiet);
}

#[test]
fn run_alias() {
    for (alias, worker) in [
        ("bulk-read", "bulk-reader"),
        ("code-write", "code-writer"),
        ("doc-write", "doc-writer"),
        ("tool-operate", "tool-operator"),
    ] {
        let ParsedCli::Run(a) = parse_cli_args(&args(&[alias])).unwrap() else {
            panic!()
        };
        assert_eq!(a.worker_name.as_deref(), Some(worker));
        // alias は extra positional を受理しない(canonical
        // invocation は `--input` / stdin JSON のみ)
        let e = err(&[alias, "extra"]);
        assert_eq!(e.message(), "unexpected arguments: extra");
    }
}

#[test]
fn run_worker_file() {
    let ParsedCli::Run(a) = parse_cli_args(&args(&["run", "--worker-file", "w.md"])).unwrap()
    else {
        panic!()
    };
    assert!(a.worker_name.is_none());
    assert_eq!(a.worker_file.as_deref(), Some("w.md"));
}

#[test]
fn run_worker_and_file_conflict() {
    let e = err(&["run", "w", "--worker-file", "w.md"]);
    assert_eq!(
        e.message(),
        "--worker-file cannot be combined with a worker name"
    );
}

#[test]
fn run_requires_worker() {
    let e = err(&["run"]);
    assert_eq!(e.message(), "run requires <worker> or --worker-file");
}

#[test]
fn run_invalid_worker_name() {
    let e = err(&["run", "Bad_Name"]);
    assert_eq!(e.code(), ErrorCode::InvalidWorkerName);
}

#[test]
fn run_options() {
    let ParsedCli::Run(a) = parse_cli_args(&args(&[
        "run",
        "w",
        "--input",
        "in.json",
        "--cwd",
        "/tmp/x",
        "--timeout",
        "30s",
        "--model",
        "openai/gpt-x",
        "--format",
        "text",
        "--quiet",
    ]))
    .unwrap() else {
        panic!()
    };
    assert_eq!(a.input, "in.json");
    assert_eq!(a.cwd.as_deref(), Some("/tmp/x"));
    assert_eq!(a.timeout.as_deref(), Some("30s"));
    assert_eq!(a.model.as_deref(), Some("openai/gpt-x"));
    assert_eq!(a.format, CliFormat::Text);
    assert!(a.quiet);
}

#[test]
fn run_invalid_model_ref() {
    let e = err(&["run", "w", "--model", "noslash"]);
    assert_eq!(
        e.message(),
        "invalid --model reference: noslash (expected <provider>/<id>)"
    );
}

#[test]
fn run_rejects_models_only_options() {
    let e = err(&["run", "w", "--provider", "openai"]);
    assert_eq!(
        e.message(),
        "--provider/--worker are only valid for `models` or `usage`"
    );
    let e = err(&["run", "w", "--tier", "low"]);
    assert_eq!(e.message(), "--tier is only valid for `models`");
}

// --- count-tokens ------------------------------------------------------------

#[test]
fn count_tokens() {
    let ParsedCli::CountTokens(a) = parse_cli_args(&args(&["count-tokens", "src/a.rs"])).unwrap()
    else {
        panic!()
    };
    assert_eq!(a.path, "src/a.rs");
}

#[test]
fn count_tokens_requires_path() {
    let e = err(&["count-tokens"]);
    assert_eq!(e.message(), "count-tokens requires <path>");
}

#[test]
fn count_tokens_rejects_run_options() {
    let e = err(&["count-tokens", "f", "--input", "x.json"]);
    assert_eq!(e.message(), "--input is only valid for worker runs");
    let e = err(&["count-tokens", "f", "--model", "a/b"]);
    assert_eq!(
        e.message(),
        "--model is only valid for worker runs or `usage`"
    );
}

// --- models ------------------------------------------------------------------

#[test]
fn models_defaults() {
    let ParsedCli::Models(a) = parse_cli_args(&args(&["models"])).unwrap() else {
        panic!()
    };
    assert!(a.provider.is_none());
    assert!(a.tier.is_none());
    assert!(a.worker.is_none());
}

#[test]
fn models_tier_validation() {
    let e = err(&["models", "--tier", "mega"]);
    assert!(e
        .message()
        .starts_with("invalid --tier: mega (expected low|medium|high)"));
}

#[test]
fn models_worker_name_validation() {
    let e = err(&["models", "--worker", "Bad"]);
    assert_eq!(e.code(), ErrorCode::InvalidWorkerName);
}

// --- auth --------------------------------------------------------------------

#[test]
fn auth_actions() {
    let ParsedCli::Auth(a) = parse_cli_args(&args(&["auth", "status"])).unwrap() else {
        panic!()
    };
    assert_eq!(a.action, AuthAction::Status);
    assert!(a.provider.is_none());

    let ParsedCli::Auth(a) = parse_cli_args(&args(&[
        "auth",
        "login",
        "openai-codex",
        "--method",
        "device",
    ]))
    .unwrap() else {
        panic!()
    };
    assert_eq!(a.action, AuthAction::Login);
    assert_eq!(a.provider.as_deref(), Some("openai-codex"));
    assert_eq!(a.method.as_deref(), Some("device"));
    assert_eq!(a.auth_type, AuthTypeArg::Oauth);
}

#[test]
fn auth_requires_provider_for_login() {
    let e = err(&["auth", "login"]);
    assert_eq!(e.message(), "auth login requires <provider>");
}

#[test]
fn auth_unknown_action() {
    let e = err(&["auth", "reset"]);
    assert_eq!(
        e.message(),
        "unknown auth action: reset (expected status|login|logout)"
    );
}

#[test]
fn auth_type_validation() {
    let e = err(&["auth", "login", "p", "--type", "bogus"]);
    assert_eq!(
        e.message(),
        "invalid --type: bogus (expected oauth|api_key)"
    );
    let ParsedCli::Auth(a) =
        parse_cli_args(&args(&["auth", "login", "p", "--type", "api_key"])).unwrap()
    else {
        panic!()
    };
    assert_eq!(a.auth_type, AuthTypeArg::ApiKey);
    // --method は login 専用
    let e = err(&["auth", "status", "--method", "x"]);
    assert_eq!(e.message(), "--method is only valid for `auth login`");
}

// --- usage -------------------------------------------------------------------

#[test]
fn usage_filters() {
    let ParsedCli::Usage(a) = parse_cli_args(&args(&[
        "usage",
        "--since",
        "24h",
        "--worker",
        "w",
        "--provider",
        "p",
        "--model",
        "p/m",
        "--compare-model",
        "p2/m2",
    ]))
    .unwrap() else {
        panic!()
    };
    assert_eq!(a.since.as_deref(), Some("24h"));
    assert_eq!(a.worker.as_deref(), Some("w"));
    assert_eq!(a.provider.as_deref(), Some("p"));
    assert_eq!(a.model.as_deref(), Some("p/m"));
    assert_eq!(a.compare_model.as_deref(), Some("p2/m2"));
}

#[test]
fn since_only_valid_for_usage() {
    let e = err(&["run", "w", "--since", "1h"]);
    assert_eq!(
        e.message(),
        "--since/--compare-model are only valid for `usage`"
    );
}

// --- workflow ----------------------------------------------------------------

#[test]
fn workflow_interface_and_run() {
    let ParsedCli::WorkflowInterface(a) =
        parse_cli_args(&args(&["workflow", "interface", "wf.json"])).unwrap()
    else {
        panic!()
    };
    assert_eq!(a.path, "wf.json");

    let ParsedCli::WorkflowRun(a) = parse_cli_args(&args(&[
        "workflow",
        "run",
        "wf.json",
        "--input",
        "in.json",
        "--timeout",
        "5m",
    ]))
    .unwrap() else {
        panic!()
    };
    assert_eq!(a.path, "wf.json");
    assert_eq!(a.input.as_deref(), Some("in.json"));
    assert_eq!(a.timeout.as_deref(), Some("5m"));

    // workflow interface は --input を拒否
    let e = err(&["workflow", "interface", "wf.json", "--input", "x"]);
    assert_eq!(e.message(), "--input is not valid for workflow interface");

    let e = err(&["workflow"]);
    assert_eq!(e.message(), "workflow requires interface|run <path>");
}

// --- parseArgs 意味論 ---------------------------------------------------------

#[test]
fn parse_args_semantics() {
    // `--name=value` 形式
    let ParsedCli::Run(a) = parse_cli_args(&args(&["run", "w", "--input=x.json"])).unwrap() else {
        panic!()
    };
    assert_eq!(a.input, "x.json");

    // 未知 long option
    let e = err(&["run", "w", "--bogus"]);
    assert!(e.message().contains("Unknown option '--bogus'"));

    // boolean に値を付けると error
    let e = err(&["--quiet=true", "run", "w"]);
    assert!(e.message().contains("does not take an argument"));

    // string option の値が option-like → ambiguous
    let e = err(&["run", "w", "--input", "--cwd"]);
    assert!(e.message().contains("argument is ambiguous"));

    // `--` terminator 以降は positional
    let e = err(&["run", "w", "--", "extra"]);
    assert_eq!(e.message(), "unexpected arguments: extra");

    // `-` 単独は positional 扱い(value として受理される)
    let ParsedCli::Run(a) = parse_cli_args(&args(&["run", "w", "--input", "-"])).unwrap() else {
        panic!()
    };
    assert_eq!(a.input, "-");

    // duplicate option は last-wins
    let ParsedCli::Run(a) = parse_cli_args(&args(&[
        "run", "w", "--input", "a.json", "--input", "b.json",
    ]))
    .unwrap() else {
        panic!()
    };
    assert_eq!(a.input, "b.json");
}

// --- global observability flags(T2) ------------------------------------------
//
// `--log`/`--log-format` は strict parse で受理される global option —
// 値は `Values`/`ParsedCli` の field には載らず、消費は
// `global_log_flags` の lenient scan が担う。

#[test]
fn log_flags_accepted_globally() {
    // run
    assert!(parse_cli_args(&args(&[
        "run",
        "w",
        "--log",
        "debug",
        "--log-format",
        "json",
    ]))
    .is_ok());
    // run alias
    assert!(parse_cli_args(&args(&["bulk-read", "--log", "info"])).is_ok());
    // workflow 両 action — literal allowed-token 表で受理
    assert!(parse_cli_args(&args(&["workflow", "interface", "wf.json", "--log=debug",])).is_ok());
    assert!(parse_cli_args(&args(&[
        "workflow",
        "run",
        "wf.json",
        "--log",
        "trace",
        "--log-format=text",
    ]))
    .is_ok());
    // 他 command — field unchecked = globally accepted
    assert!(parse_cli_args(&args(&["models", "--log", "warn"])).is_ok());
    assert!(parse_cli_args(&args(&["count-tokens", "f", "--log=debug"])).is_ok());
    assert!(parse_cli_args(&args(&["usage", "--log-format", "json"])).is_ok());
    assert!(parse_cli_args(&args(&["auth", "status", "--log", "off"])).is_ok());
}

#[test]
fn log_flags_are_string_options() {
    // string option 扱い — 値なしは missing、option-like 値は ambiguous
    let e = err(&["run", "w", "--log"]);
    assert!(e.message().contains("argument missing"));
    let e = err(&["run", "w", "--log", "--quiet"]);
    assert!(e.message().contains("argument is ambiguous"));
}

#[test]
fn global_log_flags_scan() {
    let argv = args(&["models", "--log", "debug", "--log-format=json"]);
    assert_eq!(
        global_log_flags(&argv),
        (Some("debug".to_string()), Some("json".to_string()))
    );

    // last-wins
    let argv = args(&["--log", "info", "--log", "trace"]);
    assert_eq!(global_log_flags(&argv), (Some("trace".to_string()), None));

    // `--` terminator 以降の positional `--log` は拾わない
    let argv = args(&["run", "w", "--", "--log", "debug"]);
    assert_eq!(global_log_flags(&argv), (None, None));

    // 未指定
    assert_eq!(global_log_flags(&args(&["models"])), (None, None));
}

#[test]
fn global_flag_present_scan() {
    assert!(global_flag_present(&args(&["models", "--quiet"]), "quiet"));
    assert!(!global_flag_present(&args(&["models"]), "quiet"));
    // `--` 以降の positional `--quiet` は誤検出しない
    assert!(!global_flag_present(
        &args(&["run", "w", "--", "--quiet"]),
        "quiet"
    ));
    // `--quiet=x` は bool option の受理形ではない(strict parse が
    // "does not take an argument" で弾く) — scan も拾わない
    assert!(!global_flag_present(
        &args(&["models", "--quiet=true"]),
        "quiet"
    ));
}
