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

/// `register()` が emit する `base_url`/`models` を可変にできる fake
/// plugin。`new` は既定値（`base_url: "http://fake.local"`・
/// `models: [fake-1]`）、`with_registration` で空 `base_url`/空
/// `models`/任意 defs を仕込める（user `models[]` merge の fallback
/// 規則・persist guard・変換不能 defs の検証用）。
struct FakePlugin {
    auth: Arc<FakeAuth>,
    catalog: Arc<FakeCatalog>,
    base_url: String,
    models: Vec<ModelDefinitionJson>,
}
impl FakePlugin {
    fn new(auth: Arc<FakeAuth>, catalog: Arc<FakeCatalog>) -> Self {
        Self::with_registration(
            auth,
            catalog,
            "http://fake.local",
            vec![refreshed_model("fake-1")],
        )
    }
    fn with_registration(
        auth: Arc<FakeAuth>,
        catalog: Arc<FakeCatalog>,
        base_url: &str,
        models: Vec<ModelDefinitionJson>,
    ) -> Self {
        Self {
            auth,
            catalog,
            base_url: base_url.to_string(),
            models,
        }
    }
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
            base_url: self.base_url.clone(),
            api_key: None,
            auth_header: None,
            headers: None,
            models: self.models.clone(),
            auth: Some(self.auth.clone()),
            subscription: true,
            catalog: Some(self.catalog.clone()),
        });
    }
}

#[test]
fn plugin_provider_composes_and_supports_its_api_kind() {
    let dir = tempfile::tempdir().unwrap();
    let plugin = Arc::new(FakePlugin::new(
        Arc::new(FakeAuth {
            logged_out: AtomicBool::new(false),
        }),
        Arc::new(FakeCatalog {
            refreshed: Mutex::new(None),
            fail: AtomicBool::new(false),
        }),
    ));
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
    let plugin = Arc::new(FakePlugin::new(
        auth.clone(),
        Arc::new(FakeCatalog {
            refreshed: Mutex::new(None),
            fail: AtomicBool::new(false),
        }),
    ));
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
    plugin_with(
        catalog,
        "http://fake.local",
        vec![refreshed_model("fake-1")],
    )
}

