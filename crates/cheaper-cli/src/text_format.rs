//! Port of `packages/cli/src/text-format.ts` — `--format text` の
//! 人間向け出力(機械可読ではない)。
//!
//! envelope は `serde_json::Value` として受け取り、TS と同じ分岐順
//! (workflow → error → utility → success)でテキスト化する。number は
//! `format_number`(JS `${n}` 相当)を使う。

use cheaper_core::json::number::format_number;
use cheaper_core::json::stringify::stringify_pretty;
use serde_json::Value;

fn pretty(v: &Value) -> String {
    stringify_pretty(v)
}

fn num(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => format_number(n.as_f64().unwrap_or(0.0)),
        _ => "-".to_string(),
    }
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key)
}

/// JS `padEnd` 相当(UTF-16 code unit 長の近似として char count)。
fn pad_end(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    format!("{s}{}", " ".repeat(width - len))
}

/// JS `Number.prototype.toFixed(digits)` 相当。
fn to_fixed(v: f64, digits: usize) -> String {
    format!("{v:.digits$}")
}

fn count_map(value: Option<&Value>) -> String {
    let Some(map) = value.and_then(Value::as_object) else {
        return String::new();
    };
    map.iter()
        .map(|(key, count)| format!("{key}={}", num(Some(count))))
        .collect::<Vec<_>>()
        .join(" ")
}

fn count_detail(value: Option<&Value>) -> String {
    let Some(map) = value.and_then(Value::as_object) else {
        return "{}".to_string();
    };
    let entries = map
        .iter()
        .map(|(key, count)| format!("{key}:{}", num(Some(count))))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{{{entries}}}")
}

fn usage_text(m: &Value) -> String {
    let usage = get(m, "usage").cloned().unwrap_or(Value::Null);
    format!(
        "in={} out={} cacheRead={} cacheWrite={} total={} cost=${}",
        num(get(&usage, "input")),
        num(get(&usage, "output")),
        num(get(&usage, "cacheRead")),
        num(get(&usage, "cacheWrite")),
        num(get(&usage, "totalTokens")),
        get(&usage, "costUsd")
            .and_then(Value::as_f64)
            .map(|c| to_fixed(c, 6))
            .unwrap_or_else(|| "-".to_string()),
    )
}

fn changes_text(m: &Value, lines: &mut Vec<String>) {
    let changes = get(m, "changes").cloned().unwrap_or(Value::Null);
    let totals = get(&changes, "totals").cloned().unwrap_or(Value::Null);
    lines.push(format!(
        "  {} files (+{}/-{})",
        num(get(&totals, "files")),
        num(get(&totals, "linesAdded")),
        num(get(&totals, "linesDeleted")),
    ));
    if let Some(Value::Array(files)) = get(&changes, "files") {
        for f in files {
            lines.push(format!(
                "  {} {} (+{}/-{})",
                pad_end(get(f, "kind").and_then(Value::as_str).unwrap_or(""), 8),
                get(f, "path").and_then(Value::as_str).unwrap_or(""),
                num(get(f, "linesAdded")),
                num(get(f, "linesDeleted")),
            ));
        }
    }
}

