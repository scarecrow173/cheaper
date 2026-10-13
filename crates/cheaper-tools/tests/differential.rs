//! `scripts/dump-tool-fixtures.mjs` が出力する TS fixture を Rust 実装へ
//! replay する differential harness(T2-8)。
//!
//! fixture format(dump script 冒頭コメント準拠): case は
//! `files`/`repeatFiles`/`symlinks` の workspace 構築、`runtime`
//! (`ToolRuntimeOptions` spec)または `tool`(tool-level 実行 + cap 注入)、
//! `ops`(call / `parallel` / effect `ready|abort-runtime|abort-scope|drain|
//! touch`)、`expected`/`state`(scrub 済み `<ROOT>`/`<OUT>`/`<BASE>` token)。
//! `expectRust` は設計固定ルールの divergence(bare `node` の
//! POLICY_VIOLATION、tiktoken-rs tokenizer identity)を pin する sparse
//! override(`ops`: op index → expected entry、`state`: deep-merge patch)。
//!
//! process fixture は `node -e` を使えない(設計固定ルール: Rust は `node`
//! を PATH 解決しない)ため、`<NODE>` task は `src/bin/diff_fixture_child`
//! (libtest banner を含まない clean stdout の bin target)へ翻訳する。
//! grep eval は `proc_runtime.rs` と同じく `CHEAPER_GREP_EVAL` marker
//! 付き re-exec。

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use cheaper_core::errors::CheaperError;
use cheaper_core::token::count::FileTooLargeError;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
use cheaper_core::workspace::paths::{
    canonicalize_workspace_root, SymlinkPolicy, WorkspaceEscapeError,
};
use cheaper_tools::access::ReadStrictness;
use cheaper_tools::artifact::{
    ArtifactData, ArtifactPorts, ArtifactProducer, ArtifactPublisher, ArtifactPut,
    ArtifactReadRequest, ArtifactReadResult, ArtifactReader, ArtifactReference, ArtifactSource,
};
use cheaper_tools::journal::MutationJournal;
use cheaper_tools::operation::{AbortController, AbortReason};
use cheaper_tools::registry::{
    create_tool_runtime, ToolCallContext, ToolRuntime, ToolRuntimeOptions,
};
use cheaper_tools::tools::grep::{create_grep_tool, GrepToolOptions};
use cheaper_tools::tools::grep_eval::{EvalSpawn, HELPER_ENV};
use cheaper_tools::tools::read::{create_read_tool, ReadToolOptions};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// re-exec helpers(grep eval + process fixture child)
// ---------------------------------------------------------------------------

/// `proc_runtime.rs` と同じ grep eval helper entry。
#[test]
fn grep_eval_helper_entry() {
    if std::env::var_os(HELPER_ENV).is_none() {
        return;
    }
    std::process::exit(cheaper_tools::tools::grep_eval::helper_main());
}

fn fixture_eval_spawn() -> EvalSpawn {
    EvalSpawn {
        program: std::env::current_exe().expect("current exe"),
        args: vec![
            "grep_eval_helper_entry".to_string(),
            "--exact".to_string(),
            "--test-threads=1".to_string(),
        ],
        envs: vec![(HELPER_ENV.to_string(), "1".to_string())],
    }
}

const CHILD_ENV: &str = "CHEAPER_DIFF_CHILD_MODE";
/// `src/bin/diff_fixture_child.rs` — libtest banner を含まない clean
/// stdout のため bin target に分離した fixture child。
const CHILD_BIN: &str = env!("CARGO_BIN_EXE_diff_fixture_child");

/// `<NODE>` + `-e` script → fixture child mode。dump script の `NODE_TASKS`
/// literal と同期を保つ(変えたらここも更新)。
fn script_mode(script: &str) -> &'static str {
    match script {
        "console.log('hello process')" => "hi",
        "console.log('before failure'),console.error('failure detail'),process.exitCode=7" => {
            "fail"
        }
        "console.log('a'.repeat(1000))" => "output",
        "setInterval(function(){},500)" => "sleep",
        s if s.starts_with("setInterval(function(){if(require('fs').existsSync(") => "hold",
        s if s.starts_with("console.log(JSON.stringify({cwdTail:") => "inspect",
        other => panic!("unknown node -e fixture script: {other}"),
    }
}

// ---------------------------------------------------------------------------
// fixture io / workspace materialization
// ---------------------------------------------------------------------------

fn fixture_file(name: &str) -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(&p).expect("fixture readable"))
        .unwrap_or_else(|e| panic!("fixture {name} parses: {e}"))
}

