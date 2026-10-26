//! Port of `packages/cli/src/usage-report.ts` + `cli.ts` の usage 分岐。
//!
//! `<userConfigDir>/usage.jsonl` の read-only 集計。record は
//! `serde_json::Value`(additive-only schema — 未知 field を保持したまま
//! 必要 field のみ読む)として扱い、破損行・kind 不明行は skip する。
//!
//! - `read_usage_records` — per-line JSON parse(破損行 skip、file 不在 → 空)。
//! - `filter_run_records` — `kind:"run"` のみ + `--since`/`--worker`/
//!   `--provider`/`--model` filter。
//! - `route_window` / `build_route_report` — `kind:"route"` と
//!   `"route-refetch"` を `--since` のみで window 化し、logRef join と
//!   global/per-route 集計を行う(`--route` はこの route-scope のみ)。
//! - `aggregate` — totals + measured(savings 持ちのみ) +
//!   comparable(`directlyComparable === true` のみ)集計。
//! - `build_by_error` — filter 済み run records の error code と
//!   allowlist dimension を集計する。
//! - `build_usage_report` — `--compare-model` の estimated 算出:
//!   `resolve_pricing`(runtime catalog cost → user `pricing.json` →
//!   bundled snapshot)で comparison model の価格を解決し、stale
//!   (updatedAt が 30 日超)または comparable run 0 件なら estimated を
//!   付けない。
//! - `execute_usage` — `cli.ts` の `parsed.kind === "usage"` 分岐相当
//!   (option 検証 → config dir 解決 → best-effort catalog 観測 →
//!   report → utility envelope)。

use std::path::Path;
use std::{collections::BTreeMap, collections::HashMap};

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirsOptions};
use cheaper_core::duration::parse_duration;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::json::number::format_number;
use cheaper_core::model::classify::ModelInfo;
use cheaper_core::model::model_ref::{parse_model_ref, ModelRef};
use cheaper_core::model::pricing::{resolve_pricing, ModelCost, ModelPricing, PricingSource};
use serde_json::{Map, Value};
use tracing::Instrument;

use crate::args::UsageCliArgs;
use crate::cli::{BoxError, CliHooks, CliIo, CommandOut};
use crate::envelope::utility_success_envelope;
use crate::pricing::load_pricing_snapshot;
use crate::usage_log::{ABORT_REASONS, USAGE_LOG_FILENAME};

/// snapshot の updatedAt がこの期間より古い pricing は stale とみなす
/// (TS `PRICING_STALE_MS` = 30 日)。
pub const PRICING_STALE_MS: f64 = 30.0 * 24.0 * 3_600_000.0;

/// Usage の grouped section 選択（T1 は compile seam のみ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageGroupBy {
    Route,
    Error,
}

/// `UsageFilters`。`since_ms` は `ts >= now - sinceMs` の record のみ対象。
#[derive(Debug, Clone, Default)]
pub struct UsageFilters {
    pub since_ms: Option<f64>,
    pub worker: Option<String>,
    pub provider: Option<String>,
    /// `<provider>/<id>` 形式(未検証でも可 — 不正なら filter 不適用)。
    pub model: Option<String>,
    pub route: Option<String>,
}

// ---------------------------------------------------------------------------
// JS `Date.parse` の ISO-8601 subset(日時文字列 → ms epoch)
// ---------------------------------------------------------------------------

/// Hinnant の days_from_civil(usage_log.rs の civil-from-days の逆)。
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn digits(s: &[u8], n: usize) -> Option<(i64, &[u8])> {
    if s.len() < n || !s[..n].iter().all(u8::is_ascii_digit) {
        return None;
    }
    let v = s[..n]
        .iter()
        .fold(0i64, |acc, b| acc * 10 + i64::from(b - b'0'));
    Some((v, &s[n..]))
}

