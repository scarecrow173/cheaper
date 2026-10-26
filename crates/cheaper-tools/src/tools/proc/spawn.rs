//! spawn 責務: command 構築(stdin ignore/pipe/windowsHide/detached)と
//! spawn/exit error の Node 写像、表示 fold。

use std::process::Stdio;

use tokio::process::Command;

use super::monitor::StreamCaptureResult;
use super::ProcPrepared;

pub(super) fn spawn_code(e: &std::io::Error) -> &'static str {
    if let Some(code) = e.raw_os_error() {
        #[cfg(unix)]
        let name = match code {
            libc::EPERM => Some("EPERM"),
            libc::ENOENT => Some("ENOENT"),
            libc::EINTR => Some("EINTR"),
            libc::EIO => Some("EIO"),
            libc::ENXIO => Some("ENXIO"),
            libc::E2BIG => Some("E2BIG"),
            libc::ENOEXEC => Some("ENOEXEC"),
            libc::EBADF => Some("EBADF"),
            libc::EAGAIN => Some("EAGAIN"),
            libc::ENOMEM => Some("ENOMEM"),
            libc::EACCES => Some("EACCES"),
            libc::EFAULT => Some("EFAULT"),
            libc::EBUSY => Some("EBUSY"),
            libc::EEXIST => Some("EEXIST"),
            libc::ENODEV => Some("ENODEV"),
            libc::ENOTDIR => Some("ENOTDIR"),
            libc::EISDIR => Some("EISDIR"),
            libc::EINVAL => Some("EINVAL"),
            libc::ENFILE => Some("ENFILE"),
            libc::EMFILE => Some("EMFILE"),
            libc::ETXTBSY => Some("ETXTBSY"),
            libc::ENOSPC => Some("ENOSPC"),
            libc::EROFS => Some("EROFS"),
            libc::EPIPE => Some("EPIPE"),
            libc::ENAMETOOLONG => Some("ENAMETOOLONG"),
            libc::ELOOP => Some("ELOOP"),
            _ => None,
        };
        #[cfg(windows)]
        let name = match code {
            2 | 3 | 161 => Some("ENOENT"), // FILE_NOT_FOUND/PATH_NOT_FOUND/BAD_PATHNAME
            4 => Some("EMFILE"),           // TOO_MANY_OPEN_FILES
            5 | 32 | 82 => Some("EACCES"), // ACCESS_DENIED/SHARING_VIOLATION/CANNOT_MAKE
            6 => Some("EBADF"),            // INVALID_HANDLE
            8 | 14 => Some("ENOMEM"),      // NOT_ENOUGH_MEMORY / OUTOFMEMORY
            87 => Some("EINVAL"),          // INVALID_PARAMETER
            193 => Some("EFTYPE"),         // BAD_EXE_FORMAT(Node 実測 "EFTYPE")
            267 => Some("ENOTDIR"),        // DIRECTORY
            _ => None,
        };
        #[cfg(not(any(unix, windows)))]
        let name: Option<&'static str> = None;
        if let Some(name) = name {
            return name;
        }
        if e.raw_os_error().is_some() && cfg!(windows) {
            // libuv は unmapped win32 error を `UV__UNKNOWN` に写す。
            return "UNKNOWN";
        }
    }
    match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT",
        std::io::ErrorKind::PermissionDenied => "EACCES",
        _ => "SPAWN_ERROR",
    }
}

/// spawn 失敗の message shape。Node は `ChildProcess.spawn` が
/// uv_spawn の sync error で throw する経路と、`error` event が非同期で
/// 届く経路を持つ:
///
/// - unix: `fork`+`exec` 失敗は error pipe 経由で常に非同期 — raw errno
///   を持つ失敗は全て `"managed process failed"` + `spawnCode`。
/// - win32: `uv_spawn` が `CreateProcessW` 失敗を同期返すため
///   `"managed process spawn failed"` + `cause` 相当になるが、ENOENT
///   だけは Node が非同期 `error` event として扱う(実測)ため
///   `"managed process failed"` + `spawnCode: "ENOENT"` に揃える。
/// - `raw_os_error` を持たない構築 error(引数検証等)は sync throw 経路
///   の `"managed process spawn failed"`。
///
/// 残留差分: musl 環境では Node が ENOEXEC を同期 throw し得る
/// (IR doc 参照)。
pub(super) fn spawn_failure_is_async(e: &std::io::Error) -> bool {
    e.raw_os_error().is_some() && (cfg!(unix) || spawn_code(e) == "ENOENT")
}

