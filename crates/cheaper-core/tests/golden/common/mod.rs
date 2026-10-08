//! Shared helpers for the golden replay modules.
#![allow(dead_code)]

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

use cheaper_core::errors::CheaperError;
use cheaper_core::workspace::paths::{
    canonicalize_workspace_root, SymlinkPolicy, WorkspaceEscapeError,
};

pub fn fixture(rel: &str) -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("fixtures")
        .join(rel);
    serde_json::from_str(&std::fs::read_to_string(&p).expect("fixture readable"))
        .unwrap_or_else(|e| panic!("fixture {rel} parses: {e}"))
}

pub fn cases(fixture: &Value) -> &[Value] {
    fixture["cases"].as_array().expect("cases array")
}

pub fn case_name(case: &Value) -> String {
    case["name"].as_str().unwrap_or("<unnamed>").to_string()
}

/// `axis` gating: `"win32"`/`"posix"` cases run only on the matching
/// platform; `"any"` (or absent) runs everywhere.
pub fn axis_ok(case: &Value) -> bool {
    match case.get("axis").and_then(Value::as_str).unwrap_or("any") {
        "win32" => cfg!(windows),
        "posix" => !cfg!(windows),
        _ => true,
    }
}

pub fn opt_str<'v>(case: &'v Value, key: &str) -> Option<&'v str> {
    case.get(key).and_then(Value::as_str)
}

/// TS `input ?? {}` semantics: `null` collapses to the same defaults as
/// `undefined`, so a recorded `null` input must reach the Rust port as
/// `None`, not `Some(Value::Null)`.
pub fn opt_input<'v>(case: &'v Value, key: &str) -> Option<&'v Value> {
    case.get(key).filter(|v| !v.is_null())
}

// ---------- filesystem setup ----------

fn rel_path(rel: &str) -> PathBuf {
    rel.split('/').collect()
}

/// Public variant of [`rel_path`] for sibling modules.
pub fn rel_path_pub(rel: &str) -> PathBuf {
    rel_path(rel)
}

pub fn write_files(dir: &Path, files: &Value) {
    let Some(obj) = files.as_object() else { return };
    for (rel, content) in obj {
        let p = dir.join(rel_path(rel));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        if let Some(b64) = content.get("base64").and_then(Value::as_str) {
            std::fs::write(&p, decode_base64(b64)).unwrap();
        } else {
            std::fs::write(&p, content.as_str().unwrap_or_default()).unwrap();
        }
    }
}

fn decode_base64(s: &str) -> Vec<u8> {
    // Minimal base64 decode (fixtures only carry tiny binary blobs).
    let table = |c: u8| -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("bad base64 char"),
        }
    };
    let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let n = chunk.len();
        let mut acc: u32 = 0;
        for (i, c) in chunk.iter().enumerate() {
            acc |= (table(*c) as u32) << (18 - 6 * i);
        }
        if n > 1 {
            out.push((acc >> 16) as u8);
        }
        if n > 2 {
            out.push((acc >> 8) as u8);
        }
        if n > 3 {
            out.push(acc as u8);
        }
    }
    out
}

/// Directory symlink (junction on Windows — no privilege needed).
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()?;
        if status.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "mklink /J failed: {}",
                String::from_utf8_lossy(&status.stdout)
            )))
        }
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link)
    }
}

fn create_symlinks(dir: &Path, out: &Path, list: &Value) -> bool {
    let Some(list) = list.as_array() else {
        return true;
    };
    for s in list {
        let target = match s["target"].as_str().unwrap_or_default() {
            "<OUT>" => out.to_path_buf(),
            t => dir.join(rel_path(t)),
        };
        let link = dir.join(rel_path(s["link"].as_str().unwrap()));
        if symlink_dir(&target, &link).is_err() {
            return false;
        }
    }
    true
}

// ---------- per-case tmp workspace ----------

pub struct CaseDir {
    _tmp: tempfile::TempDir,
    /// The (realpath'd) per-case base directory that holds `files`.
    pub dir: PathBuf,
    /// Canonical workspace root (== `dir` unless `rootRel` is used).
    pub root: String,
    /// `<TOKEN>` → absolute path bindings, longest-path first.
    pub needles: Vec<(String, String)>,
}

