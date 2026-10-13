//! Port of `packages/core/src/policy/process.ts`.

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::duration::{parse_duration, DURATION_PATTERN};
use crate::errors::{CheaperError, ErrorCode};
use crate::js::MAX_SAFE_INTEGER_U64;
use crate::nodepath;
use crate::resources::normalization::{ResourceManifestDeclaration, ScopeDeclaration};
use crate::resources::types::ResourceScope;

/// `ProcessTask` (resolved shape: `cwd`/`env` are always populated by
/// `resolveProcessPolicy`, matching the TS assignment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessTask {
    pub command: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub cwd: String,
    pub env: Map<String, Value>,
}

/// `ResolvedProcessPolicy["heartbeat"]` (`{interval: string}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessHeartbeat {
    pub interval: String,
}

/// `ResolvedProcessPolicy` (lifecycle defaults applied). Serialization
/// reproduces the TS field order/names (`maxConcurrent`, `tasks`,
/// `timeout`, `maxOutputBytes`, `killGrace`, `heartbeat`) so it can be
/// re-embedded into the effective policy object like TS does.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedProcessPolicy {
    pub max_concurrent: u64,
    pub tasks: Map<String, Value>,
    pub timeout: String,
    pub max_output_bytes: u64,
    pub kill_grace: String,
    pub heartbeat: ProcessHeartbeat,
}

/// `MAX_PROCESS_OUTPUT_BYTES` (16 MiB).
pub const MAX_PROCESS_OUTPUT_BYTES: u64 = 16_777_216;
/// `MAX_TIMER_MS` (signed 32-bit timer limit).
pub const MAX_TIMER_MS: u64 = 2_147_483_647;

/// `command` section が同じ lifecycle default 規約を共有するため
/// `pub(crate)`(design §(a) — `{maxConcurrent, grants, timeout,
/// maxOutputBytes, killGrace, heartbeat:{interval}}` 同型)。
pub(crate) const DEFAULT_TIMEOUT: &str = "2m";
pub(crate) const DEFAULT_MAX_OUTPUT_BYTES: u64 = 65_536;
pub(crate) const DEFAULT_KILL_GRACE: &str = "2s";
pub(crate) const DEFAULT_HEARTBEAT_INTERVAL: &str = "30s";

/// `PROCESS_POLICY_PROPERTIES` verbatim (duration `pattern` uses the
/// shared `DURATION_PATTERN` string).
pub fn process_policy_properties() -> Value {
    let bounded_string = json!({"type": "string", "minLength": 1, "maxLength": 4096});
    let paths = json!({"type": "array", "maxItems": 256, "items": bounded_string});
    json!({
        "maxConcurrent": {"type": "integer", "minimum": 1, "maximum": MAX_SAFE_INTEGER_U64},
        "timeout": {"type": "string", "pattern": DURATION_PATTERN},
        "maxOutputBytes": {"type": "integer", "minimum": 1, "maximum": MAX_PROCESS_OUTPUT_BYTES},
        "killGrace": {"type": "string", "pattern": DURATION_PATTERN},
        "heartbeat": {
            "type": "object",
            "additionalProperties": false,
            "properties": {"interval": {"type": "string", "pattern": DURATION_PATTERN}},
        },
        "tasks": {
            "type": "object",
            "maxProperties": 128,
            "propertyNames": {"pattern": "^[a-z][a-z0-9_-]{0,63}$"},
            "additionalProperties": {
                "type": "object",
                "additionalProperties": false,
                "required": ["command", "inputs", "outputs"],
                "properties": {
                    "command": {"type": "array", "minItems": 1, "maxItems": 256, "items": bounded_string},
                    "inputs": paths,
                    "outputs": paths,
                    "cwd": bounded_string,
                    "env": {
                        "type": "object",
                        "maxProperties": 64,
                        "propertyNames": {"pattern": "^[A-Za-z_][A-Za-z0-9_]*$"},
                        "additionalProperties": {"type": "string", "maxLength": 4096},
                    },
                },
            },
        },
    })
}

fn process_input_validator() -> &'static jsonschema::Validator {
    static V: std::sync::OnceLock<jsonschema::Validator> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        jsonschema::draft202012::new(&json!({
            "type": "object",
            "additionalProperties": false,
            "properties": process_policy_properties(),
        }))
        .expect("process policy input schema compiles")
    })
}

fn invalid(reason: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRuntimePolicy,
        format!("invalid process policy: {reason}"),
    )
}

fn shell_executable_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            "^(?:sh|bash|dash|zsh|fish|ksh|csh|tcsh|ash|cmd|powershell|pwsh|busybox)(?:\\.exe)?$",
        )
        .unwrap()
    })
}