/// catalog seam + registration `base_url`/`models` を可変にした fake
/// plugin（空 `base_url`・空 `models` 等の registration 検証用）。
fn plugin_with(
    catalog: FakeCatalog,
    base_url: &str,
    models: Vec<ModelDefinitionJson>,
) -> (Arc<FakeAuth>, Arc<dyn CheaperProviderPlugin>) {
    let auth = Arc::new(FakeAuth {
        logged_out: AtomicBool::new(false),
    });
    (
        auth.clone(),
        Arc::new(FakePlugin::with_registration(
            auth,
            Arc::new(catalog),
            base_url,
            models,
        )),
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

// ---------------------------------------------------------------------------
// catalog snapshot 永続化(issue #81 — login で取得した live catalog が
// `<userConfigDir>/providers/<id>/catalog.json` 経由で別 process の
// `Runtime::create` に届く) + user `models[]` additive merge(issue #82)
// ---------------------------------------------------------------------------

/// `<userConfigDir>/providers/fake/catalog.json` の path。
fn catalog_path(dir: &tempfile::TempDir) -> PathBuf {
    dir.path()
        .join("providers")
        .join("fake")
        .join("catalog.json")
}

fn new_runtime(dir: &tempfile::TempDir, plugins: &[Arc<dyn CheaperProviderPlugin>]) -> Runtime {
    Runtime::create(
        dir.path().to_path_buf(),
        Arc::new(MapEnv),
        plugins,
        BTreeMap::new(),
    )
    .unwrap()
}

fn login_oauth(runtime: &Runtime) {
    runtime
        .auth()
        .login(
            runtime.catalog(),
            "fake",
            cheaper_runtime::auth::AuthType::Oauth,
            &mut NoopInteraction,
        )
        .unwrap();
}

fn write_models_json(dir: &tempfile::TempDir, body: &str) {
    std::fs::write(dir.path().join("models.json"), body).unwrap();
}

fn no_refresh_catalog() -> FakeCatalog {
    FakeCatalog {
        refreshed: Mutex::new(None),
        fail: AtomicBool::new(false),
    }
}

#[test]
fn login_persists_catalog_snapshot_for_later_runtime() {
    // spec AC(issue #81 の根幹): login で refresh した live catalog が
    // catalog.json 経由で別 process(新規 `Runtime::create`)の catalog
    // 構築に届く。
    let dir = tempfile::tempdir().unwrap();
    let (_auth, plugin) = plugin_with_catalog(FakeCatalog {
        refreshed: Mutex::new(Some(vec![refreshed_model("fake-2")])),
        fail: AtomicBool::new(false),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);
    login_oauth(&runtime);

    // `<userConfigDir>/providers/fake/catalog.json` が version 1
    // envelope で書かれる。
    let path = catalog_path(&dir);
    assert!(path.exists(), "catalog.json not written");
    let envelope: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(envelope["version"], serde_json::json!(1));
    assert_eq!(envelope["provider"], serde_json::json!("fake"));
    assert_eq!(envelope["models"][0]["id"], serde_json::json!("fake-2"));

    // 別 process 相当の新規 `Runtime::create` が snapshot を注入する。
    let runtime2 = new_runtime(&dir, &plugins);
    let model = runtime2
        .catalog()
        .get_supported_model("fake", "fake-2")
        .unwrap()
        .expect("persisted model must be injected into a fresh runtime");
    assert_eq!(model.api, "fake-api");
    // credential も永続済みのため get_available にも現れる。
    let available = runtime2.get_available(Some("fake")).unwrap();
    assert!(available.iter().any(|m| m.id == "fake-2"), "{available:?}");
    assert!(
        runtime2.catalog().errors().is_empty(),
        "{:?}",
        runtime2.catalog().errors()
    );
}

#[test]
fn login_with_bootstrap_equal_refresh_does_not_persist() {
    // persist guard (c): refresh が bootstrap `registration.models` と
    // 等価な defs を返した場合は snapshot を書かない — 既存 file も
    // 上書きしない(bootstrap fallback 応答で正常 catalog を破壊しない)。
    let dir = tempfile::tempdir().unwrap();
    let path = catalog_path(&dir);
    // sentinel: bootstrap と異なる有効 snapshot(独自 baseUrl を持つ def
    // で compose が失敗しない形)。
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"version":1,"provider":"fake","fetchedAt":1,"models":[{"id":"sentinel-1","baseUrl":"http://sentinel.local"}]}"#,
    )
    .unwrap();
    let (_auth, plugin) = plugin_with_catalog(FakeCatalog {
        // bootstrap models([fake-1])と等価な defs を返す。
        refreshed: Mutex::new(Some(vec![refreshed_model("fake-1")])),
        fail: AtomicBool::new(false),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);
    login_oauth(&runtime);

    // sentinel file は上書きされず、guard skip は error を記録しない。
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains("sentinel-1"), "{body}");
    assert!(
        runtime.catalog().errors().is_empty(),
        "{:?}",
        runtime.catalog().errors()
    );
}

#[test]
fn login_with_empty_refresh_does_not_persist() {
    // persist guard (b): `Ok(Some([]))` → write skip + `record_error`
    // (空 list の file は load 検証で必ず失敗する自己矛盾のため書かない)。
    let dir = tempfile::tempdir().unwrap();
    let (_auth, plugin) = plugin_with_catalog(FakeCatalog {
        refreshed: Mutex::new(Some(vec![])),
        fail: AtomicBool::new(false),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);
    login_oauth(&runtime);

    assert!(!catalog_path(&dir).exists());
    let errors = runtime.catalog().errors();
    assert!(
        errors.iter().any(|e| e.contains("empty model list")),
        "{errors:?}"
    );
}

#[test]
fn unconvertible_refresh_defs_are_not_persisted() {
    // `extension_models` 変換に失敗する defs(空 base_url plugin +
    // baseUrl 無し def)は persist しない — 変換不能 snapshot は次回
    // boot の compose を同じく失敗させ、login 経由の自己修復を不可能に
    // するため。既存 file も上書きしない。
    let dir = tempfile::tempdir().unwrap();
    let path = catalog_path(&dir);
    // 既存 file 温存の sentinel として malformed file を置く(有効な
    // snapshot は provider model となり、defaults 経由で refresh 変換が
    // 救済されるためここでは読めない file が適切)。
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{ not json").unwrap();

    let (_auth, plugin) = plugin_with(
        FakeCatalog {
            refreshed: Mutex::new(Some(vec![refreshed_model("live-1")])),
            fail: AtomicBool::new(false),
        },
        "",     // plugin base_url 空
        vec![], // bootstrap models 空 → entry.models 空(defaults 無し)
    );
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);
    // malformed snapshot は load error として記録され bootstrap へ
    // fallbackする。
    assert!(runtime
        .catalog()
        .errors()
        .iter()
        .any(|e| e.contains("plugin catalog")));
    login_oauth(&runtime);
    // `refresh_plugin_models` Err → `record_error` のみ、persist しない。
    let errors = runtime.catalog().errors();
    assert!(errors.iter().any(|e| e.contains("baseUrl")), "{errors:?}");
    // malformed file は上書きされない。
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
}

#[test]
fn refresh_does_not_borrow_base_url_from_merged_user_defs() {
    // `refresh_plugin_models` の `extension_models` defaults pool は
    // compose 時の pre-extension base(plugin-only provider では空)と
    // 同一でなければならない。現在の resolved list(merge 済み user def
    // `u1` を含む)を渡すと `r1` が `u1` の baseUrl を継承して変換できて
    // しまい、その defs が persist されると次回 boot compose が同じ
    // defs で失敗し provider ごと drop される(persist gate が防ぐべき
    // 自己矛盾 snapshot)。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"u1","baseUrl":"http://user.local"}]}}}"#,
    );
    let (_auth, plugin) = plugin_with(
        FakeCatalog {
            refreshed: Mutex::new(Some(vec![refreshed_model("r1")])),
            fail: AtomicBool::new(false),
        },
        "",     // plugin base_url 空
        vec![], // bootstrap models 空 → compose 時 extension base も空
    );
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    // compose: `u1` は user 指定 baseUrl で merge される — refresh が
    // 借りてはいけない defaults 候補が entry.models に居座る状態を作る。
    assert_eq!(
        runtime.catalog().get_model("fake", "u1").unwrap().base_url,
        "http://user.local"
    );

    // login → refresh: `r1` は baseUrl 解決不能で refresh 全体が skip
    // され error 記録 — persist は呼ばれず catalog.json は書かれない。
    login_oauth(&runtime);
    assert!(runtime.catalog().get_model("fake", "r1").is_none());
    // compose 済みの `u1` は維持される。
    assert!(runtime.catalog().get_model("fake", "u1").is_some());
    let errors = runtime.catalog().errors();
    assert!(errors.iter().any(|e| e.contains("baseUrl")), "{errors:?}");
    assert!(
        !catalog_path(&dir).exists(),
        "defs that only converted via merged-user-def defaults must not be persisted"
    );
}

#[test]
fn persisted_def_headers_reach_model_headers_at_boot() {
    // persisted snapshot 由来の `headers` は compose 時の
    // `collect_model_header_sources`(plugin_defs = persisted defs)経由で
    // `model_headers` に届く — refresh 経路だけでなく boot compose でも
    // header source が構築されることの直接検証。
    let dir = tempfile::tempdir().unwrap();
    let path = catalog_path(&dir);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"version":1,"provider":"fake","fetchedAt":1,"models":[{"id":"persisted-1","headers":{"X-Persisted":"yes"}}]}"#,
    )
    .unwrap();
    let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    let model = runtime
        .catalog()
        .get_model("fake", "persisted-1")
        .expect("persisted model must be injected at boot");
    assert_eq!(model.api, "fake-api");
    let headers = runtime
        .catalog()
        .model_headers("fake", "persisted-1")
        .expect("persisted def headers must be registered at compose");
    assert_eq!(headers["X-Persisted"], serde_json::json!("yes"));
}

