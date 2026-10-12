//! Devin CLI adapter — cheaper 専用 data dir 環境の child process
//! （TS `providers/devin/cli.ts` の port。upstream pi-devin（MIT）の
//! `src/cli.ts` を isolated data dir・runner seam 向けに再設計した
//! ものの Rust 版 — ADR-0029 相当）。
//!
//! - argv 分離起動（`std::process::Command`）— shell string は使わない
//! - child env: `{...base_env, XDG_DATA_HOME: dataDir}`、Windows は
//!   `LOCALAPPDATA` 併用。呼び出し側 env / process.env は変更しない
//! - `DEVIN_CLI` env で binary path override、既定は `"devin"`
//!   （`Command` の PATH 解決に委ねる）
//! - `executable not found` は `DevinCliNotFoundError` へ写像

use std::collections::BTreeMap;

use cheaper_core::errors::{CheaperError, ErrorCode};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// TS `execFile` の既定 timeout（`timeout: opts.timeoutMs ?? 30_000`）。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// `version()` の timeout（TS 8_000ms）。
const VERSION_TIMEOUT: Duration = Duration::from_secs(8);

/// `devin` child process の結果（`{stdout, stderr, code}`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevinCliResult {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// runner への 1 回分の実行 option（`env` は isolated child env —
/// 呼び出し側が `devin_child_env` 済みのものを渡す）。
pub struct CliRunOptions<'a> {
    pub env: &'a BTreeMap<String, String>,
    pub timeout: Option<Duration>,
    /// `stdio: "inherit"` 相当（login の対話 flow）。
    pub inherit_stdio: bool,
}

/// `DevinCliRunner` — test が fake runner を注入する seam。
/// `Err` は spawn 失敗（`io::ErrorKind::NotFound` は
/// `DevinCliNotFoundError` へ写像される）。
pub trait DevinCliRunner: Send + Sync {
    fn run(
        &self,
        bin: &str,
        args: &[String],
        opts: &CliRunOptions<'_>,
    ) -> Result<DevinCliResult, std::io::Error>;
}

/// 本番 runner: `Command`（pipe / inherit stdio + timeout）。
pub struct ProcessCliRunner;

