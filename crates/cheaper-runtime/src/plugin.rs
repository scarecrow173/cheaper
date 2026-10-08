//! Provider plugin contract（G5）。
//!
//! TS `CheaperProviderPlugin`（`packages/runtime-pi/src/provider-plugin.ts`）
//! の Rust port。runtime 側が contract を所有し、devin・fake 等の provider
//! plugin は composition root が登録する。Rig type は一切含まない
//! （runtime → rig 依存禁止）。
//!
//! plugin が登録できる要素（TS extension provider 対応表）:
//!
//! | TS extension field   | `PluginProviderRegistration` field |
//! |----------------------|-------------------------------------|
//! | `id`/`name`/`baseUrl`| `id`/`name`/`base_url`              |
//! | `api`                | `api`（plugin 宣言 api kind）        |
//! | `apiKey`             | `api_key`（template/`!` 解釈）       |
//! | `authHeader`         | `auth_header`                       |
//! | `headers`            | `headers`                           |
//! | `models`             | `models`（bootstrap catalog）       |
//! | `oauth`              | `auth: PluginAuth` + `subscription` |
//! | `refreshModels`      | `catalog: PluginCatalog`            |
//! | `streamSimple`       | （T5 の adapter seam、runtime 外）  |

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use cheaper_core::errors::CheaperError;
use serde_json::{Map, Value};

use crate::auth::api_key::{ProviderEnv, ResolvedAuth};
use crate::auth::service::AuthInteraction;
use crate::auth::store::StoredCredential;
use crate::catalog::schema::ModelDefinitionJson;

/// `ProviderPluginContext` — plugin へ渡す runtime context。
/// `env` は `authContext` と同じ accessor（scoped env + ambient）。
#[derive(Clone)]
pub struct ProviderPluginContext {
    /// `<userConfigDir>`（`resolve_user_config_dir` 済み。plugin 固有 file は
    /// `<userConfigDir>/providers/<id>/` 以下に閉じ込める）。
    pub user_config_dir: PathBuf,
    /// process env accessor（plugin が CLI 委譲等に使う）。
    pub env: Arc<dyn ProviderEnv>,
}

impl ProviderPluginContext {
    /// plugin 固有の data dir（`<userConfigDir>/providers/<id>/` — devin の
    /// `<userConfigDir>/providers/devin/data/` 隔離 parent）。
    pub fn plugin_dir(&self, plugin_id: &str) -> PathBuf {
        self.user_config_dir.join("providers").join(plugin_id)
    }
}

/// extension の `oauth` 相当（plugin credential/login/logout hook）。
/// credential 永続化は runtime の `CredentialStore` が行い、plugin は
/// credential の生成・外部 state 管理・wire auth 変換のみ担う。
pub trait PluginAuth: Send + Sync {
    /// `auth login <provider> --method oauth` — plugin が対話 login を
    /// 実行し永続化する credential を返す（devin は `devin login` 委譲 +
    /// toml 読み出しで oauth-typed credential を返す）。`interaction` は
    /// prompt/notify を提供するが login を対話へ委譲してよい。
    fn login(
        &self,
        ctx: &ProviderPluginContext,
        interaction: &mut dyn AuthInteraction,
    ) -> Result<StoredCredential, CheaperError>;

    /// `oauth.toAuth` 相当 — stored credential → request 用 auth。
    /// refresh が必要な場合は flow 内部で処理し、新 credential は
    /// `refresh` 経由で service が先に呼ぶ。
    fn to_auth(
        &self,
        ctx: &ProviderPluginContext,
        credential: &StoredCredential,
    ) -> Result<ResolvedAuth, CheaperError>;

    /// credential の有効性 check（`checkProviderAuth` の oauth 分岐）。
    /// default は credential 存在のみ（TS と同じ）。
    fn check(
        &self,
        _ctx: &ProviderPluginContext,
        credential: Option<&StoredCredential>,
    ) -> Result<bool, CheaperError> {
        Ok(credential.is_some())
    }

    /// oauth refresh 相当 — expiring credential の更新。None = refresh
    /// 不可（service は既存 credential を維持する）。
    fn refresh(
        &self,
        _ctx: &ProviderPluginContext,
        _credential: &StoredCredential,
    ) -> Result<Option<StoredCredential>, CheaperError> {
        Ok(None)
    }

