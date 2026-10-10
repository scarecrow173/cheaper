use std::collections::BTreeMap;
use std::sync::Arc;

use cheaper_provider_devin::cli::{CliRunOptions, DevinCli, DevinCliResult, DevinCliRunner};
use cheaper_provider_devin::models::*;

/// argv/env を記録する fake runner。
struct FakeRunner {
    result: DevinCliResult,
    calls: std::sync::Mutex<Vec<Vec<String>>>,
}

impl DevinCliRunner for FakeRunner {
    fn run(
        &self,
        _bin: &str,
        args: &[String],
        _opts: &CliRunOptions<'_>,
    ) -> Result<DevinCliResult, std::io::Error> {
        self.calls.lock().unwrap().push(args.to_vec());
        Ok(self.result.clone())
    }
}

fn cli(result: DevinCliResult) -> (DevinCli, Arc<FakeRunner>) {
    let runner = Arc::new(FakeRunner {
        result,
        calls: std::sync::Mutex::new(Vec::new()),
    });
    (
        DevinCli::new(
            BTreeMap::new(),
            std::path::Path::new("data"),
            Some(runner.clone()),
        ),
        runner,
    )
}

fn ok(stdout: &str) -> DevinCliResult {
    DevinCliResult {
        stdout: stdout.to_string(),
        stderr: String::new(),
        code: 0,
    }
}

