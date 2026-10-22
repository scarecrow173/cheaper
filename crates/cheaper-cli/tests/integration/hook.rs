//! `cheaper hook <event> --target <agent>` の contract test —
//! `packages/cli/test/hook/hook-contract.test.ts` の移植(T6)。
//!
//! 4 target の allow/deny/malformed 出力・catch-all fail-open・
//! usage 非記録・panic → allow/exit 0 を `run_cli` + CliIo stub で
//! 検証する。TS binary との byte parity matrix は別途
//! `node packages/cli/bin/cheaper.js` との spawn diff で確認済み
//! (docs/impl/ir/phase-4-port-cli-and-workflow.md T6)。

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::common;
use common::{PipeStdin, VecSink};

use cheaper_cli::cli::{run_cli, CliHooks, CliIo, CliStdin};
use cheaper_cli::hook::targets::HookTarget;

/// `"lorem ipsum dolor sit amet ".repeat(4000)` — hook-contract.test.ts
/// と同一の 8192 token 超 fixture(27 bytes × 4000 = 108000 bytes)。
/// `pub(crate)`: sibling `hook_eval_dedup` から再利用。
pub(crate) fn big_text() -> String {
    "lorem ipsum dolor sit amet ".repeat(4000)
}

/// stdin read が panic する stub(panic → allow/exit 0 の注入用)。
struct PanicStdin;
impl CliStdin for PanicStdin {
    fn is_tty(&self) -> bool {
        false
    }
    fn read_all(&mut self) -> io::Result<Vec<u8>> {
        panic!("injected stdin panic")
    }
}

/// `pub(crate)`: sibling `hook_eval_dedup` から再利用 —
/// `CHEAPER_CONFIG_DIR`・`HOME`・`USERPROFILE` を tempdir へ隔離する
/// env map を供給し、test が実 `~/.config/cheaper/hook-eval/` を
/// 一切触れないことを構造的に保証する。
pub(crate) struct Fixture {
    _dir: tempfile::TempDir,
    pub(crate) ws: PathBuf,
    /// `makeIsolatedEnv` 相当の env map。
    pub(crate) env: HashMap<String, String>,
    pub(crate) builtin_dir: PathBuf,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("ws");
        std::fs::create_dir_all(&ws).unwrap();
        let mut env = HashMap::new();
        env.insert(
            "CHEAPER_CONFIG_DIR".to_string(),
            dir.path()
                .join("cheaper-config")
                .to_string_lossy()
                .into_owned(),
        );
        env.insert(
            "HOME".to_string(),
            dir.path().to_string_lossy().into_owned(),
        );
        env.insert(
            "USERPROFILE".to_string(),
            dir.path().to_string_lossy().into_owned(),
        );
        env.insert("NODE_ENV".to_string(), "production".to_string());
        // TS test は `builtinDir: cliRoot` — crate の bundled resource
        // dir が policy.yaml + workers/ を持つ builtin resource root。
        let builtin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        Fixture {
            _dir: dir,
            ws,
            env,
            builtin_dir,
        }
    }

    pub(crate) fn write(&self, name: &str, contents: &[u8]) {
        std::fs::write(self.ws.join(name), contents).unwrap();
    }

    pub(crate) fn write_big(&self, name: &str) {
        self.write(name, big_text().as_bytes());
    }

    /// 複数行の巨大 file(#107 の ranged-read シナリオ用) —
    /// 各行は小さいが合計が threshold を超える。
    fn write_big_lines(&self, name: &str) {
        let text = (0..2000)
            .map(|i| format!("line {i} lorem ipsum dolor sit amet\n"))
            .collect::<String>();
        self.write(name, text.as_bytes());
    }
}

/// `pub(crate)`: sibling `hook_eval_dedup` から再利用。
pub(crate) fn windsurf_read(file_path: &str) -> String {
    windsurf_tool_info(&format!(
        "{{\"file_path\":{}}}",
        serde_json::to_string(file_path).unwrap()
    ))
}

/// `tool_info` を任意 JSON で指定する windsurf `pre_read_code`
/// payload — range/search 語彙の fixture 用(issue #161)。
fn windsurf_tool_info(tool_info_json: &str) -> String {
    format!("{{\"agent_action_name\":\"pre_read_code\",\"tool_info\":{tool_info_json}}}")
}

/// windsurf `pre_run_command` payload — shell request。
pub(crate) fn windsurf_shell(command: &str) -> String {
    format!(
        "{{\"agent_action_name\":\"pre_run_command\",\"tool_info\":{{\"command_line\":{}}}}}",
        serde_json::to_string(command).unwrap()
    )
}

/// `<CHEAPER_CONFIG_DIR>/hook-eval/` — dedup verdict store。
pub(crate) fn eval_store_dir(fx: &Fixture) -> std::path::PathBuf {
    std::path::Path::new(&fx.env["CHEAPER_CONFIG_DIR"]).join("hook-eval")
}

