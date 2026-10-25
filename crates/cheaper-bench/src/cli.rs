//! CLI surface(design §1.3)— arg parse・binary 解決・preflight
//! dev-seam probe・scenario×arm×(warmups+reps) run ループ・
//! record→aggregate→report の配線(T3)。
//!
//! ```text
//! mise exec -- cargo build --release -p cheaper-cli --features dev-seam
//! mise exec -- cargo run --release -p cheaper-bench -- \
//!   --scenario <id> [--scenario <id>…] \
//!   --warmups 2 --repetitions 15 \
//!   [--output <path>] [--binary <path>] [--pricing <file>]
//! ```

use std::path::{Path, PathBuf};

use cheaper_core::errors::CheaperError;
use serde_json::{json, Map, Value};
use tempfile::TempDir;

use crate::aggregate;
use crate::arms::{self, ArmContext};
use crate::env;
use crate::fixture::bench_error;
use crate::manifest::Manifest;
use crate::measure::{self, Observation};
use crate::record;
use crate::report;
use crate::spawn::{self, SpawnOptions};
use crate::workspace;

/// scenario manifest の既定 path(repo root 相対 — §2)。
const DEFAULT_MANIFEST: &str = "benchmarks/scenarios/scenarios.json";

/// 計測対象 binary の file 名(Windows は `.exe` 付き — v1
/// `cliBinaryPath` parity)。
fn binary_file_name() -> &'static str {
    if cfg!(windows) {
        "cheaper.exe"
    } else {
        "cheaper"
    }
}

/// repo root — `env!("CARGO_MANIFEST_DIR")`(= `crates/cheaper-bench`)
/// の `../..` から導く(§1.3)。
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

/// `--scenario` 複数・`--warmups`/`--repetitions`/`--output`/
/// `--binary`/`--pricing` の parse 済み options(v1 と同意味 —
/// `--scenario` 省略時は manifest 全 scenario)。
#[derive(Debug, Clone, PartialEq)]
pub struct CliOptions {
    pub scenarios: Vec<String>,
    pub warmups: u32,
    pub repetitions: u32,
    pub output: Option<PathBuf>,
    pub binary: Option<PathBuf>,
    /// §4.3 estimated layer を有効化する opt-in pricing file。
    pub pricing: Option<PathBuf>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            scenarios: Vec::new(),
            warmups: 2,
            repetitions: 15,
            output: None,
            binary: None,
            pricing: None,
        }
    }
}

/// argv(`argv[1..]` 相当 — `--fixture-child` は `main` が既に分岐)を
/// parse する。未知 flag・値欠落・非数値は fail-fast。
pub fn parse_args(args: &[String]) -> Result<CliOptions, CheaperError> {
    let mut options = CliOptions::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |flag: &str| -> Result<String, CheaperError> {
            iter.next()
                .cloned()
                .ok_or_else(|| bench_error(format!("missing value for {flag}")))
        };
        match arg.as_str() {
            "--scenario" => options.scenarios.push(value("--scenario")?),
            "--warmups" => {
                options.warmups = value("--warmups")?
                    .parse()
                    .map_err(|_| bench_error("--warmups must be a non-negative integer"))?;
            }
            "--repetitions" => {
                options.repetitions = value("--repetitions")?
                    .parse()
                    .map_err(|_| bench_error("--repetitions must be a positive integer"))?;
            }
            "--output" => options.output = Some(PathBuf::from(value("--output")?)),
            "--binary" => options.binary = Some(PathBuf::from(value("--binary")?)),
            "--pricing" => options.pricing = Some(PathBuf::from(value("--pricing")?)),
            "--help" | "-h" => {
                return Err(bench_error(usage()));
            }
            other => {
                return Err(bench_error(format!(
                    "unknown argument: {other}\n{}",
                    usage()
                )))
            }
        }
    }
    Ok(options)
}

fn usage() -> String {
    "usage: cheaper-bench [--scenario <id>]… [--warmups <n>] [--repetitions <n>] \
     [--output <path>] [--binary <path>] [--pricing <file>]"
        .to_string()
}

