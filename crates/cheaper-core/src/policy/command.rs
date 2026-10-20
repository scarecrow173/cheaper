//! `command` policy section(design `policy-gated-command-tool` §(a)/(b)/(d))。
//!
//! trusted layer(builtin + user-scope `policy.yaml`)のみが authoring
//! できる grant 集合を resolve する。`ResolvedCommandPolicy` は
//! `ResolvedProcessPolicy` と同じ serialize 規約(`{maxConcurrent, grants,
//! timeout, maxOutputBytes, killGrace, heartbeat:{interval}}`)で、
//! resolved grant は effective な `timeout`/`killGrace`/`maxOutputBytes`
//! (grant override ?? section default)を畳み込む。
//!
//! serde-native 維持: resolved grant の `argv` は pattern の **source
//! string** を保持し、`ArgPattern` は match 時に compile する —
//! invocation parity check(`to_value` → 再 resolve → `PartialEq`)が
//! declared-shape の equality を見るため。

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::duration::DURATION_PATTERN;
use crate::errors::{CheaperError, ErrorCode};
use crate::js::MAX_SAFE_INTEGER_U64;
use crate::nodepath;
use crate::policy::process::{
    argv_shell_re, bounded_duration, is_shell_process_executable, relative, str_vec,
    ProcessHeartbeat, DEFAULT_HEARTBEAT_INTERVAL, DEFAULT_KILL_GRACE, DEFAULT_MAX_OUTPUT_BYTES,
    DEFAULT_TIMEOUT, MAX_PROCESS_OUTPUT_BYTES,
};
use crate::resources::normalization::{ResourceManifestDeclaration, ScopeDeclaration};
use crate::resources::types::ResourceScope;

/// `CommandGrant`(resolved shape: `cwd`/`env`/lifecycle override は
/// `resolveCommandPolicy` が必ず populate する)。`argv` は未指定
/// (args 個数無制約)と空指定を区別するため `Option`。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandGrant {
    pub executable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argv: Option<Vec<String>>,
    pub cwd: String,
    pub env: Map<String, Value>,
    pub timeout: String,
    pub kill_grace: String,
    pub max_output_bytes: u64,
    /// 完了扱いにする exit code 集合(default `[0]`)。route scope
    /// では scope option の `accept_any_exit_code` がこの集合を
    /// 上書きする — ここは declared grant の値のみを保持する。
    pub exit_codes: Vec<i64>,
}

/// `ResolvedCommandPolicy`(lifecycle defaults applied)。`grants` は
/// grant name → resolved grant の map。pin 済み canonical path は載せない
/// (declared-shape equality が parity check の比較対象 — design §(b))。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCommandPolicy {
    pub max_concurrent: u64,
    pub grants: BTreeMap<String, CommandGrant>,
    pub timeout: String,
    pub max_output_bytes: u64,
    pub kill_grace: String,
    pub heartbeat: ProcessHeartbeat,
}

impl ResolvedCommandPolicy {
    /// route run の単一 grant 投影(design C9)— `grants` を対象
    /// grant 1 件へ限定し、routed workflow(project 層由来でも
    /// 可)が他の user grant を `input.name` で呼ぶ confused
    /// deputy を構造的に防ぐ。Shared/invocation 構築**前**の
    /// resolved policy に適用するため worker scope の parity
    /// check は保たれる。対象 grant が無ければ false(呼出し側が
    /// route 宣言不整合として扱う)。
    pub fn project_grant(&mut self, name: &str) -> bool {
        let Some(grant) = self.grants.get(name).cloned() else {
            return false;
        };
        self.grants = BTreeMap::from([(name.to_string(), grant)]);
        true
    }
}

