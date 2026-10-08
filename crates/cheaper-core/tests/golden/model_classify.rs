use cheaper_core::model::classify::classify_models;
use cheaper_core::model::exceptions::{
    find_builtin_model_exception, find_model_exception, ModelException,
};
use cheaper_core::model::model_ref::match_model_pattern;
use cheaper_core::model::pricing::resolve_pricing;
use cheaper_core::model::tier::{compare_tier, is_tier, next_tier, parse_tier, TIERS};

use crate::common::*;

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

#[test]
fn classify_cases() {
    let f = fixture("model/classify.json");
    for case in cases(&f) {
        let name = case_name(case);
        let op = case["op"].as_str().unwrap();
        let input = &case["input"];
        let produced: Result<serde_json::Value, serde_json::Value> = Ok(match op {
            "tiers" => serde_json::json!({
                "tiers": TIERS.iter().map(|t| t.as_str()).collect::<Vec<_>>()
            }),
            "compareTier" => {
                let a = parse_tier(&input[0]).unwrap();
                let b = parse_tier(&input[1]).unwrap();
                serde_json::json!(compare_tier(a, b))
            }
            "isTier" => serde_json::json!(is_tier(input)),
            "nextTier" => {
                let t = parse_tier(input).unwrap();
                serde_json::json!(next_tier(t).map(|t| t.as_str()))
            }
            "classify" => {
                let models = models_from(&input["models"]);
                let config = model_config_from(&input["config"]);
                let opi = input["outputPerInput"].as_f64();
                let out = classify_models(&models, &config, opi);
                serde_json::json!(out.iter().map(dump_classification).collect::<Vec<_>>())
            }
            "resolvePricing" => {
                let model = model_from(&input["model"]);
                let snapshot = pricing_snapshot_from(&input["snapshot"]);
                serde_json::json!({
                    "pricing": resolve_pricing(&model, snapshot.as_ref())
                        .map(|p| dump_pricing(&p))
                        .unwrap_or(serde_json::Value::Null)
                })
            }
            "findModelException" => {
                let provider = input["provider"].as_str().unwrap();
                let id = input["id"].as_str().unwrap();
                let found: Option<&ModelException> = if input["builtin"].as_bool().unwrap() {
                    find_builtin_model_exception(provider, id)
                } else {
                    let table: &'static [(&'static str, ModelException)] = Box::leak(Box::new([
                        (
                            "p/exact",
                            ModelException {
                                tier: cheaper_core::model::tier::Tier::High,
                                reason: leak("r"),
                                source: leak("s"),
                                checked_at: leak("2026-09-19"),
                            },
                        ),
                        (
                            "p/*",
                            ModelException {
                                tier: cheaper_core::model::tier::Tier::Low,
                                reason: leak("r"),
                                source: leak("s"),
                                checked_at: leak("2026-09-19"),
                            },
                        ),
                    ]));
                    find_model_exception(provider, id, table)
                };
                found
                    .map(|e| {
                        serde_json::json!({
                            "tier": e.tier.as_str(),
                            "reason": e.reason,
                            "source": e.source,
                            "checkedAt": e.checked_at,
                        })
                    })
                    .unwrap_or(serde_json::Value::Null)
            }
            "matchModelPattern" => serde_json::json!(match_model_pattern(
                input[0].as_str().unwrap(),
                input[1].as_str().unwrap(),
                input[2].as_str().unwrap(),
            )),
            other => panic!("unknown op {other} in {name}"),
        });
        check(&name, &case["expected"], produced);
    }
}