fn success_text(env: &Value) -> String {
    let m = get(env, "meta").cloned().unwrap_or(Value::Null);
    let model = get(&m, "model").cloned().unwrap_or(Value::Null);
    let mut lines = vec![
        "ok".to_string(),
        format!(
            "worker:   {}",
            get(env, "worker").and_then(Value::as_str).unwrap_or("")
        ),
        format!(
            "model:    {}/{} (tier {})",
            get(&model, "provider")
                .and_then(Value::as_str)
                .unwrap_or(""),
            get(&model, "id").and_then(Value::as_str).unwrap_or(""),
            get(&model, "tier").and_then(Value::as_str).unwrap_or(""),
        ),
        format!("usage:    {}", usage_text(&m)),
        format!(
            "turns:    {}  toolCalls={} rejected={} repairs={} retries={}",
            num(get(&m, "turns")),
            num(get(&m, "toolCalls")),
            num(get(&m, "rejectedToolCalls")),
            num(get(&m, "repairs")),
            num(get(&m, "transientRetries")),
        ),
        format!("duration: {}ms", num(get(&m, "durationMs"))),
    ];
    // `changes:  N files (+a/-d)` + file rows
    let head_len = lines.len();
    let mut tail = Vec::new();
    changes_text(&m, &mut tail);
    if let Some(first) = tail.first() {
        lines.push(format!("changes:{}", first));
        lines.extend(tail.into_iter().skip(1));
    }
    debug_assert!(lines.len() > head_len);
    lines.push("result:".to_string());
    lines.push(pretty(get(env, "result").unwrap_or(&Value::Null)));
    format!("{}\n", lines.join("\n"))
}

fn model_row_text(m: &Value) -> String {
    let key = format!(
        "{}/{}",
        get(m, "provider").and_then(Value::as_str).unwrap_or("?"),
        get(m, "id").and_then(Value::as_str).unwrap_or("?"),
    );
    let state = if get(m, "excluded").and_then(Value::as_bool) == Some(true) {
        "excluded"
    } else {
        "available"
    };
    let auto = if get(m, "effectiveCost").is_some_and(Value::is_number)
        && get(m, "effectiveCostPercentile").is_some_and(Value::is_number)
    {
        format!(
            " eff=${}/1M p={} opi={}",
            num(get(m, "effectiveCost")),
            get(m, "effectiveCostPercentile")
                .and_then(Value::as_f64)
                .map(|p| to_fixed(p, 2))
                .unwrap_or_else(|| "?".to_string()),
            get(m, "workload")
                .and_then(|w| get(w, "outputPerInput"))
                .and_then(Value::as_f64)
                .map(|o| num(Some(&Value::from(o))))
                .unwrap_or_else(|| "?".to_string()),
        )
    } else {
        String::new()
    };
    let tier = format!(
        "tier={} ({}){}",
        get(m, "tier").and_then(Value::as_str).unwrap_or("?"),
        get(m, "tierSource").and_then(Value::as_str).unwrap_or("?"),
        auto,
    );
    let pricing = get(m, "pricing").cloned().unwrap_or(Value::Null);
    let price = if pricing.is_null()
        || !get(&pricing, "input").is_some_and(Value::is_number)
        || !get(&pricing, "output").is_some_and(Value::is_number)
    {
        "unpriced".to_string()
    } else {
        // TS `p.snapshotVersion !== undefined ? \`@${p.snapshotVersion}\` : ""`
        // — snapshotVersion は number とは限らず、string snapshot
        // version(例: "2025-09")もそのまま `@` 接頭で出す。
        let snapshot = match get(&pricing, "snapshotVersion") {
            Some(Value::String(s)) => format!("@{s}"),
            Some(Value::Number(n)) => format!("@{}", format_number(n.as_f64().unwrap_or(0.0))),
            _ => String::new(),
        };
        let updated = match get(&pricing, "updatedAt").and_then(Value::as_str) {
            Some(u) => format!(", {u}"),
            None => String::new(),
        };
        format!(
            "in=${} out=${}/1M ({}{}{})",
            num(get(&pricing, "input")),
            num(get(&pricing, "output")),
            get(&pricing, "source")
                .and_then(Value::as_str)
                .unwrap_or("?"),
            snapshot,
            updated,
        )
    };
    format!(
        "  {} {} {}",
        pad_end(&key, 44),
        pad_end(state, 9),
        pad_end(&tier, 28)
    ) + &price
}