/// unix `ExitStatus::signal()` → Node signal 名。
#[cfg(unix)]
fn signal_name(sig: i32) -> Option<String> {
    Some(
        match sig {
            libc::SIGHUP => "SIGHUP",
            libc::SIGINT => "SIGINT",
            libc::SIGQUIT => "SIGQUIT",
            libc::SIGILL => "SIGILL",
            libc::SIGTRAP => "SIGTRAP",
            libc::SIGABRT => "SIGABRT",
            libc::SIGBUS => "SIGBUS",
            libc::SIGFPE => "SIGFPE",
            libc::SIGKILL => "SIGKILL",
            libc::SIGUSR1 => "SIGUSR1",
            libc::SIGSEGV => "SIGSEGV",
            libc::SIGUSR2 => "SIGUSR2",
            libc::SIGPIPE => "SIGPIPE",
            libc::SIGALRM => "SIGALRM",
            libc::SIGTERM => "SIGTERM",
            libc::SIGCHLD => "SIGCHLD",
            libc::SIGCONT => "SIGCONT",
            libc::SIGSTOP => "SIGSTOP",
            libc::SIGTSTP => "SIGTSTP",
            libc::SIGTTIN => "SIGTTIN",
            libc::SIGTTOU => "SIGTTOU",
            libc::SIGURG => "SIGURG",
            libc::SIGXCPU => "SIGXCPU",
            libc::SIGXFSZ => "SIGXFSZ",
            libc::SIGVTALRM => "SIGVTALRM",
            libc::SIGPROF => "SIGPROF",
            libc::SIGWINCH => "SIGWINCH",
            libc::SIGIO => "SIGIO",
            _ => return None,
        }
        .to_string(),
    )
}

pub(super) fn status_parts(status: &std::process::ExitStatus) -> (Option<i64>, Option<String>) {
    if let Some(code) = status.code() {
        return (Some(code as i64), None);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return (None, signal_name(sig));
        }
    }
    (None, None)
}

/// `build_command`: `(executable, argv, cwd, env)` 引数へ汎化済み —
/// `ProcessTask` 依存は持たない(design §(e))。
pub(super) fn build_command(prepared: &ProcPrepared) -> Command {
    let mut command = Command::new(&prepared.executable);
    command
        .args(&prepared.argv)
        .current_dir(&prepared.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear();
    for (key, value) in &prepared.env {
        command.env(key, value);
    }
    // 相関値は偽装可能な明示 hint であり、policy/authorization に使わない。
    // env_clear 後、実行 owner が値を渡した場合だけ設定する。
    if let Some(correlation) = &prepared.correlation {
        if let Some(trace_id) = &correlation.trace_id {
            command.env("CHEAPER_TRACE_ID", trace_id);
        }
        if let Some(parent_execution_id) = &correlation.parent_execution_id {
            command.env("CHEAPER_PARENT_EXECUTION_ID", parent_execution_id);
        }
    }
    #[cfg(windows)]
    {
        // `creation_flags` は tokio::process::Command の inherent method。
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW(windowsHide)
    }
    #[cfg(unix)]
    {
        // `detached: true`(非 win32)相当: 新 process group で
        // `kill(-pid)` 対象になる。
        command.process_group(0);
    }
    command
}

/// `display(value)`: 表示だけを 64 UTF-16 code units で折り返す。raw
/// stdout/stderr は details に保持し、同期 BPE の長い run を防ぐ。
/// 補助面 char が境界を跨がない(TS surrogate pair backoff parity)。
pub(super) fn display(value: &StreamCaptureResult) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut units = 0usize;
    for ch in value.text.chars() {
        let u = ch.len_utf16();
        if units + u > 64 && !current.is_empty() {
            parts.push(std::mem::take(&mut current));
            units = 0;
        }
        current.push(ch);
        units += u;
    }
    if !current.is_empty() {
        parts.push(current);
    }
    format!(
        "{} bytes, {} captured, truncated={}\n{}",
        value.bytes,
        value.captured_bytes,
        value.truncated,
        parts.join("\n")
    )
}
