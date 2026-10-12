//! Port of `packages/core/src/model/config.ts`.
//!
//! `models.yaml` (user → project, last wins). Merge shares the policy
//! merge rules via `merge_policy_layers` (overrides deep-merge,
//! `preferences.<tier>` arrays replace wholesale); `exclude` is a deny
//! list and is union-aggregated across layers separately.

use serde_json::{json, Map, Value};

use crate::errors::{CheaperError, ErrorCode};
use crate::model::tier::{parse_tier, Tier, TIERS};
use crate::policy::merge::{merge_policy_layers, MergeMode};
use crate::worker::schema::{schema_error_entries, SchemaErrorEntry};

/// `ModelConfig`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelConfig {
    /// `"<provider>/<modelId>"` → tier, in file order.
    pub overrides: Vec<(String, Tier)>,
    /// tier → `["<provider>/<modelId>" | "<provider>/*"]`.
    pub preferences: TierPreferences,
    /// `"<provider>/<modelId>"` | `"<provider>/*"` exclude patterns.
    pub exclude: Option<Vec<String>>,
}

impl ModelConfig {
    /// `config.overrides[key]` lookup.
    pub fn override_tier(&self, key: &str) -> Option<Tier> {
        self.overrides
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, t)| *t)
    }
}

/// `Partial<Record<Tier, string[]>>`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TierPreferences {
    pub low: Option<Vec<String>>,
    pub medium: Option<Vec<String>>,
    pub high: Option<Vec<String>>,
}

impl TierPreferences {
    /// `preferences[tier]` lookup.
    pub fn for_tier(&self, tier: Tier) -> Option<&[String]> {
        match tier {
            Tier::Low => self.low.as_deref(),
            Tier::Medium => self.medium.as_deref(),
            Tier::High => self.high.as_deref(),
        }
    }
}

/// `MODELS_CONFIG_SCHEMA` verbatim.
pub fn models_config_schema() -> Value {
    let tier_preference = json!({"type": "array", "items": {"type": "string"}});
    let tiers: Vec<&str> = TIERS.iter().map(|t| t.as_str()).collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["version"],
        "properties": {
            "version": {"type": "integer", "const": 1},
            "overrides": {
                "type": "object",
                "additionalProperties": {"type": "string", "enum": tiers},
            },
            "preferences": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "low": tier_preference,
                    "medium": tier_preference,
                    "high": tier_preference,
                },
            },
            "exclude": {"type": "array", "items": {"type": "string"}},
        },
    })
}

fn models_config_validator() -> &'static jsonschema::Validator {
    static V: std::sync::OnceLock<jsonschema::Validator> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        jsonschema::draft202012::new(&models_config_schema())
            .expect("models config schema compiles")
    })
}

fn validate_models_config(
    raw: &Value,
    file_path: &str,
) -> Result<Map<String, Value>, CheaperError> {
    let validator = models_config_validator();
    if validator.is_valid(raw) {
        return Ok(raw.as_object().expect("schema enforces object").clone());
    }
    let errors: Vec<SchemaErrorEntry> = schema_error_entries(validator, raw);
    let mut details = Map::new();
    details.insert("path".to_string(), json!(file_path));
    details.insert(
        "errors".to_string(),
        serde_json::to_value(errors).unwrap_or(Value::Null),
    );
    Err(CheaperError::new(
        ErrorCode::InvalidModelConfig,
        format!("invalid models config: {file_path}"),
    )
    .with_details(details))
}

