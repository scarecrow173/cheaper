//! Bundled model catalog snapshot。
//!
//! `include_str!` で pi-ai `dist/providers/data/*.json` を byte 同一のまま
//! embed する（TS が `xxx.models.js` 経由で bundle する canonical snapshot
//! と同じ bytes — `scripts/generate-models.ts` 生成物の dist 出力）。
//! file 名 = provider id、内容は `{<api-kind>: {<modelId>: ModelJson}}` で、
//! `flattenModelCatalog` 相当に api group 順・model 順を保持して flatten
//! する（serde_json `preserve_order` 前提）。
//!
//! network refresh（models.dev / `models-store.json`）は v1 で実装しない
//! （offline 既定。TS の `withRemoteCatalog` wrap は publish する stored
//! models が無い限り bundled 静的 list を維持するため、bundled 読みだけで
//! TS offline 初期 state と一致する）。

use std::collections::BTreeMap;

/// pi-ai `.manifest.json` — `generatedAt` は remote catalog 鮮度管理用。
/// v1 は offline のため値の expose のみ（stale 判定は行わない）。
const MANIFEST_JSON: &str = include_str!("data/.manifest.json");

/// Bundled provider data file の生 bytes。`builtinProviders()` の宣言順。
static BUNDLED_DATA: &[(&str, &str)] = &[
    ("amazon-bedrock", include_str!("data/amazon-bedrock.json")),
    ("ant-ling", include_str!("data/ant-ling.json")),
    ("anthropic", include_str!("data/anthropic.json")),
    (
        "azure-openai-responses",
        include_str!("data/azure-openai-responses.json"),
    ),
    ("baseten", include_str!("data/baseten.json")),
    ("cerebras", include_str!("data/cerebras.json")),
    (
        "cloudflare-ai-gateway",
        include_str!("data/cloudflare-ai-gateway.json"),
    ),
    (
        "cloudflare-workers-ai",
        include_str!("data/cloudflare-workers-ai.json"),
    ),
    ("deepseek", include_str!("data/deepseek.json")),
    ("fireworks", include_str!("data/fireworks.json")),
    ("github-copilot", include_str!("data/github-copilot.json")),
    ("google", include_str!("data/google.json")),
    ("google-vertex", include_str!("data/google-vertex.json")),
    ("groq", include_str!("data/groq.json")),
    ("huggingface", include_str!("data/huggingface.json")),
    ("kimi-coding", include_str!("data/kimi-coding.json")),
    ("minimax", include_str!("data/minimax.json")),
    ("minimax-cn", include_str!("data/minimax-cn.json")),
    ("mistral", include_str!("data/mistral.json")),
    ("moonshotai", include_str!("data/moonshotai.json")),
    ("moonshotai-cn", include_str!("data/moonshotai-cn.json")),
    ("nvidia", include_str!("data/nvidia.json")),
    ("openai", include_str!("data/openai.json")),
    ("openai-codex", include_str!("data/openai-codex.json")),
    ("opencode", include_str!("data/opencode.json")),
    ("opencode-go", include_str!("data/opencode-go.json")),
    ("openrouter", include_str!("data/openrouter.json")),
    ("qwen-token-plan", include_str!("data/qwen-token-plan.json")),
    (
        "qwen-token-plan-cn",
        include_str!("data/qwen-token-plan-cn.json"),
    ),
    (
        "qwen-token-plan-individual",
        include_str!("data/qwen-token-plan-individual.json"),
    ),
    ("together", include_str!("data/together.json")),
    (
        "vercel-ai-gateway",
        include_str!("data/vercel-ai-gateway.json"),
    ),
    ("xai", include_str!("data/xai.json")),
    ("xiaomi", include_str!("data/xiaomi.json")),
    (
        "xiaomi-token-plan-ams",
        include_str!("data/xiaomi-token-plan-ams.json"),
    ),
    (
        "xiaomi-token-plan-cn",
        include_str!("data/xiaomi-token-plan-cn.json"),
    ),
    (
        "xiaomi-token-plan-sgp",
        include_str!("data/xiaomi-token-plan-sgp.json"),
    ),
    ("zai", include_str!("data/zai.json")),
    ("zai-coding-cn", include_str!("data/zai-coding-cn.json")),
];