/// `Date.parse` の ISO 形式受理部(`YYYY[-MM[-DD]][T| HH:mm[:ss[.fff]]]
/// [Z|±HH[:mm]]`)相当。date-only は UTC。offset 無し date-time は JS
/// では local time だが、TZ database を持たないため UTC として扱う
/// (既知 divergence — 本 writer の `ts` は常に `Z` 付き)。
/// `YYYY`/`YYYY-MM` 短縮形・`+YYYYYY` 拡張年も受理する。非 ISO 文字列は
/// `None`(JS `NaN` 相当 — filter 除外 / stale 判定側で処理)。
fn date_parse(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    // year: `YYYY` or `±YYYYYY`
    let (year, rest) = match b.first() {
        Some(b'+') | Some(b'-') => {
            let sign = if b[0] == b'-' { -1i64 } else { 1 };
            let (y, r) = digits(&b[1..], 6)?;
            (sign * y, r)
        }
        _ => digits(b, 4)?,
    };
    let mut rest = rest;
    // month / day(省略時は 1)
    let mut month = 1i64;
    let mut day = 1i64;
    if rest.first() == Some(&b'-') {
        let (m, r) = digits(&rest[1..], 2)?;
        month = m;
        rest = r;
        if rest.first() == Some(&b'-') {
            let (d, r) = digits(&rest[1..], 2)?;
            day = d;
            rest = r;
        }
    }
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return None;
    }
    // time part(`T` or space separator + HH:mm 必須)
    let mut hms_ms = 0i64;
    let mut offset_ms = 0i64;
    if let Some(sep) = rest.first() {
        if *sep != b'T' && *sep != b' ' {
            return None;
        }
        let t = &rest[1..];
        let (hh, t) = digits(t, 2)?;
        let t = t.strip_prefix(b":")?;
        let (mm, t) = digits(t, 2)?;
        let mut t = t;
        let mut ss = 0i64;
        if t.first() == Some(&b':') {
            let (v, r) = digits(&t[1..], 2)?;
            ss = v;
            t = r;
        }
        let mut frac = 0i64;
        if t.first() == Some(&b'.') {
            let f = &t[1..];
            let n = f.iter().take_while(|c| c.is_ascii_digit()).count();
            if n == 0 {
                return None;
            }
            // fraction は秒の小数 — ms は先頭 3 桁(右 0 padding)の切捨て。
            let take = n.min(3);
            let mut ms = 0i64;
            for c in &f[..take] {
                ms = ms * 10 + i64::from(c - b'0');
            }
            for _ in take..3 {
                ms *= 10;
            }
            frac = ms;
            t = &f[n..];
        }
        // timezone: `Z` or `±HH` / `±HH:mm` / `±HHmm`(不在 = 無 offset →
        // JS local time を UTC 近似する既知 divergence)。
        match t.first() {
            Some(b'Z') => {
                t = &t[1..];
            }
            Some(b'+') | Some(b'-') => {
                let sign = if t[0] == b'-' { -1i64 } else { 1 };
                let (oh, t2) = digits(&t[1..], 2)?;
                let (om, t2) = match t2.first() {
                    Some(b':') => {
                        let (v, r) = digits(&t2[1..], 2)?;
                        (v, r)
                    }
                    Some(c) if c.is_ascii_digit() => digits(t2, 2)?,
                    _ => (0, t2),
                };
                if !(0..=23).contains(&oh) || !(0..=59).contains(&om) {
                    return None;
                }
                offset_ms = sign * (oh * 60 + om) * 60_000;
                t = t2;
            }
            _ => {}
        }
        if !t.is_empty() {
            return None;
        }
        if !(0..=24).contains(&hh) || !(0..=59).contains(&mm) || !(0..=59).contains(&ss) {
            return None;
        }
        // ISO `24:00` は翌日 0 時(分・秒・小数は 0 のみ許容)。
        if hh == 24 && (mm != 0 || ss != 0 || frac != 0) {
            return None;
        }
        hms_ms = (hh * 3600 + mm * 60 + ss) * 1000 + frac;
    }
    // date-only / 無 offset はここまでの値で UTC として扱う。
    let days = days_from_civil(year, month, day);
    Some(days as f64 * 86_400_000.0 + hms_ms as f64 - offset_ms as f64)
}

// ---------------------------------------------------------------------------
// read / filter / aggregate
// ---------------------------------------------------------------------------

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key)
}

fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    get(v, key).and_then(Value::as_str)
}

/// `isRecord` — object かつ `kind` が `"run"` / `"count"` /
/// `"route"` / `"route-refetch"`。route 系は独立 bucket へ集計
/// され、run 集計・`directlyComparable` 母数へは混入しない(D9)。
fn is_record(v: &Value) -> bool {
    v.is_object()
        && matches!(
            str_field(v, "kind"),
            Some("run") | Some("count") | Some("route") | Some("route-refetch")
        )
}

/// `readUsageRecords` — per-line parse。file 不在・破損行は skip
/// (read-only 集計)。Node `readFile(path, "utf8")` は不正 byte を
/// U+FFFD へ置き換える lossy decode — `read_to_string` だと 1 byte の
/// 不正 UTF-8 で file 全体を失うため、生 byte → `from_utf8_lossy` で
/// 揃え、malformed な行のみを skip する。
pub fn read_usage_records(user_config_dir: &Path) -> Vec<Value> {
    let Ok(bytes) = std::fs::read(user_config_dir.join(USAGE_LOG_FILENAME)) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&bytes)
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(is_record)
        .collect()
}