/// `command_policy_properties` — `command` section の JSON schema
/// properties(design §(a))。complete schema 側の section required key は
/// `["maxConcurrent", "grants"]`(`process` の `["maxConcurrent", "tasks"]`
/// を mirror)。
pub fn command_policy_properties() -> Value {
    let bounded_string = json!({"type": "string", "minLength": 1, "maxLength": 4096});
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
                    "argv": {"type": "array", "maxItems": 256, "items": bounded_string},
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

/// `command` 専用の denied executable regex(design §(d))。basename を
/// 小文字化し `.exe`/`.com` suffix を剥がしてから match する —
/// interpreter(versioned name まで)・exec wrapper/launcher・dynamic
/// loader・Windows LOLBin・editor/pager/REPL escape・追加 shell を含む。
/// shell/shim(`sh`/`bash`/`cmd`/script ext)は process baseline の
/// `is_shell_process_executable` が別途担う — この deny set は
/// containment でなく known-bad への fail-fast。
pub fn command_denied_executable_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(concat!(
            "^(?:",
            // interpreters(versioned name まで)
            "pythonw?|python\\d+(?:\\.\\d+)*[a-z]?|",
            "pypy\\d*(?:\\.\\d+)*|perl\\d*(?:\\.\\d+)*|ruby\\d*(?:\\.\\d+)*|",
            "node|nodejs|deno|bun|php\\d*(?:\\.\\d+)*|",
            "lua|luajit|lua[\\d.]+|r|rscript|julia|",
            "wscript|cscript|mshta|java|javaw|groovy|tclsh|wish|expect|osascript|",
            // exec wrapper / launcher — grant は shell の grant と等価
            "env|xargs|find|awk|gawk|mawk|nawk|sed|busybox|toybox|",
            "nice|nohup|setsid|timeout|watch|stdbuf|flock|taskset|",
            "sudo|doas|su|pkexec|setpriv|nsenter|unshare|chroot|",
            "docker|podman|systemd-run|ssh|scp|rsync|screen|tmux|",
            // dynamic loader
            "ld\\.so|ld-linux.*|ld-musl.*|dyld|",
            // Windows LOLBin + wsl
            "rundll32|regsvr32|msbuild|dotnet|installutil|regasm|regsvcs|",
            "forfiles|pcalua|msiexec|wsl|jshell|",
            // editor / pager / REPL escape
            "vim|ex|nvim|less|man|mysql|psql|sqlite3|tar|",
            // 追加 shell
            "nu|elvish|xonsh|oil|ion",
            ")$"
        ))
        .expect("command denied executable regex compiles")
    })
}

/// `is_denied_command_executable` — basename 小文字化 + `.exe`/`.com`
/// strip 後に `command_denied_executable_re` で match する。
/// `is_shell_process_executable` と並ぶ policy-level deny wrapper で、
/// declared basename・pin 時 resolved basename・spawn 前再検証の
/// 3 適用点から参照される(design §(d))。
pub fn is_denied_command_executable(executable: &str) -> bool {
    let name = executable
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    let stem = name
        .strip_suffix(".exe")
        .or_else(|| name.strip_suffix(".com"))
        .unwrap_or(&name);
    command_denied_executable_re().is_match(stem)
}

/// design §(b) workspace-resident executable pre-filter: declared
/// executable が absolute path でなく `/`・`\` separator を含む relative
/// path なら `resolve_process_executable` の workspace-relative 解決へ
/// 着地する — cheaper-tools の pin が委譲前に `POLICY_VIOLATION` で
/// 拒否するための判定(realpath 後の inside-workspace 検査は tools 側)。
pub fn is_relative_executable_path(executable: &str) -> bool {
    let drive = {
        let b = executable.as_bytes();
        b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
    };
    !nodepath::is_absolute(executable)
        && !nodepath::win32_is_absolute(executable)
        && !drive
        && (executable.contains('/') || executable.contains('\\'))
}

/// argv pattern の positional token(design §(a))。literal exact /
/// trailing-`*` prefix match / 末尾 bare `*` の rest marker の 3 種のみ。
#[derive(Debug, Clone, PartialEq)]
enum ArgToken {
    Literal(String),
    Prefix(String),
    Rest,
}

/// `ArgPattern` — grant `argv` の compiled positional pattern。resolved
/// grant には source string を保持し、match 時に `compile` する。
#[derive(Debug, Clone, PartialEq)]
pub struct ArgPattern {
    tokens: Vec<ArgToken>,
}

impl ArgPattern {
    /// pattern compile。metachar(`argv_shell_re`)・最終要素以外の bare
    /// `*`・末尾以外/2 個以上の `*` を含む要素は `InvalidRuntimePolicy`
    /// で拒否(match 不能な pattern を静的に排除)。
    pub fn compile(pattern: &[String]) -> Result<Self, CheaperError> {
        let last = pattern.len().saturating_sub(1);
        let mut tokens = Vec::with_capacity(pattern.len());
        for (i, element) in pattern.iter().enumerate() {
            if argv_shell_re().is_match(element) {
                return Err(command_invalid("shell syntax in argv pattern is forbidden"));
            }
            let token = match element.matches('*').count() {
                0 => ArgToken::Literal(element.clone()),
                // 最終要素の bare `*` のみ zero-or-more rest marker。
                1 if element == "*" && i == last => ArgToken::Rest,
                1 if element == "*" => {
                    return Err(command_invalid(
                        "argv pattern rest marker is only allowed as the last element",
                    ));
                }
                // 末尾に `*` を 1 個だけ持ち prefix が非空 → prefix match。
                1 if element.ends_with('*') => {
                    ArgToken::Prefix(element[..element.len() - 1].to_string())
                }
                _ => {
                    return Err(command_invalid(
                        "argv pattern wildcard is only allowed as a trailing \
                         rest marker or a single trailing prefix match",
                    ));
                }
            };
            tokens.push(token);
        }
        Ok(Self { tokens })
    }