/// Bundled data file を provider id で引く（`radius` のみ remote-only で
/// data file を持たない → None）。
pub fn bundled_provider_json(provider_id: &str) -> Option<&'static str> {
    BUNDLED_DATA
        .iter()
        .find(|(id, _)| *id == provider_id)
        .map(|(_, json)| *json)
}

/// `getBuiltinModelDataGeneratedAt` 相当。manifest の `generatedAt` を
/// epoch ms で返す（parse 不能なら None、TS は `Number.isNaN` → undefined）。
pub fn bundled_generated_at_ms() -> Option<f64> {
    let v: serde_json::Value = serde_json::from_str(MANIFEST_JSON).ok()?;
    let s = v.get("generatedAt")?.as_str()?;
    // ISO8601 → epoch ms。外部 dep を増やさないため自前 parse（形式は
    // `YYYY-MM-DDTHH:MM:SS.sssZ` 固定）。
    parse_iso8601_ms(s)
}

fn parse_iso8601_ms(s: &str) -> Option<f64> {
    // `2026-09-05T11:58:56.761Z`
    let (date, time) = s.split_once('T')?;
    let time = time.strip_suffix('Z')?;
    let mut dparts = date.split('-');
    let y: i64 = dparts.next()?.parse().ok()?;
    let mo: i64 = dparts.next()?.parse().ok()?;
    let d: i64 = dparts.next()?.parse().ok()?;
    let mut tparts = time.split(':');
    let h: i64 = tparts.next()?.parse().ok()?;
    let mi: i64 = tparts.next()?.parse().ok()?;
    let sec_part = tparts.next()?;
    let (sec, frac) = match sec_part.split_once('.') {
        Some((s2, f)) => (s2.parse::<i64>().ok()?, {
            let mut f = f.to_string();
            f.truncate(3);
            while f.len() < 3 {
                f.push('0');
            }
            f.parse::<i64>().unwrap_or(0)
        }),
        None => (sec_part.parse::<i64>().ok()?, 0),
    };
    Some(
        days_from_civil(y, mo, d) as f64 * 86_400_000.0
            + (h * 3600 + mi * 60 + sec) as f64 * 1000.0
            + frac as f64,
    )
}

/// days-from-civil（Howard Hinnant の式）。`pub(crate)` — oauth の
/// `Retry-After` HTTP-date parse でも使う（T6）。
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// builtin provider の環境変数 auth 種別（pi-ai `env-api-keys.ts` の
/// provider 毎特殊解決を忠実に分岐するため enum で表す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvAuth {
    /// `envApiKeyAuth` — 最初にセットされた env var が api key。
    Vars(&'static [&'static str]),
    /// anthropic: `ANTHROPIC_AUTH_TOKEN`（Bearer header）→
    /// `ANTHROPIC_OAUTH_TOKEN` / `ANTHROPIC_API_KEY`（api key）の順。
    Anthropic,
    /// google-vertex: `GOOGLE_CLOUD_API_KEY` または ADC
    /// （credentials file + project + location）。
    GoogleVertex,
    /// amazon-bedrock: AWS profile / IAM pair / bearer / container creds /
    /// web identity のいずれか（値は不透明、"<authenticated>" 扱い）。
    AmazonBedrock,
    /// 環境変数由来の auth を持たない（openai-codex 等）。
    None,
}

/// builtin provider の OAuth 宣言（`auth.oauth` の静的 metadata）。
/// `methods`/`subscription` への反映は declaration のみで決まり、
/// login 実行のみ登録済み flow（T6 以降）を要求する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OAuthSupport {
    /// `auth.oauth.isSubscription`。
    pub subscription: bool,
}