/// 計測対象 binary の解決順: `--binary` 明示 → `<repo>/target/release/
/// cheaper(.exe)` → `<repo>/target/debug/cheaper(.exe)` fallback
/// (v1 `cliBinaryPath` 同等)。binary は dev-seam 有効 build が前提 —
/// seam 有無の実証は preflight probe(T2)が担う。
pub fn resolve_binary(explicit: Option<&Path>) -> Result<PathBuf, CheaperError> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        return Err(bench_error(format!(
            "--binary does not exist: {}",
            path.display()
        )));
    }
    let root = repo_root();
    for profile in ["release", "debug"] {
        let candidate = root.join("target").join(profile).join(binary_file_name());
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(bench_error(
        "cheaper binary not found; build it first: \
         cargo build --release -p cheaper-cli --features dev-seam \
         (or cargo build -p cheaper-cli --features dev-seam)",
    ))
}

/// preflight dev-seam probe(design §1.3)— minimal temp workspace +
/// bulk-reader input schema 適合 stdin `{"task":"probe"}` + bulk-reader
/// output schema 適合 finish の既定 session fixture +
/// `CHEAPER_TEST_FAKE_STREAM`/`NODE_ENV=test` 下の
/// `cheaper run bulk-reader --cwd <probe ws> --quiet` で envelope
/// `ok:true` を確認する。失敗 = binary に dev-seam が無い(seam 無し
/// binary は `CHEAPER_TEST_FAKE_STREAM` を解釈できず real provider へ
/// 向かい、credential 不在で fail する)— 診断 + build 手順を出して
/// fail-fast。
fn preflight_probe(binary: &Path) -> Result<(), CheaperError> {
    let ws = TempDir::new()
        .map_err(|e| bench_error(format!("preflight workspace create failed: {e}")))?;
    let config_dir = ws.path().join(".bench-probe-config");
    std::fs::create_dir_all(&config_dir).map_err(|e| {
        bench_error(format!(
            "preflight config dir create failed {}: {e}",
            config_dir.display()
        ))
    })?;
    let fixture_path = config_dir.join("probe-fixture.json");
    crate::fixture::write_fixture(&fixture_path, None)?;
    let probe_env = env::build(ws.path(), &config_dir, &fixture_path, None);
    let probe_args: Vec<String> = [
        "run",
        "bulk-reader",
        "--cwd",
        &ws.path().to_string_lossy(),
        "--quiet",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let spawned = spawn::run(
        binary,
        &probe_args,
        &SpawnOptions {
            cwd: ws.path(),
            env: &probe_env,
            stdin: Some(b"{\"task\":\"probe\"}".as_slice()),
        },
    )?;
    let envelope = measure::parse_envelope(&spawned.stdout);
    if measure::envelope_ok(envelope.as_ref()) {
        return Ok(());
    }
    // seam 有り binary なら deterministic な fake fixture が `ok:true`
    // を返す — ここへ来る場合は seam 無し binary(real provider への
    // credential 解決で fail)か envelope 破損のいずれか。
    Err(bench_error(format!(
        "preflight probe failed — the measured binary does not appear to be a \
         dev-seam build (exit={:?}, envelopeOk={}, stderr={}). \
         Build it with: cargo build -p cheaper-cli --features dev-seam",
        spawned.exit_code,
        measure::envelope_ok(envelope.as_ref()),
        spawned.stderr_lossy()
    )))
}

/// `git rev-parse HEAD` — 観測 record の `repositoryRevision`(git
/// 不在・非 repo では null — 採取不能を observation failure にしない)。
fn repository_revision(repo_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let revision = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!revision.is_empty()).then_some(revision)
}

/// ambient passthrough の記録 — `CHEAPER_LOG*` allowlist のうち ambient
/// env に実際に存在した名(PATH/SystemRoot/TEMP/TMP は baseline 要件で
/// 測定条件に関わらないため対象外。§3.1 `conditions` — log 有効 arm の
/// overhead 解釈に使う観測 fact)。
fn ambient_passthrough() -> Vec<String> {
    ["CHEAPER_LOG", "CHEAPER_LOG_FORMAT"]
        .iter()
        .filter(|name| std::env::var_os(name).is_some())
        .map(|name| (*name).to_string())
        .collect()
}