/// `pub(crate)`: sibling `hook_eval_dedup` から再利用。
pub(crate) struct HookResult {
    pub(crate) code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

/// `pub(crate)`: sibling `hook_eval_dedup` から再利用。
pub(crate) async fn run_hook(
    fx: &Fixture,
    args: &[&str],
    stdin: &str,
    extra_env: &[(&str, &str)],
) -> HookResult {
    let mut env_map = fx.env.clone();
    for (k, v) in extra_env {
        env_map.insert((*k).to_string(), (*v).to_string());
    }
    let env_lookup = move |name: &str| env_map.get(name).cloned();
    let mut stdin_impl = PipeStdin {
        data: stdin.as_bytes().to_vec(),
        tty: false,
    };
    let mut out = VecSink(Vec::new());
    let mut err = VecSink(Vec::new());
    let mut io = CliIo {
        stdin: &mut stdin_impl,
        stdout: &mut out,
        stderr: &mut err,
        env: &env_lookup,
        cwd: fx.ws.to_string_lossy().into_owned(),
    };
    let hooks = CliHooks {
        builtin_dir: Some(fx.builtin_dir.clone()),
        ..Default::default()
    };
    let mut argv: Vec<String> = vec!["hook".to_string()];
    argv.extend(args.iter().map(|s| s.to_string()));
    let code = run_cli(&argv, &mut io, &hooks).await;
    HookResult {
        code,
        stdout: String::from_utf8(out.0).unwrap(),
        stderr: String::from_utf8(err.0).unwrap(),
    }
}

// --- command path ------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn silent_exit0_for_bad_args_unknown_target_event_extra_flags() {
    let fx = Fixture::new();
    for args in [
        &[][..],
        &["pre-read-code"][..],
        &["pre-read-code", "--target", "unknown"][..],
        &["bogus-event", "--target", "windsurf"][..],
        &["pre-read-code", "--target", "windsurf", "--bogus-flag"][..],
    ] {
        let r = run_hook(&fx, args, "{}", &[]).await;
        assert_eq!(r.code, 0, "args: {args:?}");
        assert!(r.stdout.is_empty(), "args: {args:?} stdout={:?}", r.stdout);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cursor_schema_valid_allow_even_with_malformed_stdin() {
    let fx = Fixture::new();
    let r = run_hook(
        &fx,
        &["before-read-file", "--target", "cursor"],
        "not json",
        &[],
    )
    .await;
    assert_eq!(r.code, 0);
    assert_eq!(r.stdout, "{\"permission\":\"allow\"}\n");
    assert!(r.stderr.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn hook_invocations_do_not_write_usage_jsonl() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let r = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;
    assert_eq!(r.code, 2);
    assert!(!Path::new(&fx.env["CHEAPER_CONFIG_DIR"])
        .join("usage.jsonl")
        .exists());
}

#[tokio::test(flavor = "current_thread")]
async fn fails_open_when_user_policy_is_corrupt() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let config_dir = Path::new(&fx.env["CHEAPER_CONFIG_DIR"]).to_path_buf();
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("policy.yaml"), "::: not yaml :::[").unwrap();
    let r = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;
    assert_eq!(r.code, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn panic_in_stdin_read_falls_back_to_allow() {
    let fx = Fixture::new();
    let env_map = fx.env.clone();
    let env_lookup = move |name: &str| env_map.get(name).cloned();
    for (target, expect_stdout) in [
        ("windsurf", ""),
        ("copilot", ""),
        ("pretooluse", ""),
        ("cursor", "{\"permission\":\"allow\"}\n"),
        ("devin", ""),
    ] {
        let mut stdin = PanicStdin;
        let mut out = VecSink(Vec::new());
        let mut err = VecSink(Vec::new());
        let mut io = CliIo {
            stdin: &mut stdin,
            stdout: &mut out,
            stderr: &mut err,
            env: &env_lookup,
            cwd: fx.ws.to_string_lossy().into_owned(),
        };
        let hooks = CliHooks {
            builtin_dir: Some(fx.builtin_dir.clone()),
            ..Default::default()
        };
        let argv: Vec<String> = ["hook", "pre-tool-use", "--target", target]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let code = run_cli(&argv, &mut io, &hooks).await;
        assert_eq!(code, 0, "target={target}");
        assert_eq!(
            String::from_utf8(out.0).unwrap(),
            expect_stdout,
            "target={target}"
        );
    }
}

// --- windsurf -----------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn windsurf_deny_stderr_exit2_and_silent_allow() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    fx.write("small.txt", b"hello\n");
    let deny = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.txt"),
        &[],
    )
    .await;
    assert_eq!(deny.code, 2);
    assert!(deny.stdout.is_empty());
    assert!(deny.stderr.contains("tokens"), "{}", deny.stderr);
    assert!(deny.stderr.contains("bulk-read"), "{}", deny.stderr);
    // JS-format reason: 3 行 + 末尾改行。
    assert!(
        deny.stderr.starts_with("cheaper: 'big.txt' has ")
            && deny.stderr.contains(" tokens (threshold 8192).\n")
            && deny.stderr.contains(
                "Do not read it directly. Delegate this file through the cheaper-bulk-read skill.\n"
            )
            && deny.stderr.ends_with("workspace-relative path: big.txt\n"),
        "{}",
        deny.stderr
    );
    let allow = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("small.txt"),
        &[],
    )
    .await;
    assert_eq!(allow.code, 0);
    assert!(allow.stdout.is_empty());
    assert!(allow.stderr.is_empty());
}

