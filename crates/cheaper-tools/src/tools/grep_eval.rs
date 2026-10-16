//! Port of `packages/tool-runtime/src/grep.ts` の `GrepEvalWorker` /
//! `GREP_EVAL_WORKER_SOURCE` — eval 専用 subprocess 隔離(T2-5)。
//!
//! TS は `worker_threads` + `terminate()` で ReDoS を隔離していたが、Rust
//! thread は安全に kill できないため、設計固定ルール通り subprocess 分離で
//! 再現する。supervisor(本モジュール [`GrepEvalWorker`])は 1 つの常駐
//! helper process に対し、length-prefix JSON frame over stdio で
//! init/batch を送り result/error を受け取る。helper 側は
//! [`helper_main`] が担い、Phase 4 で `cheaper __grep-eval` 早期分岐
//! (argv 先頭一致)に配線される。engine は `regress::Regex`(非 unicode
//! mode = JS `new RegExp` 意味)+ `i` flag。
//!
//! ## protocol
//!
//! frame = `<payload の 10 進 byte 長>\n<payload byte 列>`。reader は数字
//! だけでない行を resync 対象として読み飛ばす(test binary を helper と
//! して spawn する際、libtest の `running 1 test` / `test <name> ... `
//! banner が stdout に混ざるため。production helper の stdout は frame のみ)。
//!
//! parent → helper:
//! - `{"type":"init","pattern":"...","flags":"i"|""}` — regex compile。
//!   成功で `ready`、失敗で `error` を返す。
//! - `{"type":"batch","id":N,"display":"...","startLine":N,"lines":[...]}` —
//!   `lines` 各行に `re.test` 相当(`Regex::find().is_some()`)を適用し、
//!   batch 内 0-indexed の一致 index 配列を `result` で返す。
//!   `display`/`startLine` は TS message 形の parity のために送る(helper は
//!   使わない)。
//!
//! helper → parent:
//! - `{"type":"ready"}` / `{"type":"error","message":"..."}` /
//!   `{"type":"result","id":N,"matched":[i,...]}`。
//!
//! ## supervisor の kill/cap
//!
//! - `watchMs` = min(per-file 残・total 残・deadline 残)が呼出し側から
//!   渡され、超過で helper を `kill()` して `EVAL_TIMEOUT` で reject
//!   (TS `terminate()` 相当)。未応答 batch は常に最大 1。
//! - stdout frame の byte 上限(`reply_max_bytes`)を超える宣言長は
//!   protocol violation として helper を kill する(output byte cap)。
//! - heap cap(TS `resourceLimits.maxOldGenerationSizeMb`、
//!   `GREP_WORKER_MAX_OLD_SPACE_MB`)は v1 **未実装**。Windows 側は Job
//!   Object `ProcessMemoryLimit`、POSIX 側は `pre_exec` での `setrlimit`
//!   が候補だが実現可否は plan Risk #7 の open question。v1 は byte cap +
//!   timeout kill のみを契約とし、差分は plan/report に明記する。

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// production helper mode の argv marker。Phase 4 で
/// `cheaper __grep-eval` argv 先頭一致の早期分岐に配線される。
pub const HELPER_ARGV: &str = "__grep-eval";
/// spawn 時に立てる内部 mode 環境変数(argv marker と併用)。
pub const HELPER_ENV: &str = "CHEAPER_GREP_EVAL";

/// TS `EVAL_TIMEOUT_MESSAGE`。timeout kill・dead worker への evaluate で
/// 使う message 契約(grep 側はこの message で timeout skip を判定する)。
pub const EVAL_TIMEOUT_MESSAGE: &str = "grep pattern evaluation timed out";
/// TS `ABORTED_MESSAGE`("Operation aborted")。abort 時の throw message。
pub const ABORTED_MESSAGE: &str = "Operation aborted";

/// helper 1 reply frame の byte 上限(output byte cap)。正当な最大 reply は
/// `{"type":"result","id":N,"matched":[...]}` で matched ≤ batch 行数
/// (既定 `GREP_BATCH_MAX_LINES` = 1000、index 1 件 ≈ 7 byte)程度のため
/// 1 MiB で十分な余裕がある。`batch_max_lines` を大きく縮退注入する test
/// では `EvalOptions::reply_max_bytes` で引き上げる。
pub const EVAL_REPLY_MAX_BYTES: usize = 1024 * 1024;

/// helper が parent から受け取る 1 request frame の byte 上限。batch は
/// `lines` の JSON 化で `batch_max_bytes` + escape/構造 overhead の数倍に
/// なり得るため 8 MiB を上限とする(防御的 cap、流量制御は batch 締め側)。
pub const EVAL_REQUEST_MAX_BYTES: usize = 8 * 1024 * 1024;

/// reader thread → supervisor の reply 滞留上限。child が reply を流し
/// 続けても bounded に保ち、読み手不在時は timeout kill に繋げる。
const REPLY_QUEUE_CAP: usize = 8;
/// abort check の polling 間隔。watch 残りとの min で wait を切る。
const ABORT_POLL: Duration = Duration::from_millis(10);

