//! Port of `packages/tool-runtime/src/process-tree.ts`.
//!
//! OS containment は提供しない。POSIX は `kill(-pid)` の process group
//! SIGTERM → live poll → SIGKILL(/proc zombie 判定付き)。Windows は
//! powershell CIM snapshot(pid/parent/created) + `taskkill /T /F` で、
//! PID reuse 防止として creation timestamp 照合(bornBefore 規則)を
//! 持つ。固定 OS helper は 10s deadline + 1MB output cap で bounded。
//!
//! Windows の snapshot 前の ancestry-loss は W1 limitation(TS 踏襲)。

use std::time::{Duration, Instant};

use tokio::time::sleep;

use crate::ToolError;

const HELPER_TIMEOUT_MS: u64 = 10_000;
#[cfg(windows)]
const HELPER_OUTPUT_BYTES: usize = 1_048_576;

fn failure(message: impl Into<String>) -> ToolError {
    std::io::Error::other(message.into()).into()
}

// ---------------------------------------------------------------------------
// POSIX backend
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod posix {
    use super::*;

    /// `process.kill(-pid, signal)`。ESRCH → `false`、その他は伝播。
    fn send(pid: u32, signal: i32) -> Result<bool, ToolError> {
        // SAFETY: `kill` with a negative pid targets the process group.
        let rc = unsafe { libc::kill(-(pid as i32), signal) };
        if rc == 0 {
            return Ok(true);
        }
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(libc::ESRCH) {
            return Ok(false);
        }
        Err(e.into())
    }

    /// group 内に非 zombie の稼働 member がいるか。Linux では SIGKILL 後の
    /// zombie を稼働 descendant と区別するため `/proc` を走査する。
    async fn live(pid: u32) -> Result<bool, ToolError> {
        if !send(pid, 0)? {
            return Ok(false);
        }
        if !cfg!(target_os = "linux") {
            return Ok(true);
        }
        let mut entries = tokio::fs::read_dir("/proc").await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.bytes().any(|b| !b.is_ascii_digit()) {
                continue;
            }
            let stat = match tokio::fs::read_to_string(entry.path().join("stat")).await {
                Ok(s) => s,
                Err(e) => {
                    // 消滅 race(ENOENT)は読み飛ばす。
                    if e.raw_os_error() == Some(libc::ENOENT) {
                        continue;
                    }
                    return Err(e.into());
                }
            };
            // `pid (comm) state ppid pgrp ...` — comm は `)` まで読み、
            // fields[0]=state, fields[1]=ppid, fields[2]=pgrp(TS parity)。
            let Some(close) = stat.rfind(')') else {
                continue;
            };
            let fields: Vec<&str> = stat[close + 2..].split(' ').collect();
            if fields.len() > 2
                && fields[2].parse::<i64>() == Ok(pid as i64)
                && fields[0] != "Z"
                && fields[0] != "X"
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn terminate(pid: u32, grace_ms: u64) -> Result<(), ToolError> {
        if !send(pid, libc::SIGTERM)? {
            return Ok(());
        }
        let deadline = Instant::now() + Duration::from_millis(grace_ms);
        while live(pid).await? && Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            sleep(
                remaining
                    .min(Duration::from_millis(25))
                    .max(Duration::from_millis(1)),
            )
            .await;
        }
        if !live(pid).await? {
            return Ok(());
        }
        send(pid, libc::SIGKILL)?;
        let force_deadline = Instant::now() + Duration::from_millis(HELPER_TIMEOUT_MS);
        while live(pid).await? {
            if Instant::now() >= force_deadline {
                return Err(failure("managed process group survived force termination"));
            }
            sleep(Duration::from_millis(10)).await;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Windows backend
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use std::collections::{HashMap, HashSet};
    use std::process::Stdio;
    use std::sync::{Arc, Mutex};

    use cheaper_core::nodepath;
    use serde_json::Value;
    use tokio::io::AsyncReadExt;
    use tokio::process::Command;
    use tokio::task::JoinHandle;

    use super::*;

    /// `CREATE_NO_WINDOW`(`windowsHide: true` 相当)。
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn system_root() -> String {
        std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string())
    }

    /// 固定 OS helper の出力・実行時間を制限し、close まで回収する。
    /// `stdio: ["ignore","pipe","ignore"]`・`shell:false`・`windowsHide`・
    /// env は `{SystemRoot, PATH:""}` のみ(TS parity)。
    async fn helper(executable: &str, args: &[String]) -> Result<String, ToolError> {
        let mut command = Command::new(executable);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env_clear()
            .env("SYSTEMROOT", system_root())
            .env("PATH", "")
            .creation_flags(CREATE_NO_WINDOW);
        let mut child = command.spawn()?;
        let mut stdout = child.stdout.take().expect("stdout piped");
        let mut chunks: Vec<u8> = Vec::new();
        // `stop` 相当: 初発 error のみ保持し kill する。
        let mut stop: Option<ToolError> = None;
        let deadline = tokio::time::Instant::from_std(
            Instant::now() + Duration::from_millis(HELPER_TIMEOUT_MS),
        );
        let mut buf = [0u8; 8192];
        let mut code: Option<i32> = None;
        // TS は `close`(exit + stdout EOF)まで回収する。process 終了後も
        // pipe 残 data を drain してから status を確定させる。
        let mut exited = false;
        let mut eof = false;
        loop {
            if stop.is_some() {
                let _ = child.kill().await;
                let _ = child.wait().await;
                break;
            }
            if exited && eof {
                break;
            }
            tokio::select! {
                read = stdout.read(&mut buf), if !eof => {
                    match read {
                        Ok(0) => eof = true,
                        Ok(n) => {
                            if chunks.len() + n > HELPER_OUTPUT_BYTES {
                                stop = Some(super::failure(
                                    "process tree helper output exceeded",
                                ));
                            } else {
                                chunks.extend_from_slice(&buf[..n]);
                            }
                        }
                        Err(e) => stop = Some(e.into()),
                    }
                }
                status = child.wait(), if !exited => {
                    code = status?.code();
                    exited = true;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    stop = Some(super::failure(
                        "process tree helper deadline exceeded",
                    ));
                }
            }
        }
        if let Some(error) = stop {
            return Err(error);
        }
        if code != Some(0) {
            return Err(super::failure(format!(
                "process tree helper failed ({})",
                code.map_or_else(|| "null".to_string(), |c| c.to_string())
            )));
        }
        Ok(String::from_utf8_lossy(&chunks).into_owned())
    }

    /// CIM snapshot の 1 row。`created` は ISO 8601(`o`)文字列。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct Identity {
        pub(super) pid: i64,
        pub(super) parent: i64,
        pub(super) created: String,
    }

    const QUERY: &str = "$ErrorActionPreference='Stop'; @(Get-CimInstance Win32_Process | Select-Object @{n='pid';e={[int]$_.ProcessId}},@{n='parent';e={[int]$_.ParentProcessId}},@{n='created';e={$_.CreationDate.ToUniversalTime().ToString('o')}}) | ConvertTo-Json -Compress";

    async fn snapshot() -> Result<Vec<Identity>, ToolError> {
        let executable = nodepath::join(&[
            &system_root(),
            "System32",
            "WindowsPowerShell",
            "v1.0",
            "powershell.exe",
        ]);
        let text = helper(
            &executable,
            &[
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-Command".to_string(),
                QUERY.to_string(),
            ],
        )
        .await?;
        let data: Value = serde_json::from_str(&text)
            .map_err(|e| super::failure(format!("invalid process identity snapshot: {e}")))?;
        let invalid = || super::failure("invalid process identity snapshot");
        let rows = data.as_array().ok_or_else(invalid)?;
        let mut out = Vec::with_capacity(rows.len());
        for p in rows {
            let pid = p["pid"].as_i64().ok_or_else(invalid)?;
            let parent = p["parent"].as_i64().ok_or_else(invalid)?;
            let created = p["created"].as_str().ok_or_else(invalid)?;
            // `Number.isSafeInteger` parity(|v| <= 2^53-1)。
            if pid.unsigned_abs() > 9_007_199_254_740_991
                || parent.unsigned_abs() > 9_007_199_254_740_991
            {
                return Err(invalid());
            }
            out.push(Identity {
                pid,
                parent,
                created: created.to_string(),
            });
        }
        Ok(out)
    }

    /// `created`(ISO 8601 `YYYY-MM-DDTHH:MM:SS.fffffffZ`)を epoch ms へ。
    /// `Date.parse` で解釈不能なら `None`(TS では NaN → `<=` が false)。
    pub(super) fn parse_iso_ms(s: &str) -> Option<i64> {
        let (date, time) = s.split_once('T')?;
        let (y, m, d) = {
            let mut it = date.split('-');
            (
                it.next()?.parse::<i64>().ok()?,
                it.next()?.parse::<i64>().ok()?,
                it.next()?.parse::<i64>().ok()?,
            )
        };
        let time = time.strip_suffix('Z').unwrap_or(time);
        let (hms, frac) = time.split_once('.').unwrap_or((time, ""));
        let mut it = hms.split(':');
        let hh = it.next()?.parse::<i64>().ok()?;
        let mm = it.next()?.parse::<i64>().ok()?;
        let ss = it.next()?.parse::<i64>().ok()?;
        // 小数秒は先頭 3 桁(ms)を使う(.NET `o` は 7 桁)。
        let ms = if frac.is_empty() {
            0
        } else {
            let mut frac3 = frac.chars().take(3).collect::<String>();
            while frac3.len() < 3 {
                frac3.push('0');
            }
            frac3.parse::<i64>().ok()?
        };
        // days_from_civil(Howard Hinnant)で epoch days を求める。
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        Some((((days * 24 + hh) * 60 + mm) * 60 + ss) * 1_000 + ms)
    }

    pub(super) fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    /// `known`/`bornBefore`/`collect` を保持する snapshot 状態。
    pub(super) struct TreeState {
        pub(super) pid: i64,
        pub(super) known: HashMap<i64, String>,
        pub(super) born_before: i64,
    }

    impl TreeState {
        /// `collect(all)`: root validity(初回 bornBefore・2 回目以降
        /// known 照合) + ancestry BFS + known 更新。
        pub(super) fn collect(&mut self, all: &[Identity]) -> Vec<Identity> {
            let root = all.iter().find(|p| p.pid == self.pid);
            // 初回 snapshot 時点で root PID が再利用済みなら、その新 tree
            // を対象にしない。
            let root_valid = root.is_some_and(|r| {
                if self.known.contains_key(&self.pid) {
                    self.known.get(&self.pid) == Some(&r.created)
                } else {
                    parse_iso_ms(&r.created).is_some_and(|c| c <= self.born_before)
                }
            });
            let mut selected: HashSet<i64> = HashSet::new();
            if root_valid {
                selected.insert(self.pid);
            }
            for p in all {
                if self.known.get(&p.pid) == Some(&p.created) {
                    selected.insert(p.pid);
                }
            }
            let mut changed = true;
            while changed {
                changed = false;
                for p in all {
                    if !selected.contains(&p.pid)
                        && (selected.contains(&p.parent)
                            || (root.is_none() && p.parent == self.pid))
                    {
                        selected.insert(p.pid);
                        changed = true;
                    }
                }
            }
            let live: Vec<Identity> = all
                .iter()
                .filter(|p| selected.contains(&p.pid))
                .cloned()
                .collect();
            for p in &live {
                self.known.insert(p.pid, p.created.clone());
            }
            live
        }
    }

    /// 初回 snapshot + collect。`create_process_tree` 呼出し時点で起動し、
    /// `terminate` で結果を検査する(TS `first` promise parity)。
    type FirstCollect = JoinHandle<Result<(), ToolError>>;

    async fn taskkill(live: &[Identity], force: bool) -> Result<(), ToolError> {
        if live.is_empty() {
            return Ok(());
        }
        let executable = nodepath::join(&[&system_root(), "System32", "taskkill.exe"]);
        let mut args: Vec<String> = live
            .iter()
            .flat_map(|p| ["/PID".to_string(), p.pid.to_string()])
            .collect();
        args.push("/T".to_string());
        if force {
            args.push("/F".to_string());
        }
        helper(&executable, &args).await.map(|_| ())
    }

    pub(super) struct WinTree {
        state: Arc<Mutex<TreeState>>,
        first: Option<FirstCollect>,
    }

    impl WinTree {
        pub(super) fn new(pid: u32) -> Self {
            let state = Arc::new(Mutex::new(TreeState {
                pid: pid as i64,
                known: HashMap::new(),
                born_before: now_ms(),
            }));
            let first_state = Arc::clone(&state);
            // rejection は terminate で明示的に伝播する(TS `first` の
            // `.then(_, e => e)` 相当)。eager start も TS parity。
            let first: FirstCollect = tokio::spawn(async move {
                let all = snapshot().await?;
                first_state.lock().expect("tree state").collect(&all);
                Ok(())
            });
            Self {
                state,
                first: Some(first),
            }
        }

        async fn collect_snapshot(&self) -> Result<Vec<Identity>, ToolError> {
            let all = snapshot().await?;
            Ok(self.state.lock().expect("tree state").collect(&all))
        }

        /// taskkill 失敗時の race 判定(TS `kill` の catch 相当)。
        async fn kill_verified(&self, live: &[Identity], force: bool) -> Result<(), ToolError> {
            match taskkill(live, force).await {
                Ok(()) => Ok(()),
                Err(error) => {
                    // taskkill と自然終了が競合した場合だけ成功として扱う。
                    if self.collect_snapshot().await?.is_empty() {
                        Ok(())
                    } else {
                        Err(error)
                    }
                }
            }
        }

        pub(super) async fn terminate(&mut self, grace_ms: u64) -> Result<(), ToolError> {
            if let Some(first) = self.first.take() {
                // 初回 snapshot error は fail closed(helper 失敗伝播)。
                match first.await {
                    Ok(result) => result?,
                    Err(e) => return Err(super::failure(format!("{e}"))),
                }
            }
            let mut live = self.collect_snapshot().await?;
            if live.is_empty() {
                return Ok(());
            }
            // console app の非 /F termination は best effort。失敗しても
            // force escalation を行う。
            let _ = self.kill_verified(&live, false).await;
            let deadline = Instant::now() + Duration::from_millis(grace_ms);
            loop {
                live = self.collect_snapshot().await?;
                if live.is_empty() {
                    return Ok(());
                }
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                sleep(remaining.min(Duration::from_millis(50))).await;
            }
            self.kill_verified(&live, true).await?;
            if !self.collect_snapshot().await?.is_empty() {
                return Err(super::failure(
                    "managed process descendants survived force termination",
                ));
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

/// `createProcessTree(pid)` が返す handle。
pub struct ProcessTree {
    #[cfg(unix)]
    pid: u32,
    #[cfg(windows)]
    inner: win::WinTree,
}

/// `createProcessTree`。Windows では初回 CIM snapshot を即時起動する
/// (tokio runtime 上での呼出しが前提。OS containment は提供しない)。
pub fn create_process_tree(pid: u32) -> ProcessTree {
    #[cfg(unix)]
    {
        ProcessTree { pid }
    }
    #[cfg(windows)]
    {
        ProcessTree {
            inner: win::WinTree::new(pid),
        }
    }
}

impl ProcessTree {
    /// `terminate(graceMs)`。grace 内に終了しない tree は force kill。
    pub async fn terminate(&mut self, grace_ms: u64) -> Result<(), ToolError> {
        #[cfg(unix)]
        {
            posix::terminate(self.pid, grace_ms).await
        }
        #[cfg(windows)]
        {
            self.inner.terminate(grace_ms).await
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    mod win_tests {
        use super::super::win::{now_ms, parse_iso_ms, Identity, TreeState};

        fn row(pid: i64, parent: i64, created: &str) -> Identity {
            Identity {
                pid,
                parent,
                created: created.to_string(),
            }
        }

        const OLD: &str = "2020-01-01T00:00:00.0000000Z";
        const NEW: &str = "2999-01-01T00:00:00.0000000Z";

        fn state(pid: i64) -> TreeState {
            TreeState {
                pid,
                known: Default::default(),
                born_before: now_ms(),
            }
        }

        #[test]
        fn iso_parse_matches_dotnet_roundtrip() {
            assert_eq!(parse_iso_ms("1970-01-01T00:00:00.0000000Z"), Some(0));
            assert_eq!(parse_iso_ms("1970-01-01T00:00:01.2345678Z"), Some(1_234));
            // 2020-01-01T00:00:00Z = 1577836800s。
            assert_eq!(parse_iso_ms(OLD), Some(1_577_836_800_000));
            assert_eq!(parse_iso_ms("not-a-date"), None);
            assert_eq!(parse_iso_ms(""), None);
        }

        #[test]
        fn collect_selects_root_and_descendants() {
            let mut s = state(41);
            let rows = vec![
                row(41, 1, OLD),
                row(42, 41, OLD),
                row(43, 42, OLD),
                row(99, 1, OLD),
            ];
            let live = s.collect(&rows);
            assert_eq!(
                live.iter().map(|p| p.pid).collect::<Vec<_>>(),
                vec![41, 42, 43]
            );
        }

        #[test]
        fn reused_root_identity_is_not_targeted() {
            // "does not target reused root or descendant identities":
            // 初回 collect 後、pid が created 差分で再利用されると対象外。
            let mut s = state(41);
            let rows = vec![row(41, 1, OLD), row(42, 41, OLD), row(43, 42, OLD)];
            s.collect(&rows);
            let reused: Vec<Identity> = rows.iter().map(|p| row(p.pid, 999, NEW)).collect();
            let live = s.collect(&reused);
            assert!(live.is_empty());
        }

        #[test]
        fn born_before_rejects_late_root_on_first_snapshot() {
            // root created が bornBefore より後なら初回から対象外。
            let mut s = state(41);
            let rows = vec![row(41, 1, NEW), row(42, 41, OLD)];
            let live = s.collect(&rows);
            assert!(live.is_empty());
        }

        #[test]
        fn descendants_survive_missing_root_via_parent_link() {
            // root が snapshot から消えても `p.parent === pid` の子は拾う
            // (TS `!root && p.parent === pid` parity)。
            let mut s = state(41);
            let rows = vec![row(41, 1, OLD), row(42, 41, OLD)];
            s.collect(&rows);
            let live = s.collect(&[row(42, 41, OLD), row(43, 42, OLD)]);
            assert_eq!(live.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![42, 43]);
        }
    }
}
