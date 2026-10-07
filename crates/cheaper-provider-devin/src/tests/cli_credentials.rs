use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::cli::*;
use crate::credentials::*;

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
    let creds = login_with_devin_cli(&cli, &data_dir).unwrap();
    assert_eq!(creds.api_key, "fresh");
    // `auth status` のみで `auth login` は走らない。
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
    let creds = login_with_devin_cli(&cli, &data_dir).unwrap();
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
    let err = login_with_devin_cli(&cli, &data_dir).unwrap_err();
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
    logout_devin_session(&cli, &data_dir);
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
    logout_devin_session(&cli, &data_dir);
    assert!(!cred_path.exists());
}
