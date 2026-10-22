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

/// 先頭 `n` 文字(char / Unicode scalar 基準)に切り詰める。
/// `cheaper-runtime` の `repair.rs` の同名 helper と同一規約 —
/// byte 境界の中間で multi-byte 文字を壊さない(runtime 側は private
/// のため再利用不可。KISS の local helper)。
fn truncate_chars(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars().take(n).collect()
}

/// `RunAbortReason::as_str` の abort 語彙(runtime `abort.rs`) —
/// `error.details.reason` が record へ載る許容値集合。語彙外の値は
/// `reason` key 自体を省略する(fail-soft)。
const ABORT_REASONS: [&str; 4] = ["timeout", "signal", "maxTurns", "maxToolCalls"];

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
        // `version` — run を実行した binary 自身の product version
        // (compile-time 定数、cheaper-cli で評価)。
        base.insert(
            "version".to_string(),
            Value::from(env!("CARGO_PKG_VERSION")),
        );
        for key in ["model", "usage", "turns", "toolCalls", "savings"] {
            if let Some(v) = get(&meta, key) {
                base.insert(key.to_string(), v.clone());
            }
        }
        // `repairs` — non-null のみ(error 分岐の `!is_null` 規約と同一)。
        if let Some(v) = get(&meta, "repairs").filter(|v| !v.is_null()) {
            base.insert("repairs".to_string(), v.clone());
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
    // `version` — success 分岐と同一規約(`ok` 直後・常時)。
    base.insert(
        "version".to_string(),
        Value::from(env!("CARGO_PKG_VERSION")),
    );
    for key in ["model", "usage"] {
        if let Some(v) = get(&meta, key) {
            base.insert(key.to_string(), v.clone());
        }
    }
    for key in ["turns", "toolCalls", "savings", "repairs"] {
        if let Some(v) = get(&meta, key).filter(|v| !v.is_null()) {
            base.insert(key.to_string(), v.clone());
        }
    }
    base.insert("error".to_string(), error_record_object(env));
    Some(Value::Object(base))
}

/// error record の `error` object — 固定 allowlist projection
/// (DESIGN-034KqI29dOTRXERRPTp5gV / ADR Option A)。挿入順は
/// `code` → `reason` → `message` → `schemaErrors` → `errorMessage` →
/// `checkpoint` で固定(determinism)。値が無い・型が合わない field は
/// key 自体を省略する fail-soft。allowlist 外(`rawOutput`/
/// `rawOutputPath`/`exitCode`/`retryable`/`cause`/その他 details
/// key)は一切写さない。
fn error_record_object(env: &Value) -> Value {
    let error = get(env, "error");
    let details = error.and_then(|e| get(e, "details"));
    let mut obj = Map::new();
    let code = error
        .and_then(|e| get(e, "code"))
        .cloned()
        .unwrap_or(Value::Null);
    obj.insert("code".to_string(), code);
    // `reason` — abort 語彙に一致する `details.reason` string のみ。
    if let Some(reason) = details
        .and_then(|d| get(d, "reason"))
        .and_then(Value::as_str)
        .filter(|r| ABORT_REASONS.contains(r))
    {
        obj.insert("reason".to_string(), Value::from(reason));
    }
    // `message` — `error.message` string の先頭 300 文字。
    if let Some(message) = error
        .and_then(|e| get(e, "message"))
        .and_then(Value::as_str)
    {
        obj.insert(
            "message".to_string(),
            Value::from(truncate_chars(message, 300)),
        );
    }
    // `schemaErrors` — `details.errors` array の object entry 先頭 5 件を
    // `{instancePath ≤200 文字, message ≤120 文字}` で写す。
    // 欠落 key はその key を省略し、写せる entry が無ければ key 自体を
    // 省略する。
    if let Some(errors) = details
        .and_then(|d| get(d, "errors"))
        .and_then(Value::as_array)
    {
        let projected: Vec<Value> = errors
            .iter()
            .filter_map(Value::as_object)
            .take(5)
            .map(|entry| {
                let mut e = Map::new();
                if let Some(p) = entry.get("instancePath").and_then(Value::as_str) {
                    e.insert(
                        "instancePath".to_string(),
                        Value::from(truncate_chars(p, 200)),
                    );
                }
                if let Some(m) = entry.get("message").and_then(Value::as_str) {
                    e.insert("message".to_string(), Value::from(truncate_chars(m, 120)));
                }
                Value::Object(e)
            })
            .collect();
        if !projected.is_empty() {
            obj.insert("schemaErrors".to_string(), Value::Array(projected));
        }
    }
    // `errorMessage` — `details.errorMessage` string の先頭 300 文字。
    if let Some(m) = details
        .and_then(|d| get(d, "errorMessage"))
        .and_then(Value::as_str)
    {
        obj.insert(
            "errorMessage".to_string(),
            Value::from(truncate_chars(m, 300)),
        );
    }
    // `checkpoint` — `details.checkpoint` object の `{path,
    // observationsCount}` のみ写す。両 key 欠落なら `checkpoint`
    // 自体を省略(best-effort sidecar の参照のみ)。
    if let Some(cp) = details
        .and_then(|d| get(d, "checkpoint"))
        .and_then(Value::as_object)
    {
        let mut c = Map::new();
        for key in ["path", "observationsCount"] {
            if let Some(v) = cp.get(key) {
                c.insert(key.to_string(), v.clone());
            }
        }
        if !c.is_empty() {
            obj.insert("checkpoint".to_string(), Value::Object(c));
        }
    }
    Value::Object(obj)
}