fn history_page_text(result: &Value) -> String {
    let records = get(result, "records").cloned().unwrap_or(Value::Null);
    let scan_complete = get(result, "scanComplete").and_then(Value::as_bool) == Some(true);
    let mut lines = vec!["ok history".to_string()];
    if let Some(trace_id) = get(result, "traceId").and_then(Value::as_str) {
        lines.push(format!("traceId: {trace_id}"));
    }
    lines.push(format!(
        "records: {}",
        records.as_array().map_or(0, Vec::len)
    ));
    lines.push(format!(
        "scan: {}",
        if scan_complete {
            "complete"
        } else {
            "incomplete"
        }
    ));
    lines.push("records:".to_string());
    lines.push(pretty(&records));
    lines.push(format!(
        "nextCursor: {}",
        get(result, "nextCursor")
            .and_then(Value::as_str)
            .unwrap_or("null")
    ));
    if !scan_complete {
        lines.push("incomplete scan; narrow filters or restart query".to_string());
    }
    format!("{}\n", lines.join("\n"))
}

fn history_show_text(result: &Value) -> String {
    let state = get(result, "recordState")
        .and_then(Value::as_str)
        .unwrap_or("corrupt");
    let mut lines = vec!["ok history".to_string(), format!("recordState: {state}")];
    if state == "not-found" {
        lines.push("not found".to_string());
    } else {
        lines.push("record:".to_string());
        lines.push(pretty(get(result, "record").unwrap_or(&Value::Null)));
    }
    format!("{}\n", lines.join("\n"))
}

fn utility_text(env: &Value) -> String {
    let r = get(env, "result").cloned().unwrap_or(Value::Null);
    // `cheaper route check --format text` は generic utility fallback
    // ではなく専用 deterministic renderer を使う。
    if get(&r, "sampleProvided").is_some()
        && get(&r, "routes").is_some_and(Value::is_array)
        && get(&r, "policy").is_some_and(Value::is_object)
    {
        return crate::route::check::text_report(&r);
    }
    // `cheaper usage --format text`: TS の cli.ts は formatText を通さず
    // `formatUsageReport(report)` を直接書く。Rust 側は dispatch の
    // `write_output` が envelope 経由でここへ来るため、report 形状
    // (`result.records` number + `result.measured` object)を検出して
    // 同じ出力を返す。
    if get(&r, "records").is_some_and(Value::is_number)
        && get(&r, "measured").is_some_and(Value::is_object)
    {
        return format_usage_report(&r);
    }
    if get(&r, "records").is_some_and(Value::is_array) && get(&r, "scanComplete").is_some() {
        return history_page_text(&r);
    }
    if get(&r, "recordState").is_some_and(Value::is_string) {
        return history_show_text(&r);
    }
    let meta = get(env, "meta").cloned().unwrap_or(Value::Null);
    let duration = num(get(&meta, "durationMs"));
    let mut lines = vec![
        format!("duration: {duration}ms"),
        "result:".to_string(),
        pretty(&r),
    ];
    if let Some(Value::Array(models)) = get(&r, "models") {
        let mut out = vec!["ok".to_string(), format!("models: {}", models.len())];
        out.extend(models.iter().map(model_row_text));
        out.push(format!("duration: {duration}ms"));
        out.push(String::new());
        return out.join("\n");
    }
    let t = get(&r, "tokenizer").cloned().unwrap_or(Value::Null);
    if get(&r, "path").is_some_and(Value::is_string)
        && get(&r, "tokens").is_some_and(Value::is_number)
        && get(&t, "name").is_some_and(Value::is_string)
    {
        lines.insert(0, "ok".to_string());
        lines.insert(
            1,
            format!(
                "path:      {}",
                get(&r, "path").and_then(Value::as_str).unwrap_or("")
            ),
        );
        lines.insert(2, format!("tokens:    {}", num(get(&r, "tokens"))));
        lines.insert(
            3,
            format!(
                "bytes:     {}",
                get(&r, "bytes")
                    .and_then(Value::as_f64)
                    .map(format_number)
                    .unwrap_or_else(|| "-".to_string())
            ),
        );
        lines.insert(
            4,
            format!(
                "tokenizer: {} ({} {})",
                get(&t, "name").and_then(Value::as_str).unwrap_or(""),
                get(&t, "implementation")
                    .and_then(Value::as_str)
                    .unwrap_or("?"),
                get(&t, "version").and_then(Value::as_str).unwrap_or("?"),
            ),
        );
    } else {
        lines.insert(0, "ok".to_string());
    }
    format!("{}\n", lines.join("\n"))
}

