use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use cheaper_core::errors::CheaperError;
use cheaper_runtime::auth::{AuthInteraction, AuthPrompt};

use cheaper_provider::devin::cli::*;
use cheaper_provider::devin::credentials::*;

/// notify を記録する `AuthInteraction` stub — Windows contract の
/// 告知・非告知を検証する。
#[derive(Default)]
struct RecordingInteraction {
    notifications: Vec<String>,
}
impl AuthInteraction for RecordingInteraction {
    fn prompt(&mut self, _prompt: &AuthPrompt<'_>) -> Result<String, CheaperError> {
        Ok(String::new())
    }
    fn notify(&mut self, message: &str) -> Result<(), CheaperError> {
        self.notifications.push(message.to_string());
        Ok(())
    }
}

fn env(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

struct Recorded {
    bin: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    inherit_stdio: bool,
}

struct FakeRunner {
    /// args 列 → result。call 順に消費せず全 call に同じ result を返す。
    result: Result<DevinCliResult, std::io::ErrorKind>,
    calls: Mutex<Vec<Recorded>>,
}

impl DevinCliRunner for FakeRunner {
    fn run(
        &self,
        bin: &str,
        args: &[String],
        opts: &CliRunOptions<'_>,
    ) -> Result<DevinCliResult, std::io::Error> {
        self.calls.lock().unwrap().push(Recorded {
            bin: bin.to_string(),
            args: args.to_vec(),
            env: opts.env.clone(),
            inherit_stdio: opts.inherit_stdio,
        });
        match &self.result {
            Ok(r) => Ok(r.clone()),
            Err(kind) => Err(std::io::Error::new(*kind, "fake")),
        }
    }
}

fn cli_with(
    result: Result<DevinCliResult, std::io::ErrorKind>,
    env: BTreeMap<String, String>,
    data_dir: &Path,
) -> (DevinCli, Arc<FakeRunner>) {
    let runner = Arc::new(FakeRunner {
        result,
        calls: Mutex::new(Vec::new()),
    });
    (DevinCli::new(env, data_dir, Some(runner.clone())), runner)
}

fn ok(stdout: &str) -> Result<DevinCliResult, std::io::ErrorKind> {
    Ok(DevinCliResult {
        stdout: stdout.to_string(),
        stderr: String::new(),
        code: 0,
    })
}

#[test]
fn child_env_isolated_and_base_untouched() {
    let base = env(&[("KEEP", "1")]);
    let child = devin_child_env(&base, Path::new("/data"));
    assert_eq!(child["XDG_DATA_HOME"], "/data");
    #[cfg(windows)]
    assert_eq!(child["LOCALAPPDATA"], "/data");
    assert_eq!(child["KEEP"], "1");
    assert!(!base.contains_key("XDG_DATA_HOME"));
}

#[test]
fn devin_cli_env_override_picks_binary() {
    let (cli, runner) = cli_with(
        ok("1.0"),
        env(&[("DEVIN_CLI", "/opt/devin")]),
        Path::new("d"),
    );
    assert_eq!(cli.bin, "/opt/devin");
    assert_eq!(cli.version().as_deref(), Some("1.0"));
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls[0].args, vec!["version".to_string()]);
    assert_eq!(calls[0].env["XDG_DATA_HOME"], "d");
}

#[test]
fn version_returns_none_on_failure() {
    let (cli, _) = cli_with(
        Err(std::io::ErrorKind::NotFound),
        BTreeMap::new(),
        Path::new("d"),
    );
    assert!(cli.version().is_none());
    let (cli, _) = cli_with(
        Ok(DevinCliResult {
            stdout: String::new(),
            stderr: "x".to_string(),
            code: 1,
        }),
        BTreeMap::new(),
        Path::new("d"),
    );
    assert!(cli.version().is_none());
}

#[test]
fn not_found_maps_to_cli_not_found() {
    let (cli, _) = cli_with(
        Err(std::io::ErrorKind::NotFound),
        BTreeMap::new(),
        Path::new("d"),
    );
    let err = cli.run(&["auth", "status"], None, false).unwrap_err();
    assert!(matches!(err, CliError::NotFound(_)));
    assert!(err.to_string().contains("Devin CLI not found"));
}