/// issue #161 — windsurf `pre_read_code` の range 語彙を共有
/// `tool_use_normalize` と parity で受理する。bounded window は
/// threshold 内なら allow、無効・矛盾値は全文評価へ縮退して deny
/// (過少評価しない fail-closed)。
#[tokio::test(flavor = "current_thread")]
async fn windsurf_ranged_read_vocabularies_bounded_eval() {
    let fx = Fixture::new();
    fx.write_big_lines("big_lines.txt");
    // bounded allow 系 — 20 行 window は threshold 内。
    for tool_info in [
        "{\"file_path\":\"big_lines.txt\",\"start_line\":1,\"end_line\":20}",
        "{\"file_path\":\"big_lines.txt\",\"view_range\":[1,20]}",
        "{\"file_path\":\"big_lines.txt\",\"offset\":1,\"limit\":20}",
        "{\"file_path\":\"big_lines.txt\",\"line_offset\":1,\"line_limit\":20}",
        "{\"file_path\":\"big_lines.txt\",\"offset\":1,\"max_lines\":20}",
        "{\"file_path\":\"big_lines.txt\",\"start_line\":1,\"limit\":20}",
        "{\"file_path\":\"big_lines.txt\",\"end_line\":20}",
        // `[s,-1]` "to EOF" sentinel → `{s,None}`(末尾 6 行 → allow)。
        "{\"file_path\":\"big_lines.txt\",\"view_range\":[1995,-1]}",
        // lone `start_line` → `{s,None}`(s 行〜EOF、末尾 6 行 → allow)。
        "{\"file_path\":\"big_lines.txt\",\"start_line\":1995}",
    ] {
        let r = run_hook(
            &fx,
            &["pre-read-code", "--target", "windsurf"],
            &windsurf_tool_info(tool_info),
            &[],
        )
        .await;
        assert_eq!(r.code, 0, "tool_info={tool_info} stderr={:?}", r.stderr);
        assert!(r.stdout.is_empty(), "tool_info={tool_info}");
    }
    // 無効・矛盾値は全文評価へ縮退 → deny(全文 token 計測文言で
    // pin し、`{s,None}` 等の狭い window 評価でないことを示す)。
    for tool_info in [
        // 矛盾 pair — Step C の `{1900,None}`(末尾 100 行 ≈ allow)
        // へ落ちないことの識別 pin: 全文評価 deny が成立する。
        "{\"file_path\":\"big_lines.txt\",\"start_line\":1900,\"end_line\":20}",
        "{\"file_path\":\"big_lines.txt\",\"view_range\":[1900,2]}",
        "{\"file_path\":\"big_lines.txt\",\"start_line\":0}",
        "{\"file_path\":\"big_lines.txt\",\"start_line\":\"x\"}",
        "{\"file_path\":\"big_lines.txt\",\"end_line\":\"x\"}",
        "{\"file_path\":\"big_lines.txt\",\"view_range\":[1,\"x\"]}",
        "{\"file_path\":\"big_lines.txt\",\"view_range\":[1,-5]}",
        // range 無しは従来どおり全文 deny。
        "{\"file_path\":\"big_lines.txt\"}",
    ] {
        let r = run_hook(
            &fx,
            &["pre-read-code", "--target", "windsurf"],
            &windsurf_tool_info(tool_info),
            &[],
        )
        .await;
        assert_eq!(r.code, 2, "tool_info={tool_info}");
        assert!(
            r.stderr.contains("'big_lines.txt' has") && !r.stderr.contains("read window"),
            "tool_info={tool_info} stderr={:?}",
            r.stderr
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn windsurf_exit0_fast_path_wrong_events_missing_fields() {
    let fx = Fixture::new();
    fx.write("exact.bin", &[0x61u8; 8192]);
    let outside = fx
        ._dir
        .path()
        .join("outside.txt")
        .to_string_lossy()
        .into_owned();
    let ws = fx.ws.to_string_lossy().into_owned();
    let cases: Vec<String> = vec![
        windsurf_read("exact.bin"), // 8192 bytes <= threshold → fast path
        windsurf_read("missing.txt"),
        windsurf_read(&ws), // directory
        "not json".to_string(),
        "{}".to_string(),
        "{\"agent_action_name\":\"post_read_code\",\"tool_info\":{}}".to_string(),
        "{\"agent_action_name\":\"pre_read_code\",\"tool_info\":{}}".to_string(),
        "{\"agent_action_name\":\"pre_read_code\",\"tool_info\":{\"file_path\":42}}".to_string(),
        windsurf_read(&outside), // outside workspace
    ];
    for stdin in &cases {
        let r = run_hook(&fx, &["pre-read-code", "--target", "windsurf"], stdin, &[]).await;
        assert_eq!(r.code, 0, "stdin: {}", &stdin[..stdin.len().min(60)]);
        assert!(
            r.stdout.is_empty(),
            "stdin: {}",
            &stdin[..stdin.len().min(60)]
        );
    }
}

// --- copilot -----------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn copilot_deny_json_exit0_and_never_nonzero() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let big = fx.ws.join("big.txt").to_string_lossy().into_owned();
    let big_json = serde_json::to_string(&big).unwrap();
    let cases: Vec<(String, &str)> = vec![
        (
            format!("{{\"toolName\":\"view\",\"toolArgs\":{{\"path\":{big_json}}}}}"),
            "deny-case",
        ),
        ("not json".to_string(), "malformed"),
        ("{\"toolName\":\"view\"}".to_string(), "missing toolArgs"),
        (
            "{\"toolName\":\"view\",\"toolArgs\":\"{broken\"}".to_string(),
            "broken toolArgs",
        ),
        (
            format!("{{\"toolName\":\"write\",\"toolArgs\":{{\"path\":{big_json}}}}}"),
            "unrelated tool",
        ),
        (
            format!("{{\"tool_name\":\"Read\",\"tool_input\":{{\"file_path\":{big_json}}}}}"),
            "foreign shape",
        ),
    ];
    for (stdin, label) in &cases {
        let r = run_hook(&fx, &["pre-tool-use", "--target", "copilot"], stdin, &[]).await;
        assert_eq!(r.code, 0, "case: {label}");
    }
    let deny = run_hook(
        &fx,
        &["pre-tool-use", "--target", "copilot"],
        &format!("{{\"toolName\":\"view\",\"toolArgs\":{{\"path\":{big_json}}}}}"),
        &[],
    )
    .await;
    let out: serde_json::Value = serde_json::from_str(deny.stdout.trim()).unwrap();
    assert_eq!(out["permissionDecision"], "deny");
    assert!(
        out["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("bulk-read"),
        "{}",
        deny.stdout
    );
    // byte-level shape: 先頭が固定 prefix、改行が flatten されている。
    assert!(
        deny.stdout.starts_with(
            "{\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"cheaper: '"
        ),
        "{}",
        deny.stdout
    );
    assert!(!deny.stdout.contains("\\n"), "{}", deny.stdout);
    assert!(deny.stdout.ends_with("}\n"));
}

#[tokio::test(flavor = "current_thread")]
async fn copilot_dual_schema_allow_for_foreign_payload() {
    let fx = Fixture::new();
    let r = run_hook(
        &fx,
        &["pre-tool-use", "--target", "copilot"],
        "{\"tool_name\":\"Read\",\"tool_input\":{\"file_path\":\"x.txt\"}}",
        &[],
    )
    .await;
    assert_eq!(r.code, 0);
    // byte-exact: dual schema allow。
    assert_eq!(
        r.stdout,
        "{\"permission\":\"allow\",\"permissionDecision\":\"allow\"}\n"
    );
}

// --- pretooluse ---------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn pretooluse_hook_specific_output_deny_exit0() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let r = run_hook(
        &fx,
        &["pre-tool-use", "--target", "pretooluse"],
        "{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"cat big.txt\"}}",
        &[],
    )
    .await;
    assert_eq!(r.code, 0);
    assert!(
        r.stdout.starts_with(
            "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\""
        ),
        "{}",
        r.stdout
    );
    let out: serde_json::Value = serde_json::from_str(r.stdout.trim()).unwrap();
    let hso = &out["hookSpecificOutput"];
    assert_eq!(hso["hookEventName"], "PreToolUse");
    assert_eq!(hso["permissionDecision"], "deny");
    assert!(
        hso["permissionDecisionReason"]
            .as_str()
            .unwrap()
            .contains("blocked shell command"),
        "{}",
        r.stdout
    );
}

#[tokio::test(flavor = "current_thread")]
async fn pretooluse_silent_exit0_on_every_fail_open_path() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let cases: Vec<String> = vec![
        "not json".to_string(),
        "{}".to_string(),
        "{\"tool_name\":\"Write\",\"tool_input\":{\"file_path\":\"big.txt\"}}".to_string(),
        "{\"tool_name\":\"Read\"}".to_string(),
        "{\"tool_name\":\"Read\",\"tool_input\":{}}".to_string(),
        "{\"tool_name\":\"Read\",\"tool_input\":{\"file_path\":\"missing.txt\"}}".to_string(),
        "{\"tool_name\":\"Read\",\"tool_input\":{\"file_path\":\"small.txt\"}}".to_string(),
    ];
    for stdin in &cases {
        let r = run_hook(&fx, &["pre-tool-use", "--target", "pretooluse"], stdin, &[]).await;
        assert_eq!(r.code, 0, "stdin: {}", &stdin[..stdin.len().min(60)]);
        assert!(
            r.stdout.is_empty(),
            "stdin: {}",
            &stdin[..stdin.len().min(60)]
        );
    }
}