fn error_text(env: &Value) -> String {
    let e = get(env, "error").cloned().unwrap_or(Value::Null);
    let m = get(env, "meta").cloned().unwrap_or(Value::Null);
    let mut lines = vec![
        format!(
            "error {}",
            get(&e, "code").and_then(Value::as_str).unwrap_or("")
        ),
        format!(
            "worker:   {}",
            get(env, "worker").and_then(Value::as_str).unwrap_or("-")
        ),
        format!(
            "message:  {}",
            get(&e, "message").and_then(Value::as_str).unwrap_or("")
        ),
        format!(
            "exit:     {}  retryable={}",
            num(get(&e, "exitCode")),
            get(&e, "retryable")
                .and_then(Value::as_bool)
                .map(|b| if b { "true" } else { "false" })
                .unwrap_or("false"),
        ),
        format!("duration: {}ms", num(get(&m, "durationMs"))),
    ];
    if !get(&m, "changes").is_none_or(Value::is_null) {
        let mut tail = Vec::new();
        changes_text(&m, &mut tail);
        if let Some(first) = tail.first() {
            lines.push(format!("changes:{}", first));
            lines.extend(tail.into_iter().skip(1));
        }
    }
    lines.push("error:".to_string());
    lines.push(pretty(&e));
    format!("{}\n", lines.join("\n"))
}

fn workflow_text(env: &Value) -> String {
    let ok = get(env, "ok").and_then(Value::as_bool) == Some(true);
    let mut lines = vec![
        if ok {
            "ok workflow".to_string()
        } else {
            format!(
                "error {}",
                get(env, "error")
                    .and_then(|e| get(e, "code"))
                    .and_then(Value::as_str)
                    .unwrap_or("TOOL_EXECUTION_FAILURE")
            )
        },
        format!(
            "duration: {}ms",
            num(get(env, "meta").and_then(|m| get(m, "durationMs")))
        ),
        "result:".to_string(),
        pretty(get(env, "result").unwrap_or(&Value::Null)),
        "jobs:".to_string(),
        pretty(get(env, "jobs").unwrap_or(&Value::Null)),
    ];
    if let Some(err) = get(env, "error").filter(|e| !e.is_null()) {
        lines.splice(
            1..1,
            [
                format!(
                    "message: {}",
                    get(err, "message").and_then(Value::as_str).unwrap_or("")
                ),
                format!("exit: {}", num(get(err, "exitCode"))),
            ],
        );
    }
    format!("{}\n", lines.join("\n"))
}

