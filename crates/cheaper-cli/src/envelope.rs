//! Port of `packages/cli/src/envelope.ts` — `schemaVersion: 1` の 4 形態
//! (success / utility / error / workflow)。
//!
//! byte 互換の要点:
//! - key 順は TS の object literal 構築順(insertion order。serde_json
//!   `preserve_order` で保持)。
//! - number は [`cheaper_core::json::stringify::stringify`] が
//!   `JSON.stringify` 規則で format する(`1.0`→`1`、非有限→`null`)。
//!   envelope は `serde_json::Value` として構築し、`envelope_json` が
//!   `stringify(env) + "\n"` を返す。
//! - error 形態の `details`/`cause` は値が無いとき key 自体を省略し、
//!   `meta` の未取得 field は `null`(null 意味論)。
//! - `cause` は CheaperError(TS `isCheaperError` — WorkerRunError も
//!   含む subclass 相当)のときのみ `{code,message}` として出す。

use std::error::Error;

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::json::stringify::stringify;
use cheaper_runtime::{PartialRunMeta, WorkerRunError, WorkerRunMeta};
use serde_json::{Map, Value};

pub const SCHEMA_VERSION: u64 = 1;

fn object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k.to_string(), v);
    }
    Value::Object(m)
}

fn opt_number(v: Option<f64>) -> Value {
    // `Value::from(f64)` は非有限値を Null に写す(`Number::from_f64`
    // の None → Null) — `JSON.stringify` の NaN/Infinity → null と一致。
    v.map(Value::from).unwrap_or(Value::Null)
}

/// `successEnvelope`: `{schemaVersion, ok:true, worker, result,
/// meta:{durationMs, ...meta}}`。meta field 順は `WorkerRunMeta` の
/// declaration 順(model→…→savings)で `durationMs` が先頭。
pub fn success_envelope(
    worker: &str,
    result: Map<String, Value>,
    meta: &WorkerRunMeta,
    duration_ms: f64,
) -> Value {
    let mut meta_map = Map::new();
    meta_map.insert("durationMs".to_string(), opt_number(Some(duration_ms)));
    if let Value::Object(inner) =
        serde_json::to_value(meta).unwrap_or_else(|_| Value::Object(Map::new()))
    {
        meta_map.extend(inner);
    }
    object([
        ("schemaVersion", Value::from(SCHEMA_VERSION)),
        ("ok", Value::Bool(true)),
        ("worker", Value::from(worker)),
        ("result", Value::Object(result)),
        ("meta", Value::Object(meta_map)),
    ])
}

/// `utilitySuccessEnvelope`: worker run を伴わない utility command。
pub fn utility_success_envelope(result: Map<String, Value>, duration_ms: f64) -> Value {
    object([
        ("schemaVersion", Value::from(SCHEMA_VERSION)),
        ("ok", Value::Bool(true)),
        ("worker", Value::Null),
        ("result", Value::Object(result)),
        (
            "meta",
            object([("durationMs", opt_number(Some(duration_ms)))]),
        ),
    ])
}

/// CheaperError 相当(`CheaperError` または `WorkerRunError`)の
/// `(内部 CheaperError, runMeta)` view。TS の `isCheaperError` 判定と
/// `err.runMeta` 読取りを `dyn Error` downcast で再現する。
fn cheaper_view<'e>(
    e: &'e (dyn Error + 'static),
) -> Option<(&'e CheaperError, Option<&'e PartialRunMeta>)> {
    if let Some(w) = e.downcast_ref::<WorkerRunError>() {
        // WorkerRunError は inner CheaperError が code/message/details/
        // cause を持ち、`runMeta` を別途保持する(TS では同一 object)。
        return Some((w.error(), w.run_meta()));
    }
    e.downcast_ref::<CheaperError>().map(|c| (c, None))
}

