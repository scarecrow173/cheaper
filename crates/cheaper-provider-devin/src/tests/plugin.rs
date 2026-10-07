use std::sync::Arc;

use cheaper_runtime::auth::{CredentialData, ProcessEnv, StoredCredential};
use cheaper_runtime::plugin::{ProviderPluginContext, ProviderRegistry};
use cheaper_runtime::CheaperProviderPlugin;

use crate::cli::{CliRunOptions, DevinCliResult, DevinCliRunner};
use crate::plugin::{DevinProviderPlugin, PROVIDER_ID};
use crate::rig_provider::{API_KIND, DEFAULT_HOST};

fn ctx(dir: &std::path::Path) -> ProviderPluginContext {
    ProviderPluginContext {
        user_config_dir: dir.to_path_buf(),
        env: Arc::new(ProcessEnv),
    }
}

#[test]
fn registers_devin_provider_shape() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = ProviderRegistry::default();
    let plugin = DevinProviderPlugin::new();
    assert_eq!(plugin.id(), PROVIDER_ID);
    plugin.register(&mut registry, &ctx(dir.path()));
    let registrations = registry.into_registrations();
    assert_eq!(registrations.len(), 1);
    let reg = &registrations[0];
    assert_eq!(reg.id, "devin");
    assert_eq!(reg.name, "Devin Local");
    assert_eq!(reg.api, API_KIND);
    assert_eq!(reg.base_url, DEFAULT_HOST);
    assert!(reg.subscription);
    assert!(reg.auth.is_some());
    assert!(reg.catalog.is_some());
    // bootstrap fallback models。
    assert!(!reg.models.is_empty());
    assert!(reg.models.iter().all(|m| m.api.is_none()));
}

#[test]
fn to_auth_reads_isolated_credentials_toml() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = crate::credentials::devin_data_dir(dir.path());
    let cred_path = crate::credentials::devin_credentials_path(&data_dir);
    std::fs::create_dir_all(cred_path.parent().unwrap()).unwrap();
    std::fs::write(
        &cred_path,
        "windsurf_api_key = \"toml-key\"\napi_server_url = \"https://alt.example\"\n",
    )
    .unwrap();
    let mut registry = ProviderRegistry::default();
    DevinProviderPlugin::new().register(&mut registry, &ctx(dir.path()));
    let registrations = registry.into_registrations();
    let auth = registrations[0].auth.clone().unwrap();
    let ctx = ctx(dir.path());
    let stored = StoredCredential::oauth(CredentialData::Oauth {
        access: "stored-key".to_string(),
        refresh: String::new(),
        expires: i64::MAX,
        account_id: None,
        subscription: None,
        available_model_ids: None,
        extra: Default::default(),
    });
    // credentials.toml 優先（rotated token 拾い）。
    let resolved = auth.to_auth(&ctx, &stored).unwrap();
    assert_eq!(resolved.api_key.as_deref(), Some("toml-key"));
    assert_eq!(resolved.base_url.as_deref(), Some("https://alt.example"));
    // toml が無ければ stored access へ fallback。
    std::fs::remove_file(&cred_path).unwrap();
    let resolved = auth.to_auth(&ctx, &stored).unwrap();
    assert_eq!(resolved.api_key.as_deref(), Some("stored-key"));
    assert_eq!(resolved.base_url, None);
}

/// 呼ばれたら記録する runner — check が subprocess を spawn しない
/// ことの検証に使う。`run` が呼ばれても NotFound を返す（CLI 未到達
/// 相当）。
struct RecordingRunner {
    calls: std::sync::Mutex<Vec<Vec<String>>>,
}

impl DevinCliRunner for RecordingRunner {
    fn run(
        &self,
        _bin: &str,
        args: &[String],
        _opts: &CliRunOptions<'_>,
    ) -> Result<DevinCliResult, std::io::Error> {
        self.calls.lock().unwrap().push(args.to_vec());
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "fake"))
    }
}

fn stored() -> StoredCredential {
    StoredCredential::oauth(CredentialData::Oauth {
        access: "stored-key".to_string(),
        refresh: String::new(),
        expires: i64::MAX,
        account_id: None,
        subscription: None,
        available_model_ids: None,
        extra: Default::default(),
    })
}

#[test]
fn check_is_credential_presence_without_cli_subprocess() {
    // TS `plugin.ts` は `oauth.check` を持たない — Pi は credential
    // 存在を `configured: true` とみなし、`devin auth status`
    // subprocess は走らない（CLI 未到達でも stored credential が
    // あれば configured）。
    let dir = tempfile::tempdir().unwrap();
    let runner = Arc::new(RecordingRunner {
        calls: std::sync::Mutex::new(Vec::new()),
    });
    let mut registry = ProviderRegistry::default();
    DevinProviderPlugin::with_runner(runner.clone()).register(&mut registry, &ctx(dir.path()));
    let registrations = registry.into_registrations();
    let auth = registrations[0].auth.clone().unwrap();
    let ctx = ctx(dir.path());
    assert!(!auth.check(&ctx, None).unwrap());
    assert!(auth.check(&ctx, Some(&stored())).unwrap());
    assert!(runner.calls.lock().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn register_survives_non_utf8_process_env() {
    // Unix では任意 byte 列の env var が合法的に存在しうる —
    // `std::env::vars()` は非 UTF-8 で panic するため `vars_os()` +
    // lossy 変換が必須（Runtime::create → register が起動経路）。
    use std::os::unix::ffi::OsStrExt;
    let key = std::ffi::OsStr::from_bytes(b"CHEAPER_DEVIN_TEST_NONUTF8_\xff");
    let value = std::ffi::OsStr::from_bytes(b"\xff\xfe");
    std::env::set_var(key, value);
    let dir = tempfile::tempdir().unwrap();
    let mut registry = ProviderRegistry::default();
    DevinProviderPlugin::new().register(&mut registry, &ctx(dir.path()));
    std::env::remove_var(key);
    assert_eq!(registry.into_registrations().len(), 1);
}

#[test]
fn catalog_refresh_uses_fallback_without_credential() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = ProviderRegistry::default();
    DevinProviderPlugin::new().register(&mut registry, &ctx(dir.path()));
    let registrations = registry.into_registrations();
    let catalog = registrations[0].catalog.clone().unwrap();
    let ctx = ctx(dir.path());
    let models = catalog.refresh_models(&ctx, None).unwrap().unwrap();
    assert_eq!(models, crate::models::fallback_models());
}
