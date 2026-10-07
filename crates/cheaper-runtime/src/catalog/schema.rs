//! Pi `models.json` user file schema.
//!
//! Port of `pi-coding-agent/dist/core/model-config.ts` (`ModelsConfigSchema` +
//! `ModelConfig.load`): JSONC 相当のコメント・trailing comma・BOM を許容し、
//! compiled JSON Schema で validate してから型付き構造へ deserialize する。
//! unknown key は schema 同様に無視する（serde の default 挙動で一致）。
//!
//! エラー時の扱いも TS に揃える: file 不在は empty config、parse/schema
//! 失敗は error string を返して「provider 定義なし」として扱う
//! （TS の `ModelConfig.error` 相当。caller が catalog load error として記録する）。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

/// `stripBom` — leading `\u{FEFF}` を除去する。
fn strip_bom(content: &str) -> &str {
    content.strip_prefix('\u{feff}').unwrap_or(content)
}

/// `stripJsonComments` — `//` line comment と trailing comma を除去する
/// （string literal は保護）。TS は regex 2 段処理だが、こちらは scanner で
/// 同じ結果を得る: `//` から行末までを除去し、次に `,` の直後（空白のみ挟む）
/// が `}` または `]` の comma を除去する。
fn strip_json_comments(input: &str) -> String {
    // pass 1: `//` comments outside strings.
    let mut stage1 = String::with_capacity(input.len());
    {
        let mut chars = input.chars().peekable();
        let mut in_string = false;
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    in_string = !in_string;
                    stage1.push(c);
                }
                '\\' if in_string => {
                    stage1.push(c);
                    if let Some(next) = chars.next() {
                        stage1.push(next);
                    }
                }
                '/' if !in_string && chars.peek() == Some(&'/') => {
                    chars.next();
                    for c in chars.by_ref() {
                        if c == '\n' {
                            stage1.push(c);
                            break;
                        }
                    }
                }
                _ => stage1.push(c),
            }
        }
    }
    // pass 2: `,(\s*[}\]])` trailing commas outside strings.
    let mut out = String::with_capacity(stage1.len());
    {
        let bytes = stage1.as_bytes();
        let mut i = 0;
        let mut in_string = false;
        while i < bytes.len() {
            let b = bytes[i];
            match b {
                b'"' => {
                    in_string = !in_string;
                    out.push('"');
                    i += 1;
                }
                b'\\' if in_string => {
                    out.push('\\');
                    i += 1;
                    if i < bytes.len() {
                        out.push(bytes[i] as char);
                        i += 1;
                    }
                }
                b',' if !in_string => {
                    let mut j = i + 1;
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && (bytes[j] == b'}' || bytes[j] == b']') {
                        i += 1; // drop the comma
                    } else {
                        out.push(',');
                        i += 1;
                    }
                }
                _ => {
                    // multi-byte UTF-8 はそのまま span コピーする。
                    let ch_len = utf8_len(b);
                    out.push_str(&stage1[i..i + ch_len]);
                    i += ch_len;
                }
            }
        }
    }
    out
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first < 0xE0 {
        2
    } else if first < 0xF0 {
        3
    } else {
        4
    }
}

