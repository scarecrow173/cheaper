//! result `aggregate` block の組立て(design §4)— scenario×arm
//! group 化・median/p95・stableValue・successRate・comparisons・
//! reduction・estimated-cost layer。
//!
//! - median/p95 の algorithm は **v1 parity を pin する**
//!   (`benchmarks/lib/aggregate.mjs` — 数値差分の説明可能性のため
//!   補間しない nearest-rank 規約をそのまま再実装)。
//! - `stableValue` は「全 obs で全同一ならその値」の回帰 anchor —
//!   数値 dim は finite number 同士の等値(v1 と同一規約)、
//!   bool/string/null dim(errorCode・condenseFired・runId 等)は
//!   同じ「全同一」規則を JSON 等値へ拡張する(§4.1 — `null` も
//!   stableValue 対象の決定値。runId のように rep 毎に変わる値は
//!   実質 null 化する)。
//! - 集計入力は **record JSON**(schema v2 observation record) —
//!   harness 内部型へ逆投影せず、emit する形そのものを集計源にする
//!   (表現 drift 防止: dim 参照は本 module の [`dim_value`] 1 箇所)。
//! - comparisons は manifest 宣言駆動(v1 の code-side hardcode
//!   `COMPARISONS` を scenario `comparisons` field へ移した —
//!   §4.2)。`--pricing` file が与えられたときのみ estimated entry
//!   を追加する(§4.3)。

use std::collections::BTreeMap;
use std::path::Path;

use cheaper_core::errors::CheaperError;
use serde_json::{json, Map, Value};

use crate::fixture::bench_error;
use crate::manifest::{ComparisonSpec, Scenario};

// ---------------------------------------------------------------------------
// §4.1 純粋関数(median / p95 / stableValue — v1 parity)
// ---------------------------------------------------------------------------

/// `median(xs)`: 昇順 sort → 奇数 `sorted[floor(n/2)]`・偶数は
/// `sorted[n/2-1]` と `sorted[n/2]` の平均(v1 `aggregate.mjs`
/// と同一規約)。空列は `None`。
pub fn median(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    let middle = n / 2;
    if n % 2 == 1 {
        Some(sorted[middle])
    } else {
        Some((sorted[middle - 1] + sorted[middle]) / 2.0)
    }
}

/// `percentile(xs, f)`: 昇順 sort → nearest-rank
/// `sorted[max(0, ceil(n*f)-1)]`(補間しない — v1 parity)。空列は
/// `None`。
pub fn percentile(values: &[f64], fraction: f64) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    let rank = (n as f64 * fraction).ceil() as usize;
    Some(sorted[rank.saturating_sub(1).min(n - 1)])
}

/// `p95` = `percentile(xs, 0.95)`(§4.1)。
pub fn p95(values: &[f64]) -> Option<f64> {
    percentile(values, 0.95)
}

/// `stableValue` — 全 obs の選択値が全同一ならその値、変動・欠測・
/// 型混在を含むなら `Null`(v1 `stableValue` parity + §4.1 の
/// 「null も決定値」拡張)。
///
/// - 数値 dim は `as_f64` 等値で比較する(`json!(4)` と `json!(4.0)`
///   の表記差を同一値とみなす — v1 は JS number しか持たず型区別
///   が存在しなかった parity)。
/// - bool/string/null dim(errorCode・condenseFired・runId 等)は
///   JSON 等値 — 「全 obs 全同一」規則は同じで、型だけ数値以外へ
///   拡張されている(非数値が同一でも emit するのはこの規則の
///   適用範囲のみ)。
/// - 全要素 `Null`(全 obs 欠測)は「全同一 null」として `Null` を
///   返す — emit 値と「不安定」の両方が null になるのは §4.1 の
///   意図どおり。
pub fn stable_value(values: &[Value]) -> Value {
    let Some(first) = values.first() else {
        return Value::Null;
    };
    let all_equal = values.iter().all(|v| match (v, first) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => v == first,
    });
    if all_equal {
        first.clone()
    } else {
        Value::Null
    }
}

// ---------------------------------------------------------------------------
// dim 参照規則(stableMetrics・comparisons・reduction の共通表)
// ---------------------------------------------------------------------------

/// observation record の route block dim(§3.3 — `route.*`)。
const ROUTE_DIMS: &[&str] = &[
    "payloadBytes",
    "rewritten",
    "runId",
    "condenseFired",
    "truncated",
    "originalTokens",
    "returnedTokens",
    "childOutputBytes",
    "returnedBodyBytes",
    "logRefetchBytes",
];