impl DevinCliRunner for ProcessCliRunner {
    fn run(
        &self,
        bin: &str,
        args: &[String],
        opts: &CliRunOptions<'_>,
    ) -> Result<DevinCliResult, std::io::Error> {
        let mut command = Command::new(bin);
        command.args(args).envs(opts.env.iter());
        if opts.inherit_stdio {
            // TS spawn({stdio:"inherit"}) — stdout/stderr は capture せず
            // exit code のみ返す（この経路では timeout 非適用、TS 同様）。
            let status = command
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()?;
            return Ok(DevinCliResult {
                stdout: String::new(),
                stderr: String::new(),
                code: status.code().unwrap_or(1),
            });
        }

        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // pipe デッドロック防止: stdout/stderr は reader thread で回収。
        let mut stdout = child.stdout.take().expect("stdout piped");
        let mut stderr = child.stderr.take().expect("stderr piped");
        let (tx, rx) = mpsc::channel::<std::io::Result<(bool, Vec<u8>)>>();
        let tx_err = tx.clone();
        let stdout_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let res = stdout.read_to_end(&mut buf).map(|_| (true, buf));
            let _ = tx.send(res);
        });
        let stderr_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let res = stderr.read_to_end(&mut buf).map(|_| (false, buf));
            let _ = tx_err.send(res);
        });

        let deadline = opts.timeout.map(|t| Instant::now() + t);
        let status = loop {
            match child.try_wait()? {
                Some(status) => break status,
                None => {
                    if deadline.is_some_and(|d| Instant::now() >= d) {
                        // execFile timeout — kill して error を返す。
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = rx.recv();
                        let _ = rx.recv();
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!(
                                "devin timed out after {:?}",
                                opts.timeout.unwrap_or_default()
                            ),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        };
        let mut out_buf = Vec::new();
        let mut err_buf = Vec::new();
        for _ in 0..2 {
            if let Ok(Ok((is_stdout, buf))) = rx.recv() {
                if is_stdout {
                    out_buf = buf;
                } else {
                    err_buf = buf;
                }
            }
        }
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();
        Ok(DevinCliResult {
            stdout: String::from_utf8_lossy(&out_buf).into_owned(),
            stderr: String::from_utf8_lossy(&err_buf).into_owned(),
            code: status.code().unwrap_or(1),
        })
    }
}

/// `DevinCliNotFoundError` — binary 未検出時の actionable error。
#[derive(Debug, thiserror::Error)]
#[error("Devin CLI not found ({0}). Install the Devin CLI, or set DEVIN_CLI to the devin binary.")]
pub struct DevinCliNotFoundError(pub String);

/// `cli.run` が返しうる error。
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// `devin` 実行ファイルが見つからない。
    #[error(transparent)]
    NotFound(#[from] DevinCliNotFoundError),
    /// spawn/IO 失敗（timeout 含む）。
    #[error("devin CLI error: {0}")]
    Io(#[from] std::io::Error),
}

/// `CliError` → `CheaperError` の単一変換点 — devin plugin の全経路
/// で `PROVIDER_FAILURE` + cause 付き error に統一する。
pub(crate) fn cli_error(e: CliError) -> CheaperError {
    CheaperError::new(ErrorCode::ProviderFailure, e.to_string()).with_cause(e)
}

/// `error.code` field 用の分類値（metadata-only — message/payload は
/// field に載せない）。
fn cli_error_code(e: &CliError) -> &'static str {
    match e {
        CliError::NotFound(_) => "NOT_FOUND",
        CliError::Io(_) => "IO_ERROR",
    }
}

/// `devinChildEnv` — isolated child env を組み立てる。`XDG_DATA_HOME`
/// に data dir、Windows は `LOCALAPPDATA` 併用。入力・process env は
/// 変更しない。
pub fn devin_child_env(
    base: &BTreeMap<String, String>,
    data_dir: &Path,
) -> BTreeMap<String, String> {
    let mut child = base.clone();
    let data = data_dir.to_string_lossy().into_owned();
    child.insert("XDG_DATA_HOME".to_string(), data.clone());
    if cfg!(windows) {
        child.insert("LOCALAPPDATA".to_string(), data);
    }
    child
}

/// `DevinCli` — binary 名 + isolated env + runner を束ねる。
pub struct DevinCli {
    /// `DEVIN_CLI` env または `"devin"`。
    pub bin: String,
    child_env: BTreeMap<String, String>,
    runner: std::sync::Arc<dyn DevinCliRunner>,
}

impl DevinCli {
    /// `createDevinCli`。`env` は child env の base（plugin は
    /// process env + scoped env の `DEVIN_CLI` を適用済みで渡す）。
    pub fn new(
        env: BTreeMap<String, String>,
        data_dir: &Path,
        runner: Option<std::sync::Arc<dyn DevinCliRunner>>,
    ) -> Self {
        let bin = env
            .get("DEVIN_CLI")
            .filter(|v| !v.is_empty())
            .cloned()
            .unwrap_or_else(|| "devin".to_string());
        Self {
            bin,
            child_env: devin_child_env(&env, data_dir),
            runner: runner.unwrap_or_else(|| std::sync::Arc::new(ProcessCliRunner)),
        }
    }

    /// `cli.run(args)` — argv 分離 + isolated env。
    ///
    /// `cheaper.provider.devin` event(`devin models list` 起動・終了を
    /// 含む CLI 実行境界 — design §instrumentation 配置)。`command` は
    /// subcommand 先頭語のみの分類値で、argv 全文・出力・env は載せない。
    pub fn run(
        &self,
        args: &[&str],
        timeout: Option<Duration>,
        inherit_stdio: bool,
    ) -> Result<DevinCliResult, CliError> {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let opts = CliRunOptions {
            env: &self.child_env,
            timeout: timeout.or(Some(DEFAULT_TIMEOUT)),
            inherit_stdio,
        };
        let command = args.first().map(String::as_str).unwrap_or("");
        tracing::debug!(target: "cheaper.provider.devin", command = command, "devin cli start");
        // TS `isNotFound`: `error.code === "ENOENT" || error.code === 127`。
        // spawn 段の ENOENT は runner の `io::ErrorKind::NotFound`、
        // shell wrapper 等が返す exit 127 は完了 result の `code` に
        // 乗る — どちらも `DevinCliNotFoundError` へ写像する。
        let result = self
            .runner
            .run(&self.bin, &args, &opts)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CliError::NotFound(DevinCliNotFoundError(self.bin.clone()))
                } else {
                    CliError::Io(e)
                }
            })
            .and_then(|result| {
                if result.code == 127 {
                    Err(CliError::NotFound(DevinCliNotFoundError(self.bin.clone())))
                } else {
                    Ok(result)
                }
            });
        match &result {
            Ok(r) => {
                tracing::debug!(
                    target: "cheaper.provider.devin",
                    command = command,
                    exit.code = r.code,
                    "devin cli end"
                );
            }
            Err(e) => {
                tracing::debug!(
                    target: "cheaper.provider.devin",
                    command = command,
                    error.code = cli_error_code(e),
                    "devin cli end"
                );
            }
        }
        result
    }

    /// `cli.version()` — trimmed stdout、取得不能時 `None`。
    pub fn version(&self) -> Option<String> {
        let result = self.run(&["version"], Some(VERSION_TIMEOUT), false).ok()?;
        if result.code != 0 {
            return None;
        }
        let trimmed = result.stdout.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }
}
