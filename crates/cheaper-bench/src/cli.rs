//! CLI surface(design §1.3)— arg parse・binary 解決・
//! scenario×arm×(warmups+reps) run ループ骨格。
//!
//! **T1 scope は構造のみ** — arm dispatch・preflight probe・
//! record/aggregate は T2/T3 が接続する。run ループは未接続 arm を
//! 「未実装」の明示 error で fail-fast させる骨格まで。
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

use crate::fixture::bench_error;
use crate::manifest::Manifest;

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

/// scenario×arm×(warmups+repetitions) の run ループ骨格 — T2 が arm
/// dispatch を接続するまで、observation は「未実装」の明示 error で
/// fail-fast する(構造のみの T1 contract)。
fn run_observation(
    scenario_id: &str,
    arm: &str,
    repetition: u32,
    warmup: bool,
) -> Result<(), CheaperError> {
    let _ = (scenario_id, arm, repetition, warmup);
    Err(bench_error(format!(
        "arm dispatch is not implemented yet (T2): scenario={scenario_id} arm={arm}"
    )))
}

/// harness 本体の通常経路 — parse → manifest load → binary 解決 →
/// run ループ骨格。preflight probe・record/aggregate は T2/T3。
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
    let manifest = Manifest::load(&repo_root().join(DEFAULT_MANIFEST))?;
    let binary = resolve_binary(options.binary.as_deref())?;
    eprintln!(
        "cheaper-bench: binary={} scenarios={} warmups={} repetitions={}",
        binary.display(),
        if options.scenarios.is_empty() {
            "<all>".to_string()
        } else {
            options.scenarios.join(",")
        },
        options.warmups,
        options.repetitions,
    );
    for scenario in &manifest.scenarios {
        if !options.scenarios.is_empty() && !options.scenarios.iter().any(|s| s == &scenario.id) {
            continue;
        }
        for arm in &scenario.arms {
            for index in 0..(options.warmups + options.repetitions) {
                run_observation(&scenario.id, arm, index, index < options.warmups)?;
            }
        }
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