pub fn case_dir(case: &Value) -> Option<CaseDir> {
    let tmp = tempfile::tempdir().unwrap();
    let base = dunce::canonicalize(tmp.path())
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let dir = tmp.path().join("case");
    std::fs::create_dir_all(&dir).unwrap();
    write_files(&dir, &case["files"]);
    let out = tmp.path().join("outside");
    std::fs::create_dir_all(&out).unwrap();
    if !create_symlinks(&dir, &out, &case["symlinks"]) {
        return None;
    }
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    let out_c = dunce::canonicalize(&out)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let abs = abs_pred_base();
    let fsroot = Path::new(&abs)
        .ancestors()
        .last()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let mut needles: Vec<(String, String)> = vec![
        ("<ROOT>".to_string(), root.clone()),
        ("<OUT>".to_string(), out_c),
        ("<ABS>".to_string(), abs),
        ("<FSROOT>".to_string(), fsroot),
        ("<BASE>".to_string(), base),
    ];
    needles.sort_by_key(|a| std::cmp::Reverse(a.1.len()));
    Some(CaseDir {
        _tmp: tmp,
        dir: realpath_dir(&dir),
        root,
        needles,
    })
}

fn realpath_dir(p: &Path) -> PathBuf {
    dunce::canonicalize(p).unwrap()
}

// ---------- token (un)binding ----------

fn replace_tokens(s: &str, needles: &[(String, String)], forward: bool) -> String {
    let mut out = s.to_string();
    for (token, path) in needles {
        if forward {
            // recorded value → real path
            out = out.replace(token.as_str(), path);
        } else if path == "/" {
            // POSIX fsroot is the bare separator; only bind it at the
            // start of the value so interior separators of other paths
            // are not rewritten to `<FSROOT>`.
            if let Some(rest) = out.strip_prefix('/') {
                out = format!("{token}{rest}");
            }
        } else {
            // real path → token (both separator spellings, plus the
            // JSON-escaped form inside serialized strings like
            // `resourceKey` output)
            out = out.replace(path.as_str(), token.as_str());
            out = out.replace(&path.replace('\\', "/"), token.as_str());
            out = out.replace(&path.replace('\\', "\\\\"), token.as_str());
        }
    }
    out
}

fn walk(v: &Value, f: &dyn Fn(&str) -> String) -> Value {
    match v {
        Value::String(s) => Value::String(f(s)),
        Value::Array(a) => Value::Array(a.iter().map(|x| walk(x, f)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), walk(x, f)))
                .collect::<Map<_, _>>(),
        ),
        _ => v.clone(),
    }
}

/// Bind `<TOKEN>` occurrences in recorded inputs to real paths.
pub fn unscrub(v: &Value, needles: &[(String, String)]) -> Value {
    walk(v, &|s| replace_tokens(s, needles, true))
}

/// Replace real paths in produced outputs by `<TOKEN>`s.
pub fn scrub(v: &Value, needles: &[(String, String)]) -> Value {
    walk(v, &|s| replace_tokens(s, needles, false))
}

/// Slash normalization for path comparisons (config dirs fixture uses
/// POSIX-style inputs on every platform).
pub fn slashify(s: &str) -> String {
    s.replace('\\', "/")
}

// ---------- error normalization ----------

/// Apply the plan's normalization rules to error `details`:
/// `errors` arrays collapse to `instancePath`, `errno` keys are dropped.
pub fn normalize_details(v: Value) -> Value {
    walk_owned(v)
}

