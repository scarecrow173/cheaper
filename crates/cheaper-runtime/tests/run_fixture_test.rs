//! T4 fixture replay 統合テスト — `tests/fixtures/*.json` を load し、
//! `run_worker` を end-to-end で実行して `expect` 節と比較する。
//!
//! 3 層 seam の検証(G7):
//! - catalog 層: replay 毎に `get_available` が fixture model を露出する
//!   ことを確認する(unsupported api fixture でも configured provider の
//!   model は列挙される — api check は `get_supported_model` 側)。
//! - keyless credential 層: `provider.api_key` template(literal/`$VAR`)
//!   + `env` overlay、`auth_logins` の `AuthService::login` 永続化経路。
//! - scripted stream 層: `sessions[].steps` → `ScriptStep` replay。
//!
//! normalize 規則(設計書): `expect` 宣言 field のみ比較するため
//! `meta.durationMs`・`meta.execution.*Ms`・`error.message` 本文・
//! stderr・JSON key 順・schema error message 文言は除外され、error
//! code/exit code/usage token field/turns/tool calls/rejected tool
//! calls/transient retries/changes/savings 形/pricing 由来 `costUsd`
//! は厳密に比較される。

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use cheaper_core::errors::CheaperError;
use cheaper_core::model::config::ModelConfig;
use cheaper_runtime::fake_provider::CheaperFakePlugin;
use cheaper_runtime::fixture::{
    ExpectedStatus, FixtureMetaExpect, RunFixture, ScriptedAuthInteraction,
};
use cheaper_runtime::plugin::CheaperProviderPlugin;
use cheaper_runtime::run::{run_worker, RunWorkerDeps, RunWorkerOk};
use cheaper_runtime::run_error::WorkerRunError;
use cheaper_runtime::{AuthType, Runtime};
use cheaper_tools::operation::{AbortController, AbortReason};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// replay
// ---------------------------------------------------------------------------

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

struct ReplayOutput {
    dir: tempfile::TempDir,
    result: Result<RunWorkerOk, WorkerRunError>,
    pending_responses: usize,
    catalog_models: Vec<String>,
}

/// fixture 1 件を `run_worker` まで replay する。
async fn replay(fixture: &RunFixture) -> ReplayOutput {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("ws");
    std::fs::create_dir_all(&root).unwrap();
    let user_config = dir.path().join("config");
    std::fs::create_dir_all(&user_config).unwrap();

    // --- catalog + keyless credential 層 ------------------------------------
    let plugin: Arc<dyn CheaperProviderPlugin> = Arc::new(fixture.plugin());
    let runtime = Runtime::create(
        user_config,
        Arc::new(fixture.env()),
        &[plugin],
        BTreeMap::new(),
    )
    .unwrap();

    // `auth_logins` — AuthService::login(api_key)で credential を永続化。
    for (provider, secret) in &fixture.auth_logins {
        let mut interaction = ScriptedAuthInteraction::new(secret.clone());
        runtime
            .auth()
            .login(
                runtime.catalog(),
                provider,
                AuthType::ApiKey,
                &mut interaction,
            )
            .unwrap();
    }

    // catalog 層 — fixture model が get_available に露出していること。
    let catalog_models: Vec<String> = runtime
        .get_available(None)
        .unwrap()
        .iter()
        .map(|m| format!("{}/{}", m.provider, m.id))
        .collect();

    // --- worker file(fixture name = file stem)-------------------------------
    let worker_path = root.join(format!("{}.md", fixture.worker.name));
    std::fs::write(&worker_path, fixture.worker_markdown()).unwrap();
    let worker = fixture.worker_definition(&worker_path).unwrap();
    let policy = fixture.policy();
    let factory = fixture.session_factory();
    let input = fixture.input.clone();
    let model_config = ModelConfig::default();
    let explicit = fixture.explicit_model();

    // --- signal abort --------------------------------------------------------
    let controller = AbortController::new();
    let signal = controller.signal();
    if let Some(abort) = &fixture.abort {
        let c = controller.clone();
        let after_ms = abort.after_ms;
        let message = abort.message.clone().unwrap_or_else(|| "user abort".into());
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(after_ms)).await;
            c.abort(AbortReason::new("fixture", message, None));
        });
    }

    let deps = RunWorkerDeps {
        worker: &worker,
        input: &input,
        root: root.to_str().unwrap().to_string(),
        policy: &policy,
        model_config: &model_config,
        runtime: &runtime,
        session_factory: &factory,
        explicit_model: Some(&explicit),
        signal: fixture.abort.as_ref().map(|_| &signal),
        abort_grace_ms: Some(fixture.abort_grace_ms.unwrap_or(80.0)),
        execution: None,
        operation_clock: None,
    };
    let result = run_worker(&deps).await;
    ReplayOutput {
        dir,
        result,
        pending_responses: factory.pending_responses(),
        catalog_models,
    }
}