/// observation record の top-level dim(metrics/route block 以外の
/// record field — §3.3)。
const TOP_LEVEL_DIMS: &[&str] = &[
    "wallTimeMs",
    "exitCode",
    "errorCode",
    "directUsageRecordCount",
    "directInvocationTurnsProxy",
    "workflowDefinitionBytes",
    "parentVisibleInterfaceBytes",
    "interfaceDiscoveryWallTimeMs",
    "definitionGenerationMs",
];

/// observation record から dim 値を参照する(stableMetrics・delta・
/// reduction が同じ参照表を使う — §4.2「route arm の byte/token dim
/// は `route.*` の stable 値から参照」規則をここへ集約)。
fn dim_value<'a>(record: &'a Value, dim: &str) -> Option<&'a Value> {
    match dim {
        "usageInput" => record.pointer("/metrics/usage/input"),
        "usageOutput" => record.pointer("/metrics/usage/output"),
        "usageCachedInput" => record.pointer("/metrics/usage/cachedInput"),
        "usageCachedOutput" => record.pointer("/metrics/usage/cachedOutput"),
        "usageTotal" => record.pointer("/metrics/usage/total"),
        dim if ROUTE_DIMS.contains(&dim) => record.get("route").and_then(|r| r.get(dim)),
        dim if TOP_LEVEL_DIMS.contains(&dim) => record.get(dim),
        // 残りは metrics.*(KNOWN_METRIC_FIELDS の metric 名)。
        dim => record.get("metrics").and_then(|m| m.get(dim)),
    }
}

/// record 列(単一 arm 分)の dim 安定値 — stableMetrics・
/// reduction・delta の共通 primitive。
fn stable_dim(records: &[&Value], dim: &str) -> Value {
    let values: Vec<Value> = records
        .iter()
        .map(|r| dim_value(r, dim).cloned().unwrap_or(Value::Null))
        .collect();
    stable_value(&values)
}

/// `stableMetrics` に emit する dim 集合 — v1 comparable/usage/
/// size dims(§4.1 parity)+ §4.1 v2 追加(exitCode/errorCode・
/// postprocessFailures・logRefetches・route dims)+ 残りの scalar
/// 観測 dim(seeded fixture の回帰 anchor として同規則で畳む)。
/// `childInvocations`(array)・reduction 派生名・`wallTimeMs`
/// (median/p95 が担う)は含めない。
const STABLE_DIMS: &[&str] = &[
    // v1 comparableFields
    "jobCount",
    "workerJobCount",
    "toolOnlyJobCount",
    "workerTurns",
    "toolCalls",
    "retainedBytes",
    "materializedPromptBytes",
    "maxConcurrency",
    "criticalPathMs",
    "directToolOperations",
    // v1 usageFields + cached 分解(estimated layer・route 観測)
    "usageInput",
    "usageOutput",
    "usageTotal",
    "usageCachedInput",
    "usageCachedOutput",
    // v1 sizeFields
    "workflowDefinitionBytes",
    "parentVisibleInterfaceBytes",
    "definitionReadBytes",
    "directInvocationTurnsProxy",
    // §4.1 v2 追加(top-level 観測値)
    "exitCode",
    "errorCode",
    "directUsageRecordCount",
    "interfaceDiscoveryWallTimeMs",
    "definitionGenerationMs",
    // §4.1 v2 追加(metrics 観測値)
    "postprocessFailures",
    "logRefetches",
    "retainedResultBytes",
    "artifactBytes",
    "finalResultBytes",
    "repairs",
    "transientRetries",
    "interfaceBytes",
    "nodeCount",
    "toolOperationConcurrency",
    "cliReportedDurationMs",
    "workerStreamConcurrency",
    "routeChildExitCode",
    // §4.1 v2 追加(route dims — runId は rep 毎に変わるため実質
    // null。presence/形式 pin は armExpect 側が担う)
    "payloadBytes",
    "condenseFired",
    "truncated",
    "runId",
    "logRefetchBytes",
    "originalTokens",
    "returnedTokens",
    "childOutputBytes",
    "returnedBodyBytes",
];