    /// `auth logout` 時の外部 state 片付け（devin `devin logout`）。
    /// default は no-op。
    fn logout(&self, _ctx: &ProviderPluginContext) -> Result<(), CheaperError> {
        Ok(())
    }
}

/// live catalog refresh seam（extension `refreshModels` 相当）。
/// devin は `devin models list` 委譲で返す。
pub trait PluginCatalog: Send + Sync {
    /// login/credential 変化後の catalog refresh。`Ok(None)` = 更新なし
    /// （bootstrap list 維持）。error は service が catalog errors へ記録し
    /// silent fallback しない（`get_model` には影響しない）。
    fn refresh_models(
        &self,
        ctx: &ProviderPluginContext,
        credential: Option<&StoredCredential>,
    ) -> Result<Option<Vec<ModelDefinitionJson>>, CheaperError>;
}

/// plugin が登録する provider 定義（extension provider registration の
/// materialized 形）。
pub struct PluginProviderRegistration {
    pub id: String,
    /// display name（`ProviderAuthInfo.name` / provider display）。
    pub name: String,
    /// extension `api` — plugin 宣言 api kind（adapter が dispatch する。
    /// catalog の `SUPPORTED_APIS` に含まれなくても plugin api は
    /// supported 扱い — devin は独自 api kind を宣言する）。
    pub api: String,
    /// extension `baseUrl`。
    pub base_url: String,
    /// extension `apiKey`（models.json `apiKey` と同じ template 解釈）。
    pub api_key: Option<String>,
    /// extension `authHeader`。
    pub auth_header: Option<bool>,
    /// extension `headers`（template 値を含みうる）。
    pub headers: Option<Map<String, Value>>,
    /// bootstrap model list（extension `models`。空 = baseUrl override のみ）。
    pub models: Vec<ModelDefinitionJson>,
    /// extension `oauth` 相当の auth flow。
    pub auth: Option<Arc<dyn PluginAuth>>,
    /// `oauth.isSubscription`（`ProviderAuthInfo.subscription`）。
    pub subscription: bool,
    /// `refreshModels` 相当。
    pub catalog: Option<Arc<dyn PluginCatalog>>,
}

/// plugin register の受け口（`ProviderRegistry` — TS の runtime 登録
/// surface を narrow 化したもの）。
#[derive(Default)]
pub struct ProviderRegistry {
    registrations: Vec<PluginProviderRegistration>,
    /// providerId → logout hook（provider 登録と別経路で hook だけ
    /// 登録したい fake 用。plugin auth.logout と同一経路でもよいが、
    /// fake が catalog/auth を持たない provider に hook を足す用途を許す）。
    logout_hooks: BTreeMap<String, Arc<dyn PluginAuth>>,
}

impl ProviderRegistry {
    /// provider を登録する。同一 id の再登録は上書き（TS Set 順序の
    /// plugin 後勝ちには該当しない — provider id dedup は先勝ち。後で
    /// 同 id を登録しても先の registration が残る）。
    pub fn register_provider(&mut self, registration: PluginProviderRegistration) {
        self.registrations.push(registration);
    }
    /// logout hook のみ追加登録（fake/test 用の最小 seam）。
    pub fn register_logout_hook(&mut self, provider_id: &str, hook: Arc<dyn PluginAuth>) {
        self.logout_hooks.insert(provider_id.to_string(), hook);
    }
    /// materialize — `CatalogStore::build` と `AuthService` へ渡す。
    pub fn into_registrations(self) -> Vec<PluginProviderRegistration> {
        self.registrations
    }
    pub fn logout_hooks(&self) -> &BTreeMap<String, Arc<dyn PluginAuth>> {
        &self.logout_hooks
    }
}

/// `CheaperProviderPlugin` — composition root が runtime へ登録する
/// plugin contract。`id` は provider id と一致させる慣例（TS と同じ）。
pub trait CheaperProviderPlugin: Send + Sync {
    fn id(&self) -> &str;
    /// runtime 構築時に一度だけ呼ばれる。`registry` へ provider/logout
    /// hook を登録する（TS `register(runtime, context)` の port）。
    fn register(&self, registry: &mut ProviderRegistry, context: &ProviderPluginContext);
}