#[test]
fn exit_127_maps_to_cli_not_found() {
    // TS `isNotFound` の `code === 127` 相当 — wrapper 等が
    // "command not found" の exit 127 で完了した場合も not-found
    // error へ写像する。
    let (cli, _) = cli_with(
        Ok(DevinCliResult {
            stdout: String::new(),
            stderr: "devin: command not found".to_string(),
            code: 127,
        }),
        BTreeMap::new(),
        Path::new("d"),
    );
    let err = cli.run(&["auth", "status"], None, false).unwrap_err();
    assert!(matches!(err, CliError::NotFound(_)));
    assert!(err.to_string().contains("Devin CLI not found"));
}

#[test]
fn credentials_toml_parsing() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let cred_path = devin_credentials_path(&data_dir);
    assert_eq!(cred_path, data_dir.join("devin").join("credentials.toml"));
    std::fs::create_dir_all(cred_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cred_path,
        "windsurf_api_key = \"wk-123\"\napi_server_url = \"https://alt.example/\"\nother = 42\n",
    )
    .unwrap();
    let creds = read_devin_credentials(&data_dir).unwrap();
    assert_eq!(creds.api_key, "wk-123");
    assert_eq!(creds.api_server_url, "https://alt.example");
    // 欠損時 default。
    std::fs::write(&cred_path, "api_key = \"k2\"\n").unwrap();
    let creds = read_devin_credentials(&data_dir).unwrap();
    assert_eq!(creds.api_key, "k2");
    assert_eq!(creds.api_server_url, "https://server.codeium.com");
    assert_eq!(creds.webapp_host, "app.devin.ai");
}

#[test]
fn auth_status_cli_authoritative_and_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    // CLI が "logged in" を返せば credentials.toml 無しでも true。
    let (cli, _) = cli_with(ok("You are logged in as x"), BTreeMap::new(), &data_dir);
    assert!(devin_auth_status(&cli, &data_dir).logged_in);
    // CLI が logged out → toml があっても false（CLI が authoritative）。
    let cred_path = devin_credentials_path(&data_dir);
    std::fs::create_dir_all(cred_path.parent().unwrap()).unwrap();
    std::fs::write(&cred_path, "api_key = \"k\"\n").unwrap();
    let (cli, _) = cli_with(
        Ok(DevinCliResult {
            stdout: "not logged in".to_string(),
            stderr: String::new(),
            code: 1,
        }),
        BTreeMap::new(),
        &data_dir,
    );
    assert!(!devin_auth_status(&cli, &data_dir).logged_in);
    // CLI 未到達 + toml 有り → logged in fallback。
    let (cli, _) = cli_with(
        Err(std::io::ErrorKind::NotFound),
        BTreeMap::new(),
        &data_dir,
    );
    assert!(devin_auth_status(&cli, &data_dir).logged_in);
    // CLI 未到達 + toml 無し → logged out。
    std::fs::remove_file(&cred_path).unwrap();
    let (cli, _) = cli_with(
        Err(std::io::ErrorKind::NotFound),
        BTreeMap::new(),
        &data_dir,
    );
    assert!(!devin_auth_status(&cli, &data_dir).logged_in);
}

#[test]
fn login_reuses_fresh_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let cred_path = devin_credentials_path(&data_dir);
    std::fs::create_dir_all(cred_path.parent().unwrap()).unwrap();
    std::fs::write(&cred_path, "api_key = \"fresh\"\n").unwrap();
    let (cli, runner) = cli_with(ok("logged in"), BTreeMap::new(), &data_dir);
    let mut interaction = RecordingInteraction::default();
    let creds = login_with_devin_cli(&cli, &data_dir, false, None, &mut interaction).unwrap();
    assert_eq!(creds.api_key, "fresh");
    // `auth status` のみで `auth login` は走らない。reuse 経路では
    // login 起動 notify も発火しない。
    assert!(interaction.notifications.is_empty());
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args, vec!["auth", "status"]);
}

