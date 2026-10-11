//! `executeAuth`(`packages/cli/src/auth.ts`)の port(T5)。
//!
//! `auth status`/`login`/`logout` の 3 action + terminal interaction
//! (`createTerminalInteraction` 相当)を実装する。stdout contract は他の
//! utility command と同じく envelope 1 枚(`utility_success_envelope`)で、
//! secret 値は stdout/stderr/envelope いずれにも出さない(永続 store のみ)。
//!
//! runtime は `Runtime::create`(TS `createModelRuntime` 相当)で組み立て、
//! `bundledProviderPlugins()` 相当として devin plugin を登録する。
//! test は [`execute_with`] で runtime/terminal/signal/`open_url` を
//! 注入できる(TS `hooks.createModelRuntime`/`openUrl`/`signal` 相当)。
//!
//! 既知の divergence:
//! - `Runtime::create` は `Arc<dyn ProviderEnv>`(`'static`)を要求するため
//!   `io.env`(`&dyn Fn`)をそのまま runtime env へ渡せない。production の
//!   `io.env` は `std::env::var` wrapper なので `ProcessEnv` で等価。
//!   `userConfigDir` 解決(`CHEAPER_CONFIG_DIR` 等)のみ `io.env` を使う
//!   — TS も同じ env で `resolveUserConfigDir` しているため一致。
//!   env 注入が必要な test は `execute_with` で Runtime を組み立てる。
//! - `ask`(readline)の abort は TS では `AbortError`(非 CheaperError)が
//!   propagate して `INTERNAL_ERROR`(70)envelope になる。ここでは同じ
//!   code/exit code に畳むため `INTERNAL_ERROR` を返す。`askSecret` の
//!   abort/Ctrl-C は TS と同じく `INVALID_REQUEST "login cancelled"`。

use std::collections::BTreeMap;
use std::io::{self, BufRead};
use std::process::Command;
use std::sync::Arc;
use std::thread::JoinHandle;

use cheaper_core::config::dirs::{resolve_user_config_dir, UserConfigDirOptions};
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::js::js_trim;
use cheaper_provider::devin::DevinProviderPlugin;
use cheaper_runtime::auth::{AuthEvent, AuthInteraction, AuthPrompt, AuthType, ProcessEnv};
use cheaper_runtime::{CheaperProviderPlugin, CredentialStatus, Runtime};
use cheaper_tools::operation::{AbortController, AbortReason, AbortSignal};
use serde_json::{Map, Value};
use tracing::Instrument;

use crate::args::{AuthAction, AuthCliArgs, AuthTypeArg};
use crate::cli::{emit_catalog_warnings, BoxError, CliHooks, CliIo, CliWrite, CommandOut};
use crate::envelope::utility_success_envelope;

fn invalid_request(message: String, details: Option<Map<String, Value>>) -> CheaperError {
    let e = CheaperError::new(ErrorCode::InvalidRequest, message);
    match details {
        Some(d) => e.with_details(d),
        None => e,
    }
}

/// `askSecret` の abort/Ctrl-C reject 値(TS `invalidRequest("login cancelled")`)。
fn login_cancelled() -> CheaperError {
    invalid_request("login cancelled".to_string(), None)
}

/// `ask`(readline)の abort — TS の `AbortError` propagate 相当
/// (非 CheaperError は `INTERNAL_ERROR`(70)へ畳まれる)。
fn aborted_error() -> CheaperError {
    CheaperError::new(ErrorCode::InternalError, "This operation was aborted")
}

/// `openUrlBestEffort` — auth_url を browser で開く best-effort。
/// 起動失敗は無視する(URL は stderr に表示済みで flow は継続)。
///
/// dep-free 実装: `open`/`webbrowser` crate は未導入のため OS 別 opener を
/// `std::process::Command` で detached spawn する(TS の `spawn + unref`
/// と同等)。Windows は `cmd /c start` が query の `&` を区切るため
/// `rundll32 url.dll,FileProtocolHandler` を使う(TS と同じ選択)。
pub fn open_url_best_effort(url: &str) {
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("rundll32", &["url.dll,FileProtocolHandler", url])
    } else if cfg!(target_os = "macos") {
        ("open", &[url])
    } else {
        ("xdg-open", &[url])
    };
    // `stdio: "ignore"` + `unref()` 相当 — wait しない。
    let _ = Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// auth prompt の terminal 境界。production は `ProcessTerminal`(実