/// `formatUsageReport` — `cheaper usage --format text`(T4 で
/// `UsageReport` が `Value` として渡る)。TS と同じ行構成で出力する。
pub fn format_usage_report(r: &Value) -> String {
    let usage = get(r, "usage").cloned().unwrap_or(Value::Null);
    let measured = get(r, "measured").cloned().unwrap_or(Value::Null);
    let route = get(r, "route").cloned().unwrap_or(Value::Null);
    let mut lines = vec![
        "ok".to_string(),
        format!(
            "runs:          {} (ok={} errors={})",
            num(get(r, "records")),
            num(get(r, "ok")),
            num(get(r, "errors")),
        ),
        format!(
            "worker tokens: in={} out={} cacheRead={} cacheWrite={} total={}",
            num(get(&usage, "input")),
            num(get(&usage, "output")),
            num(get(&usage, "cacheRead")),
            num(get(&usage, "cacheWrite")),
            num(get(&usage, "totalTokens")),
        ),
        format!(
            "worker cost:   ${}",
            get(&usage, "costUsd")
                .and_then(Value::as_f64)
                .map(|c| to_fixed(c, 6))
                .unwrap_or_default(),
        ),
        format!(
            "turns:         {}  toolCalls={}  duration={}ms",
            num(get(r, "turns")),
            num(get(r, "toolCalls")),
            num(get(r, "durationMs")),
        ),
        format!(
            "avoided parent context: source={} returned={} avoided={} ({} runs measured)",
            num(get(&measured, "sourceTokens")),
            num(get(&measured, "returnedTokens")),
            num(get(&measured, "avoidedContextTokens")),
            num(get(&measured, "runs")),
        ),
    ];
    lines.push(format!(
        "route:         executions={} failures={} refetches={} refetchBytes={}",
        num(get(&route, "executions")),
        num(get(&route, "failures")),
        num(get(&route, "refetches")),
        num(get(&route, "refetchBytes")),
    ));
    lines.push(format!(
        "route tokens:  original={} returned={} postprocessFailures={}",
        num(get(&route, "originalTokens")),
        num(get(&route, "returnedTokens")),
        num(get(&route, "postprocessFailures")),
    ));
    if let Some(orphan) = get(&route, "orphanRefetches") {
        lines.push(format!(
            "route scope:   orphanRefetches={} orphanRefetchBytes={} unkeyedRecords={}",
            num(get(orphan, "count")),
            num(get(orphan, "bytes")),
            num(get(&route, "unkeyedRecords")),
        ));
    }
    if let Some(rows) = get(r, "byRoute").and_then(Value::as_array) {
        lines.push("byRoute:".to_string());
        for row in rows {
            lines.push(format!(
                "  {}: executions={} ok={} failed={} tokens={}->{} reduction={} postprocessFailures={}",
                get(row, "route").and_then(Value::as_str).unwrap_or(""),
                num(get(row, "executions")),
                num(get(row, "succeeded")),
                num(get(row, "failed")),
                num(get(row, "originalTokens")),
                num(get(row, "returnedTokens")),
                num(get(row, "reductionRatio")),
                num(get(row, "postprocessFailures")),
            ));
            let durations = get(row, "durationsMs");
            let unmeasurable = get(row, "unmeasurable");
            lines.push(format!(
                "    refetches={} bytes={} exec/analyze/total={}/{}/{}ms unmeasurable={}/{}",
                num(get(row, "refetches").and_then(|v| get(v, "count"))),
                num(get(row, "refetches").and_then(|v| get(v, "bytes"))),
                num(durations.and_then(|v| v.get("exec"))),
                num(durations.and_then(|v| v.get("analyze"))),
                num(durations.and_then(|v| v.get("total"))),
                num(unmeasurable.and_then(|v| v.get("tokens"))),
                num(unmeasurable.and_then(|v| v.get("durations"))),
            ));
        }
    }
    if let Some(rows) = get(r, "byError").and_then(Value::as_array) {
        lines.push("byError:".to_string());
        for row in rows {
            lines.push(format!(
                "  {}: count={} shareOfRuns={} shareOfFailures={} durationMs={}",
                get(row, "code").and_then(Value::as_str).unwrap_or(""),
                num(get(row, "count")),
                num(get(row, "shareOfRuns")),
                num(get(row, "shareOfFailures")),
                num(get(row, "durationMs")),
            ));
            for dimension in ["workers", "models", "reasons"] {
                let values = get(row, dimension).and_then(Value::as_object);
                if values.is_some_and(|values| !values.is_empty()) {
                    lines.push(format!(
                        "    {dimension}: {}",
                        count_map(get(row, dimension))
                    ));
                }
            }
            if let Some(execution) = get(row, "execution") {
                let dimensions = ["timeoutMs", "maxTurns", "maxToolCalls"];
                if dimensions.iter().any(|key| {
                    get(execution, key)
                        .and_then(Value::as_object)
                        .is_some_and(|map| !map.is_empty())
                }) {
                    lines.push(format!(
                        "    execution: timeoutMs={} maxTurns={} maxToolCalls={}",
                        count_detail(get(execution, "timeoutMs")),
                        count_detail(get(execution, "maxTurns")),
                        count_detail(get(execution, "maxToolCalls")),
                    ));
                }
            }
        }
    }
    if let Some(e) = get(r, "estimated").filter(|v| !v.is_null()) {
        let pct = match get(e, "estimatedSavingsPercent").and_then(Value::as_f64) {
            Some(p) => format!(" ({}%)", to_fixed(p, 1)),
            None => String::new(),
        };
        let pricing = get(e, "pricing").cloned().unwrap_or(Value::Null);
        let updated = match get(&pricing, "updatedAt").and_then(Value::as_str) {
            Some(u) => format!(" updatedAt={u}"),
            None => String::new(),
        };
        lines.push(format!(
            "Estimated (compare={}, pricing={}{}):",
            get(e, "compareModel").and_then(Value::as_str).unwrap_or(""),
            get(&pricing, "source")
                .and_then(Value::as_str)
                .unwrap_or(""),
            updated,
        ));
        lines.push(format!(
            "  parent equivalent: ${}",
            get(e, "estimatedParentEquivalentCostUsd")
                .and_then(Value::as_f64)
                .map(|c| to_fixed(c, 6))
                .unwrap_or_default(),
        ));
        lines.push(format!(
            "  savings:           ${}{}",
            get(e, "estimatedSavingsUsd")
                .and_then(Value::as_f64)
                .map(|c| to_fixed(c, 6))
                .unwrap_or_default(),
            pct,
        ));
    }
    format!("{}\n", lines.join("\n"))
}