/// `builtinProviders()` の provider metadata（`auth status` の
/// `result.providers[]` 再現に必要な静的表。model data 自体は data file）。
#[derive(Debug, Clone, Copy)]
pub struct BuiltinProviderMeta {
    pub id: &'static str,
    /// display name（`ProviderAuthInfo.name`）。
    pub name: &'static str,
    /// provider-level `baseUrl`（descriptor 値。model 自身の baseUrl は
    /// bundled data 側にあり、こちらは provider 既定として残す）。
    pub base_url: Option<&'static str>,
    /// api_key login の prompt name（`auth.apiKey.name`、login method の
    /// 有無判定にも使う。None = api_key login 非対応 — openai-codex）。
    pub api_key_name: Option<&'static str>,
    /// api key / ambient credential の env 解決規則。
    pub env_auth: EnvAuth,
    /// OAuth capability 宣言。declaration の有無だけで `oauth` method を
    /// advertise する（TS `auth.ts` L56-66 parity）— flow 登録は
    /// `login --type oauth` の実行可否のみを制御する。
    pub oauth: Option<OAuthSupport>,
}

const fn bp(
    id: &'static str,
    base_url: Option<&'static str>,
    name: &'static str,
    api_key_name: Option<&'static str>,
    env_auth: EnvAuth,
    oauth: Option<OAuthSupport>,
) -> BuiltinProviderMeta {
    BuiltinProviderMeta {
        id,
        name,
        base_url,
        api_key_name,
        env_auth,
        oauth,
    }
}