/// `readModelsConfig`: only `ENOENT`/`ENOTDIR` skip.
fn read_models_config(file_path: &str) -> Result<Option<Map<String, Value>>, CheaperError> {
    let bytes = match std::fs::read(file_path) {
        Ok(b) => b,
        Err(e) => {
            if crate::errors::is_absent_file_error(&e) {
                return Ok(None);
            }
            let mut details = Map::new();
            details.insert("path".to_string(), json!(file_path));
            details.insert("reason".to_string(), json!("read-failed"));
            details.insert(
                "errno".to_string(),
                json!(crate::errors::io_errno_display(file_path, &e)),
            );
            return Err(CheaperError::new(
                ErrorCode::InvalidModelConfig,
                format!("models config read failed: {file_path}"),
            )
            .with_details(details)
            .with_cause(e));
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    // `cheaper-core` event(design §instrumentation 配置)— `file.name`
    // は basename のみ(full path は載せない)。
    tracing::trace!(
        file.name = crate::nodepath::basename(file_path, None).as_str(),
        "models config read"
    );
    let parsed: Value = crate::yaml::parse_yaml(&text).map_err(|e| {
        let mut details = Map::new();
        details.insert("path".to_string(), json!(file_path));
        details.insert("reason".to_string(), json!("yaml-parse"));
        CheaperError::new(
            ErrorCode::InvalidModelConfig,
            format!("models yaml parse failed: {file_path}"),
        )
        .with_details(details)
        .with_cause(e)
    })?;
    match parsed {
        Value::Object(_) => validate_models_config(&parsed, file_path).map(Some),
        _ => {
            let mut details = Map::new();
            details.insert("path".to_string(), json!(file_path));
            details.insert("reason".to_string(), json!("not-object"));
            Err(CheaperError::new(
                ErrorCode::InvalidModelConfig,
                format!("models config is not an object: {file_path}"),
            )
            .with_details(details))
        }
    }
}

/// `loadModelConfig` options.
pub struct LoadModelConfigOptions<'a> {
    pub user_config_dir: &'a str,
    pub project_dir: &'a str,
}

/// `loadModelConfig`: user → project, last-wins merge; `exclude` is
/// union-aggregated (a project layer cannot unset a user deny).
pub fn load_model_config(opts: &LoadModelConfigOptions<'_>) -> Result<ModelConfig, CheaperError> {
    let mut merged = Map::new();
    let mut excluded: Vec<String> = Vec::new();
    for (layer, file_path) in [
        (
            "user",
            crate::nodepath::join(&[opts.user_config_dir, "models.yaml"]),
        ),
        (
            "project",
            crate::nodepath::join(&[opts.project_dir, "models.yaml"]),
        ),
    ] {
        let Some(raw) = read_models_config(&file_path)? else {
            continue;
        };
        tracing::debug!(config.layer = layer, "models config layer merged");
        let mut rest = raw.clone();
        if let Some(exclude) = rest.remove("exclude").and_then(|v| v.as_array().cloned()) {
            for pattern in exclude {
                if let Some(s) = pattern.as_str() {
                    if !excluded.contains(&s.to_string()) {
                        excluded.push(s.to_string());
                    }
                }
            }
        }
        merged = merge_policy_layers(&merged, &rest, MergeMode::Replace)
            .expect("replace-mode merge cannot fail");
    }
    let overrides = merged
        .get("overrides")
        .and_then(Value::as_object)
        .map(|o| {
            o.iter()
                .filter_map(|(k, v)| parse_tier(v).map(|t| (k.clone(), t)))
                .collect()
        })
        .unwrap_or_default();
    let pref_obj = merged.get("preferences").and_then(Value::as_object);
    let pref_for = |tier: Tier| -> Option<Vec<String>> {
        pref_obj
            .and_then(|p| p.get(tier.as_str()))
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
    };
    let preferences = TierPreferences {
        low: pref_for(Tier::Low),
        medium: pref_for(Tier::Medium),
        high: pref_for(Tier::High),
    };
    let config = ModelConfig {
        overrides,
        preferences,
        exclude: if excluded.is_empty() {
            None
        } else {
            Some(excluded)
        },
    };
    // resolved config の集計値のみ(pattern・key 一覧は載せない)。
    tracing::debug!(
        override.count = config.overrides.len(),
        exclude.count = config.exclude.as_ref().map_or(0, Vec::len),
        "models config resolved"
    );
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &std::path::Path, content: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("models.yaml"), content).unwrap();
    }

    #[test]
    fn absent_files_give_empty_config() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = load_model_config(&LoadModelConfigOptions {
            user_config_dir: tmp.path().join("user").to_str().unwrap(),
            project_dir: tmp.path().join("proj").to_str().unwrap(),
        })
        .unwrap();
        assert!(cfg.overrides.is_empty());
        assert!(cfg.exclude.is_none());
    }

    #[test]
    fn project_overrides_user_and_exclude_unions() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("user");
        let proj = tmp.path().join("proj");
        write(&user, "version: 1\noverrides: {p/a: high, p/b: low}\npreferences: {low: [p/b]}\nexclude: [p/x]\n");
        write(
            &proj,
            "version: 1\noverrides: {p/a: low}\nexclude: [p/y, p/x]\n",
        );
        let cfg = load_model_config(&LoadModelConfigOptions {
            user_config_dir: user.to_str().unwrap(),
            project_dir: proj.to_str().unwrap(),
        })
        .unwrap();
        // Project wins on the shared key; user-only key survives.
        assert_eq!(cfg.override_tier("p/a"), Some(Tier::Low));
        assert_eq!(cfg.override_tier("p/b"), Some(Tier::Low));
        assert_eq!(cfg.preferences.for_tier(Tier::Low).unwrap(), &["p/b"]);
        // exclude is a union in user-then-project order.
        assert_eq!(
            cfg.exclude.unwrap(),
            vec!["p/x".to_string(), "p/y".to_string()]
        );
    }

    #[test]
    fn invalid_yaml_and_schema_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("user");
        write(&user, "version: 2\n");
        let e = load_model_config(&LoadModelConfigOptions {
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("p").to_str().unwrap(),
        })
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidModelConfig);
        // not an object
        write(&user, "- just\n- a\n- list\n");
        let e = load_model_config(&LoadModelConfigOptions {
            user_config_dir: user.to_str().unwrap(),
            project_dir: tmp.path().join("p").to_str().unwrap(),
        })
        .unwrap_err();
        assert_eq!(e.details().unwrap()["reason"], json!("not-object"));
    }
}