#[test]
fn login_runs_cli_when_stale_then_reads_toml() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    // stale credential（CLI が logged out と言う）→ `auth login` 起動。
    let cred_path = devin_credentials_path(&data_dir);
    std::fs::create_dir_all(cred_path.parent().unwrap()).unwrap();
    std::fs::write(&cred_path, "api_key = \"stale\"\n").unwrap();
    // status → not logged in、login → その間に toml を更新する runner。
    struct LoginWriter {
        path: std::path::PathBuf,
        calls: Mutex<Vec<Vec<String>>>,
    }
    impl DevinCliRunner for LoginWriter {
        fn run(
            &self,
            _bin: &str,
            args: &[String],
            _opts: &CliRunOptions<'_>,
        ) -> Result<DevinCliResult, std::io::Error> {
            self.calls.lock().unwrap().push(args.to_vec());
            if args == ["auth", "login"] {
                std::fs::write(&self.path, "api_key = \"new\"\n").unwrap();
                return Ok(DevinCliResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    code: 0,
                });
            }
            Ok(DevinCliResult {
                stdout: "not logged in".to_string(),
                stderr: String::new(),
                code: 1,
            })
        }
    }
    let runner = Arc::new(LoginWriter {
        path: cred_path.clone(),
        calls: Mutex::new(Vec::new()),
    });
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let creds = login_with_devin_cli(
        &cli,
        &data_dir,
        false,
        None,
        &mut RecordingInteraction::default(),
    )
    .unwrap();
    assert_eq!(creds.api_key, "new");
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls[1], vec!["auth", "login"]);
}

#[test]
fn login_fails_when_credentials_toml_missing() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    // status not logged in → login runs → toml 未生成 → error。
    let runner = Arc::new(FakeRunner {
        result: Ok(DevinCliResult {
            stdout: "not logged in".to_string(),
            stderr: String::new(),
            code: 1,
        }),
        calls: Mutex::new(Vec::new()),
    });
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let err = login_with_devin_cli(
        &cli,
        &data_dir,
        false,
        None,
        &mut RecordingInteraction::default(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("credentials.toml"));
    // credential 不在時は `auth status` を skip して `auth login` が
    // 1 件目（inherit stdio + 既定 binary）。
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args, vec!["auth", "login"]);
    assert!(calls[0].inherit_stdio);
    assert_eq!(calls[0].bin, "devin");
}

#[test]
fn logout_runs_cli_and_removes_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let cred_path = devin_credentials_path(&data_dir);
    std::fs::create_dir_all(cred_path.parent().unwrap()).unwrap();
    std::fs::write(&cred_path, "api_key = \"k\"\n").unwrap();
    let (cli, runner) = cli_with(ok(""), BTreeMap::new(), &data_dir);
    logout_devin_session(&cli, &data_dir, false, &mut RecordingInteraction::default());
    assert!(!cred_path.exists());
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls[0].args, vec!["auth", "logout"]);
    // CLI 未到達でも片付けは best-effort で成功する。
    std::fs::write(&cred_path, "api_key = \"k\"\n").unwrap();
    let (cli, _) = cli_with(
        Err(std::io::ErrorKind::NotFound),
        BTreeMap::new(),
        &data_dir,
    );
    logout_devin_session(&cli, &data_dir, false, &mut RecordingInteraction::default());
    assert!(!cred_path.exists());
}

// ---------------------------------------------------------------------------
// Windows credential contract（`windows: true` 注入 — Linux CI で実行可能）
// ---------------------------------------------------------------------------

/// Windows contract 用 runner — `auth status` は固定 result、
/// `auth login` は `login_code` を返し exit `0` 時に
/// `login_writes` の内容を global toml へ書き込む。
struct WindowsRunner {
    status: DevinCliResult,
    login_code: i32,
    login_writes: Option<(std::path::PathBuf, String)>,
    calls: Mutex<Vec<Recorded>>,
}

