//! `cheaper hook-config sync|verify|remove` — Devin hook 登録の
//! CLI 所有 lifecycle(ADR-034H2WBNtC4We8dyC5lQZJ、issue #107)。
//!
//! 旧 `install-devin-hooks.ps1` が持っていた責務(冪等 upsert・
//! legacy 検出・atomic write・外部変更検出)をここへ集約し、
//! PowerShell script は thin wrapper に縮退する。
//!
//! 所有境界:
//! - cheaper が書き込む own entry は command が
//!   [`OWN_COMMAND`] と完全一致する group のみ。
//! - 他 tool の entry・user が書いた entry・他 key は一切触れない。
//! - `.devin/hooks.v1.json` は generate/rewrite しない(precedence
//!   shadow のみ verify が報告する)。

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use cheaper_core::errors::CheaperError;
use cheaper_core::json::stringify::stringify;

use crate::args::{HookConfigAction, HookConfigCliArgs};
use crate::cli::{CliHooks, CliIo, CommandOut};
use crate::envelope::utility_success_envelope;

/// 登録する hook command。own entry の identity。
const OWN_COMMAND: &str = "cheaper hook pre-tool-use --target devin";

/// 新規 matcher — `read`/`grep`/`exec` の case-insensitive 完全一致。
const CANONICAL_MATCHER: &str = "^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Ee][Xx][Ee][Cc])$";

/// 収束対象として認める既知 matcher(旧 `read|grep` 形を canonical へ)。
/// 未知値の own matcher は「外部変更済み」とみなし書き換えない。
const KNOWN_MATCHERS: &[&str] = &["^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$", CANONICAL_MATCHER];

/// legacy cheaper read-hook 登録を検出する pattern
/// (旧 `install-devin-hooks.ps1` の `$conflictRegex` と同一)。
const CONFLICT_PATTERN: &str = r"cheaper[^\r\n]*hook[^\r\n]*(pre-read-code|pre-tool-use)";

/// project scope の legacy 走査対象(既知 config file 群)。
/// `.devin/config.json` 自体は target file として別途扱う。
const KNOWN_LEGACY_FILES: &[&str] = &[
    ".windsurf/hooks.json",
    ".devin/hooks.json",
    ".devin/hooks.v1.json",
    ".devin/config.local.json",
    ".claude/settings.json",
    ".claude/settings.local.json",
];

/// own entry の canonical JSON shape。
fn canonical_entry() -> Value {
    let mut entry = Map::new();
    entry.insert("matcher".to_string(), Value::from(CANONICAL_MATCHER));
    let mut hook = Map::new();
    hook.insert("type".to_string(), Value::from("command"));
    hook.insert("command".to_string(), Value::from(OWN_COMMAND));
    entry.insert("hooks".to_string(), Value::Array(vec![Value::Object(hook)]));
    Value::Object(entry)
}

fn invalid_request(message: impl Into<String>) -> CheaperError {
    crate::errors::invalid_request(message.into(), None)
}

// ---------------------------------------------------------------------------
// scope / path 解決
// ---------------------------------------------------------------------------

/// 対象 config path を scope から解決する。
/// - `project`(既定): `<root>/.devin/config.json`
/// - `user`: POSIX `~/.config/devin/config.json` /
///   Windows `%APPDATA%\devin\config.json`
fn resolve_config_path(args: &HookConfigCliArgs, io: &CliIo<'_>) -> Result<PathBuf, CheaperError> {
    match args.scope.as_deref().unwrap_or("project") {
        "project" => {
            let root = args.project_root.clone().unwrap_or_else(|| io.cwd.clone());
            let root_path = PathBuf::from(&root);
            if !root_path.is_dir() {
                return Err(invalid_request(format!("project root not found: {root}")));
            }
            Ok(root_path.join(".devin").join("config.json"))
        }
        "user" => {
            let base = user_devin_dir(io)?;
            Ok(base.join("config.json"))
        }
        other => Err(invalid_request(format!(
            "unsupported --scope: {other} (expected project|user)"
        ))),
    }
}

/// Devin の user config dir(devin.ai docs の一次情報):
/// POSIX `$HOME/.config/devin`、Windows `%APPDATA%\devin`。
/// `~/.devin` は Devin CLI の実 path ではないため使わない。
fn user_devin_dir(io: &CliIo<'_>) -> Result<PathBuf, CheaperError> {
    if cfg!(windows) {
        if let Some(appdata) = (io.env)("APPDATA") {
            if !appdata.is_empty() {
                return Ok(PathBuf::from(appdata).join("devin"));
            }
        }
        return Err(invalid_request(
            "--scope user requires %APPDATA% to resolve the Devin user config",
        ));
    }
    match (io.env)("HOME") {
        Some(home) if !home.is_empty() => Ok(PathBuf::from(home).join(".config").join("devin")),
        _ => Err(invalid_request(
            "--scope user requires $HOME to resolve the Devin user config",
        )),
    }
}