/// comparisons `deltas` の対象 dim(§4.2 確定集合 — `wallTimeMs` は
/// median/p95 の双方を別 key で出すため本表に含めない)。
const DELTA_DIMS: &[&str] = &[
    "usageInput",
    "usageOutput",
    "usageTotal",
    "workerTurns",
    "toolCalls",
    "jobCount",
    "workerJobCount",
    "toolOnlyJobCount",
    "retainedBytes",
    "materializedPromptBytes",
    "maxConcurrency",
    "criticalPathMs",
    "directToolOperations",
    "workflowDefinitionBytes",
    "parentVisibleInterfaceBytes",
    "definitionReadBytes",
    "directInvocationTurnsProxy",
    "postprocessFailures",
    "logRefetches",
    "directUsageRecordCount",
    "childOutputBytes",
    "returnedBodyBytes",
    "originalTokens",
    "returnedTokens",
    "logRefetchBytes",
];

// ---------------------------------------------------------------------------
// §4.1 arm 集計
// ---------------------------------------------------------------------------

/// `aggregate.parentUsage` — 全 obs unavailable ⇒ `unavailable` +
/// 先頭 obs の reason、そうでなければ `observed` + 重複排除した
/// source 列(v1 `summarizeArm` parity)。
fn aggregate_parent_usage(records: &[&Value]) -> Value {
    let all_unavailable = records
        .iter()
        .all(|r| r.pointer("/parentUsage/status").and_then(Value::as_str) == Some("unavailable"));
    if all_unavailable {
        let reason = records
            .first()
            .and_then(|r| r.pointer("/parentUsage/reason"))
            .and_then(Value::as_str)
            .unwrap_or("unavailable");
        return json!({"status": "unavailable", "reason": reason});
    }
    let sources: std::collections::BTreeSet<String> = records
        .iter()
        .filter_map(|r| {
            let usage = r.get("parentUsage")?;
            if usage.get("status").and_then(Value::as_str) == Some("observed") {
                usage
                    .get("source")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            } else {
                None
            }
        })
        .collect();
    json!({"status": "observed", "sources": sources})
}

/// scenario×arm の 1 group を集計する(warmup obs は呼出側で
/// 除外済み)。v1 `summarizeArm` + v2 `successRate`・追加 stable dims。
fn summarize_arm(scenario_id: &str, arm: &str, records: &[&Value]) -> Result<Value, CheaperError> {
    // v1: `wallTimeMs` 非有限の record は TypeError — 集計に混ぜず
    // fail-fast(record 組立て側の不具合を検出する)。
    let mut times = Vec::with_capacity(records.len());
    for record in records {
        match record.get("wallTimeMs").and_then(Value::as_f64) {
            Some(t) if t.is_finite() => times.push(t),
            _ => {
                return Err(bench_error(format!(
                    "{scenario_id}/{arm}: every benchmark arm record must include finite wallTimeMs"
                )))
            }
        }
    }
    let successes = records
        .iter()
        .filter(|r| r.get("success").and_then(Value::as_bool) == Some(true))
        .count();
    let mut stable_metrics = Map::new();
    for dim in STABLE_DIMS {
        stable_metrics.insert((*dim).to_string(), stable_dim(records, dim));
    }
    let mut entry = Map::new();
    entry.insert("scenarioId".to_string(), json!(scenario_id));
    entry.insert("arm".to_string(), json!(arm));
    entry.insert("observationCount".to_string(), json!(records.len() as u64));
    entry.insert("parentUsage".to_string(), aggregate_parent_usage(records));
    entry.insert(
        "medianWallTimeMs".to_string(),
        median(&times).map_or(Value::Null, |v| json!(v)),
    );
    entry.insert(
        "p95WallTimeMs".to_string(),
        p95(&times).map_or(Value::Null, |v| json!(v)),
    );
    entry.insert(
        "successRate".to_string(),
        json!(successes as f64 / records.len() as f64),
    );
    entry.insert("stableMetrics".to_string(), Value::Object(stable_metrics));
    Ok(Value::Object(entry))
}

// ---------------------------------------------------------------------------
// §4.2 comparisons + §4.3 estimated layer
// ---------------------------------------------------------------------------

/// `--pricing <file>` の型(§4.3)— `{model, inputUsdPerMTok,
/// outputUsdPerMTok, cachedInputUsdPerMTok?, source,
/// catalogRevision?}`。load 時 fail-fast(malformed JSON・必須 key
/// 欠落・非数値・未知 key は即 error — 中途半端な assumptions を
/// record に残さない)。
#[derive(Debug, Clone)]
pub struct Pricing {
    pub model: String,
    pub catalog_revision: Option<String>,
    /// 価格の出所(必須の自由記述 — catalog resolve は harness 内で
    /// 行わず、明示前提のみを記録する)。
    pub source: String,
    pub input_usd_per_mtok: f64,
    pub output_usd_per_mtok: f64,
    pub cached_input_usd_per_mtok: Option<f64>,
}