/// dump の `file.replace(/\W/g,"_")` — case dir 名 `c{file_stem}_{i}` は
/// `inspect` task の `cwdTail` が leaf 名を観測するため決定性が必要。
fn file_stem(file: &str) -> String {
    file.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn rel_path(rel: &str) -> PathBuf {
    rel.split('/').collect()
}

fn decode_base64(s: &str) -> Vec<u8> {
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

fn write_files(dir: &Path, files: &Value) {
    let Some(obj) = files.as_object() else { return };
    for (rel, content) in obj {
        let p = dir.join(rel_path(rel));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        if let Some(b64) = content.get("base64").and_then(Value::as_str) {
            std::fs::write(&p, decode_base64(b64)).unwrap();
        } else if let Some(repeat) = content.get("repeat").and_then(Value::as_array) {
            let unit = repeat[0].as_str().unwrap_or_default();
            let times = repeat[1].as_u64().unwrap_or(0) as usize;
            std::fs::write(&p, unit.repeat(times)).unwrap();
        } else if let Some(mode) = content.get("mode").and_then(Value::as_u64) {
            // `{content, mode}`: 書き込み後に chmod(read-only fixture 用)。
            std::fs::write(&p, content["content"].as_str().unwrap_or_default()).unwrap();
            set_mode(&p, mode as u32);
        } else {
            std::fs::write(&p, content.as_str().unwrap_or_default()).unwrap();
        }
    }
}

/// `chmodSync(p, mode)` 相当。Windows には permission bits が無いため
/// write bit 皆無を `readonly` 属性へ写像する(dump 側と同じ近似)。
fn set_mode(path: &Path, mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    #[cfg(not(unix))]
    {
        let mut perm = std::fs::metadata(path).unwrap().permissions();
        perm.set_readonly(mode & 0o222 == 0);
        std::fs::set_permissions(path, perm).unwrap();
    }
}

/// `repeatFiles` spec: `{key: {dir, prefix, count, content}}` →
/// `dir/prefixNNNN.txt`。
fn write_repeat_files(dir: &Path, spec: &Value) {
    let Some(obj) = spec.as_object() else { return };
    for (_key, r) in obj {
        let sub = dir.join(rel_path(r["dir"].as_str().unwrap_or_default()));
        std::fs::create_dir_all(&sub).unwrap();
        let prefix = r["prefix"].as_str().unwrap_or_default();
        let content = r["content"].as_str().unwrap_or_default();
        for i in 0..r["count"].as_u64().unwrap_or(0) {
            std::fs::write(sub.join(format!("{prefix}{i:04}.txt")), content).unwrap();
        }
    }
}

/// directory link を作る。Windows の `junction` は privilege 不要のため
/// `mklink /J` を使う(golden harness と同一手法)。失敗時は `false` で
/// case skip(dump が symlink 不可環境で case を落とすのと同じ扱い)。
fn symlink_dir(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

fn make_symlinks(dir: &Path, out_dir: &Path, list: &Value) -> bool {
    let Some(list) = list.as_array() else {
        return true;
    };
    for s in list {
        let target = match s["target"].as_str().unwrap_or_default() {
            "<OUT>" => out_dir.to_path_buf(),
            t => dir.join(rel_path(t)),
        };
        let link = dir.join(rel_path(s["link"].as_str().unwrap()));
        if !symlink_dir(&target, &link) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// per-case workspace(`TMP_BASE/c{file_stem}_{i}` + sibling `outside`)
// ---------------------------------------------------------------------------

struct TempBase(PathBuf);

impl TempBase {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-tools-diff-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }
}

impl Drop for TempBase {
    fn drop(&mut self) {
        // `{content, mode}` で readonly にした file は Windows で
        // `remove_dir_all` が失敗するため、先に属性を解除する
        // (best-effort — 失敗しても temp に残るだけ)。
        clear_readonly(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(not(unix))]
fn clear_readonly(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() && !p.is_symlink() {
            clear_readonly(&p);
        } else if let Ok(md) = std::fs::metadata(&p) {
            let mut perm = md.permissions();
            if perm.readonly() {
                // cfg(not(unix)) のため unix の world-writable 化リスクは
                // ない(lint は cfg を考慮しない)。
                #[allow(clippy::permissions_set_readonly_false)]
                perm.set_readonly(false);
                let _ = std::fs::set_permissions(&p, perm);
            }
        }
    }
}

#[cfg(unix)]
fn clear_readonly(_: &Path) {}

struct CaseCtx {
    dir: PathBuf,
    /// canonicalized workspace root(`<ROOT>` の実値)。
    root: String,
    needles: Vec<(String, String)>,
    /// `runtime.signal: true` の scope signal 用 controller。
    scope: AbortController,
}

/// `caseDir` 相当。symlink 構築に失敗した場合は `None`(dump 側が case を
/// drop するのと同じ扱い)。
fn case_dir(file: &str, index: usize, spec: &Value, base: &TempBase) -> Option<CaseCtx> {
    let dir = base.0.join(format!("c{}_{}", file_stem(file), index));
    std::fs::create_dir_all(&dir).unwrap();
    write_files(&dir, &spec["files"]);
    write_repeat_files(&dir, &spec["repeatFiles"]);
    let out_dir = base.0.join("outside");
    std::fs::create_dir_all(&out_dir).unwrap();
    if !make_symlinks(&dir, &out_dir, &spec["symlinks"]) {
        return None;
    }
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    let real_dir = PathBuf::from(&root);
    let mut needles = vec![
        ("<ROOT>".to_string(), root.clone()),
        (
            "<OUT>".to_string(),
            canonicalize_workspace_root(out_dir.to_str().unwrap()).unwrap(),
        ),
        (
            "<BASE>".to_string(),
            canonicalize_workspace_root(base.0.to_str().unwrap()).unwrap(),
        ),
    ];
    needles.sort_by_key(|a| std::cmp::Reverse(a.1.len()));
    Some(CaseCtx {
        dir: real_dir,
        root,
        needles,
        scope: AbortController::new(),
    })
}

// ---------------------------------------------------------------------------
// token (un)binding — dump の `scrub`/`unScrub` 相当
// ---------------------------------------------------------------------------

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

/// `<TOKEN>` → 実 path(input/runtime spec の rebinding)。`<NODE>` は
/// process task 翻訳が扱うためここでは残す。
fn unscrub(v: &Value, needles: &[(String, String)]) -> Value {
    walk(v, &|s| {
        let mut out = s.to_string();
        for (token, path) in needles {
            out = out.replace(token.as_str(), path);
        }
        out
    })
}

/// 実 path → `<TOKEN>`。`\`/`/`/`\\`(JSON escape 済み内部文字列)の
/// 3 変種を処理する(dump `scrub` と同じ)。
fn scrub(v: &Value, needles: &[(String, String)]) -> Value {
    walk(v, &|s| {
        let mut out = s.to_string();
        for (token, path) in needles {
            out = out.replace(path.as_str(), token.as_str());
            out = out.replace(&path.replace('\\', "/"), token.as_str());
            out = out.replace(&path.replace('\\', "\\\\"), token.as_str());
        }
        out
    })
}

/// 比較用の platform canonicalization。fixture は win32 dump 由来のため:
/// - `\`/`/` separator と、serialized inner string 内の `\\`(escape 済み
///   separator)を `/` に揃える
/// - `Error code:` の errno 表記差(Windows `EPERM` ↔ POSIX `EACCES`)を
///   canonical token に揃える — Node on POSIX も EACCES を返すため、実
///   TS-on-Linux の値は EACCES であり、どちらの表記も同一意味として比較する
fn compare_norm(v: &Value) -> Value {
    walk(v, &|s| {
        s.replace("\\\\", "/")
            .replace('\\', "/")
            .replace("Error code: EPERM", "Error code: <errno>")
            .replace("Error code: EACCES", "Error code: <errno>")
    })
}

// ---------------------------------------------------------------------------
// error normalization — dump の `cap`/`normalizeDetails` 相当
// ---------------------------------------------------------------------------

fn normalize_details(v: Value) -> Value {
    match v {
        Value::Object(mut o) => {
            o.remove("errno");
            if let Some(Value::Array(errors)) = o.get_mut("errors") {
                // ajv union-branch 重複を sorted unique instancePath へ。
                let mut paths: Vec<String> = errors
                    .iter()
                    .map(|e| {
                        e.get("instancePath")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect();
                paths.sort();
                paths.dedup();
                *errors = paths
                    .into_iter()
                    .map(|p| json!({ "instancePath": p }))
                    .collect();
            }
            Value::Object(
                o.into_iter()
                    .map(|(k, x)| (k, normalize_details(x)))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Array(a) => Value::Array(a.into_iter().map(normalize_details).collect()),
        other => other,
    }
}

/// `cap(e)`: thrown value の normalized capture。
/// - `CheaperError` → `{kind:"cheaper",code,exitCode,retryable,details}`
/// - `WorkspaceEscapeError` → `{kind:"escape",input,reason}`
/// - `FileTooLargeError` → `{kind:"tooLarge",path,bytes,maxBytes}`
/// - `AbortReason` → 内包 error を再帰 capture(reason として元 error が
///   そのまま throw される TS と同一)。payload reason は `{kind:"other"}`。
/// - `std::io::Error`(raw OS error 付き)→ `{kind:"io"}`
/// - `std::io::Error::other` / `DeniedError` / `String` error →
///   `{kind:"other",name:"Error",message}`(TS `new Error(msg)` 相当)
fn cap(err: &(dyn std::error::Error + Send + Sync + 'static)) -> Value {
    if let Some(reason) = err.downcast_ref::<AbortReason>() {
        return match reason.error_object() {
            Some(inner) => cap(inner.as_ref()),
            None => json!({
                "kind": "other",
                "name": reason.name(),
                "message": reason.message(),
            }),
        };
    }
    if let Some(e) = err.downcast_ref::<CheaperError>() {
        let details = e
            .details()
            .map(|d| normalize_details(Value::Object(d.clone())))
            .unwrap_or(Value::Null);
        return json!({
            "kind": "cheaper",
            "code": e.code().as_str(),
            "exitCode": e.exit_code(),
            "retryable": e.retryable(),
            "details": details,
        });
    }
    if let Some(e) = err.downcast_ref::<WorkspaceEscapeError>() {
        return json!({
            "kind": "escape",
            "input": e.input(),
            "reason": e.reason(),
        });
    }
    if let Some(e) = err.downcast_ref::<FileTooLargeError>() {
        return json!({
            "kind": "tooLarge",
            "path": e.path(),
            "bytes": e.bytes(),
            "maxBytes": e.max_bytes(),
        });
    }
    // `path.posix.matchesGlob` over-long pattern — Node の `TypeError`。
    if let Some(e) = err.downcast_ref::<cheaper_tools::glob::PatternTooLongError>() {
        return json!({
            "kind": "other",
            "name": "TypeError",
            "message": e.to_string(),
        });
    }
    // dump の `cap`: `e instanceof Error && typeof e.code === "string"`
    // → `{kind:"io"}`。Rust 側は `NodeIoError`(enum wrap される経路も
    // あるため source chain を走査)または raw `io::Error`(OS error 付き)。
    {
        let mut cur: Option<&(dyn std::error::Error + 'static)> =
            Some(err as &(dyn std::error::Error + 'static));
        while let Some(e) = cur {
            if let Some(io) = e.downcast_ref::<std::io::Error>() {
                if io.raw_os_error().is_some() {
                    return json!({ "kind": "io" });
                }
            }
            cur = e.source();
        }
    }
    json!({
        "kind": "other",
        "name": "Error",
        "message": err.to_string(),
    })
}

/// `cap` for `Result<_, ToolError>`。
fn cap_tool(err: &ToolError) -> Value {
    cap(err.as_ref())
}

// ---------------------------------------------------------------------------
// deterministic artifact ports — dump の `fixtureArtifacts` 相当
// ---------------------------------------------------------------------------

struct FixtureReader {
    data: Vec<u8>,
}

impl ArtifactReader for FixtureReader {
    fn read(
        &self,
        _source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, ToolError> {
        let start = (request.offset as usize).min(self.data.len());
        let end = (start + request.max_bytes as usize).min(self.data.len());
        Ok(ArtifactReadResult {
            data: self.data[start..end].to_vec(),
            offset: start as u64,
            next_offset: end as u64,
            eof: end >= self.data.len(),
        })
    }
}

struct FixturePublisher;

impl ArtifactPublisher for FixturePublisher {
    fn max_artifact_bytes(&self) -> u64 {
        128
    }

    fn put(&self, value: ArtifactPut) -> Result<ArtifactReference, ToolError> {
        let bytes = match value.data {
            ArtifactData::Text(s) => s.into_bytes(),
            ArtifactData::Bytes(b) => b,
        };
        Ok(ArtifactReference {
            artifact: format!("fixture/{}", bytes.len()),
            run_id: "fixture-run".to_string(),
            producer: ArtifactProducer {
                job_id: value.producer.job_id,
                name: value.producer.name,
            },
            bytes: bytes.len() as u64,
            preview: String::from_utf8_lossy(&bytes[..bytes.len().min(16)]).into_owned(),
            truncated: bytes.len() > 16,
        })
    }
}

fn fixture_artifacts(blob: &Value) -> ArtifactPorts {
    let data = if let Some(b64) = blob.get("base64").and_then(Value::as_str) {
        decode_base64(b64)
    } else {
        blob.as_str().unwrap_or_default().as_bytes().to_vec()
    };
    ArtifactPorts {
        reader: Some(Arc::new(FixtureReader { data })),
        publisher: Some(Arc::new(FixturePublisher)),
    }
}

// ---------------------------------------------------------------------------
// runtime construction
// ---------------------------------------------------------------------------

fn string_vec(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn symlink_policy(v: &Value) -> SymlinkPolicy {
    match v.as_str() {
        Some("within-workspace") => SymlinkPolicy::WithinWorkspace,
        _ => SymlinkPolicy::Deny,
    }
}

fn denylist_of(spec: &Value) -> Option<PathDenylist> {
    let list = string_vec(&spec["denylist"]);
    (!list.is_empty() || spec.get("denylist").is_some())
        .then(|| create_path_denylist(&list.iter().map(String::as_str).collect::<Vec<_>>()))
}

/// `<NODE>` task を fixture child re-exec へ翻訳する
/// (`command[0]=="<NODE>"` → `[diff_fixture_child bin, ...payload]` +
/// `CHEAPER_DIFF_CHILD_MODE` env 注入)。
/// bare `node` は expectRust pin で意図的 divergence として残す。
fn translate_process_spec(spec: &mut Value) {
    let Some(tasks) = spec.get_mut("tasks").and_then(Value::as_object_mut) else {
        return;
    };
    for (_name, task) in tasks.iter_mut() {
        let Some(cmd) = task.get("command").and_then(Value::as_array) else {
            continue;
        };
        if cmd.first().and_then(Value::as_str) != Some("<NODE>") {
            continue;
        }
        let script = cmd
            .get(2)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let mode = script_mode(&script);
        // bin target へ翻訳(payload args は argv[1..] — `node -e` の
        // `process.argv.slice(1)` 相当)。
        let mut command = vec![json!(CHILD_BIN)];
        command.extend(cmd.iter().skip(3).cloned());
        task["command"] = Value::Array(command);
        let env = task
            .as_object_mut()
            .unwrap()
            .entry("env".to_string())
            .or_insert_with(|| json!({}));
        env.as_object_mut()
            .unwrap()
            .insert(CHILD_ENV.to_string(), json!(mode));
    }
}

fn runtime_options(spec: &Value, ctx: &CaseCtx) -> (ToolRuntimeOptions, Arc<MutationJournal>) {
    let names: Vec<ToolName> = spec["names"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|n| n.as_str().and_then(ToolName::from_canonical))
                .collect()
        })
        .unwrap_or_else(|| vec![ToolName::Read]);
    let journal = Arc::new(MutationJournal::new());
    let mut process = spec.get("process").map(|p| unscrub(p, &ctx.needles));
    if let Some(p) = &mut process {
        translate_process_spec(p);
    }
    let artifacts = spec
        .get("artifacts")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        .then(|| fixture_artifacts(&spec["artifactBlob"]));
    let read_strictness = spec
        .get("readStrictness")
        .and_then(|v| serde_json::from_value::<ReadStrictness>(v.clone()).ok());
    (
        ToolRuntimeOptions {
            resource_manifest: spec.get("resourceManifest").cloned(),
            write_scope: spec.get("writeScope").map(string_vec),
            read_strictness,
            artifacts,
            process,
            command: None,
            root: ctx.root.clone(),
            symlink_policy: symlink_policy(&spec["symlinkPolicy"]),
            names,
            max_tool_calls: spec["maxToolCalls"].as_u64().unwrap_or(100),
            on_limit: Box::new(|| {}),
            journal: Some(Arc::clone(&journal)),
            denylist: denylist_of(spec),
            parent_lease: None,
            signal: spec
                .get("signal")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                .then(|| ctx.scope.signal()),
            abort_grace_ms: spec.get("abortGraceMs").and_then(Value::as_f64),
            concurrency: spec.get("concurrency").cloned(),
            eval_spawn: Some(fixture_eval_spawn()),
        },
        journal,
    )
}

// ---------------------------------------------------------------------------
// op runners — dump の runCall / runParallel / runEffect 相当
// ---------------------------------------------------------------------------

fn entry_ok(r: &cheaper_tools::catalog::CanonicalToolResult) -> Value {
    json!({ "ok": serde_json::to_value(r).expect("result serializes") })
}

fn entry_err(e: &ToolError) -> Value {
    json!({ "error": cap_tool(e) })
}

async fn run_call(rt: &ToolRuntime, op: &Value, needles: &[(String, String)]) -> Value {
    let ctl = AbortController::new();
    if let Some(reason) = op.get("preAborted").and_then(Value::as_str) {
        ctl.abort(AbortReason::msg(reason));
    }
    let input = unscrub(&op["input"], needles);
    let tool = op["tool"].as_str().unwrap_or_default().to_string();
    match rt
        .execute(
            &tool,
            &input,
            Some(ToolCallContext {
                signal: Some(ctl.signal()),
            }),
        )
        .await
    {
        Ok(r) => entry_ok(&r),
        Err(e) => entry_err(&e),
    }
}

type MemberFuture = Pin<Box<dyn Future<Output = Value> + Send>>;

/// TS `members.map(m => runtime.execute(...))`: 各 `execute` 呼出しは
/// 同期 prefix(admission・operation 登録)まで member 順に走る。Rust の
/// async fn は poll まで body が進まないため、noop waker で member 順に
/// 1 回ずつ poll して submission 順序を再現してから spawn する
/// (Pending 時に登録した waker は spawn 後の再 poll で実 waker に
/// 上書きされるため安全)。
async fn run_parallel(
    rt: &ToolRuntime,
    spec: &Value,
    needles: &[(String, String)],
    ctx: &CaseCtx,
) -> Value {
    let members = spec["parallel"].as_array().expect("parallel array");
    let mut ctls = Vec::with_capacity(members.len());
    let mut futs: Vec<MemberFuture> = Vec::with_capacity(members.len());
    for m in members {
        let ctl = AbortController::new();
        if let Some(reason) = m.get("preAborted").and_then(Value::as_str) {
            ctl.abort(AbortReason::msg(reason));
        }
        let signal = ctl.signal();
        ctls.push(ctl);
        let rt2 = rt.clone();
        let tool = m["tool"].as_str().unwrap_or_default().to_string();
        let input = unscrub(&m["input"], needles);
        futs.push(Box::pin(async move {
            match rt2
                .execute(
                    &tool,
                    &input,
                    Some(ToolCallContext {
                        signal: Some(signal),
                    }),
                )
                .await
            {
                Ok(r) => entry_ok(&r),
                Err(e) => entry_err(&e),
            }
        }) as MemberFuture);
    }
    // submission: member 順に最初の poll(TS の sync prefix 相当 — admission /
    // operation 登録の順序を固定する)。この poll で登録された Pending waker は
    // noop のため、以後の進行は spawn 後の実 waker 登録に任せる。
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut ready: Vec<Option<Value>> = Vec::with_capacity(futs.len());
    for f in futs.iter_mut() {
        ready.push(match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => Some(v),
            Poll::Pending => None,
        });
    }
    // member-level abort は全 submission の後。
    for (i, m) in members.iter().enumerate() {
        if let Some(reason) = m.get("abortReason").and_then(Value::as_str) {
            ctls[i].abort(AbortReason::msg(reason));
        }
    }
    // TS の promise は event loop で独立に進行する。drain 等の `then` effect が
    // in-flight op の settle を待つため、member future は effects 実行前に
    // spawn して実 waker を登録させる(noop waker のままでは wake が届かず
    // drain がデッドロックする)。
    let handles: Vec<Option<tokio::task::JoinHandle<Value>>> = futs
        .into_iter()
        .enumerate()
        .map(|(i, f)| (ready[i].is_none()).then(|| tokio::spawn(f)))
        .collect();
    // `then` effects は pending 解決前に逐次実行。
    let mut then = Vec::new();
    for step in spec
        .get("then")
        .and_then(Value::as_array)
        .map(|a| a.as_slice())
        .unwrap_or(&[])
    {
        then.push(run_effect(rt, step, ctx).await);
    }
    let mut calls = Vec::with_capacity(handles.len());
    for (i, h) in handles.into_iter().enumerate() {
        match h {
            Some(h) => calls.push(h.await.expect("member task")),
            None => calls.push(ready[i].take().unwrap()),
        }
    }
    json!({ "ok": { "calls": calls, "then": then } })
}

async fn run_effect(rt: &ToolRuntime, op: &Value, ctx: &CaseCtx) -> Value {
    match op["op"].as_str().unwrap_or_default() {
        "ready" => match rt.ready().await {
            Ok(_) => json!({ "ok": null }),
            Err(reason) => json!({ "error": cap(&reason) }),
        },
        "abort-runtime" => {
            rt.abort(
                op.get("reason")
                    .and_then(Value::as_str)
                    .map(AbortReason::msg),
            );
            json!({ "ok": null })
        }
        "abort-scope" => {
            ctx.scope
                .abort(AbortReason::msg(op["reason"].as_str().unwrap_or_default()));
            json!({ "ok": null })
        }
        "drain" => match rt.drain().await {
            Ok(()) => json!({ "ok": null }),
            Err(e) => json!({ "error": cap(&e) }),
        },
        "touch" => {
            let p = ctx.dir.join(rel_path(op["path"].as_str().unwrap()));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, op["content"].as_str().unwrap_or("go")).unwrap();
            json!({ "ok": null })
        }
        other => panic!("unknown op {other}"),
    }
}

// ---------------------------------------------------------------------------
// post-state projection — dump の projectOperations/projectAccesses/
// projectState 相当
// ---------------------------------------------------------------------------

fn project_operations(rt: &ToolRuntime) -> Value {
    Value::Array(
        rt.operations()
            .iter()
            .map(|op| {
                let mut m = Map::new();
                m.insert(
                    "toolName".to_string(),
                    op.tool_name.clone().map(Value::from).unwrap_or(Value::Null),
                );
                m.insert("state".to_string(), json!(op.state.as_str()));
                m.insert(
                    "settlement".to_string(),
                    serde_json::to_value(op.settlement).unwrap(),
                );
                if let Some(claims) = &op.resource_claims {
                    m.insert(
                        "resourceClaims".to_string(),
                        serde_json::to_value(claims).unwrap(),
                    );
                }
                if let Some(cancel) = &op.cancellation {
                    m.insert(
                        "cancellation".to_string(),
                        json!({
                            "state": cancel.state.as_str(),
                            "reason": serde_json::to_value(&cancel.reason).unwrap(),
                        }),
                    );
                }
                if let Some(error) = &op.error {
                    m.insert("error".to_string(), serde_json::to_value(error).unwrap());
                }
                Value::Object(m)
            })
            .collect(),
    )
}

fn project_accesses(rt: &ToolRuntime) -> Value {
    let snap = rt.accesses();
    json!({
        "total": snap.total,
        "dropped": snap.dropped,
        "observations": snap.observations.iter().map(|o| {
            let mut m = Map::new();
            m.insert("path".to_string(), json!(o.path));
            m.insert("claim".to_string(), serde_json::to_value(&o.claim).unwrap());
            m.insert("declared".to_string(), json!(o.declared));
            m.insert("allowed".to_string(), json!(o.allowed));
            m.insert("support".to_string(), json!(o.support));
            if o.operation_id.is_some() {
                m.insert("operationId".to_string(), json!("<OP>"));
            }
            Value::Object(m)
        }).collect::<Vec<_>>(),
    })
}

fn project_state(rt: &ToolRuntime, spec: &Value) -> Value {
    let budget = rt.budget();
    let mut state = json!({
        "budget": {
            "toolCalls": budget.tool_calls(),
            "rejectedToolCalls": budget.rejected_tool_calls(),
            "limitHit": budget.limit_hit().map(|h| h.as_str()),
        },
        "meter": { "sourceTokens": rt.meter().source_tokens() },
        "journal": serde_json::to_value(rt.journal().finalize()).unwrap(),
        "operations": project_operations(rt),
    });
    if spec
        .get("accesses")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        state["accesses"] = project_accesses(rt);
    }
    state
}

// ---------------------------------------------------------------------------
// tool-level runner — dump の runToolLevel 相当
// ---------------------------------------------------------------------------

fn run_tool_level(spec: &Value, ctx: &CaseCtx) -> Vec<Value> {
    let name = spec["tool"]["name"].as_str().expect("tool.name");
    let opts = &spec["tool"]["opts"];
    let denylist = denylist_of(spec);
    let policy = symlink_policy(&spec["symlinkPolicy"]);
    let execute_ops = |run: &dyn Fn(
        &Value,
    ) -> Result<
        cheaper_tools::catalog::CanonicalToolResult,
        ToolError,
    >|
     -> Vec<Value> {
        spec["ops"]
            .as_array()
            .expect("ops array")
            .iter()
            .map(|op| {
                let input = unscrub(&op["input"], &ctx.needles);
                match run(&input) {
                    Ok(r) => entry_ok(&r),
                    Err(e) => entry_err(&e),
                }
            })
            .collect()
    };
    match name {
        "grep" => {
            let tool = create_grep_tool(GrepToolOptions {
                root: &ctx.root,
                symlink_policy: policy,
                denylist: denylist.as_ref(),
                observe_access: None,
                eval_per_file_ms: opts["evalPerFileMs"].as_u64(),
                eval_total_ms: opts["evalTotalMs"].as_u64(),
                scan_deadline_ms: opts["scanDeadlineMs"].as_u64(),
                chunk_bytes: opts["chunkBytes"].as_u64().map(|n| n as usize),
                max_line_bytes: opts["maxLineBytes"].as_u64().map(|n| n as usize),
                batch_max_lines: opts["batchMaxLines"].as_u64().map(|n| n as usize),
                batch_max_bytes: opts["batchMaxBytes"].as_u64().map(|n| n as usize),
                output_max_bytes: opts["outputMaxBytes"].as_u64().map(|n| n as usize),
                output_line_max_chars: opts["outputLineMaxChars"].as_u64().map(|n| n as usize),
                skip_footer_max_entries: opts["skipFooterMaxEntries"].as_u64().map(|n| n as usize),
                skip_path_max_chars: opts["skipPathMaxChars"].as_u64().map(|n| n as usize),
                skip_footer_max_bytes: opts["skipFooterMaxBytes"].as_u64().map(|n| n as usize),
                eval_spawn: Some(fixture_eval_spawn()),
                eval_reply_max_bytes: None,
                now: None,
            });
            let tool = match tool {
                Ok(t) => t,
                Err(e) => {
                    return spec["ops"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|_| entry_err(&e))
                        .collect()
                }
            };
            execute_ops(&|input| tool.execute(input, None))
        }
        "read" => {
            let artifacts = spec
                .get("artifacts")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                .then(|| fixture_artifacts(&spec["artifactBlob"]));
            let tool = create_read_tool(ReadToolOptions {
                artifacts: artifacts.as_ref(),
                owner_id: None,
                observe_access: None,
                root: &ctx.root,
                symlink_policy: policy,
                denylist: denylist.as_ref(),
                max_bytes: opts["maxBytes"].as_u64().map(|n| n as usize),
                chunk_bytes: opts["chunkBytes"].as_u64().map(|n| n as usize),
            });
            execute_ops(&|input| tool.execute(input, None))
        }
        other => panic!("unknown tool-level tool {other}"),
    }
}

// ---------------------------------------------------------------------------
// expectRust — documented divergence pins
// ---------------------------------------------------------------------------

/// `expectRust.ops` の sparse override を `expected` へ適用する
/// (key は op index の文字列)。
fn apply_rust_ops(expected: &Value, expect: &Value) -> Value {
    let Some(ops) = expect.get("ops").and_then(Value::as_object) else {
        return expected.clone();
    };
    let mut out = expected.clone();
    for (k, v) in ops {
        let i: usize = k.parse().expect("expectRust op index");
        out.as_array_mut().unwrap()[i] = v.clone();
    }
    out
}

/// `expectRust.state` の deep-merge patch(object は再帰 merge、それ以外は
/// 置換)。
fn merge_patch(base: &Value, patch: &Value) -> Value {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            let mut o = b.clone();
            for (k, v) in p {
                let merged = match o.get(k) {
                    Some(existing) => merge_patch(existing, v),
                    None => v.clone(),
                };
                o.insert(k.clone(), merged);
            }
            Value::Object(o)
        }
        // patch 未指定(`expectRust` なし)= base をそのまま使う。
        (b, Value::Null) => b.clone(),
        // 同長 array は要素ごとに merge(state.operations 等の sparse
        // pin を可能にする)。
        (Value::Array(b), Value::Array(p)) if b.len() == p.len() => Value::Array(
            b.iter()
                .zip(p.iter())
                .map(|(x, y)| merge_patch(x, y))
                .collect(),
        ),
        (_, p) => p.clone(),
    }
}

// ---------------------------------------------------------------------------
// case driver
// ---------------------------------------------------------------------------

struct CaseOutcome {
    name: String,
    diffs: Vec<String>,
    skipped: bool,
}

async fn run_case(file: &str, index: usize, spec: &Value, base: &TempBase) -> CaseOutcome {
    let name = spec["name"].as_str().unwrap_or("<unnamed>").to_string();
    // `axis` gating: `"win32"`/`"posix"`/`"any"`。
    let axis_ok = match spec.get("axis").and_then(Value::as_str).unwrap_or("any") {
        "win32" => cfg!(windows),
        "posix" => !cfg!(windows),
        _ => true,
    };
    if !axis_ok {
        return CaseOutcome {
            name,
            diffs: vec![],
            skipped: true,
        };
    }
    let Some(ctx) = case_dir(file, index, spec, base) else {
        return CaseOutcome {
            name,
            diffs: vec![],
            skipped: true,
        };
    };
    let mut produced: Vec<Value>;
    let mut state: Option<Value> = None;
    if spec.get("tool").is_some() {
        produced = run_tool_level(spec, &ctx);
    } else {
        let runtime_spec = &spec["runtime"];
        let (options, _journal) = runtime_options(runtime_spec, &ctx);
        match create_tool_runtime(options, None) {
            Err(e) => {
                produced = vec![entry_err(&e)];
            }
            Ok(rt) => {
                produced = Vec::new();
                for op in spec["ops"].as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
                    if op.get("parallel").is_some() {
                        produced.push(run_parallel(&rt, op, &ctx.needles, &ctx).await);
                    } else if op.get("op").is_some() {
                        produced.push(run_effect(&rt, op, &ctx).await);
                    } else {
                        produced.push(run_call(&rt, op, &ctx.needles).await);
                    }
                }
                state = Some(scrub(&project_state(&rt, runtime_spec), &ctx.needles));
                rt.dispose().await;
            }
        }
    }

    // `expectRust` override — 文書化済み divergence の pin。
    let expect_rust = &spec["expectRust"];
    // fixture 側 `expected` は `<ROOT>` token 化済み — produced のみ
    // scrub して揃える(pin も token 表記で書く)。比較は両側に
    // compare_norm(separator/errno canonicalization)を掛ける。
    let produced = match compare_norm(&scrub(&Value::Array(produced), &ctx.needles)) {
        Value::Array(a) => a,
        _ => unreachable!(),
    };
    let mut diffs = Vec::new();
    let expected = compare_norm(&apply_rust_ops(&spec["expected"], expect_rust));
    if produced != *expected.as_array().unwrap_or(&vec![]) {
        diffs.push(format!(
            "expected mismatch:\n  want {}\n  got  {}",
            serde_json::to_string_pretty(&expected).unwrap(),
            serde_json::to_string_pretty(&produced).unwrap(),
        ));
    }
    if let Some(want_state) = spec.get("state").filter(|s| !s.is_null()) {
        let patched = compare_norm(&merge_patch(want_state, &expect_rust["state"]));
        match &state {
            Some(got) if compare_norm(got) == patched => {}
            Some(got) => diffs.push(format!(
                "state mismatch:\n  want {}\n  got  {}",
                serde_json::to_string_pretty(&patched).unwrap(),
                serde_json::to_string_pretty(got).unwrap(),
            )),
            None => diffs.push("state missing (runtime construction failed)".to_string()),
        }
    }
    CaseOutcome {
        name,
        diffs,
        skipped: false,
    }
}

async fn run_file(file: &str) {
    let fixture = fixture_file(file);
    let base = TempBase::new();
    let cases = fixture["cases"].as_array().expect("cases array");
    let mut failures = Vec::new();
    let mut skipped = 0usize;
    for (i, spec) in cases.iter().enumerate() {
        let outcome = run_case(file, i, spec, &base).await;
        if outcome.skipped {
            skipped += 1;
            continue;
        }
        for d in outcome.diffs {
            failures.push(format!("{file} :: {} :: {d}", outcome.name));
        }
    }
    assert!(
        failures.is_empty(),
        "{file}: {} case(s) diverged ({} skipped):\n{}",
        failures.len(),
        skipped,
        failures.join("\n\n"),
    );
}

// ---------------------------------------------------------------------------
// per-file test entries
// ---------------------------------------------------------------------------

macro_rules! fixture_test {
    ($name:ident, $file:literal) => {
        #[tokio::test]
        async fn $name() {
            run_file($file).await;
        }
    };
}

fixture_test!(read_fixtures, "read.json");
fixture_test!(grep_fixtures, "grep.json");
fixture_test!(find_fixtures, "find.json");
fixture_test!(ls_fixtures, "ls.json");
fixture_test!(write_fixtures, "write.json");
fixture_test!(edit_fixtures, "edit.json");
fixture_test!(count_tokens_fixtures, "count_tokens.json");
fixture_test!(process_fixtures, "process.json");
fixture_test!(runtime_fixtures, "runtime.json");

// ---------------------------------------------------------------------------
// glob.json(`path.posix.matchesGlob` の直接 fixture)
// ---------------------------------------------------------------------------

/// `{p, g, expected}` の `expected` が null の case は Node が
/// SyntaxError を投げる pattern。Rust 側は `matches_glob` が
/// `Result<bool>` を返し `Err` は `TypeError("pattern is too long")`
/// 相当のみなので、null case は「評価しても panic しない」ことのみ
/// 確認する(設計固定の divergence)。`expected: "error"` の case は
/// Rust が `Err` を返すことを確認する(Node の throw 相当)。
#[test]
fn glob_fixtures() {
    let fixture = fixture_file("glob.json");
    let cases = fixture["cases"].as_array().expect("cases array");
    let mut failures = Vec::new();
    let mut skipped = 0usize;
    for spec in cases {
        let p = spec["p"].as_str().expect("p");
        let g = spec["g"].as_str().expect("g");
        let got = cheaper_tools::glob::matches_glob(p, g);
        match spec["expected"].as_bool() {
            Some(want) if want == got.as_ref().ok().copied().unwrap_or(false) => {}
            Some(want) => failures.push(format!("p={p:?} g={g:?} want={want} got={got:?}")),
            None if spec["expected"].as_str() == Some("error") => {
                if got.is_ok() {
                    failures.push(format!("p={p:?} g={g:?} want=error got={got:?}"));
                }
            }
            None => skipped += 1,
        }
    }
    assert!(
        failures.is_empty(),
        "glob: {} case(s) diverged ({} node-throw skipped):\n{}",
        failures.len(),
        skipped,
        failures.join("\n"),
    );
}
