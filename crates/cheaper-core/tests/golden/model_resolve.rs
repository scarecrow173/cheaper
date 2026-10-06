use cheaper_core::model::config::{load_model_config, LoadModelConfigOptions};
use cheaper_core::model::resolve::{
    resolve_explicit_model, resolve_model, ModelPolicySlice, ResolveExplicitModelOptions,
    ResolveModelOptions,
};
use cheaper_core::model::tier::parse_tier;
use cheaper_core::policy::schema::TierFallback;

use crate::common::*;

#[test]
fn resolve_cases() {
    let f = fixture("model/resolve.json");
    for case in cases(&f) {
        let name = case_name(case);
        let input = &case["input"];
        let op = case["op"].as_str().unwrap();
        let available = models_from(&input["available"]);
        let config = model_config_from(&input["config"]);
        let opi = input["outputPerInput"].as_f64();
        let produced = match op {
            "resolveModel" => {
                let policy = &input["policy"];
                match resolve_model(&ResolveModelOptions {
                    available: &available,
                    requested_tier: parse_tier(&input["requestedTier"]).unwrap(),
                    policy: ModelPolicySlice {
                        max_tier: parse_tier(&policy["maxTier"]).unwrap(),
                        tier_fallback: if policy["tierFallback"].as_str() == Some("higher") {
                            TierFallback::Higher
                        } else {
                            TierFallback::None
                        },
                    },
                    config: &config,
                    output_per_input: opi,
                }) {
                    Ok(r) => Ok(dump_resolved(&r)),
                    Err(e) => Err(cheaper_err(&e, &[])),
                }
            }
            "resolveExplicitModel" => {
                match resolve_explicit_model(&ResolveExplicitModelOptions {
                    available: &available,
                    model_ref: &model_ref_from(&input["ref"]),
                    policy_max_tier: parse_tier(&input["maxTier"]).unwrap(),
                    config: &config,
                    output_per_input: opi,
                }) {
                    Ok(r) => Ok(dump_resolved(&r)),
                    Err(e) => Err(cheaper_err(&e, &[])),
                }
            }
            other => panic!("unknown op {other} in {name}"),
        };
        check(&name, &case["expected"], produced);
    }
}

#[test]
fn config_cases() {
    let f = fixture("model/config.json");
    for case in cases(&f) {
        let name = case_name(case);
        let Some(c) = case_dir(case) else {
            panic!("{name}: case dir setup failed");
        };
        let dir = c.dir.to_string_lossy().to_string();
        let user = join(&dir, case["opts"]["userConfigDir"].as_str().unwrap());
        let project = join(&dir, case["opts"]["projectDir"].as_str().unwrap());
        let produced = match load_model_config(&LoadModelConfigOptions {
            user_config_dir: &user,
            project_dir: &project,
        }) {
            Ok(config) => {
                let mut overrides = serde_json::Map::new();
                let mut keys: Vec<&String> = config.overrides.iter().map(|(k, _)| k).collect();
                keys.sort();
                for k in keys {
                    let tier = config.override_tier(k).unwrap();
                    overrides.insert(k.clone(), serde_json::json!(tier.as_str()));
                }
                let mut prefs = serde_json::Map::new();
                for (name, list) in [
                    ("low", &config.preferences.low),
                    ("medium", &config.preferences.medium),
                    ("high", &config.preferences.high),
                ] {
                    if let Some(list) = list {
                        prefs.insert(name.to_string(), serde_json::json!(list));
                    }
                }
                let mut out = serde_json::Map::new();
                out.insert(
                    "overrides".to_string(),
                    serde_json::Value::Object(overrides),
                );
                out.insert("preferences".to_string(), serde_json::Value::Object(prefs));
                if let Some(exclude) = &config.exclude {
                    out.insert("exclude".to_string(), serde_json::json!(exclude));
                }
                Ok(serde_json::Value::Object(out))
            }
            Err(e) => Err(cheaper_err(&e, &c.needles)),
        };
        check(&name, &case["expected"], produced);
    }
}