impl Pricing {
    const KNOWN_KEYS: &'static [&'static str] = &[
        "model",
        "catalogRevision",
        "source",
        "inputUsdPerMTok",
        "outputUsdPerMTok",
        "cachedInputUsdPerMTok",
    ];

    /// `--pricing` file の load(§4.3 fail-fast 規約)。
    pub fn load(path: &Path) -> Result<Self, CheaperError> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            bench_error(format!("pricing file read failed {}: {e}", path.display()))
        })?;
        Self::parse(&text)
            .map_err(|e| bench_error(format!("pricing file {}: {}", path.display(), e.message())))
    }

    /// pricing JSON の構造検証(必須 key・数値・未知 key reject)。
    pub fn parse(text: &str) -> Result<Self, CheaperError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|e| bench_error(format!("invalid pricing json: {e}")))?;
        let obj = value
            .as_object()
            .ok_or_else(|| bench_error("pricing file must be an object"))?;
        for key in obj.keys() {
            if !Self::KNOWN_KEYS.contains(&key.as_str()) {
                return Err(bench_error(format!(
                    "pricing file contains unknown key: {key} (known: {})",
                    Self::KNOWN_KEYS.join(", ")
                )));
            }
        }
        let required_str = |key: &str| -> Result<String, CheaperError> {
            obj.get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| bench_error(format!("pricing file requires non-empty string {key}")))
        };
        let required_num = |key: &str| -> Result<f64, CheaperError> {
            match obj.get(key).and_then(Value::as_f64) {
                Some(v) if v.is_finite() && v >= 0.0 => Ok(v),
                _ => Err(bench_error(format!(
                    "pricing file requires finite non-negative number {key}"
                ))),
            }
        };
        let optional_num = |key: &str| -> Result<Option<f64>, CheaperError> {
            match obj.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => match v.as_f64() {
                    Some(n) if n.is_finite() && n >= 0.0 => Ok(Some(n)),
                    _ => Err(bench_error(format!(
                        "pricing file requires finite non-negative number {key}"
                    ))),
                },
            }
        };
        let model = required_str("model")?;
        let source = required_str("source")?;
        let catalog_revision = match obj.get("catalogRevision") {
            None | Some(Value::Null) => None,
            Some(v) => Some(
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| bench_error("pricing file catalogRevision must be a string"))?,
            ),
        };
        Ok(Self {
            model,
            catalog_revision,
            source,
            input_usd_per_mtok: required_num("inputUsdPerMTok")?,
            output_usd_per_mtok: required_num("outputUsdPerMTok")?,
            cached_input_usd_per_mtok: optional_num("cachedInputUsdPerMTok")?,
        })
    }

    /// result top-level `pricing` field の echo(§3.1 — `--pricing`
    /// 未指定時は null)。
    pub fn to_json(&self) -> Value {
        let mut block = Map::new();
        block.insert("model".to_string(), json!(self.model));
        if let Some(revision) = &self.catalog_revision {
            block.insert("catalogRevision".to_string(), json!(revision));
        }
        block.insert("source".to_string(), json!(self.source));
        block.insert(
            "inputUsdPerMTok".to_string(),
            json!(self.input_usd_per_mtok),
        );
        block.insert(
            "outputUsdPerMTok".to_string(),
            json!(self.output_usd_per_mtok),
        );
        if let Some(cached) = self.cached_input_usd_per_mtok {
            block.insert("cachedInputUsdPerMTok".to_string(), json!(cached));
        }
        Value::Object(block)
    }

    /// `claim:"estimated"` entry の `assumptions` echo(§4.3 —
    /// `pricing.model`(+`catalogRevision`)を必ず含めて「どの価格
    /// 前提の推計か」を entry から辿れるようにする)。
    pub fn assumptions(&self) -> Value {
        json!({
            "pricing": self.to_json(),
            "tokenizer": "o200k_base",
            "boundary": "scripted-usage",
        })
    }

    /// arm 集計の stable usage から USD cost を導く(§4.3 —
    /// `usage{input,output,cachedInput}×単価`)。`usageInput`/
    /// `usageOutput` が安定値として取れない(欠測・変動)場合は
    /// `None` — cost delta は null になる。cached rate 未宣言時は
    /// cachedInput を価格に含めない(assumptions echo 側でも key を
    /// emit しないため、未価格付けは record から読み取れる)。
    fn cost_usd(&self, arm_entry: &Value) -> Option<f64> {
        let stable = arm_entry.get("stableMetrics")?.as_object()?;
        let input = stable.get("usageInput")?.as_f64()?;
        let output = stable.get("usageOutput")?.as_f64()?;
        let cached = stable
            .get("usageCachedInput")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let cached_rate = self.cached_input_usd_per_mtok.unwrap_or(0.0);
        Some(
            (input * self.input_usd_per_mtok
                + output * self.output_usd_per_mtok
                + cached * cached_rate)
                / 1_000_000.0,
        )
    }
}