fn compat_schema() -> Value {
    // Type.Union([OpenAICompletionsCompat, OpenAIResponsesCompat,
    // AnthropicMessagesCompat]) — いずれも optional prop のみの object で、
    // unknown key は許容。anyOf 3 件の object で「object であること」を表現し、
    // 宣言 prop の型制約を各 variant で確認する。
    let openai_completions = serde_json::json!({
        "type": "object",
        "properties": {
            "supportsStore": { "type": "boolean" },
            "supportsDeveloperRole": { "type": "boolean" },
            "supportsReasoningEffort": { "type": "boolean" },
            "supportsUsageInStreaming": { "type": "boolean" },
            "supportsFinishReason": { "type": "boolean" },
            "maxTokensField": { "enum": ["max_completion_tokens", "max_tokens"] },
            "requiresToolResultName": { "type": "boolean" },
            "requiresAssistantAfterToolResult": { "type": "boolean" },
            "requiresThinkingAsText": { "type": "boolean" },
            "requiresReasoningContentOnAssistantMessages": { "type": "boolean" },
            "thinkingFormat": { "enum": [
                "openai", "openrouter", "together", "baseten", "deepseek",
                "zai", "qwen", "chat-template", "qwen-chat-template",
                "string-thinking", "ant-ling"
            ] },
            "chatTemplateKwargs": { "type": "object" },
            "chatTemplateArgs": { "type": "object" },
            "cacheControlFormat": { "const": "anthropic" },
            "openRouterRouting": { "type": "object" },
            "vercelGatewayRouting": { "type": "object" },
            "supportsOpenAIGrammarTools": { "type": "boolean" },
            "supportsStrictMode": { "type": "boolean" },
            "sendSessionAffinityHeaders": { "type": "boolean" },
            "deferredToolsMode": { "const": "kimi" },
            "sessionAffinityFormat": { "enum": ["openai", "openai-nosession", "openrouter"] },
            "supportsLongCacheRetention": { "type": "boolean" },
            "vllmPriority": { "type": "number" }
        }
    });
    let openai_responses = serde_json::json!({
        "type": "object",
        "properties": {
            "supportsDeveloperRole": { "type": "boolean" },
            "sessionAffinityFormat": { "enum": ["openai", "openai-nosession", "openrouter"] },
            "supportsLongCacheRetention": { "type": "boolean" },
            "supportsStrictMode": { "type": "boolean" },
            "supportsOpenAIGrammarTools": { "type": "boolean" },
            "supportsAdditionalTools": { "type": "boolean" },
            "supportsToolSearch": { "type": "boolean" },
            "supportsMaxOutputTokens": { "type": "boolean" }
        }
    });
    let anthropic_messages = serde_json::json!({
        "type": "object",
        "properties": {
            "supportsEagerToolInputStreaming": { "type": "boolean" },
            "supportsLongCacheRetention": { "type": "boolean" },
            "sendSessionAffinityHeaders": { "type": "boolean" },
            "supportsCacheControlOnTools": { "type": "boolean" },
            "supportsTemperature": { "type": "boolean" },
            "forceAdaptiveThinking": { "type": "boolean" },
            "allowEmptySignature": { "type": "boolean" },
            "supportsStrictTools": { "type": "boolean" },
            "supportsMidConvoEffort": { "type": "boolean" },
            "supportsToolReferences": { "type": "boolean" }
        }
    });
    serde_json::json!({ "anyOf": [openai_completions, openai_responses, anthropic_messages] })
}