/// stdin/TTY + `io.stderr`)、test は scripted 実装を注入する。
/// secret の echo-off 入力と select/text の 1 行入力を分離し、
/// `rpassword` 依存を production 実装に閉じ込める。
pub trait AuthTerminal {
    /// `io.stdin.isTTY` 相当。
    fn is_tty(&self) -> bool;
    /// `io.stderr.write(s)` 相当。
    fn write(&mut self, s: &str);
    /// readline `question` 相当の 1 行読取り(echo あり)。
    /// 生値を返し、trim は呼び出し側(TS `ask` が `.trim()` する)。
    fn read_line(&mut self) -> io::Result<String>;
    /// echo-off の secret 入力(TS raw-mode stdin read 相当)。
    /// Ctrl-C は `ErrorKind::Interrupted` を返す約束とする
    /// (TS `askSecret` の `\x03` → abort 分岐)。
    fn read_secret(&mut self) -> io::Result<String>;
}

/// production terminal — TTY 判定と stderr は `CliIo` 境界経由、
/// stdin の実読取りは process stdin(TTY 到達時のみ呼ばれる)、
/// secret は `rpassword` の TTY raw-mode 入力。
struct ProcessTerminal<'a> {
    tty: bool,
    stderr: &'a mut dyn CliWrite,
}

impl AuthTerminal for ProcessTerminal<'_> {
    fn is_tty(&self) -> bool {
        self.tty
    }
    fn write(&mut self, s: &str) {
        self.stderr.write_str(s);
    }
    fn read_line(&mut self) -> io::Result<String> {
        let mut buf = String::new();
        io::stdin().lock().read_line(&mut buf)?;
        Ok(buf)
    }
    fn read_secret(&mut self) -> io::Result<String> {
        // prompt/改行/feedback は呼び出し側が `io.stderr` 経由で出すので
        // rpassword 側の出力(/dev/tty|CONOUT$)は捨てる — CliIo 境界の
        // stderr byte 列を TS と一致させるため。
        let config = rpassword::ConfigBuilder::new().output_discard().build();
        rpassword::read_password_with_config(config)
    }
}

/// `createTerminalInteraction` の port — `AuthPrompt`/`AuthEvent` を
/// `AuthTerminal` 経由の terminal I/O に bridge する。
/// `select` prompt は `method`(`--method`)が option id に一致すれば
/// 非対話で即答する。stdin 非 TTY で対話 prompt が来たら
/// `INVALID_REQUEST` で失敗する。
pub struct TerminalInteraction<'a> {
    terminal: &'a mut dyn AuthTerminal,
    method: Option<&'a str>,
    signal: Option<AbortSignal>,
    open_url: &'a dyn Fn(&str),
}

impl<'a> TerminalInteraction<'a> {
    pub fn new(
        terminal: &'a mut dyn AuthTerminal,
        method: Option<&'a str>,
        signal: Option<AbortSignal>,
        open_url: &'a dyn Fn(&str),
    ) -> Self {
        Self {
            terminal,
            method,
            signal,
            open_url,
        }
    }

    /// `ask` — prompt を stderr へ出して 1 行読み、`.trim()` する。
    /// abort は INTERNAL_ERROR に畳む(TS AbortError → errorEnvelope
    /// の非 CheaperError 分岐と同じ code/exit code)。
    ///
    /// residual gap: `read_line` は blocking なため read 中の abort
    /// をその場で中断できない(TS は readline AbortSignal で即 reject
    /// する)。post-read check で入力完了直後にだけ検知し、per-prompt
    /// `AuthPrompt.signal` は seam に無い(TS prompt option)。
    fn ask(&mut self, message: &str) -> Result<String, CheaperError> {
        if self.is_cancelled() {
            return Err(aborted_error());
        }
        self.terminal.write(message);
        let read = self.terminal.read_line();
        // read 中に到達した abort を post-read で拾う(中断自体は
        // できない — 上記 residual gap)。
        if self.is_cancelled() {
            return Err(aborted_error());
        }
        match read {
            Ok(line) => Ok(js_trim(&line).to_string()),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => Err(aborted_error()),
            Err(e) => Err(CheaperError::new(ErrorCode::InternalError, e.to_string())),
        }
    }

