//! `run_cli` dispatch smoke + `__grep-eval` 早期分岐 test。

use std::process::Command;

use cheaper_cli::cli::{run_cli, CliHooks, CliIo, CliStdin, CliWrite};

struct VecSink(Vec<u8>);
impl CliWrite for VecSink {
    fn write_str(&mut self, s: &str) {
        self.0.extend_from_slice(s.as_bytes());
    }
}

struct PipeStdin {
    data: Vec<u8>,
    tty: bool,
}
impl CliStdin for PipeStdin {
    fn is_tty(&self) -> bool {
        self.tty
    }
    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        Ok(std::mem::take(&mut self.data))
    }
}

fn env_lookup(name: &str) -> Option<String> {
    // test isolation: CHEAPER_CONFIG_DIR 等を孤立 dir へ向けるのは
    // caller の責務 — ここでは process env をそのまま使う。
    std::env::var(name).ok()
}

async fn run(argv: &[&str]) -> (i32, String, String) {
    let mut stdin = PipeStdin {
        data: Vec::new(),
        tty: true,
    };
    let mut out = VecSink(Vec::new());
    let mut err = VecSink(Vec::new());
    let mut io = CliIo {
        stdin: &mut stdin,
        stdout: &mut out,
        stderr: &mut err,
        env: &env_lookup,
        cwd: std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    };
    let hooks = CliHooks::default();
    let code = run_cli(
        &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        &mut io,
        &hooks,
    )
    .await;
    (
        code,
        String::from_utf8(out.0).unwrap(),
        String::from_utf8(err.0).unwrap(),
    )
}

#[tokio::test(flavor = "current_thread")]
async fn help_writes_help_no_envelope() {
    let (code, out, err) = run(&["--help"]).await;
    assert_eq!(code, 0);
    assert!(out.starts_with("Usage: cheaper <command> [options]\n"));
    assert!(err.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn version_writes_version_line() {
    let (code, out, _err) = run(&["--version"]).await;
    assert_eq!(code, 0);
    // embedded version.json(= packages/cli/package.json の version)
    let pkg: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/cli/package.json"
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(out, format!("{}\n", pkg["version"].as_str().unwrap()));
}

#[tokio::test(flavor = "current_thread")]
async fn unknown_command_error_envelope() {
    let (code, out, err) = run(&["frobnicate"]).await;
    assert_eq!(code, 2);
    let v: serde_json::Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert_eq!(v["error"]["exitCode"], 2);
    assert_eq!(v["error"]["message"], "unknown command: frobnicate");
    assert!(err.starts_with("error INVALID_REQUEST: unknown command"));
    // stdout は envelope 1 JSON + 末尾改行のみ
    assert!(out.ends_with("}\n"));
    assert_eq!(out.matches('\n').count(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn missing_command_error_envelope() {
    let (code, out, _err) = run(&[]).await;
    assert_eq!(code, 2);
    let v: serde_json::Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert!(v["meta"]["durationMs"].is_number());
}

// --- `__grep-eval` 早期分岐 ---------------------------------------------------
// 実 binary を spawn して helper 経路(envelope なし・exit 0・stdin EOF
// 正常終了)を確認する。`CARGO_BIN_EXE_<name>` は cargo test が提供。

#[test]
fn grep_eval_subprocess_dispatch() {
    let exe = env!("CARGO_BIN_EXE_cheaper");
    let output = Command::new(exe)
        .arg("__grep-eval")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("spawn cheaper __grep-eval");
    assert_eq!(output.status.code(), Some(0));
    // helper は先頭に隔離用の 1 改行を出すだけ — envelope ではない。
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("\"schemaVersion\""),
        "internal helper must not emit envelope: {stdout:?}"
    );
}

#[test]
fn normal_dispatch_still_works() {
    // `__grep-eval` 分岐が通常 command を妨げないこと。
    let exe = env!("CARGO_BIN_EXE_cheaper");
    let output = Command::new(exe)
        .arg("--help")
        .output()
        .expect("spawn cheaper --help");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("Usage: cheaper"));
}