fn script_ext_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new("\\.(?:cmd|bat|ps1|sh)$").unwrap())
}

/// argv token / pattern の shell metachar 検査 shared regex。
/// cheaper-tools の `command` execute が cross-crate で参照するため `pub`。
pub fn argv_shell_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new("[\0\r\n|&;<>`]|\\$\\(").unwrap())
}

/// `isShellProcessExecutable`: the same forbidden-executable rules apply
/// after alias realpath resolution.
pub fn is_shell_process_executable(executable: &str) -> bool {
    let name = executable
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    shell_executable_re().is_match(&name) || script_ext_re().is_match(&name)
}

/// workspace-relative path 検査。error prefix は呼出し側の section
/// label(`"process"`/`"command"`)に合わせる — "path must be
/// workspace-relative without traversal" 自体は section 非依存。
pub(crate) fn relative(input: &str, label: &str) -> Result<(), CheaperError> {
    // Both platform `path.isAbsolute` and always-win32 `path.win32.
    // isAbsolute`, plus `/^[a-z]:/i` and `..` segments on either
    // separator — exactly the TS checks.
    let traversal = input.split(['/', '\\']).any(|s| s == "..");
    let drive = {
        let b = input.as_bytes();
        b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
    };
    if input.contains('\0')
        || nodepath::is_absolute(input)
        || nodepath::win32_is_absolute(input)
        || drive
        || traversal
    {
        return Err(CheaperError::new(
            ErrorCode::InvalidRuntimePolicy,
            format!("invalid {label} policy: path must be workspace-relative without traversal"),
        ));
    }
    Ok(())
}

pub(crate) fn str_vec(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// `MAX_TIMER_MS` 上限の bounded duration 検査を section 間で共有する。
/// `label` は error message の section 名("process"/"command") —
/// `invalid {label} policy: {field} exceeds the {label} timer limit`。
pub(crate) fn bounded_duration(
    value: &str,
    field: &str,
    label: &str,
) -> Result<String, CheaperError> {
    let over = || {
        CheaperError::new(
            ErrorCode::InvalidRuntimePolicy,
            format!("invalid {label} policy: {field} exceeds the {label} timer limit"),
        )
    };
    let milliseconds = match parse_duration(value) {
        Ok(ms) => ms,
        Err(_) => return Err(over()),
    };
    if milliseconds > MAX_TIMER_MS {
        return Err(over());
    }
    Ok(value.to_string())
}

/// `resolveProcessPolicy`: typed runtime options go through the same
/// config boundary validation; caller-mutable values are not retained.
/// Input is the raw `process` section (`Option<&Value>`).
pub fn resolve_process_policy(
    input: Option<&Value>,
) -> Result<ResolvedProcessPolicy, CheaperError> {
    // TS `raw = input ?? {}` then `validate(raw)`: `undefined`/`null`
    // coalesce to `{}`; other non-object inputs fail `type: "object"`.
    let raw = match input {
        None | Some(Value::Null) => json!({}),
        Some(v) => v.clone(),
    };
    if !process_input_validator().is_valid(&raw) {
        return Err(invalid("missing, unknown or invalid config field"));
    }
    let empty = Map::new();
    let raw_obj: &Map<String, Value> = match &raw {
        Value::Object(m) => m,
        _ => &empty, // unreachable post-validation
    };
    let timeout = bounded_duration(
        raw_obj
            .get("timeout")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_TIMEOUT),
        "timeout",
        "process",
    )?;
    let kill_grace = bounded_duration(
        raw_obj
            .get("killGrace")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_KILL_GRACE),
        "killGrace",
        "process",
    )?;
    let interval = bounded_duration(
        raw_obj
            .get("heartbeat")
            .and_then(Value::as_object)
            .and_then(|h| h.get("interval"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL),
        "heartbeat.interval",
        "process",
    )?;
    let mut tasks = Map::new();
    let empty_tasks = Map::new();
    let raw_tasks = raw_obj
        .get("tasks")
        .and_then(Value::as_object)
        .unwrap_or(&empty_tasks);
    // TS sorts entries with `a.localeCompare(b)` — same comparator for
    // task names and env keys (task names are `[a-z0-9_-]`-only so the
    // two orders coincide there; env keys may mix case).
    let mut names: Vec<&String> = raw_tasks.keys().collect();
    names.sort_by(|a, b| crate::js::locale_compare(a, b));
    for name in names {
        let task = &raw_tasks[name];
        let command = str_vec(&task["command"]);
        let inputs = str_vec(&task["inputs"]);
        let outputs = str_vec(&task["outputs"]);
        if command
            .first()
            .is_some_and(|c| is_shell_process_executable(c))
        {
            return Err(invalid("shell or script shim executable is forbidden"));
        }
        if command.iter().any(|arg| argv_shell_re().is_match(arg)) {
            return Err(invalid("shell syntax in argv is forbidden"));
        }
        let cwd = task
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_string();
        relative(&cwd, "process")?;
        for value in inputs.iter().chain(&outputs) {
            relative(value, "process")?;
        }
        let mut env = Map::new();
        let mut seen = std::collections::HashSet::new();
        let mut env_entries: Vec<(&String, &Value)> = task
            .get("env")
            .and_then(Value::as_object)
            .map(|e| e.iter().collect())
            .unwrap_or_default();
        env_entries.sort_by(|(a, _), (b, _)| crate::js::locale_compare(a, b));
        for (key, value) in env_entries {
            let identity = if cfg!(windows) {
                key.to_uppercase()
            } else {
                (*key).clone()
            };
            if !seen.insert(identity) || value.as_str().is_none_or(|v| v.contains('\0')) {
                return Err(invalid("invalid or duplicate environment variable"));
            }
            env.insert(key.clone(), value.clone());
        }
        tasks.insert(
            name.clone(),
            json!({
                "command": command,
                "inputs": inputs,
                "outputs": outputs,
                "cwd": cwd,
                "env": env,
            }),
        );
    }
    Ok(ResolvedProcessPolicy {
        max_concurrent: raw_obj
            .get("maxConcurrent")
            .and_then(crate::js::safe_u64)
            .unwrap_or(1),
        tasks,
        timeout,
        max_output_bytes: raw_obj
            .get("maxOutputBytes")
            .and_then(crate::js::safe_u64)
            .unwrap_or(DEFAULT_MAX_OUTPUT_BYTES),
        kill_grace,
        heartbeat: ProcessHeartbeat { interval },
    })
}

