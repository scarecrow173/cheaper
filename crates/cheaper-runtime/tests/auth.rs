//! credential store + api-key 解決 + auth service の contract test。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use cheaper_core::errors::ErrorCode;
use cheaper_runtime::auth::api_key::{
    get_config_value_env_var_names, is_command_config_value, OverlayEnv, ProviderEnv,
};
use cheaper_runtime::auth::service::{AuthInteraction, AuthPrompt, AuthService};
use cheaper_runtime::auth::store::{AuthType, CredentialData, CredentialStore, StoredCredential};
use cheaper_runtime::catalog::CatalogStore;
use cheaper_runtime::plugin::ProviderPluginContext;

/// test 用 env accessor（process env に依存しない注入 map）。
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

fn overlay<'a>(explicit: BTreeMap<String, String>, env: &'a dyn ProviderEnv) -> OverlayEnv<'a> {
    OverlayEnv::new(explicit, env)
}

fn tmp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn service(dir: &std::path::Path, env: Arc<dyn ProviderEnv>) -> AuthService {
    AuthService::new(
        ProviderPluginContext {
            user_config_dir: dir.to_path_buf(),
            env,
        },
        CredentialStore::new(dir),
        BTreeMap::new(),
        BTreeMap::new(),
    )
}

// ---- credential store ----

#[test]
fn credential_store_roundtrip_api_key() {
    let dir = tmp_dir();
    let store = CredentialStore::new(dir.path());
    assert_eq!(store.path(), dir.path().join("credentials.json"));
    store
        .set("openai", StoredCredential::api_key("sk-test".to_string()))
        .unwrap();
    let cred = store.get("openai").unwrap().unwrap();
    assert_eq!(cred.as_api_key(), Some("sk-test"));
    // file は新規 schema（version/providers）。
    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(raw["version"], 1);
    assert_eq!(raw["providers"]["openai"]["type"], "api_key");
    assert_eq!(raw["providers"]["openai"]["key"], "sk-test");
}

#[test]
fn credential_store_atomic_replace_and_remove() {
    let dir = tmp_dir();
    let store = CredentialStore::new(dir.path());
    store
        .set("anthropic", StoredCredential::api_key("k1".into()))
        .unwrap();
    store
        .set("anthropic", StoredCredential::api_key("k2".into()))
        .unwrap();
    assert_eq!(
        store.get("anthropic").unwrap().unwrap().as_api_key(),
        Some("k2")
    );
    assert!(store.remove("anthropic").unwrap());
    assert!(store.get("anthropic").unwrap().is_none());
    // temp file が残らない（atomic write の痕跡なし）。
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("tmp"))
        .collect();
    assert!(leftovers.is_empty());
}

/// F12: Unix の `rename(2)` は既存 target を原子置換するため、並行
/// reader は file 不在 / 不完全な JSON を一切観測しない。旧実装の
/// remove→rename は置換毎に file 不在の crash window を作っていた
/// （この test は window を必ず捉える訳ではないが、fixed impl では
/// 絶対に失敗しない — 回帰検出のみを目的とする）。
#[test]
#[cfg(unix)]
fn credential_store_replace_never_exposes_missing_file_on_unix() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let dir = tmp_dir();
    let store = CredentialStore::new(dir.path());
    store
        .set("openai", StoredCredential::api_key("k0".into()))
        .unwrap();
    let path = store.path().to_path_buf();
    let stop = Arc::new(AtomicBool::new(false));
    let reader = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let raw = std::fs::read_to_string(&path)
                    .expect("credentials.json must never be absent during replace");
                let parsed: serde_json::Value =
                    serde_json::from_str(&raw).expect("credentials.json is always valid JSON");
                assert!(parsed["providers"]["openai"].is_object());
            }
        })
    };
    for i in 1..=64 {
        store
            .set("openai", StoredCredential::api_key(format!("k{i}")))
            .unwrap();
    }
    stop.store(true, Ordering::Relaxed);
    reader.join().unwrap();
    assert_eq!(
        store.get("openai").unwrap().unwrap().as_api_key(),
        Some("k64")
    );
}