    /// `askSecret` — echo-off 入力 + 後処理で stderr へ `\n`。
    /// abort/Ctrl-C は `INVALID_REQUEST "login cancelled"`。
    /// `read_secret` も blocking のため read 中の abort は post-read
    /// check でのみ検知する(`ask` と同じ residual gap)。
    fn ask_secret(&mut self, message: &str) -> Result<String, CheaperError> {
        if self.is_cancelled() {
            return Err(login_cancelled());
        }
        self.terminal.write(message);
        let result = self.terminal.read_secret();
        // TS `askSecret` の cleanup(`io.stderr.write("\n")`)相当。
        self.terminal.write("\n");
        if self.is_cancelled() {
            return Err(login_cancelled());
        }
        match result {
            Ok(secret) => Ok(secret),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => Err(login_cancelled()),
            Err(e) => Err(CheaperError::new(ErrorCode::InternalError, e.to_string())),
        }
    }
}

/// `AuthPrompt` variant → TS `p.type` 文字列。
fn prompt_type_str(p: &AuthPrompt<'_>) -> &'static str {
    match p {
        AuthPrompt::Secret { .. } => "secret",
        AuthPrompt::Text { .. } => "text",
        AuthPrompt::ManualCode { .. } => "manual_code",
        AuthPrompt::Select { .. } => "select",
    }
}

/// `Number.parseInt(answer, 10)` 相当 — 先頭 whitespace → 符号 →
/// digits の prefix を parse(digits が無ければ NaN → `None`)。
fn parse_int_prefix(s: &str) -> Option<i64> {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut i = 0;
    let negative = match bytes.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    let start = i;
    while matches!(bytes.get(i), Some(b) if b.is_ascii_digit()) {
        i += 1;
    }
    if i == start {
        return None;
    }
    // parseInt は桁溢れでも integer を返すが、options 数を大幅に超える
    // index はどちらにしても invalid selection — i64 parse 失敗は None で
    // 同じ分岐に落とす。
    let v: i64 = s[start..i].parse().ok()?;
    Some(if negative { -v } else { v })
}

impl AuthInteraction for TerminalInteraction<'_> {
    fn prompt(&mut self, p: &AuthPrompt<'_>) -> Result<String, CheaperError> {
        if let AuthPrompt::Select { message, options } = p {
            let ids: Vec<&str> = options.iter().map(|o| o.id).collect();
            let options_value =
                || Value::Array(ids.iter().map(|id| Value::from(*id)).collect::<Vec<_>>());
            // `--method` が option id に一致すれば非対話で即答。
            if let Some(method) = self.method {
                if let Some(hit) = options.iter().find(|o| o.id == method) {
                    return Ok(hit.id.to_string());
                }
                self.terminal.write(&format!(
                    "note: --method {method} did not match options ({})\n",
                    ids.join(", ")
                ));
            }
            if !self.terminal.is_tty() {
                let mut details = Map::new();
                details.insert("options".to_string(), options_value());
                return Err(invalid_request(
                    format!(
                        "interactive selection required; re-run with --method <{}>",
                        ids.join("|")
                    ),
                    Some(details),
                ));
            }
            self.terminal.write(&format!("{message}\n"));
            for (i, o) in options.iter().enumerate() {
                let description = o.description.map(|d| format!(" — {d}")).unwrap_or_default();
                self.terminal
                    .write(&format!("  {}) {}{}\n", i + 1, o.label, description));
            }
            let answer = self.ask(&format!("select [1-{}]: ", options.len()))?;
            let selected = parse_int_prefix(&answer)
                .and_then(|n| usize::try_from(n - 1).ok())
                .and_then(|idx| options.get(idx));
            return match selected {
                Some(opt) => Ok(opt.id.to_string()),
                None => {
                    let mut details = Map::new();
                    details.insert("options".to_string(), options_value());
                    Err(invalid_request(
                        format!("invalid selection: {answer}"),
                        Some(details),
                    ))
                }
            };
        }
        if !self.terminal.is_tty() {
            let mut details = Map::new();
            details.insert("promptType".to_string(), Value::from(prompt_type_str(p)));
            return Err(invalid_request(
                format!(
                    "auth {} prompt requires an interactive terminal",
                    prompt_type_str(p)
                ),
                Some(details),
            ));
        }
        match p {
            AuthPrompt::Secret { message, .. } => self.ask_secret(&format!("{message}: ")),
            AuthPrompt::Text { message, .. } | AuthPrompt::ManualCode { message, .. } => {
                self.ask(&format!("{message}: "))
            }
            AuthPrompt::Select { .. } => unreachable!("select handled above"),
        }
    }

    /// `interaction.notify(message)` — 平文 message はそのまま stderr へ。
    fn notify(&mut self, message: &str) -> Result<(), CheaperError> {
        self.terminal.write(&format!("{message}\n"));
        Ok(())
    }

    /// `interaction.notify(event)` — TS `notify` の switch を byte 一致で
    /// 再現するため default の `render_event` 畳み込みは使わない。
    fn event(&mut self, event: &AuthEvent) -> Result<(), CheaperError> {
        match event {
            AuthEvent::AuthUrl { url, instructions } => {
                self.terminal
                    .write(&format!("\nOpen this URL to sign in:\n  {url}\n\n"));
                if let Some(instructions) = instructions {
                    self.terminal.write(&format!("{instructions}\n"));
                }
                (self.open_url)(url);
            }
            AuthEvent::DeviceCode {
                user_code,
                verification_uri,
                expires_in_seconds,
                ..
            } => {
                self.terminal.write(&format!(
                    "\nGo to {verification_uri} and enter code: {user_code}\n"
                ));
                if let Some(seconds) = expires_in_seconds {
                    self.terminal.write(&format!("(expires in {seconds}s)\n"));
                }
            }
            AuthEvent::Info { message, links } => {
                self.terminal.write(&format!("{message}\n"));
                for link in links {
                    self.terminal.write(&format!(
                        "  {}: {}\n",
                        link.label.as_deref().unwrap_or("link"),
                        link.url
                    ));
                }
            }
            AuthEvent::Progress { message } => {
                self.terminal.write(&format!("{message}\n"));
            }
        }
        Ok(())
    }

    /// `interaction.signal.aborted` 相当。
    fn is_cancelled(&self) -> bool {
        self.signal
            .as_ref()
            .map(|s| s.is_aborted())
            .unwrap_or(false)
    }
}