/// `ModelsConfigSchema` の JSON Schema 表現（draft 2020-12）。
/// TypeBox `Type.Object` は additionalProperties を制限しないため
/// 「宣言済み prop の型 + unknown key 許容」の挙動を再現する。
/// definition の `cost` は required rates、override の `cost` は全 optional
/// とするため `definitionCost`/`overrideCost` を分けている。
fn full_models_json_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["providers"],
        "properties": {
            "providers": {
                "type": "object",
                "additionalProperties": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "baseUrl": { "type": "string", "minLength": 1 },
                        "apiKey": { "type": "string", "minLength": 1 },
                        "api": { "type": "string", "minLength": 1 },
                        "oauth": { "const": "radius" },
                        "headers": { "$ref": "#/$defs/stringMap" },
                        "compat": { "$ref": "#/$defs/compat" },
                        "authHeader": { "type": "boolean" },
                        "models": {
                            "type": "array",
                            "items": { "$ref": "#/$defs/modelDefinition" }
                        },
                        "modelOverrides": {
                            "type": "object",
                            "additionalProperties": { "$ref": "#/$defs/modelOverride" }
                        }
                    }
                }
            }
        },
        "$defs": {
            "stringMap": {
                "type": "object",
                "additionalProperties": { "type": "string" }
            },
            "thinkingLevelMap": {
                "type": "object",
                "properties": {
                    "off": { "type": ["string", "null"] },
                    "minimal": { "type": ["string", "null"] },
                    "low": { "type": ["string", "null"] },
                    "medium": { "type": ["string", "null"] },
                    "high": { "type": ["string", "null"] },
                    "xhigh": { "type": ["string", "null"] },
                    "max": { "type": ["string", "null"] }
                }
            },
            "compat": compat_schema(),
            "costTier": {
                "type": "object",
                "required": ["inputTokensAbove", "input", "output", "cacheRead", "cacheWrite"],
                "properties": {
                    "inputTokensAbove": { "type": "number" },
                    "input": { "type": "number" },
                    "output": { "type": "number" },
                    "cacheRead": { "type": "number" },
                    "cacheWrite": { "type": "number" }
                }
            },
            "definitionCost": {
                "type": "object",
                "required": ["input", "output", "cacheRead", "cacheWrite"],
                "properties": {
                    "input": { "type": "number" },
                    "output": { "type": "number" },
                    "cacheRead": { "type": "number" },
                    "cacheWrite": { "type": "number" },
                    "tiers": { "type": "array", "items": { "$ref": "#/$defs/costTier" } }
                }
            },
            "overrideCost": {
                "type": "object",
                "properties": {
                    "input": { "type": "number" },
                    "output": { "type": "number" },
                    "cacheRead": { "type": "number" },
                    "cacheWrite": { "type": "number" },
                    "tiers": { "type": "array", "items": { "$ref": "#/$defs/costTier" } }
                }
            },
            "modelDefinition": {
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string", "minLength": 1 },
                    "name": { "type": "string", "minLength": 1 },
                    "api": { "type": "string", "minLength": 1 },
                    "baseUrl": { "type": "string", "minLength": 1 },
                    "reasoning": { "type": "boolean" },
                    "thinkingLevelMap": { "$ref": "#/$defs/thinkingLevelMap" },
                    "input": { "type": "array", "items": { "enum": ["text", "image"] } },
                    "cost": { "$ref": "#/$defs/definitionCost" },
                    "contextWindow": { "type": "number" },
                    "maxTokens": { "type": "number" },
                    "samplingParams": { "type": "object" },
                    "headers": { "$ref": "#/$defs/stringMap" },
                    "compat": { "$ref": "#/$defs/compat" }
                }
            },
            "modelOverride": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "minLength": 1 },
                    "reasoning": { "type": "boolean" },
                    "thinkingLevelMap": { "$ref": "#/$defs/thinkingLevelMap" },
                    "input": { "type": "array", "items": { "enum": ["text", "image"] } },
                    "cost": { "$ref": "#/$defs/overrideCost" },
                    "contextWindow": { "type": "number" },
                    "maxTokens": { "type": "number" },
                    "samplingParams": { "type": "object" },
                    "headers": { "$ref": "#/$defs/stringMap" },
                    "compat": { "$ref": "#/$defs/compat" }
                }
            }
        }
    })
}

fn validator() -> &'static jsonschema::Validator {
    static V: std::sync::OnceLock<jsonschema::Validator> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        jsonschema::draft202012::new(&full_models_json_schema())
            .expect("models.json schema must compile")
    })
}

/// `formatValidationPath` — required error は `base.missingProp` 形にする。
fn format_validation_path(error: &jsonschema::ValidationError<'_>) -> String {
    let instance = error.instance_path().to_string();
    let base = instance.trim_start_matches('/').replace('/', ".");
    if let jsonschema::error::ValidationErrorKind::Required { property } = error.kind() {
        let prop = property.to_string();
        let prop = prop.trim_matches('"');
        return if base.is_empty() {
            prop.to_string()
        } else {
            format!("{base}.{prop}")
        };
    }
    if base.is_empty() {
        "root".to_string()
    } else {
        base
    }
}

// ---- typed model (Pi models.json schema compatible) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelInput {
    Text,
    Image,
}

/// `thinkingLevelMap`: 7 level → string | null。
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct ThinkingLevelMap {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub off: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimal: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medium: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xhigh: Option<Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<Option<String>>,
}

