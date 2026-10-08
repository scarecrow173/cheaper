//! `cargo test -p cheaper-cli --test differential` entry。
//!
//! `tests/differential/cases/*.json` を列挙し、1 case = 1 回の
//! rust/ts pair 実行 + normalize 比較で回す。`#[test]` を case 毎に生成
//! できない(libtest の制約)ため、1 test が全 case を回して per-case
//! pass/divergence/fail の matrix を出力し、fail した case があれば
//! panic で test を落とす。

mod harness;
mod normalize;

use std::path::PathBuf;

use harness::{Case, CaseMode};
use serde_json::Value;

/// cases dir — `CARGO_MANIFEST_DIR/tests/differential/cases`。
fn cases_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("differential")
        .join("cases")
}

fn load_cases() -> Vec<(String, Case)> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(cases_dir())
        .expect("cases dir readable")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    entries.sort();
    for path in entries {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let mut case: Case =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        // fixture の `expect` 節は runtime fixture DSL 向けの field で
        // differential 比較とは無関係 — step 消費に影響しないが、両 binary
        // に同じ JSON を渡すため parse は維持し adapter 側で無視される。
        if let Some(mut fv) = case.fixture.take() {
            harness::strip_fixture_expect(&mut fv);
            case.fixture = Some(fv);
        }
        out.push((case.name.clone().unwrap_or(stem), case));
    }
    out
}

#[test]
fn differential_golden() {
    let cases = load_cases();
    assert!(!cases.is_empty(), "no differential cases found");

    // Rust binary — cargo が `CARGO_BIN_EXE_cheaper` を integration test
    // に注入する。明示 env も受ける(手動 binary 指定)。
    let rust_bin = std::env::var("CARGO_BIN_EXE_cheaper")
        .or_else(|_| std::env::var("CHEAPER_DIFF_RUST"))
        .map(PathBuf::from)
        .expect("rust binary path (CARGO_BIN_EXE_cheaper)");

    // TS 側前提 — node binary + `packages/cli/dist`(pnpm build 済み)。
    let node = harness::node_binary();
    let ts_entry = harness::ts_entry();
    let ts_dist = harness::repo_root().join("packages/cli/dist/cli.js");
    let ts_ready = node.is_some() && ts_entry.exists() && ts_dist.exists();
    if !ts_ready {
        eprintln!(
            "[differential] TS binary unavailable (node={:?} entry={} dist={}) — cases run rust-only (SKIP)",
            node,
            ts_entry.exists(),
            ts_dist.exists(),
        );
    }

    let mut pass = 0usize;
    let mut divergent = 0usize;
    let mut rust_only = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (name, case) in &cases {
        let env = harness::setup_case(name, case);
        let Some(pair) =
            harness::run_pair(case, &env, std::path::Path::new(&rust_bin), node.as_deref())
        else {
            rust_only += 1;
            continue;
        };

        // stdout は単一行 JSON envelope(mode=envelope)か raw bytes。
        let mut lines = compare_stdout(name, case, &env, &pair);
        // opt-in で stderr も byte 比較(hook deny は stderr が contract)。
        if case.compare_stderr {
            let r = String::from_utf8_lossy(&pair.rust.stderr);
            let t = String::from_utf8_lossy(&pair.ts.stderr);
            let r = normalize::scrub_public(&r, &harness::scrub_roots(&env.rust, &env));
            let t = normalize::scrub_public(&t, &harness::scrub_roots(&env.ts, &env));
            if r != t {
                lines.push(format!("stderr(raw):\n  rust={r:?}\n  ts={t:?}"));
            }
        }
        let mut status = Status::Pass;
        if !lines.is_empty() {
            status = Status::Fail;
        }
        if pair.rust.code != pair.ts.code {
            status = Status::Fail;
            lines.push(format!(
                "exit code: rust={:?} ts={:?}",
                pair.rust.code, pair.ts.code
            ));
        }
        if pair.rust.timed_out || pair.ts.timed_out {
            status = Status::Fail;
            lines.push(format!(
                "timeout: rust={} ts={}",
                pair.rust.timed_out, pair.ts.timed_out
            ));
        }

        match (&case.expect_divergence, status) {
            (Some(note), Status::Fail) => {
                divergent += 1;
                eprintln!("[diff] {name}: DIVERGENCE(pinned) — {note}");
                for l in &lines {
                    eprintln!("       {l}");
                }
            }
            (Some(note), Status::Pass) => {
                // pin 済み divergence が解消 — 異常ではないが report する。
                divergent += 1;
                eprintln!("[diff] {name}: divergence resolved (was: {note})");
            }
            (None, Status::Pass) => {
                pass += 1;
            }
            (None, Status::Fail) => {
                failures.push(format!("{name}:\n  {}", lines.join("\n  ")));
                eprintln!("[diff] {name}: FAIL");
                for l in &lines {
                    eprintln!("       {l}");
                }
                eprintln!("       rust: {}", harness::describe(&pair.rust));
                eprintln!("       ts:   {}", harness::describe(&pair.ts));
            }
        }
    }

    eprintln!(
        "[differential] cases={} pass={} divergence={} fail={} rust-only={}",
        cases.len(),
        pass,
        divergent,
        failures.len(),
        rust_only,
    );
    assert!(
        failures.is_empty(),
        "{} differential case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Pass,
    Fail,
}

/// stdout を mode に応じて比較し、diff 行の列を返す。
fn compare_stdout(
    name: &str,
    case: &Case,
    env: &harness::CaseEnv,
    pair: &harness::PairOut,
) -> Vec<String> {
    let mut lines = Vec::new();
    match case.mode {
        CaseMode::Raw => {
            let r = String::from_utf8_lossy(&pair.rust.stdout);
            let t = String::from_utf8_lossy(&pair.ts.stdout);
            let r = normalize::scrub_public(&r, &harness::scrub_roots(&env.rust, env));
            let t = normalize::scrub_public(&t, &harness::scrub_roots(&env.ts, env));
            if r != t {
                lines.push(format!("stdout(raw):\n  rust={r:?}\n  ts={t:?}"));
            }
        }
        CaseMode::Envelope => {
            let r: Value = match parse_single_line(&pair.rust.stdout) {
                Ok(v) => v,
                Err(e) => {
                    lines.push(format!("rust stdout not envelope: {e}"));
                    return lines;
                }
            };
            let t: Value = match parse_single_line(&pair.ts.stdout) {
                Ok(v) => v,
                Err(e) => {
                    lines.push(format!("ts stdout not envelope: {e}"));
                    return lines;
                }
            };
            let mut rn = r;
            let mut tn = t;
            normalize::normalize(&mut rn, &harness::scrub_roots(&env.rust, env));
            normalize::normalize(&mut tn, &harness::scrub_roots(&env.ts, env));
            normalize::diff(&rn, &tn, "$", &mut lines);
        }
    }
    let _ = name;
    lines
}

fn parse_single_line(stdout: &[u8]) -> Result<Value, String> {
    let text = String::from_utf8_lossy(stdout);
    let mut iter = text.lines();
    let first = iter.next().unwrap_or_default();
    if iter.next().is_some() {
        return Err(format!("multi-line stdout: {text:?}"));
    }
    serde_json::from_str(first).map_err(|e| format!("json parse: {e} (stdout={text:?})"))
}