#[test]
fn logout_retains_catalog_snapshot_and_later_runtime_still_injects() {
    // logout は catalog.json を削除しない(stale snapshot は次回 login
    // refresh で自己修復 — spec freshness AC)。logout 後の新規
    // `Runtime::create` も同じ snapshot を注入し続ける。
    let dir = tempfile::tempdir().unwrap();
    let (_auth, plugin) = plugin_with_catalog(FakeCatalog {
        refreshed: Mutex::new(Some(vec![refreshed_model("fake-2")])),
        fail: AtomicBool::new(false),
    });
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);
    login_oauth(&runtime);
    let path = catalog_path(&dir);
    assert!(path.exists());

    runtime.auth().logout(runtime.catalog(), "fake").unwrap();
    // file は残り、別 process 相当の runtime も persisted model を見る。
    assert!(path.exists());
    let runtime2 = new_runtime(&dir, &plugins);
    assert!(runtime2
        .catalog()
        .get_supported_model("fake", "fake-2")
        .unwrap()
        .is_some());
}

#[test]
fn invalid_catalog_snapshot_is_ignored_with_recorded_error() {
    // malformed / version 不一致 / provider 不一致 / 空 `models` →
    // file 全体を無視して bootstrap `plugin.models` へ fallback +
    // `Provider "<id>": <msg>` 形式で error 記録。
    for (name, body) in [
        ("malformed", "{ not json".to_string()),
        (
            "version mismatch",
            serde_json::json!({
                "version": 2,
                "provider": "fake",
                "fetchedAt": 1,
                "models": [{"id": "x"}]
            })
            .to_string(),
        ),
        (
            "provider mismatch",
            serde_json::json!({
                "version": 1,
                "provider": "other",
                "fetchedAt": 1,
                "models": [{"id": "x"}]
            })
            .to_string(),
        ),
        (
            "empty models",
            serde_json::json!({
                "version": 1,
                "provider": "fake",
                "fetchedAt": 1,
                "models": []
            })
            .to_string(),
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = catalog_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &body).unwrap();
        let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
        let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
        let runtime = new_runtime(&dir, &plugins);
        let errors = runtime.catalog().errors();
        assert!(
            errors.iter().any(|e| e.starts_with("Provider \"fake\":")),
            "{name}: {errors:?}"
        );
        // bootstrap fallback — fake-1 が残る。
        assert!(
            runtime
                .catalog()
                .get_supported_model("fake", "fake-1")
                .unwrap()
                .is_some(),
            "{name}"
        );
    }
}