/// `builtinProviders()` 宣言順の静的表。`api_key_name`・`env_auth`・`oauth`
/// は pi-ai provider descriptor をそのまま写した値。
pub static BUILTIN_PROVIDERS: &[BuiltinProviderMeta] = &[
    bp(
        "amazon-bedrock",
        None,
        "Amazon Bedrock",
        Some("AWS credentials or bearer token"),
        EnvAuth::AmazonBedrock,
        None,
    ),
    bp(
        "ant-ling",
        Some("https://api.ant-ling.com/v1"),
        "Ant Ling",
        Some("Ant Ling API key"),
        EnvAuth::Vars(&["ANT_LING_API_KEY"]),
        None,
    ),
    bp(
        "anthropic",
        Some("https://api.anthropic.com"),
        "Anthropic",
        Some("Anthropic API key"),
        EnvAuth::Anthropic,
        Some(OAuthSupport { subscription: true }),
    ),
    bp(
        "azure-openai-responses",
        None,
        "Azure OpenAI",
        Some("Azure OpenAI API key"),
        EnvAuth::Vars(&["AZURE_OPENAI_API_KEY"]),
        None,
    ),
    bp(
        "baseten",
        Some("https://inference.baseten.co/v1"),
        "Baseten",
        Some("Baseten API key"),
        EnvAuth::Vars(&["BASETEN_API_KEY"]),
        None,
    ),
    bp(
        "cerebras",
        Some("https://api.cerebras.ai/v1"),
        "Cerebras",
        Some("Cerebras API key"),
        EnvAuth::Vars(&["CEREBRAS_API_KEY"]),
        None,
    ),
    bp(
        "cloudflare-ai-gateway",
        None,
        "Cloudflare AI Gateway",
        Some("Cloudflare API key"),
        EnvAuth::Vars(&["CLOUDFLARE_API_KEY"]),
        None,
    ),
    bp(
        "cloudflare-workers-ai",
        None,
        "Cloudflare Workers AI",
        Some("Cloudflare API key"),
        EnvAuth::Vars(&["CLOUDFLARE_API_KEY"]),
        None,
    ),
    bp(
        "deepseek",
        Some("https://api.deepseek.com"),
        "DeepSeek",
        Some("DeepSeek API key"),
        EnvAuth::Vars(&["DEEPSEEK_API_KEY"]),
        None,
    ),
    bp(
        "fireworks",
        Some("https://api.fireworks.ai/inference"),
        "Fireworks",
        Some("Fireworks API key"),
        EnvAuth::Vars(&["FIREWORKS_API_KEY"]),
        None,
    ),
    bp(
        "github-copilot",
        Some("https://api.individual.githubcopilot.com"),
        "GitHub Copilot",
        Some("GitHub Copilot token"),
        EnvAuth::Vars(&["COPILOT_GITHUB_TOKEN"]),
        Some(OAuthSupport { subscription: true }),
    ),
    bp(
        "google",
        Some("https://generativelanguage.googleapis.com/v1beta"),
        "Google",
        Some("Gemini API key"),
        EnvAuth::Vars(&["GEMINI_API_KEY"]),
        None,
    ),
    bp(
        "google-vertex",
        None,
        "Google Vertex AI",
        Some("Google Cloud credentials"),
        EnvAuth::GoogleVertex,
        None,
    ),
    bp(
        "groq",
        Some("https://api.groq.com/openai/v1"),
        "Groq",
        Some("Groq API key"),
        EnvAuth::Vars(&["GROQ_API_KEY"]),
        None,
    ),
    bp(
        "huggingface",
        Some("https://router.huggingface.co/v1"),
        "Hugging Face",
        Some("Hugging Face token"),
        EnvAuth::Vars(&["HF_TOKEN"]),
        None,
    ),
    bp(
        "kimi-coding",
        Some("https://api.kimi.com/coding"),
        "Kimi For Coding",
        Some("Kimi API key"),
        EnvAuth::Vars(&["KIMI_API_KEY"]),
        Some(OAuthSupport { subscription: true }),
    ),
    bp(
        "minimax",
        Some("https://api.minimax.io/anthropic"),
        "MiniMax",
        Some("MiniMax API key"),
        EnvAuth::Vars(&["MINIMAX_API_KEY"]),
        None,
    ),
    bp(
        "minimax-cn",
        Some("https://api.minimaxi.com/anthropic"),
        "MiniMax CN",
        Some("MiniMax CN API key"),
        EnvAuth::Vars(&["MINIMAX_CN_API_KEY"]),
        None,
    ),
    bp(
        "mistral",
        Some("https://api.mistral.ai"),
        "Mistral",
        Some("Mistral API key"),
        EnvAuth::Vars(&["MISTRAL_API_KEY"]),
        None,
    ),
    bp(
        "moonshotai",
        Some("https://api.moonshot.ai/v1"),
        "Moonshot AI",
        Some("Moonshot AI API key"),
        EnvAuth::Vars(&["MOONSHOT_API_KEY"]),
        None,
    ),
    bp(
        "moonshotai-cn",
        Some("https://api.moonshot.cn/v1"),
        "Moonshot AI CN",
        Some("Moonshot AI API key"),
        EnvAuth::Vars(&["MOONSHOT_API_KEY"]),
        None,
    ),
    bp(
        "nvidia",
        Some("https://integrate.api.nvidia.com/v1"),
        "NVIDIA",
        Some("NVIDIA API key"),
        EnvAuth::Vars(&["NVIDIA_API_KEY"]),
        None,
    ),
    bp(
        "openai",
        Some("https://api.openai.com/v1"),
        "OpenAI",
        Some("OpenAI API key"),
        EnvAuth::Vars(&["OPENAI_API_KEY"]),
        None,
    ),
    bp(
        "openai-codex",
        Some("https://chatgpt.com/backend-api"),
        "OpenAI Codex",
        None,
        EnvAuth::None,
        Some(OAuthSupport { subscription: true }),
    ),
    bp(
        "opencode",
        None,
        "OpenCode Zen",
        Some("OpenCode API key"),
        EnvAuth::Vars(&["OPENCODE_API_KEY"]),
        None,
    ),
    bp(
        "opencode-go",
        None,
        "OpenCode Go",
        Some("OpenCode API key"),
        EnvAuth::Vars(&["OPENCODE_API_KEY"]),
        None,
    ),
    bp(
        "openrouter",
        Some("https://openrouter.ai/api/v1"),
        "OpenRouter",
        Some("OpenRouter API key"),
        EnvAuth::Vars(&["OPENROUTER_API_KEY"]),
        Some(OAuthSupport {
            subscription: false,
        }),
    ),
    bp(
        "qwen-token-plan",
        Some("https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1"),
        "Qwen Token Plan",
        Some("Qwen Token Plan API key"),
        EnvAuth::Vars(&["QWEN_TOKEN_PLAN_API_KEY"]),
        None,
    ),
    bp(
        "qwen-token-plan-cn",
        Some("https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1"),
        "Qwen Token Plan CN",
        Some("Qwen Token Plan CN API key"),
        EnvAuth::Vars(&["QWEN_TOKEN_PLAN_CN_API_KEY"]),
        None,
    ),
    bp(
        "qwen-token-plan-individual",
        Some("https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1"),
        "Qwen Token Plan Individual",
        Some("Qwen Token Plan Individual API key"),
        EnvAuth::Vars(&["QWEN_TOKEN_PLAN_API_KEY"]),
        None,
    ),
    bp(
        "radius",
        None,
        "Radius",
        Some("Radius API key"),
        EnvAuth::Vars(&["RADIUS_API_KEY"]),
        Some(OAuthSupport {
            subscription: false,
        }),
    ),
    bp(
        "together",
        Some("https://api.together.ai/v1"),
        "Together",
        Some("Together API key"),
        EnvAuth::Vars(&["TOGETHER_API_KEY"]),
        None,
    ),
    bp(
        "vercel-ai-gateway",
        Some("https://ai-gateway.vercel.sh"),
        "Vercel AI Gateway",
        Some("Vercel AI Gateway API key"),
        EnvAuth::Vars(&["AI_GATEWAY_API_KEY"]),
        None,
    ),
    bp(
        "xai",
        Some("https://api.x.ai/v1"),
        "xAI",
        Some("xAI API key"),
        EnvAuth::Vars(&["XAI_API_KEY"]),
        Some(OAuthSupport { subscription: true }),
    ),
    bp(
        "xiaomi",
        Some("https://api.xiaomimimo.com/v1"),
        "Xiaomi",
        Some("Xiaomi API key"),
        EnvAuth::Vars(&["XIAOMI_API_KEY"]),
        None,
    ),
    bp(
        "xiaomi-token-plan-ams",
        Some("https://token-plan-ams.xiaomimimo.com/v1"),
        "Xiaomi Token Plan AMS",
        Some("Xiaomi Token Plan AMS API key"),
        EnvAuth::Vars(&["XIAOMI_TOKEN_PLAN_AMS_API_KEY"]),
        None,
    ),
    bp(
        "xiaomi-token-plan-cn",
        Some("https://token-plan-cn.xiaomimimo.com/v1"),
        "Xiaomi Token Plan CN",
        Some("Xiaomi Token Plan CN API key"),
        EnvAuth::Vars(&["XIAOMI_TOKEN_PLAN_CN_API_KEY"]),
        None,
    ),
    bp(
        "xiaomi-token-plan-sgp",
        Some("https://token-plan-sgp.xiaomimimo.com/v1"),
        "Xiaomi Token Plan SGP",
        Some("Xiaomi Token Plan SGP API key"),
        EnvAuth::Vars(&["XIAOMI_TOKEN_PLAN_SGP_API_KEY"]),
        None,
    ),
    bp(
        "zai",
        Some("https://api.z.ai/api/coding/paas/v4"),
        "Z.AI",
        Some("Z.AI API key"),
        EnvAuth::Vars(&["ZAI_API_KEY"]),
        None,
    ),
    bp(
        "zai-coding-cn",
        Some("https://open.bigmodel.cn/api/coding/paas/v4"),
        "Z.AI Coding CN",
        Some("Z.AI Coding CN API key"),
        EnvAuth::Vars(&["ZAI_CODING_CN_API_KEY"]),
        None,
    ),
];