#[test]
fn cli_catalog_uses_exact_argv() {
    let (cli, runner) = cli(ok(r#"{"families":[]}"#));
    load_cli_catalog(&cli).unwrap();
    let calls = runner.calls.lock().unwrap();
    assert_eq!(
        calls[0],
        ["models", "list", "--format", "json"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn cli_catalog_nonzero_exit_includes_stderr() {
    let (cli, _) = cli(DevinCliResult {
        stdout: String::new(),
        stderr: "auth required".to_string(),
        code: 3,
    });
    let err = load_cli_catalog(&cli).unwrap_err();
    assert!(err.to_string().contains("auth required"));
}

#[test]
fn cli_catalog_malformed_json_is_actionable() {
    let (c1, _) = cli(ok("not json"));
    let err = load_cli_catalog(&c1).unwrap_err();
    assert!(err.to_string().contains("malformed JSON"));
    let (c2, _) = cli(ok(r#"{"other":1}"#));
    // `families` 必須 — 欠落は deserialize error。
    assert!(load_cli_catalog(&c2).is_err());
}

fn catalog_json(variants: &[&str]) -> String {
    let variants: Vec<String> = variants
        .iter()
        .map(|uid| {
            format!(
                r#"{{"model_uid":"{uid}","label":"{uid}","cost_summary":"$5.00 / MTok In · $25.00 / MTok Out"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"families":[{{"family_label":"Fam","family_uid":"fam","slug":"fam","variants":[{}]}}]}}"#,
        variants.join(",")
    )
}

#[test]
fn family_conversion_filters_priority_fast_and_thinking_1m() {
    let catalog: DevinCatalog = serde_json::from_str(&catalog_json(&[
        "fam-low",
        "fam-high",
        "fam-high-priority",
        "fam-high-fast",
        "fam-thinking-1m",
    ]))
    .unwrap();
    let models = models_from_catalog(Some(&catalog));
    assert_eq!(models.len(), 1);
    let model = &models[0];
    // reasoning family → Pi-facing id は family slug。
    assert_eq!(model.id, "fam");
    assert_eq!(model.reasoning, Some(true));
    let map = model.thinking_level_map.as_ref().unwrap();
    assert_eq!(map.low.as_ref().unwrap().as_deref(), Some("fam-low"));
    assert_eq!(map.high.as_ref().unwrap().as_deref(), Some("fam-high"));
    // 未出荷 level は明示 null。
    assert_eq!(map.max, Some(None));
    // cost summary parse。
    let cost = model.cost.as_ref().unwrap();
    assert_eq!(cost.input, 5.0);
    assert_eq!(cost.output, 25.0);
    assert!((cost.cache_read - 0.5).abs() < 1e-9);
    assert!((cost.cache_write - 6.25).abs() < 1e-9);
}

#[test]
fn family_conversion_falls_back_when_all_variants_filtered() {
    let catalog: DevinCatalog =
        serde_json::from_str(&catalog_json(&["fam-high-priority", "fam-thinking-1m"])).unwrap();
    let models = models_from_catalog(Some(&catalog));
    assert_eq!(models.len(), 1);
    // 全除外時は全 variant を対象に戻す → `high-priority`→`high`、
    // `thinking-1m`→`high` が map される。mapped level は 1 種のみなので
    // non-reasoning、Pi-facing id は preferred default（high の uid）。
    assert_eq!(models[0].reasoning, Some(false));
    assert_eq!(models[0].id, "fam-high-priority");
}

#[test]
fn single_variant_family_is_non_reasoning() {
    let catalog: DevinCatalog = serde_json::from_str(&catalog_json(&["fam-only"])).unwrap();
    let models = models_from_catalog(Some(&catalog));
    assert_eq!(models[0].reasoning, Some(false));
    assert!(models[0].thinking_level_map.is_none());
    assert_eq!(models[0].id, "fam-only");
}

#[test]
fn family_missing_label_uid_slug_uses_ts_fallbacks() {
    // TS `||` fallback — `family_label`/`family_uid`/`slug` 欠損は
    // catalog 全体を fail させず degraded model になる。
    let catalog: DevinCatalog =
        serde_json::from_str(r#"{"families":[{"variants":[{"model_uid":"fam-only"}]}]}"#).unwrap();
    let models = models_from_catalog(Some(&catalog));
    assert_eq!(models.len(), 1);
    // non-reasoning（mapped level は high のみ）→ id は default uid。
    assert_eq!(models[0].id, "fam-only");
    // name: label / slug 欠損 → default uid fallback。
    assert_eq!(models[0].name.as_deref(), Some("fam-only"));
}

#[test]
fn reasoning_family_missing_slug_and_uid_falls_back_to_default_uid() {
    // reasoning family: `family.slug || family.family_uid || defaultUid`
    // — slug/uid 欠損時は preferred default（high）の uid が
    // Pi-facing id になる。
    let catalog: DevinCatalog = serde_json::from_str(
        r#"{"families":[{"variants":[{"model_uid":"x-low"},{"model_uid":"x-high"}]}]}"#,
    )
    .unwrap();
    let models = models_from_catalog(Some(&catalog));
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].reasoning, Some(true));
    assert_eq!(models[0].id, "x-high");
    assert_eq!(models[0].name.as_deref(), Some("x-high"));
}

#[test]
fn empty_catalog_falls_back_to_static_models() {
    let catalog: DevinCatalog = serde_json::from_str(r#"{"families":[]}"#).unwrap();
    assert_eq!(models_from_catalog(Some(&catalog)), fallback_models());
    assert_eq!(models_from_catalog(None), fallback_models());
    assert!(!fallback_models().is_empty());
}

#[test]
fn resolve_model_uid_prefers_requested_then_default_then_id() {
    let catalog: DevinCatalog =
        serde_json::from_str(&catalog_json(&["fam-low", "fam-high"])).unwrap();
    let model = &models_from_catalog(Some(&catalog))[0];
    let map = model.thinking_level_map.as_ref();
    assert_eq!(
        resolve_model_uid("fam", map, Some("low")),
        "fam-low".to_string()
    );
    // 未対応 level 要求 → preferred default（high）。
    assert_eq!(
        resolve_model_uid("fam", map, Some("xhigh")),
        "fam-high".to_string()
    );
    // map 無し → model id そのまま。
    assert_eq!(
        resolve_model_uid("plain", None, Some("high")),
        "plain".to_string()
    );
}