/// `resolveProcessTask` (`policy.tasks[name]`, `Object.hasOwn` check).
pub fn resolve_process_task(
    policy: &ResolvedProcessPolicy,
    name: &str,
) -> Result<ProcessTask, CheaperError> {
    let task = policy.tasks.get(name).ok_or_else(|| {
        let mut details = Map::new();
        details.insert("task".to_string(), json!(name));
        CheaperError::new(
            ErrorCode::PolicyViolation,
            "unknown or unauthorized process task",
        )
        .with_details(details)
    })?;
    Ok(task_from_value(task))
}

fn task_from_value(task: &Value) -> ProcessTask {
    let env = task
        .get("env")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    ProcessTask {
        command: str_vec(&task["command"]),
        inputs: str_vec(&task["inputs"]),
        outputs: str_vec(&task["outputs"]),
        cwd: task
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_string(),
        env,
    }
}

/// `processTaskResources`: workspace-root-relative declarations; glob
/// safety expansion and ownership verification belong to core resources.
pub fn process_task_resources(
    policy: &ResolvedProcessPolicy,
    name: &str,
) -> Result<ResourceManifestDeclaration, CheaperError> {
    let task = resolve_process_task(policy, name)?;
    let scopes = |patterns: &[String]| -> Vec<ScopeDeclaration> {
        patterns
            .iter()
            .map(|value| ScopeDeclaration {
                path: value.clone(),
                scope: if ends_with_globstar(value) {
                    ResourceScope::Subtree
                } else {
                    ResourceScope::Exact
                },
            })
            .collect()
    };
    Ok(ResourceManifestDeclaration {
        reads: Some(scopes(&task.inputs)),
        writes: Some(scopes(&task.outputs)),
        produces: Some(scopes(&task.outputs)),
    })
}