/// `deltas` object — 全 dim を emit する固定 shape(value は
/// null 可。unavailable 時は全値 null が invariant)。
fn deltas_object() -> Map<String, Value> {
    let mut deltas = Map::new();
    deltas.insert("wallTimeMs".to_string(), Value::Null);
    deltas.insert("p95WallTimeMs".to_string(), Value::Null);
    for dim in DELTA_DIMS {
        deltas.insert((*dim).to_string(), Value::Null);
    }
    deltas
}

/// `reduction` 宣言の emit — `reductionBytes = leftStable −
/// rightStable`・`reductionRatio = (left−right)/left`(どちらか
/// null・left==0 なら null — §4.2)。宣言 field 名を echo して
/// 「何の削減か」を entry から辿れるようにする。`deltas` の外に
/// 置く — deltas は flat な `{dim: number|null}` map として
/// 「全値 null」invariant が読み取りやすい形を保つ。
fn reduction_object(
    spec: &crate::manifest::ReductionSpec,
    left: &Value,
    right: &Value,
) -> Map<String, Value> {
    let mut block = Map::new();
    block.insert("left".to_string(), json!(spec.left));
    block.insert("right".to_string(), json!(spec.right));
    match (left.as_f64(), right.as_f64()) {
        (Some(l), Some(r)) => {
            block.insert("reductionBytes".to_string(), json!(l - r));
            block.insert(
                "reductionRatio".to_string(),
                if l == 0.0 {
                    Value::Null
                } else {
                    json!((l - r) / l)
                },
            );
        }
        _ => {
            block.insert("reductionBytes".to_string(), Value::Null);
            block.insert("reductionRatio".to_string(), Value::Null);
        }
    }
    block
}