// ---------------------------------------------------------------------------
// user `models[]` additive merge(issue #82 — plugin provider で user def
// が silent discard されない)
// ---------------------------------------------------------------------------

#[test]
fn user_model_def_merges_with_plugin_fallback_values() {
    // `providers.fake.models[]` の plugin 未定義 id は plugin 宣言値
    // (`api`/`baseUrl`)fallback で解決され catalog に残る。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"user-1"}]}}}"#,
    );
    let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    let model = runtime
        .catalog()
        .get_supported_model("fake", "user-1")
        .unwrap()
        .expect("user model def must merge");
    assert_eq!(model.api, "fake-api");
    assert_eq!(model.base_url, "http://fake.local");
    // plugin 定義順 → user 追加順で末尾に置かれる。
    let ids: Vec<_> = runtime
        .catalog()
        .provider("fake")
        .unwrap()
        .models
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    assert_eq!(ids, ["fake-1", "user-1"]);
    assert!(
        runtime.catalog().errors().is_empty(),
        "{:?}",
        runtime.catalog().errors()
    );
}

#[test]
fn user_model_def_colliding_with_plugin_id_is_skipped_with_error() {
    // post-extension list の id と衝突する user def は skip + error 記録
    // — list 側(plugin 定義)が authoritative で shadow されない。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"fake-1","baseUrl":"http://user.local","contextWindow":7777}]}}}"#,
    );
    let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    // plugin 側 def が維持される(user def の baseUrl/contextWindow は
    // 反映されない)。
    let model = runtime.catalog().get_model("fake", "fake-1").unwrap();
    assert_eq!(model.base_url, "http://fake.local");
    assert_ne!(model.context_window, 7777.0);
    let errors = runtime.catalog().errors();
    assert!(
        errors.iter().any(|e| e.contains("Provider \"fake\"")
            && e.contains("fake-1")
            && e.contains("conflict")),
        "{errors:?}"
    );
    // provider・plugin model は存続する。
    assert!(runtime.catalog().provider("fake").is_some());
}