    /// positional 全体 match。`Rest` を持たない pattern は arg 個数の
    /// 完全一致を要求し(fail-closed)、`Rest` 付きは fixed 要素以降の
    /// 残余 token を zero-or-more で受ける。
    pub fn matches(&self, args: &[String]) -> bool {
        let (fixed, rest) = match self.tokens.last() {
            Some(ArgToken::Rest) => (&self.tokens[..self.tokens.len() - 1], true),
            _ => (&self.tokens[..], false),
        };
        if rest {
            // args が pattern(trailing `*` を除く)より短い場合は reject。
            if args.len() < fixed.len() {
                return false;
            }
        } else if args.len() != fixed.len() {
            return false;
        }
        fixed
            .iter()
            .zip(args.iter())
            .all(|(token, arg)| match token {
                ArgToken::Literal(literal) => arg == literal,
                ArgToken::Prefix(prefix) => arg.starts_with(prefix.as_str()),
                ArgToken::Rest => unreachable!("rest marker is trailing-only"),
            })
    }
}

/// `resolveCommandPolicy`: raw `command` section(`Option<&Value>`)を
/// boundary validation + lifecycle defaults で resolve する。
/// `resolve_process_policy` と同じく `None`/`null` は `{}` に collapseし、
/// `command` section を持たない policy は empty grants(=deny)へ
/// normalize される(design §(l))。
pub fn resolve_command_policy(
    input: Option<&Value>,
) -> Result<ResolvedCommandPolicy, CheaperError> {
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
        // declared basename の fail-fast(design §(d) 適用点 1) —
        // process baseline(shell/shim/script ext)+ `command` 専用
        // interpreter/exec-wrapper deny set の双方を検査する。
        if is_shell_process_executable(&executable) {
            return Err(command_invalid(
                "shell or script shim executable is forbidden",
            ));
        }
        if is_denied_command_executable(&executable) {
            return Err(command_invalid(
                "interpreter or exec-capable executable is forbidden",
            ));
        }
        // `argv` は optional — 省略時は args 個数無制約(metachar 拒否と
        // 長さ cap のみ)。pattern は resolve 時に compile 検証だけ行い、
        // resolved grant には source string を保持する。
        let argv = match grant.get("argv") {
            Some(v) if !v.is_null() => {
                let pattern = str_vec(v);
                ArgPattern::compile(&pattern)?;
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
        // `env` は `process` task と同一規則(NUL 拒否・Windows case
        // 重複拒否。key pattern は schema が担保)。
        let mut env = Map::new();
        let mut seen = std::collections::HashSet::new();
        let mut env_entries: Vec<(&String, &Value)> = grant
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
                return Err(command_invalid("invalid or duplicate environment variable"));
            }
            env.insert(key.clone(), value.clone());
        }
        // per-grant override ?? section default の畳み込み。
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
            CommandGrant {
                executable,
                argv,
                cwd,
                env,
                timeout: grant_timeout,
                kill_grace: grant_kill_grace,
                max_output_bytes: grant_max_output_bytes,
                exit_codes,
            },
        );
    }
    Ok(ResolvedCommandPolicy {
        max_concurrent: raw_obj
            .get("maxConcurrent")
            .and_then(crate::js::safe_u64)
            .unwrap_or(1),
        grants,
        timeout,
        max_output_bytes,
        kill_grace,
        heartbeat: ProcessHeartbeat { interval },
    })
}

/// `resolveCommandGrant`(`policy.grants[name]` の `Object.hasOwn` 相当)。
/// 未知 grant は `POLICY_VIOLATION`(task resolve の "unknown or
/// unauthorized process task" に準じた error shape)。
pub fn resolve_command_grant(
    policy: &ResolvedCommandPolicy,
    name: &str,
) -> Result<CommandGrant, CheaperError> {
    policy.grants.get(name).cloned().ok_or_else(|| {
        let mut details = Map::new();
        details.insert("grant".to_string(), json!(name));
        CheaperError::new(
            ErrorCode::PolicyViolation,
            "unknown or unauthorized command grant",
        )
        .with_details(details)
    })
}

