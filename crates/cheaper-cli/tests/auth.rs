//! `auth` command(`auth.rs`)の contract test — status shape・logout・
//! login の非 TTY 拒否・`TerminalInteraction` の select/secret/event
//! rendering(`auth.ts` の terminal I/O contract)。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod common;
use common::{PipeStdin, VecSink};

use cheaper_cli::args::{AuthAction, AuthCliArgs, AuthTypeArg, CliFormat};
use cheaper_cli::auth::{execute, execute_with, AuthTerminal, TerminalInteraction};
use cheaper_cli::cli::{run_cli, CliHooks, CliIo};
use cheaper_provider::devin::DevinProviderPlugin;
use cheaper_runtime::auth::{
    AuthEvent, AuthInfoLink, AuthInteraction, AuthPrompt, AuthSelectOption, AuthSource,
    ProviderEnv, ResolvedAuth,
};
use cheaper_runtime::catalog::ModelDefinitionJson;
use cheaper_runtime::{
    CheaperProviderPlugin, CredentialData, CredentialStore, PluginAuth, PluginCatalog,
    PluginProviderRegistration, ProviderPluginContext, ProviderRegistry, Runtime, StoredCredential,
};
use cheaper_tools::operation::{AbortController, AbortReason, AbortSignal};
use serde_json::Value;

// ---------------------------------------------------------------------------
// fakes
// ---------------------------------------------------------------------------

/// injected env — `resolve_user_config_dir` には CHEAPER_CONFIG_DIR を
/// 渡し、process env 由来の外乱を避ける。
struct EnvMap(BTreeMap<String, String>);
impl EnvMap {
    fn lookup(&self, name: &str) -> Option<String> {
        self.0.get(name).cloned()
    }
}

