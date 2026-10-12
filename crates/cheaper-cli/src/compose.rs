//! Composition root — `Runtime::create` + bundled plugin 登録の単一
//! 入口。command 実装は `run.rs`/`auth.rs`/`usage_report.rs` 等に
//! 残し、runtime 構築の配線はここに集約する。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use cheaper_core::errors::CheaperError;
use cheaper_provider::devin::DevinProviderPlugin;
use cheaper_runtime::auth::{OAuthFlow, ProcessEnv, ProviderEnv};
use cheaper_runtime::plugin::CheaperProviderPlugin;
use cheaper_runtime::Runtime;

/// `bundledProviderPlugins()` 相当 — `DevinProviderPlugin` を常時
/// bundle する(Pi extension loader は使わない)。
pub(crate) fn bundled_provider_plugins() -> Vec<Arc<dyn CheaperProviderPlugin>> {
    vec![Arc::new(DevinProviderPlugin::new())]
}

/// `Runtime::create` の単一入口。builtin oauth flow seed は
/// `Runtime::create` 内部で行われる(TS `createModelRuntime` 相当)。
/// `env` は通常 `process_env()`、fake-stream fixture 経路のみ
/// `ScopedProviderEnv` overlay を渡す。
pub(crate) fn create_runtime(
    user_config_dir: PathBuf,
    env: Arc<dyn ProviderEnv>,
    plugins: &[Arc<dyn CheaperProviderPlugin>],
) -> Result<Runtime, CheaperError> {
    Runtime::create(
        user_config_dir,
        env,
        plugins,
        BTreeMap::<String, Arc<dyn OAuthFlow>>::new(),
    )
}

/// 素の process env(`Arc<dyn ProviderEnv>` 固定の newtype)。
pub(crate) fn process_env() -> Arc<dyn ProviderEnv> {
    Arc::new(ProcessEnv)
}