/// abort signal の観測面(`signal.aborted` 相当)。`true` を返した時点で
/// `ABORTED_MESSAGE` error に写像する。
pub type AbortCheck<'a> = dyn Fn() -> bool + Send + Sync + 'a;

/// helper process の spawn 指定。production は
/// [`EvalSpawn::current_exe`](`current_exe` + `__grep-eval` argv + env
/// marker)。test は test binary 自身を env marker 付きで差し替える
/// (plan File Map「spawn 対象は option で差替可」)。
#[derive(Debug, Clone)]
pub struct EvalSpawn {
    /// spawn する executable。
    pub program: PathBuf,
    /// 内部 mode を示す argv(production は `[__grep-eval]`)。
    pub args: Vec<String>,
    /// 追加 env(production は `CHEAPER_GREP_EVAL=1`)。
    pub envs: Vec<(String, String)>,
}

impl EvalSpawn {
    /// production 既定: `current_exe` + `__grep-eval` + `CHEAPER_GREP_EVAL=1`。
    /// Phase 4 の CLI dispatch が argv 先頭一致で [`helper_main`] を起動する。
    pub fn current_exe() -> std::io::Result<Self> {
        Ok(Self {
            program: std::env::current_exe()?,
            args: vec![HELPER_ARGV.to_string()],
            envs: vec![(HELPER_ENV.to_string(), "1".to_string())],
        })
    }
}

/// supervisor option(spawn 差替・reply cap 縮退注入)。heap cap は v1 では
/// 持たない(モジュール doc 参照)。
#[derive(Debug, Clone)]
pub struct EvalOptions {
    /// helper の spawn 指定。
    pub spawn: EvalSpawn,
    /// 1 reply frame の byte 上限。
    pub reply_max_bytes: usize,
}

impl EvalOptions {
    /// production 既定値を組み立てる。
    pub fn current_exe() -> std::io::Result<Self> {
        Ok(Self {
            spawn: EvalSpawn::current_exe()?,
            reply_max_bytes: EVAL_REPLY_MAX_BYTES,
        })
    }
}

/// parent → helper message(TS `postMessage` 形の parity)。
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RequestMessage {
    /// `{"type":"init","pattern":...,"flags":...}`。
    Init { pattern: String, flags: String },
    /// `{"type":"batch","id":...,"display":...,"startLine":...,"lines":[...]}`。
    /// `display`/`startLine` は TS が送っていた形を保つ(helper 側では
    /// 未使用だが protocol 形の drift を避ける)。
    Batch {
        id: u64,
        #[allow(dead_code)]
        display: Option<String>,
        #[allow(dead_code)]
        #[serde(rename = "startLine")]
        start_line: Option<u64>,
        lines: Vec<String>,
    },
}

/// helper → parent message。
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ReplyMessage {
    /// `{"type":"ready"}` — init 成功。
    Ready,
    /// `{"type":"error","message":...}` — init 失敗など。
    Error { message: String },
    /// `{"type":"result","id":...,"matched":[...]}`。
    Result { id: u64, matched: Vec<usize> },
}

/// `evaluate` の error 分類。`Timeout` は Display が
/// `EVAL_TIMEOUT_MESSAGE` と一致する(grep 側の timeout skip 判定 contract)。
#[derive(Debug)]
pub enum EvalError {
    /// watch 超過で helper を kill した(TS `terminate()` + EVAL_TIMEOUT)。
    Timeout,
    /// abort signal 観測(呼出し側が `Operation aborted` に写像する)。
    Aborted,
    /// helper が `error` reply を返した(init compile 失敗等)。message は
    /// helper の error message をそのまま運ぶ(TS は
    /// `invalid grep pattern: ${msg}` で包む — 呼出し側で包む)。
    InvalidPattern(String),
    /// helper の stdout EOF・process 終了・reader 断線。
    ChildDead(String),
    /// stdin write・spawn 等の OS error。
    Io(std::io::Error),
    /// frame 破損(宣言長 cap 超過・JSON 不成立等)。
    Protocol(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => f.write_str(EVAL_TIMEOUT_MESSAGE),
            Self::Aborted => f.write_str(ABORTED_MESSAGE),
            Self::InvalidPattern(m) => write!(f, "invalid grep pattern: {m}"),
            Self::ChildDead(m) => write!(f, "grep eval helper died: {m}"),
            Self::Io(e) => write!(f, "grep eval helper io error: {e}"),
            Self::Protocol(m) => write!(f, "grep eval protocol violation: {m}"),
        }
    }
}

impl std::error::Error for EvalError {}

/// 1 frame = `<10 進 byte 長>\n<payload>` を書く。
fn write_frame(out: &mut impl Write, payload: &[u8]) -> std::io::Result<()> {
    writeln!(out, "{}", payload.len())?;
    out.write_all(payload)?;
    out.flush()
}

/// frame 読み取りの失敗分類。
#[derive(Debug)]
enum FrameError {
    /// read 自体の IO 失敗(helper 死亡と同じ扱い)。
    Io(String),
    /// 宣言長 cap 超過・形式不成立(protocol violation)。
    Violated(String),
}

