//! `cheaper hook <event> --target <agent>` entry point —
//! `packages/cli/src/hook/hook.ts` の port。
//!
//! この経路は runCli の envelope / recordUsage / 汎用 catch とは意図的に
//! 切り離されている: hook 呼出しは target native の出力のみを render
//! し、あらゆる失敗は target の fail-open 形へ解決して壊れた setup が
//! tool call を block しないようにする。Copilot は「いかなる」非 0
//! exit も deny と解釈するため、この command は windsurf deny の
//! exit 2 を除き非 0 を返さない。
//!
//! - `targets.rs` — 4 target (windsurf/copilot/pretooluse/cursor) の
//!   normalize + render(byte 互換 JSON 出力が contract)
//! - `routing_policy.rs` — threshold・byte fast path・30s 計測
//!   timeout・shell tokenizer / `extract_read_candidates`

pub mod routing_policy;
pub mod targets;

use serde_json::Value;

use self::targets::{fallback_allow, handle_hook, HookDeps, HookResponse, HookTarget};
use crate::cli::{CliHooks, CliIo};

/// `KNOWN_EVENTS` — 受け付ける hook event 名。
const KNOWN_EVENTS: &[&str] = &[
    "pre-read-code",
    "pre-run-command",
    "pre-tool-use",
    "before-read-file",
    "before-shell-execution",
];

struct HookArgv {
    event: Option<String>,
    target: Option<HookTarget>,
}

/// 緩い引数解析: 不正入力でも panic せず `None` を返す。
/// TS `parseHookArgv` と同じく `--target <v>` / `--target=<v>` のみを
/// 解釈し、未知 option は flag 本体だけ skip する(後続値は positional
/// として残る — これも TS 規則)。
fn parse_hook_argv(argv: &[String]) -> HookArgv {
    let mut target_raw: Option<String> = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut i = 0usize;
    while i < argv.len() {
        let arg = &argv[i];
        if arg == "--target" {
            target_raw = argv.get(i + 1).cloned();
            i += 2;
            continue;
        }
        if let Some(rest) = arg.strip_prefix("--target=") {
            target_raw = Some(rest.to_string());
            i += 1;
            continue;
        }
        if arg.starts_with('-') {
            i += 1;
            continue; // 未知 option は無視(fail-open)
        }
        positional.push(arg);
        i += 1;
    }
    HookArgv {
        event: positional.first().map(|s| (*s).to_string()),
        target: target_raw.as_deref().and_then(HookTarget::parse),
    }
}

/// `readStdinText` — TTY なら空文字列、そうでなければ EOF まで読み
/// UTF-8 lossy decode(`Buffer.concat(...).toString("utf8")` 相当)。
/// 読取り失敗も空文字列(fail-open → JSON parse 失敗 → fallback)。
fn read_stdin_text(io: &mut CliIo<'_>) -> String {
    if io.stdin.is_tty() {
        return String::new();
    }
    match io.stdin.read_all() {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(_) => String::new(),
    }
}

/// `emit` — stdout/stderr text があれば書き、`response.exit` を返す。
/// TS は write 自体を try/catch する(出力失敗でも exit code は維持
/// する fail-open)— `CliWrite` impl の panic も同じく飲み込む。
fn emit(io: &mut CliIo<'_>, response: HookResponse) -> i32 {
    if let Some(s) = &response.stdout {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            io.stdout.write_str(s);
        }));
    }
    if let Some(s) = &response.stderr {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            io.stderr.write_str(s);
        }));
    }
    response.exit
}

/// `executeHook` — 全 error を target 適合の allow + exit 0 へ変換する
/// catch-all を持ち、runCli の汎用 catch には到達しない(呼出し側は
/// 早期 return で dispatch する)。
///
/// panic も fail-open の対象: T2 の `catch_run_panic` 境界(main.rs
/// 側は panic → INTERNAL_ERROR envelope + exit 70)に到達する前に、
/// hook 経路内で panic を `fallback_allow` + exit 0 へ畳む —
/// envelope JSON が stdout に出たり非 0 が返ると copilot が全 tool
/// call を deny するため。
pub async fn execute_hook(argv: &[String], io: &mut CliIo<'_>, hooks: &CliHooks) -> i32 {
    let HookArgv { event, target } = parse_hook_argv(argv);
    let inner = execute_hook_inner(event, target, io, hooks);
    // scrutinee の temporary が match 腕まで `&mut CliIo` を保持する
    // のを避けるため一度 let へ束縛する。
    let outcome = cheaper_runtime::catch_run_panic(inner).await;
    match outcome {
        Ok(code) => code,
        // panic も fail-open — payload は破棄して target 適合の
        // allow + exit 0(copilot の非 0 = deny 規則のため)。
        Err(_) => emit(io, fallback_allow(target)),
    }
}

/// `executeHook` の本体(TS の try ブロック内)。event/target が揃わ
/// ない・未知 event・malformed stdin の経路は全て `fallback_allow`
/// (cursor は schema-valid allow JSON、他は silent)へ。
async fn execute_hook_inner(
    event: Option<String>,
    target: Option<HookTarget>,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
) -> i32 {
    let (event, target) = match (event, target) {
        (Some(e), Some(t)) if KNOWN_EVENTS.contains(&e.as_str()) => (e, t),
        (_, t) => return emit(io, fallback_allow(t)),
    };
    let text = read_stdin_text(io);
    let input: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            // malformed stdin: cursor は schema-valid JSON 必須、他は silent
            return emit(io, fallback_allow(Some(target)));
        }
    };
    let deps = HookDeps {
        env: io.env,
        builtin_dir: hooks.builtin_dir.as_ref(),
        cwd: &io.cwd,
    };
    let response = handle_hook(target, &event, &input, &deps).await;
    emit(io, response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_hook_argv_target_forms() {
        let a = parse_hook_argv(&argv(&["pre-read-code", "--target", "windsurf"]));
        assert_eq!(a.event.as_deref(), Some("pre-read-code"));
        assert_eq!(a.target, Some(HookTarget::Windsurf));

        let a = parse_hook_argv(&argv(&["pre-read-code", "--target=cursor"]));
        assert_eq!(a.target, Some(HookTarget::Cursor));

        // unknown target / missing value / empty value → None。
        for v in [
            &["pre-read-code", "--target", "unknown"][..],
            &["pre-read-code", "--target"][..],
            &["pre-read-code", "--target="][..],
        ] {
            assert_eq!(parse_hook_argv(&argv(v)).target, None, "{v:?}");
        }
        // last wins。
        let a = parse_hook_argv(&argv(&["e", "--target", "copilot", "--target", "cursor"]));
        assert_eq!(a.target, Some(HookTarget::Cursor));
    }

    #[test]
    fn parse_hook_argv_unknown_flags_and_positionals() {
        // 未知 flag は skip するが後続値は positional に残る(TS 規則)。
        let a = parse_hook_argv(&argv(&[
            "--bogus",
            "x",
            "pre-read-code",
            "--target",
            "windsurf",
        ]));
        assert_eq!(a.event.as_deref(), Some("x"));
        // 単一 `-` 始まりも skip。
        let a = parse_hook_argv(&argv(&["-v", "pre-tool-use", "--target", "copilot"]));
        assert_eq!(a.event.as_deref(), Some("pre-tool-use"));
        assert_eq!(a.target, Some(HookTarget::Copilot));
        // positional 無し → None。
        assert!(parse_hook_argv(&argv(&["--target", "cursor"]))
            .event
            .is_none());
        assert!(parse_hook_argv(&argv(&[])).event.is_none());
    }
}