#[test]
fn invalid_user_model_def_is_skipped_without_dropping_provider() {
    // `model_from_json` 失敗(contextWindow <= 0)はその def のみ skip +
    // error 記録 — provider・plugin models は残る(custom provider の
    // fatal drop とは異なる plugin 経路の規則)。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"bad","contextWindow":-1},{"id":"user-ok"}]}}}"#,
    );
    let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    assert!(runtime.catalog().provider("fake").is_some());
    assert!(runtime.catalog().get_model("fake", "bad").is_none());
    assert!(runtime.catalog().get_model("fake", "fake-1").is_some());
    // 同じ file の valid def は merge される。
    assert!(runtime.catalog().get_model("fake", "user-ok").is_some());
    let errors = runtime.catalog().errors();
    assert!(
        errors.iter().any(|e| e.contains("invalid contextWindow")),
        "{errors:?}"
    );
}

#[test]
fn user_model_def_merges_when_plugin_models_empty() {
    // `plugin.models` が空でも同一規則で merge が走る(extension 適用の
    // 有無に依らない)。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"user-1"}]}}}"#,
    );
    let (_auth, plugin) = plugin_with(no_refresh_catalog(), "http://fake.local", vec![]);
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    let model = runtime
        .catalog()
        .get_supported_model("fake", "user-1")
        .unwrap()
        .expect("user def must merge even with empty plugin models");
    assert_eq!(model.api, "fake-api");
    assert_eq!(model.base_url, "http://fake.local");
    assert!(
        runtime.catalog().errors().is_empty(),
        "{:?}",
        runtime.catalog().errors()
    );
}

#[test]
fn user_model_def_without_base_url_is_skipped_on_compose_and_refresh() {
    // plugin `base_url: ""` + baseUrl 無し user def — fallback chain
    // (def → config → plugin → defaults)が全て尽きるため compose では
    // def skip + error 記録。refresh(login → refresh_plugin_catalog)
    // 経路でも refreshed defs が変換不能なら `refresh_plugin_models`
    // Err → `record_error` され、空 base_url の model は catalog に
    // 残らない(helper 内非空 filter が両経路で同一に効くことの検証)。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"user-1"}]}}}"#,
    );
    let (_auth, plugin) = plugin_with(
        FakeCatalog {
            // refreshed def も baseUrl 解決不能 — refresh 経路でも
            // user def は merge されない。
            refreshed: Mutex::new(Some(vec![refreshed_model("r-1")])),
            fail: AtomicBool::new(false),
        },
        "",     // plugin base_url 空
        vec![], // bootstrap models 空
    );
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    // compose 経路: def skip + error、provider は残る。
    assert!(runtime.catalog().provider("fake").is_some());
    assert!(runtime.catalog().get_model("fake", "user-1").is_none());
    let errors = runtime.catalog().errors();
    assert!(errors.iter().any(|e| e.contains("baseUrl")), "{errors:?}");

    // refresh 経路: `refresh_plugin_models` が Err → `record_error`。
    login_oauth(&runtime);
    let errors = runtime.catalog().errors();
    assert!(
        errors.iter().filter(|e| e.contains("baseUrl")).count() >= 2,
        "{errors:?}"
    );
    assert!(runtime.catalog().get_model("fake", "user-1").is_none());
    assert!(runtime.catalog().get_model("fake", "r-1").is_none());
    assert!(runtime
        .catalog()
        .provider("fake")
        .unwrap()
        .models
        .iter()
        .all(|m| !m.base_url.is_empty()));
}