/// `/(?:^|[\\/])\*\*$/`
fn ends_with_globstar(value: &str) -> bool {
    if let Some(rest) = value.strip_suffix("**") {
        rest.is_empty() || rest.ends_with('/') || rest.ends_with('\\')
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_executable_detection() {
        for exe in [
            "sh",
            "bash",
            "cmd.exe",
            "powershell",
            "pwsh",
            "bin/zsh",
            "x.PS1",
            "run.sh",
            "a.BAT",
            "busybox",
        ] {
            assert!(is_shell_process_executable(exe), "{exe}");
        }
        for exe in ["shell", "command", "pwsh-tool", "a.shx"] {
            assert!(!is_shell_process_executable(exe), "{exe}");
        }
    }

    #[test]
    fn defaults_when_absent() {
        let p = resolve_process_policy(None).unwrap();
        assert_eq!(p.max_concurrent, 1);
        assert_eq!(p.timeout, "2m");
        assert_eq!(p.max_output_bytes, 65_536);
        assert_eq!(p.kill_grace, "2s");
        assert_eq!(p.heartbeat.interval, "30s");
        assert!(p.tasks.is_empty());
    }

    #[test]
    fn rejects_unknown_fields_and_bad_durations() {
        let e = resolve_process_policy(Some(&json!({"bogus": 1}))).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        let e = resolve_process_policy(Some(&json!({"timeout": "5x"}))).unwrap_err();
        assert!(e.message().contains("invalid process policy"));
        // Over the timer limit.
        let e = resolve_process_policy(Some(&json!({"timeout": "100000000000000000000h"})))
            .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    #[test]
    fn rejects_shell_executables_and_argv_syntax() {
        let base = json!({
            "tasks": {"t": {"command": ["bash", "-c", "x"], "inputs": [], "outputs": []}}
        });
        let e = resolve_process_policy(Some(&base)).unwrap_err();
        assert!(e.message().contains("shell or script shim"));
        let base = json!({
            "tasks": {"t": {"command": ["echo", "a|b"], "inputs": [], "outputs": []}}
        });
        let e = resolve_process_policy(Some(&base)).unwrap_err();
        assert!(e.message().contains("shell syntax in argv"));
    }

    #[test]
    fn rejects_absolute_and_traversal_task_paths() {
        let abs = if cfg!(windows) { "C:\\x" } else { "/x" };
        for task in [
            json!({"command": ["cc"], "inputs": [abs], "outputs": []}),
            json!({"command": ["cc"], "inputs": ["a/../b"], "outputs": []}),
            json!({"command": ["cc"], "inputs": [], "outputs": [], "cwd": "d:e"}),
            json!({"command": ["cc"], "inputs": ["a\\..\\b"], "outputs": []}),
        ] {
            let e = resolve_process_policy(Some(&json!({"tasks": {"t": task}}))).unwrap_err();
            assert!(e.message().contains("workspace-relative"), "{task}");
        }
    }

    #[test]
    fn task_normalization_sorted_and_filled() {
        let p = resolve_process_policy(Some(&json!({
            "tasks": {
                "b": {"command": ["bb"], "inputs": ["i"], "outputs": ["o"]},
                "a": {"command": ["aa"], "inputs": [], "outputs": [], "cwd": "sub", "env": {"K": "v"}},
            }
        })))
        .unwrap();
        let names: Vec<&String> = p.tasks.keys().collect();
        assert_eq!(names, ["a", "b"]);
        let a = &p.tasks["a"];
        assert_eq!(a["cwd"], json!("sub"));
        assert_eq!(a["env"]["K"], json!("v"));
        assert_eq!(p.tasks["b"]["cwd"], json!("."));
        assert_eq!(p.tasks["b"]["env"], json!({}));
    }

    #[test]
    fn env_keys_sort_like_locale_compare() {
        // TS `a.localeCompare(b)`: base letters first — "A" < "b" < "C"
        // (byte order would produce A, C, b).
        let p = resolve_process_policy(Some(&json!({
            "tasks": {"t": {"command": ["cc"], "inputs": [], "outputs": [],
                          "env": {"C": "3", "b": "1", "A": "2"}}}
        })))
        .unwrap();
        let keys: Vec<&String> = p.tasks["t"]["env"].as_object().unwrap().keys().collect();
        assert_eq!(keys, ["A", "b", "C"]);
    }

    #[test]
    fn env_case_collision_rejected_on_windows() {
        let raw = json!({
            "tasks": {"t": {"command": ["cc"], "inputs": [], "outputs": [],
                          "env": {"Path": "a", "PATH": "b"}}}
        });
        let result = resolve_process_policy(Some(&raw));
        if cfg!(windows) {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn resolve_process_task_and_resources() {
        let p = resolve_process_policy(Some(&json!({
            "tasks": {"build": {"command": ["make"], "inputs": ["src/**"], "outputs": ["out.txt"]}}
        })))
        .unwrap();
        let e = resolve_process_task(&p, "nope").unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        let task = resolve_process_task(&p, "build").unwrap();
        assert_eq!(task.command, vec!["make".to_string()]);
        let res = process_task_resources(&p, "build").unwrap();
        let reads = res.reads.unwrap();
        assert_eq!(reads[0].path, "src/**");
        assert_eq!(reads[0].scope, ResourceScope::Subtree);
        assert_eq!(res.writes.unwrap()[0].scope, ResourceScope::Exact);
    }
}