/// `filterRunRecords`。`kind:"run"` のみ + since/worker/provider/model
/// filter。`now` は ms epoch を返す。
pub fn filter_run_records(
    records: Vec<Value>,
    filters: &UsageFilters,
    now: &dyn Fn() -> f64,
) -> Vec<Value> {
    let model_ref = filters.model.as_deref().map(parse_model_ref);
    records
        .into_iter()
        .filter(|r| {
            if str_field(r, "kind") != Some("run") {
                return false;
            }
            if let Some(since_ms) = filters.since_ms {
                // `Date.parse(r.ts)` — 非文字列・parse 不能は NaN 相当で除外。
                let ts = str_field(r, "ts").and_then(date_parse);
                match ts {
                    Some(ts) if ts >= now() - since_ms => {}
                    _ => return false,
                }
            }
            if let Some(w) = &filters.worker {
                if str_field(r, "worker") != Some(w.as_str()) {
                    return false;
                }
            }
            if let Some(p) = &filters.provider {
                if r.get("model").and_then(|m| str_field(m, "provider")) != Some(p.as_str()) {
                    return false;
                }
            }
            if let Some(Some(mref)) = &model_ref {
                let m = r.get("model");
                if m.and_then(|m| str_field(m, "provider")) != Some(mref.provider.as_str())
                    || m.and_then(|m| str_field(m, "id")) != Some(mref.id.as_str())
                {
                    return false;
                }
            }
            true
        })
        .collect()
}

/// JS truthiness(record の `ok` 判定用。writer が出すのは bool のみだが
/// 外部生成 record も許容する)。
fn truthy(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Null) | None => false,
        // object/array は常に truthy。
        Some(_) => true,
    }
}

/// JS `acc += v` の number 側意味論: absent(undefined) → NaN、
/// null → +0、number → その値、その他 → NaN(string の + は string
/// 結合になるが、writer の生成する record では発生しないため NaN で近似)。
fn plus(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(v) => v.as_f64().unwrap_or(f64::NAN),
    }
}

/// JS `v ?? 0`(null/undefined → 0)。非 number 値は JS では string 結合等に
/// なるが、record schema 上は number のみ — NaN 近似しないで 0 に揃える
/// 代わりに number のみ採用する。
fn or_zero(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(0.0)
}

/// `r.usage`/`r.savings` が null のときの `Cannot read properties of null`
/// TypeError 相当(TS は aggregate が throw → INTERNAL_ERROR envelope)。
fn null_property_error(reading: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InternalError,
        format!("Cannot read properties of null (reading '{reading}')"),
    )
}

/// `aggregate` の数値 accumulator(f64 — JS number 意味論、非有限は
/// serialize 時に null になる)。
#[derive(Debug, Default)]
struct Agg {
    ok: f64,
    errors: f64,
    usage_input: f64,
    usage_output: f64,
    usage_cache_read: f64,
    usage_cache_write: f64,
    usage_total_tokens: f64,
    usage_cost_usd: f64,
    turns: f64,
    tool_calls: f64,
    duration_ms: f64,
    m_runs: f64,
    m_source: f64,
    m_returned: f64,
    m_avoided: f64,
    c_runs: f64,
    c_source: f64,
    c_returned: f64,
    c_cost_usd: f64,
}

/// `aggregate`(TS `aggregate`)。usage/savings が明示 `null` の record は
/// JS で TypeError を投げるため、`CheaperError`(INTERNAL_ERROR)を返す。
fn aggregate(records: &[Value]) -> Result<Agg, CheaperError> {
    let mut report = Agg { ..Agg::default() };
    for r in records {
        if truthy(get(r, "ok")) {
            report.ok += 1.0;
        } else {
            report.errors += 1.0;
        }
        match get(r, "usage") {
            None => {}
            Some(Value::Null) => return Err(null_property_error("input")),
            Some(u) => {
                report.usage_input += plus(u.get("input"));
                report.usage_output += plus(u.get("output"));
                report.usage_cache_read += plus(u.get("cacheRead"));
                report.usage_cache_write += plus(u.get("cacheWrite"));
                report.usage_total_tokens += plus(u.get("totalTokens"));
                report.usage_cost_usd += plus(u.get("costUsd"));
            }
        }
        // 旧 record には turns/toolCalls が無い(?? 0 で 0 加算)。
        report.turns += or_zero(get(r, "turns"));
        report.tool_calls += or_zero(get(r, "toolCalls"));
        report.duration_ms += or_zero(get(r, "durationMs"));
        match get(r, "savings") {
            None => {}
            Some(Value::Null) => return Err(null_property_error("sourceTokens")),
            Some(s) => {
                report.m_runs += 1.0;
                report.m_source += plus(s.get("sourceTokens"));
                report.m_returned += plus(s.get("returnedTokens"));
                report.m_avoided += plus(s.get("avoidedContextTokens"));
                // 直接比較が成立する worker の run のみ estimated の母数。
                // 旧 record(flag 不在)や write 系 worker は comparable に
                // 含めない。
                if s.get("directlyComparable") == Some(&Value::Bool(true)) {
                    report.c_runs += 1.0;
                    report.c_source += plus(s.get("sourceTokens"));
                    report.c_returned += plus(s.get("returnedTokens"));
                    report.c_cost_usd += or_zero(get(r, "usage").and_then(|u| u.get("costUsd")));
                }
            }
        }
    }
    Ok(report)
}

