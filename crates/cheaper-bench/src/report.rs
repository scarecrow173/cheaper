//! result JSON の組立てと immutable 書込み(design §3.1/§3.5)。
//!
//! - 保存先は既定 `benchmarks/results/<runId>.json`(UUIDv4)。
//!   書込みは `OpenOptions::create_new(true)` — 既存 file の上書きを
//!   拒否し、`--output` 明示 path も同じ非上書き契約(append-only・
//!   commit 対象は不変)。
//! - load/aggregate 経路は `schemaVersion` を確認して v1 file を
//!   reject する(v1 JSON は一切改変しない・混在 run を作らない)。
//! - file 書き出しの serialization は計測値ではないため
//!   `stringify_pretty`(`JSON.stringify` 意味論の pretty 版)を使う。

use std::path::{Path, PathBuf};

use cheaper_core::errors::CheaperError;
use cheaper_core::json::stringify::stringify_pretty;
use serde_json::{json, Map, Value};

use crate::fixture::bench_error;
use crate::record;

/// result top-level(§3.1)の組立てに必要な run meta。
pub struct RunMeta<'a> {
    /// UUIDv4 — file 名 `<runId>.json` と record `runId` に使う。
    pub run_id: &'a str,
    /// ISO 8601 UTC([`record::created_at_iso8601`])。
    pub created_at: &'a str,
    pub fixture_revision: &'a str,
    /// `git rev-parse HEAD`(採取不能時 `None` → null)。
    pub repository_revision: Option<&'a str>,
    /// `warmupCount`/`repetitionCount`/`selectedScenarios`/
    /// `ambientPassthrough`。
    pub conditions: Map<String, Value>,
    /// [`record::measured_binary_block`]。
    pub measured_binary: Map<String, Value>,
    /// [`record::capture_environment`]。
    pub environment: Map<String, Value>,
    /// `--pricing` echo(未指定時 `None` → null)。
    pub pricing: Option<Value>,
}

/// 既定の保存先 `<repo>/benchmarks/results/<runId>.json`。
pub fn default_result_path(repo_root: &Path, run_id: &str) -> PathBuf {
    repo_root
        .join("benchmarks")
        .join("results")
        .join(format!("{run_id}.json"))
}

/// §3.1 の result top-level を組み立てる。
pub fn assemble_result(meta: &RunMeta<'_>, observations: Vec<Value>, aggregate: Value) -> Value {
    let mut result = Map::new();
    result.insert("schemaVersion".to_string(), json!(2));
    result.insert("runId".to_string(), json!(meta.run_id));
    result.insert("createdAt".to_string(), json!(meta.created_at));
    result.insert("fixtureRevision".to_string(), json!(meta.fixture_revision));
    result.insert(
        "repositoryRevision".to_string(),
        meta.repository_revision.map_or(Value::Null, |r| json!(r)),
    );
    result.insert(
        "conditions".to_string(),
        Value::Object(meta.conditions.clone()),
    );
    result.insert(
        "measuredBinary".to_string(),
        Value::Object(meta.measured_binary.clone()),
    );
    result.insert(
        "environment".to_string(),
        Value::Object(meta.environment.clone()),
    );
    result.insert(
        "pricing".to_string(),
        meta.pricing.clone().unwrap_or(Value::Null),
    );
    result.insert("observations".to_string(), Value::Array(observations));
    result.insert("aggregate".to_string(), aggregate);
    Value::Object(result)
}

/// result file の非上書き書込み — `create_new(true)` で既存 file
/// (v1 artifact・同 runId の再生成・`--output` 衝突)を拒否する
/// (§3.5)。
pub fn write_result(path: &Path, result: &Value) -> Result<(), CheaperError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                bench_error(format!(
                    "result dir create failed {}: {e}",
                    parent.display()
                ))
            })?;
        }
    }
    let mut text = stringify_pretty(result);
    text.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            bench_error(format!(
                "result write refused (results are immutable — path must not exist) {}: {e}",
                path.display()
            ))
        })?;
    use std::io::Write;
    file.write_all(text.as_bytes())
        .map_err(|e| bench_error(format!("result write failed {}: {e}", path.display())))
}

/// result file の load — `schemaVersion` を確認して v1 file を
/// reject し(混在 run を作らない)、v2 invariant も通す(§3.5)。
pub fn load_result(path: &Path) -> Result<Value, CheaperError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| bench_error(format!("result read failed {}: {e}", path.display())))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|e| bench_error(format!("invalid result json {}: {e}", path.display())))?;
    record::validate_result(&value)
        .map_err(|e| bench_error(format!("result {}: {}", path.display(), e.message())))?;
    Ok(value)
}