fn walk_owned(v: Value) -> Value {
    match v {
        Value::Object(mut o) => {
            o.remove("errno");
            if let Some(Value::Array(errors)) = o.get_mut("errors") {
                for e in errors.iter_mut() {
                    if let Some(obj) = e.as_object() {
                        let path = obj.get("instancePath").cloned().unwrap_or(Value::Null);
                        *e = json!({ "instancePath": path });
                    }
                }
            }
            Value::Object(
                o.into_iter()
                    .map(|(k, x)| (k, walk_owned(x)))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Array(a) => Value::Array(a.into_iter().map(walk_owned).collect()),
        other => other,
    }
}

/// `cap(e)` for `CheaperError`: code + exitCode + retryable + normalized,
/// token-scrubbed details.
pub fn cheaper_err(e: &CheaperError, needles: &[(String, String)]) -> Value {
    let details = e
        .details()
        .map(|d| normalize_details(scrub(&Value::Object(d.clone()), needles)))
        .unwrap_or(Value::Null);
    json!({
        "kind": "cheaper",
        "code": e.code().as_str(),
        "exitCode": e.exit_code(),
        "retryable": e.retryable(),
        "details": details,
    })
}

pub fn escape_err(e: &WorkspaceEscapeError, needles: &[(String, String)]) -> Value {
    json!({
        "kind": "escape",
        "input": scrub(&json!(e.input()), needles),
        "reason": e.reason(),
    })
}

/// The dump builds pure path-string inputs via `path.join(...)` (native
/// separators) but records them with `/` so fixtures stay
/// platform-independent. Replay must apply the same join before calling
/// the ported string-prefix predicates.
pub fn native_path(s: &str) -> String {
    if cfg!(windows) {
        s.replace('/', "\\")
    } else {
        s.to_string()
    }
}

/// JS number emission: JSON.stringify serializes integral f64s without a
/// decimal point (`20`, not `20.0`); `serde_json` distinguishes int/float
/// `Number` values, so f64 results must be emitted as integers when the
/// value is integral and safe.
pub fn js_num(x: f64) -> Value {
    if x.is_finite() && x.fract() == 0.0 && x.abs() <= 9_007_199_254_740_992.0 {
        Value::from(x as i64)
    } else {
        Value::from(x)
    }
}

// ---------- result comparison ----------

/// Separator normalization: fixtures were dumped on one platform and
/// replayed on any; `\` and `/` spellings of the same path compare equal.
/// Values whose `\` is data (NUL bytes, literal escape sequences) are not
/// affected — those characters differ from the `\` byte used as a
/// separator.
pub fn sep_norm(v: &Value) -> Value {
    walk(v, &|s| s.replace('\\', "/"))
}

/// Compare a produced `Result<Value, Value>` (error already in `cap`
/// shape) against the case's recorded `expected`.
pub fn check(name: &str, expected: &Value, produced: Result<Value, Value>) {
    let expected = sep_norm(expected);
    match (
        expected.get("ok"),
        produced.map(|v| sep_norm(&v)).map_err(|e| sep_norm(&e)),
    ) {
        (Some(want), Ok(got)) => assert_eq!(&got, want, "{name}: ok mismatch"),
        (None, Err(got_err)) => {
            let want = &expected["error"];
            assert_eq!(&got_err, want, "{name}: error mismatch")
        }
        (Some(want), Err(got)) => panic!("{name}: expected ok {want}, got error {got}"),
        (None, Ok(got)) => panic!("{name}: expected error {}, got ok {got}", expected["error"]),
    }
}

/// Extract the produced error from a `Result<_, E>` via `f`.
pub fn err_of<T, E>(r: Result<T, E>, f: impl FnOnce(&E) -> Value) -> Value {
    match r {
        Err(e) => f(&e),
        Ok(_) => panic!("expected error, got ok"),
    }
}

pub fn symlink_policy(v: &Value) -> SymlinkPolicy {
    match v.as_str() {
        Some("within-workspace") => SymlinkPolicy::WithinWorkspace,
        _ => SymlinkPolicy::Deny,
    }
}

/// Join a token-free relative path to `base`. The exact byte shape is
/// irrelevant — produced paths are compared after `sep_norm` and token
/// scrubbing.
pub fn join(base: &str, rel: &str) -> String {
    Path::new(base)
        .join(rel.split('/').collect::<PathBuf>())
        .to_string_lossy()
        .into_owned()
}

/// `path.resolve("golden-pred-base")` — the synthetic absolute base the
/// predicate fixtures bind `<ABS>` to. The dump resolves it from the
/// repository root; `cargo test` runs with the crate directory as cwd,
/// so anchor at `CARGO_MANIFEST_DIR/../..` instead.
pub fn abs_pred_base() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("golden-pred-base")
        .to_string_lossy()
        .into_owned()
}

/// Serialize a `WorkerDefinition` into the dump shape used by
/// `dumpWorkerDef` in `scripts/dump-core-golden.mjs`.
pub fn dump_worker_def(
    def: &cheaper_core::worker::validate::WorkerDefinition,
    needles: &[(String, String)],
) -> Value {
    let mut model = Map::new();
    model.insert("tier".to_string(), json!(def.cheaper.model.tier.as_str()));
    if let Some(w) = &def.cheaper.model.workload {
        model.insert(
            "workload".to_string(),
            json!({"outputPerInput": w.output_per_input}),
        );
    }
    let mut cheaper = Map::new();
    cheaper.insert("model".to_string(), Value::Object(model));
    if let Some(input) = &def.cheaper.input {
        cheaper.insert(
            "input".to_string(),
            json!({"schema": serde_json::to_value(&input.schema).unwrap_or(Value::Null)}),
        );
    }
    cheaper.insert(
        "output".to_string(),
        json!({"schema": serde_json::to_value(&def.cheaper.output.schema).unwrap_or(Value::Null)}),
    );
    if let Some(ws) = &def.cheaper.write_scope {
        cheaper.insert("writeScope".to_string(), json!(ws));
    }
    let v = json!({
        "name": def.name,
        "description": def.description,
        "tools": def.tools.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
        "cheaper": cheaper,
        "instructions": def.instructions,
        "origin": def.origin.as_str(),
        "sourcePath": def.source_path,
        "directlyComparable": cheaper_core::worker::validate::is_directly_comparable_worker(def),
        "inputSchema": def.input_schema.as_ref().map(|s| json!({"schema": s.schema})),
        "outputSchema": {"schema": def.output_schema.schema},
    });
    scrub(&v, needles)
}

// ---------- model dump/build helpers (shared by model_* modules) ----------

use cheaper_core::model::classify::{Classification, ModelInfo};
use cheaper_core::model::config::{ModelConfig, TierPreferences};
use cheaper_core::model::model_ref::ModelRef;
use cheaper_core::model::pricing::{ModelCost, ModelPricing, PricingSnapshotEntry};
use cheaper_core::model::resolve::ResolvedModel;

fn f64_of(v: &Value) -> f64 {
    v.as_f64().expect("number")
}

fn opt_f64(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

pub fn model_cost_from(v: &Value) -> ModelCost {
    ModelCost {
        input: f64_of(&v["input"]),
        output: f64_of(&v["output"]),
        cache_read: opt_f64(v, "cacheRead"),
        cache_write: opt_f64(v, "cacheWrite"),
    }
}

pub fn model_from(v: &Value) -> ModelInfo {
    ModelInfo {
        provider: v["provider"].as_str().unwrap().to_string(),
        id: v["id"].as_str().unwrap().to_string(),
        cost: v.get("cost").map(model_cost_from),
        max_tokens: None,
    }
}

/// `dumpModel` — `{provider, id, cost?}` (cost keys camelCase, only when
/// present).
pub fn dump_model(m: &ModelInfo) -> Value {
    let mut o = Map::new();
    o.insert("provider".to_string(), json!(m.provider));
    o.insert("id".to_string(), json!(m.id));
    if let Some(c) = &m.cost {
        let mut cost = Map::new();
        cost.insert("input".to_string(), js_num(c.input));
        cost.insert("output".to_string(), js_num(c.output));
        if let Some(v) = c.cache_read {
            cost.insert("cacheRead".to_string(), js_num(v));
        }
        if let Some(v) = c.cache_write {
            cost.insert("cacheWrite".to_string(), js_num(v));
        }
        o.insert("cost".to_string(), Value::Object(cost));
    }
    Value::Object(o)
}

/// `dumpPricing` — same optional-field omission as the dump script.
pub fn dump_pricing(p: &ModelPricing) -> Value {
    let mut o = Map::new();
    o.insert("input".to_string(), js_num(p.input));
    o.insert("output".to_string(), js_num(p.output));
    if let Some(v) = p.cache_read {
        o.insert("cacheRead".to_string(), js_num(v));
    }
    if let Some(v) = p.cache_write {
        o.insert("cacheWrite".to_string(), js_num(v));
    }
    o.insert("currency".to_string(), json!(p.currency));
    o.insert("unit".to_string(), json!(p.unit));
    o.insert("source".to_string(), json!(p.source.as_str()));
    if let Some(v) = &p.snapshot_version {
        o.insert("snapshotVersion".to_string(), json!(v));
    }
    if let Some(v) = &p.updated_at {
        o.insert("updatedAt".to_string(), json!(v));
    }
    Value::Object(o)
}

fn dump_auto(a: &cheaper_core::model::classify::AutoClassificationDetail) -> Value {
    json!({
        "effectiveCost": js_num(a.effective_cost),
        "effectiveCostPercentile": js_num(a.effective_cost_percentile),
        "outputPerInput": js_num(a.output_per_input),
    })
}

/// `dumpClassification`.
pub fn dump_classification(c: &Classification) -> Value {
    let mut o = Map::new();
    o.insert("tier".to_string(), json!(c.tier.as_str()));
    o.insert("source".to_string(), json!(c.source.as_str()));
    if let Some(auto) = &c.auto {
        o.insert("auto".to_string(), dump_auto(auto));
    }
    Value::Object(o)
}

/// `dumpResolved`.
pub fn dump_resolved(r: &ResolvedModel) -> Value {
    let mut o = Map::new();
    o.insert("model".to_string(), dump_model(&r.model));
    o.insert("tier".to_string(), json!(r.tier.as_str()));
    o.insert("tierSource".to_string(), json!(r.tier_source.as_str()));
    if let Some(p) = &r.pricing {
        o.insert("pricing".to_string(), dump_pricing(p));
    }
    if let Some(a) = &r.auto {
        o.insert("auto".to_string(), dump_auto(a));
    }
    o.insert("selection".to_string(), json!(r.selection.as_str()));
    Value::Object(o)
}

/// `{overrides, preferences, exclude?}` JSON → `ModelConfig`.
pub fn model_config_from(v: &Value) -> ModelConfig {
    let overrides = v
        .get("overrides")
        .and_then(Value::as_object)
        .map(|o| {
            o.iter()
                .map(|(k, t)| (k.clone(), cheaper_core::model::tier::parse_tier(t).unwrap()))
                .collect()
        })
        .unwrap_or_default();
    let pref = |tier: &str| -> Option<Vec<String>> {
        v.get("preferences")
            .and_then(|p| p.get(tier))
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
    };
    ModelConfig {
        overrides,
        preferences: TierPreferences {
            low: pref("low"),
            medium: pref("medium"),
            high: pref("high"),
        },
        exclude: v.get("exclude").and_then(Value::as_array).map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(String::from))
                .collect()
        }),
    }
}