/// manifest `comparisons` 宣言 1 件の entry(claim 別)を組み立てる。
/// `arm_index` は `(scenarioId, arm)` → arm 集計 entry。
fn comparison_entry(
    scenario_id: &str,
    spec: &ComparisonSpec,
    arm_index: &BTreeMap<(String, String), Value>,
    pricing: Option<&Pricing>,
) -> Vec<Value> {
    let left = arm_index.get(&(scenario_id.to_string(), spec.left.clone()));
    let right = arm_index.get(&(scenario_id.to_string(), spec.right.clone()));
    let mut entry = Map::new();
    entry.insert("scenarioId".to_string(), json!(scenario_id));
    entry.insert("left".to_string(), json!(spec.left));
    entry.insert("right".to_string(), json!(spec.right));
    let (Some(left), Some(right)) = (left, right) else {
        // v1 `status:"incomplete"` の fold — claim:"unavailable" +
        // reason。deltas 全値 null + assumptions null(invariant 5)。
        entry.insert("claim".to_string(), json!("unavailable"));
        entry.insert("deltas".to_string(), Value::Object(deltas_object()));
        if let Some(reduction) = &spec.reduction {
            entry.insert(
                "reduction".to_string(),
                Value::Object(reduction_object(reduction, &Value::Null, &Value::Null)),
            );
        }
        entry.insert(
            "reason".to_string(),
            json!("one or both comparison arms are absent"),
        );
        entry.insert("assumptions".to_string(), Value::Null);
        return vec![Value::Object(entry)];
    };
    let mut deltas = deltas_object();
    let delta = |a: &Value, b: &Value| match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => json!(a - b),
        _ => Value::Null,
    };
    let stable = |arm: &Value, dim: &str| {
        arm.get("stableMetrics")
            .and_then(|s| s.get(dim))
            .cloned()
            .unwrap_or(Value::Null)
    };
    deltas.insert(
        "wallTimeMs".to_string(),
        delta(
            left.get("medianWallTimeMs").unwrap_or(&Value::Null),
            right.get("medianWallTimeMs").unwrap_or(&Value::Null),
        ),
    );
    deltas.insert(
        "p95WallTimeMs".to_string(),
        delta(
            left.get("p95WallTimeMs").unwrap_or(&Value::Null),
            right.get("p95WallTimeMs").unwrap_or(&Value::Null),
        ),
    );
    for dim in DELTA_DIMS {
        deltas.insert(
            (*dim).to_string(),
            delta(&stable(left, dim), &stable(right, dim)),
        );
    }
    entry.insert("claim".to_string(), json!("measured"));
    entry.insert("deltas".to_string(), Value::Object(deltas));
    if let Some(reduction) = &spec.reduction {
        entry.insert(
            "reduction".to_string(),
            Value::Object(reduction_object(
                reduction,
                &stable(left, &reduction.left),
                &stable(right, &reduction.right),
            )),
        );
    }
    entry.insert("reason".to_string(), Value::Null);
    entry.insert("assumptions".to_string(), Value::Null);
    let mut entries = vec![Value::Object(entry)];
    // §4.3 — estimated entry は measured entry の sibling として
    // 追加する(`--pricing` 無しでは一切 emit しない)。
    if let Some(pricing) = pricing {
        let cost_delta = match (pricing.cost_usd(left), pricing.cost_usd(right)) {
            (Some(l), Some(r)) => json!(l - r),
            _ => Value::Null,
        };
        let mut estimated = Map::new();
        estimated.insert("scenarioId".to_string(), json!(scenario_id));
        estimated.insert("left".to_string(), json!(spec.left));
        estimated.insert("right".to_string(), json!(spec.right));
        estimated.insert("claim".to_string(), json!("estimated"));
        estimated.insert(
            "deltas".to_string(),
            json!({"estimatedCostUsd": cost_delta}),
        );
        estimated.insert("reason".to_string(), Value::Null);
        estimated.insert("assumptions".to_string(), pricing.assumptions());
        entries.push(Value::Object(estimated));
    }
    entries
}

// ---------------------------------------------------------------------------
// §4 集計本体 — record JSON 列 → `aggregate` block
// ---------------------------------------------------------------------------

/// `result.aggregate` block の組立て — warmup obs 除外・
/// scenario×arm group 化 → arm 集計 → manifest `comparisons` 宣言
/// 駆動の comparisons(measured/unavailable + estimated sibling)。
pub fn aggregate(
    observations: &[Value],
    scenarios: &[Scenario],
    pricing: Option<&Pricing>,
) -> Result<Value, CheaperError> {
    // scenario×arm group 化(encounter order を保持 — observation は
    // scenario→arm→rep 順に生成される)。warmup は集計除外。
    let mut order: Vec<(String, String)> = Vec::new();
    let mut groups: BTreeMap<(String, String), Vec<&Value>> = BTreeMap::new();
    for record in observations {
        if record.get("warmup").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let key = (
            record
                .get("scenarioId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            record
                .get("arm")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        );
        if !groups.contains_key(&key) {
            order.push(key.clone());
        }
        groups.entry(key).or_default().push(record);
    }
    let mut arm_index: BTreeMap<(String, String), Value> = BTreeMap::new();
    let mut arms = Vec::with_capacity(order.len());
    for (scenario_id, arm) in order {
        let records = &groups[&(scenario_id.clone(), arm.clone())];
        let entry = summarize_arm(&scenario_id, &arm, records)?;
        arm_index.insert((scenario_id, arm), entry.clone());
        arms.push(entry);
    }
    // comparisons — manifest 宣言駆動(v1 `COMPARISONS` hardcode の
    // 置換。scenario 単位で宣言順に処理し、measured entry には
    // `--pricing` 指定時のみ estimated sibling を続ける)。
    let mut comparisons = Vec::new();
    for scenario in scenarios {
        for spec in &scenario.comparisons {
            comparisons.extend(comparison_entry(&scenario.id, spec, &arm_index, pricing));
        }
    }
    let mut aggregate = Map::new();
    aggregate.insert("arms".to_string(), Value::Array(arms));
    aggregate.insert("comparisons".to_string(), Value::Array(comparisons));
    Ok(Value::Object(aggregate))
}