impl ThinkingLevelMap {
    /// `{...base, ...override}` per-key merge（`applyModelOverride` 相当）。
    /// override で key が存在すれば base 値を置き換える（null 指定も上書き）。
    pub fn merged(&self, override_map: &ThinkingLevelMap) -> ThinkingLevelMap {
        fn pick(a: &Option<Option<String>>, b: &Option<Option<String>>) -> Option<Option<String>> {
            if b.is_some() {
                b.clone()
            } else {
                a.clone()
            }
        }
        ThinkingLevelMap {
            off: pick(&self.off, &override_map.off),
            minimal: pick(&self.minimal, &override_map.minimal),
            low: pick(&self.low, &override_map.low),
            medium: pick(&self.medium, &override_map.medium),
            high: pick(&self.high, &override_map.high),
            xhigh: pick(&self.xhigh, &override_map.xhigh),
            max: pick(&self.max, &override_map.max),
        }
    }
}

/// cost tier (`ModelCostTierSchema`)。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCostTierJson {
    pub input_tokens_above: f64,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// `models[].cost` — 4 rates 必須。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCostJson {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    #[serde(default)]
    pub tiers: Option<Vec<ModelCostTierJson>>,
}

/// `modelOverrides[].cost` — 全 field optional。
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCostOverrideJson {
    #[serde(default)]
    pub input: Option<f64>,
    #[serde(default)]
    pub output: Option<f64>,
    #[serde(default)]
    pub cache_read: Option<f64>,
    #[serde(default)]
    pub cache_write: Option<f64>,
    #[serde(default)]
    pub tiers: Option<Vec<ModelCostTierJson>>,
}

/// `providers.*.models[]` (`ModelDefinitionSchema`)。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDefinitionJson {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub reasoning: Option<bool>,
    #[serde(default)]
    pub thinking_level_map: Option<ThinkingLevelMap>,
    #[serde(default)]
    pub input: Option<Vec<ModelInput>>,
    #[serde(default)]
    pub cost: Option<ModelCostJson>,
    #[serde(default)]
    pub context_window: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<f64>,
    #[serde(default)]
    pub sampling_params: Option<Map<String, Value>>,
    #[serde(default)]
    pub headers: Option<Map<String, Value>>,
    #[serde(default)]
    pub compat: Option<Value>,
}

/// `providers.*.modelOverrides.*` (`ModelOverrideSchema`)。
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOverrideJson {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub reasoning: Option<bool>,
    #[serde(default)]
    pub thinking_level_map: Option<ThinkingLevelMap>,
    #[serde(default)]
    pub input: Option<Vec<ModelInput>>,
    #[serde(default)]
    pub cost: Option<ModelCostOverrideJson>,
    #[serde(default)]
    pub context_window: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<f64>,
    #[serde(default)]
    pub sampling_params: Option<Map<String, Value>>,
    #[serde(default)]
    pub headers: Option<Map<String, Value>>,
    #[serde(default)]
    pub compat: Option<Value>,
}

/// `providers.*` (`ProviderConfigSchema`)。
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigJson {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub oauth: Option<String>,
    #[serde(default)]
    pub headers: Option<Map<String, Value>>,
    #[serde(default)]
    pub compat: Option<Value>,
    #[serde(default)]
    pub auth_header: Option<bool>,
    #[serde(default)]
    pub models: Option<Vec<ModelDefinitionJson>>,
    #[serde(default)]
    pub model_overrides: Option<BTreeMap<String, ModelOverrideJson>>,
}

/// `ModelsConfigSchema` — `{providers: Record<providerId, ProviderConfig>}`。
/// file 内の provider 宣言順を保持する（`preserve_order` 前提）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelsJsonFile {
    pub providers: Vec<(String, ProviderConfigJson)>,
}

impl<'de> Deserialize<'de> for ModelsJsonFile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Root {
            providers: Map<String, Value>,
        }
        let root = Root::deserialize(deserializer)?;
        let mut providers = Vec::with_capacity(root.providers.len());
        for (id, raw) in root.providers {
            let config = ProviderConfigJson::deserialize(raw).map_err(serde::de::Error::custom)?;
            providers.push((id, config));
        }
        Ok(ModelsJsonFile { providers })
    }
}

