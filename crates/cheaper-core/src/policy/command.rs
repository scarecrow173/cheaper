//! `command` policy section(design `policy-gated-command-tool` §(a)/(b)/(d)
//! + 統一 exec grant model: issue #165 の段 1 resolver)。
//!
//! trusted layer(builtin + user-scope `policy.yaml` + invocation
//! `command.grants`)のみが authoring できる grant 集合を resolve する。
//! `ResolvedCommandSection` は `ResolvedProcessSection` と同じ serialize
//! 規約(`{maxConcurrent, grants, timeout, maxOutputBytes, killGrace,
//! heartbeat:{interval}}`)を維持し、grant は `policy::exec::ExecGrant`
//! (`fixed`/`scope`/`inputs`/`outputs` の既定値畳み込み済み)として
//! 保持する — synthesis(`build_exec_input`)の入力となる中間段。
//!
//! serde-native 維持: resolved grant の `argv` は pattern の **source
//! string** を保持し、`ArgPattern` は match 時に compile する —
//! invocation parity check(`to_value` → 再 resolve → `PartialEq`)が
//! declared-shape の equality を見るため。
//!
//! deny 照合(適用点 1)は `resolve_exec_policy` へ一本化 — 本 resolver
//! は deny 引数を取らない(authoring 境界 validation のみ担う)。

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::duration::DURATION_PATTERN;
use crate::errors::{CheaperError, ErrorCode};
use crate::js::MAX_SAFE_INTEGER_U64;
use crate::policy::exec::{
    bounded_duration, relative, resolve_env, str_vec, ArgPattern, ExecGrant, ExecGrantScope,
    ExecHeartbeat, DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_KILL_GRACE, DEFAULT_MAX_OUTPUT_BYTES,
    DEFAULT_TIMEOUT, MAX_PROCESS_OUTPUT_BYTES,
};

/// `ResolvedCommandSection`(旧 `ResolvedCommandPolicy`)— `command`
/// section の resolved 中間段。`grants` は grant name → resolved
/// `ExecGrant` の map。pin 済み canonical path は載せない
/// (declared-shape equality が parity check の比較対象 — design §(b))。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCommandSection {
    pub max_concurrent: u64,
    pub grants: BTreeMap<String, ExecGrant>,
    pub timeout: String,
    pub max_output_bytes: u64,
    pub kill_grace: String,
    pub heartbeat: ExecHeartbeat,
}

/// `command_policy_properties` — `command` section の JSON schema
/// properties(design §(a) + issue #165 の `fixed`/`inputs`/`outputs`/
/// `scope` 拡張 — trusted authoring は grant の 2 意味化 flag・
/// provenance・resource 宣言を書ける。CLI 経路(`--grant`/`policy grant
/// add`)は別層の entry validation で `fixed`/`scope` を拒否する)。
/// complete schema 側の section required key は
/// `["maxConcurrent", "grants"]`(`process` の `["maxConcurrent",
/// "tasks"]` を mirror)。
pub fn command_policy_properties() -> Value {
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
        "grants": {
            "type": "object",
            "maxProperties": 128,
            "propertyNames": {"pattern": "^[a-z][a-z0-9_-]{0,63}$"},
            "additionalProperties": {
                "type": "object",
                "additionalProperties": false,
                "required": ["executable"],
                "properties": {
                    "executable": bounded_string,
                    "fixed": {"type": "boolean"},
                    "argv": {"type": "array", "maxItems": 256, "items": bounded_string},
                    "inputs": paths,
                    "outputs": paths,
                    "cwd": bounded_string,
                    "env": {
                        "type": "object",
                        "maxProperties": 64,
                        "propertyNames": {"pattern": "^[A-Za-z_][A-Za-z0-9_]*$"},
                        "additionalProperties": {"type": "string", "maxLength": 4096},
                    },
                    "timeout": {"type": "string", "pattern": DURATION_PATTERN},
                    "killGrace": {"type": "string", "pattern": DURATION_PATTERN},
                    "maxOutputBytes": {"type": "integer", "minimum": 1, "maximum": MAX_PROCESS_OUTPUT_BYTES},
                    "exitCodes": {
                        "type": "array",
                        "maxItems": 64,
                        "items": {"type": "integer", "minimum": 0, "maximum": 4294967295u64},
                    },
                    "scope": {"type": "string", "enum": ["trusted", "project"]},
                },
            },
        },
    })
}