#[test]
fn credential_store_oauth_extra_claims() {
    let dir = tmp_dir();
    let store = CredentialStore::new(dir.path());
    let mut extra = serde_json::Map::new();
    extra.insert(
        "tomlPath".to_string(),
        serde_json::json!("C:/devin/config.toml"),
    );
    store
        .set(
            "devin",
            StoredCredential::oauth(CredentialData::Oauth {
                access: "a".into(),
                refresh: "r".into(),
                expires: 999999999,
                account_id: None,
                subscription: Some("pro".into()),
                available_model_ids: Some(vec!["m1".into()]),
                extra,
            }),
        )
        .unwrap();
    let cred = store.get("devin").unwrap().unwrap();
    assert_eq!(cred.auth_type(), AuthType::Oauth);
    assert_eq!(cred.available_model_ids().unwrap(), vec!["m1"]);
    match &cred.data {
        CredentialData::Oauth {
            extra,
            subscription,
            ..
        } => {
            assert_eq!(extra["tomlPath"], "C:/devin/config.toml");
            assert_eq!(subscription.as_deref(), Some("pro"));
        }
        _ => panic!(),
    }
}

#[test]
fn credential_store_never_reads_legacy_pi_state() {
    let dir = tmp_dir();
    // legacy pi state を配置しても参照しない。
    std::fs::create_dir_all(dir.path().join("pi")).unwrap();
    std::fs::write(
        dir.path().join("pi").join("auth.json"),
        r#"{"anthropic": {"type": "api_key", "key": "legacy"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("auth.json"),
        r#"{"anthropic": {"type": "api_key", "key": "legacy2"}}"#,
    )
    .unwrap();
    let store = CredentialStore::new(dir.path());
    assert!(store.get("anthropic").unwrap().is_none());
    assert!(store.all().unwrap().is_empty());
    // credentials.json も作成されない（read は file を作らない）。
    assert!(!dir.path().join("credentials.json").exists());
}

// ---- config value template ----

#[test]
fn template_env_var_resolution() {
    let env = MapEnv(BTreeMap::from([
        ("A".to_string(), "va".to_string()),
        ("B".to_string(), "vb".to_string()),
    ]));
    let ov = overlay(BTreeMap::new(), &env);
    assert_eq!(
        cheaper_runtime::auth::api_key::resolve_config_value("$A-$B", &ov),
        Some("va-vb".to_string())
    );
    assert_eq!(
        cheaper_runtime::auth::api_key::resolve_config_value("${A}x", &ov),
        Some("vax".to_string())
    );
    // 未設定 var 参照は全体未解決。
    assert_eq!(
        cheaper_runtime::auth::api_key::resolve_config_value("$A-$MISSING", &ov),
        None
    );
    // escape: `$$` → `$`、`$!` → `!`。
    assert_eq!(
        cheaper_runtime::auth::api_key::resolve_config_value("$$A", &ov),
        Some("$A".to_string())
    );
    assert_eq!(
        cheaper_runtime::auth::api_key::resolve_config_value("$!echo x", &ov),
        Some("!echo x".to_string())
    );
    // literal。
    assert_eq!(
        cheaper_runtime::auth::api_key::resolve_config_value("sk-literal", &ov),
        Some("sk-literal".to_string())
    );
}

#[test]
fn template_var_name_extraction() {
    assert_eq!(
        get_config_value_env_var_names("$A-${B}-lit"),
        vec!["A".to_string(), "B".to_string()]
    );
    assert_eq!(get_config_value_env_var_names("!cmd"), Vec::<String>::new());
    assert!(is_command_config_value("!gh auth token"));
    assert!(!is_command_config_value("$!literal"));
}