/// `Value::from(f64)` — 非有限は Null(JSON.stringify の NaN/Infinity →
/// null と一致)。
fn num(v: f64) -> Value {
    Value::from(v)
}

/// `Omit<UsageReport, "estimated">` の object literal 順で Map を構築。
fn report_map(records: usize, agg: &Agg) -> Map<String, Value> {
    let usage = Map::from_iter([
        ("input".to_string(), num(agg.usage_input)),
        ("output".to_string(), num(agg.usage_output)),
        ("cacheRead".to_string(), num(agg.usage_cache_read)),
        ("cacheWrite".to_string(), num(agg.usage_cache_write)),
        ("totalTokens".to_string(), num(agg.usage_total_tokens)),
        ("costUsd".to_string(), num(agg.usage_cost_usd)),
    ]);
    let comparable = Map::from_iter([
        ("runs".to_string(), num(agg.c_runs)),
        ("sourceTokens".to_string(), num(agg.c_source)),
        ("returnedTokens".to_string(), num(agg.c_returned)),
        ("costUsd".to_string(), num(agg.c_cost_usd)),
    ]);
    let measured = Map::from_iter([
        ("runs".to_string(), num(agg.m_runs)),
        ("sourceTokens".to_string(), num(agg.m_source)),
        ("returnedTokens".to_string(), num(agg.m_returned)),
        ("avoidedContextTokens".to_string(), num(agg.m_avoided)),
        ("comparable".to_string(), Value::Object(comparable)),
    ]);
    Map::from_iter([
        ("records".to_string(), num(records as f64)),
        ("ok".to_string(), num(agg.ok)),
        ("errors".to_string(), num(agg.errors)),
        ("usage".to_string(), Value::Object(usage)),
        ("turns".to_string(), num(agg.turns)),
        ("toolCalls".to_string(), num(agg.tool_calls)),
        ("durationMs".to_string(), num(agg.duration_ms)),
        ("measured".to_string(), Value::Object(measured)),
    ])
}

/// snapshot 由来 pricing の updatedAt が stale か(updatedAt 不在 =
/// runtime-catalog は stale にしない)。
fn is_stale_pricing(pricing: &ModelPricing, now_ms: f64) -> bool {
    let Some(updated_at) = &pricing.updated_at else {
        return false;
    };
    match date_parse(updated_at) {
        None => true,
        Some(updated_ms) => now_ms - updated_ms > PRICING_STALE_MS,
    }
}

/// `estimate`: 直接比較可能な measured run があり、かつ pricing が
/// stale でない場合のみ estimated object を返す。
/// parentEquiv = (comparable.sourceTokens * input + comparable.returnedTokens
/// * output) / 1e6。`stale: false` 固定(stale は先に弾く)。
fn estimate(
    agg: &Agg,
    pricing: &ModelPricing,
    provenance: &str,
    stale: bool,
    compare_model: &str,
) -> Option<Value> {
    if stale || agg.c_runs == 0.0 {
        return None;
    }
    let parent_equiv = (agg.c_source * pricing.input + agg.c_returned * pricing.output) / 1e6;
    let savings = parent_equiv - agg.c_cost_usd;
    let percent = if parent_equiv > 0.0 {
        num(savings / parent_equiv * 100.0)
    } else {
        Value::Null
    };
    let mut pricing_obj = Map::new();
    pricing_obj.insert("source".to_string(), Value::from(provenance));
    if let Some(u) = &pricing.updated_at {
        pricing_obj.insert("updatedAt".to_string(), Value::from(u.clone()));
    }
    if let Some(v) = &pricing.snapshot_version {
        pricing_obj.insert("snapshotVersion".to_string(), Value::from(v.clone()));
    }
    pricing_obj.insert("stale".to_string(), Value::Bool(false));
    Some(Value::Object(Map::from_iter([
        ("compareModel".to_string(), Value::from(compare_model)),
        (
            "estimatedParentEquivalentCostUsd".to_string(),
            num(parent_equiv),
        ),
        ("estimatedSavingsUsd".to_string(), num(savings)),
        ("estimatedSavingsPercent".to_string(), percent),
        ("pricing".to_string(), Value::Object(pricing_obj)),
    ])))
}

struct RouteWindow<'a> {
    routes: Vec<&'a Value>,
    refetches: Vec<&'a Value>,
}