/// `ModelConfig.load` 相当: 不在 → `Ok(None)`、失敗は error string。
/// 成功時は `Ok(Some(file))`。
pub fn load_models_json(path: &Path) -> Result<Option<ModelsJsonFile>, String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(format!(
                "Failed to load models.json: {}\n\nFile: {}",
                e,
                path.display()
            ));
        }
    };
    parse_models_json(&content).map(Some).map_err(|e| {
        // TS は JSON parse 失敗 `Failed to parse models.json:` と schema
        // 失敗 `Invalid models.json schema:` を分ける。`ModelsJsonError` が
        // 既に schema message を持つ場合はそのまま使う。
        let message = if e.message.starts_with("Invalid models.json schema:") {
            e.message
        } else {
            format!("Failed to parse models.json: {}", e.message)
        };
        format!("{}\n\nFile: {}", message, path.display())
    })
}

/// 文字列からの parse。schema validation 失敗・parse 失敗は error string 付き
/// `ModelsJsonError` にする（TS の `Invalid models.json schema:` / parse error
/// message を区別できる形）。
#[derive(Debug)]
pub struct ModelsJsonError {
    pub message: String,
}

pub fn parse_models_json(content: &str) -> Result<ModelsJsonFile, ModelsJsonError> {
    let stripped = strip_json_comments(strip_bom(content));
    let parsed: Value = serde_json::from_str(&stripped).map_err(|e| ModelsJsonError {
        message: e.to_string(),
    })?;
    let validator = validator();
    let errors: Vec<String> = validator
        .iter_errors(&parsed)
        .map(|e| format!("  - {}: {}", format_validation_path(&e), e))
        .collect();
    if !errors.is_empty() {
        return Err(ModelsJsonError {
            message: format!("Invalid models.json schema:\n{}", errors.join("\n")),
        });
    }
    serde_json::from_value(parsed).map_err(|e| ModelsJsonError {
        message: e.to_string(),
    })
}

/// 直接 parse（test / fixture 用）。ファイル名を伴う `load` wrapper を使うこと。
pub fn parse_models_json_str(content: &str) -> Result<ModelsJsonFile, String> {
    parse_models_json(content).map_err(|e| e.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_comments_and_trailing_commas() {
        let input = r#"{
            // leading comment
            "providers": {
                "custom": { "baseUrl": "http://x", }, // trailing
            },
        }"#;
        let parsed: Value = serde_json::from_str(&strip_json_comments(input)).unwrap();
        assert!(parsed["providers"]["custom"]["baseUrl"].is_string());
    }

    #[test]
    fn does_not_strip_comment_like_content_inside_strings() {
        let input = r#"{"a": "http://example.com//x", "b": "]}"}"#;
        let out = strip_json_comments(input);
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["a"], "http://example.com//x");
        assert_eq!(parsed["b"], "]}");
    }

    #[test]
    fn rejects_missing_providers() {
        assert!(parse_models_json_str("{}").is_err());
    }

    #[test]
    fn accepts_minimal_custom_provider_and_ignores_unknown_keys() {
        let file = parse_models_json_str(
            r#"{"providers": {"ollama": {
                "api": "openai-completions",
                "baseUrl": "http://localhost:11434/v1",
                "apiKey": "ollama",
                "unknownFutureKey": {"nested": true},
                "models": [{"id": "qwen3.5:4b"}]
            }}}"#,
        )
        .unwrap();
        assert_eq!(file.providers.len(), 1);
        assert_eq!(file.providers[0].0, "ollama");
    }

    #[test]
    fn rejects_model_definition_without_id() {
        let err = parse_models_json_str(
            r#"{"providers": {"x": {"baseUrl": "http://x", "models": [{"name": "m"}]}}}"#,
        )
        .unwrap_err();
        assert!(err.contains("id"), "{err}");
    }
}