#[test]
fn or_throw_error_messages_match_ts() {
    let env = MapEnv(BTreeMap::new());
    let ov = overlay(BTreeMap::new(), &env);
    let err = cheaper_runtime::auth::api_key::resolve_config_value_or_throw(
        "$MISSING",
        "API key for provider \"openai\"",
        &ov,
    )
    .unwrap_err();
    assert_eq!(
        err.message(),
        "Failed to resolve API key for provider \"openai\" from environment variable: MISSING"
    );
    let err = cheaper_runtime::auth::api_key::resolve_config_value_or_throw(
        "$A-$B",
        "API key for provider \"openai\"",
        &ov,
    )
    .unwrap_err();
    assert_eq!(
        err.message(),
        "Failed to resolve API key for provider \"openai\" from environment variables: A, B"
    );
    let err = cheaper_runtime::auth::api_key::resolve_config_value_or_throw(
        "!false",
        "API key for provider \"openai\"",
        &ov,
    )
    .unwrap_err();
    assert_eq!(
        err.message(),
        "Failed to resolve API key for provider \"openai\" from shell command: false"
    );
}

#[test]
fn command_config_value_executes_and_caches() {
    let env = MapEnv(BTreeMap::new());
    let ov = overlay(BTreeMap::new(), &env);
    let v = cheaper_runtime::auth::api_key::resolve_config_value("!echo secret-key", &ov);
    assert_eq!(v, Some("secret-key".to_string()));
}

/// F11: pipe buffer(〜64KB)を超える stdout を持つ `!cmd` でも drain が
/// 先行するため子の write block → timeout 死にしない。
#[test]
#[cfg(unix)]
fn command_stdout_over_pipe_buffer_does_not_deadlock() {
    let env = MapEnv(BTreeMap::new());
    let ov = overlay(BTreeMap::new(), &env);
    // `seq 1 50000` は ~289KB — 64KB pipe buffer を明確に超過。
    let v = cheaper_runtime::auth::api_key::resolve_config_value("!seq 1 50000", &ov)
        .expect("large stdout command resolves");
    assert!(v.len() >= 200_000, "len={}", v.len());
    assert!(v.ends_with("50000"));
}

#[test]
#[cfg(windows)]
fn command_stdout_over_pipe_buffer_does_not_deadlock() {
    let env = MapEnv(BTreeMap::new());
    let ov = overlay(BTreeMap::new(), &env);
    // cmd builtin の `for /l` で ~260KB の stdout を生成。
    let v = cheaper_runtime::auth::api_key::resolve_config_value(
        "!for /l %i in (1,1,4000) do @echo xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        &ov,
    )
    .expect("large stdout command resolves");
    assert!(v.len() >= 200_000, "len={}", v.len());
}

// ---- auth service ----

struct ScriptedInteraction {
    answers: Vec<String>,
}
impl AuthInteraction for ScriptedInteraction {
    fn prompt(
        &mut self,
        _prompt: &AuthPrompt<'_>,
    ) -> Result<String, cheaper_core::errors::CheaperError> {
        Ok(self.answers.remove(0))
    }
}

#[test]
fn api_key_login_persists_credential_and_status_reflects() {
    let dir = tmp_dir();
    let env: Arc<dyn ProviderEnv> = Arc::new(MapEnv(BTreeMap::new()));
    let svc = service(dir.path(), env);
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    let mut interaction = ScriptedInteraction {
        answers: vec!["sk-secret".to_string()],
    };
    let result = svc
        .login(&catalog, "openai", AuthType::ApiKey, &mut interaction)
        .unwrap();
    assert_eq!(result.provider, "openai");
    assert!(result.authenticated);
    assert_eq!(result.method, "api_key");
    // secret が status に現れない。
    let status = svc.status(&catalog).unwrap();
    let openai = status
        .providers
        .iter()
        .find(|p| p.provider_id == "openai")
        .unwrap();
    assert!(openai.configured);
    assert_eq!(openai.methods, vec!["api_key"]);
    let cred = status
        .credentials
        .iter()
        .find(|c| c.provider_id == "openai")
        .unwrap();
    assert_eq!(cred.credential_type, "api_key");
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.contains("sk-secret"), "secret must not leak: {json}");
}

