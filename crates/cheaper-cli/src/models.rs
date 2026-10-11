//! `executeModels`(`packages/cli/src/cli.ts`)の command module(T3 本体)。
//!
//! `cheaper models`(issue #45 / #46): runtime の available catalog を
//! 列挙し、各 model に exclude・effective pricing・classification を
//! 適用した一覧を `result.models[]` で返す。auto 分類は `--worker`
//! の workload profile(未指定は `DEFAULT_OUTPUT_PER_INPUT`)を基準に
//! eligible(exclude 後)母集団の相対 ranking で行う — 母集団には
//! provider filter 後ではなく catalog 全体の eligible が入る
//! (TS `buildAutoTierContext(available.filter(!excluded))` と同一)。
//! worker run を伴わないため utility envelope を使い、usage record
//! も書かない。

use std::path::Path;

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirsOptions};
use cheaper_core::model::classify::{
    build_auto_tier_context, classify_model, ClassifyOptions, ModelInfo,
};
use cheaper_core::model::config::{load_model_config, LoadModelConfigOptions};
use cheaper_core::model::pricing::{resolve_pricing, ModelCost};
use cheaper_core::model::resolve::is_excluded_model;
use cheaper_core::workspace::paths::canonicalize_workspace_root;
use cheaper_runtime::catalog::CatalogModel;
use serde_json::{json, Map, Value};
use tracing::Instrument;

use crate::args::ModelsCliArgs;
use crate::cli::{
    emit_catalog_warnings, now, resolve_builtin_dir, BoxError, CliHooks, CliIo, CommandOut,
};
use crate::envelope::utility_success_envelope;

/// `toModelInfo` 相当 — catalog model を分類/価格層の入力 shape へ。
fn to_model_info(m: &CatalogModel) -> ModelInfo {
    ModelInfo {
        provider: m.provider.clone(),
        id: m.id.clone(),
        cost: Some(ModelCost {
            input: m.cost.input,
            output: m.cost.output,
            cache_read: Some(m.cost.cache_read),
            cache_write: Some(m.cost.cache_write),
        }),
        max_tokens: Some(m.max_tokens as u64),
    }
}

/// `execute` の entry — `cheaper.cli.command` span(design
/// §instrumentation 配置)。
pub async fn execute(
    args: &ModelsCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    execute_inner(args, io, hooks, started_at)
        .instrument(tracing::info_span!(
            "cheaper.cli.command",
            command = "models",
        ))
        .await
}

async fn execute_inner(
    args: &ModelsCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    let builtin_dir = resolve_builtin_dir(hooks)?;
    let root = canonicalize_workspace_root(args.cwd.as_deref().unwrap_or(&io.cwd))?;
    let dirs = resolve_config_dirs(&ConfigDirsOptions {
        cwd: Path::new(&root),
        builtin_dir: &builtin_dir,
        env: Some(io.env),
        homedir: None,
    });
    let ucd = dirs.user_config_dir.to_string_lossy();
    let pdir = dirs.project_dir.to_string_lossy();

    let model_config = load_model_config(&LoadModelConfigOptions {
        user_config_dir: &ucd,
        project_dir: &pdir,
    })?;

    // `--worker <name>` — workload profile の outputPerInput を取る
    // (fixture fallback はしない — models は fake-stream 非対象)。
    let output_per_input = match &args.worker {
        Some(name) => crate::run::resolve_named_worker(
            name,
            &dirs,
            #[cfg(feature = "dev-seam")]
            &root,
            #[cfg(feature = "dev-seam")]
            None,
        )?
        .cheaper
        .model
        .workload
        .map(|w| w.output_per_input),
        None => None,
    };

    let runtime = crate::run::create_runtime(
        &dirs.user_config_dir,
        #[cfg(feature = "dev-seam")]
        None,
    )?;
    // spec Req 4 — catalog error を fallible consumer(`get_available`
    // の `?`)より先に `warning:` 行として stderr へ出す。
    emit_catalog_warnings(
        &runtime.catalog().errors(),
        &mut |s: &str| io.stderr.write_str(s),
        args.quiet,
    );
    let available = runtime.get_available(None)?;

    let infos: Vec<ModelInfo> = available.iter().map(to_model_info).collect();
    // ranking 母集団は exclude 適用後の eligible model。excluded 行の
    // 分類は表示用で、母集団自体には入れない(issue #46)。
    let eligible: Vec<ModelInfo> = infos
        .iter()
        .filter(|m| !is_excluded_model(m, &model_config))
        .cloned()
        .collect();
    let auto_context = build_auto_tier_context(&eligible, output_per_input);

    let mut models: Vec<Value> = Vec::new();
    for info in infos.iter() {
        if let Some(p) = &args.provider {
            if &info.provider != p {
                continue;
            }
        }
        let pricing = resolve_pricing(info, None);
        let classification = classify_model(
            info,
            &model_config,
            &ClassifyOptions {
                pricing: pricing.clone(),
                auto_context: Some(&auto_context),
            },
        );
        if let Some(tier) = args.tier {
            if classification.tier != tier {
                continue;
            }
        }
        // key 順は TS object literal の構築順(provider → id →
        // excluded → tier → tierSource → [auto fields] → pricing)。
        let mut entry = Map::new();
        entry.insert("provider".to_string(), json!(info.provider));
        entry.insert("id".to_string(), json!(info.id));
        entry.insert(
            "excluded".to_string(),
            Value::Bool(is_excluded_model(info, &model_config)),
        );
        entry.insert("tier".to_string(), json!(classification.tier.as_str()));
        entry.insert(
            "tierSource".to_string(),
            json!(classification.source.as_str()),
        );
        if let Some(auto) = &classification.auto {
            entry.insert(
                "effectiveCost".to_string(),
                Value::from(auto.effective_cost),
            );
            entry.insert(
                "effectiveCostPercentile".to_string(),
                Value::from(auto.effective_cost_percentile),
            );
            entry.insert(
                "workload".to_string(),
                json!({ "outputPerInput": auto.output_per_input }),
            );
        }
        entry.insert(
            "pricing".to_string(),
            match &pricing {
                Some(p) => serde_json::to_value(p).unwrap_or(Value::Null),
                None => Value::Null,
            },
        );
        models.push(Value::Object(entry));
    }

    let mut result = Map::new();
    result.insert("models".to_string(), Value::Array(models));
    let duration_ms = now(hooks) - started_at;
    Ok(CommandOut {
        envelope: utility_success_envelope(result, duration_ms),
        exit_code: 0,
        model_line: None,
    })
}