/// observation 1 行の進行表示(stderr — stdout は `--output` 無しでも
/// report JSON 専用にはせず v1 同様進行のみ出す)。
fn log_observation(obs: &Observation) {
    eprintln!(
        "cheaper-bench: {} / {} rep={} warmup={} wall={:.1}ms success={} exit={:?} envelopeOk={:?}",
        obs.scenario_id,
        obs.arm,
        obs.repetition,
        obs.warmup,
        obs.wall_time_ms,
        obs.success,
        obs.exit_code,
        obs.envelope_ok,
    );
}

/// harness 本体の通常経路 — parse → manifest load → binary 解決 →
/// preflight probe → run ループ → `--output` 書き出し。
/// 戻り値は `process::exit` へ渡す exit code。
pub fn run(argv: &[String]) -> i32 {
    match run_inner(argv) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("cheaper-bench: {e}");
            e.exit_code() as i32
        }
    }
}

fn run_inner(argv: &[String]) -> Result<(), CheaperError> {
    let options = parse_args(argv)?;
    let root = repo_root();
    let manifest = Manifest::load(&root.join(DEFAULT_MANIFEST))?;
    // §4.3 — pricing file は計測開始前に load する(malformed・必須
    // key 欠落・非数値・未知 key は run 前に fail-fast)。
    let pricing = options
        .pricing
        .as_deref()
        .map(aggregate::Pricing::load)
        .transpose()?;
    let binary = resolve_binary(options.binary.as_deref())?;
    let fixture_child = arms::fixture_child_exe()?;
    preflight_probe(&binary)?;
    // run-level 採取値は計測前に確定する — `ambientRustc` 必須 field
    // の採取不能を run 完了後ではなく開始前に検出する(§3.2/§3.4)。
    let environment = record::capture_environment()?;
    let run_id = uuid::Uuid::new_v4().to_string();
    let measured_binary = record::measured_binary_block(&binary, &root, "passed");
    let revision = repository_revision(&root);
    eprintln!(
        "cheaper-bench: runId={} binary={} revision={} scenarios={} warmups={} repetitions={}",
        run_id,
        binary.display(),
        revision.as_deref().unwrap_or("<unavailable>"),
        if options.scenarios.is_empty() {
            "<all>".to_string()
        } else {
            options.scenarios.join(",")
        },
        options.warmups,
        options.repetitions,
    );
    let mut selected_scenarios: Vec<String> = Vec::new();
    let mut observations: Vec<Observation> = Vec::new();
    for scenario in &manifest.scenarios {
        if !options.scenarios.is_empty() && !options.scenarios.iter().any(|s| s == &scenario.id) {
            continue;
        }
        selected_scenarios.push(scenario.id.clone());
        for arm in &scenario.arms {
            for index in 0..(options.warmups + options.repetitions) {
                let warmup = index < options.warmups;
                // repetition は v1 parity の phase 内 1-based —
                // warmup obs は 1..=warmups、計測 obs は 1..=repetitions。
                let repetition = if warmup {
                    index + 1
                } else {
                    index - options.warmups + 1
                };
                // observation 毎に fresh ws/cfgDir(§2.4 — usage.jsonl・
                // route-logs・hook-eval・fixture file の全副作用を
                // observation 単位でリセット)。
                let ws = workspace::prepare(scenario, &fixture_child)?;
                let spawn_env = env::build(
                    ws.path(),
                    ws.config_dir(),
                    ws.fixture_path(),
                    scenario.arm_env.get(arm),
                );
                let ctx = ArmContext {
                    binary: &binary,
                    workspace: &ws,
                    env: &spawn_env,
                    scenario,
                    fixture_child_exe: &fixture_child,
                    repo_root: &root,
                };
                let observation = arms::run_observation(&ctx, arm, repetition, warmup)?;
                log_observation(&observation);
                observations.push(observation);
            }
        }
    }
    // observation → schema v2 record → aggregate → result 組立て →
    // §3.4 invariant validate → create_new 書込み(§3.5 — 既定
    // `benchmarks/results/<runId>.json`、`--output` 明示も非上書き)。
    let run_ctx = record::RunContext {
        run_id,
        fixture_revision: manifest.fixture_revision.clone(),
        environment,
    };
    let records: Vec<Value> = observations
        .iter()
        .map(|obs| record::observation_record(obs, &run_ctx))
        .collect();
    let aggregated = aggregate::aggregate(&records, &manifest.scenarios, pricing.as_ref())?;
    let mut conditions = Map::new();
    conditions.insert("warmupCount".to_string(), json!(options.warmups));
    conditions.insert("repetitionCount".to_string(), json!(options.repetitions));
    conditions.insert("selectedScenarios".to_string(), json!(selected_scenarios));
    conditions.insert(
        "ambientPassthrough".to_string(),
        json!(ambient_passthrough()),
    );
    let created_at = record::created_at_iso8601();
    let result = report::assemble_result(
        &report::RunMeta {
            run_id: &run_ctx.run_id,
            created_at: &created_at,
            fixture_revision: &run_ctx.fixture_revision,
            repository_revision: revision.as_deref(),
            conditions,
            measured_binary,
            environment: run_ctx.environment.clone(),
            pricing: pricing.as_ref().map(aggregate::Pricing::to_json),
        },
        records,
        aggregated,
    );
    // emit 前 invariant — violation は harness 側の不具合として
    // fail(record は書かない。§3.4 fail-fast)。
    record::validate_result(&result)?;
    let output = options
        .output
        .clone()
        .unwrap_or_else(|| report::default_result_path(&root, &run_ctx.run_id));
    report::write_result(&output, &result)?;
    eprintln!(
        "cheaper-bench: wrote {} observations to {}",
        observations.len(),
        output.display()
    );
    // §3.3 — `armExpect` 不一致・fallback 失敗を持つ observation は
    // `success:false` で記録された上で、run 終了時に scenario 単位で
    // 失敗報告する(deterministic fixture の回帰 anchor — mismatch を
    // 黙って exit 0 にしない)。
    let failures: Vec<&Observation> = observations.iter().filter(|o| !o.success).collect();
    if !failures.is_empty() {
        let mut detail = String::new();
        for obs in &failures {
            detail.push_str(&format!(
                "\n  {}/{}/rep{} warmup={} exit={:?} envelopeOk={:?} errorCode={:?}",
                obs.scenario_id,
                obs.arm,
                obs.repetition,
                obs.warmup,
                obs.exit_code,
                obs.envelope_ok,
                obs.error_code,
            ));
            for mismatch in &obs.expect_mismatches {
                detail.push_str(&format!("\n    mismatch: {mismatch}"));
            }
        }
        return Err(bench_error(format!(
            "{} observation(s) did not match their expected shape:{detail}",
            failures.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_defaults() {
        let options = parse_args(&[]).unwrap();
        assert_eq!(options, CliOptions::default());
        assert_eq!(options.warmups, 2);
        assert_eq!(options.repetitions, 15);
    }

    #[test]
    fn parse_all_flags() {
        let options = parse_args(&args(&[
            "--scenario",
            "a",
            "--scenario",
            "b",
            "--warmups",
            "1",
            "--repetitions",
            "3",
            "--output",
            "out.json",
            "--binary",
            "/x/cheaper",
            "--pricing",
            "p.json",
        ]))
        .unwrap();
        assert_eq!(options.scenarios, vec!["a", "b"]);
        assert_eq!(options.warmups, 1);
        assert_eq!(options.repetitions, 3);
        assert_eq!(options.output.unwrap(), PathBuf::from("out.json"));
        assert_eq!(options.binary.unwrap(), PathBuf::from("/x/cheaper"));
        assert_eq!(options.pricing.unwrap(), PathBuf::from("p.json"));
    }

    #[test]
    fn parse_rejects_unknown_and_missing() {
        assert!(parse_args(&args(&["--bogus"])).is_err());
        assert!(parse_args(&args(&["--scenario"])).is_err());
        assert!(parse_args(&args(&["--warmups", "x"])).is_err());
    }

    #[test]
    fn resolve_binary_missing_explicit_errors() {
        let e = resolve_binary(Some(Path::new("/nonexistent/cheaper"))).unwrap_err();
        assert!(e.message().contains("--binary"));
    }
}