#[test]
fn env_var_counts_as_configured() {
    let dir = tmp_dir();
    let env: Arc<dyn ProviderEnv> = Arc::new(MapEnv(BTreeMap::from([(
        "OPENAI_API_KEY".to_string(),
        "sk-env".to_string(),
    )])));
    let svc = service(dir.path(), env);
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    let info = svc.describe_provider(&catalog, "openai").unwrap();
    assert!(info.configured);
    // anthropic は未設定。oauth 宣言があるため methods は
    // api_key+oauth(declaration ベース — flow 登録と無関係)。
    let anthropic = svc.describe_provider(&catalog, "anthropic").unwrap();
    assert!(!anthropic.configured);
    assert_eq!(anthropic.methods, vec!["api_key", "oauth"]);
}

/// TS `auth.ts` L56-66 — `methods`/`subscription` は `auth.oauth`
/// declaration のみで決まり、登録済み flow の有無に関与しない。
/// flow 登録は `login --type oauth` の実行可否のみを制御する。
#[test]
fn oauth_declaration_is_advertised_without_flow() {
    let dir = tmp_dir();
    let svc = service(dir.path(), Arc::new(MapEnv(BTreeMap::new())));
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    // github-copilot は oauth 宣言あり・flow 未登録 → methods は
    // api_key+oauth、subscription は declaration 値(true)。
    let info = svc.describe_provider(&catalog, "github-copilot").unwrap();
    assert_eq!(info.methods, vec!["api_key", "oauth"]);
    assert!(info.subscription);
    // openai-codex は oauth-only → methods は oauth のみ。
    let codex = svc.describe_provider(&catalog, "openai-codex").unwrap();
    assert_eq!(codex.methods, vec!["oauth"]);
    // advertise されても oauth login の実行は flow 登録が必要 —
    // 未登録 provider への oauth login は INVALID_REQUEST。
    let mut interaction = ScriptedInteraction { answers: vec![] };
    let err = svc
        .login(
            &catalog,
            "github-copilot",
            AuthType::Oauth,
            &mut interaction,
        )
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
    assert!(err.message().contains("does not support oauth login"));
}

#[test]
fn login_rejects_unsupported_method() {
    let dir = tmp_dir();
    let svc = service(dir.path(), Arc::new(MapEnv(BTreeMap::new())));
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    let mut interaction = ScriptedInteraction { answers: vec![] };
    let err = svc
        .login(&catalog, "openai", AuthType::Oauth, &mut interaction)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
    assert!(err.message().contains("does not support oauth login"));
    // openai-codex (oauth-only) に api_key login。
    let err = svc
        .login(&catalog, "openai-codex", AuthType::ApiKey, &mut interaction)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
    // 未知 provider。
    let err = svc
        .login(&catalog, "nope", AuthType::ApiKey, &mut interaction)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidRequest);
    assert!(err.message().contains("unknown provider"));
}

#[test]
fn logout_removes_credential_and_is_idempotent() {
    let dir = tmp_dir();
    let svc = service(dir.path(), Arc::new(MapEnv(BTreeMap::new())));
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    svc.credentials()
        .set("openai", StoredCredential::api_key("k".into()))
        .unwrap();
    let result = svc.logout(&catalog, "openai").unwrap();
    assert!(result.logged_out);
    assert!(svc.credentials().get("openai").unwrap().is_none());
    // 未登録 credential でも冪等。
    let result = svc.logout(&catalog, "openai").unwrap();
    assert!(result.logged_out);
}

#[test]
fn logout_all_clears_credentials() {
    let dir = tmp_dir();
    let svc = service(dir.path(), Arc::new(MapEnv(BTreeMap::new())));
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    svc.credentials()
        .set("openai", StoredCredential::api_key("k".into()))
        .unwrap();
    svc.credentials()
        .set("anthropic", StoredCredential::api_key("k".into()))
        .unwrap();
    let results = svc.logout_all(&catalog).unwrap();
    assert_eq!(results.len(), 2);
    assert!(svc.credentials().all().unwrap().is_empty());
}