/// 数字のみの header 行かを判定する。数字以外を含む行は resync 対象
/// (読み飛ばす — libtest banner 混入の test helper で必要)。全て数字
/// なのに `usize` に収まらない宣言長は stream 破損として `Err`。
fn parse_header(line: &[u8]) -> Result<Option<usize>, FrameError> {
    let digits = line.strip_suffix(b"\n").unwrap_or(line);
    let digits = digits.strip_suffix(b"\r").unwrap_or(digits);
    if digits.is_empty() || !digits.iter().all(|b| b.is_ascii_digit()) {
        return Ok(None);
    }
    if digits.len() > 20 {
        // usize の最大 20 桁を超える宣言は parse 不能ではなく破損とみなす
        return Err(FrameError::Violated(
            "invalid frame length declaration".to_string(),
        ));
    }
    std::str::from_utf8(digits)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .map(Some)
        .ok_or_else(|| FrameError::Violated("invalid frame length declaration".to_string()))
}

/// 1 frame を読む。EOF(クリーン・header 途中・payload 途中問わず)は
/// `Ok(None)`。宣言長が `max_len` 超過・数字だけの長大な header 等の
/// 形式不成立は `Err(protocol)`。数字でない行は読み飛ばす(長さ不問)。
fn read_frame(r: &mut impl BufRead, max_len: usize) -> Result<Option<Vec<u8>>, FrameError> {
    loop {
        let mut header = Vec::new();
        match r.read_until(b'\n', &mut header) {
            Err(e) => return Err(FrameError::Io(format!("header read failed: {e}"))),
            Ok(0) => return Ok(None), // EOF
            Ok(_) => {}
        }
        if !header.ends_with(b"\n") {
            // 改行なしで EOF に達した断片(helper 途中終了)
            return Ok(None);
        }
        let Some(len) = parse_header(&header)? else {
            continue; // banner/garbage 行は読み飛ばして resync する
        };
        if len > max_len {
            return Err(FrameError::Violated(format!(
                "frame length {len} exceeds cap {max_len}"
            )));
        }
        let mut payload = vec![0u8; len];
        match r.read_exact(&mut payload) {
            Err(_) => return Ok(None), // payload 途中 EOF = helper 終了
            Ok(()) => return Ok(Some(payload)),
        }
    }
}

/// reader thread から supervisor へ届く通知。
enum ReaderMsg {
    /// 正常 parse 済み reply。
    Reply(ReplyMessage),
    /// EOF/IO 失敗(helper 死亡)。
    Dead(String),
    /// frame 形式違反(cap 超過・JSON 不成立)。supervisor は Protocol error
    /// に写像して worker を dead にする。
    Violated(String),
}

/// child stdout の frame を読み続けて channel に流す reader thread。
fn reply_reader(stdout: ChildStdout, cap: usize, tx: SyncSender<ReaderMsg>) {
    let mut r = BufReader::new(stdout);
    loop {
        match read_frame(&mut r, cap) {
            Ok(Some(payload)) => match serde_json::from_slice::<ReplyMessage>(&payload) {
                Ok(msg) => {
                    if tx.send(ReaderMsg::Reply(msg)).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = tx.send(ReaderMsg::Violated(format!("invalid reply json: {e}")));
                    return;
                }
            },
            Ok(None) => {
                let _ = tx.send(ReaderMsg::Dead("helper closed stdout".to_string()));
                return;
            }
            Err(FrameError::Io(m)) => {
                let _ = tx.send(ReaderMsg::Dead(m));
                return;
            }
            Err(FrameError::Violated(m)) => {
                let _ = tx.send(ReaderMsg::Violated(m));
                return;
            }
        }
    }
}

/// TS `GrepEvalWorker` 相当の subprocess supervisor。spawn 時に init を
/// 送り、batch ごとに 0-indexed 一致 index 配列を受け取る。未応答 batch
/// は最大 1 で、次の `evaluate` は前の result を受けてからしか呼べない
/// (`&mut self` で構造的に保証)。timeout/abort/child 死亡で dead になり、
/// 呼出し側(grep)が次 file 用に再生成する。
pub struct GrepEvalWorker {
    opts: EvalOptions,
    pattern: String,
    flags: String,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    rx: Option<Receiver<ReaderMsg>>,
    ready: bool,
    dead: bool,
    next_id: u64,
}

impl GrepEvalWorker {
    /// spawn + init 送信(TS constructor の `postMessage(init)` 相当)。
    /// `flags` は JS flag 文字列(`""` or `"i"`)。
    pub fn spawn(opts: &EvalOptions, pattern: &str, flags: &str) -> Result<Self, EvalError> {
        let mut w = Self {
            opts: opts.clone(),
            pattern: pattern.to_string(),
            flags: flags.to_string(),
            child: None,
            stdin: None,
            rx: None,
            ready: false,
            dead: false,
            next_id: 0,
        };
        w.spawn_child()?;
        Ok(w)
    }