// ---------------------------------------------------------------------------
// normalized 比較 — `expect` 宣言 field のみ
// ---------------------------------------------------------------------------

/// JSON subset 比較(expected の全 key が actual に一致)。
fn assert_json_subset(expected: &Value, actual: &Value, path: &str, ctx: &str) {
    match (expected, actual) {
        (Value::Object(e), Value::Object(a)) => {
            for (k, ev) in e {
                let av = a.get(k).unwrap_or_else(|| {
                    panic!("{ctx}: {path}.{k} missing in actual {a:?}");
                });
                assert_json_subset(ev, av, &format!("{path}.{k}"), ctx);
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            assert_eq!(e.len(), a.len(), "{ctx}: {path} array length");
            for (i, (ev, av)) in e.iter().zip(a.iter()).enumerate() {
                assert_json_subset(ev, av, &format!("{path}[{i}]"), ctx);
            }
        }
        (e, a) => assert_eq!(e, a, "{ctx}: {path}"),
    }
}

fn get<'a>(meta: &'a Value, key: &str) -> &'a Value {
    &meta[key]
}

/// `expect.meta` の宣言 field を serialized meta(camelCase JSON)と比較。
/// `meta` が無い run(error で runMeta 未付与)は `None` を渡す。
fn assert_meta(expect: &FixtureMetaExpect, meta: Option<&Value>, ctx: &str) {
    let meta = meta.unwrap_or_else(|| panic!("{ctx}: expected meta fields but run has no meta"));
    if let Some(model) = &expect.model {
        let m = get(meta, "model");
        assert!(m.is_object(), "{ctx}: meta.model missing: {meta}");
        if let Some(p) = &model.provider {
            assert_eq!(m["provider"], json!(p), "{ctx}: meta.model.provider");
        }
        if let Some(id) = &model.id {
            assert_eq!(m["id"], json!(id), "{ctx}: meta.model.id");
        }
        if let Some(t) = &model.tier {
            assert_eq!(m["tier"], json!(t), "{ctx}: meta.model.tier");
        }
    }
    if let Some(usage) = &expect.usage {
        let u = get(meta, "usage");
        assert!(u.is_object(), "{ctx}: meta.usage missing: {meta}");
        let cmp = |key: &str, want: Option<u64>| {
            if let Some(w) = want {
                assert_eq!(u[key], json!(w), "{ctx}: meta.usage.{key}");
            }
        };
        cmp("input", usage.input);
        cmp("output", usage.output);
        cmp("cacheRead", usage.cache_read);
        cmp("cacheWrite", usage.cache_write);
        cmp("totalTokens", usage.total_tokens);
        if let Some(w) = usage.cost_usd {
            // pricing 層算出 costUsd(provider 応答値ではない)を厳密比較。
            let got = u["costUsd"].as_f64().unwrap_or(f64::NAN);
            assert!(
                (got - w).abs() < 1e-9,
                "{ctx}: meta.usage.costUsd expected {w} got {got}"
            );
        }
    }
    let cmp_u64 = |key: &str, want: Option<u64>| {
        if let Some(w) = want {
            assert_eq!(meta[key], json!(w), "{ctx}: meta.{key}");
        }
    };
    cmp_u64("turns", expect.turns);
    cmp_u64("toolCalls", expect.tool_calls);
    cmp_u64("rejectedToolCalls", expect.rejected_tool_calls);
    cmp_u64("repairs", expect.repairs);
    cmp_u64("transientRetries", expect.transient_retries);
    if let Some(w) = expect.changes_files {
        assert_eq!(
            meta["changes"]["totals"]["files"],
            json!(w),
            "{ctx}: meta.changes.totals.files"
        );
    }
    if let Some(w) = expect.changes_files_min {
        let got = meta["changes"]["totals"]["files"].as_u64().unwrap_or(0);
        assert!(
            got >= w,
            "{ctx}: meta.changes.totals.files expected >= {w} got {got}"
        );
    }
    if let Some(savings) = &expect.savings {
        if let Some(w) = savings.directly_comparable {
            assert_eq!(
                meta["savings"]["directlyComparable"],
                json!(w),
                "{ctx}: meta.savings.directlyComparable"
            );
        }
    }
}

