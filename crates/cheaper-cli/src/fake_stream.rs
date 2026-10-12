//! `CHEAPER_TEST_FAKE_STREAM` seam(TS `cli.ts::loadTestStreamFn` 相当)。
//!
//! TS 版は module を動的 import して `default export` を streamFn に
//! 変換するが、Rust 版は **fixture JSON path** 形式を取る:
//! `CHEAPER_TEST_FAKE_STREAM` が指す JSON file を
//! [`cheaper_runtime::dev::fixture::RunFixture`] として load し、
//! fake provider / scripted session を組み立てる素材として run 経路
//! (T3)へ渡す。
//!
//! activation 規則(TS と同一):
//! - `NODE_ENV === "test"` でないと無効
//! - env 未設定・空文字なら無効
//!
//! fixture load 失敗は INVALID_REQUEST として呼び出し側へ伝播する
//! (TS で module import / shape 検証が throw する経路に相当)。

use cheaper_core::config::dirs::EnvLookup;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_runtime::auth::{AuthType, ProcessEnv, ProviderEnv};
use cheaper_runtime::dev::fixture::{RunFixture, ScriptedAuthInteraction};
use cheaper_runtime::plugin::CheaperProviderPlugin;
use cheaper_runtime::Runtime;
use serde_json::{json, Map};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// `loadTestStreamFn` 相当。有効なら load 済み fixture を返す。
/// `None` = fake stream 無効(通常経路)。
pub fn load_test_stream(env: EnvLookup<'_>) -> Result<Option<RunFixture>, CheaperError> {
    if env("NODE_ENV").as_deref() != Some("test") {
        return Ok(None);
    }
    let spec = env("CHEAPER_TEST_FAKE_STREAM").unwrap_or_default();
    if spec.is_empty() {
        return Ok(None);
    }
    let fixture = RunFixture::load(Path::new(&spec)).map_err(|e| {
        CheaperError::new(
            ErrorCode::InvalidRequest,
            format!(
                "CHEAPER_TEST_FAKE_STREAM fixture load failed: {}",
                e.message()
            ),
        )
        .with_details(Map::from_iter([("spec".to_string(), json!(spec))]))
        .with_cause(e)
    })?;
    Ok(Some(fixture))
}

// ---------------------------------------------------------------------------
// fixture plumbing — compose.rs の `create_runtime`/`session_factory`
// が呼ぶ dev-seam 専用 seam(load 済み fixture → env/plugin/auth)
// ---------------------------------------------------------------------------

/// `ProviderEnv` — base は `ProcessEnv` で、fake-stream fixture の
/// `env` map を overlay として先に引く。
///
/// `io.env`(EnvLookup closure)は `'static` でないため `ProviderEnv`
/// trait object には載せられない — production の `io.env` は
/// `process.env` そのものなので `ProcessEnv` base で等価。fixture が
/// 宣言する env(`$VAR` template 解決用)は overlay で供給する。
struct ScopedProviderEnv {
    overlay: BTreeMap<String, String>,
}

impl ProviderEnv for ScopedProviderEnv {
    fn env(&self, name: &str) -> Option<String> {
        self.overlay
            .get(name)
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| ProcessEnv.env(name))
    }
    fn file_exists(&self, path: &str) -> bool {
        ProcessEnv.file_exists(path)
    }
    fn home_dir(&self) -> Option<PathBuf> {
        ProcessEnv.home_dir()
    }
}

/// fixture `env` map overlay 付き `ProviderEnv`。fixture 無しでも
/// 空 overlay の `ScopedProviderEnv`(process env と等価)を返す —
/// feature on ビルドで経路を分けない。
pub(crate) fn fixture_env(fixture: Option<&RunFixture>) -> Arc<dyn ProviderEnv> {
    Arc::new(ScopedProviderEnv {
        overlay: fixture.map(|f| f.env.clone()).unwrap_or_default(),
    })
}

/// fixture が宣言する `cheaper-fake` plugin(`RunFixture::plugin`)。
pub(crate) fn fixture_plugin(fixture: &RunFixture) -> Arc<dyn CheaperProviderPlugin> {
    Arc::new(fixture.plugin())
}

/// fixture `auth_logins` → `AuthService::login(api_key)` で credential
/// を永続化する(replay harness と同じ経路)。
pub(crate) fn apply_fixture_auth_logins(
    runtime: &Runtime,
    fixture: &RunFixture,
) -> Result<(), CheaperError> {
    for (provider_id, secret) in &fixture.auth_logins {
        let mut interaction = ScriptedAuthInteraction::new(secret.clone());
        runtime.auth().login(
            runtime.catalog(),
            provider_id,
            AuthType::ApiKey,
            &mut interaction,
        )?;
    }
    Ok(())
}