/// `kind:"route"` record の入力(R34)— `route run` の計測値を
/// caller が束ねる。env 値・credential は field に含めない
/// (構造保証)。
pub struct RouteRecordInput {
    /// route 名(宣言 `name`)。
    pub route: String,
    /// 原 shell command 文字列 — record には `DefaultHasher` hex
    /// の `commandHash` のみ載せ、本文は残さない。
    pub command: String,
    /// capture した原 output の token 数。
    pub original_tokens: f64,
    /// 親へ返した compact result の token 数。
    pub returned_tokens: f64,
    /// exec job の worker meta(`{model, usage}`)。
    pub worker: Value,
    /// durationMs 内訳 — `{exec, analyze, total}`。
    pub duration_ms: Value,
    /// route log の runId。
    pub log_ref: String,
    /// write 時点までの `route logs` refetch 観測数(通常 0)。
    pub refetches: f64,
    /// postprocess の縮退数(partial-count・tee sink 失敗等)。
    pub postprocess_failures: f64,
    pub suppressed: Option<String>,
    /// exit mirror した子 exit code(usage-bearing failure も対象)。
    pub exit_code: i32,
    /// 成否 — `false` は usage-bearing failure record。
    pub ok: bool,
}

/// `kind:"route"` record(R34)を構築する。`commandHash` は
/// `embedded_fingerprint` と同じ `DefaultHasher` hex 規約(同一
/// toolchain 内のみ安定 — design C14f)。
pub fn build_route_record(input: &RouteRecordInput, root: &str, now: &dyn Fn() -> f64) -> Value {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    input.command.hash(&mut h);
    let reduction = if input.original_tokens > 0.0 {
        1.0 - input.returned_tokens / input.original_tokens
    } else {
        0.0
    };
    let mut base = Map::new();
    base.insert("schemaVersion".to_string(), Value::from(1));
    base.insert("ts".to_string(), Value::from(iso8601(now())));
    base.insert("root".to_string(), Value::from(root));
    base.insert("kind".to_string(), Value::from("route"));
    base.insert("route".to_string(), Value::from(input.route.clone()));
    base.insert(
        "commandHash".to_string(),
        Value::from(format!("{:016x}", h.finish())),
    );
    base.insert(
        "originalTokens".to_string(),
        Value::from(input.original_tokens),
    );
    base.insert(
        "returnedTokens".to_string(),
        Value::from(input.returned_tokens),
    );
    base.insert("reductionRatio".to_string(), Value::from(reduction));
    base.insert("worker".to_string(), input.worker.clone());
    base.insert("durationMs".to_string(), input.duration_ms.clone());
    base.insert("logRef".to_string(), Value::from(input.log_ref.clone()));
    base.insert("refetches".to_string(), Value::from(input.refetches));
    base.insert(
        "postprocessFailures".to_string(),
        Value::from(input.postprocess_failures),
    );
    base.insert("exitCode".to_string(), Value::from(input.exit_code));
    base.insert("ok".to_string(), Value::Bool(input.ok));
    if let Some(suppressed) = &input.suppressed {
        base.insert("suppressed".to_string(), Value::from(suppressed.clone()));
    }
    Value::Object(base)
}

/// `kind:"route-refetch"` record — `route logs <id>` 1 実行ごとの
/// append。`cheaper usage` が `logRef` で route record と join して
/// 真の refetch 数を出す(JSONL は append-only で元 record を
/// 更新できないための 2 面構成)。
pub fn build_route_refetch_record(
    log_ref: &str,
    bytes_read: u64,
    root: &str,
    now: &dyn Fn() -> f64,
) -> Value {
    let mut base = Map::new();
    base.insert("schemaVersion".to_string(), Value::from(1));
    base.insert("ts".to_string(), Value::from(iso8601(now())));
    base.insert("root".to_string(), Value::from(root));
    base.insert("kind".to_string(), Value::from("route-refetch"));
    base.insert("logRef".to_string(), Value::from(log_ref));
    base.insert("bytesRead".to_string(), Value::from(bytes_read));
    Value::Object(base)
}

/// `record_usage` と同じ best-effort append 規約の route 版 —
/// `route run`(D1)が呼ぶ。失敗は stderr 行のみで exit/stdout 不変。
pub fn record_route(
    input: &RouteRecordInput,
    root: &str,
    user_config_dir: &str,
    stderr: &mut dyn CliWrite,
    quiet: bool,
    now: &dyn Fn() -> f64,
) -> bool {
    let record = build_route_record(input, root, now);
    match append_usage_log(user_config_dir, &record) {
        Ok(()) => true,
        Err(e) => {
            if !quiet {
                stderr.write_str(&format!("usage-log: {e}\n"));
            }
            false
        }
    }
}

/// `route logs <id>`(D4)が呼ぶ refetch record の best-effort
/// append。
pub fn record_route_refetch(
    log_ref: &str,
    bytes_read: u64,
    root: &str,
    user_config_dir: &str,
    stderr: &mut dyn CliWrite,
    quiet: bool,
    now: &dyn Fn() -> f64,
) {
    let record = build_route_refetch_record(log_ref, bytes_read, root, now);
    if let Err(e) = append_usage_log(user_config_dir, &record) {
        if !quiet {
            stderr.write_str(&format!("usage-log: {e}\n"));
        }
    }
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
