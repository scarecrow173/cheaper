//! Regression harness の canonical normalization — 旧 differential
//! harness(T8)から継承した唯一の共有実装
//! (`docs/test-specs/rust-migration-differential-golden.md` と
//! `docs/test-specs/rust-migration-test-spec.md` の「JSON 比較規約」節を
//! そのまま適用する)。snapshot はこの normalize 適用後の値を固定する。
//!
//! 除外(ignore)対象は厳密比較対象として spec に書かれていない nondeterminism
//! に限定する:
//! - 時刻/実行時間: `meta.durationMs`、`meta.execution.*Ms`
//! - 生成物の ephemeral 値: `tokenizerSource`(cached tokenizer の provenance)
//! - error prose: `error.message`(error `code`/`status`/`retryable`/
//!   `details.path`/`details.errors[].instancePath` は厳密比較を維持)
//! - 実装依存 path: workspace/config/home/tmp path → token
//! - JSON object key 順序(Value 比較は key 順非依存)
//!
//! 厳密比較を維持するもの:
//! - envelope shape / `ok` / `error.code` / exit code / `result` 全体
//! - `meta.usage` 6 fields(`costUsd` 含む)、`turns`、`toolCalls`、
//!   `transientRetries`、`repairs`、`reason` 等の counter/分類
//! - `worker`/`meta.model`(id/provider/tier/effort/contextWindow/
//!   effectiveCost/effectiveCostPercentile)
//! - `meta.execution` の状態遷移 counter(budget/spawnedTool... 数値)と
//!   `changedFiles`/`files`/`loadedRefs`
//! - `cause` envelope には再帰適用(message は中でも除外)

use serde_json::{Map, Value};

/// `normalize(v, scrub)` — `scrub` は `(token, raw_path)` pair 列。
/// `raw_path` は long→short 順に sort してから適用する(prefix 衝突で
/// nested path が partial replace されないよう)。
pub fn normalize(v: &mut Value, scrub: &[(String, String)]) {
    normalize_inner(v, &sorted_scrub(scrub));
}

fn sorted_scrub(scrub: &[(String, String)]) -> Vec<(String, String)> {
    let mut s = scrub.to_vec();
    s.sort_by_key(|a| std::cmp::Reverse(a.1.len()));
    s
}

/// object から timing key を除去する。`*Ms` の suffix match でなく、
/// spec が規定する正確な key 集合のみを対象にする。
fn normalize_obj(obj: &mut Map<String, Value>, scrub: &[(String, String)]) {
    // durationMs / tokenizer — timing と tokenizer provenance
    // (`implementation`/`version`/`tokenizerSource` — tiktoken-rs vs
    // js-tiktoken の実装識別子)は nondeterministic/実装依存。token 数値
    // 自体(`tokens`/`bytes`)は厳密比較を維持する。
    for key in ["durationMs", "tokenizer"] {
        obj.remove(key);
    }
    // `execution` — `*Ms` timing field のみ除去し、counter は残す。
    if let Some(exec) = obj.get_mut("execution").and_then(Value::as_object_mut) {
        let timing_keys: Vec<String> = exec.keys().filter(|k| k.ends_with("Ms")).cloned().collect();
        for k in timing_keys {
            exec.remove(&k);
        }
        for v in exec.values_mut() {
            normalize_inner(v, scrub);
        }
    }
    // `error` — prose 系 key(`message`/`errorMessage`/`cause.message` 等)
    // を error subtree 全体から再帰的に除去する。code/status/retryable/
    // details.path/details.errors[].instancePath は厳密比較を維持。
    if let Some(err) = obj.get_mut("error") {
        strip_prose(err);
        if let Some(details) = err.get_mut("details").and_then(Value::as_object_mut) {
            if let Some(Value::Array(errors)) = details.get_mut("errors") {
                for e in errors.iter_mut() {
                    normalize_inner(e, scrub);
                }
                errors.sort_by_key(instance_path);
                errors.dedup_by(|a, b| instance_path(a) == instance_path(b));
            }
            for v in details.values_mut() {
                normalize_inner(v, scrub);
            }
        }
        for v in err
            .as_object_mut()
            .map(|e| e.values_mut())
            .into_iter()
            .flatten()
        {
            normalize_inner(v, scrub);
        }
    }
    for v in obj.values_mut() {
        normalize_inner(v, scrub);
    }
}