// --- cursor -------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn cursor_always_emits_schema_valid_permission_json() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let ws = fx.ws.to_string_lossy().into_owned();
    let ws_json = serde_json::to_string(&ws).unwrap();
    let outside = fx
        ._dir
        .path()
        .join("out.txt")
        .to_string_lossy()
        .into_owned();
    let outside_json = serde_json::to_string(&outside).unwrap();
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "before-read-file",
            format!("{{\"file_path\":\"big.txt\",\"cwd\":{ws_json}}}"),
            "deny",
        ),
        (
            "before-shell-execution",
            format!("{{\"command\":\"cat big.txt\",\"cwd\":{ws_json}}}"),
            "deny",
        ),
        ("before-read-file", "not json".to_string(), "allow"),
        ("before-read-file", "{}".to_string(), "allow"),
        ("before-read-file", "{\"other\":1}".to_string(), "allow"),
        (
            "before-read-file",
            format!("{{\"file_path\":\"missing.txt\",\"cwd\":{ws_json}}}"),
            "allow",
        ),
        (
            "before-read-file",
            format!("{{\"file_path\":{outside_json},\"cwd\":{ws_json}}}"),
            "allow",
        ),
    ];
    for (event, stdin, expected) in &cases {
        let r = run_hook(&fx, &[event, "--target", "cursor"], stdin, &[]).await;
        assert_eq!(
            r.code,
            0,
            "event: {event} stdin: {}",
            &stdin[..40.min(stdin.len())]
        );
        let out: serde_json::Value = serde_json::from_str(r.stdout.trim()).unwrap();
        assert!(
            ["allow", "deny", "ask"].contains(&out["permission"].as_str().unwrap()),
            "{}",
            r.stdout
        );
        assert_eq!(out["permission"], *expected, "{stdin}");
        if *expected == "deny" {
            // schema: user_message / agent_message が 1 行 flatten。
            assert_eq!(out["user_message"], out["agent_message"]);
        }
    }
}

// --- shared policy ------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn file_too_large_is_fail_closed_deny_with_byte_cap_wording() {
    let fx = Fixture::new();
    fx.write("big.bin", &[0x61u8; 64]);
    let r = run_hook(
        &fx,
        &["pre-read-code", "--target", "windsurf"],
        &windsurf_read("big.bin"),
        &[
            ("CHEAPER_READ_TOKEN_THRESHOLD", "8"),
            ("CHEAPER_COUNT_MAX_BYTES", "16"),
        ],
    )
    .await;
    assert_eq!(r.code, 2);
    assert!(r.stderr.contains("64 bytes"), "{}", r.stderr);
    assert!(r.stderr.contains("16-byte measurement cap"), "{}", r.stderr);
    assert!(r.stderr.contains("bulk-read"), "{}", r.stderr);
}

#[tokio::test(flavor = "current_thread")]
async fn shell_deny_and_cwd_changing_commands_fail_open() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let deny = run_hook(
        &fx,
        &["pre-run-command", "--target", "windsurf"],
        &windsurf_shell("cat big.txt"),
        &[],
    )
    .await;
    assert_eq!(deny.code, 2);
    assert!(
        deny.stderr.contains("blocked shell command 'cat big.txt'."),
        "{}",
        deny.stderr
    );
    let allow = run_hook(
        &fx,
        &["pre-run-command", "--target", "windsurf"],
        &windsurf_shell("cd src && cat big.txt"),
        &[],
    )
    .await;
    assert_eq!(allow.code, 0);
}

// --- route supply(C1)------------------------------------------------------