    fn spawn_child(&mut self) -> Result<(), EvalError> {
        let mut cmd = Command::new(&self.opts.spawn.program);
        cmd.args(&self.opts.spawn.args)
            .envs(self.opts.spawn.envs.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // helper stdout 以外の noise が親へ漏れないよう、spawn 失敗時も
        // console window を出さない(Windows GUI 付き起動に備える)。
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        // 内部 subprocess 境界の metadata-only event(design
        // §instrumentation 配置) — 親側が spawn を記録する。
        let mut child = match cmd.spawn() {
            Ok(child) => {
                tracing::debug!(
                    target: "cheaper.tool.process",
                    task = HELPER_ARGV,
                    "process spawn"
                );
                child
            }
            Err(e) => {
                tracing::debug!(
                    target: "cheaper.tool.process",
                    task = HELPER_ARGV,
                    error.code = ?e.kind(),
                    "process spawn failed"
                );
                return Err(EvalError::Io(e));
            }
        };
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        if let Some(stderr) = child.stderr.take() {
            // helper が stderr に書き込んでも詰まらないよう drain して捨てる
            // (panic message 等の diagnostic は v1 では収集しない)。
            thread::spawn(move || {
                let mut r = BufReader::new(stderr);
                let mut sink = Vec::new();
                while r.read_until(b'\n', &mut sink).unwrap_or(0) > 0 {
                    sink.clear();
                }
            });
        }
        let (tx, rx) = sync_channel(REPLY_QUEUE_CAP);
        let cap = self.opts.reply_max_bytes;
        thread::spawn(move || reply_reader(stdout, cap, tx));
        self.stdin = Some(stdin);
        self.rx = Some(rx);
        self.child = Some(child);
        self.ready = false;
        // init 送信。書き込み失敗 = spawn 直後死亡として spawn error。
        let init = serde_json::to_vec(&serde_json::json!({
            "type": "init",
            "pattern": self.pattern,
            "flags": self.flags,
        }))
        .expect("init frame serializes");
        let stdin = self.stdin.as_mut().expect("stdin");
        if let Err(e) = write_frame(stdin, &init) {
            self.kill();
            return Err(EvalError::Io(e));
        }
        Ok(())
    }

    /// TS `isDead`。
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    /// TS `evaluate`。`watch_ms` は呼出し側が残予算の最小値として計算した
    /// 値(TS は `Math.max(1, watchMs)` で clamp する)。batch 内 0-indexed
    /// の一致 index 配列を返す。
    pub fn evaluate(
        &mut self,
        display: &str,
        start_line: usize,
        lines: &[String],
        watch_ms: f64,
        aborted: Option<&AbortCheck<'_>>,
    ) -> Result<Vec<usize>, EvalError> {
        if self.dead {
            return Err(EvalError::Timeout);
        }
        let watch = if watch_ms.is_finite() && watch_ms > 1.0 {
            watch_ms
        } else {
            1.0
        };
        let deadline = Instant::now() + Duration::from_secs_f64(watch / 1000.0);

        // init reply(ready/error)の未消費分を先に処理する。TS は message
        // queue 経由で順序が保たれるので同じ意味になる。
        while !self.ready {
            match self.wait_reply(deadline, aborted)? {
                ReplyMessage::Ready => self.ready = true,
                ReplyMessage::Error { message } => {
                    self.mark_dead();
                    return Err(EvalError::InvalidPattern(message));
                }
                // 想定外の順序(先行 result 等)は読み飛ばす。
                _ => {}
            }
        }

        let id = self.next_id;
        self.next_id += 1;
        let batch = serde_json::to_vec(&serde_json::json!({
            "type": "batch",
            "id": id,
            "display": display,
            "startLine": start_line,
            "lines": lines,
        }))
        .expect("batch frame serializes");
        let stdin = self.stdin.as_mut().expect("stdin");
        if let Err(e) = write_frame(stdin, &batch) {
            self.mark_dead();
            return Err(EvalError::Io(e));
        }

        loop {
            match self.wait_reply(deadline, aborted)? {
                // TS は一致しない id の result を黙殺する(pending は timeout
                // で回収される)。同じく読み飛ばす。
                ReplyMessage::Result { id: rid, matched } if rid == id => {
                    return Ok(matched);
                }
                ReplyMessage::Error { message } => {
                    self.mark_dead();
                    return Err(EvalError::InvalidPattern(message));
                }
                _ => {}
            }
        }
    }

    /// deadline/abort poll つきで 1 reply を待つ。
    fn wait_reply(
        &mut self,
        deadline: Instant,
        aborted: Option<&AbortCheck<'_>>,
    ) -> Result<ReplyMessage, EvalError> {
        loop {
            if aborted.is_some_and(|a| a()) {
                return Err(EvalError::Aborted);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                // TS と同じく timeout kill で dead にする。
                self.kill();
                return Err(EvalError::Timeout);
            }
            let rx = match &self.rx {
                Some(rx) => rx,
                None => return Err(EvalError::ChildDead("no reply channel".to_string())),
            };
            match rx.recv_timeout(remaining.min(ABORT_POLL)) {
                Ok(ReaderMsg::Reply(m)) => return Ok(m),
                Ok(ReaderMsg::Dead(m)) => {
                    self.mark_dead();
                    return Err(EvalError::ChildDead(m));
                }
                Ok(ReaderMsg::Violated(m)) => {
                    // 壊れた helper は残さない(cap 超過 reply を吐き続ける
                    // 可能性を潰す)。TS `terminate()` 相当。
                    self.kill();
                    return Err(EvalError::Protocol(m));
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    self.mark_dead();
                    return Err(EvalError::ChildDead("reply reader gone".to_string()));
                }
            }
        }
    }

    fn mark_dead(&mut self) {
        self.dead = true;
        self.ready = false;
    }

    /// TS `kill`: pending を EVAL_TIMEOUT で reject して terminate する。
    /// 呼出し側は次 file 用に再生成する。
    pub fn kill(&mut self) {
        self.mark_dead();
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let status = child.wait().ok();
            tracing::debug!(
                target: "cheaper.tool.process",
                task = HELPER_ARGV,
                exit.code = status.and_then(|s| s.code()),
                "process exit"
            );
        }
    }
}