/// `command` の保守的 resource manifest(design §(f)) — workspace root
/// subtree の reads+writes を宣言する単一所有者。`produces` は宣言しない
/// (duplicate-producer 検出との衝突回避 + ownership semantics 不充足)。
pub fn command_grant_resources() -> ResourceManifestDeclaration {
    let root = || ScopeDeclaration {
        path: ".".to_string(),
        scope: ResourceScope::Subtree,
    };
    ResourceManifestDeclaration {
        reads: Some(vec![root()]),
        writes: Some(vec![root()]),
        produces: None,
    }
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
    fn denied_executables_cover_interpreters_wrappers_and_versioned_names() {
        for exe in [
            "python",
            "pythonw",
            "python3",
            "python3.13",
            "python3.13t",
            "python.exe",
            "pypy",
            "pypy3.10",
            "perl",
            "perl5.38",
            "ruby",
            "ruby3.2",
            "node",
            "nodejs",
            "node.exe",
            "node.com",
            "deno",
            "bun",
            "php",
            "php8.3",
            "lua",
            "luajit",
            "lua5.4",
            "r",
            "rscript",
            "julia",
            "wscript",
            "cscript",
            "mshta",
            "java",
            "javaw",
            "groovy",
            "tclsh",
            "wish",
            "expect",
            "osascript",
            "env",
            "xargs",
            "find",
            "awk",
            "sed",
            "busybox",
            "nice",
            "nohup",
            "setsid",
            "timeout",
            "watch",
            "stdbuf",
            "flock",
            "taskset",
            "sudo",
            "doas",
            "su",
            "pkexec",
            "setpriv",
            "nsenter",
            "unshare",
            "chroot",
            "docker",
            "podman",
            "systemd-run",
            "ssh",
            "scp",
            "rsync",
            "screen",
            "tmux",
            "ld.so",
            "ld-linux-x86-64.so.2",
            "ld-musl-x86_64.so.1",
            "dyld",
            "rundll32",
            "regsvr32",
            "msbuild",
            "dotnet",
            "installutil",
            "regasm",
            "regsvcs",
            "forfiles",
            "pcalua",
            "msiexec",
            "wsl",
            "jshell",
            "vim",
            "ex",
            "nvim",
            "less",
            "man",
            "mysql",
            "psql",
            "sqlite3",
            "tar",
            "nu",
            "elvish",
            "xonsh",
            "oil",
            "ion",
            // absolute path / dir 付きでも basename で捕捉する
            "/usr/bin/python3",
            "C:\\Tools\\node.exe",
        ] {
            assert!(is_denied_command_executable(exe), "{exe}");
        }
        for exe in [
            "git", "cargo", "make", "cc", "grep", "my-tool", "pythonic", "nodes",
        ] {
            assert!(!is_denied_command_executable(exe), "{exe}");
        }
        // shell/shim は command deny set ではなく process baseline が担う。
        assert!(!is_denied_command_executable("bash"));
        assert!(is_shell_process_executable("bash"));
    }

    #[test]
    fn resolve_rejects_denied_and_shell_executables() {
        for (exe, reason) in [
            ("python3", "interpreter or exec-capable"),
            ("python3.13t", "interpreter or exec-capable"),
            ("env", "interpreter or exec-capable"),
            ("ld-linux-x86-64.so.2", "interpreter or exec-capable"),
            ("bash", "shell or script shim"),
            ("deploy.sh", "shell or script shim"),
        ] {
            let e = resolve_command_policy(Some(&json!({
                "grants": {"g": {"executable": exe}}
            })))
            .unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{exe}");
            assert!(e.message().contains(reason), "{exe}: {}", e.message());
        }
        // 正当な executable は受理される(.exe suffix 付きの非 deny 名も)。
        for exe in ["git", "git.exe", "cargo"] {
            let p = resolve_command_policy(Some(&json!({
                "grants": {"git-read": {"executable": exe}}
            })))
            .unwrap();
            assert_eq!(p.grants["git-read"].executable, exe);
        }
        let p = resolve_command_policy(Some(&json!({
            "grants": {"git-read": {"executable": "git"}}
        })))
        .unwrap();
        assert_eq!(p.grants["git-read"].cwd, ".");
        assert_eq!(p.grants["git-read"].timeout, "2m");
    }

    #[test]
    fn argv_pattern_compile_and_match() {
        // literal 完全一致 — 超過 args は既定拒否(fail-closed)。
        let p = ArgPattern::compile(&["log".to_string()]).unwrap();
        assert!(p.matches(&["log".to_string()]));
        assert!(!p.matches(&["log".to_string(), "--oneline".to_string()]));
        assert!(!p.matches(&[]));
        // trailing rest marker は zero-or-more の残余を受ける。
        let p = ArgPattern::compile(&["log".to_string(), "*".to_string()]).unwrap();
        assert!(p.matches(&["log".to_string()]));
        assert!(p.matches(&[
            "log".to_string(),
            "--oneline".to_string(),
            "HEAD~3".to_string()
        ]));
        assert!(!p.matches(&["diff".to_string()]));
        assert!(!p.matches(&[]));
        // bare `*` 単体は unconstrained 相当。
        let p = ArgPattern::compile(&["*".to_string()]).unwrap();
        assert!(p.matches(&[]));
        assert!(p.matches(&["a".to_string(), "b".to_string()]));
        // trailing `*` の prefix match。
        let p = ArgPattern::compile(&["log".to_string(), "--format=*".to_string()]).unwrap();
        assert!(p.matches(&["log".to_string(), "--format=%H".to_string()]));
        assert!(!p.matches(&["log".to_string(), "--format".to_string()]));
        assert!(!p.matches(&["log".to_string(), "--stat".to_string()]));
        // interior `*`・複数 `*`・metachar は compile 時拒否。
        for bad in ["a*b", "**", "*a", "a|b", "x;y", "v$(i)"] {
            assert!(ArgPattern::compile(&[bad.to_string()]).is_err(), "{bad}");
        }
        // 最終要素以外の bare `*` は拒否。
        assert!(ArgPattern::compile(&["*".to_string(), "log".to_string()]).is_err());
        let e = resolve_command_policy(Some(&json!({
            "grants": {"g": {"executable": "git", "argv": ["a*b"]}}
        })))
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
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
    fn resolved_policy_is_serde_native_for_parity() {
        // `to_value` → 再 resolve → `PartialEq`(invocation parity check と
        // 同じ経路)。argv は source string のまま往復する。
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

    /// route run の単一 grant 投影(C9)— resolved policy の
    /// `grants` map が対象 1 件へ縮退し、投影外 grant は
    /// `resolve_command_grant` で unknown 扱いになる。
    /// resolved JSON への serialize は再 resolve で同一形に
    /// 戻る(scope parity check が保たれる)。
    #[test]
    fn project_grant_limits_grants_to_declared_route_grant() {
        let mut p = resolve_command_policy(Some(&json!({
            "grants": {
                "route-grant": {"executable": "git"},
                "other": {"executable": "my-tool"},
            }
        })))
        .unwrap();
        assert!(p.grants.contains_key("other"));
        assert!(p.project_grant("route-grant"));
        assert_eq!(p.grants.len(), 1);
        assert!(p.grants.contains_key("route-grant"));
        // 投影外 grant は unknown grant 扱い。
        assert!(resolve_command_grant(&p, "other").is_err());
        assert_eq!(
            resolve_command_grant(&p, "route-grant").unwrap().executable,
            "git"
        );
        // resolved → serialize → re-resolve round-trip が同一形
        // (worker scope parity check を通過する)。
        let serialized = serde_json::to_value(&p).unwrap();
        assert_eq!(p, resolve_command_policy(Some(&serialized)).unwrap());
        // 対象不在は false で policy を変えない。
        let mut q = resolve_command_policy(Some(&json!({
            "grants": {"g": {"executable": "git"}}
        })))
        .unwrap();
        assert!(!q.project_grant("absent"));
        assert!(q.grants.contains_key("g"));
    }

    #[test]
    fn resolve_command_grant_and_resources() {
        let p = resolve_command_policy(Some(&json!({
            "grants": {"g": {"executable": "git"}}
        })))
        .unwrap();
        let e = resolve_command_grant(&p, "nope").unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        let g = resolve_command_grant(&p, "g").unwrap();
        assert_eq!(g.executable, "git");
        // root-subtree reads+writes、produces は宣言しない。
        let res = command_grant_resources();
        assert_eq!(res.reads.as_ref().unwrap()[0].path, ".");
        assert_eq!(res.reads.as_ref().unwrap()[0].scope, ResourceScope::Subtree);
        assert_eq!(
            res.writes.as_ref().unwrap()[0].scope,
            ResourceScope::Subtree
        );
        assert!(res.produces.is_none());
    }

    #[test]
    fn relative_executable_path_prefilter() {
        for exe in ["bin/tool", "a\\b", "./run", ".\\run", "../x"] {
            assert!(is_relative_executable_path(exe), "{exe}");
        }
        let abs = if cfg!(windows) {
            "C:\\bin\\git.exe"
        } else {
            "/usr/bin/git"
        };
        for exe in ["git", "git.exe", "my-tool", abs] {
            assert!(!is_relative_executable_path(exe), "{exe}");
        }
    }
}