/// api_key env var を持たない provider で使う空 table（`EnvAuth::Vars` に
/// `&[]` を渡す代わりの名前付き定数）。
pub const EMPTY_ENV_VARS: &[&str] = &[];

/// bundled model data file `{<api>: {<modelId>: ModelJson}}` から
/// `CatalogModel` を flatten する。group key 順 → model key 順（JSON key
/// 順 = TS `Object.values` 順）を保持する。
pub fn bundled_models(provider_id: &str) -> Result<Vec<super::CatalogModel>, String> {
    let Some(json) = bundled_provider_json(provider_id) else {
        return Ok(Vec::new());
    };
    let parsed: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("bundled catalog for {provider_id} is corrupt: {e}"))?;
    let groups = parsed
        .as_object()
        .ok_or_else(|| format!("bundled catalog for {provider_id} is not an object"))?;
    let mut models = Vec::new();
    for (_api_group, models_obj) in groups {
        let Some(entries) = models_obj.as_object() else {
            return Err(format!(
                "bundled catalog for {provider_id}: api group is not an object"
            ));
        };
        for (_model_id, raw) in entries {
            let mut m: super::CatalogModel = serde_json::from_value(raw.clone())
                .map_err(|e| format!("bundled catalog for {provider_id} has invalid model: {e}"))?;
            m.provider = provider_id.to_string();
            models.push(m);
        }
    }
    Ok(models)
}