/// `formatText`(DESIGN-0005)。分岐順: workflow → error →
/// utility(worker null) → success。
pub fn format_text(env: &Value) -> String {
    if get(env, "workflow").and_then(Value::as_bool) == Some(true) {
        return workflow_text(env);
    }
    if get(env, "ok").and_then(Value::as_bool) == Some(false) {
        return error_text(env);
    }
    if get(env, "worker").is_some_and(Value::is_null) {
        return utility_text(env);
    }
    success_text(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// models 行の最小 envelope。`pricing` は差し替え前提の共通形。
    fn models_env(pricing: Value) -> Value {
        json!({
            "ok": true,
            "worker": null,
            "meta": {"durationMs": 1},
            "result": {
                "models": [{
                    "provider": "p",
                    "id": "m",
                    "excluded": false,
                    "tier": "low",
                    "tierSource": "default",
                    "pricing": pricing
                }]
            }
        })
    }

    /// TS `p.snapshotVersion !== undefined ? \`@${p.snapshotVersion}\` : ""`
    /// — string snapshot version はそのまま `@` 接頭で出す。
    #[test]
    fn models_text_preserves_string_snapshot_version() {
        let env = models_env(json!({
            "input": 1, "output": 2,
            "source": "catalog",
            "snapshotVersion": "2025-09"
        }));
        let out = format_text(&env);
        assert!(
            out.contains("catalog@2025-09"),
            "string snapshot version must be preserved: {out}"
        );
    }

    /// number snapshot version も従来どおり JS number 表記で出る。
    #[test]
    fn models_text_preserves_number_snapshot_version() {
        let env = models_env(json!({
            "input": 1, "output": 2,
            "source": "catalog",
            "snapshotVersion": 7
        }));
        let out = format_text(&env);
        assert!(
            out.contains("catalog@7"),
            "number snapshot version must render: {out}"
        );
    }

    /// snapshotVersion 欠落時は suffix なし(TS `undefined` 分岐)。
    #[test]
    fn models_text_omits_missing_snapshot_version() {
        let env = models_env(json!({"input": 1, "output": 2, "source": "catalog"}));
        let out = format_text(&env);
        assert!(out.contains("(catalog)"), "expected `(catalog)`: {out}");
        assert!(!out.contains('@'), "unexpected @ suffix: {out}");
    }
}
