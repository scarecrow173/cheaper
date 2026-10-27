//! `cheaper` binary entry — `main.ts`/`sea-main.ts` 相当。
//!
//! 責務は process 境界のみ:
//! 1. `argv[0] === "__grep-eval"` の内部 mode を**最初に**分岐
//!    (`grep_eval` subprocess supervisor の spawn 先。envelope・
//!    usage record・通常診断を一切出さない)。
//! 2. panic hook(panic trace は stderr のみ — stdout は envelope
//!    契約で汚染禁止)+ `catch_unwind` 境界で unhandled panic を
//!    `INTERNAL_ERROR`(exit 70)envelope へ変換する。
//! 3. `cli::run_cli` を tokio runtime 上で await し、戻り値を
//!    `process::exit` する。
//!
//! devin plugin 登録・adapter session 構築の composition root 結線は
//! T3 が run 経路と一緒に行う(runtime→devin は循環依存のため不可)。

use std::io::Read;
use std::io::Write;

use cheaper_cli::args::is_route_check_invocation;
use cheaper_cli::cli::{route_check_error_envelope, run_cli, CliHooks, CliIo, CliStdin, CliWrite};
use cheaper_cli::envelope::{envelope_json, error_envelope};

/// 実 process stdin。
struct ProcessStdin;

impl CliStdin for ProcessStdin {
    fn is_tty(&self) -> bool {
        use std::io::IsTerminal;
        std::io::stdin().is_terminal()
    }

    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        Ok(buf)
    }
}

/// 実 process stdout/stderr(`write(s)` 相当 — write 失敗は黙殺
/// するのではなく best-effort で流す。envelope 書出し失敗時の
/// fallback は T8 differential で扱う)。
struct ProcessStdout;
struct ProcessStderr;

impl CliWrite for ProcessStdout {
    fn write_str(&mut self, s: &str) {
        let _ = std::io::stdout().write_all(s.as_bytes());
        let _ = std::io::stdout().flush();
    }
}

impl CliWrite for ProcessStderr {
    fn write_str(&mut self, s: &str) {
        let _ = std::io::stderr().write_all(s.as_bytes());
        let _ = std::io::stderr().flush();
    }
}

/// panic payload → INTERNAL_ERROR envelope を stdout へ書き、70 を返す。
/// panic hook が既に stderr へ trace を出している想定。
fn panic_exit(payload: &str, is_route_check: bool) -> i32 {
    let err = cheaper_core::errors::CheaperError::new(
        cheaper_core::errors::ErrorCode::InternalError,
        format!("panic: {payload}"),
    );
    let env = if is_route_check {
        route_check_error_envelope(&err, None)
    } else {
        error_envelope(&err, None, None)
    };
    ProcessStdout.write_str(&envelope_json(&env));
    70
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // `__grep-eval` 内部 mode — 通常 parse/catch/usage より前の
    // argv 先頭一致で分岐する(必須。production grep tool の spawn 先)。
    if argv.first().map(String::as_str) == Some(cheaper_tools::tools::grep_eval::HELPER_ARGV) {
        std::process::exit(cheaper_tools::tools::grep_eval::helper_main());
    }

    // panic trace は stderr のみ(stdout envelope 契約を汚染しない)。
    std::panic::set_hook(Box::new(|info| {
        eprintln!("cheaper internal panic: {info}");
    }));

    // entry 全体を catch_unwind で包み、future 外(runtime 構築・
    // dispatch 前後の同期処理)の unhandled panic も INTERNAL_ERROR
    // envelope + exit 70 に畳む。
    let is_route_check = is_route_check_invocation(&argv);
    let code = std::panic::catch_unwind(|| run_process(argv)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<&'static str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".to_string());
        panic_exit(&message, is_route_check)
    });
    std::process::exit(code);
}

fn run_process(argv: Vec<String>) -> i32 {
    let is_route_check = is_route_check_invocation(&argv);
    let mut stdin = ProcessStdin;
    let mut stdout = ProcessStdout;
    let mut stderr = ProcessStderr;
    let env_lookup = |name: &str| std::env::var(name).ok();
    let cwd = match std::env::current_dir() {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(e) => {
            // `process.cwd()` の throw 相当 — envelope に畳む。
            let err = cheaper_core::errors::CheaperError::new(
                cheaper_core::errors::ErrorCode::InternalError,
                format!("process.cwd() failed: {e}"),
            );
            let env = if is_route_check {
                route_check_error_envelope(&err, None)
            } else {
                error_envelope(&err, None, None)
            };
            ProcessStdout.write_str(&envelope_json(&env));
            return 70;
        }
    };
    let mut io = CliIo {
        stdin: &mut stdin,
        stdout: &mut stdout,
        stderr: &mut stderr,
        env: &env_lookup,
        cwd,
    };
    let hooks = CliHooks::default();

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            let err = cheaper_core::errors::CheaperError::new(
                cheaper_core::errors::ErrorCode::InternalError,
                format!("tokio runtime init failed: {e}"),
            );
            let env = if is_route_check {
                route_check_error_envelope(&err, None)
            } else {
                error_envelope(&err, None, None)
            };
            ProcessStdout.write_str(&envelope_json(&env));
            return 70;
        }
    };

    // `AssertUnwindSafe(run_cli(..)).catch_unwind()` — runtime の
    // `catch_run_panic` が panic payload → INTERNAL_ERROR に写す。
    // JoinError は各 spawn サイトで `join_error_to_internal` に畳む
    // 責務を持つ(T3+ の command 実装側)。
    let outcome = rt.block_on(cheaper_runtime::catch_run_panic(run_cli(
        &argv, &mut io, &hooks,
    )));
    match outcome {
        Ok(code) => code,
        Err(err) => {
            let env = if is_route_check {
                route_check_error_envelope(&err, None)
            } else {
                error_envelope(&err, None, None)
            };
            ProcessStdout.write_str(&envelope_json(&env));
            err.exit_code() as i32
        }
    }
}