impl DevinCliRunner for WindowsRunner {
    fn run(
        &self,
        bin: &str,
        args: &[String],
        opts: &CliRunOptions<'_>,
    ) -> Result<DevinCliResult, std::io::Error> {
        self.calls.lock().unwrap().push(Recorded {
            bin: bin.to_string(),
            args: args.to_vec(),
            env: opts.env.clone(),
            inherit_stdio: opts.inherit_stdio,
        });
        if args == ["auth", "status"] {
            return Ok(self.status.clone());
        }
        if args == ["auth", "login"] {
            if self.login_code == 0 {
                if let Some((path, text)) = &self.login_writes {
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(path, text).unwrap();
                }
            }
            return Ok(DevinCliResult {
                stdout: String::new(),
                stderr: String::new(),
                code: self.login_code,
            });
        }
        Ok(DevinCliResult {
            stdout: String::new(),
            stderr: String::new(),
            code: 0,
        })
    }
}

fn windows_runner(
    status_stdout: String,
    login_code: i32,
    login_writes: Option<(std::path::PathBuf, String)>,
) -> Arc<WindowsRunner> {
    Arc::new(WindowsRunner {
        status: DevinCliResult {
            stdout: status_stdout,
            stderr: String::new(),
            code: 0,
        },
        login_code,
        login_writes,
        calls: Mutex::new(Vec::new()),
    })
}

fn write_toml(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

#[test]
fn windows_login_exit0_snapshots_global_toml() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let global_path = dir
        .path()
        .join("global")
        .join("devin")
        .join("credentials.toml");
    let runner = windows_runner(
        format!("You are logged in\n  File: {}\n", global_path.display()),
        0,
        Some((global_path.clone(), "api_key = \"gk\"\n".to_string())),
    );
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let mut interaction = RecordingInteraction::default();
    let creds = login_with_devin_cli(&cli, &data_dir, true, None, &mut interaction).unwrap();
    assert_eq!(creds.api_key, "gk");
    // global → cheaper data dir への snapshot copy。
    let dest = devin_credentials_path(&data_dir);
    assert_eq!(read_credentials_at(&dest).unwrap().api_key, "gk");
    // `auth status`（File: parse）→ `auth login` の 2 call。
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].args, vec!["auth", "status"]);
    assert_eq!(calls[1].args, vec!["auth", "login"]);
}

#[test]
fn windows_login_nonzero_exit_does_not_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let global_path = dir
        .path()
        .join("global")
        .join("devin")
        .join("credentials.toml");
    // global toml は既に存在する（別 account の stale session）。
    write_toml(&global_path, "api_key = \"other\"\n");
    let runner = windows_runner(
        format!("You are logged in\n  File: {}\n", global_path.display()),
        1,
        None,
    );
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let err = login_with_devin_cli(
        &cli,
        &data_dir,
        true,
        None,
        &mut RecordingInteraction::default(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("exited 1"));
    // abort した login が stale global を snapshot しない。
    assert!(!devin_credentials_path(&data_dir).exists());
}

#[test]
fn windows_login_reuses_on_matching_api_key() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    write_toml(&devin_credentials_path(&data_dir), "api_key = \"k\"\n");
    let global_path = dir
        .path()
        .join("global")
        .join("devin")
        .join("credentials.toml");
    write_toml(&global_path, "api_key = \"k\"\n");
    let runner = windows_runner(
        format!("You are logged in\n  File: {}\n", global_path.display()),
        0,
        None,
    );
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let mut interaction = RecordingInteraction::default();
    let creds = login_with_devin_cli(&cli, &data_dir, true, None, &mut interaction).unwrap();
    assert_eq!(creds.api_key, "k");
    // key 一致 → reuse。`auth login` は spawn せず `auth status`
    // （File: parse）の 1 回のみ。notify も発火しない。
    assert!(interaction.notifications.is_empty());
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args, vec!["auth", "status"]);
}