/// error subtree 内の prose key を再帰除去する。`cause` envelope の
/// message や `details.errorMessage`(provider error 本文の carrier)も
/// 対象 — 分類用の code/details 構造は残す。
fn strip_prose(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.remove("message");
            o.remove("errorMessage");
            // `cause` は error serialization — object 形(code/details)は
            // 厳密比較を維持し、string 形(OS IO error 本文等の実装依存
            // prose)は除外する。
            if matches!(o.get("cause"), Some(Value::String(_))) {
                o.remove("cause");
            }
            // `searched` — worker/model 探索 path 列。builtin dir の
            // 展開先は version 付き temp dir(`cheaper-builtin-*`)で
            // 実行毎に変わり得るため除去する。workerNotFound の分類
            // 自体は `code` で比較。
            o.remove("searched");
            for inner in o.values_mut() {
                strip_prose(inner);
            }
        }
        Value::Array(a) => {
            for inner in a.iter_mut() {
                strip_prose(inner);
            }
        }
        _ => {}
    }
}

/// `details.errors[]` の sort key — `instancePath` で stable 化。
fn instance_path(v: &Value) -> String {
    v.get("instancePath")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn normalize_inner(v: &mut Value, scrub: &[(String, String)]) {
    match v {
        Value::Object(o) => normalize_obj(o, scrub),
        Value::Array(a) => {
            for item in a.iter_mut() {
                normalize_inner(item, scrub);
            }
        }
        Value::String(s) => {
            let replaced = scrub_string(s, scrub);
            *s = replaced;
        }
        _ => {}
    }
}

/// 実装依存 path token への scrub。Windows path は `\`/`/` 混在し得るので
/// 両 separator 表現で scrub する。
fn scrub_string(s: &str, scrub: &[(String, String)]) -> String {
    let mut out = s.to_string();
    for (token, raw) in scrub {
        if raw.is_empty() {
            continue;
        }
        let fwd = raw.replace('\\', "/");
        let bwd = raw.replace('/', "\\");
        out = out.replace(raw.as_str(), token);
        out = out.replace(fwd.as_str(), token);
        out = out.replace(bwd.as_str(), token);
        // JSON-escaped backslash(`C:\\…`) — normalize は parse 後の
        // unescaped 文字列に対して動くため通常不要だが、stderr/raw 比較や
        // 二重 serialize の保険として残す。
        let esc = raw.replace('\\', "\\\\");
        out = out.replace(esc.as_str(), token);
    }
    out
}

/// normalized `a`/`b` の差分を人間可読な行の列で返す(空なら一致)。
/// object は set-union key で再帰、array は index 対応。差分は最深部の
/// primitive leaf で報告する。
pub fn diff(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                match (x.get(k), y.get(k)) {
                    (Some(xv), Some(yv)) => diff(xv, yv, &format!("{path}.{k}"), out),
                    (Some(xv), None) => {
                        out.push(format!("{path}.{k}: only in rust = {}", preview(xv)))
                    }
                    (None, Some(yv)) => {
                        out.push(format!("{path}.{k}: only in ts = {}", preview(yv)))
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            for i in 0..x.len().max(y.len()) {
                match (x.get(i), y.get(i)) {
                    (Some(xv), Some(yv)) => diff(xv, yv, &format!("{path}[{i}]"), out),
                    (Some(xv), None) => {
                        out.push(format!("{path}[{i}]: only in rust = {}", preview(xv)))
                    }
                    (None, Some(yv)) => {
                        out.push(format!("{path}[{i}]: only in ts = {}", preview(yv)))
                    }
                    (None, None) => unreachable!(),
                }
            }
        }
        _ => {
            if a != b {
                out.push(format!(
                    "{path}: rust = {} | ts = {}",
                    preview(a),
                    preview(b)
                ));
            }
        }
    }
}

/// raw stdout(stderr 含む)比較向けの scrub を公開する。envelope mode は
/// `normalize` 内部で string 毎に適用するため、これは mode=`raw` のみ使う。
pub fn scrub_public(s: &str, scrub: &[(String, String)]) -> String {
    scrub_string(s, &sorted_scrub(scrub))
}

fn preview(v: &Value) -> String {
    let s = serde_json::to_string(v).unwrap_or_else(|_| "?".to_string());
    if s.chars().count() > 160 {
        format!("{}…", s.chars().take(160).collect::<String>())
    } else {
        s
    }
}