/// routes.yaml + grant policy + workflow file を `<cfg>`(user 層)へ
/// 配置する fixture — `cat` へ route し、grant executable は test
/// binary 自身を pin する(route.rs の fixture と同じ構造)。
fn routed_shell_fixture() -> Fixture {
    let f = Fixture::new();
    let cfg = std::path::Path::new(&f.env["CHEAPER_CONFIG_DIR"]).to_path_buf();
    std::fs::create_dir_all(&cfg).unwrap();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(
        cfg.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: cat-route\n    match: {command: [cat]}\n    target: {kind: workflow, path: w.json}\n    grant: g\n",
    )
    .unwrap();
    std::fs::write(cfg.join("w.json"), r#"{"version":1,"jobs":{}}"#).unwrap();
    std::fs::write(
        cfg.join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    g:\n      executable: \"{}\"\n      argv: [\"*\"]\n",
            exe.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    )
    .unwrap();
    f
}

/// dedup store 内の `<fp>.json` verdict record を列挙する。
fn eval_records(fx: &Fixture) -> Vec<std::path::PathBuf> {
    let dir = eval_store_dir(fx);
    std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default()
}

/// C1 contract — hook firing が effective routes + command policy を
/// load して eval へ供給する。routed `cat big.txt` は
/// `Verdict::Rewrite{fallback: Deny}` を返し、windsurf render は
/// fallback の deny 応答(exit 2)を返す。Rewrite は dedup cache へ
/// persist されないため store に record が残らない — これが
/// route arm が firing load 経由で活性化した観測可能な証明
/// (unrouted の同一 command は Deny record を publish する)。
#[tokio::test(flavor = "current_thread")]
async fn shell_route_arm_engages_via_firing_load() {
    let fx = routed_shell_fixture();
    fx.write_big("big.txt");
    let r = run_hook(
        &fx,
        &["pre-run-command", "--target", "windsurf"],
        &windsurf_shell("cat big.txt"),
        &[],
    )
    .await;
    // fallback deny が render される — routing が deny を素通し
    // しない(C6)。
    assert_eq!(r.code, 2);
    assert!(
        r.stderr.contains("blocked shell command 'cat big.txt'."),
        "{}",
        r.stderr
    );
    assert!(
        eval_records(&fx).is_empty(),
        "Rewrite verdict は dedup record を書かない"
    );

    // control — routes/policy 無しの同一 firing は Deny record を
    // publish する(route arm 不活性の対照経路)。
    let plain = Fixture::new();
    plain.write_big("big.txt");
    let r = run_hook(
        &plain,
        &["pre-run-command", "--target", "windsurf"],
        &windsurf_shell("cat big.txt"),
        &[],
    )
    .await;
    assert_eq!(r.code, 2);
    assert!(
        !eval_records(&plain).is_empty(),
        "unrouted deny は dedup record を publish する"
    );
}

/// C4 — routed shell request の per-target render contract。
/// claude(pretooluse)は `hookSpecificOutput.updatedInput`、codex は
/// fallback verdict(hookSpecificOutput deny/allow)のみ render する。
#[tokio::test(flavor = "current_thread")]
async fn routed_shell_per_target_render_contract() {
    let payload = || tool_use_payload("Bash", "{\"command\":\"cat big.txt\"}");

    // claude — Rewrite → updatedInput.command + exit 0。
    let claude_fx = routed_shell_fixture();
    claude_fx.write("small.txt", b"hi\n");
    let r = run_hook(
        &claude_fx,
        &["pre-tool-use", "--target", "pretooluse"],
        &tool_use_payload("Bash", "{\"command\":\"cat small.txt\"}"),
        &[],
    )
    .await;
    assert_eq!(r.code, 0, "{}", r.stderr);
    let v: serde_json::Value = serde_json::from_str(r.stdout.trim()).unwrap();
    let cmd = v
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("claude updatedInput 無し: {}", r.stdout));
    assert!(cmd.contains("route run --payload"), "{cmd}");
    assert!(v
        .pointer("/hookSpecificOutput/permissionDecision")
        .is_none());

    // codex — 同一 payload は fallback verdict(deny)のみ render
    // され updatedInput は発行されない。
    let codex_fx = routed_shell_fixture();
    codex_fx.write_big("big.txt");
    let r = run_hook(
        &codex_fx,
        &["pre-tool-use", "--target", "codex"],
        &payload(),
        &[],
    )
    .await;
    assert_eq!(r.code, 0, "{}", r.stderr);
    let v: serde_json::Value = serde_json::from_str(r.stdout.trim()).unwrap();
    assert_eq!(
        v.pointer("/hookSpecificOutput/permissionDecision")
            .and_then(serde_json::Value::as_str),
        Some("deny"),
        "{}",
        r.stdout
    );
    assert!(v.pointer("/hookSpecificOutput/updatedInput").is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn evaluate_typed_entry_matches_ts_verdicts() {
    // routing-policy.test.ts の file-read/shell verdict 相当を
    // `evaluate` で直接確認(spawn せず policy 層の決定的 coverage)。
    use cheaper_cli::hook::routing_policy::{evaluate, EvaluateOptions, RoutingRequest, Verdict};
    let fx = Fixture::new();
    fx.write_big("big.txt");
    fx.write("small.txt", b"hello\n");
    let env_map = fx.env.clone();
    let env_lookup = move |name: &str| env_map.get(name).cloned();
    let opts = EvaluateOptions {
        env: &env_lookup,
        builtin_dir: Some(&fx.builtin_dir),
        route: None,
    };
    let ws = fx.ws.to_string_lossy().into_owned();

    // outside / missing / directory / "." → allow。
    let outside = fx
        ._dir
        .path()
        .join("outside.txt")
        .to_string_lossy()
        .into_owned();
    for path in [&outside, "missing.txt", &ws, "."] {
        let v = evaluate(
            &RoutingRequest::FileRead {
                path: path.to_string(),
                cwd: ws.clone(),
                source_tool: Some("test".to_string()),
                range: None,
            },
            &opts,
        )
        .await;
        assert!(matches!(v, Verdict::Allow), "path={path}");
    }
    // big.txt → deny with JS-format reason。
    let v = evaluate(
        &RoutingRequest::FileRead {
            path: "big.txt".to_string(),
            cwd: ws.clone(),
            source_tool: Some("test".to_string()),
            range: None,
        },
        &opts,
    )
    .await;
    let Verdict::Deny { reason } = v else {
        panic!("expected deny");
    };
    assert!(reason.contains("'big.txt' has "));
    assert!(reason.contains(" tokens (threshold 8192)."), "{reason}");
    assert!(reason.contains("cheaper-bulk-read skill"), "{reason}");
    assert!(
        reason.contains("workspace-relative path: big.txt"),
        "{reason}"
    );

    // shell: cat big.txt → blocked-command prefix付き deny。
    let v = evaluate(
        &RoutingRequest::Shell {
            command: "cat big.txt".to_string(),
            cwd: ws.clone(),
            source_tool: Some("test".to_string()),
        },
        &opts,
    )
    .await;
    let Verdict::Deny { reason } = v else {
        panic!("expected deny");
    };
    assert!(
        reason.contains("blocked shell command 'cat big.txt'."),
        "{reason}"
    );
    assert!(
        reason.contains("workspace-relative path: big.txt"),
        "{reason}"
    );

    // allow 系: small・operand 無し・非 read・cwd 遷移後・missing。
    for command in [
        "cat small.txt",
        "cat",
        "ls -la",
        "cd src && cat big.txt",
        "cat missing.txt",
    ] {
        let v = evaluate(
            &RoutingRequest::Shell {
                command: command.to_string(),
                cwd: ws.clone(),
                source_tool: Some("test".to_string()),
            },
            &opts,
        )
        .await;
        assert!(matches!(v, Verdict::Allow), "command={command}");
    }
}

/// `HookTarget::parse` — `HOOK_TARGETS` の 6 値のみ。
#[test]
fn hook_target_parse_table() {
    assert_eq!(HookTarget::parse("windsurf"), Some(HookTarget::Windsurf));
    assert_eq!(HookTarget::parse("copilot"), Some(HookTarget::Copilot));
    assert_eq!(
        HookTarget::parse("pretooluse"),
        Some(HookTarget::Pretooluse)
    );
    assert_eq!(HookTarget::parse("cursor"), Some(HookTarget::Cursor));
    assert_eq!(HookTarget::parse("devin"), Some(HookTarget::Devin));
    assert_eq!(HookTarget::parse("codex"), Some(HookTarget::Codex));
    assert_eq!(HookTarget::parse("claude"), None);
    assert_eq!(HookTarget::parse(""), None);
}

/// `pub(crate)`: sibling `hook_eval_dedup` から再利用。
pub(crate) fn tool_use_payload(tool_name: &str, tool_input: &str) -> String {
    format!(
        "{{\"tool_name\":{},\"tool_input\":{}}}",
        serde_json::to_string(tool_name).unwrap(),
        tool_input
    )
}

#[tokio::test(flavor = "current_thread")]
async fn pretooluse_read_grep_case_variants() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    fx.write("small.txt", b"hello\n");
    // 全文 read(範囲無し)は従来どおり threshold 超過で deny。
    for tool_name in ["Read", "READ", "read", "rEaD"] {
        let r = run_hook(
            &fx,
            &["pre-tool-use", "--target", "pretooluse"],
            &tool_use_payload(tool_name, "{\"file_path\":\"big.txt\"}"),
            &[],
        )
        .await;
        assert_eq!(r.code, 0, "tool_name={tool_name}");
        assert!(
            r.stdout.contains("\"permissionDecision\":\"deny\""),
            "tool_name={tool_name} stdout={:?}",
            r.stdout
        );
    }
    // 範囲 read(#107): window のみ評価する。複数行巨大 file の
    // 小 window は allow、window が大きすぎれば deny、単一行が巨大
    // なら window 自体も deny。
    fx.write_big_lines("big_lines.txt");
    for (tool_input, expect_deny) in [
        (
            "{\"file_path\":\"big_lines.txt\",\"offset\":100,\"limit\":10}",
            false,
        ),
        ("{\"file_path\":\"big_lines.txt\",\"offset\":1}", true),
        ("{\"file_path\":\"big_lines.txt\"}", true),
        // 無効値は全文評価へ縮退 → deny。
        (
            "{\"file_path\":\"big_lines.txt\",\"offset\":0,\"limit\":10}",
            true,
        ),
        (
            "{\"file_path\":\"big_lines.txt\",\"offset\":1.5,\"limit\":10}",
            true,
        ),
        (
            "{\"file_path\":\"big_lines.txt\",\"offset\":100,\"limit\":0}",
            true,
        ),
        // EOF 超過 window は空 → allow。
        (
            "{\"file_path\":\"big_lines.txt\",\"offset\":99999,\"limit\":10}",
            false,
        ),
        ("{\"file_path\":\"big.txt\",\"offset\":1,\"limit\":1}", true),
    ] {
        for tool_name in ["Read", "rEaD"] {
            let r = run_hook(
                &fx,
                &["pre-tool-use", "--target", "pretooluse"],
                &tool_use_payload(tool_name, tool_input),
                &[],
            )
            .await;
            assert_eq!(r.code, 0, "tool_name={tool_name} input={tool_input}");
            assert_eq!(
                r.stdout.contains("\"permissionDecision\":\"deny\""),
                expect_deny,
                "tool_name={tool_name} input={tool_input} stdout={:?}",
                r.stdout
            );
        }
    }
    // grep: pattern が一致する巨大行の実体化は threshold 超過で
    // deny、非一致 pattern は allow。
    for (tool_input, expect_deny) in [
        (
            "{\"path\":\"big.txt\",\"pattern\":\"lorem\",\"max_results\":1}",
            true,
        ),
        (
            "{\"path\":\"big.txt\",\"pattern\":\"zzz-no-such-token\",\"max_results\":1}",
            false,
        ),
    ] {
        for tool_name in ["Grep", "GREP", "grep", "gReP"] {
            let r = run_hook(
                &fx,
                &["pre-tool-use", "--target", "pretooluse"],
                &tool_use_payload(tool_name, tool_input),
                &[],
            )
            .await;
            assert_eq!(r.code, 0, "tool_name={tool_name} input={tool_input}");
            assert_eq!(
                r.stdout.contains("\"permissionDecision\":\"deny\""),
                expect_deny,
                "tool_name={tool_name} input={tool_input} stdout={:?}",
                r.stdout
            );
        }
    }
    let allow = run_hook(
        &fx,
        &["pre-tool-use", "--target", "pretooluse"],
        &tool_use_payload("read", "{\"file_path\":\"small.txt\"}"),
        &[],
    )
    .await;
    assert_eq!(allow.code, 0);
    assert!(allow.stdout.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn pretooluse_grep_path_file_dir_omitted_and_nonstring() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let ws = fx.ws.to_string_lossy().into_owned();
    let dir_input = format!("{{\"path\":{}}}", serde_json::to_string(&ws).unwrap());
    for (tool_input, expect_deny) in [
        // pattern 一致行の実体化が巨大 → deny。
        (
            "{\"path\":\"big.txt\",\"pattern\":\"lorem\",\"max_results\":1}".to_string(),
            true,
        ),
        // dir path だが pattern 無し → normalize 失敗 → allow。
        (dir_input, false),
        // dir 走査(ws)で非一致 → allow。
        ("{\"pattern\":\"zzz-no-such-token\"}".to_string(), false),
        // path が非 string + pattern 無し → allow。
        ("{\"path\":42}".to_string(), false),
    ] {
        let r = run_hook(
            &fx,
            &["pre-tool-use", "--target", "pretooluse"],
            &tool_use_payload("grep", &tool_input),
            &[],
        )
        .await;
        assert_eq!(r.code, 0, "tool_input={tool_input}");
        assert_eq!(
            r.stdout.contains("\"permissionDecision\":\"deny\""),
            expect_deny,
            "tool_input={tool_input} stdout={:?}",
            r.stdout
        );
        if !expect_deny {
            assert!(r.stdout.is_empty(), "tool_input={tool_input}");
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn pretooluse_shell_case_variants_and_fail_open_shapes() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    for tool_name in ["Bash", "bash", "BASH", "PowerShell", "powershell"] {
        let r = run_hook(
            &fx,
            &["pre-tool-use", "--target", "pretooluse"],
            &tool_use_payload(tool_name, "{\"command\":\"cat big.txt\"}"),
            &[],
        )
        .await;
        assert_eq!(r.code, 0, "tool_name={tool_name}");
        assert!(
            r.stdout.contains("\"permissionDecision\":\"deny\""),
            "tool_name={tool_name} stdout={:?}",
            r.stdout
        );
    }
    let allow = run_hook(
        &fx,
        &["pre-tool-use", "--target", "pretooluse"],
        &tool_use_payload("bash", "{\"command\":\"cd src && cat big.txt\"}"),
        &[],
    )
    .await;
    assert_eq!(allow.code, 0);
    assert!(allow.stdout.is_empty());
    for stdin in [
        tool_use_payload("reads", "{\"file_path\":\"big.txt\"}"),
        tool_use_payload("xread", "{\"file_path\":\"big.txt\"}"),
        tool_use_payload("grepx", "{\"path\":\"big.txt\"}"),
        tool_use_payload("bashx", "{\"command\":\"cat big.txt\"}"),
        tool_use_payload("Write", "{\"file_path\":\"big.txt\"}"),
        "{\"tool_name\":\"read\"}".to_string(),
        "{\"tool_name\":\"read\",\"tool_input\":\"x\"}".to_string(),
        "{\"tool_name\":\"read\",\"tool_input\":{\"file_path\":42}}".to_string(),
        "{\"tool_input\":{\"file_path\":\"big.txt\"}}".to_string(),
        "[]".to_string(),
    ] {
        let r = run_hook(
            &fx,
            &["pre-tool-use", "--target", "pretooluse"],
            &stdin,
            &[],
        )
        .await;
        assert_eq!(r.code, 0, "stdin={stdin}");
        assert!(r.stdout.is_empty(), "stdin={stdin} stdout={:?}", r.stdout);
    }
}

/// `pub(crate)`: sibling `hook_eval_dedup` から再利用。
pub(crate) fn devin_payload(tool_name: &str, tool_input: &str) -> String {
    format!(
        "{{\"hook_event_name\":\"PreToolUse\",\"tool_name\":{},\"tool_input\":{}}}",
        serde_json::to_string(tool_name).unwrap(),
        tool_input
    )
}

#[tokio::test(flavor = "current_thread")]
async fn devin_deny_emits_block_json_with_reason_passthrough() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    let reference = run_hook(
        &fx,
        &["pre-tool-use", "--target", "pretooluse"],
        &tool_use_payload("Read", "{\"file_path\":\"big.txt\"}"),
        &[],
    )
    .await;
    let reference_out: serde_json::Value = serde_json::from_str(reference.stdout.trim()).unwrap();
    let reference_reason = reference_out["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap()
        .to_string();
    for tool_name in ["read", "Read", "READ"] {
        let r = run_hook(
            &fx,
            &["pre-tool-use", "--target", "devin"],
            &devin_payload(tool_name, "{\"file_path\":\"big.txt\"}"),
            &[],
        )
        .await;
        assert_eq!(r.code, 0, "tool_name={tool_name}");
        assert!(r.stderr.is_empty(), "tool_name={tool_name}");
        assert!(
            r.stdout
                .starts_with("{\"decision\":\"block\",\"reason\":\"cheaper: 'big.txt' has "),
            "tool_name={tool_name} stdout={:?}",
            r.stdout
        );
        assert!(r.stdout.ends_with("}\n"), "tool_name={tool_name}");
        let out: serde_json::Value = serde_json::from_str(r.stdout.trim()).unwrap();
        let reason = out["reason"].as_str().unwrap();
        assert!(
            reason.contains("workspace-relative path: big.txt"),
            "{reason}"
        );
        assert!(reason.contains('\n'), "{reason}");
        assert_eq!(reason.replace('\n', " "), reference_reason);
    }
    // grep: 一致行の実体化が threshold 超過 → deny。
    let grep = run_hook(
        &fx,
        &["pre-tool-use", "--target", "devin"],
        &devin_payload(
            "Grep",
            "{\"path\":\"big.txt\",\"pattern\":\"lorem\",\"max_results\":1}",
        ),
        &[],
    )
    .await;
    assert_eq!(grep.code, 0);
    assert!(
        grep.stdout.starts_with("{\"decision\":\"block\""),
        "{}",
        grep.stdout
    );
    // 検証済み shell tool `exec` — `cat big.txt` の read candidate が
    // threshold 超過 file を指すため deny。
    let exec = run_hook(
        &fx,
        &["pre-tool-use", "--target", "devin"],
        &devin_payload("exec", "{\"command\":\"cat big.txt\"}"),
        &[],
    )
    .await;
    assert_eq!(exec.code, 0);
    assert!(
        exec.stdout.starts_with("{\"decision\":\"block\""),
        "{}",
        exec.stdout
    );
}

#[tokio::test(flavor = "current_thread")]
async fn devin_event_gate_and_fail_open_are_silent() {
    let fx = Fixture::new();
    fx.write_big("big.txt");
    fx.write("small.txt", b"hello\n");
    let ws = fx.ws.to_string_lossy().into_owned();
    let cases: Vec<(&[&str], String)> = vec![
        (
            &["pre-read-code", "--target", "devin"],
            devin_payload(
                "read",
                "{\"file_path\":\"big.txt\",\"offset\":334,\"limit\":81}",
            ),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            "{\"hook_event_name\":\"PostToolUse\",\"tool_name\":\"read\",\"tool_input\":{\"file_path\":\"big.txt\",\"offset\":334,\"limit\":81}}"
                .to_string(),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            "{\"tool_name\":\"read\",\"tool_input\":{\"file_path\":\"big.txt\",\"offset\":334,\"limit\":81}}".to_string(),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            "{\"hook_event_name\":42,\"tool_name\":\"read\",\"tool_input\":{\"file_path\":\"big.txt\",\"offset\":334,\"limit\":81}}"
                .to_string(),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            devin_payload("bash", "{\"command\":\"cat big.txt\"}"),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            devin_payload("write", "{\"file_path\":\"big.txt\"}"),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            devin_payload("read", "{\"file_path\":\"small.txt\"}"),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            devin_payload("read", "{\"file_path\":\"missing.txt\"}"),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            devin_payload(
                "grep",
                &format!("{{\"path\":{}}}", serde_json::to_string(&ws).unwrap()),
            ),
        ),
        (
            &["pre-tool-use", "--target", "devin"],
            devin_payload("grep", "{\"pattern\":\"x\"}"),
        ),
        (&["pre-tool-use", "--target", "devin"], "not json".to_string()),
        (&["pre-tool-use", "--target", "devin"], "[]".to_string()),
    ];
    for (args, stdin) in cases {
        let r = run_hook(&fx, args, &stdin, &[]).await;
        assert_eq!(r.code, 0, "args={args:?} stdin={stdin}");
        assert!(r.stdout.is_empty(), "args={args:?} stdout={:?}", r.stdout);
        assert!(r.stderr.is_empty(), "args={args:?} stderr={:?}", r.stderr);
    }
}

const TARGETS_JSON_LINE: &str =
    "{\"schemaVersion\":1,\"targets\":[\"windsurf\",\"copilot\",\"pretooluse\",\"cursor\",\"devin\",\"codex\"],\"hookConfig\":true}\n";

#[tokio::test(flavor = "current_thread")]
async fn hook_targets_json_emits_capability_line_without_usage() {
    let fx = Fixture::new();
    let r = run_hook(&fx, &["targets", "--json"], "ignored stdin", &[]).await;
    assert_eq!(r.code, 0);
    assert_eq!(r.stdout, TARGETS_JSON_LINE);
    assert!(r.stderr.is_empty());
    assert!(!Path::new(&fx.env["CHEAPER_CONFIG_DIR"])
        .join("usage.jsonl")
        .exists());
}

#[tokio::test(flavor = "current_thread")]
async fn hook_targets_json_does_not_read_stdin() {
    let fx = Fixture::new();
    let env_map = fx.env.clone();
    let env_lookup = move |name: &str| env_map.get(name).cloned();
    let mut stdin = PanicStdin;
    let mut out = VecSink(Vec::new());
    let mut err = VecSink(Vec::new());
    let mut io = CliIo {
        stdin: &mut stdin,
        stdout: &mut out,
        stderr: &mut err,
        env: &env_lookup,
        cwd: fx.ws.to_string_lossy().into_owned(),
    };
    let hooks = CliHooks {
        builtin_dir: Some(fx.builtin_dir.clone()),
        ..Default::default()
    };
    let argv: Vec<String> = ["hook", "targets", "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let code = run_cli(&argv, &mut io, &hooks).await;
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out.0).unwrap(), TARGETS_JSON_LINE);
    assert!(String::from_utf8(err.0).unwrap().is_empty());
}

#[test]
fn hook_target_parse_as_str_roundtrip() {
    for name in ["windsurf", "copilot", "pretooluse", "cursor", "devin"] {
        assert_eq!(
            HookTarget::parse(name).map(|t| t.as_str()),
            Some(name),
            "target={name}"
        );
    }
    for name in ["Devin", "DEVIN", "claude", ""] {
        assert_eq!(HookTarget::parse(name), None, "target={name}");
    }
}