fn route_window<'a>(
    records: &'a [Value],
    since_ms: Option<f64>,
    now: &dyn Fn() -> f64,
) -> RouteWindow<'a> {
    let cutoff = since_ms.map(|duration| now() - duration);
    let mut window = RouteWindow {
        routes: Vec::new(),
        refetches: Vec::new(),
    };
    for record in records {
        let kind = str_field(record, "kind");
        if !matches!(kind, Some("route" | "route-refetch")) {
            continue;
        }
        if let Some(cutoff) = cutoff {
            let in_window = get(record, "ts")
                .and_then(Value::as_str)
                .and_then(date_parse)
                .is_some_and(|timestamp| timestamp >= cutoff);
            if !in_window {
                continue;
            }
        }
        match kind {
            Some("route") => window.routes.push(record),
            Some("route-refetch") => window.refetches.push(record),
            _ => unreachable!("route kind was checked above"),
        }
    }
    window
}

#[derive(Default)]
struct RouteAgg {
    executions: f64,
    succeeded: f64,
    failed: f64,
    original_tokens: f64,
    returned_tokens: f64,
    duration_exec: f64,
    duration_analyze: f64,
    duration_total: f64,
    postprocess_failures: f64,
    unmeasurable_tokens: f64,
    unmeasurable_durations: f64,
}

#[derive(Default)]
struct RouteReport {
    bucket: Map<String, Value>,
    by_route: Vec<Map<String, Value>>,
}

#[derive(Default)]
struct ErrorAgg {
    count: f64,
    duration_ms: f64,
    workers: BTreeMap<String, f64>,
    models: BTreeMap<String, f64>,
    reasons: BTreeMap<String, f64>,
    exec_timeout_ms: BTreeMap<String, f64>,
    exec_max_turns: BTreeMap<String, f64>,
    exec_max_tool_calls: BTreeMap<String, f64>,
}

fn is_number(value: Option<&Value>) -> bool {
    value.is_some_and(Value::is_number)
}

fn build_route_report(
    window: &RouteWindow<'_>,
    route_name: Option<&str>,
    group_by_route: bool,
) -> RouteReport {
    let mut owners: HashMap<&str, &str> = HashMap::new();
    for record in &window.routes {
        let (Some(route), Some(log_ref)) =
            (str_field(record, "route"), str_field(record, "logRef"))
        else {
            continue;
        };
        if !log_ref.is_empty() {
            owners.entry(log_ref).or_insert(route);
        }
    }

    let mut joined: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
    let (mut orphan_count, mut orphan_bytes, mut all_refetches, mut all_refetch_bytes) =
        (0.0, 0.0, 0.0, 0.0);
    for record in &window.refetches {
        let bytes = or_zero(get(record, "bytesRead"));
        all_refetches += 1.0;
        all_refetch_bytes += bytes;
        let owner = str_field(record, "logRef")
            .filter(|log_ref| !log_ref.is_empty())
            .and_then(|log_ref| owners.get(log_ref).copied());
        if let Some(route) = owner {
            let entry = joined.entry(route).or_default();
            entry.0 += 1.0;
            entry.1 += bytes;
        } else {
            orphan_count += 1.0;
            orphan_bytes += bytes;
        }
    }

    let scoped_routes =
        window.routes.iter().copied().filter(|record| {
            route_name.is_none_or(|name| str_field(record, "route") == Some(name))
        });
    let mut groups: BTreeMap<&str, RouteAgg> = BTreeMap::new();
    let mut executions = 0.0;
    let mut failures = 0.0;
    let mut original_tokens = 0.0;
    let mut returned_tokens = 0.0;
    let mut postprocess_failures = 0.0;
    let unkeyed_records = window
        .routes
        .iter()
        .filter(|record| str_field(record, "route").is_none())
        .count() as f64;
    for record in scoped_routes {
        executions += 1.0;
        let failed = !truthy(get(record, "ok"));
        failures += f64::from(failed);
        original_tokens += or_zero(get(record, "originalTokens"));
        returned_tokens += or_zero(get(record, "returnedTokens"));
        postprocess_failures += or_zero(get(record, "postprocessFailures"));

        let Some(route) = str_field(record, "route") else {
            continue;
        };
        let agg = groups.entry(route).or_default();
        agg.executions += 1.0;
        if failed {
            agg.failed += 1.0;
        } else {
            agg.succeeded += 1.0;
        }
        let tokens_measurable =
            is_number(get(record, "originalTokens")) && is_number(get(record, "returnedTokens"));
        if !tokens_measurable {
            agg.unmeasurable_tokens += 1.0;
        }
        agg.original_tokens += or_zero(get(record, "originalTokens"));
        agg.returned_tokens += or_zero(get(record, "returnedTokens"));
        agg.postprocess_failures += or_zero(get(record, "postprocessFailures"));

        let duration = get(record, "durationMs");
        let duration_values = [
            duration.and_then(|value| value.get("exec")),
            duration.and_then(|value| value.get("analyze")),
            duration.and_then(|value| value.get("total")),
        ];
        if !duration_values.into_iter().all(is_number) {
            agg.unmeasurable_durations += 1.0;
        }
        agg.duration_exec += or_zero(duration_values[0]);
        agg.duration_analyze += or_zero(duration_values[1]);
        agg.duration_total += or_zero(duration_values[2]);
    }

    let scoped_refetches = match route_name {
        Some(name) => joined.get(name).copied().unwrap_or_default(),
        None => (all_refetches, all_refetch_bytes),
    };
    let mut bucket = Map::from_iter([
        ("executions".to_string(), num(executions)),
        ("failures".to_string(), num(failures)),
        ("refetches".to_string(), num(scoped_refetches.0)),
        ("refetchBytes".to_string(), num(scoped_refetches.1)),
        ("originalTokens".to_string(), num(original_tokens)),
        ("returnedTokens".to_string(), num(returned_tokens)),
        ("postprocessFailures".to_string(), num(postprocess_failures)),
    ]);
    if group_by_route {
        bucket.insert(
            "orphanRefetches".to_string(),
            Value::Object(Map::from_iter([
                ("count".to_string(), num(orphan_count)),
                ("bytes".to_string(), num(orphan_bytes)),
            ])),
        );
        bucket.insert("unkeyedRecords".to_string(), num(unkeyed_records));
    }

    let by_route = groups
        .into_iter()
        .map(|(route, agg)| {
            let refetch = joined.get(route).copied().unwrap_or_default();
            Map::from_iter([
                ("route".to_string(), Value::from(route)),
                ("executions".to_string(), num(agg.executions)),
                ("succeeded".to_string(), num(agg.succeeded)),
                ("failed".to_string(), num(agg.failed)),
                ("originalTokens".to_string(), num(agg.original_tokens)),
                ("returnedTokens".to_string(), num(agg.returned_tokens)),
                (
                    "reductionRatio".to_string(),
                    num(if agg.original_tokens > 0.0 {
                        1.0 - agg.returned_tokens / agg.original_tokens
                    } else {
                        0.0
                    }),
                ),
                (
                    "refetches".to_string(),
                    Value::Object(Map::from_iter([
                        ("count".to_string(), num(refetch.0)),
                        ("bytes".to_string(), num(refetch.1)),
                    ])),
                ),
                (
                    "durationsMs".to_string(),
                    Value::Object(Map::from_iter([
                        ("exec".to_string(), num(agg.duration_exec)),
                        ("analyze".to_string(), num(agg.duration_analyze)),
                        ("total".to_string(), num(agg.duration_total)),
                    ])),
                ),
                (
                    "postprocessFailures".to_string(),
                    num(agg.postprocess_failures),
                ),
                (
                    "unmeasurable".to_string(),
                    Value::Object(Map::from_iter([
                        ("tokens".to_string(), num(agg.unmeasurable_tokens)),
                        ("durations".to_string(), num(agg.unmeasurable_durations)),
                    ])),
                ),
            ])
        })
        .collect();
    RouteReport { bucket, by_route }
}

