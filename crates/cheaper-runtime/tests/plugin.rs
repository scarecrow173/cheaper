//! plugin contract seam test — fake provider で registration/catalog/
//! auth/logout の contract が compile・動作することを検証する。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cheaper_core::errors::CheaperError;
use cheaper_runtime::auth::api_key::{AuthSource, ProviderEnv, ResolvedAuth};
use cheaper_runtime::auth::service::AuthInteraction;
use cheaper_runtime::auth::store::{CredentialData, StoredCredential};
use cheaper_runtime::catalog::schema::ModelDefinitionJson;
use cheaper_runtime::plugin::{
    CheaperProviderPlugin, PluginAuth, PluginCatalog, PluginProviderRegistration,
    ProviderPluginContext, ProviderRegistry,
};
use cheaper_runtime::Runtime;

struct MapEnv;
impl ProviderEnv for MapEnv {
    fn env(&self, _name: &str) -> Option<String> {
        None
    }
    fn file_exists(&self, _path: &str) -> bool {
        false
    }
    fn home_dir(&self) -> Option<PathBuf> {
        None
    }
}

struct FakeAuth {
    logged_out: AtomicBool,
}
impl PluginAuth for FakeAuth {
    fn login(
        &self,
        _ctx: &ProviderPluginContext,
        _interaction: &mut dyn AuthInteraction,
    ) -> Result<StoredCredential, CheaperError> {
        let mut extra = serde_json::Map::new();
        extra.insert("fake".to_string(), serde_json::json!(true));
        Ok(StoredCredential::oauth(CredentialData::Oauth {
            access: "fake-token".into(),
            refresh: "".into(),
            expires: i64::MAX,
            account_id: None,
            subscription: Some("team".into()),
            available_model_ids: None,
            extra,
        }))
    }
    fn to_auth(
        &self,
        _ctx: &ProviderPluginContext,
        credential: &StoredCredential,
    ) -> Result<ResolvedAuth, CheaperError> {
        match &credential.data {
            CredentialData::Oauth { access, .. } => Ok(ResolvedAuth {
                api_key: Some(access.clone()),
                headers: BTreeMap::new(),
                base_url: None,
                source: AuthSource::OAuth,
            }),
            _ => Err(CheaperError::new(
                cheaper_core::errors::ErrorCode::InternalError,
                "expected oauth credential",
            )),
        }
    }
    fn logout(&self, _ctx: &ProviderPluginContext) -> Result<(), CheaperError> {
        self.logged_out.store(true, Ordering::SeqCst);
        Ok(())
    }
}

struct FakeCatalog {
    refreshed: Mutex<Option<Vec<ModelDefinitionJson>>>,
    /// refresh 失敗注入 — `Err(CheaperError)` を返す(F13 失敗経路)。
    fail: AtomicBool,
}
impl PluginCatalog for FakeCatalog {
    fn refresh_models(
        &self,
        _ctx: &ProviderPluginContext,
        _credential: Option<&StoredCredential>,
    ) -> Result<Option<Vec<ModelDefinitionJson>>, CheaperError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(CheaperError::new(
                cheaper_core::errors::ErrorCode::InternalError,
                "refresh boom",
            ));
        }
        Ok(self.refreshed.lock().unwrap().clone())
    }
}

struct FakePlugin {
    auth: Arc<FakeAuth>,
    catalog: Arc<FakeCatalog>,
}
impl CheaperProviderPlugin for FakePlugin {
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
            models: vec![ModelDefinitionJson {
                id: "fake-1".to_string(),
                name: None,
                api: None,
                base_url: None,
                reasoning: None,
                thinking_level_map: None,
                input: None,
                cost: None,
                context_window: None,
                max_tokens: None,
                sampling_params: None,
                headers: None,
                compat: None,
            }],
            auth: Some(self.auth.clone()),
            subscription: true,
            catalog: Some(self.catalog.clone()),
        });
    }
}

