//! Port of `packages/cli/src/usage-log.ts` — `usage.jsonl` record 構築
//! と best-effort append(ADR-0025 / DESIGN-0027)。
//!
//! record は `serde_json::Value`(insertion order = TS object literal
//! 順)として構築し、`stringify(record) + "\n"` で append する。
//! 書き込み先は `<userConfigDir>/usage.jsonl` のみ — workspace /
//! `.cheaper/` には書かない。

use cheaper_core::json::stringify::stringify;
use serde_json::{Map, Value};

use crate::cli::CliWrite;

pub const USAGE_LOG_FILENAME: &str = "usage.jsonl";

/// `Date.prototype.toISOString` 相当(ms epoch →
/// `YYYY-MM-DDTHH:MM:SS.sssZ`)。dep を増やさず civil-from-days
/// (Hinnant)で算出する。
fn iso8601(ms_epoch: f64) -> String {
    if !ms_epoch.is_finite() {
        return String::new();
    }
    let ms_i = ms_epoch as i64;
    let secs = ms_i.div_euclid(1000);
    let millis = ms_i.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400);
    // civil_from_days(Hinnant): era/year-of-era → March-based doy を
    // 計算したあと、月長テーブルを順に減算して month/day を得る
    // (定数 formula より検証しやすい)。
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let mut y = yoe + era * 400;
    let mut rem = doe - (365 * yoe + yoe / 4 - yoe / 100); // March 1 起算
    let mut m: i64 = 3;
    loop {
        let month_len = match m {
            2 => {
                // m==2 に達した時点の y は既に Gregorian year(m=1 で
                // +1 済み)。
                if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if rem < month_len {
            break;
        }
        rem -= month_len;
        m += 1;
        if m == 13 {
            m = 1;
            y += 1;
        }
    }
    let d = rem + 1;
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z")
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get(key)
}

fn is_number(v: &Value) -> bool {
    v.is_number()
}

/// `buildUsageRecord` — usage を持つ run envelope(success + meta.usage
/// 付き error)と count-tokens の utility envelope のみ記録対象。
/// `now` は ms epoch を返す。
pub fn build_usage_record(env: &Value, root: &str, now: &dyn Fn() -> f64) -> Option<Value> {
    if get(env, "workflow").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let ts = iso8601(now());
    let duration_ms = get(env, "meta")
        .and_then(|m| get(m, "durationMs"))
        .cloned()
        .unwrap_or(Value::Null);
    let mut base = Map::new();
    base.insert("schemaVersion".to_string(), Value::from(1));
    base.insert("ts".to_string(), Value::from(ts));
    base.insert("root".to_string(), Value::from(root));
    base.insert("durationMs".to_string(), duration_ms);

    let meta = get(env, "meta").cloned().unwrap_or(Value::Null);
    let ok = get(env, "ok").and_then(Value::as_bool) == Some(true);
    let worker = get(env, "worker").cloned().unwrap_or(Value::Null);
    if ok {
        if worker.is_null() {
            // utility envelope: count-tokens のみ(`result.tokens` を
            // 持つもの)を記録。
            let result = get(env, "result")?;
            let tokens = get(result, "tokens")?;
            if !is_number(tokens) {
                return None;
            }
            base.insert("kind".to_string(), Value::from("count"));
            base.insert("worker".to_string(), Value::Null);
            base.insert("ok".to_string(), Value::Bool(true));
            let mut r = Map::new();
            if let Some(p) = get(result, "path").filter(|v| v.is_string()) {
                r.insert("path".to_string(), p.clone());
            }
            r.insert("tokens".to_string(), tokens.clone());
            if let Some(b) = get(result, "bytes").filter(|v| is_number(v)) {
                r.insert("bytes".to_string(), b.clone());
            }
            base.insert("result".to_string(), Value::Object(r));
            return Some(Value::Object(base));
        }
        base.insert("kind".to_string(), Value::from("run"));
        base.insert("worker".to_string(), worker);
        base.insert("ok".to_string(), Value::Bool(true));
        for key in ["model", "usage", "turns", "toolCalls", "savings"] {
            if let Some(v) = get(&meta, key) {
                base.insert(key.to_string(), v.clone());
            }
        }
        return Some(Value::Object(base));
    }

    // error envelope: runMeta 由来の usage がある run 失敗のみ記録。
    if get(&meta, "usage").is_none_or(Value::is_null)
        || get(&meta, "model").is_none_or(Value::is_null)
    {
        return None;
    }
    base.insert("kind".to_string(), Value::from("run"));
    base.insert("worker".to_string(), worker);
    base.insert("ok".to_string(), Value::Bool(false));
    for key in ["model", "usage"] {
        if let Some(v) = get(&meta, key) {
            base.insert(key.to_string(), v.clone());
        }
    }
    for key in ["turns", "toolCalls", "savings"] {
        if let Some(v) = get(&meta, key).filter(|v| !v.is_null()) {
            base.insert(key.to_string(), v.clone());
        }
    }
    let code = get(env, "error")
        .and_then(|e| get(e, "code"))
        .cloned()
        .unwrap_or(Value::Null);
    base.insert(
        "error".to_string(),
        Value::Object(Map::from_iter([("code".to_string(), code)])),
    );
    Some(Value::Object(base))
}

/// `appendUsageLog` — `<userConfigDir>/usage.jsonl` へ 1 行 append
/// (dir は必要に応じて作成)。
pub fn append_usage_log(user_config_dir: &str, record: &Value) -> std::io::Result<()> {
    std::fs::create_dir_all(user_config_dir)?;
    let path = std::path::Path::new(user_config_dir).join(USAGE_LOG_FILENAME);
    let mut line = stringify(record);
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(line.as_bytes())
        })
}

/// `recordUsage` — best-effort。失敗しても exit code / stdout を変えず、
/// quiet でなければ stderr に `usage-log: <msg>` を出すのみ。
pub fn record_usage(
    env: &Value,
    root: &str,
    user_config_dir: &str,
    stderr: &mut dyn CliWrite,
    quiet: bool,
    now: &dyn Fn() -> f64,
) {
    let result = (|| -> std::io::Result<()> {
        let Some(record) = build_usage_record(env, root, now) else {
            return Ok(());
        };
        append_usage_log(user_config_dir, &record)
    })();
    if let Err(e) = result {
        if !quiet {
            stderr.write_str(&format!("usage-log: {e}\n"));
        }
    }
}
