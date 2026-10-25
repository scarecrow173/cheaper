//! spawn env 組立(design §1.1)— v1 `benchmarkEnvironment` の Rust
//! port。ambient passthrough は PATH・SystemRoot/TEMP/TMP・
//! `CHEAPER_LOG*` allowlist のみ(v1 parity・Non-Functional
//! Constraints)。

use std::collections::BTreeMap;
use std::path::Path;

/// `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` — observability activation の
/// ambient passthrough allowlist(`CHEAPER_LOG=debug cargo run …` の
/// ambient 注入で log 有効時の overhead arm を計測できるようにする —
/// v1 `benchmarkEnvironment` parity・DESIGN-034DV9ypzmxdAobGSJ6Cm6)。
const AMBIENT_PASSTHROUGH: &[&str] = &[
    "PATH",
    "SystemRoot",
    "TEMP",
    "TMP",
    "CHEAPER_LOG",
    "CHEAPER_LOG_FORMAT",
];

/// product binary spawn へ渡す完全 env 組立て。`Command::env_clear()`
/// 前提の self-contained map(v1 `spawn(…, {env})` の全置換 semantics
/// parity)。
///
/// - `PATH` passthrough・`HOME`/`USERPROFILE` = workspace・
///   `CHEAPER_CONFIG_DIR` = observation cfgDir・
///   `CHEAPER_TEST_FAKE_STREAM` = 生成 fixture path・
///   SystemRoot/TEMP/TMP・`CHEAPER_LOG*` ambient passthrough。
/// - `NODE_ENV=test` — seam activation var として残す
///   (`fake_stream.rs` の `NODE_ENV === "test"` 規則。TS 命名の
///   vestige で rename は本決定の範囲外 — design §7/Non-Functional)。
/// - `arm_env`(scenario `armEnv`)は ambient passthrough より優先し、
///   固定 key(`HOME` 等)も上書きできる(v1 `...(options.env ?? {})`
///   の末尾 spread parity)。
pub fn build(
    workspace: &Path,
    config_dir: &Path,
    fixture_path: &Path,
    arm_env: Option<&BTreeMap<String, String>>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    for name in AMBIENT_PASSTHROUGH {
        if let Ok(value) = std::env::var(name) {
            env.insert((*name).to_string(), value);
        }
    }
    env.insert("HOME".to_string(), workspace.to_string_lossy().into_owned());
    env.insert(
        "USERPROFILE".to_string(),
        workspace.to_string_lossy().into_owned(),
    );
    env.insert("NODE_ENV".to_string(), "test".to_string());
    env.insert(
        "CHEAPER_CONFIG_DIR".to_string(),
        config_dir.to_string_lossy().into_owned(),
    );
    env.insert(
        "CHEAPER_TEST_FAKE_STREAM".to_string(),
        fixture_path.to_string_lossy().into_owned(),
    );
    if let Some(overrides) = arm_env {
        for (key, value) in overrides {
            env.insert(key.clone(), value.clone());
        }
    }
    env
}