fn execution_key(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some("null".to_string()),
        Value::Number(value) => value.as_f64().map(format_number),
        Value::Bool(_) | Value::String(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

fn number_map(values: BTreeMap<String, f64>) -> Map<String, Value> {
    values
        .into_iter()
        .map(|(key, value)| (key, num(value)))
        .collect()
}

fn build_by_error(records: &[Value]) -> Vec<Map<String, Value>> {
    let mut groups: BTreeMap<String, ErrorAgg> = BTreeMap::new();
    let mut error_count = 0.0;
    for record in records {
        if truthy(get(record, "ok")) {
            continue;
        }
        error_count += 1.0;
        let code = get(record, "error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let agg = groups.entry(code.to_string()).or_default();
        agg.count += 1.0;
        agg.duration_ms += or_zero(get(record, "durationMs"));
        if let Some(worker) = str_field(record, "worker") {
            *agg.workers.entry(worker.to_string()).or_default() += 1.0;
        }
        if let (Some(provider), Some(id)) = (
            get(record, "model")
                .and_then(|model| model.get("provider"))
                .and_then(Value::as_str),
            get(record, "model")
                .and_then(|model| model.get("id"))
                .and_then(Value::as_str),
        ) {
            *agg.models.entry(format!("{provider}/{id}")).or_default() += 1.0;
        }
        if let Some(reason) = get(record, "error")
            .and_then(|error| error.get("reason"))
            .and_then(Value::as_str)
            .filter(|reason| ABORT_REASONS.contains(reason))
        {
            *agg.reasons.entry(reason.to_string()).or_default() += 1.0;
        }
        if let Some(execution) = get(record, "execution").and_then(Value::as_object) {
            for (field, target) in [
                ("timeoutMs", &mut agg.exec_timeout_ms),
                ("maxTurns", &mut agg.exec_max_turns),
                ("maxToolCalls", &mut agg.exec_max_tool_calls),
            ] {
                if let Some(key) = execution.get(field).and_then(execution_key) {
                    *target.entry(key).or_default() += 1.0;
                }
            }
        }
    }

    let mut rows = groups.into_iter().collect::<Vec<_>>();
    rows.sort_by(|(left_code, left), (right_code, right)| {
        right
            .count
            .total_cmp(&left.count)
            .then_with(|| left_code.cmp(right_code))
    });
    rows.into_iter()
        .map(|(code, agg)| {
            let execution = Map::from_iter([
                (
                    "timeoutMs".to_string(),
                    Value::Object(number_map(agg.exec_timeout_ms)),
                ),
                (
                    "maxTurns".to_string(),
                    Value::Object(number_map(agg.exec_max_turns)),
                ),
                (
                    "maxToolCalls".to_string(),
                    Value::Object(number_map(agg.exec_max_tool_calls)),
                ),
            ]);
            Map::from_iter([
                ("code".to_string(), Value::from(code)),
                ("count".to_string(), num(agg.count)),
                (
                    "shareOfRuns".to_string(),
                    num(agg.count / records.len() as f64),
                ),
                ("shareOfFailures".to_string(), num(agg.count / error_count)),
                (
                    "workers".to_string(),
                    Value::Object(number_map(agg.workers)),
                ),
                ("models".to_string(), Value::Object(number_map(agg.models))),
                ("durationMs".to_string(), num(agg.duration_ms)),
                (
                    "reasons".to_string(),
                    Value::Object(number_map(agg.reasons)),
                ),
                ("execution".to_string(), Value::Object(execution)),
            ])
        })
        .collect()
}

/// `buildUsageReport`。usage.jsonl → route window/scope + run filter → 集計 →(compare-model +
/// pricing が揃い、直接比較可能な run があるときのみ)estimated。
/// 戻り値は `UsageReport` object(`Map`、TS literal 順)で、caller が
/// utility envelope の `result` に埋める。
pub fn build_usage_report(
    user_config_dir: &Path,
    filters: &UsageFilters,
    group_by: Option<UsageGroupBy>,
    compare_model: Option<&str>,
    compare_catalog_model: Option<ModelInfo>,
    now: &dyn Fn() -> f64,
) -> Result<Map<String, Value>, CheaperError> {
    let all = read_usage_records(user_config_dir);
    let window = route_window(&all, filters.since_ms, now);
    let routes = build_route_report(
        &window,
        filters.route.as_deref(),
        group_by == Some(UsageGroupBy::Route),
    );
    let records = filter_run_records(all, filters, now);
    let agg = aggregate(&records)?;
    let mut report = report_map(records.len(), &agg);
    report.insert("route".to_string(), Value::Object(routes.bucket));
    if group_by == Some(UsageGroupBy::Route) {
        report.insert(
            "byRoute".to_string(),
            Value::Array(routes.by_route.into_iter().map(Value::Object).collect()),
        );
    }
    if group_by == Some(UsageGroupBy::Error) {
        report.insert(
            "byError".to_string(),
            Value::Array(
                build_by_error(&records)
                    .into_iter()
                    .map(Value::Object)
                    .collect(),
            ),
        );
    }

    let Some(cm) = compare_model else {
        return Ok(report);
    };
    let Some(mref) = parse_model_ref(cm) else {
        return Ok(report);
    };
    let model = compare_catalog_model.unwrap_or(ModelInfo {
        provider: mref.provider,
        id: mref.id,
        cost: None,
        max_tokens: None,
    });

    // runtime-catalog cost は resolve_pricing 内で最優先。snapshot 由来が
    // user file から返った場合は provenance を "pricing.json" に読み替える。
    let user_snapshot = load_pricing_snapshot(user_config_dir);
    let mut pricing: Option<ModelPricing> = None;
    let mut provenance: Option<&'static str> = None;
    if let Some(snapshot) = &user_snapshot {
        pricing = resolve_pricing(&model, Some(snapshot));
        if let Some(p) = &pricing {
            provenance = Some(match p.source {
                PricingSource::BundledSnapshot => "pricing.json",
                other => other.as_str(),
            });
        }
    }
    if pricing.is_none() {
        pricing = resolve_pricing(&model, None);
        provenance = pricing.as_ref().map(|p| p.source.as_str());
    }
    let (Some(pricing), Some(prov)) = (pricing, provenance) else {
        return Ok(report);
    };
    let stale = is_stale_pricing(&pricing, now());
    if let Some(estimated) = estimate(&agg, &pricing, prov, stale, cm) {
        report.insert("estimated".to_string(), estimated);
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// runtime catalog 観測(cli.ts `lookupCatalogModel` 相当)
// ---------------------------------------------------------------------------

/// `toModelInfo`(runtime-pi)相当 — `CatalogModel` → core `ModelInfo`。
fn catalog_model_info(m: cheaper_runtime::CatalogModel) -> ModelInfo {
    ModelInfo {
        provider: m.provider,
        id: m.id,
        cost: Some(ModelCost {
            input: m.cost.input,
            output: m.cost.output,
            cache_read: Some(m.cost.cache_read),
            cache_write: Some(m.cost.cache_write),
        }),
        max_tokens: Some(m.max_tokens as u64),
    }
}

/// `lookupCatalogModel` — `--compare-model` 用に runtime catalog から対象
/// model の観測(cost 等)を取る。集計は read-only が本筋のため、catalog
/// 不在・runtime 構築失敗・panic は全て `None`(estimated を諦めるだけで
/// report には影響させない best-effort、TS の catch 相当)。
fn lookup_catalog_model(mref: &ModelRef, user_config_dir: &Path) -> Option<ModelInfo> {
    let lookup = || -> Option<ModelInfo> {
        // TS `createModelRuntime({env, refreshOnCreate:false})` —
        // 永続 catalog + builtin static model のみを見る offline 版が
        // `Runtime::create`。bundled plugin(devin)も登録する
        // (`bundledProviderPlugins` 相当)。
        let runtime = crate::compose::create_runtime(
            user_config_dir,
            #[cfg(feature = "dev-seam")]
            None,
        )
        .ok()?;
        runtime
            .catalog()
            .get_model(&mref.provider, &mref.id)
            .map(catalog_model_info)
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(lookup))
        .ok()
        .flatten()
}

// ---------------------------------------------------------------------------
// `cli.ts` usage 分岐
// ---------------------------------------------------------------------------

fn invalid_request(message: String, detail_key: &str, detail_value: &str) -> BoxError {
    let details = Map::from_iter([(detail_key.to_string(), Value::from(detail_value))]);
    Box::new(crate::errors::invalid_request(message, Some(details)))
}

/// `cli.ts` の `parsed.kind === "usage"` 分岐相当。usage command 自体は
/// 記録対象外(dispatch 側も record_usage を呼ばない)。
/// `--format text` は dispatch の `write_output` → `format_text` が
/// `text_format::format_usage_report` へ分岐して出す。
/// `cheaper.cli.command` span(design §instrumentation 配置)。
pub async fn execute_usage(
    args: &UsageCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    builtin_dir: &Path,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    execute_usage_inner(args, io, hooks, builtin_dir, started_at)
        .instrument(tracing::info_span!(
            "cheaper.cli.command",
            command = "usage",
        ))
        .await
}

async fn execute_usage_inner(
    args: &UsageCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    builtin_dir: &Path,
    started_at: f64,
) -> Result<CommandOut, BoxError> {
    let since_ms =
        match &args.since {
            Some(s) => Some(parse_duration(s).map_err(|_| {
                invalid_request(format!("invalid --since duration: {s}"), "since", s)
            })? as f64),
            None => None,
        };
    let group_by = match args.group_by.as_deref() {
        Some("route") => Some(UsageGroupBy::Route),
        Some("error") => Some(UsageGroupBy::Error),
        Some(value) => {
            return Err(invalid_request(
                format!("invalid --group-by: {value} (expected route|error)"),
                "groupBy",
                value,
            ));
        }
        None => None,
    };
    if let Some(m) = &args.model {
        if parse_model_ref(m).is_none() {
            return Err(invalid_request(
                format!("invalid --model: {m} (expected <provider>/<id>)"),
                "model",
                m,
            ));
        }
    }
    let compare_ref = match &args.compare_model {
        Some(m) => Some(parse_model_ref(m).ok_or_else(|| {
            invalid_request(
                format!("invalid --compare-model: {m} (expected <provider>/<id>)"),
                "compareModel",
                m,
            )
        })?),
        None => None,
    };
    // usage.jsonl の read-only 集計。worker/model 解決を伴わないため
    // config dir のみ解決する。
    let dirs = resolve_config_dirs(&ConfigDirsOptions {
        cwd: Path::new(&io.cwd),
        builtin_dir,
        env: Some(io.env),
        homedir: None,
    });
    // runtime catalog の cost を estimated の第1供給源として渡す
    // (未取得でも snapshot 経路で試行される)。
    let compare_catalog_model = compare_ref
        .as_ref()
        .and_then(|mref| lookup_catalog_model(mref, &dirs.user_config_dir));
    let filters = UsageFilters {
        since_ms,
        worker: args.worker.clone(),
        provider: args.provider.clone(),
        model: args.model.clone(),
        route: args.route.clone(),
    };
    // `Date.now` は整数 ms — `cli::system_now` の truncating clock
    // を共有する(端数 ms を残すと envelope parity が崩れる)。
    let now_fn = || crate::cli::now(hooks);
    let report = build_usage_report(
        &dirs.user_config_dir,
        &filters,
        group_by,
        args.compare_model.as_deref(),
        compare_catalog_model,
        &now_fn,
    )?;
    let envelope = utility_success_envelope(report, now_fn() - started_at);
    Ok(CommandOut {
        envelope,
        exit_code: 0,
        model_line: None,
    })
}