#[test]
fn refresh_re_evaluates_user_def_collisions_both_directions() {
    // 衝突判定は refresh ごとに再評価される: plugin catalog から消えた
    // id の user def は effective になり、復活した id・新規追加された
    // id では list 側が優先 + error 記録される。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"fake-1","baseUrl":"http://user.local"},{"id":"user-1"}]}}}"#,
    );
    let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    // compose: `fake-1` は plugin def と衝突 → plugin 側優先 + error、
    // `user-1` は merge される。
    let model = runtime.catalog().get_model("fake", "fake-1").unwrap();
    assert_eq!(model.base_url, "http://fake.local");
    assert_eq!(
        runtime
            .catalog()
            .get_model("fake", "user-1")
            .unwrap()
            .base_url,
        "http://fake.local"
    );
    let errors = runtime.catalog().errors();
    assert_eq!(
        errors.iter().filter(|e| e.contains("conflict")).count(),
        1,
        "{errors:?}"
    );

    // refresh #1: plugin catalog が `fake-1` を落とす → user def の
    // `fake-1` が effective になり、`user-1` も保持される。
    runtime
        .catalog()
        .refresh_plugin_models("fake", &[refreshed_model("fake-2")])
        .unwrap();
    assert_eq!(
        runtime
            .catalog()
            .get_model("fake", "fake-1")
            .unwrap()
            .base_url,
        "http://user.local"
    );
    assert!(runtime.catalog().get_model("fake", "fake-2").is_some());
    assert!(runtime.catalog().get_model("fake", "user-1").is_some());

    // refresh #2: plugin catalog が `fake-1` を復活させ、さらに
    // `user-1` と同名の新 id を追加 → 両 user def が衝突 skip +
    // error 記録で list 側が優先される。
    let mut plugin_user_1 = refreshed_model("user-1");
    plugin_user_1.base_url = Some("http://plugin.local".to_string());
    runtime
        .catalog()
        .refresh_plugin_models("fake", &[refreshed_model("fake-1"), plugin_user_1])
        .unwrap();
    assert_eq!(
        runtime
            .catalog()
            .get_model("fake", "fake-1")
            .unwrap()
            .base_url,
        "http://fake.local"
    );
    assert_eq!(
        runtime
            .catalog()
            .get_model("fake", "user-1")
            .unwrap()
            .base_url,
        "http://plugin.local"
    );
    let errors = runtime.catalog().errors();
    assert!(
        errors.iter().filter(|e| e.contains("conflict")).count() >= 3,
        "{errors:?}"
    );
}

#[test]
fn refresh_rebuilds_model_header_sources() {
    // refreshed defs の `headers` は `model_header_sources` 再構築を経て
    // `CatalogStore::model_headers` に届く。
    let dir = tempfile::tempdir().unwrap();
    let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    let mut def = refreshed_model("fake-2");
    def.headers = Some(serde_json::Map::from_iter([(
        "X-Test".to_string(),
        serde_json::Value::from("yes"),
    )]));
    runtime
        .catalog()
        .refresh_plugin_models("fake", &[def])
        .unwrap();
    let headers = runtime
        .catalog()
        .model_headers("fake", "fake-2")
        .expect("refreshed model headers must be registered");
    assert_eq!(headers["X-Test"], serde_json::json!("yes"));
}