#[test]
fn windows_login_reuses_even_when_cli_reports_logged_out() {
    // `auth status` が logged-out（code != 0）でも `File:` 行があれば
    // global path として採用し、file 上の key 一致だけで reuse する
    // — CLI の logged_in 判定で reuse を gate する回帰を防ぐ。
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    write_toml(&devin_credentials_path(&data_dir), "api_key = \"k\"\n");
    let global_path = dir
        .path()
        .join("global")
        .join("devin")
        .join("credentials.toml");
    write_toml(&global_path, "api_key = \"k\"\n");
    let runner = Arc::new(WindowsRunner {
        status: DevinCliResult {
            stdout: format!("not logged in\r\n  File: {}\r\n", global_path.display()),
            stderr: String::new(),
            code: 1,
        },
        login_code: 0,
        login_writes: None,
        calls: Mutex::new(Vec::new()),
    });
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let creds = login_with_devin_cli(
        &cli,
        &data_dir,
        true,
        None,
        &mut RecordingInteraction::default(),
    )
    .unwrap();
    assert_eq!(creds.api_key, "k");
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args, vec!["auth", "status"]);
}

#[test]
fn windows_login_mismatch_notifies_then_snapshots_new_global() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    write_toml(&devin_credentials_path(&data_dir), "api_key = \"a\"\n");
    let global_path = dir
        .path()
        .join("global")
        .join("devin")
        .join("credentials.toml");
    write_toml(&global_path, "api_key = \"b\"\n");
    let runner = windows_runner(
        format!("You are logged in\n  File: {}\n", global_path.display()),
        0,
        Some((global_path.clone(), "api_key = \"b2\"\n".to_string())),
    );
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let mut interaction = RecordingInteraction::default();
    let creds = login_with_devin_cli(&cli, &data_dir, true, None, &mut interaction).unwrap();
    // login 後の新 global key が snapshot される。
    assert_eq!(creds.api_key, "b2");
    // 別 account 告知 + login 起動告知。
    assert!(interaction
        .notifications
        .iter()
        .any(|m| m.contains("different account")));
    assert!(interaction
        .notifications
        .iter()
        .any(|m| m.contains("Launching")));
    let calls = runner.calls.lock().unwrap();
    assert!(calls.iter().any(|c| c.args == vec!["auth", "login"]));
}

#[test]
fn windows_login_appdata_fallback_copies_after_login() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let appdata = dir.path().join("appdata");
    let global_path = appdata.join("devin").join("credentials.toml");
    // `File:` 行なし → APPDATA fallback。login が global toml を書く。
    let runner = windows_runner(
        "not logged in".to_string(),
        0,
        Some((global_path.clone(), "api_key = \"new\"\n".to_string())),
    );
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let mut interaction = RecordingInteraction::default();
    let creds = login_with_devin_cli(
        &cli,
        &data_dir,
        true,
        Some(appdata.to_str().unwrap()),
        &mut interaction,
    )
    .unwrap();
    assert_eq!(creds.api_key, "new");
    assert_eq!(
        read_credentials_at(&devin_credentials_path(&data_dir))
            .unwrap()
            .api_key,
        "new"
    );
    // global 不在 → 「別 account」の誤告知はしない（Launching のみ）。
    assert!(!interaction
        .notifications
        .iter()
        .any(|m| m.contains("different account")));
}

#[test]
fn windows_login_unresolved_global_path_after_login_errors() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    // `File:` 行なし + APPDATA 無し → global path 未解決のまま
    // login exit 0 → actionable error。
    let runner = windows_runner("not logged in".to_string(), 0, None);
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let err = login_with_devin_cli(
        &cli,
        &data_dir,
        true,
        None,
        &mut RecordingInteraction::default(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("could not be resolved"));
}

#[test]
fn windows_login_missing_global_after_login_errors() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let appdata = dir.path().join("appdata");
    // exit 0 だが global toml が現れない → actionable error。
    let runner = windows_runner("not logged in".to_string(), 0, None);
    let cli = DevinCli::new(BTreeMap::new(), &data_dir, Some(runner.clone()));
    let err = login_with_devin_cli(
        &cli,
        &data_dir,
        true,
        Some(appdata.to_str().unwrap()),
        &mut RecordingInteraction::default(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("no credentials file appeared"));
    assert!(!devin_credentials_path(&data_dir).exists());
}