/// `mergeHeaders` — case-insensitive 衝突解決（後勝ち、case は後者を保持）。
/// pi-ai `models.ts` / pi-coding-agent `model-runtime.ts` の同名関数の port。
pub fn merge_headers(
    base: Option<&serde_json::Map<String, serde_json::Value>>,
    override_: Option<&serde_json::Map<String, serde_json::Value>>,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    if base.is_none() && override_.is_none() {
        return None;
    }
    let mut merged = base.cloned().unwrap_or_default();
    if let Some(ov) = override_ {
        for (name, value) in ov {
            let lower = name.to_lowercase();
            let existing = merged.keys().find(|k| k.to_lowercase() == lower).cloned();
            if let Some(k) = existing {
                merged.remove(&k);
            }
            merged.insert(name.clone(), value.clone());
        }
    }
    Some(merged)
}

/// Anthropic auth resolution で使う env var 名（`env-api-keys.ts` 定数）。
pub const ANTHROPIC_AUTH_TOKEN_ENV: &str = "ANTHROPIC_AUTH_TOKEN";
pub const ANTHROPIC_OAUTH_TOKEN_ENV: &str = "ANTHROPIC_OAUTH_TOKEN";
pub const ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";

/// `hasVertexAdcCredentials` — `GOOGLE_APPLICATION_CREDENTIALS` 指す file が
/// 存在するか、既定 ADC path（`~/.config/gcloud/application_default_credentials.json`）
/// が存在するか。
pub fn has_vertex_adc_credentials(
    env: &dyn Fn(&str) -> Option<String>,
    homedir: &dyn Fn() -> Option<std::path::PathBuf>,
) -> bool {
    if let Some(path) = env("GOOGLE_APPLICATION_CREDENTIALS") {
        return std::path::Path::new(&path).exists();
    }
    homedir()
        .map(|h| {
            h.join(".config")
                .join("gcloud")
                .join("application_default_credentials.json")
                .exists()
        })
        .unwrap_or(false)
}

/// `getEnvApiKey` の amazon-bedrock ambient 条件。
pub fn has_bedrock_env_credentials(env: &dyn Fn(&str) -> Option<String>) -> bool {
    let v = |n: &str| env(n).map(|s| !s.is_empty()).unwrap_or(false);
    v("AWS_PROFILE")
        || (v("AWS_ACCESS_KEY_ID") && v("AWS_SECRET_ACCESS_KEY"))
        || v("AWS_BEARER_TOKEN_BEDROCK")
        || v("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI")
        || v("AWS_CONTAINER_CREDENTIALS_FULL_URI")
        || v("AWS_WEB_IDENTITY_TOKEN_FILE")
}