/// `process.on("SIGINT"/"SIGTERM") → ac.abort()` の relay。
/// `AuthService::login` は sync(device-code poll が thread sleep で block)
/// なので、専用 thread の current-thread runtime で OS signal を待ち、
/// 受信したら controller を abort する(`AuthInteraction::is_cancelled`
/// が拾う)。login 終了時に `done` で thread を解放する。
struct SignalRelay {
    done: AbortController,
    handle: Option<JoinHandle<()>>,
}

impl SignalRelay {
    fn install(controller: AbortController) -> Option<SignalRelay> {
        let done = AbortController::new();
        let done_signal = done.signal();
        let handle = std::thread::Builder::new()
            .name("auth-signal-relay".to_string())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                rt.block_on(wait_for_signal(controller, done_signal));
            })
            .ok()?;
        Some(SignalRelay {
            done,
            handle: Some(handle),
        })
    }
}

impl Drop for SignalRelay {
    fn drop(&mut self) {
        // login 完了 → relay thread を閉じる(TS `process.off` 相当)。
        self.done.abort(AbortReason::aborted());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

async fn wait_for_signal(controller: AbortController, done: AbortSignal) {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);
    #[cfg(unix)]
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
    #[cfg(unix)]
    {
        tokio::select! {
            _ = &mut ctrl_c => controller.abort(AbortReason::aborted()),
            _ = async {
                match &mut sigterm {
                    Some(s) => { s.recv().await; }
                    None => std::future::pending::<()>().await,
                }
            } => controller.abort(AbortReason::aborted()),
            _ = done.cancelled() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::select! {
            _ = &mut ctrl_c => controller.abort(AbortReason::aborted()),
            _ = done.cancelled() => {}
        }
    }
}

/// `createModelRuntime({env: io.env})` 相当 — `<userConfigDir>` 解決 +
/// bundled plugin(`bundledProviderPlugins()` = devin)登録 + builtin
/// oauth flow seed(`Runtime::create` 内部)。
fn create_runtime(io: &CliIo<'_>) -> Result<Runtime, CheaperError> {
    let user_config_dir = resolve_user_config_dir(UserConfigDirOptions {
        env: Some(io.env),
        homedir: None,
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![Arc::new(DevinProviderPlugin::new())];
    Runtime::create(
        user_config_dir,
        Arc::new(ProcessEnv),
        &plugins,
        BTreeMap::new(),
    )
}

/// login/logout の provider 引数(args 解析で必須検証済み。
/// `execute_with` 経由の direct call でも同じ error を返す)。
fn require_provider_arg<'a>(args: &'a AuthCliArgs, action: &str) -> Result<&'a str, CheaperError> {
    args.provider
        .as_deref()
        .ok_or_else(|| invalid_request(format!("auth {action} requires <provider>"), None))
}

/// `executeAuth` 本体 — `CliHooks.now`/`signal` と process TTY を使う
/// production 経路。
/// `cheaper.cli.command` span(design §instrumentation 配置)。
pub async fn execute(
    args: &AuthCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    execute_inner(args, io, hooks, started_at)
        .instrument(tracing::info_span!("cheaper.cli.command", command = "auth",))
        .await
}

async fn execute_inner(
    args: &AuthCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    let runtime = create_runtime(io).map_err(BoxError::from)?;
    let mut terminal = ProcessTerminal {
        tty: io.stdin.is_tty(),
        stderr: &mut *io.stderr,
    };
    // `Date.now` は整数 ms — `cli::system_now` の truncating clock
    // を共有する(端数 ms を残すと envelope parity が崩れる)。
    let now = || crate::cli::now(hooks);
    if args.action == AuthAction::Login {
        // TS `executeAuth`: login 中だけ SIGINT/SIGTERM を AbortSignal に
        // 中継する(`process.on` → `ac.abort()`)。
        let controller = AbortController::new();
        let _relay = SignalRelay::install(controller.clone());
        let signal = match &hooks.signal {
            Some(external) => AbortSignal::any(&[controller.signal(), external.clone()]),
            None => controller.signal(),
        };
        return execute_with(
            args,
            &runtime,
            &mut terminal,
            &signal,
            &open_url_best_effort,
            &now,
            started_at,
        )
        .await;
    }
    execute_with(
        args,
        &runtime,
        &mut terminal,
        &AbortSignal::detached(),
        &open_url_best_effort,
        &now,
        started_at,
    )
    .await
}

/// `execute` の注入 seam — runtime・terminal・signal・`open_url`・`now`
/// を外部から渡す(test の scripted terminal / seeded Runtime / opener
/// capture に対応する、TS `hooks.createModelRuntime`/`openUrl`/`signal`
/// 相当の差し替え点)。
pub async fn execute_with(
    args: &AuthCliArgs,
    runtime: &Runtime,
    terminal: &mut dyn AuthTerminal,
    signal: &AbortSignal,
    open_url: &dyn Fn(&str),
    now: &dyn Fn() -> f64,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    let catalog = runtime.catalog();
    let auth = runtime.auth();
    // spec Req 4 — compose/snapshot error を action dispatch 前に emit
    // する。action match 内は全分岐 `?` 伝播するため match 後の emit は
    // error 経路で走らず、dropped plugin provider への
    // `auth status <id>`/`auth login <id>` は `require_provider` の
    // `unknown provider` で記録済み compose error が見えないまま終了
    // する — ここで emit しておくと `warning:` 行 + envelope error が
    // 併存する。sink は `AuthTerminal::write`(stderr 境界)。
    let errs = catalog.errors();
    emit_catalog_warnings(&errs, &mut |s: &str| terminal.write(s), args.quiet);
    // login 中に `refresh_plugin_catalog` が記録する error 用の emit
    // 済み件数 cursor(post-login 差分 emit で `errs[emitted..]` のみ
    // 出す)。
    let emitted = errs.len();
    let result = match args.action {
        AuthAction::Status => {
            // TS: `args.provider` があれば単一 describe、無ければ全件 list。
            // credentials は filter に関わらず全 credential(file 宣言順)。
            let (providers, credentials) = match &args.provider {
                Some(provider_id) => {
                    let providers = vec![auth
                        .describe_provider(catalog, provider_id)
                        .map_err(BoxError::from)?];
                    let credentials: Vec<CredentialStatus> = auth
                        .credentials()
                        .all()
                        .map_err(BoxError::from)?
                        .into_iter()
                        .map(|(provider_id, credential)| CredentialStatus {
                            provider_id,
                            credential_type: credential.auth_type().as_str(),
                        })
                        .collect();
                    (providers, credentials)
                }
                None => {
                    let status = auth.status(catalog).map_err(BoxError::from)?;
                    (status.providers, status.credentials)
                }
            };
            // `result: { credentials, providers }` — TS literal 順。
            let mut result = Map::new();
            result.insert(
                "credentials".to_string(),
                serde_json::to_value(&credentials).unwrap_or(Value::Null),
            );
            result.insert(
                "providers".to_string(),
                serde_json::to_value(&providers).unwrap_or(Value::Null),
            );
            result
        }
        AuthAction::Logout => {
            let provider = require_provider_arg(args, "logout").map_err(BoxError::from)?;
            // logout hook も「外部 session は残る」等の notify を発火
            // しうるため login と同じ interaction seam を渡す（prompt
            // は logout 経路では発生しない）。
            let mut interaction =
                TerminalInteraction::new(terminal, None, Some(signal.clone()), open_url);
            let out = auth
                .logout(catalog, provider, &mut interaction)
                .map_err(BoxError::from)?;
            // `{provider, loggedOut: true}`。
            let mut result = Map::new();
            result.insert("provider".to_string(), Value::from(out.provider));
            result.insert("loggedOut".to_string(), Value::from(out.logged_out));
            result
        }
        AuthAction::Login => {
            let provider = require_provider_arg(args, "login").map_err(BoxError::from)?;
            let method = match args.auth_type {
                AuthTypeArg::Oauth => AuthType::Oauth,
                AuthTypeArg::ApiKey => AuthType::ApiKey,
            };
            let mut interaction = TerminalInteraction::new(
                terminal,
                args.method.as_deref(),
                Some(signal.clone()),
                open_url,
            );
            auth.login(catalog, provider, method, &mut interaction)
                .map_err(BoxError::from)?;
            // spec Req 4 — login `Ok` 時点で refresh/persist error は
            // 記録済み(`refresh_plugin_catalog` は `Ok` 返却前に走る)
            // ので、early emit 済み分を cursor で除いた差分を emit する。
            // login が Err なら refresh は走っていないため emit 不要。
            // 同一 merge 衝突 message が compose/refresh で 2 回記録
            // される場合 warning 行も 2 行出得る(accepted stderr
            // noise — dedup しない)。
            let errs = catalog.errors();
            emit_catalog_warnings(
                &errs[emitted.min(errs.len())..],
                &mut |s: &str| terminal.write(s),
                args.quiet,
            );
            // TS `loginProvider` は login 後に `describeProviderAuth` を再
            // 評価し、`authenticated`/`subscription` をその結果から取る。
            let info = auth
                .describe_provider(catalog, provider)
                .map_err(BoxError::from)?;
            // `{provider, authenticated, method, subscription}`。
            let mut result = Map::new();
            result.insert("provider".to_string(), Value::from(info.provider_id));
            result.insert("authenticated".to_string(), Value::from(info.configured));
            result.insert("method".to_string(), Value::from(method.as_str()));
            result.insert("subscription".to_string(), Value::from(info.subscription));
            result
        }
    };
    Ok(CommandOut {
        // TS `now() - startedAt` は操作完了後に採る。
        envelope: utility_success_envelope(result, now() - started_at),
        exit_code: 0,
        model_line: None,
    })
}
