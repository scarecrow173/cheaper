//! `cargo test -p cheaper-cli --test regression` entry。
//!
//! `tests/regression/cases/*.json` を列挙し、1 case = 1 回の Rust binary
//! replay + normalize 後、`expected/<case>.json` の固定 snapshot と比較
//! する。`#[test]` を case 毎に生成できない(libtest の制約)ため、1 test
//! が全 case を回して per-case pass/fail matrix を出力し、fail した
//! case があれば panic で test を落とす。
//!
//! snapshot は differential harness(TS oracle)撤去直前の最終 green 状態
//! から capture した Rust 側 normalized 出力。意図的な挙動変更時は
//! `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --test
//! regression` で再 capture し、`expected/` の diff を review する。

mod harness;
mod normalize;

use std::path::{Path, PathBuf};
use std::time::Duration;

use harness::{Case, CaseMode};
use serde_json::{json, Value};

/// cases dir — `CARGO_MANIFEST_DIR/tests/regression/cases`。
fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("regression")
        .join("cases")
}

fn expected_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("regression")
        .join("expected")
}

fn load_cases() -> Vec<(String, Case, PathBuf)> {
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
        // replay 比較とは無関係 — 両 binary へ同じ JSON を渡していた
        // differential 時代の名残で strip を維持する。
        if let Some(mut fv) = case.fixture.take() {
            harness::strip_fixture_expect(&mut fv);
            case.fixture = Some(fv);
        }
        out.push((case.name.clone().unwrap_or(stem), case, path));
    }
    out
}

/// 1 case 分の replay 結果を normalize した snapshot Value。
fn snapshot_case(case: &Case, env: &harness::CaseEnv, out: &harness::ProcOut) -> Value {
    let scrub = harness::scrub_roots(&env.side, env);
    let stdout = match case.mode {
        CaseMode::Envelope => match parse_single_line(&out.stdout) {
            Ok(mut v) => {
                normalize::normalize(&mut v, &scrub);
                v
            }
            // envelope 非適合も観測値として固定する(capture 時に review)。
            Err(e) => json!({ "__nonEnvelope": e, "stdout": String::from_utf8_lossy(&out.stdout) }),
        },
        CaseMode::Raw => Value::String(normalize::scrub_public(
            &String::from_utf8_lossy(&out.stdout),
            &scrub,
        )),
    };
    let mut snap = json!({
        "code": out.code,
        "timedOut": out.timed_out,
        "stdout": stdout,
    });
    if case.compare_stderr {
        snap["stderr"] = Value::String(normalize::scrub_public(
            &String::from_utf8_lossy(&out.stderr),
            &scrub,
        ));
    }
    snap
}

/// snapshot と replay 結果の差分行を返す(空 = 一致)。
fn diff_snapshot(expected: &Value, actual: &Value, case: &Case) -> Vec<String> {
    let mut lines = Vec::new();
    if expected.get("code") != actual.get("code") {
        lines.push(format!(
            "exit code: expected={} actual={}",
            expected["code"], actual["code"]
        ));
    }
    if expected.get("timedOut") != actual.get("timedOut") {
        lines.push(format!(
            "timedOut: expected={} actual={}",
            expected["timedOut"], actual["timedOut"]
        ));
    }
    match case.mode {
        CaseMode::Envelope => {
            normalize::diff(
                &expected["stdout"],
                &actual["stdout"],
                "$.stdout",
                &mut lines,
            );
        }
        CaseMode::Raw => {
            if expected["stdout"] != actual["stdout"] {
                lines.push(format!(
                    "stdout(raw):\n  expected={:?}\n  actual={:?}",
                    expected["stdout"].as_str().unwrap_or_default(),
                    actual["stdout"].as_str().unwrap_or_default()
                ));
            }
        }
    }
    if case.compare_stderr && expected.get("stderr") != actual.get("stderr") {
        lines.push(format!(
            "stderr:\n  expected={:?}\n  actual={:?}",
            expected["stderr"].as_str().unwrap_or_default(),
            actual["stderr"].as_str().unwrap_or_default()
        ));
    }
    lines
}

#[test]
fn regression_golden() {
    let cases = load_cases();
    assert!(!cases.is_empty(), "no regression cases found");

    // Rust binary — cargo が `CARGO_BIN_EXE_cheaper` を integration test
    // に注入する。明示 env も受ける(手動 binary 指定)。
    let rust_bin = std::env::var("CARGO_BIN_EXE_cheaper")
        .or_else(|_| std::env::var("CHEAPER_REGRESSION_BIN"))
        .map(PathBuf::from)
        .expect("rust binary path (CARGO_BIN_EXE_cheaper)");

    let capture = std::env::var_os("CHEAPER_REGRESSION_CAPTURE").is_some();
    let expected_dir = expected_dir();
    if capture {
        std::fs::create_dir_all(&expected_dir).expect("expected dir writable");
    }

    let mut pass = 0usize;
    let mut captured = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (name, case, case_path) in &cases {
        let env = harness::setup_case(name, case);
        let timeout = Duration::from_millis(case.timeout_ms.unwrap_or(120_000));
        let out = harness::spawn_collect(
            &rust_bin,
            &harness::case_argv(case),
            &env.side.ws,
            &harness::run_env(&env),
            &harness::case_stdin(case),
            timeout,
        );
        let actual = snapshot_case(case, &env, &out);
        let snap_path = expected_dir.join(format!(
            "{}.json",
            case_path.file_stem().unwrap().to_string_lossy()
        ));

        if capture {
            let mut text = serde_json::to_string_pretty(&actual).unwrap();
            text.push('\n');
            std::fs::write(&snap_path, text)
                .unwrap_or_else(|e| panic!("write {}: {e}", snap_path.display()));
            captured += 1;
            continue;
        }

        let text = match std::fs::read_to_string(&snap_path) {
            Ok(t) => t,
            Err(e) => {
                failures.push(format!(
                    "{name}: snapshot missing ({e}) — run with CHEAPER_REGRESSION_CAPTURE=1"
                ));
                continue;
            }
        };
        let expected: Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("parse {}: {e}", snap_path.display()));
        let lines = diff_snapshot(&expected, &actual, case);
        if lines.is_empty() {
            pass += 1;
        } else {
            failures.push(format!("{name}:\n  {}", lines.join("\n  ")));
            eprintln!("[regr] {name}: FAIL");
            for l in &lines {
                eprintln!("       {l}");
            }
            eprintln!("       actual: {}", harness::describe(&out));
        }
    }

    if capture {
        eprintln!(
            "[regression] captured {captured} snapshot(s) -> {}",
            expected_dir.display()
        );
    } else {
        eprintln!(
            "[regression] cases={} pass={} fail={}",
            cases.len(),
            pass,
            failures.len(),
        );
    }
    assert!(
        failures.is_empty(),
        "{} regression case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
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