/// config path の各 component が symlink/reparse point でないことを
/// 確認する(存在しない末尾 component は許容)。
fn check_no_reparse(path: &Path) -> Result<(), CheaperError> {
    let mut cur = PathBuf::new();
    for comp in path.components() {
        cur.push(comp.as_os_str());
        match std::fs::symlink_metadata(&cur) {
            Ok(md) if md.file_type().is_symlink() => {
                return Err(invalid_request(format!(
                    "refusing to write through a symlinked path component: {}",
                    cur.display()
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// config 読み込み/検証
// ---------------------------------------------------------------------------

/// object が「意味上は同じだが大小文字だけ違う key」を持つか走査する
/// (PS script の `Get-ConfigKey` fail 条件)。見つけたら key 名を返す。
fn find_case_conflict(map: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    for want in keys {
        for k in map.keys() {
            if k != want && k.eq_ignore_ascii_case(want) {
                return Some(k.clone());
            }
        }
    }
    None
}

/// `PreToolUse` group の schema 検証(PS script と同じ条件):
/// - object であること
/// - `matcher` は存在すれば string
/// - `hooks` は存在すれば array、各 entry は `type: "command"` +
///   string `command`、または `type: "prompt"` + string `prompt`
fn validate_hook_group(group: &Value, file: &str) -> Result<(), CheaperError> {
    let Some(obj) = group.as_object() else {
        return Err(invalid_request(format!(
            "{file}: hooks.PreToolUse entries must be objects"
        )));
    };
    if let Some(m) = obj.get("matcher") {
        if !m.is_string() {
            return Err(invalid_request(format!(
                "{file}: hooks.PreToolUse[].matcher must be a string"
            )));
        }
    }
    if let Some(hooks) = obj.get("hooks") {
        let Some(arr) = hooks.as_array() else {
            return Err(invalid_request(format!(
                "{file}: hooks.PreToolUse[].hooks must be an array"
            )));
        };
        for h in arr {
            let Some(ho) = h.as_object() else {
                return Err(invalid_request(format!(
                    "{file}: hooks.PreToolUse[].hooks[] entries must be objects"
                )));
            };
            match ho.get("type").and_then(Value::as_str) {
                Some("command") => {
                    if ho.get("command").and_then(Value::as_str).is_none() {
                        return Err(invalid_request(format!(
                            "{file}: command hook requires a string \"command\""
                        )));
                    }
                }
                Some("prompt") => {
                    if ho.get("prompt").and_then(Value::as_str).is_none() {
                        return Err(invalid_request(format!(
                            "{file}: prompt hook requires a string \"prompt\""
                        )));
                    }
                }
                _ => {
                    return Err(invalid_request(format!(
                        "{file}: hook entries require \"type\": \"command\"|\"prompt\""
                    )));
                }
            }
        }
    }
    Ok(())
}

/// config file の load + 構造検証結果。
struct LoadedConfig {
    /// 読み取った raw bytes(外部変更検出の比較用)。
    raw: Vec<u8>,
    /// parse 済み object(root は object 保証済み)。
    root: Map<String, Value>,
    /// `hooks.PreToolUse` 内の own entry index(原則 0 か 1)。
    own_indexes: Vec<usize>,
}

/// strict JSON で読み、構造検証と own index 収集まで行う。
/// file 不在・非 regular file は `Ok(None)`。
fn load_config(path: &Path) -> Result<Option<LoadedConfig>, CheaperError> {
    let display = path.display().to_string();
    let md = match std::fs::symlink_metadata(path) {
        Ok(md) => md,
        Err(_) => return Ok(None),
    };
    if md.file_type().is_symlink() || !md.is_file() {
        return Err(invalid_request(format!(
            "{display}: not a regular file (refusing to read through a symlink/reparse point)"
        )));
    }
    let raw =
        std::fs::read(path).map_err(|e| invalid_request(format!("{display}: cannot read: {e}")))?;
    let value: Value = serde_json::from_slice(&raw).map_err(|e| {
        invalid_request(format!(
            "{display}: not strict JSON (JSONC/comments/trailing commas are unsupported): {e}"
        ))
    })?;
    let Value::Object(root) = value else {
        return Err(invalid_request(format!(
            "{display}: config root must be an object"
        )));
    };
    if let Some(k) = find_case_conflict(&root, &["hooks"]) {
        return Err(invalid_request(format!(
            "{display}: case-conflicting key \"{k}\" (expected \"hooks\")"
        )));
    }
    let mut own_indexes = Vec::new();
    if let Some(hooks) = root.get("hooks") {
        let Some(hooks_obj) = hooks.as_object() else {
            return Err(invalid_request(format!(
                "{display}: \"hooks\" must be an object"
            )));
        };
        if let Some(k) = find_case_conflict(hooks_obj, &["PreToolUse"]) {
            return Err(invalid_request(format!(
                "{display}: case-conflicting key \"{k}\" (expected \"PreToolUse\")"
            )));
        }
        if let Some(pre) = hooks_obj.get("PreToolUse") {
            let Some(arr) = pre.as_array() else {
                return Err(invalid_request(format!(
                    "{display}: hooks.PreToolUse must be an array"
                )));
            };
            for (i, group) in arr.iter().enumerate() {
                validate_hook_group(group, &display)?;
                if is_own_group(group) {
                    own_indexes.push(i);
                }
            }
        }
    }
    Ok(Some(LoadedConfig {
        raw,
        root,
        own_indexes,
    }))
}

/// group が own entry か — `hooks[].command === OWN_COMMAND` を
/// 1件でも持つ group(PS script と同じ identity 規則)。
fn is_own_group(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|arr| {
            arr.iter()
                .any(|h| h.get("command").and_then(Value::as_str) == Some(OWN_COMMAND))
        })
}

/// own group の厳密 shape 検証。戻り値は `(matcher, drift)` —
/// `drift` は matcher が既知値だが canonical でない場合 true。
/// shape が壊れている・matcher が未知なら `Err`(modified とみなす)。
fn own_group_matcher(group: &Value) -> Result<(&str, bool), CheaperError> {
    let obj = group.as_object().expect("validated group");
    if obj.len() != 2 || !obj.contains_key("matcher") || !obj.contains_key("hooks") {
        return Err(invalid_request(
            "cheaper hook entry has unexpected extra keys (treating as externally modified)",
        ));
    }
    let matcher = obj
        .get("matcher")
        .and_then(Value::as_str)
        .expect("validated matcher");
    if !KNOWN_MATCHERS.contains(&matcher) {
        return Err(invalid_request(format!(
            "cheaper hook entry matcher is unrecognized: {matcher} (treating as externally modified)"
        )));
    }
    let hooks = obj
        .get("hooks")
        .and_then(Value::as_array)
        .expect("validated hooks");
    let canonical_hook = {
        let mut h = Map::new();
        h.insert("type".to_string(), Value::from("command"));
        h.insert("command".to_string(), Value::from(OWN_COMMAND));
        Value::Object(h)
    };
    if hooks.len() != 1 || hooks[0] != canonical_hook {
        return Err(invalid_request(
            "cheaper hook entry has modified hooks (treating as externally modified)",
        ));
    }
    Ok((matcher, matcher != CANONICAL_MATCHER))
}

// ---------------------------------------------------------------------------
// legacy conflict scan
// ---------------------------------------------------------------------------

/// JSON value 内の command-ish key の string 値を再帰収集する
/// (PS script の `command`/`powershell`/`bash`/`windows`/`linux`/`osx`
/// key 走査と同じ条件)。
fn collect_command_strings(v: &Value, out: &mut Vec<String>) {
    const COMMAND_KEYS: &[&str] = &["command", "powershell", "bash", "windows", "linux", "osx"];
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                if COMMAND_KEYS.contains(&k.as_str()) {
                    if let Some(s) = val.as_str() {
                        out.push(s.to_string());
                    }
                }
                collect_command_strings(val, out);
            }
        }
        Value::Array(arr) => {
            for val in arr {
                collect_command_strings(val, out);
            }
        }
        _ => {}
    }
}

/// 既知 file 群 + target file 自体から legacy cheaper hook 登録を
/// 検出する。target file 内の own group は除外する(自前の entry は
/// conflict ではない)。戻り値は conflict を含む file 一覧。
fn legacy_conflicts(root: &Path, target: &Path) -> Vec<String> {
    let re = regex::Regex::new(CONFLICT_PATTERN).expect("static regex");
    let mut hits: Vec<String> = Vec::new();
    let mut files: Vec<PathBuf> = KNOWN_LEGACY_FILES
        .iter()
        .map(|rel| root.join(rel))
        .collect();
    if !files.iter().any(|f| f == target) {
        files.push(target.to_path_buf());
    }
    for file in files {
        let Ok(bytes) = std::fs::read(&file) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
            // parse 不能な sibling file は best-effort 規約で黙って skip —
            // verify は sibling の parse 失敗を finding として報告しない。
            continue;
        };
        let mut strings = Vec::new();
        collect_command_strings(&value, &mut strings);
        // target file 内では own command そのもの(`== OWN_COMMAND`)
        // は conflict ではない。他 file では cheaper の hook 登録が
        // 残っていれば全て conflict。
        let file_hit = if file == target {
            strings.iter().any(|s| re.is_match(s) && s != OWN_COMMAND)
        } else {
            strings.iter().any(|s| re.is_match(s))
        };
        if file_hit {
            hits.push(file.display().to_string());
        }
    }
    hits
}

// ---------------------------------------------------------------------------
// capability(PATH 上の cheaper) + atomic write
// ---------------------------------------------------------------------------

/// PATH 上の `cheaper` が `hook targets --json` に応答し、devin
/// target と `hookConfig` capability を持つことを確認する。
/// hook が呼ぶのは PATH 上の binary であり、実行中の self が対応
/// していても PATH binary が旧版なら登録先で失敗するため、
/// self-check ではなく spawn で確認する。
fn check_capability(io: &CliIo<'_>) -> Result<(), CheaperError> {
    let exe = find_on_path(io, "cheaper").ok_or_else(|| {
        invalid_request(
            "cheaper was not found on PATH — install cheaper on PATH before registering hooks",
        )
    })?;
    let out = std::process::Command::new(&exe)
        .args(["hook", "targets", "--json"])
        .output()
        .map_err(|e| {
            invalid_request(format!(
                "failed to run `{} hook targets --json`: {e}",
                exe.display()
            ))
        })?;
    if !out.status.success() {
        return Err(invalid_request(format!(
            "`{} hook targets --json` exited with {}",
            exe.display(),
            out.status
        )));
    }
    let parsed: Value = serde_json::from_slice(&out.stdout).map_err(|e| {
        invalid_request(format!(
            "cannot parse `cheaper hook targets --json` output: {e}"
        ))
    })?;
    let has_devin = parsed
        .get("targets")
        .and_then(Value::as_array)
        .is_some_and(|t| t.iter().any(|v| v.as_str() == Some("devin")));
    if !has_devin {
        return Err(invalid_request(
            "the cheaper binary on PATH does not report the `devin` hook target",
        ));
    }
    if parsed.get("hookConfig").and_then(Value::as_bool) != Some(true) {
        return Err(invalid_request(
            "the cheaper binary on PATH does not support `hook-config` (hookConfig missing)",
        ));
    }
    Ok(())
}

/// PATH lookup — `io.env("PATH")` 由来の dir から実行可能 file を
/// 探す。Windows は `cheaper.exe`/`cheaper.cmd`/… を PATHEXT 順で
/// 探し、それ以外は `cheaper` そのまま。
fn find_on_path(io: &CliIo<'_>, name: &str) -> Option<PathBuf> {
    let path = (io.env)("PATH")?;
    let dirs: Vec<PathBuf> = std::env::split_paths(&path).collect();
    #[cfg(windows)]
    let candidates: Vec<String> = {
        let pathext = (io.env)("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());
        let mut v = vec![name.to_string()];
        for ext in pathext.split(';').filter(|e| !e.is_empty()) {
            v.push(format!("{name}{ext}"));
            v.push(format!("{name}{}", ext.to_ascii_lowercase()));
        }
        v
    };
    #[cfg(not(windows))]
    let candidates: Vec<String> = vec![name.to_string()];
    for dir in dirs {
        for cand in &candidates {
            let p = dir.join(cand);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// `{name}.` prefix + `.tmp` suffix(`config.json.*.tmp` 相当)の
/// orphan を削除する。戻り値は削除した file 名 — sync result の
/// `orphansRemoved` field 用(verify が使う走査は `find_orphans`)。
fn sweep_orphans(config_path: &Path) -> Vec<String> {
    let Some(dir) = config_path.parent() else {
        return Vec::new();
    };
    let Some(name) = config_path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let tmp_prefix = format!("{name}.");
    let mut removed = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    for entry in rd.flatten() {
        let fname = entry.file_name().to_string_lossy().into_owned();
        // 削除対象は自身の atomic write が残しうる `{name}.` prefix +
        // `.tmp` suffix のみ — `.bak` は cheaper が生成しない(所有境界外)。
        let is_orphan = fname.starts_with(&tmp_prefix) && fname.ends_with(".tmp");
        if is_orphan && std::fs::remove_file(entry.path()).is_ok() {
            removed.push(fname);
        }
    }
    removed
}

/// orphan temp/bak を走査する(verify 用 — 削除はしない)。
fn find_orphans(config_path: &Path) -> Vec<String> {
    let Some(dir) = config_path.parent() else {
        return Vec::new();
    };
    let Some(name) = config_path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let tmp_prefix = format!("{name}.");
    let mut found = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    for entry in rd.flatten() {
        let fname = entry.file_name().to_string_lossy().into_owned();
        if fname.starts_with(&tmp_prefix) && (fname.ends_with(".tmp") || fname.ends_with(".bak")) {
            found.push(fname);
        }
    }
    found
}

/// temp + rename の atomic write。書き込み直前に file を再読して
/// 読み取り時点の raw bytes と比較し、外部変更があれば error。
fn write_config_atomic(
    config_path: &Path,
    expected_raw: Option<&[u8]>,
    root: &Map<String, Value>,
) -> Result<(), CheaperError> {
    check_no_reparse(config_path)?;
    let dir = config_path
        .parent()
        .ok_or_else(|| invalid_request("config path has no parent"))?;
    std::fs::create_dir_all(dir)
        .map_err(|e| invalid_request(format!("cannot create {}: {e}", dir.display())))?;
    // 外部変更検出 — 読み取り時点の bytes と現在を比較。
    let current = std::fs::read(config_path).ok();
    match (expected_raw, current) {
        (Some(expected), Some(now)) if expected != now.as_slice() => {
            return Err(invalid_request(format!(
                "{}: file changed between read and write (refusing to overwrite external edits)",
                config_path.display()
            )));
        }
        (Some(_), None) => {
            return Err(invalid_request(format!(
                "{}: file was removed between read and write",
                config_path.display()
            )));
        }
        (None, Some(_)) => {
            return Err(invalid_request(format!(
                "{}: file appeared between read and write",
                config_path.display()
            )));
        }
        _ => {}
    }
    let mut out = stringify(&Value::Object(root.clone()));
    out.push('\n');
    // temp name は sweep_orphans の `{name}.` prefix + `.tmp` suffix
    // 規約に合わせる。
    let tmp = dir.join(format!(
        "{}.{}.tmp",
        config_path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, out.as_bytes())
        .map_err(|e| invalid_request(format!("cannot write {}: {e}", tmp.display())))?;
    if let Err(e) = std::fs::rename(&tmp, config_path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(invalid_request(format!(
            "cannot replace {}: {e}",
            config_path.display()
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// action 実装
// ---------------------------------------------------------------------------

/// `verify` — 読み取りのみ。findings + ok flag を envelope result へ。
fn run_verify(
    args: &HookConfigCliArgs,
    io: &CliIo<'_>,
    quiet: bool,
) -> Result<(Map<String, Value>, i32), CheaperError> {
    let config_path = resolve_config_path(args, io)?;
    check_no_reparse(&config_path)?;
    let root_dir = args
        .project_root
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(io.cwd.clone()));
    let mut findings: Vec<Value> = Vec::new();
    let mut blocker = false;
    let mut result = Map::new();
    result.insert(
        "configPath".to_string(),
        Value::from(config_path.display().to_string()),
    );
    result.insert(
        "scope".to_string(),
        Value::from(args.scope.as_deref().unwrap_or("project")),
    );

    let loaded = match load_config(&config_path) {
        Ok(l) => l,
        Err(e) => {
            blocker = true;
            findings.push(Value::from(format!("config-unreadable: {e}")));
            None
        }
    };

    // own entry 状態。
    let mut own_state = "absent";
    if let Some(l) = &loaded {
        match l.own_indexes.len() {
            0 => {
                own_state = "missing";
            }
            1 => {
                let arr = l
                    .root
                    .get("hooks")
                    .and_then(|h| h.get("PreToolUse"))
                    .and_then(Value::as_array)
                    .expect("validated");
                match own_group_matcher(&arr[l.own_indexes[0]]) {
                    Ok((_, drift)) if drift => {
                        own_state = "drifted";
                        findings.push(Value::from(
                            "matcher-drift: cheaper hook entry uses a pre-canonical matcher \
                             (run `cheaper hook-config sync` to converge)",
                        ));
                    }
                    Ok(_) => own_state = "present",
                    Err(e) => {
                        own_state = "modified";
                        blocker = true;
                        findings.push(Value::from(format!("modified: {e}")));
                    }
                }
            }
            _ => {
                own_state = "duplicate";
                blocker = true;
                findings.push(Value::from(
                    "duplicate: more than one cheaper hook entry in hooks.PreToolUse",
                ));
            }
        }
    }
    if own_state == "missing" && loaded.is_some() {
        blocker = true;
        findings.push(Value::from(
            "missing: no cheaper hook entry in hooks.PreToolUse \
             (run `cheaper hook-config sync`)",
        ));
    }
    result.insert("ownEntry".to_string(), Value::from(own_state));

    // legacy conflict scan — informational(dedup により共存は安全)。
    let conflicts = legacy_conflicts(&root_dir, &config_path);
    for f in &conflicts {
        findings.push(Value::from(format!(
            "legacy-conflict: {f} contains a cheaper hook registration \
             (safe to keep — duplicate firings are deduplicated at eval \
             time for identical requests; remove that registration to \
             converge on a single registration)"
        )));
    }
    result.insert(
        "legacyConflicts".to_string(),
        Value::Array(conflicts.iter().map(|f| Value::from(f.clone())).collect()),
    );

    // shadow/coexistence — `.devin/hooks.v1.json` 等が config.json と
    // 同居すると precedence 規則で shadow される可能性を報告する。
    let mut shadows: Vec<String> = Vec::new();
    for name in ["hooks.v1.json", "hooks.json"] {
        let p = root_dir.join(".devin").join(name);
        if p.is_file() && p != config_path {
            shadows.push(p.display().to_string());
        }
    }
    if !shadows.is_empty() {
        for s in &shadows {
            findings.push(Value::from(format!(
                "shadow: {s} may take precedence over .devin/config.json \
                 (verify which file the Devin CLI actually reads)"
            )));
        }
    }
    result.insert(
        "shadows".to_string(),
        Value::Array(shadows.iter().map(|s| Value::from(s.clone())).collect()),
    );

    // orphan temp/bak — `.tmp` は自身の atomic write 残留で次回 sync が
    // 自動回収、`.bak` は cheaper 管理外 file で削除判断は user。
    let orphans = find_orphans(&config_path);
    if !orphans.is_empty() {
        for o in &orphans {
            if o.ends_with(".tmp") {
                findings.push(Value::from(format!(
                    "orphan: {o} (leftover atomic-write temp; auto-removed \
                     by the next `hook-config sync`)"
                )));
            } else {
                findings.push(Value::from(format!(
                    "orphan: {o} (not managed by cheaper; remove manually \
                     if unwanted)"
                )));
            }
        }
    }
    result.insert(
        "orphans".to_string(),
        Value::Array(orphans.iter().map(|o| Value::from(o.clone())).collect()),
    );

    if loaded.is_none() && !blocker {
        findings.push(Value::from(
            "absent-config: no Devin config at the resolved path \
             (run `cheaper hook-config sync` to create one)",
        ));
        blocker = true;
    }

    result.insert("findings".to_string(), Value::Array(findings));
    result.insert("clean".to_string(), Value::from(!blocker));
    if !quiet && blocker {
        result.insert(
            "remediation".to_string(),
            Value::from(
                "run `cheaper hook-config sync` to converge the registration; \
                 see docs/guides/distribution.md for the APM lifecycle wiring",
            ),
        );
    }
    Ok((result, if blocker { 2 } else { 0 }))
}

/// `sync` — canonical own entry の冪等 upsert + legacy conflict の
/// informational 報告。
fn run_sync(
    args: &HookConfigCliArgs,
    io: &CliIo<'_>,
    quiet: bool,
) -> Result<(Map<String, Value>, i32), CheaperError> {
    check_capability(io)?;
    let config_path = resolve_config_path(args, io)?;
    check_no_reparse(&config_path)?;
    let root_dir = args
        .project_root
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(io.cwd.clone()));

    // legacy conflict は blocker ではなく informational — dedup が
    // 重複発火を無害化するため upsert を継続し、結果で報告する。
    let conflicts = legacy_conflicts(&root_dir, &config_path);

    let removed_orphans = sweep_orphans(&config_path);
    let loaded = load_config(&config_path)?;
    let mut result = Map::new();
    result.insert(
        "configPath".to_string(),
        Value::from(config_path.display().to_string()),
    );
    result.insert(
        "scope".to_string(),
        Value::from(args.scope.as_deref().unwrap_or("project")),
    );
    if !removed_orphans.is_empty() {
        result.insert(
            "orphansRemoved".to_string(),
            Value::Array(
                removed_orphans
                    .iter()
                    .map(|o| Value::from(o.clone()))
                    .collect(),
            ),
        );
    }

    let mut changed = false;
    let (mut root, expected_raw) = match loaded {
        Some(l) => (l.root, Some(l.raw)),
        None => (Map::new(), None),
    };

    // own entry の収束。
    let own_indexes: Vec<usize> = root
        .get("hooks")
        .and_then(|h| h.get("PreToolUse"))
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .filter_map(|(i, g)| is_own_group(g).then_some(i))
                .collect()
        })
        .unwrap_or_default();
    match own_indexes.len() {
        0 => {
            let hooks = root
                .entry("hooks".to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            let hooks_obj = hooks.as_object_mut().expect("just inserted object");
            let pre = hooks_obj
                .entry("PreToolUse".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            let arr = pre.as_array_mut().expect("just inserted array");
            arr.push(canonical_entry());
            changed = true;
        }
        1 => {
            let arr = root
                .get_mut("hooks")
                .and_then(|h| h.get_mut("PreToolUse"))
                .and_then(Value::as_array_mut)
                .expect("validated");
            let group = &mut arr[own_indexes[0]];
            let (_, drift) = own_group_matcher(group)?;
            if drift {
                group
                    .as_object_mut()
                    .expect("validated")
                    .insert("matcher".to_string(), Value::from(CANONICAL_MATCHER));
                changed = true;
            }
        }
        _ => {
            return Err(invalid_request(
                "multiple cheaper hook entries found — resolve the ambiguity before sync",
            ));
        }
    }

    if changed {
        write_config_atomic(&config_path, expected_raw.as_deref(), &root)?;
    }
    result.insert("changed".to_string(), Value::from(changed));
    result.insert("matcher".to_string(), Value::from(CANONICAL_MATCHER));
    result.insert(
        "legacyConflicts".to_string(),
        Value::Array(conflicts.iter().map(|f| Value::from(f.clone())).collect()),
    );
    if !conflicts.is_empty() {
        result.insert(
            "warning".to_string(),
            Value::from(
                "legacy cheaper hook registrations remain (see legacyConflicts); \
                 coexisting is safe — duplicate firings are deduplicated at eval \
                 time for identical requests, so removal is needed only to \
                 converge on a single registration. A pre-dedup cheaper binary \
                 on PATH still evaluates each firing independently (duplicate \
                 evaluation only — the verdict is identical); see \
                 docs/guides/distribution.md",
            ),
        );
    }
    if !quiet {
        result.insert(
            "note".to_string(),
            Value::from(
                "hook-config manages only the entry it owns; unrelated hooks are preserved",
            ),
        );
    }
    Ok((result, 0))
}

/// `remove` — own entry のみ除去(非破壊)。modified/ambiguous は拒否。
fn run_remove(
    args: &HookConfigCliArgs,
    io: &CliIo<'_>,
    _quiet: bool,
) -> Result<(Map<String, Value>, i32), CheaperError> {
    let config_path = resolve_config_path(args, io)?;
    check_no_reparse(&config_path)?;
    let mut result = Map::new();
    result.insert(
        "configPath".to_string(),
        Value::from(config_path.display().to_string()),
    );
    result.insert(
        "scope".to_string(),
        Value::from(args.scope.as_deref().unwrap_or("project")),
    );

    let loaded = match load_config(&config_path)? {
        Some(l) => l,
        None => {
            result.insert("changed".to_string(), Value::from(false));
            result.insert("ownEntry".to_string(), Value::from("absent"));
            return Ok((result, 0));
        }
    };
    let mut root = loaded.root;
    let expected_raw = loaded.raw;

    match loaded.own_indexes.len() {
        0 => {
            result.insert("changed".to_string(), Value::from(false));
            result.insert("ownEntry".to_string(), Value::from("absent"));
            return Ok((result, 0));
        }
        1 => {}
        _ => {
            return Err(invalid_request(
                "multiple cheaper hook entries found — resolve the ambiguity before remove",
            ));
        }
    }
    let idx = loaded.own_indexes[0];
    let arr = root
        .get_mut("hooks")
        .and_then(|h| h.get_mut("PreToolUse"))
        .and_then(Value::as_array_mut)
        .expect("validated");
    // shape が strict 既知形でない(外部変更)場合は消さない — 所有境界を
    // 越えて他の hook 設定を壊さないため。
    own_group_matcher(&arr[idx])?;
    arr.remove(idx);
    write_config_atomic(&config_path, Some(&expected_raw), &root)?;
    result.insert("changed".to_string(), Value::from(true));
    result.insert("ownEntry".to_string(), Value::from("removed"));

    // 他 file に残る非 APM 登録は remove 管轄外 — residual を verify
    // へ委ねる旨を報告する。
    let root_dir = args
        .project_root
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(io.cwd.clone()));
    let residual = legacy_conflicts(&root_dir, &config_path);
    if !residual.is_empty() {
        result.insert(
            "residualRegistrations".to_string(),
            Value::Array(residual.iter().map(|r| Value::from(r.clone())).collect()),
        );
        result.insert(
            "note".to_string(),
            Value::from(
                "remove deleted only the canonical entry; other files still contain \
                 cheaper registrations (safe to keep — duplicate firings are \
                 deduplicated at eval time for identical requests); run \
                 `cheaper hook-config verify` to inspect",
            ),
        );
    }
    Ok((result, 0))
}

// ---------------------------------------------------------------------------
// --event-stdin (APM lifecycle)
// ---------------------------------------------------------------------------

/// payload の `packages` に cheaper が含まれるか。entry は string
/// (`"cheaper"`, `"scarecrow173/cheaper"`)または object
/// (`{name|slug|package}`)。`packages` 自体が無い/非 array は
/// `None`(判定不能)を返す。
fn payload_has_cheaper(packages: Option<&Value>) -> Option<bool> {
    let arr = packages?.as_array()?;
    let is_cheaper = |s: &str| {
        let name = s.rsplit('/').next().unwrap_or(s);
        name == "cheaper" || s == "cheaper"
    };
    Some(arr.iter().any(|p| {
        match p {
            Value::String(s) => is_cheaper(s),
            Value::Object(o) => ["name", "slug", "package", "source"]
                .iter()
                .filter_map(|k| o.get(*k).and_then(Value::as_str))
                .any(is_cheaper),
            _ => false,
        }
    }))
}

/// `--event-stdin` — APM lifecycle event を stdin から読み、
/// `post-install`/`post-update` → sync+verify、`post-uninstall` →
/// (packages に cheaper が含まれるときのみ) remove、それ以外 → noop。
/// event が無い/malformed は error。
fn run_event_stdin(
    args: &HookConfigCliArgs,
    io: &mut CliIo<'_>,
    quiet: bool,
) -> Result<(Map<String, Value>, i32), CheaperError> {
    let text = if io.stdin.is_tty() {
        String::new()
    } else {
        match io.stdin.read_all() {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(_) => String::new(),
        }
    };
    let payload: Value = serde_json::from_str(&text).map_err(|e| {
        invalid_request(format!(
            "--event-stdin requires a JSON lifecycle payload: {e}"
        ))
    })?;
    let event = payload
        .get("event")
        .or_else(|| payload.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_request("--event-stdin payload has no \"event\" field"))?;

    let mut result = Map::new();
    result.insert("event".to_string(), Value::from(event));

    match event {
        "post-install" | "post-update" => {
            let (sync_result, _) = run_sync(args, io, quiet)?;
            let (verify_result, verify_code) = run_verify(args, io, quiet)?;
            result.insert("sync".to_string(), Value::Object(sync_result));
            result.insert("verify".to_string(), Value::Object(verify_result));
            Ok((result, verify_code))
        }
        "post-uninstall" => {
            match payload_has_cheaper(payload.get("packages")) {
                Some(true) => {
                    let (remove_result, code) = run_remove(args, io, quiet)?;
                    result.insert("remove".to_string(), Value::Object(remove_result));
                    Ok((result, code))
                }
                Some(false) => {
                    result.insert("action".to_string(), Value::from("noop"));
                    result.insert(
                        "reason".to_string(),
                        Value::from("uninstalled package is not cheaper"),
                    );
                    Ok((result, 0))
                }
                None => {
                    // packages 判定不能 — remove せず残留を verify で
                    // 報告する方向へ倒す(誤って remove しない)。
                    let (verify_result, code) = run_verify(args, io, quiet)?;
                    result.insert("action".to_string(), Value::from("skipped"));
                    result.insert(
                        "reason".to_string(),
                        Value::from(
                            "packages field missing/undetermined — removal skipped; verify reported",
                        ),
                    );
                    result.insert("verify".to_string(), Value::Object(verify_result));
                    Ok((result, code))
                }
            }
        }
        e if e.starts_with("pre-") => {
            result.insert("action".to_string(), Value::from("noop"));
            result.insert("reason".to_string(), Value::from("pre-* event"));
            Ok((result, 0))
        }
        other => {
            result.insert("action".to_string(), Value::from("noop"));
            result.insert(
                "reason".to_string(),
                Value::from(format!("unknown event: {other}")),
            );
            Ok((result, 0))
        }
    }
}

// ---------------------------------------------------------------------------
// dispatch
// ---------------------------------------------------------------------------

/// `cheaper hook-config` entry。envelope + exit code の通常 contract
/// (utility command — usage log 記録対象外)。
pub async fn execute(
    args: &HookConfigCliArgs,
    io: &mut CliIo<'_>,
    hooks: &CliHooks,
    started_at: f64,
) -> Result<CommandOut, crate::cli::BoxError> {
    let (result, exit_code) = if args.event_stdin {
        run_event_stdin(args, io, args.quiet)?
    } else {
        match args.action.expect("parse guarantees action or event-stdin") {
            HookConfigAction::Sync => run_sync(args, io, args.quiet)?,
            HookConfigAction::Verify => run_verify(args, io, args.quiet)?,
            HookConfigAction::Remove => run_remove(args, io, args.quiet)?,
        }
    };
    let _ = hooks; // builtin_dir は hook-config では使わない。
    Ok(CommandOut {
        envelope: utility_success_envelope(result, crate::cli::now(hooks) - started_at),
        exit_code,
        model_line: None,
    })
}