/// fixture 1 件の replay + expect 比較。panic message は fixture 名付き。
async fn run_and_check(path: &Path) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let fixture =
        RunFixture::load(path).unwrap_or_else(|e| panic!("fixture {name}: failed to load: {e}"));
    let ctx = fixture.name.clone().unwrap_or_else(|| name.clone());
    let out = replay(&fixture).await;
    let expect = &fixture.expect;

    // catalog 層 — fixture model が get_available に露出。
    let model_id = format!(
        "{}/{}",
        CheaperFakePlugin::PROVIDER_ID,
        fixture
            .model
            .id
            .as_deref()
            .unwrap_or(CheaperFakePlugin::MODEL_ID),
    );
    assert!(
        out.catalog_models.contains(&model_id),
        "{ctx}: catalog get_available missing {model_id}: {:?}",
        out.catalog_models
    );

    match expect.status.unwrap_or(ExpectedStatus::Ok) {
        ExpectedStatus::Ok => {
            let RunWorkerOk { result, meta } = out
                .result
                .map_err(|e| format!("{ctx}: expected ok, got {e:?}"))
                .unwrap();
            if let Some(w) = &expect.result {
                assert_eq!(&Value::Object(result.clone()), w, "{ctx}: result mismatch");
            }
            if let Some(exit) = expect.exit_code {
                assert_eq!(exit, 0, "{ctx}: ok run exit code must be 0");
            }
            if let Some(m) = &expect.meta {
                let meta_json = serde_json::to_value(&meta).unwrap();
                assert_meta(m, Some(&meta_json), &ctx);
            }
        }
        ExpectedStatus::Error => {
            let err = out
                .result
                .err()
                .unwrap_or_else(|| panic!("{ctx}: expected error, run succeeded"));
            if let Some(code) = expect.error_code {
                assert_eq!(
                    err.code(),
                    code,
                    "{ctx}: error code (got {}: {})",
                    err.code(),
                    err.message()
                );
            }
            if let Some(exit) = expect.exit_code {
                assert_eq!(err.exit_code(), exit, "{ctx}: exit code");
            }
            if let Some(details) = &expect.error_details {
                let actual = err
                    .details()
                    .map(|d| Value::Object(d.clone()))
                    .unwrap_or(Value::Null);
                assert_json_subset(&Value::Object(details.clone()), &actual, "details", &ctx);
            }
            if let Some(cause) = expect.cause_code {
                let c = err
                    .error()
                    .source()
                    .and_then(|s| s.downcast_ref::<CheaperError>())
                    .unwrap_or_else(|| panic!("{ctx}: expected cause CheaperError"));
                assert_eq!(c.code(), cause, "{ctx}: cause code");
            }
            if let Some(m) = &expect.meta {
                let meta_json = err.run_meta().map(|rm| serde_json::to_value(rm).unwrap());
                assert_meta(m, meta_json.as_ref(), &ctx);
            }
        }
    }
    if let Some(w) = expect.pending_responses {
        assert_eq!(out.pending_responses, w, "{ctx}: pendingResponses");
    }
    // TempDir を返り値まで保持(drop で掃除)。
    drop(out.dir);
}

