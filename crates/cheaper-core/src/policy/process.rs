//! Port of `packages/core/src/policy/process.ts` — `process` section の
//! authoring 境界 validation + section defaults 供給(統一 exec grant
//! model: issue #165 の段 1 resolver)。
//!
//! task → grant 変換・単一 grant namespace・`argv` 契約・pin/deny の
//! 実行経路は `policy::exec` が所有する。本 module は authoring 形
//! (`process.tasks`)の typed 中間段のみを保持し、`tasks` の typed map
//! は `build_exec_input` の synthesis 入力となる。shared 実行概念
//! (`bounded_duration`/`relative`/`str_vec`/heartbeat/argv shell
//! regex/lifecycle const)は `policy::exec` へ移設済み。

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::duration::DURATION_PATTERN;
use crate::errors::{CheaperError, ErrorCode};
use crate::js::MAX_SAFE_INTEGER_U64;
use crate::policy::exec::{
    bounded_duration, relative, resolve_env, str_vec, ExecHeartbeat, DEFAULT_HEARTBEAT_INTERVAL,
    DEFAULT_KILL_GRACE, DEFAULT_MAX_OUTPUT_BYTES, DEFAULT_TIMEOUT, MAX_PROCESS_OUTPUT_BYTES,
};

/// `ProcessTask` — `process.tasks` の authoring decl(typed tasks —
/// resolved section は `Map<String,Value>` ではなく typed map を
/// 保持)。`cwd`/`env` は resolver が必ず populate する従来不変。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcessTask {
    pub command: Vec<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub cwd: String,
    pub env: Map<String, Value>,
}

/// `ResolvedProcessSection`(旧 `ResolvedProcessPolicy`)— `process`
/// section の resolved 中間段(defaults + typed tasks)。
/// serialization は `{maxConcurrent, tasks, timeout, maxOutputBytes,
/// killGrace, heartbeat}` の従来 shape を維持する(load.rs の
/// re-embed が complete schema の section required key を満たす規約)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedProcessSection {
    pub max_concurrent: u64,
    pub tasks: BTreeMap<String, ProcessTask>,
    pub timeout: String,
    pub max_output_bytes: u64,
    pub kill_grace: String,
    pub heartbeat: ExecHeartbeat,
}

/// `PROCESS_POLICY_PROPERTIES` verbatim (duration `pattern` uses the
/// shared `DURATION_PATTERN` string). authoring schema は不変
/// (issue #165 — task 宣言形への変更はない)。
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

/// `resolveProcessPolicy`: typed runtime options go through the same
/// config boundary validation; caller-mutable values are not retained.
/// Input is the raw `process` section (`Option<&Value>`)。
///
/// 戻り型は `ResolvedProcessPolicy` → `ResolvedProcessSection` へ改名
/// (issue #165 — 統一 exec 層の段 1 resolver で、resolved policy では
/// なく synthesis への中間入力を表す)。`tasks` は typed map。
pub fn resolve_process_policy(
    input: Option<&Value>,
) -> Result<ResolvedProcessSection, CheaperError> {
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
    let mut tasks = BTreeMap::new();
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
        let cwd = task
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_string();
        relative(&cwd, "process")?;
        for value in inputs.iter().chain(&outputs) {
            relative(value, "process")?;
        }
        // `env` の NUL/Windows-case-dup 規則 — shared helper で
        // section prefix("invalid process policy:")を attribution。
        let env = resolve_env(task.get("env"), invalid)?;
        tasks.insert(
            name.clone(),
            ProcessTask {
                command,
                inputs,
                outputs,
                cwd,
                env,
            },
        );
    }
    Ok(ResolvedProcessSection {
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
        heartbeat: ExecHeartbeat { interval },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn accepts_shell_executables_and_argv_metachars() {
        // task command/argv の内容は審査しない — declared contract
        // (wildcard 構造・cwd/env・lifecycle)のみを強制する。
        for command in [
            json!(["bash", "-c", "x"]),
            json!(["run.sh"]),
            json!(["echo", "a|b"]),
            json!(["echo", "$(x)"]),
        ] {
            let p = resolve_process_policy(Some(&json!({
                "tasks": {"t": {"command": command, "inputs": [], "outputs": []}}
            })))
            .unwrap_or_else(|e| panic!("{command}: task resolves: {e}"));
            assert_eq!(json!(p.tasks["t"].command), command);
        }
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
        assert_eq!(a.cwd, "sub");
        assert_eq!(a.env["K"], json!("v"));
        assert_eq!(p.tasks["b"].cwd, ".");
        assert!(p.tasks["b"].env.is_empty());
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
        let keys: Vec<&String> = p.tasks["t"].env.keys().collect();
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
}