/// `available`/`models` JSON arrays → `Vec<ModelInfo>`.
pub fn models_from(v: &Value) -> Vec<ModelInfo> {
    v.as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(model_from)
        .collect()
}

pub fn model_ref_from(v: &Value) -> ModelRef {
    ModelRef {
        provider: v["provider"].as_str().unwrap().to_string(),
        id: v["id"].as_str().unwrap().to_string(),
    }
}

/// `PricingSnapshot` uses `&'static str` fields — the fixture strings are
/// leaked so the snapshot can be built at replay time.
pub fn pricing_snapshot_from(v: &Value) -> Option<cheaper_core::model::pricing::PricingSnapshot> {
    if v.is_null() {
        return None;
    }
    let leak = |s: &str| -> &'static str { Box::leak(s.to_string().into_boxed_str()) };
    let models = v["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            let arr = pair.as_array().unwrap();
            let key = arr[0].as_str().unwrap().to_string();
            let e = &arr[1];
            (
                key,
                PricingSnapshotEntry {
                    input: f64_of(&e["input"]),
                    output: f64_of(&e["output"]),
                    cache_read: opt_f64(e, "cacheRead"),
                    cache_write: opt_f64(e, "cacheWrite"),
                    updated_at: e.get("updatedAt").and_then(Value::as_str).map(String::from),
                },
            )
        })
        .collect();
    Some(cheaper_core::model::pricing::PricingSnapshot {
        version: leak(v["version"].as_str().unwrap()),
        updated_at: leak(v["updatedAt"].as_str().unwrap()),
        models,
    })
}
