//! result record schema v2 — run-level 採取値・observation record
//! 投影・design §3.4 の 6 invariant を fail-fast で強制する
//! validation。
//!
//! - run-level field(`runId`・`parentUsage`・`fixtureRevision`・
//!   `environment`)は [`RunContext`] が run 毎に 1 度採取し、全
//!   observation record へ同じ値を投影する(§3.1/§3.3)。
//! - `environment` block は §3.2 — `os`/`osRelease`/`arch`/
//!   `cpuCount`/`ambientRustc`/`harnessProfile`。v1 の
//!   `environment.node` は削除済みで、本 module の validator が
//!   再混入を reject する。
//! - [`validate_result`] は組立て済み result JSON(schemaVersion・
//!   parentUsage・usage.source・environment・comparisons claim・
//!   数値 parent-usage field)を emit 前に検証する — invariant
//!   違反の record は file へ書かない。

use std::path::Path;

use cheaper_core::errors::CheaperError;
use serde_json::{json, Map, Value};

use crate::fixture::bench_error;
use crate::measure::Observation;

/// `parentUsage` block — v1 parity の unavailable 固定値(親 agent
/// telemetry は harness へ露出しない。`observed` 配線は取得経路が
/// できた時点の contract 拡張 — design Deferred Design Concerns)。
pub fn unavailable_parent_usage() -> Value {
    json!({
        "status": "unavailable",
        "reason": "no parent-agent telemetry is exposed to the benchmark runner",
    })
}

/// run 1 回分の共有 record 値 — `observation_record` が全
/// observation へ同じ `runId`・`fixtureRevision`・`environment` を
/// 投影する。
pub struct RunContext {
    /// UUIDv4(`benchmarks/results/<runId>.json` の file 名にもなる)。
    pub run_id: String,
    /// manifest の `fixtureRevision`(`"seeded-v2"`)。
    pub fixture_revision: String,
    /// §3.2 environment block(result top-level と各 observation の
    /// 両方へ同じ block を載せる)。
    pub environment: Map<String, Value>,
}