#[test]
fn get_auth_resolves_stored_credential_then_env() {
    let dir = tmp_dir();
    let env: Arc<dyn ProviderEnv> = Arc::new(MapEnv(BTreeMap::from([(
        "OPENAI_API_KEY".to_string(),
        "sk-env".to_string(),
    )])));
    let svc = service(dir.path(), env);
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    let entry = catalog.provider("openai").unwrap();
    // env fallback。
    let auth = svc.get_auth(entry, &BTreeMap::new()).unwrap().unwrap();
    assert_eq!(auth.api_key.as_deref(), Some("sk-env"));
    // stored credential wins。
    svc.credentials()
        .set("openai", StoredCredential::api_key("sk-stored".into()))
        .unwrap();
    let auth = svc.get_auth(entry, &BTreeMap::new()).unwrap().unwrap();
    assert_eq!(auth.api_key.as_deref(), Some("sk-stored"));
}

#[test]
fn get_auth_resolves_configured_api_key_template() {
    let dir = tmp_dir();
    let env: Arc<dyn ProviderEnv> = Arc::new(MapEnv(BTreeMap::from([(
        "MY_KEY".to_string(),
        "resolved".to_string(),
    )])));
    let svc = service(dir.path(), env);
    let file = cheaper_runtime::catalog::schema::parse_models_json_str(
        r#"{"providers": {"custom": {
            "api": "openai-completions", "baseUrl": "http://x",
            "apiKey": "$MY_KEY", "models": [{"id": "m"}]
        }}}"#,
    )
    .unwrap();
    let catalog = CatalogStore::build(Some(file), None, &[], &BTreeMap::new()).unwrap();
    let entry = catalog.provider("custom").unwrap();
    let auth = svc.get_auth(entry, &BTreeMap::new()).unwrap().unwrap();
    assert_eq!(auth.api_key.as_deref(), Some("resolved"));
}

#[test]
fn get_auth_missing_env_is_explicit_error() {
    let dir = tmp_dir();
    let svc = service(dir.path(), Arc::new(MapEnv(BTreeMap::new())));
    let file = cheaper_runtime::catalog::schema::parse_models_json_str(
        r#"{"providers": {"custom": {
            "api": "openai-completions", "baseUrl": "http://x",
            "apiKey": "$NOPE", "models": [{"id": "m"}]
        }}}"#,
    )
    .unwrap();
    let catalog = CatalogStore::build(Some(file), None, &[], &BTreeMap::new()).unwrap();
    let entry = catalog.provider("custom").unwrap();
    // configured(apiKey template) → check は configured=true でも
    // resolve は明示 error（silent fallback ではない）。
    let err = svc.get_auth(entry, &BTreeMap::new()).unwrap_err();
    assert!(err.message().contains("NOPE"));
}

#[test]
fn oauth_credential_without_flow_does_not_fall_back_to_env() {
    let dir = tmp_dir();
    let env: Arc<dyn ProviderEnv> = Arc::new(MapEnv(BTreeMap::from([(
        "COPILOT_GITHUB_TOKEN".to_string(),
        "ghp_x".to_string(),
    )])));
    let svc = service(dir.path(), env);
    svc.credentials()
        .set(
            "github-copilot",
            StoredCredential::oauth(CredentialData::Oauth {
                access: "a".into(),
                refresh: "r".into(),
                expires: i64::MAX,
                account_id: None,
                subscription: None,
                available_model_ids: None,
                extra: serde_json::Map::new(),
            }),
        )
        .unwrap();
    let catalog = CatalogStore::build(None, None, &[], &BTreeMap::new()).unwrap();
    let entry = catalog.provider("github-copilot").unwrap();
    // oauth handler 未登録 → env fallback しない（None）。
    assert!(svc.get_auth(entry, &BTreeMap::new()).unwrap().is_none());
    // check は configured（stored oauth + oauth 宣言あり）。
    assert!(svc
        .check_provider_auth(
            entry,
            svc.credentials().get("github-copilot").unwrap().as_ref()
        )
        .unwrap());
}
