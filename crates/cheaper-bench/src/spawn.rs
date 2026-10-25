//! product binary / fixture child の blocking spawn(design §1.1・§7
//! nonzero-exit capture path)。
//!
//! - `std::process::Command` の blocking spawn で逐次実行(async
//!   runtime は導入しない — KISS)。wall-time は `Instant::now` 前後差。
//! - stdout/stderr は常に capture し **exit≠0 で throw しない** —
//!   失敗系計測(worker-failure-modes・route-child-nonzero-exit)の
//!   前提。exit code・生 byte をそのまま観測値として返す。
//! - stdout の envelope parse は best-effort(解釈不能は `envelopeOk`
//!   判定材料 — per-mode parser 列は T2 `measure.rs`)。
//! - fixture child は `current_exe()` abs path 直 spawn・shell 非経由・
//!   `Command` API のみ(Windows/Unix parity — quoting 差を排除)。

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cheaper_core::errors::{CheaperError, ErrorCode};

/// spawn 1 回分の観測出力。nonzero exit でも `Ok` で返る — 失敗は
/// `exit_code`/`stdout`/`stderr` の観測値。
#[derive(Debug)]
pub struct Spawned {
    /// 生 stdout byte(envelope parse・trailer regex・byte 計測の素材。
    /// UTF-8 非適合も捨てない)。
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// process exit code。signal kill 等 code 非付与は `None`
    /// (観測不能を 0/-1 へ潰さない)。
    pub exit_code: Option<i32>,
    /// spawn〜wait 完了までの wall-time。
    pub wall_time: Duration,
}

impl Spawned {
    /// stdout の lossy UTF-8 view(parser 系は String 入力を取る)。
    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// spawn 入力 — env は `env::build` が組立てた **完全 map**
/// (`env_clear` + `envs` で渡し ambient を混入させない。v1
/// `spawn(…, {env})` の全置換 semantics parity)。
pub struct SpawnOptions<'a> {
    pub cwd: &'a Path,
    pub env: &'a std::collections::BTreeMap<String, String>,
    /// stdin へ書いて閉じる input(省略時は stdin を閉じるのみ —
    /// `child.stdin.end()` parity)。
    pub stdin: Option<&'a [u8]>,
}

/// `executable` を `args` で blocking spawn し capture を返す。
/// spawn 自体の失敗(binary 不在等)のみ `Err` — exit≠0 は `Ok`。
pub fn run(
    executable: &Path,
    args: &[String],
    options: &SpawnOptions<'_>,
) -> Result<Spawned, CheaperError> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(options.cwd)
        .env_clear()
        .envs(options.env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match options.stdin {
        Some(_) => {
            command.stdin(Stdio::piped());
        }
        None => {
            command.stdin(Stdio::null());
        }
    }
    let started = Instant::now();
    let mut child = command.spawn().map_err(|e| {
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("spawn failed {}: {e}", executable.display()),
        )
    })?;
    if let Some(input) = options.stdin {
        // stdin へ input を書き drop で閉じる(`child.stdin.end(input)`
        // parity)。write 失敗(EPIPE — 早期 exit 済み child)は
        // 観測対象の失敗形として潰さず exit code 側で観測する。
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input);
        }
    }
    let output = child.wait_with_output().map_err(|e| {
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!("wait failed {}: {e}", executable.display()),
        )
    })?;
    Ok(Spawned {
        stdout: output.stdout,
        stderr: output.stderr,
        exit_code: output.status.code(),
        wall_time: started.elapsed(),
    })
}