// ---------------------------------------------------------------------------
// fixture corpus replay
// ---------------------------------------------------------------------------

/// `tests/fixtures/*.json` を全て replay する。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixtures_replay_all() {
    let mut files: Vec<PathBuf> = std::fs::read_dir(fixture_dir())
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            (p.extension().and_then(|x| x.to_str()) == Some("json")).then_some(p)
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no fixtures found");
    for path in &files {
        run_and_check(path).await;
    }
}

// ---------------------------------------------------------------------------
// layer-specific 追加検証
// ---------------------------------------------------------------------------

/// catalog + keyless credential 層 — `cheaper-fake/fake-model` が
/// `get_available` に露出し keyless(literal api_key)で configured。
#[tokio::test]
async fn fake_plugin_exposes_catalog_and_keyless_auth() {
    let dir = tempfile::tempdir().unwrap();
    let user_config = dir.path().join("config");
    std::fs::create_dir_all(&user_config).unwrap();
    let fixture = RunFixture::default();
    let plugin: Arc<dyn CheaperProviderPlugin> = Arc::new(fixture.plugin());
    let runtime = Runtime::create(
        user_config,
        Arc::new(fixture.env()),
        &[plugin],
        BTreeMap::new(),
    )
    .unwrap();
    let available = runtime.get_available(None).unwrap();
    assert!(available
        .iter()
        .any(|m| m.provider == "cheaper-fake" && m.id == "fake-model"));
    let info = runtime
        .auth()
        .describe_provider(runtime.catalog(), "cheaper-fake")
        .unwrap();
    assert!(info.configured, "keyless fake provider must be configured");
}

/// `auth_logins` の login → stored credential 経路が `get_auth` に反映
/// されることを直接確認する(fixture 20 の層単位検証)。
#[tokio::test]
async fn auth_login_persists_credential_for_run_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let user_config = dir.path().join("config");
    std::fs::create_dir_all(&user_config).unwrap();
    let fixture = RunFixture::default();
    let plugin: Arc<dyn CheaperProviderPlugin> = Arc::new(fixture.plugin());
    let runtime = Runtime::create(
        user_config.clone(),
        Arc::new(fixture.env()),
        &[plugin],
        BTreeMap::new(),
    )
    .unwrap();
    let mut interaction = ScriptedAuthInteraction::new("stored-secret-key");
    let result = runtime
        .auth()
        .login(
            runtime.catalog(),
            "cheaper-fake",
            AuthType::ApiKey,
            &mut interaction,
        )
        .unwrap();
    assert!(result.authenticated);
    // credential は credentials.json に永続化され、status の
    // credentials[] に api_key として現れる。
    let status = runtime.auth().status(runtime.catalog()).unwrap();
    assert!(status
        .credentials
        .iter()
        .any(|c| c.provider_id == "cheaper-fake" && c.credential_type == "api_key"));
    let entry = runtime.catalog().provider("cheaper-fake").unwrap();
    let auth = runtime.auth().get_auth(entry, &BTreeMap::new()).unwrap();
    // stored credential が resolve される(secret は ResolvedAuth 内に
    // 閉じ込め — source label は "stored credential")。
    let auth = auth.expect("stored credential must resolve");
    assert_eq!(auth.api_key.as_deref(), Some("stored-secret-key"));
}