fn command_input_validator() -> &'static jsonschema::Validator {
    static V: std::sync::OnceLock<jsonschema::Validator> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        jsonschema::draft202012::new(&json!({
            "type": "object",
            "additionalProperties": false,
            "properties": command_policy_properties(),
        }))
        .expect("command policy input schema compiles")
    })
}

/// `invalid` の `command` 版 — message prefix `"invalid command policy"`。
/// `process` 側の `invalid`("invalid process policy" 固定)とは prefix が
/// 異なるため共通化しない(design §(k) の判断)。
pub fn command_invalid(reason: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRuntimePolicy,
        format!("invalid command policy: {reason}"),
    )
}

/// `resolveCommandPolicy`: raw `command` section(`Option<&Value>`)を
/// boundary validation + lifecycle defaults で resolve する。
/// `resolve_process_policy` と同じく `None`/`null` は `{}` に collapseし、
/// `command` section を持たない policy は empty grants(=deny)へ
/// normalize される(design §(l))。
///
/// issue #165 — deny 照合(適用点 1)は `resolve_exec_policy` へ一本化
/// したため `deny` 引数を持たない。`fixed`/`scope`/`inputs`/`outputs`
/// の既定値(`fixed:false`・`scope:"trusted"`・未宣言)はここで
/// 畳み込み、grant は `ExecGrant` として保持する。
pub fn resolve_command_policy(
    input: Option<&Value>,
) -> Result<ResolvedCommandSection, CheaperError> {
    let raw = match input {
        None | Some(Value::Null) => json!({}),
        Some(v) => v.clone(),
    };
    if !command_input_validator().is_valid(&raw) {
        return Err(command_invalid("missing, unknown or invalid config field"));
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
        "command",
    )?;
    let kill_grace = bounded_duration(
        raw_obj
            .get("killGrace")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_KILL_GRACE),
        "killGrace",
        "command",
    )?;
    let interval = bounded_duration(
        raw_obj
            .get("heartbeat")
            .and_then(Value::as_object)
            .and_then(|h| h.get("interval"))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL),
        "heartbeat.interval",
        "command",
    )?;
    let max_output_bytes = raw_obj
        .get("maxOutputBytes")
        .and_then(crate::js::safe_u64)
        .unwrap_or(DEFAULT_MAX_OUTPUT_BYTES);
    let empty_grants = Map::new();
    let raw_grants = raw_obj
        .get("grants")
        .and_then(Value::as_object)
        .unwrap_or(&empty_grants);
    let mut names: Vec<&String> = raw_grants.keys().collect();
    names.sort_by(|a, b| crate::js::locale_compare(a, b));
    let mut grants = BTreeMap::new();
    for name in names {
        let grant = &raw_grants[name];
        let executable = grant
            .get("executable")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let fixed = grant.get("fixed").and_then(Value::as_bool).unwrap_or(false);
        let scope = match grant.get("scope").and_then(Value::as_str) {
            Some("project") => ExecGrantScope::Project,
            _ => ExecGrantScope::Trusted,
        };
        // `argv` は optional — `!fixed` は pattern(省略 = args 個数
        // 無制約)で構造 compile 検査、`fixed` は literal 扱いで
        // compile しない(exec stage の `resolve_exec_policy` が同
        // 規則で再評価する — serialized shape 経由で冪等)。
        let argv = match grant.get("argv") {
            Some(v) if !v.is_null() => {
                let pattern = str_vec(v);
                if !fixed {
                    ArgPattern::compile(&pattern)?;
                }
                Some(pattern)
            }
            _ => None,
        };
        let cwd = grant
            .get("cwd")
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_string();
        relative(&cwd, "command")?;
        // `inputs`/`outputs` は宣言時のみ `relative` 検査(未宣言は
        // `None` — 保守的 manifest に落ちる)。
        let inputs = grant.get("inputs").filter(|v| !v.is_null()).map(str_vec);
        let outputs = grant.get("outputs").filter(|v| !v.is_null()).map(str_vec);
        for value in inputs.iter().flatten().chain(outputs.iter().flatten()) {
            relative(value, "command")?;
        }
        // `env` は `process` task と同一規則(NUL 拒否・Windows case
        // 重複拒否。key pattern は schema が担保)。
        let env = resolve_env(grant.get("env"), command_invalid)?;
        // per-grant override ?? section default の畳み込み(origin =
        // `command` section — `fixed:true` を `command.grants` に書いて
        // も lifecycle は command defaults から畳む design §(b) の
        // 意図的非対称)。
        let grant_timeout = match grant.get("timeout").and_then(Value::as_str) {
            Some(v) => bounded_duration(v, "timeout", "command")?,
            None => timeout.clone(),
        };
        let grant_kill_grace = match grant.get("killGrace").and_then(Value::as_str) {
            Some(v) => bounded_duration(v, "killGrace", "command")?,
            None => kill_grace.clone(),
        };
        let grant_max_output_bytes = grant
            .get("maxOutputBytes")
            .and_then(crate::js::safe_u64)
            .unwrap_or(max_output_bytes);
        // 未宣言は `[0]`(従来挙動)。schema が非 integer/負値を
        // 排除済み。
        let exit_codes = grant
            .get("exitCodes")
            .and_then(Value::as_array)
            .map(|codes| codes.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_else(|| vec![0]);
        grants.insert(
            name.clone(),
            ExecGrant {
                executable,
                fixed,
                argv,
                inputs,
                outputs,
                cwd,
                env,
                timeout: grant_timeout,
                kill_grace: grant_kill_grace,
                max_output_bytes: grant_max_output_bytes,
                exit_codes,
                scope,
            },
        );
    }
    Ok(ResolvedCommandSection {
        max_concurrent: raw_obj
            .get("maxConcurrent")
            .and_then(crate::js::safe_u64)
            .unwrap_or(1),
        grants,
        timeout,
        max_output_bytes,
        kill_grace,
        heartbeat: ExecHeartbeat { interval },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_absent() {
        let p = resolve_command_policy(None).unwrap();
        assert_eq!(p.max_concurrent, 1);
        assert_eq!(p.timeout, "2m");
        assert_eq!(p.max_output_bytes, 65_536);
        assert_eq!(p.kill_grace, "2s");
        assert_eq!(p.heartbeat.interval, "30s");
        assert!(p.grants.is_empty());
        // null coalesces to the same defaults as absent.
        let p = resolve_command_policy(Some(&Value::Null)).unwrap();
        assert!(p.grants.is_empty());
    }

    #[test]
    fn rejects_unknown_fields_and_bad_durations() {
        let e = resolve_command_policy(Some(&json!({"bogus": 1}))).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("invalid command policy"));
        let e = resolve_command_policy(Some(&json!({"timeout": "5x"}))).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        let e = resolve_command_policy(Some(&json!({"timeout": "100000000000000000000h"})))
            .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    #[test]
    fn resolve_accepts_declared_executable_names() {
        // declared executable の内容は審査しない — policy resolve は
        // PATH 非依存で受理する(pin・spawn viability は tools 層)。
        for exe in [
            "python3",
            "python3.13t",
            "env",
            "ld-linux-x86-64.so.2",
            "bash",
            "deploy.sh",
            "git",
            "git.exe",
            "cargo",
        ] {
            let p = resolve_command_policy(Some(&json!({
                "grants": {"g": {"executable": exe}}
            })))
            .unwrap_or_else(|e| panic!("{exe}: grant resolves: {e}"));
            assert_eq!(p.grants["g"].executable, exe);
        }
        let p = resolve_command_policy(Some(&json!({
            "grants": {"git-read": {"executable": "git"}}
        })))
        .unwrap();
        assert_eq!(p.grants["git-read"].cwd, ".");
        assert_eq!(p.grants["git-read"].timeout, "2m");
        // `fixed`/`scope` 既定値畳み込み — authoring 省略で
        // `fixed:false`・`scope:"trusted"`、宣言は保守的 manifest の
        // `None`(undeclared)。
        let g = &p.grants["git-read"];
        assert!(!g.fixed);
        assert_eq!(g.scope, ExecGrantScope::Trusted);
        assert!(g.inputs.is_none());
        assert!(g.outputs.is_none());
    }

    /// `fixed`/`scope`/`inputs`/`outputs` の新規 field を trusted
    /// authoring が受理し、`fixed` grant は command section defaults
    /// で lifecycle を畳む(design §(b) の意図的非対称)。
    #[test]
    fn accepts_fixed_scope_and_resource_fields() {
        let p = resolve_command_policy(Some(&json!({
            "grants": {
                "fix": {"executable": "cc", "fixed": true,
                        "argv": ["--version"], "inputs": ["src/**"],
                        "outputs": [], "scope": "project"},
            }
        })))
        .unwrap();
        let g = &p.grants["fix"];
        assert!(g.fixed);
        assert_eq!(g.scope, ExecGrantScope::Project);
        assert_eq!(g.argv, Some(vec!["--version".to_string()]));
        assert_eq!(g.inputs, Some(vec!["src/**".to_string()]));
        assert_eq!(g.outputs, Some(vec![]));
        // wildcard 構造違反でも `fixed` なら argv は literal —
        // compile しない(`resolve_exec_policy` と同じ規則)。
        let p = resolve_command_policy(Some(&json!({
            "grants": {"fix": {"executable": "cc", "fixed": true, "argv": ["a*b"]}}
        })))
        .unwrap();
        assert_eq!(p.grants["fix"].argv, Some(vec!["a*b".to_string()]));
        // `!fixed` は pattern — `a*b` は構造違反で拒否。
        assert!(resolve_command_policy(Some(&json!({
            "grants": {"g": {"executable": "cc", "argv": ["a*b"]}}
        })))
        .is_err());
        // inputs/outputs の traversal は authoring 境界で拒否。
        assert!(resolve_command_policy(Some(&json!({
            "grants": {"fix": {"executable": "cc", "fixed": true, "outputs": ["../x"]}}
        })))
        .is_err());
    }

    #[test]
    fn grant_overrides_fold_and_relative_cwd_checked() {
        let p = resolve_command_policy(Some(&json!({
            "timeout": "5m",
            "maxOutputBytes": 1024,
            "grants": {
                "a": {"executable": "git", "timeout": "30s", "killGrace": "1s",
                      "maxOutputBytes": 512, "cwd": "sub", "env": {"K": "v"}},
                "b": {"executable": "cargo"},
            },
        })))
        .unwrap();
        let a = &p.grants["a"];
        assert_eq!(a.timeout, "30s");
        assert_eq!(a.kill_grace, "1s");
        assert_eq!(a.max_output_bytes, 512);
        assert_eq!(a.cwd, "sub");
        assert_eq!(a.env["K"], json!("v"));
        let b = &p.grants["b"];
        assert_eq!(b.timeout, "5m");
        assert_eq!(b.max_output_bytes, 1024);
        // cwd escape は workspace-relative 規則で拒否。
        for cwd in ["..", "a/../b", "/abs", "c:\\x"] {
            let e = resolve_command_policy(Some(&json!({
                "grants": {"g": {"executable": "git", "cwd": cwd}}
            })))
            .unwrap_err();
            assert!(e.message().contains("workspace-relative"), "{cwd}");
        }
    }

    #[test]
    fn resolved_section_is_serde_native_for_parity() {
        // `to_value` → 再 resolve → `PartialEq`(invocation parity check と
        // 同じ経路)。argv は source string のまま往復し、`fixed`/
        // `scope`/`inputs`/`outputs` も serialized 形で冪等。
        let raw = json!({
            "grants": {
                "g": {"executable": "git", "argv": ["log", "*"], "env": {"K": "v"}}
            },
        });
        let p = resolve_command_policy(Some(&raw)).unwrap();
        let serialized = serde_json::to_value(&p).unwrap();
        let reparsed = resolve_command_policy(Some(&serialized)).unwrap();
        assert_eq!(p, reparsed);
        // argv 省略 grant は serialize で key を出さない。
        assert!(serialized["grants"]["g"].get("argv").is_some());
        let p = resolve_command_policy(Some(&json!({
            "grants": {"g": {"executable": "git"}}
        })))
        .unwrap();
        let serialized = serde_json::to_value(&p).unwrap();
        assert!(serialized["grants"]["g"].get("argv").is_none());
        assert_eq!(p, resolve_command_policy(Some(&serialized)).unwrap());
    }
}