/// `getEnvApiKey` の google-vertex ambient 条件（file check は
/// `has_vertex_adc_credentials` に分離）。
pub fn has_vertex_env_credentials(env: &dyn Fn(&str) -> Option<String>) -> bool {
    let v = |n: &str| env(n).map(|s| !s.is_empty()).unwrap_or(false);
    has_vertex_adc_credentials(env, &|| {
        env("USERPROFILE")
            .or_else(|| env("HOME"))
            .map(std::path::PathBuf::from)
            .or_else(std::env::home_dir)
    }) && (v("GOOGLE_CLOUD_PROJECT") || v("GCLOUD_PROJECT"))
        && v("GOOGLE_CLOUD_LOCATION")
}

/// `findEnvKeys` — provider の env auth で「設定済み」を示す env var 名。
/// 返り値は「configured 判定に使う全候補」（anthropic は AUTH_TOKEN 含む）。
pub fn env_auth_is_configured(
    meta: &BuiltinProviderMeta,
    env: &dyn Fn(&str) -> Option<String>,
) -> bool {
    let nonblank = |n: &str| env(n).map(|v| !v.is_empty()).unwrap_or(false);
    match meta.env_auth {
        EnvAuth::Vars(vars) => vars.iter().any(|n| nonblank(n)),
        EnvAuth::Anthropic => {
            nonblank(ANTHROPIC_AUTH_TOKEN_ENV)
                || nonblank(ANTHROPIC_OAUTH_TOKEN_ENV)
                || nonblank(ANTHROPIC_API_KEY_ENV)
        }
        EnvAuth::GoogleVertex => {
            nonblank("GOOGLE_CLOUD_API_KEY") || has_vertex_env_credentials(env)
        }
        EnvAuth::AmazonBedrock => has_bedrock_env_credentials(env),
        EnvAuth::None => false,
    }
}

/// `EnvAuth::Vars` の最初のセット済み var を返す（resolve 用）。
pub fn first_env_var<'a>(
    vars: &'a [&'a str],
    env: &dyn Fn(&str) -> Option<String>,
) -> Option<(&'a str, String)> {
    vars.iter()
        .find_map(|n| env(n).filter(|v| !v.is_empty()).map(|v| (*n, v)))
}

/// catalog 全体で共有される model id → `BTreeMap` 変換の補助（将来の
/// pricing 参照経路向け。現状は provider entry が Vec 順を保持する）。
pub type ModelIndex = BTreeMap<String, usize>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_generated_at_parses() {
        assert!(bundled_generated_at_ms().is_some());
    }

    #[test]
    fn builtin_table_covers_bundled_data_files() {
        for (id, _) in BUNDLED_DATA {
            assert!(
                BUILTIN_PROVIDERS.iter().any(|m| m.id == *id),
                "meta missing for {id}"
            );
        }
        for m in BUILTIN_PROVIDERS {
            assert!(
                BUNDLED_DATA.iter().any(|(id, _)| id == &m.id)
                    || m.id == "openai-codex"
                    || m.id == "radius",
                "data missing for {}",
                m.id
            );
        }
    }

    #[test]
    fn bundled_anthropic_models_flatten() {
        let models = bundled_models("anthropic").unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().all(|m| m.provider == "anthropic"));
        assert!(models.iter().all(|m| m.api == "anthropic-messages"));
    }

    #[test]
    fn merge_headers_case_insensitive_override_wins() {
        let mut base = serde_json::Map::new();
        base.insert("Authorization".into(), "a".into());
        base.insert("X-Keep".into(), "k".into());
        let mut ov = serde_json::Map::new();
        ov.insert("authorization".into(), "b".into());
        let merged = merge_headers(Some(&base), Some(&ov)).unwrap();
        assert_eq!(merged.get("authorization").unwrap(), "b");
        assert_eq!(merged.get("X-Keep").unwrap(), "k");
        assert!(!merged.contains_key("Authorization"));
    }
}