#[test]
fn collision_skipped_user_def_headers_do_not_leak_on_compose_or_refresh() {
    // id 衝突で skip された user def の `headers` は残存 plugin model の
    // `model_headers` に混入しない — skip は def の rejection であり、
    // headers だけが漏れるのは不完全な reject。compose と refresh の
    // 両経路で検証する。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(
        &dir,
        r#"{"providers":{"fake":{"models":[{"id":"fake-1","headers":{"X-User":"rejected"}}]}}}"#,
    );
    let mut bootstrap = refreshed_model("fake-1");
    bootstrap.headers = Some(serde_json::Map::from_iter([(
        "X-Plugin".to_string(),
        serde_json::Value::from("bootstrap"),
    )]));
    let (_auth, plugin) = plugin_with(no_refresh_catalog(), "http://fake.local", vec![bootstrap]);
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);

    // compose: user def は衝突 skip + error 記録され、その headers は
    // 残存 plugin model に混入しない。
    let errors = runtime.catalog().errors();
    assert!(errors.iter().any(|e| e.contains("conflict")), "{errors:?}");
    let headers = runtime
        .catalog()
        .model_headers("fake", "fake-1")
        .expect("plugin def headers must be registered");
    assert_eq!(headers["X-Plugin"], serde_json::json!("bootstrap"));
    assert!(!headers.contains_key("X-User"), "{headers:?}");

    // refresh: 同じ衝突が再評価されても skip 済み user def の headers
    // は残存 plugin model に混入しない。
    let mut refreshed = refreshed_model("fake-1");
    refreshed.headers = Some(serde_json::Map::from_iter([(
        "X-Plugin".to_string(),
        serde_json::Value::from("refreshed"),
    )]));
    runtime
        .catalog()
        .refresh_plugin_models("fake", &[refreshed])
        .unwrap();
    let headers = runtime
        .catalog()
        .model_headers("fake", "fake-1")
        .expect("refreshed def headers must be registered");
    assert_eq!(headers["X-Plugin"], serde_json::json!("refreshed"));
    assert!(!headers.contains_key("X-User"), "{headers:?}");
}

#[test]
fn plugin_provider_oauth_requires_base_url_like_custom() {
    // plugin provider の provider-level 検証は custom と同一 semantics
    // — `{"oauth":"radius"}`(baseUrl 無し)は検証 Err → provider drop +
    // error 記録(緩めない)。
    let dir = tempfile::tempdir().unwrap();
    write_models_json(&dir, r#"{"providers":{"fake":{"oauth":"radius"}}}"#);
    let (_auth, plugin) = plugin_with_catalog(no_refresh_catalog());
    let plugins: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin];
    let runtime = new_runtime(&dir, &plugins);
    assert!(runtime.catalog().provider("fake").is_none());
    let errors = runtime.catalog().errors();
    assert!(
        errors
            .iter()
            .any(|e| e.contains("\"fake\"") && e.contains("baseUrl") && e.contains("oauth")),
        "{errors:?}"
    );

    // `{"oauth":"radius","baseUrl":…}` は従来どおり radius env auth
    // semantics のまま検証を通過する。
    let dir2 = tempfile::tempdir().unwrap();
    write_models_json(
        &dir2,
        r#"{"providers":{"fake":{"oauth":"radius","baseUrl":"http://radius.local"}}}"#,
    );
    let (_auth2, plugin2) = plugin_with_catalog(no_refresh_catalog());
    let plugins2: Vec<Arc<dyn CheaperProviderPlugin>> = vec![plugin2];
    let runtime2 = new_runtime(&dir2, &plugins2);
    assert!(runtime2.catalog().provider("fake").is_some());
    assert!(runtime2.catalog().get_model("fake", "fake-1").is_some());
}