/// observation 1 件分の正式 record(§3.3)— `Observation::to_json`
/// の観測 local 投影へ run-level field を canonical field 順で挿入
/// する。armExpect 照合は引き続き `to_json`(観測 local 形)を参照
/// するため、expect 語彙に run-level field は見えない。
pub fn observation_record(obs: &Observation, run: &RunContext) -> Value {
    let local = obs.to_json();
    let mut record = Map::new();
    record.insert("schemaVersion".to_string(), json!(2));
    record.insert("runId".to_string(), json!(run.run_id));
    for key in [
        "scenarioId",
        "scenarioTitle",
        "arm",
        "repetition",
        "warmup",
        "mode",
    ] {
        record.insert(
            key.to_string(),
            local.get(key).cloned().unwrap_or(Value::Null),
        );
    }
    record.insert("parentUsage".to_string(), unavailable_parent_usage());
    record.insert("fixtureRevision".to_string(), json!(run.fixture_revision));
    record.insert(
        "environment".to_string(),
        Value::Object(run.environment.clone()),
    );
    const RUN_INSERTED: &[&str] = &[
        "schemaVersion",
        "scenarioId",
        "scenarioTitle",
        "arm",
        "repetition",
        "warmup",
        "mode",
    ];
    if let Value::Object(map) = &local {
        for (key, value) in map {
            if !RUN_INSERTED.contains(&key.as_str()) {
                record.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(record)
}

// ---------------------------------------------------------------------------
// §3.2 environment 採取(ambient 由来 — dev tool 内の spawn 許容)
// ---------------------------------------------------------------------------

/// `environment` block(§3.2)— v1 `environment.node` の後続。
/// `ambientRustc` は必須 field(invariant #4)のため、採取不能は
/// 計測開始前に fail-fast する(run 完了後に record が書けない形で
/// 気づくのを避ける)。
pub fn capture_environment() -> Result<Map<String, Value>, CheaperError> {
    let mut env = Map::new();
    env.insert("os".to_string(), json!(std::env::consts::OS));
    env.insert(
        "osRelease".to_string(),
        os_release().map_or(Value::Null, |v| json!(v)),
    );
    env.insert("arch".to_string(), json!(std::env::consts::ARCH));
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get() as u64)
        .unwrap_or(1);
    env.insert("cpuCount".to_string(), json!(cpu_count));
    env.insert("ambientRustc".to_string(), json!(ambient_rustc()?));
    env.insert("harnessProfile".to_string(), json!(harness_profile()));
    Ok(env)
}

/// bench bin 自身の profile(`cfg!(debug_assertions)` 由来 —
/// debug build した harness の spawn-loop overhead を結果へ
/// 可視化する。§3.2)。
fn harness_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// ambient `rustc --version` の先頭行 trim(§3.2 — mise pin が
/// ambient PATH に載る前提の `mise exec` 経由実行。計測対象 binary
/// を build した toolchain の保証ではなく、あくまで ambient の記録)。
/// 必須 field なので spawn 失敗・非 0 exit・空出力は fail-fast。
fn ambient_rustc() -> Result<String, CheaperError> {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|e| {
            bench_error(format!(
                "environment.ambientRustc is required but `rustc --version` failed to spawn: {e} \
                 (run via `mise exec` so the managed toolchain is on PATH)"
            ))
        })?;
    if !output.status.success() {
        return Err(bench_error(format!(
            "environment.ambientRustc is required but `rustc --version` exited {}",
            output.status
        )));
    }
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if line.is_empty() {
        return Err(bench_error(
            "environment.ambientRustc is required but `rustc --version` produced no output",
        ));
    }
    Ok(line)
}

/// platform 別 best-effort osRelease(§3.2 — Windows: `cmd /c ver`
/// の番号抽出 / Unix: `uname -sr`。失敗時 `None` → record では
/// null。spawn は dev tool 内の ambient 許容)。
fn os_release() -> Option<String> {
    #[cfg(windows)]
    {
        let output = std::process::Command::new("cmd")
            .args(["/c", "ver"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        // "Microsoft Windows [Version 10.0.26200.xxxx]" 形から
        // 数字+`.` の version token を抽出する。
        let start = text.find(|c: char| c.is_ascii_digit())?;
        let tail = &text[start..];
        let end = tail
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .map(|i| start + i)
            .unwrap_or(text.len());
        let version = text[start..end].trim_end_matches('.');
        (!version.is_empty()).then(|| version.to_string())
    }
    #[cfg(not(windows))]
    {
        let output = std::process::Command::new("uname")
            .arg("-sr")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let release = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!release.is_empty()).then_some(release)
    }
}

/// §3.1 `measuredBinary` block — repo 相対 path・`profile`(解決先
/// dir 名 `release|debug`、解決不能な `--binary` 明示 path では
/// `"unknown"`)・`devSeamProbe`(preflight probe の結果 echo —
/// probe 失敗時は run が止まり record を emit しないため、ここへ
/// 来る値は常に `"passed"`)。
pub fn measured_binary_block(
    binary: &Path,
    repo_root: &Path,
    dev_seam_probe: &str,
) -> Map<String, Value> {
    let mut block = Map::new();
    let shown = binary
        .strip_prefix(repo_root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| binary.to_string_lossy().into_owned());
    block.insert("path".to_string(), json!(shown));
    let profile = binary
        .parent()
        .and_then(|dir| dir.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| matches!(*name, "release" | "debug"))
        .unwrap_or("unknown");
    block.insert("profile".to_string(), json!(profile));
    block.insert("devSeamProbe".to_string(), json!(dev_seam_probe));
    block
}

/// `createdAt` — ISO 8601 UTC(v1 `new Date().toISOString()` parity、
/// millisecond 精度)。外部 dep を持ち込まないため civil-from-days
/// で自前換算する。
pub fn created_at_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let millis = now.as_millis() as u64;
    let secs = millis / 1000;
    let ms = millis % 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{ms:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Unix epoch day → (year, month, day)(Howard Hinnant の
/// civil_from_days — うるう年含む proleptic Gregorian)。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ---------------------------------------------------------------------------
// §3.4 validation invariant(emit 前 fail-fast — claim 境界を schema
// 形で強制する)
// ---------------------------------------------------------------------------

/// result record 全体の §3.4 invariant を検証する。
/// `benchmarks/results/` からの load 経路(v1 file reject)も
/// この validator を通る。
pub fn validate_result(result: &Value) -> Result<(), CheaperError> {
    // invariant 1 — `schemaVersion === 2` 完全一致(v1 record は
    // reject — v1 file は改変せず残るが受理しない)。
    if result.get("schemaVersion").and_then(Value::as_u64) != Some(2) {
        return Err(bench_error(format!(
            "record schemaVersion must be 2 (got {})",
            result.get("schemaVersion").unwrap_or(&Value::Null)
        )));
    }
    // invariant 4 — top-level `environment` と `measuredBinary.profile`。
    validate_environment(result.get("environment"), "record.environment")?;
    let profile = result
        .get("measuredBinary")
        .and_then(|b| b.get("profile"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if profile.is_empty() {
        return Err(bench_error(
            "record.measuredBinary.profile is required (release|debug)",
        ));
    }
    // invariant 6 — record 全体で `parentUsage` block 以外に parent
    // usage の数値 field を持たない(数値主張を schema 形で排除)。
    scan_parent_usage_fields(result, "record")?;
    // per-observation invariant(1/2/3/4)。
    let observations = result
        .get("observations")
        .and_then(Value::as_array)
        .ok_or_else(|| bench_error("record.observations must be an array"))?;
    for (index, obs) in observations.iter().enumerate() {
        validate_observation(obs, index)?;
    }
    // invariant 5 — comparisons entry の claim enum + claim 別形状。
    if let Some(aggregate) = result.get("aggregate") {
        if let Some(comparisons) = aggregate.get("comparisons").and_then(Value::as_array) {
            for (index, entry) in comparisons.iter().enumerate() {
                validate_comparison(entry, &format!("comparisons[{index}]"))?;
            }
        }
    }
    Ok(())
}

fn validate_observation(obs: &Value, index: usize) -> Result<(), CheaperError> {
    let ctx = || format!("observations[{index}]: ");
    if obs.get("schemaVersion").and_then(Value::as_u64) != Some(2) {
        return Err(bench_error(format!("{}schemaVersion must be 2", ctx())));
    }
    // invariant 2 — parentUsage status enum + status 別必須 key。
    let usage = obs
        .get("parentUsage")
        .and_then(Value::as_object)
        .ok_or_else(|| bench_error(format!("{}parentUsage must be an object", ctx())))?;
    match usage.get("status").and_then(Value::as_str) {
        Some("unavailable") => {
            if usage.get("reason").and_then(Value::as_str).is_none() {
                return Err(bench_error(format!(
                    "{}parentUsage.status=unavailable requires reason",
                    ctx()
                )));
            }
        }
        Some("observed") => {
            if usage.get("source").and_then(Value::as_str).is_none() {
                return Err(bench_error(format!(
                    "{}parentUsage.status=observed requires source",
                    ctx()
                )));
            }
        }
        _ => {
            return Err(bench_error(format!(
                "{}parentUsage.status must be observed|unavailable",
                ctx()
            )))
        }
    }
    // invariant 3 — metrics.usage を持つ observation は
    // `usage.source === "scripted"` 必須(未標識 scripted usage を
    // 記録しない)。`usage: null` は未計測の許容値。
    if let Some(metrics) = obs.get("metrics").and_then(Value::as_object) {
        if let Some(usage) = metrics.get("usage") {
            match usage {
                Value::Null => {}
                Value::Object(_) => {
                    if usage.get("source").and_then(Value::as_str) != Some("scripted") {
                        return Err(bench_error(format!(
                            "{}metrics.usage requires source:\"scripted\"",
                            ctx()
                        )));
                    }
                }
                _ => {
                    return Err(bench_error(format!(
                        "{}metrics.usage must be an object or null",
                        ctx()
                    )))
                }
            }
        }
    }
    // invariant 4 — observation の environment block も node 拒否 +
    // ambientRustc 必須。
    validate_environment(obs.get("environment"), &format!("{}environment", ctx()))?;
    Ok(())
}

/// `environment` block — `node` key 存在は reject(v1 結合残滓の
/// 再混入 guard)、`ambientRustc` 非空文字列は必須。
fn validate_environment(env: Option<&Value>, path: &str) -> Result<(), CheaperError> {
    let obj = env
        .and_then(Value::as_object)
        .ok_or_else(|| bench_error(format!("{path} must be an object")))?;
    if obj.contains_key("node") {
        return Err(bench_error(format!(
            "{path}.node is forbidden in schema v2 (v1 leftover)"
        )));
    }
    match obj.get("ambientRustc") {
        Some(v) if v.as_str().is_some_and(|s| !s.is_empty()) => Ok(()),
        _ => Err(bench_error(format!("{path}.ambientRustc is required"))),
    }
}

/// invariant 6 の走査 — `parentUsage` 以外の key で名前が
/// `parent` + `usage|token|cost|usd` を含む数値 field を reject
/// する(`parentVisibleInterfaceBytes` 等の宣言語彙は該当しない)。
fn scan_parent_usage_fields(value: &Value, path: &str) -> Result<(), CheaperError> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let lower = key.to_ascii_lowercase();
                if key != "parentUsage"
                    && lower.contains("parent")
                    && (lower.contains("usage")
                        || lower.contains("token")
                        || lower.contains("cost")
                        || lower.contains("usd"))
                    && child.is_number()
                {
                    return Err(bench_error(format!(
                        "{path}.{key}: numeric parent-usage fields are forbidden outside the parentUsage block"
                    )));
                }
                scan_parent_usage_fields(child, &format!("{path}.{key}"))?;
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                scan_parent_usage_fields(item, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// comparisons entry の claim 別形状(invariant 5)—
/// `reason`/`assumptions` は claim に関わらず **key 自体を常に
/// emit** する固定 shape で、claim 毎の値規則を強制する。
fn validate_comparison(entry: &Value, path: &str) -> Result<(), CheaperError> {
    let obj = entry
        .as_object()
        .ok_or_else(|| bench_error(format!("{path} must be an object")))?;
    let has = |key: &str| obj.contains_key(key);
    if !has("reason") || !has("assumptions") {
        return Err(bench_error(format!(
            "{path} must always emit reason/assumptions keys (fixed field presence)"
        )));
    }
    let claim = obj.get("claim").and_then(Value::as_str);
    match claim {
        Some("measured") => {
            // null 以外の値を持つ measured entry は mislabel。
            if !obj["reason"].is_null() || !obj["assumptions"].is_null() {
                return Err(bench_error(format!(
                    "{path}: claim=measured requires reason/assumptions to be null"
                )));
            }
        }
        Some("unavailable") => {
            if obj["reason"].as_str().map(str::is_empty).unwrap_or(true) {
                return Err(bench_error(format!(
                    "{path}: claim=unavailable requires a non-empty reason"
                )));
            }
            if !obj["assumptions"].is_null() {
                return Err(bench_error(format!(
                    "{path}: claim=unavailable requires assumptions to be null"
                )));
            }
            // `deltas` 全値 null — unavailable 上に数値主張を置かない。
            let deltas = obj
                .get("deltas")
                .and_then(Value::as_object)
                .ok_or_else(|| bench_error(format!("{path}.deltas must be an object")))?;
            for (dim, value) in deltas {
                if !value.is_null() {
                    return Err(bench_error(format!(
                        "{path}.deltas.{dim} must be null when claim=unavailable"
                    )));
                }
            }
        }
        Some("estimated") => {
            if !obj["reason"].is_null() {
                return Err(bench_error(format!(
                    "{path}: claim=estimated requires reason to be null"
                )));
            }
            validate_assumptions(obj.get("assumptions"), path)?;
        }
        _ => {
            return Err(bench_error(format!(
                "{path}.claim must be measured|estimated|unavailable"
            )))
        }
    }
    Ok(())
}

/// `claim:"estimated"` の `assumptions` record(§4.3)—
/// `{pricing:{model, catalogRevision?, source, inputUsdPerMTok,
/// outputUsdPerMTok, cachedInputUsdPerMTok?}, tokenizer:"o200k_base",
/// boundary:"scripted-usage"}` を要求する。
fn validate_assumptions(assumptions: Option<&Value>, path: &str) -> Result<(), CheaperError> {
    let obj = assumptions.and_then(Value::as_object).ok_or_else(|| {
        bench_error(format!(
            "{path}: claim=estimated requires an assumptions record"
        ))
    })?;
    if obj.get("tokenizer").and_then(Value::as_str) != Some("o200k_base") {
        return Err(bench_error(format!(
            "{path}.assumptions.tokenizer must be \"o200k_base\""
        )));
    }
    if obj.get("boundary").and_then(Value::as_str) != Some("scripted-usage") {
        return Err(bench_error(format!(
            "{path}.assumptions.boundary must be \"scripted-usage\""
        )));
    }
    let pricing = obj
        .get("pricing")
        .and_then(Value::as_object)
        .ok_or_else(|| bench_error(format!("{path}.assumptions.pricing must be an object")))?;
    if pricing
        .get("model")
        .and_then(Value::as_str)
        .map(str::is_empty)
        .unwrap_or(true)
    {
        return Err(bench_error(format!(
            "{path}.assumptions.pricing.model is required"
        )));
    }
    if pricing.get("source").and_then(Value::as_str).is_none() {
        return Err(bench_error(format!(
            "{path}.assumptions.pricing.source is required"
        )));
    }
    for key in ["inputUsdPerMTok", "outputUsdPerMTok"] {
        match pricing.get(key) {
            Some(v) if v.is_number() => {}
            _ => {
                return Err(bench_error(format!(
                    "{path}.assumptions.pricing.{key} must be a number"
                )))
            }
        }
    }
    Ok(())
}