/// `ProviderEnv` — test runtime 用(`.env` file / home dir は扱わない)。
struct MapEnv(BTreeMap<String, String>);
impl ProviderEnv for MapEnv {
    fn env(&self, name: &str) -> Option<String> {
        self.0.get(name).filter(|v| !v.trim().is_empty()).cloned()
    }
    fn file_exists(&self, path: &str) -> bool {
        PathBuf::from(path).exists()
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
}

/// scripted terminal — stderr 出力を capture し、line/secret 入力は
/// queue から返す。
struct ScriptedTerminal {
    tty: bool,
    lines: std::collections::VecDeque<Result<String, std::io::Error>>,
    secrets: std::collections::VecDeque<Result<String, std::io::Error>>,
    out: String,
}
impl AuthTerminal for ScriptedTerminal {
    fn is_tty(&self) -> bool {
        self.tty
    }
    fn write(&mut self, s: &str) {
        self.out.push_str(s);
    }
    fn read_line(&mut self) -> std::io::Result<String> {
        self.lines.pop_front().unwrap_or_else(|| Ok(String::new()))
    }
    fn read_secret(&mut self) -> std::io::Result<String> {
        self.secrets
            .pop_front()
            .unwrap_or_else(|| Ok(String::new()))
    }
}

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// production と同じ bundled plugin(devin)を登録する test runtime。
fn test_runtime(dir: &std::path::Path, env: BTreeMap<String, String>) -> Runtime {
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![Arc::new(DevinProviderPlugin::new())];
    Runtime::create(
        dir.to_path_buf(),
        Arc::new(MapEnv(env)),
        &plugins,
        BTreeMap::new(),
    )
    .unwrap()
}

fn auth_args(action: AuthAction, provider: Option<&str>) -> AuthCliArgs {
    AuthCliArgs {
        action,
        provider: provider.map(String::from),
        method: None,
        auth_type: AuthTypeArg::Oauth,
        format: CliFormat::Json,
        quiet: false,
    }
}

fn no_open(_: &str) {
    panic!("open_url must not be called");
}

/// `Result<CommandOut, BoxError>::unwrap_err` — `CommandOut` は Debug
/// を持たないため err だけ取り出す helper。
fn expect_err(
    r: Result<cheaper_cli::cli::CommandOut, cheaper_cli::cli::BoxError>,
) -> cheaper_cli::cli::BoxError {
    match r {
        Ok(_) => panic!("expected error, got CommandOut"),
        Err(e) => e,
    }
}

fn ok_out(out: cheaper_cli::cli::CommandOut) -> Value {
    assert_eq!(out.exit_code, 0);
    out.envelope
}

/// production `execute` 経路 — `CHEAPER_CONFIG_DIR` を tmpdir に向ける。
async fn run_execute(
    args: &AuthCliArgs,
    config_dir: &std::path::Path,
    tty: bool,
) -> Result<cheaper_cli::cli::CommandOut, cheaper_cli::cli::BoxError> {
    let env_map = EnvMap(BTreeMap::from([(
        "CHEAPER_CONFIG_DIR".to_string(),
        config_dir.to_string_lossy().into_owned(),
    )]));
    let env_lookup = |name: &str| env_map.lookup(name);
    let mut stdin = PipeStdin::empty(tty);
    let mut out = VecSink(Vec::new());
    let mut err = VecSink(Vec::new());
    let mut io = CliIo {
        stdin: &mut stdin,
        stdout: &mut out,
        stderr: &mut err,
        env: &env_lookup,
        cwd: config_dir.to_string_lossy().into_owned(),
    };
    let hooks = CliHooks::default();
    execute(args, &mut io, &hooks, 1_000.0).await
}

/// `run_cli` 経由の dispatch(実 command 文字列 → envelope + exit code)。
async fn run_cli_auth(
    argv: &[&str],
    config_dir: &std::path::Path,
    tty: bool,
) -> (i32, String, String) {
    let env_map = EnvMap(BTreeMap::from([(
        "CHEAPER_CONFIG_DIR".to_string(),
        config_dir.to_string_lossy().into_owned(),
    )]));
    let env_lookup = |name: &str| env_map.lookup(name);
    let mut stdin = PipeStdin::empty(tty);
    let mut out = VecSink(Vec::new());
    let mut err = VecSink(Vec::new());
    let mut io = CliIo {
        stdin: &mut stdin,
        stdout: &mut out,
        stderr: &mut err,
        env: &env_lookup,
        cwd: config_dir.to_string_lossy().into_owned(),
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

// ---------------------------------------------------------------------------
// auth status
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn status_lists_providers_and_credentials_shape() {
    let dir = tmp();
    let store = CredentialStore::new(dir.path());
    store
        .set("openai", StoredCredential::api_key("sk-SEED-SECRET".into()))
        .unwrap();

    let args = auth_args(AuthAction::Status, None);
    let out = run_execute(&args, dir.path(), false).await.unwrap();
    let env = ok_out(out);
    assert_eq!(env["schemaVersion"], 1);
    assert_eq!(env["ok"], true);
    assert!(env["worker"].is_null());
    assert!(env["meta"]["durationMs"].is_number());

    // credentials[] — {providerId, type} のみ(secret 非開示)。
    let creds = env["result"]["credentials"].as_array().unwrap();
    assert_eq!(creds.len(), 1);
    assert_eq!(creds[0]["providerId"], "openai");
    assert_eq!(creds[0]["type"], "api_key");
    assert_eq!(
        creds[0].as_object().unwrap().keys().collect::<Vec<_>>(),
        vec!["providerId", "type"]
    );

    // providers[] — {providerId, name, methods, subscription, configured}。
    let providers = env["result"]["providers"].as_array().unwrap();
    assert!(providers.len() > 2);
    for p in providers {
        let keys: Vec<_> = p.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            vec![
                "providerId",
                "name",
                "methods",
                "subscription",
                "configured"
            ],
            "provider element shape: {p}"
        );
        assert!(p["methods"].is_array());
        assert!(p["subscription"].is_boolean());
        assert!(p["configured"].is_boolean());
    }
    // bundled plugin(devin)が provider list に載る。
    let devin = providers
        .iter()
        .find(|p| p["providerId"] == "devin")
        .expect("devin plugin provider must be listed");
    assert_eq!(devin["methods"], serde_json::json!(["oauth"]));
    assert_eq!(devin["subscription"], true);
    assert_eq!(devin["configured"], false); // credential 無し

    // openai は stored credential で configured。
    let openai = providers
        .iter()
        .find(|p| p["providerId"] == "openai")
        .unwrap();
    assert_eq!(openai["configured"], true);
    assert_eq!(openai["methods"], serde_json::json!(["api_key"]));

    // secret は出力のどこにも出ない。
    let raw = env.to_string();
    assert!(!raw.contains("sk-SEED-SECRET"), "secret leaked: {raw}");
}

#[tokio::test(flavor = "current_thread")]
async fn status_empty_store() {
    let dir = tmp();
    let args = auth_args(AuthAction::Status, None);
    let env = ok_out(run_execute(&args, dir.path(), false).await.unwrap());
    assert_eq!(env["result"]["credentials"], serde_json::json!([]));
    assert!(!env["result"]["providers"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn status_provider_filter() {
    let dir = tmp();
    let args = auth_args(AuthAction::Status, Some("anthropic"));
    let env = ok_out(run_execute(&args, dir.path(), false).await.unwrap());
    let providers = env["result"]["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0]["providerId"], "anthropic");
    assert_eq!(providers[0]["name"], "Anthropic");
}

#[tokio::test(flavor = "current_thread")]
async fn status_unknown_provider_is_invalid_request() {
    let dir = tmp();
    let args = auth_args(AuthAction::Status, Some("nope"));
    let err = expect_err(run_execute(&args, dir.path(), false).await);
    let e = err
        .downcast_ref::<cheaper_core::errors::CheaperError>()
        .unwrap();
    assert_eq!(e.code(), cheaper_core::errors::ErrorCode::InvalidRequest);
    assert!(e.message().contains("unknown provider: nope"));
}

// ---------------------------------------------------------------------------
// auth logout
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn logout_removes_credential_and_is_idempotent() {
    let dir = tmp();
    let store = CredentialStore::new(dir.path());
    store
        .set("openai", StoredCredential::api_key("sk-x".into()))
        .unwrap();

    let args = auth_args(AuthAction::Logout, Some("openai"));
    let env = ok_out(run_execute(&args, dir.path(), false).await.unwrap());
    assert_eq!(
        env["result"],
        serde_json::json!({"provider": "openai", "loggedOut": true})
    );
    assert!(store.get("openai").unwrap().is_none());

    // 未登録 credential でも冪等に成功。
    let env = ok_out(run_execute(&args, dir.path(), false).await.unwrap());
    assert_eq!(env["result"]["loggedOut"], true);
}

#[tokio::test(flavor = "current_thread")]
async fn logout_unknown_provider_is_invalid_request() {
    let dir = tmp();
    let args = auth_args(AuthAction::Logout, Some("nope"));
    let err = expect_err(run_execute(&args, dir.path(), false).await);
    let e = err
        .downcast_ref::<cheaper_core::errors::CheaperError>()
        .unwrap();
    assert_eq!(e.code(), cheaper_core::errors::ErrorCode::InvalidRequest);
}

// ---------------------------------------------------------------------------
// auth login — dispatch / non-TTY / scripted
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn login_api_key_non_tty_rejected_via_cli() {
    let dir = tmp();
    let (code, out, err) = run_cli_auth(
        &["auth", "login", "openai", "--type", "api_key"],
        dir.path(),
        false,
    )
    .await;
    assert_eq!(code, 2);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert_eq!(
        v["error"]["message"],
        "auth secret prompt requires an interactive terminal"
    );
    assert_eq!(v["error"]["details"]["promptType"], "secret");
    assert!(err.starts_with("error INVALID_REQUEST:"));
    // credential は書き込まれない。
    assert!(!dir.path().join("credentials.json").exists());
}

#[tokio::test(flavor = "current_thread")]
async fn login_oauth_text_prompt_non_tty_rejected() {
    let dir = tmp();
    // github-copilot oauth login の最初の prompt は Text
    // (GitHub Enterprise URL)。非 TTY → INVALID_REQUEST、network 未到達。
    let (code, out, _err) =
        run_cli_auth(&["auth", "login", "github-copilot"], dir.path(), false).await;
    assert_eq!(code, 2);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert_eq!(
        v["error"]["message"],
        "auth text prompt requires an interactive terminal"
    );
    assert_eq!(v["error"]["details"]["promptType"], "text");
}

#[tokio::test(flavor = "current_thread")]
async fn login_unsupported_method_is_invalid_request() {
    let dir = tmp();
    // openai に oauth login → method 非対応。
    let (code, out, _err) = run_cli_auth(&["auth", "login", "openai"], dir.path(), false).await;
    assert_eq!(code, 2);
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .contains("provider openai does not support oauth login"));
}

#[tokio::test(flavor = "current_thread")]
async fn login_api_key_tty_persists_credential() {
    let dir = tmp();
    let runtime = test_runtime(dir.path(), BTreeMap::new());
    let mut terminal = ScriptedTerminal {
        tty: true,
        lines: Default::default(),
        secrets: [Ok("sk-TEST-KEY".to_string())].into_iter().collect(),
        out: String::new(),
    };
    let signal = AbortSignal::detached();
    let args = AuthCliArgs {
        auth_type: AuthTypeArg::ApiKey,
        ..auth_args(AuthAction::Login, Some("openai"))
    };
    // `now` は credential 永続化の有無で値を変える — `durationMs` が
    // login 完了後に採られれば 234.0、先読みなら 100.0 になる。
    let now = || {
        if runtime
            .auth()
            .credentials()
            .get("openai")
            .unwrap()
            .is_some()
        {
            1_234.0
        } else {
            1_100.0
        }
    };
    let out = execute_with(
        &args,
        &runtime,
        &mut terminal,
        &signal,
        &no_open,
        &now,
        1_000.0,
    )
    .await
    .unwrap();
    let env = ok_out(out);
    assert_eq!(
        env["result"],
        serde_json::json!({
            "provider": "openai",
            "authenticated": true,
            "method": "api_key",
            "subscription": false,
        })
    );
    assert_eq!(env["meta"]["durationMs"], 234.0);
    // secret prompt は stderr 側に message + 改行のみ。
    assert!(
        terminal.out.contains("Enter OpenAI API key: \n"),
        "stderr: {:?}",
        terminal.out
    );
    assert!(!terminal.out.contains("sk-TEST-KEY"));
    // credential は永続化され、envelope に secret は出ない。
    let stored = runtime.auth().credentials().get("openai").unwrap().unwrap();
    assert_eq!(stored.as_api_key(), Some("sk-TEST-KEY"));
    assert!(!env.to_string().contains("sk-TEST-KEY"));
}

// ---------------------------------------------------------------------------
// TerminalInteraction — select / event / cancel
// ---------------------------------------------------------------------------

fn select_options<'a>() -> Vec<AuthSelectOption<'a>> {
    vec![
        AuthSelectOption {
            id: "device",
            label: "Device code",
            description: Some("use a browser on another device"),
        },
        AuthSelectOption {
            id: "browser",
            label: "Browser",
            description: None,
        },
    ]
}

#[test]
fn select_method_immediate_answer_needs_no_tty() {
    let mut term = ScriptedTerminal {
        tty: false,
        lines: Default::default(),
        secrets: Default::default(),
        out: String::new(),
    };
    let options = select_options();
    let mut interaction = TerminalInteraction::new(&mut term, Some("device"), None, &no_open);
    let answer = interaction
        .prompt(&AuthPrompt::Select {
            message: "How?",
            options: &options,
        })
        .unwrap();
    assert_eq!(answer, "device");
    assert!(term.out.is_empty());
}

#[test]
fn select_method_mismatch_non_tty_is_invalid_request() {
    let mut term = ScriptedTerminal {
        tty: false,
        lines: Default::default(),
        secrets: Default::default(),
        out: String::new(),
    };
    let options = select_options();
    let mut interaction = TerminalInteraction::new(&mut term, Some("qr"), None, &no_open);
    let err = interaction
        .prompt(&AuthPrompt::Select {
            message: "How?",
            options: &options,
        })
        .unwrap_err();
    assert_eq!(err.code(), cheaper_core::errors::ErrorCode::InvalidRequest);
    assert_eq!(
        err.message(),
        "interactive selection required; re-run with --method <device|browser>"
    );
    assert_eq!(
        err.details().unwrap()["options"],
        serde_json::json!(["device", "browser"])
    );
    // method mismatch の note は先に stderr へ出る。
    assert_eq!(
        term.out,
        "note: --method qr did not match options (device, browser)\n"
    );
}

#[test]
fn select_tty_reads_numbered_answer() {
    let mut term = ScriptedTerminal {
        tty: true,
        lines: [Ok("  2\n".to_string())].into_iter().collect(),
        secrets: Default::default(),
        out: String::new(),
    };
    let options = select_options();
    let mut interaction = TerminalInteraction::new(&mut term, None, None, &no_open);
    let answer = interaction
        .prompt(&AuthPrompt::Select {
            message: "Choose a method",
            options: &options,
        })
        .unwrap();
    assert_eq!(answer, "browser");
    assert_eq!(
        term.out,
        "Choose a method\n  1) Device code — use a browser on another device\n  2) Browser\nselect [1-2]: "
    );
}

#[test]
fn select_invalid_answer_is_invalid_request() {
    let mut term = ScriptedTerminal {
        tty: true,
        lines: [Ok("9".to_string())].into_iter().collect(),
        secrets: Default::default(),
        out: String::new(),
    };
    let options = select_options();
    let mut interaction = TerminalInteraction::new(&mut term, None, None, &no_open);
    let err = interaction
        .prompt(&AuthPrompt::Select {
            message: "How?",
            options: &options,
        })
        .unwrap_err();
    assert_eq!(err.code(), cheaper_core::errors::ErrorCode::InvalidRequest);
    assert_eq!(err.message(), "invalid selection: 9");
}

#[test]
fn secret_prompt_writes_message_and_newline() {
    let mut term = ScriptedTerminal {
        tty: true,
        lines: Default::default(),
        secrets: [Ok("s3cr3t".to_string())].into_iter().collect(),
        out: String::new(),
    };
    let mut interaction = TerminalInteraction::new(&mut term, None, None, &no_open);
    let answer = interaction
        .prompt(&AuthPrompt::Secret {
            message: "Enter API key",
            placeholder: None,
        })
        .unwrap();
    assert_eq!(answer, "s3cr3t");
    // `{message}: ` + cleanup の "\n"。
    assert_eq!(term.out, "Enter API key: \n");
}

#[test]
fn secret_prompt_ctrl_c_is_login_cancelled() {
    let mut term = ScriptedTerminal {
        tty: true,
        lines: Default::default(),
        secrets: [Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "interrupted",
        ))]
        .into_iter()
        .collect(),
        out: String::new(),
    };
    let mut interaction = TerminalInteraction::new(&mut term, None, None, &no_open);
    let err = interaction
        .prompt(&AuthPrompt::Secret {
            message: "Enter API key",
            placeholder: None,
        })
        .unwrap_err();
    assert_eq!(err.code(), cheaper_core::errors::ErrorCode::InvalidRequest);
    assert_eq!(err.message(), "login cancelled");
}

#[test]
fn cancelled_signal_makes_is_cancelled_true() {
    let mut term = ScriptedTerminal {
        tty: true,
        lines: Default::default(),
        secrets: Default::default(),
        out: String::new(),
    };
    let controller = AbortController::new();
    controller.abort(AbortReason::aborted());
    let mut interaction =
        TerminalInteraction::new(&mut term, None, Some(controller.signal()), &no_open);
    assert!(interaction.is_cancelled());
    // 既 abort の signal で text prompt → aborted error。
    let err = interaction
        .prompt(&AuthPrompt::Text {
            message: "Enter code",
            placeholder: None,
        })
        .unwrap_err();
    assert_eq!(err.code(), cheaper_core::errors::ErrorCode::InternalError);
}

#[test]
fn event_rendering_matches_ts() {
    let opened: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = {
        let opened = Arc::clone(&opened);
        move |url: &str| opened.lock().unwrap().push(url.to_string())
    };
    let mut term = ScriptedTerminal {
        tty: false,
        lines: Default::default(),
        secrets: Default::default(),
        out: String::new(),
    };
    let mut interaction = TerminalInteraction::new(&mut term, None, None, &captured);
    interaction
        .event(&AuthEvent::AuthUrl {
            url: "https://auth.example/login?a=1".to_string(),
            instructions: Some("then paste the code".to_string()),
        })
        .unwrap();
    interaction
        .event(&AuthEvent::DeviceCode {
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            interval_seconds: Some(5),
            expires_in_seconds: Some(900),
        })
        .unwrap();
    interaction
        .event(&AuthEvent::Info {
            message: "read the docs".to_string(),
            links: vec![
                AuthInfoLink {
                    url: "https://a".to_string(),
                    label: None,
                },
                AuthInfoLink {
                    url: "https://b".to_string(),
                    label: Some("guide".to_string()),
                },
            ],
        })
        .unwrap();
    interaction
        .event(&AuthEvent::Progress {
            message: "Enabling models...".to_string(),
        })
        .unwrap();
    interaction.notify("plain note").unwrap();
    assert_eq!(
        term.out,
        "\nOpen this URL to sign in:\n  https://auth.example/login?a=1\n\nthen paste the code\n\
         \nGo to https://github.com/login/device and enter code: ABCD-1234\n(expires in 900s)\n\
         read the docs\n  link: https://a\n  guide: https://b\n\
         Enabling models...\nplain note\n"
    );
    assert_eq!(
        opened.lock().unwrap().as_slice(),
        &["https://auth.example/login?a=1".to_string()]
    );
}

// ---------------------------------------------------------------------------
// catalog error の stderr surfacing(spec Req 4 — `auth` 系は action
// dispatch 前の early emit + login Ok 後の差分 emit)
// ---------------------------------------------------------------------------

/// `providers/devin/catalog.json` に malformed snapshot を配置する —
/// `Runtime::create` の snapshot load error が catalog errors に記録
/// され `warning:` 行として stderr に出る fixture。
fn write_malformed_devin_catalog(dir: &std::path::Path) {
    let provider_dir = dir.join("providers").join("devin");
    std::fs::create_dir_all(&provider_dir).unwrap();
    std::fs::write(provider_dir.join("catalog.json"), "{ not json").unwrap();
}

/// refresh error 注入用の最小 fake plugin — `auth login fake` は
/// prompt 無しで oauth credential を返し、`refreshModels` は Err を
/// 返す(post-login 差分 emit の検証用。`run_cli` 非 TTY 経路は対話
/// prompt 拒否で refresh 到達前に失敗するため `execute_with` +
/// `ScriptedTerminal` で検証する)。
struct FailingCatalogPlugin;

impl CheaperProviderPlugin for FailingCatalogPlugin {
    fn id(&self) -> &str {
        "fake"
    }
    fn register(&self, registry: &mut ProviderRegistry, _ctx: &ProviderPluginContext) {
        registry.register_provider(PluginProviderRegistration {
            id: "fake".to_string(),
            name: "Fake Provider".to_string(),
            api: "fake-api".to_string(),
            base_url: "http://fake.local".to_string(),
            api_key: None,
            auth_header: None,
            headers: None,
            models: vec![
                serde_json::from_str::<ModelDefinitionJson>(r#"{"id":"fake-1"}"#).unwrap(),
            ],
            auth: Some(Arc::new(FakePluginAuth)),
            subscription: true,
            catalog: Some(Arc::new(FailingPluginCatalog)),
        });
    }
}

struct FakePluginAuth;
impl PluginAuth for FakePluginAuth {
    fn login(
        &self,
        _ctx: &ProviderPluginContext,
        _interaction: &mut dyn AuthInteraction,
    ) -> Result<StoredCredential, cheaper_core::errors::CheaperError> {
        Ok(StoredCredential::oauth(CredentialData::Oauth {
            access: "fake-token".into(),
            refresh: String::new(),
            expires: i64::MAX,
            account_id: None,
            subscription: None,
            available_model_ids: None,
            extra: serde_json::Map::new(),
        }))
    }
    fn to_auth(
        &self,
        _ctx: &ProviderPluginContext,
        _credential: &StoredCredential,
    ) -> Result<ResolvedAuth, cheaper_core::errors::CheaperError> {
        Ok(ResolvedAuth {
            api_key: Some("fake-token".into()),
            headers: BTreeMap::new(),
            base_url: None,
            source: AuthSource::OAuth,
        })
    }
}

struct FailingPluginCatalog;
impl PluginCatalog for FailingPluginCatalog {
    fn refresh_models(
        &self,
        _ctx: &ProviderPluginContext,
        _credential: Option<&StoredCredential>,
    ) -> Result<Option<Vec<ModelDefinitionJson>>, cheaper_core::errors::CheaperError> {
        Err(cheaper_core::errors::CheaperError::new(
            cheaper_core::errors::ErrorCode::InternalError,
            "refresh boom",
        ))
    }
}

fn fake_plugin_runtime(dir: &std::path::Path) -> Runtime {
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![Arc::new(FailingCatalogPlugin)];
    Runtime::create(
        dir.to_path_buf(),
        Arc::new(MapEnv(BTreeMap::new())),
        &plugins,
        BTreeMap::new(),
    )
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn status_emits_catalog_warnings_for_malformed_plugin_catalog() {
    // spec Req 4 — snapshot load error は action dispatch 前の early
    // emit で stderr の `warning:` 行になる。envelope/exit code 不変。
    let dir = tmp();
    write_malformed_devin_catalog(dir.path());
    let (code, out, err) = run_cli_auth(&["auth", "status"], dir.path(), false).await;
    assert_eq!(code, 0, "stderr: {err}");
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["schemaVersion"], serde_json::json!(1));
    assert!(v["result"]["providers"].is_array());
    assert!(
        err.contains("warning: Provider \"devin\":"),
        "stderr: {err}"
    );
    assert!(err.contains("plugin catalog"), "stderr: {err}");
}

#[tokio::test(flavor = "current_thread")]
async fn status_quiet_suppresses_catalog_warnings() {
    let dir = tmp();
    write_malformed_devin_catalog(dir.path());
    let (code, out, err) = run_cli_auth(&["auth", "status", "--quiet"], dir.path(), false).await;
    assert_eq!(code, 0, "stderr: {err}");
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["ok"], true);
    assert!(!err.contains("warning:"), "stderr: {err}");
}

#[tokio::test(flavor = "current_thread")]
async fn status_dropped_provider_emits_warning_and_unknown_provider() {
    // compose 時に drop された plugin provider への `auth status <id>`
    // — early emit が action dispatch 前に走るため `warning:` 行と
    // `unknown provider` の envelope error が併存する。
    let dir = tmp();
    // `{"oauth":"radius"}`(baseUrl 無し)→ oauth⇒baseUrl 検証 Err で
    // devin provider が drop される。
    std::fs::write(
        dir.path().join("models.json"),
        r#"{"providers":{"devin":{"oauth":"radius"}}}"#,
    )
    .unwrap();
    let (code, out, err) = run_cli_auth(&["auth", "status", "devin"], dir.path(), false).await;
    assert_eq!(code, 2, "stderr: {err}");
    let v: Value = serde_json::from_str(out.trim_end()).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "INVALID_REQUEST");
    assert_eq!(v["error"]["message"], "unknown provider: devin");
    // compose error は warning 行として action 失敗の前に emit 済み。
    assert!(
        err.contains("warning: Provider \"devin\":"),
        "stderr: {err}"
    );
    assert!(err.contains("baseUrl"), "stderr: {err}");
    assert!(
        err.contains("error INVALID_REQUEST: unknown provider: devin"),
        "stderr: {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn status_execute_with_emits_compose_warnings_via_terminal() {
    // `execute_with` の early emit は `AuthTerminal::write`(stderr 境界)
    // 経由で出る — scripted terminal の capture seam で直接 assert する
    // (sink seam の検証)。
    let dir = tmp();
    write_malformed_devin_catalog(dir.path());
    let runtime = test_runtime(dir.path(), BTreeMap::new());
    let mut terminal = ScriptedTerminal {
        tty: false,
        lines: Default::default(),
        secrets: Default::default(),
        out: String::new(),
    };
    let signal = AbortSignal::detached();
    let args = auth_args(AuthAction::Status, None);
    let out = execute_with(
        &args,
        &runtime,
        &mut terminal,
        &signal,
        &no_open,
        &|| 1_234.0,
        1_000.0,
    )
    .await
    .unwrap();
    ok_out(out);
    assert!(
        terminal.out.contains("warning: Provider \"devin\":"),
        "stderr: {:?}",
        terminal.out
    );
    assert!(
        terminal.out.contains("plugin catalog"),
        "stderr: {:?}",
        terminal.out
    );
}

#[tokio::test(flavor = "current_thread")]
async fn login_emits_post_login_catalog_warnings() {
    // spec Req 4 — login 中に `refresh_plugin_catalog` が記録した error
    // は login `Ok` 後の差分 emit で同一 run の stderr へ出る。
    let dir = tmp();
    let runtime = fake_plugin_runtime(dir.path());
    let mut terminal = ScriptedTerminal {
        tty: true,
        lines: Default::default(),
        secrets: Default::default(),
        out: String::new(),
    };
    let signal = AbortSignal::detached();
    let args = auth_args(AuthAction::Login, Some("fake"));
    let out = execute_with(
        &args,
        &runtime,
        &mut terminal,
        &signal,
        &no_open,
        &|| 1_234.0,
        1_000.0,
    )
    .await
    .unwrap();
    let env = ok_out(out);
    // envelope `result` shape は不変。
    assert_eq!(env["schemaVersion"], serde_json::json!(1));
    assert_eq!(
        env["result"],
        serde_json::json!({
            "provider": "fake",
            "authenticated": true,
            "method": "oauth",
            "subscription": true,
        })
    );
    // refresh error が post-login 差分 emit で warning 行として出る。
    // compose 時点では error 無し — emit は差分のみ(重複 emit 無し)。
    assert!(
        terminal.out.contains("warning: Provider \"fake\":"),
        "stderr: {:?}",
        terminal.out
    );
    assert!(
        terminal.out.contains("refreshModels failed"),
        "stderr: {:?}",
        terminal.out
    );
    assert_eq!(
        terminal.out.matches("warning:").count(),
        1,
        "stderr: {:?}",
        terminal.out
    );
}

#[tokio::test(flavor = "current_thread")]
async fn login_quiet_suppresses_post_login_warnings() {
    let dir = tmp();
    let runtime = fake_plugin_runtime(dir.path());
    let mut terminal = ScriptedTerminal {
        tty: true,
        lines: Default::default(),
        secrets: Default::default(),
        out: String::new(),
    };
    let signal = AbortSignal::detached();
    let args = AuthCliArgs {
        quiet: true,
        ..auth_args(AuthAction::Login, Some("fake"))
    };
    let out = execute_with(
        &args,
        &runtime,
        &mut terminal,
        &signal,
        &no_open,
        &|| 1_234.0,
        1_000.0,
    )
    .await
    .unwrap();
    ok_out(out);
    assert!(
        !terminal.out.contains("warning:"),
        "stderr: {:?}",
        terminal.out
    );
}