#[test]
fn plugin_provider_composes_and_supports_its_api_kind() {
    let dir = tempfile::tempdir().unwrap();
    let plugin = Arc::new(FakePlugin {
        auth: Arc::new(FakeAuth {
            logged_out: AtomicBool::new(false),
        }),
        catalog: Arc::new(FakeCatalog {
            refreshed: Mutex::new(None),
            fail: AtomicBool::new(false),
        }),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = Runtime::create(
        dir.path().to_path_buf(),
        Arc::new(MapEnv),
        &plugins,
        BTreeMap::new(),
    )
    .unwrap();
    // plugin provider は catalog に現れ、宣言 api kind は supported 扱い。
    let entry = runtime.catalog().provider("fake").unwrap();
    assert_eq!(entry.name, "Fake Provider");
    let model = runtime
        .catalog()
        .get_supported_model("fake", "fake-1")
        .unwrap()
        .unwrap();
    assert_eq!(model.api, "fake-api");
    assert_eq!(model.provider, "fake");
    // status: plugin oauth auth → methods=["oauth"]、subscription=true。
    let info = runtime
        .auth()
        .describe_provider(runtime.catalog(), "fake")
        .unwrap();
    assert_eq!(info.methods, vec!["oauth"]);
    assert!(info.subscription);
    assert!(!info.configured);
}

#[test]
fn plugin_login_logout_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let auth = Arc::new(FakeAuth {
        logged_out: AtomicBool::new(false),
    });
    let plugin = Arc::new(FakePlugin {
        auth: auth.clone(),
        catalog: Arc::new(FakeCatalog {
            refreshed: Mutex::new(None),
            fail: AtomicBool::new(false),
        }),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = Runtime::create(
        dir.path().to_path_buf(),
        Arc::new(MapEnv),
        &plugins,
        BTreeMap::new(),
    )
    .unwrap();
    struct Noop;
    impl AuthInteraction for Noop {
        fn prompt(
            &mut self,
            _p: &cheaper_runtime::auth::AuthPrompt<'_>,
        ) -> Result<String, CheaperError> {
            Ok(String::new())
        }
    }
    let result = runtime
        .auth()
        .login(
            runtime.catalog(),
            "fake",
            cheaper_runtime::auth::AuthType::Oauth,
            &mut Noop,
        )
        .unwrap();
    assert!(result.authenticated);
    assert_eq!(result.method, "oauth");
    assert!(result.subscription);
    // credential 永続化 → configured → get_auth が plugin to_auth を呼ぶ。
    let status = runtime.auth().status(runtime.catalog()).unwrap();
    let fake = status
        .providers
        .iter()
        .find(|p| p.provider_id == "fake")
        .unwrap();
    assert!(fake.configured);
    let entry = runtime.catalog().provider("fake").unwrap();
    let resolved = runtime
        .auth()
        .get_auth(entry, &BTreeMap::new())
        .unwrap()
        .unwrap();
    assert_eq!(resolved.api_key.as_deref(), Some("fake-token"));
    // logout → plugin hook 起動 + credential 削除。
    runtime.auth().logout(runtime.catalog(), "fake").unwrap();
    assert!(auth.logged_out.load(Ordering::SeqCst));
    assert!(
        !runtime
            .auth()
            .describe_provider(runtime.catalog(), "fake")
            .unwrap()
            .configured
    );
}

// ---------------------------------------------------------------------------
// F13 — login 後の plugin catalog refresh
// ---------------------------------------------------------------------------

struct NoopInteraction;
impl AuthInteraction for NoopInteraction {
    fn prompt(
        &mut self,
        _p: &cheaper_runtime::auth::AuthPrompt<'_>,
    ) -> Result<String, CheaperError> {
        Ok(String::new())
    }
}

fn refreshed_model(id: &str) -> ModelDefinitionJson {
    ModelDefinitionJson {
        id: id.to_string(),
        name: None,
        api: None,
        base_url: None,
        reasoning: None,
        thinking_level_map: None,
        input: None,
        cost: None,
        context_window: None,
        max_tokens: None,
        sampling_params: None,
        headers: None,
        compat: None,
    }
}

fn plugin_with_catalog(catalog: FakeCatalog) -> (Arc<FakeAuth>, Arc<dyn CheaperProviderPlugin>) {
    let auth = Arc::new(FakeAuth {
        logged_out: AtomicBool::new(false),
    });
    (
        auth.clone(),
        Arc::new(FakePlugin {
            auth,
            catalog: Arc::new(catalog),
        }),
    )
}

#[test]
fn login_refreshes_plugin_catalog_models() {
    // F13: login 成功後に `refreshModels` 結果で live model catalog が
    // 更新される(devin `devin models list` 経路の seam)。
    let dir = tempfile::tempdir().unwrap();
    let (_auth, plugin) = plugin_with_catalog(FakeCatalog {
        refreshed: Mutex::new(Some(vec![refreshed_model("fake-2")])),
        fail: AtomicBool::new(false),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = Runtime::create(
        dir.path().to_path_buf(),
        Arc::new(MapEnv),
        &plugins,
        BTreeMap::new(),
    )
    .unwrap();
    // login 前は bootstrap list のみ。
    assert!(runtime
        .catalog()
        .get_supported_model("fake", "fake-2")
        .unwrap()
        .is_none());
    let result = runtime
        .auth()
        .login(
            runtime.catalog(),
            "fake",
            cheaper_runtime::auth::AuthType::Oauth,
            &mut NoopInteraction,
        )
        .unwrap();
    assert!(result.authenticated);
    // refresh 結果が catalog に反映される。
    let model = runtime
        .catalog()
        .get_supported_model("fake", "fake-2")
        .unwrap()
        .expect("refreshed model is visible");
    assert_eq!(model.provider, "fake");
    // 宣言 api kind は plugin api に継承される。
    assert_eq!(model.api, "fake-api");
    assert!(runtime.catalog().errors().is_empty());
}

#[test]
fn login_succeeds_when_plugin_refresh_fails() {
    // F13: refresh の失敗は login を失敗させない — TS の
    // `refreshModels().catch(() => {})` 相当で error は catalog errors
    // へ記録されるのみ。
    let dir = tempfile::tempdir().unwrap();
    let (_auth, plugin) = plugin_with_catalog(FakeCatalog {
        refreshed: Mutex::new(None),
        fail: AtomicBool::new(true),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = Runtime::create(
        dir.path().to_path_buf(),
        Arc::new(MapEnv),
        &plugins,
        BTreeMap::new(),
    )
    .unwrap();
    let result = runtime
        .auth()
        .login(
            runtime.catalog(),
            "fake",
            cheaper_runtime::auth::AuthType::Oauth,
            &mut NoopInteraction,
        )
        .unwrap();
    assert!(result.authenticated);
    let errors = runtime.catalog().errors();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("fake") && e.contains("refreshModels")),
        "{errors:?}"
    );
    // bootstrap list は維持される。
    assert!(runtime
        .catalog()
        .get_supported_model("fake", "fake-1")
        .unwrap()
        .is_some());
}