/// `errorMeta`(envelope.ts) — `runMeta` 取得不能な field は `null`。
/// `changes` は `runMeta.changes ?? details.changes`(details 側は
/// `typeof === "object" && !== null` — JS では array も object)。
fn error_meta(
    duration_ms: Option<f64>,
    run_meta: Option<&PartialRunMeta>,
    details: Option<&Map<String, Value>>,
) -> Value {
    let details_changes = details
        .and_then(|d| d.get("changes"))
        .and_then(|c| match c {
            Value::Object(_) | Value::Array(_) => Some(c.clone()),
            _ => None,
        });
    let field = |v: Option<Value>| v.unwrap_or(Value::Null);
    let model = run_meta
        .and_then(|m| m.model.as_ref())
        .and_then(|v| serde_json::to_value(v).ok());
    let usage = run_meta
        .and_then(|m| m.usage.as_ref())
        .and_then(|v| serde_json::to_value(v).ok());
    let changes = run_meta
        .and_then(|m| m.changes.as_ref())
        .and_then(|v| serde_json::to_value(v).ok())
        .or(details_changes);
    let savings = run_meta
        .and_then(|m| m.savings.as_ref())
        .and_then(|v| serde_json::to_value(v).ok());
    let execution = run_meta
        .and_then(|m| m.execution.as_ref())
        .and_then(|v| serde_json::to_value(v).ok());
    let u64_field = |f: fn(&PartialRunMeta) -> Option<u64>| run_meta.and_then(f).map(Value::from);
    object([
        ("durationMs", opt_number(duration_ms)),
        ("model", field(model)),
        ("usage", field(usage)),
        ("turns", field(u64_field(|m| m.turns))),
        ("toolCalls", field(u64_field(|m| m.tool_calls))),
        (
            "rejectedToolCalls",
            field(u64_field(|m| m.rejected_tool_calls)),
        ),
        ("repairs", field(u64_field(|m| m.repairs))),
        (
            "transientRetries",
            field(u64_field(|m| m.transient_retries)),
        ),
        ("changes", field(changes)),
        ("savings", field(savings)),
        ("execution", field(execution)),
    ])
}

/// `errorEnvelope`(DESIGN-0005)。CheaperError 相当は code/exitCode/
/// retryable/details/cause をそのまま移し、それ以外は
/// `INTERNAL_ERROR`(70)。`err` は panic 等の非 Error payload を
/// 呼び出し側で CheaperError 化した後に渡す想定だが、素の
/// `io::Error` 等も INTERNAL_ERROR に畳む。
pub fn error_envelope(
    err: &(dyn Error + Send + Sync + 'static),
    worker: Option<&str>,
    duration_ms: Option<f64>,
) -> Value {
    let (code, message, exit_code, retryable, details, cause, run_meta) = match cheaper_view(err) {
        Some((e, run_meta)) => (
            e.code(),
            e.message().to_string(),
            e.exit_code(),
            e.retryable(),
            e.details().cloned(),
            // `cause` は CheaperError 相当のときのみ {code,message}。
            e.source()
                .and_then(|s| cheaper_view(s).map(|(c, _)| (c.code(), c.message().to_string()))),
            run_meta,
        ),
        None => (
            ErrorCode::InternalError,
            err.to_string(),
            ErrorCode::InternalError.exit_code(),
            false,
            None,
            None,
            None,
        ),
    };
    let mut error = Map::new();
    error.insert("code".to_string(), Value::from(code.as_str()));
    error.insert("message".to_string(), Value::from(message));
    error.insert("exitCode".to_string(), Value::from(exit_code));
    error.insert("retryable".to_string(), Value::Bool(retryable));
    if let Some(d) = &details {
        error.insert("details".to_string(), Value::Object(d.clone()));
    }
    if let Some((cause_code, cause_message)) = cause {
        error.insert(
            "cause".to_string(),
            object([
                ("code", Value::from(cause_code.as_str())),
                ("message", Value::from(cause_message)),
            ]),
        );
    }
    object([
        ("schemaVersion", Value::from(SCHEMA_VERSION)),
        ("ok", Value::Bool(false)),
        ("worker", worker.map(Value::from).unwrap_or(Value::Null)),
        ("error", Value::Object(error)),
        ("meta", error_meta(duration_ms, run_meta, details.as_ref())),
    ])
}

/// `WorkflowEnvelope`(`workflow-run.ts` の構築順: `schemaVersion,
/// workflow, ok, worker, result, jobs, meta, ...error?`)。
/// `meta`/`error` の中身は T7(engine outcome 投影)が組み立てる。
pub fn workflow_envelope(
    ok: bool,
    result: Map<String, Value>,
    jobs: Vec<Value>,
    meta: Map<String, Value>,
    error: Option<Value>,
) -> Value {
    let mut m = Map::new();
    m.insert("schemaVersion".to_string(), Value::from(SCHEMA_VERSION));
    m.insert("workflow".to_string(), Value::Bool(true));
    m.insert("ok".to_string(), Value::Bool(ok));
    m.insert("worker".to_string(), Value::Null);
    m.insert("result".to_string(), Value::Object(result));
    m.insert("jobs".to_string(), Value::Array(jobs));
    m.insert("meta".to_string(), Value::Object(meta));
    if let Some(e) = error {
        m.insert("error".to_string(), e);
    }
    Value::Object(m)
}

/// `envelopeJson`: stdout 契約 — envelope 1 JSON + 末尾改行 1 つのみ。
pub fn envelope_json(env: &Value) -> String {
    let mut s = stringify(env);
    s.push('\n');
    s
}