impl Drop for GrepEvalWorker {
    /// TS `dispose`。
    fn drop(&mut self) {
        self.kill();
    }
}

/// helper process 側の入口。`cheaper __grep-eval` の早期分岐から argv を
/// 落として呼ぶ(Phase 4 配線)。stdin から frame を読み、`init` で
/// `regress::Regex` を compile し、`batch` ごとに一致 index 配列を返す。
/// stdin EOF で正常終了(0)。stdout 書き込み失敗では 1 を返す。
pub fn helper_main() -> i32 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = BufReader::new(stdin.lock());
    let mut out = stdout.lock();
    // test binary を helper として spawn した際、libtest が `test <name> ... `
    // を改行なしで書き残すため、最初の frame が banner と同一行に連結して
    // 欠ける。先に 1 改行を出して dangling text を独立行として隔離する
    // (reader は数字でない行を読み飛ばすため無害)。
    if out.write_all(b"\n").and_then(|_| out.flush()).is_err() {
        return 1;
    }
    let mut re: Option<regress::Regex> = None;
    loop {
        let payload = match read_frame(&mut input, EVAL_REQUEST_MAX_BYTES) {
            Ok(Some(p)) => p,
            Ok(None) => return 0, // stdin EOF = parent が閉じた
            Err(_) => return 1,
        };
        let Ok(msg) = serde_json::from_slice::<RequestMessage>(&payload) else {
            continue; // TS と同じく非対象 message は無視する
        };
        match msg {
            RequestMessage::Init { pattern, flags } => {
                let reply = match regress::Regex::with_flags(&pattern, flags.as_str()) {
                    Ok(compiled) => {
                        re = Some(compiled);
                        ReplyMessage::Ready
                    }
                    Err(e) => ReplyMessage::Error {
                        message: e.to_string(),
                    },
                };
                let bytes = serde_json::to_vec(&reply).expect("reply serializes");
                if write_frame(&mut out, &bytes).is_err() {
                    return 1;
                }
            }
            // TS: `msg.type === "batch" && re !== null` のときだけ評価する
            // (init 前の batch は無視)。
            RequestMessage::Batch { id, lines, .. } => {
                let Some(re) = &re else { continue };
                let matched: Vec<usize> = lines
                    .iter()
                    .enumerate()
                    .filter(|(_, line)| re.find(line).is_some())
                    .map(|(i, _)| i)
                    .collect();
                let reply = ReplyMessage::Result { id, matched };
                let bytes = serde_json::to_vec(&reply).expect("reply serializes");
                if write_frame(&mut out, &bytes).is_err() {
                    return 1;
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! unit/integration test 用: test binary 自身を helper として spawn
    //! するための spawn spec。`helper_entry` test が env marker 観測時に
    //! `helper_main()` を実行して `exit` する。
    use super::*;

    /// test binary 内の helper entry test 名(libtest filter 文字列)。
    pub const HELPER_ENTRY_TEST: &str = "tools::grep_eval::tests::helper_entry";

    /// 設定時、helper_entry が `helper_main` 開始前に指定 ms sleep する
    /// (watch timeout の確定的検証用 — process spawn 時間に依存させない)。
    pub const HELPER_DELAY_ENV: &str = "CHEAPER_GREP_EVAL_TEST_DELAY_MS";

    /// `std::env::current_exe()`(test binary)を helper 化する spawn spec。
    pub fn test_spawn() -> EvalSpawn {
        EvalSpawn {
            program: std::env::current_exe().expect("current exe"),
            args: vec![
                HELPER_ENTRY_TEST.to_string(),
                "--exact".to_string(),
                "--test-threads=1".to_string(),
            ],
            envs: vec![(HELPER_ENV.to_string(), "1".to_string())],
        }
    }

    /// `test_spawn` + 既定 reply cap の options。
    pub fn test_eval_options() -> EvalOptions {
        EvalOptions {
            spawn: test_spawn(),
            reply_max_bytes: EVAL_REPLY_MAX_BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;
    use std::process::{Command, Stdio};

    use super::test_support::test_eval_options;
    use super::*;

    /// test binary 自身が `CHEAPER_GREP_EVAL=1` 付きで spawn されたときの
    /// helper entry。通常の `cargo test` では env が無いので即座に pass
    /// する(実 helper としての起動かどうかは env marker で判定する)。
    #[test]
    fn helper_entry() {
        if std::env::var_os(HELPER_ENV).is_none() {
            return;
        }
        if let Some(ms) = std::env::var(test_support::HELPER_DELAY_ENV)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
        {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
        std::process::exit(helper_main());
    }

    /// spawn spec が test binary + env marker を差し替えられることの
    /// 確認(差替 option が動くこと自体の証明)。
    #[test]
    fn spawn_override_substitutes_test_helper() {
        let mut w =
            GrepEvalWorker::spawn(&test_eval_options(), "hit", "").expect("spawn test helper");
        let m = w
            .evaluate(
                "a.txt",
                1,
                &["hit".to_string(), "miss".to_string()],
                5_000.0,
                None,
            )
            .expect("evaluate");
        assert_eq!(m, vec![0]);
        w.kill();
    }

    #[test]
    fn matches_zero_indexed_indices_within_batch() {
        let mut w = GrepEvalWorker::spawn(&test_eval_options(), "x\\d", "").expect("spawn");
        let lines: Vec<String> = ["a", "x1", "b", "x2", "x3"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let m = w.evaluate("f.txt", 7, &lines, 5_000.0, None).expect("eval");
        assert_eq!(m, vec![1, 3, 4]);
    }

    #[test]
    fn honors_case_insensitive_flag() {
        let mut w = GrepEvalWorker::spawn(&test_eval_options(), "Foo", "i").expect("spawn");
        let lines: Vec<String> = ["FOO", "bar"].iter().map(|s| s.to_string()).collect();
        assert_eq!(w.evaluate("f", 1, &lines, 5_000.0, None).unwrap(), vec![0]);
    }

    #[test]
    fn invalid_pattern_replies_error_not_ready() {
        let mut w = GrepEvalWorker::spawn(&test_eval_options(), "(((", "").expect("spawn");
        let err = w
            .evaluate("f", 1, &["x".to_string()], 5_000.0, None)
            .expect_err("init error expected");
        let msg = err.to_string();
        assert!(msg.starts_with("invalid grep pattern"), "unexpected: {msg}");
        assert!(w.is_dead());
    }

    #[test]
    fn dead_worker_rejects_with_timeout_message() {
        let mut w = GrepEvalWorker::spawn(&test_eval_options(), "x", "").expect("spawn");
        w.kill();
        assert!(w.is_dead());
        let err = w
            .evaluate("f", 1, &["x".to_string()], 5_000.0, None)
            .expect_err("dead worker");
        assert_eq!(err.to_string(), EVAL_TIMEOUT_MESSAGE);
    }

    #[test]
    fn watch_timeout_kills_helper() {
        // helper を 5s 遅延させて watch=1ms が必ず切れるようにする
        // (process spawn に依存した 1ms race を排除)。
        let mut spawn = test_support::test_spawn();
        spawn.envs.push((
            test_support::HELPER_DELAY_ENV.to_string(),
            "5000".to_string(),
        ));
        let opts = EvalOptions {
            spawn,
            reply_max_bytes: EVAL_REPLY_MAX_BYTES,
        };
        let mut w = GrepEvalWorker::spawn(&opts, "x", "").expect("spawn");
        let err = w
            .evaluate("f", 1, &["x".to_string()], 1.0, None)
            .expect_err("should time out");
        assert_eq!(err.to_string(), EVAL_TIMEOUT_MESSAGE);
        assert!(w.is_dead());
        // 次 file 用の再生成(TS: evaluator.isDead → new worker)
        let mut w2 = GrepEvalWorker::spawn(&test_eval_options(), "x", "").expect("respawn");
        assert_eq!(
            w2.evaluate("g", 1, &["x".to_string()], 10_000.0, None)
                .unwrap(),
            vec![0]
        );
    }

    #[test]
    fn oversized_reply_frame_is_protocol_violation() {
        // reply cap を極小に縮退し、ready frame すら超過させる →
        // reader が Dead(protocol) を送り worker が dead になる。
        let opts = EvalOptions {
            spawn: test_support::test_spawn(),
            reply_max_bytes: 4,
        };
        let mut w = GrepEvalWorker::spawn(&opts, "x", "").expect("spawn");
        let err = w
            .evaluate("f", 1, &["x".to_string()], 10_000.0, None)
            .expect_err("cap violation");
        assert!(matches!(err, EvalError::Protocol(_)), "{err}");
        assert!(w.is_dead());
    }

    #[test]
    fn abort_check_interrupts_wait() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let flag = std::sync::Arc::new(AtomicBool::new(false));
        let flag2 = flag.clone();
        let aborted = move || flag2.load(Ordering::SeqCst);
        let mut w = GrepEvalWorker::spawn(&test_eval_options(), "x", "").expect("spawn");
        // ready までは素早く済ませる
        w.evaluate("f", 1, &["x".to_string()], 10_000.0, None)
            .expect("warm eval");
        flag.store(true, Ordering::SeqCst);
        let err = w
            .evaluate("f", 2, &["x".to_string()], 60_000.0, Some(&aborted))
            .expect_err("abort expected");
        assert!(matches!(err, EvalError::Aborted));
    }

    #[test]
    fn helper_writes_only_framed_output() {
        // test binary を spawn した場合でも reader は banner を resync して
        // 最初の frame を正しく取れること(banner 混入の回帰)。
        let spawn = test_support::test_spawn();
        let mut child = Command::new(&spawn.program)
            .args(&spawn.args)
            .envs(spawn.envs.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        let stdout = child.stdout.take().unwrap();
        let mut r = BufReader::new(stdout);
        let mut stdin = child.stdin.take().unwrap();
        let init = serde_json::json!({"type":"init","pattern":"z","flags":""});
        write_frame(&mut stdin, &serde_json::to_vec(&init).unwrap()).unwrap();
        let frame = read_frame(&mut r, EVAL_REPLY_MAX_BYTES)
            .expect("frame")
            .expect("some frame");
        let reply: ReplyMessage = serde_json::from_slice(&frame).unwrap();
        assert!(matches!(reply, ReplyMessage::Ready));
        drop(stdin);
        let _ = child.kill();
        let _ = child.wait();
    }

    /// framing の単体検証: 数字でない行の読み飛ばし・cap・EOF。
    #[test]
    fn read_frame_skips_non_digit_lines() {
        let mut input: &[u8] = b"running 1 test\n5\nready".as_slice();
        let f = read_frame(&mut input, 1024).unwrap().unwrap();
        assert_eq!(f, b"ready");
        assert!(read_frame(&mut input, 1024).unwrap().is_none());
    }

    #[test]
    fn read_frame_rejects_oversized_declaration() {
        let mut input: &[u8] = b"9999\n".as_slice();
        assert!(read_frame(&mut input, 16).is_err());
    }

    // ---- regress dialect probe -------------------------------------
    // tool contract は「JavaScript regular expression」(pattern schema の
    // description)。`new RegExp(pattern, "i"|"")` で利用可能な方言が
    // regress で再現できるかを feature ごとに probe する。期待値は
    // Node 26 `new RegExp` の実測値。regress が表現できない feature が
    // 見つかった場合は contract 変更の正式起案対象であり、黙って
    // subset 化しない(plan 固定ルール)。

    fn t(pattern: &str, flags: &str, text: &str) -> Result<bool, String> {
        regress::Regex::with_flags(pattern, flags)
            .map(|re| re.find(text).is_some())
            .map_err(|e| e.to_string())
    }

    #[test]
    fn dialect_lookaround_backrefs_named_groups() {
        // lookahead / lookbehind / backreference / named group
        assert_eq!(t("(?<=x)y", "", "xy"), Ok(true));
        assert_eq!(t("(?<!x)y", "", "zy"), Ok(true));
        assert_eq!(t("(?<!x)y", "", "xy"), Ok(false));
        assert_eq!(t("(?=a)a", "", "a"), Ok(true));
        assert_eq!(t("(?!b)a", "", "a"), Ok(true));
        assert_eq!(t("(a|b)\\1", "", "aa"), Ok(true));
        assert_eq!(t("(a|b)\\1", "", "ab"), Ok(false));
        assert_eq!(t("(?<n>a)\\k<n>", "", "aa"), Ok(true));
        assert_eq!(t("(?<n>a)\\k<n>", "", "ak"), Ok(false));
    }

    #[test]
    fn dialect_annex_b_literals() {
        // u flag なしの Annex B 拡張: `\p{Lu}` は literal 'p' + "{Lu}"。
        // Node: /\p{Lu}/.test("p{Lu}") === true, .test("L") === false
        assert_eq!(t("\\p{Lu}", "", "p{Lu}"), Ok(true));
        assert_eq!(t("\\p{Lu}", "", "L"), Ok(false));
        // DIVERGENCE(known, regress): JS Annex B は `\u{...}`(u flag 無し)を
        // identity escape 'u' + `{...}` と解釈する(`\u{41}` は quantifier
        // `u{41}` として 'u'×41 に一致、`[\u{41}]` は {u,{,4,1,}} の
        // literal class)。regress は u flag 無しでも `\u{...}` を
        // code-point escape として parse する。pattern 経由で到達可能。
        assert_eq!(t("\\u{1F600}", "", "u{1F600}"), Ok(false)); // JS: true
        assert_eq!(t("\\u{41}", "", "A"), Ok(true)); // JS: false
        assert_eq!(t("\\u{41}", "", &"u".repeat(41)), Ok(false)); // JS: true
        assert_eq!(t("[\\u{41}]", "", "u"), Ok(false)); // JS: true
        assert_eq!(t("[\\u{41}]", "", "A"), Ok(true)); // JS: false
                                                       // 未定義 named backref の `\k` は literal 'k'。
        assert_eq!(t("\\k<m>", "", "k<m>"), Ok(true));
        // `\8` は literal '8'(octal ではない)。
        assert_eq!(t("\\8", "", "8"), Ok(true));
        // identity escape `\Q` → literal 'Q'。
        assert_eq!(t("\\Q", "", "Q"), Ok(true));
        // lone `{` は literal。
        assert_eq!(t("a{", "", "a{"), Ok(true));
        // legacy octal `\07` → \x07。
        assert_eq!(t("\\07", "", "\x07"), Ok(true));
        // 範囲外 backref `\2`(group 2 非存在)は octal \x02。
        assert_eq!(t("(a)\\2", "", "a\x02"), Ok(true));
        assert_eq!(t("(a)\\2", "", "a2"), Ok(false));
        // control escape `\cA` → \x01。
        assert_eq!(t("\\cA", "", "\x01"), Ok(true));
        // `[\b]` は backspace。
        assert_eq!(t("[\\b]", "", "\x08"), Ok(true));
        // `\x41` / `\u0041` / `\0`。
        assert_eq!(t("\\x41", "", "A"), Ok(true));
        assert_eq!(t("\\u0041", "", "A"), Ok(true));
        assert_eq!(t("\\0", "", "\x00"), Ok(true));
        // `\-` は class 外でも literal '-'。
        assert_eq!(t("\\-", "", "-"), Ok(true));
    }

    #[test]
    fn dialect_classes_boundaries_dot() {
        // `.` は line terminator(\n \r \u2028 \u2029)に一致しない。
        for lt in ["\n", "\r", "\u{2028}", "\u{2029}"] {
            assert_eq!(t(".", "", lt), Ok(false), "dot vs {lt:?}");
        }
        assert_eq!(t(".", "", "x"), Ok(true));
        // `\w`/`\d` は u flag 無しでは ASCII のみ。
        assert_eq!(t("\\w", "", "é"), Ok(false));
        assert_eq!(t("\\d", "", "\u{0663}"), Ok(false));
        // `\b` / `\B`。
        assert_eq!(t("\\bfoo\\b", "", " foo "), Ok(true));
        assert_eq!(t("\\B", "", "x"), Ok(false));
        // `[]` は何にも一致しない。
        assert_eq!(t("[]", "", "x"), Ok(false));
        // class 内 `\d` を含む range は分解される(`[\d-z]` = \d or - or z..z)。
        assert_eq!(t("[\\d-z]", "", "5"), Ok(true));
        assert_eq!(t("[\\d-z]", "", "-"), Ok(true));
        // anchors: u/m 無しでは入力両端のみ。
        assert_eq!(t("a$", "", "a\r"), Ok(false));
        assert_eq!(t("$^", "", ""), Ok(true));
        assert_eq!(t("$^", "", "x"), Ok(false));
    }

    #[test]
    fn dialect_modifiers_and_groups() {
        // ES2025 RegExp modifiers `(?ims:...)` — Node 26 は有効。
        assert_eq!(t("(?i:foo)", "", "FOO"), Ok(true));
        // bare `(?i)` は JS でも SyntaxError(Invalid group)。regress も
        // compile error として拒否すること(superset 化しない)。
        assert!(t("(?i)foo", "", "FOO").is_err());
        // `(?:x)` 非 capture group。
        assert_eq!(t("(?:x)", "", "x"), Ok(true));
        // alternation。
        assert_eq!(t("a|b", "", "b"), Ok(true));
        // reversed quantifier は SyntaxError。
        assert!(t("x{2,1}", "", "xxx").is_err());
    }

    #[test]
    fn dialect_icase_simple_folding() {
        // JS の i flag は simple case folding: σ→Σ は fold するが
        // ß→SS・k→KELVIN・ſ→S のような full folding はしない。
        assert_eq!(t("σ", "i", "Σ"), Ok(true));
        assert_eq!(t("ß", "i", "SS"), Ok(false));
        assert_eq!(t("k", "i", "\u{212A}"), Ok(false));
        assert_eq!(t("\u{212B}", "i", "\u{00C5}"), Ok(false)); // angstrom/Å JS も false
        assert_eq!(t("Foo", "i", "FOO"), Ok(true));
        // DIVERGENCE(known, regress): JS simple folding が折らない
        // I↔ı(dotless i U+0131)・s↔ſ(long s U+017F)を regress の icase
        // は fold する。pattern・line どちら側に書いても発動し、JS では
        // 一致しない text に一致する(superset 方向の非互換)。
        assert_eq!(t("I", "i", "\u{0131}"), Ok(true)); // JS: false
        assert_eq!(t("i", "i", "\u{0131}"), Ok(true)); // JS: false
        assert_eq!(t("\u{0131}", "i", "I"), Ok(true)); // JS: false
        assert_eq!(t("s", "i", "\u{017F}"), Ok(true)); // JS: false
        assert_eq!(t("\u{017F}", "i", "s"), Ok(true)); // JS: false
    }

    #[test]
    fn dialect_empty_pattern() {
        // new RegExp("").source === "(?:)" — 全てに一致。
        // regress にも同じ意味が必要。空文字が compile error になるなら
        // grep 側で "(?:)" に写す。
        let ok = t("", "", "anything").unwrap_or(false);
        if !ok {
            assert_eq!(t("(?:)", "", "anything"), Ok(true));
        }
    }
}