#[test]
fn windows_logout_removes_snapshot_without_spawning_cli() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let cred_path = devin_credentials_path(&data_dir);
    write_toml(&cred_path, "api_key = \"k\"\n");
    let (cli, runner) = cli_with(ok(""), BTreeMap::new(), &data_dir);
    let mut interaction = RecordingInteraction::default();
    logout_devin_session(&cli, &data_dir, true, &mut interaction);
    assert!(!cred_path.exists());
    // `devin auth logout` は spawn しない（global session 非破壊）。
    assert!(runner.calls.lock().unwrap().is_empty());
    assert!(interaction
        .notifications
        .iter()
        .any(|m| m.contains("remains logged in")));
}

#[test]
fn resolve_global_path_parses_last_file_line() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let first = dir.path().join("first").join("credentials.toml");
    let last = dir.path().join("last path").join("credentials.toml");
    let appdata = dir.path().join("appdata");
    // CRLF 出力・複数 `File:` 行は最後を採用・APPDATA より `File:` 優先。
    let (cli, _) = cli_with(
        ok(&format!(
            "File: {}\r\nother\r\n  File: {}  \r\n",
            first.display(),
            last.display()
        )),
        BTreeMap::new(),
        &data_dir,
    );
    assert_eq!(
        resolve_global_credentials_path(&cli, appdata.to_str()),
        Some(last)
    );
}

#[test]
fn resolve_global_path_rejects_bogus_file_lines() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let appdata = dir.path().join("appdata");
    let expected = appdata.join("devin").join("credentials.toml");
    let bogus = dir.path().join("other").join("config.toml");
    // 空値・`credentials.toml` 以外を指す `File:` 行は採用せず
    // APPDATA fallback へ回る。
    let (cli, _) = cli_with(
        ok(&format!("File:   \nFile: {}\n", bogus.display())),
        BTreeMap::new(),
        &data_dir,
    );
    assert_eq!(
        resolve_global_credentials_path(&cli, appdata.to_str()),
        Some(expected)
    );
    // bogus な `File:` + APPDATA 無し → None。
    assert_eq!(resolve_global_credentials_path(&cli, None), None);
    // 空白のみの APPDATA も未設定扱い。
    let (cli, _) = cli_with(ok("not logged in"), BTreeMap::new(), &data_dir);
    assert_eq!(resolve_global_credentials_path(&cli, Some("  ")), None);
}

#[test]
fn resolve_global_path_appdata_fallback_and_none() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let appdata = dir.path().join("appdata");
    let expected = appdata.join("devin").join("credentials.toml");
    // `File:` 行なし → APPDATA fallback。
    let (cli, _) = cli_with(ok("not logged in"), BTreeMap::new(), &data_dir);
    assert_eq!(
        resolve_global_credentials_path(&cli, appdata.to_str()),
        Some(expected.clone())
    );
    // `File:` 行なし + APPDATA 無し → None。
    assert_eq!(resolve_global_credentials_path(&cli, None), None);
    // CLI 未到達 + APPDATA → fallback。
    let (cli, _) = cli_with(
        Err(std::io::ErrorKind::NotFound),
        BTreeMap::new(),
        &data_dir,
    );
    assert_eq!(
        resolve_global_credentials_path(&cli, appdata.to_str()),
        Some(expected)
    );
}

#[test]
fn resolve_global_path_parses_stderr_when_logged_out() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = devin_data_dir(dir.path());
    let path = dir.path().join("logged out path").join("credentials.toml");
    let (cli, _) = cli_with(
        Ok(DevinCliResult {
            stdout: "not logged in".to_string(),
            stderr: format!("  File: {}\n", path.display()),
            code: 1,
        }),
        BTreeMap::new(),
        &data_dir,
    );
    assert_eq!(resolve_global_credentials_path(&cli, None), Some(path));
}
