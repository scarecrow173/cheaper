//! 統一 exec grant model(issue #165 — `command.grants` と `process.tasks`
//! の単一 grant namespace・argv 契約・provenance の owner)。
//!
//! 3 段 resolver 構成(ADR O4a/D4):
//!
//! 1. section resolver(`resolve_process_policy`/`resolve_command_policy` —
//!    `process.rs`/`command.rs`)が authoring 境界 validation と section
//!    defaults 供給を担う(`"invalid process policy:"`/`"invalid command
//!    policy:"` prefix で宣言位置を attribution)。
//! 2. [`build_exec_input`] が merged 2 section と project 宣言 task 名集合を
//!    exec-shaped doc(`ResolvedExecPolicy` の serialized shape と同一 input
//!    shape)へ畳む。`command.grants`×`process.tasks` の同名衝突はここで
//!    `InvalidRuntimePolicy` fail-loud(`details.{grant,task}`)。
//! 3. [`resolve_exec_policy`] が exec-doc を boundary validate + lifecycle
//!    畳み込み + 適用点 1(declared basename deny)で resolved へ。
//!
//! `RuntimePolicy.exec` は段 3 の出力のみを保持する — serialized round-trip
//! 冪等性(`to_value` → 再 resolve → `PartialEq`)が invocation parity の
//! 不変条件。authoring section 形(`process.tasks`/`command.grants` を内包
//! する doc)の direct 投入は `additionalProperties: false` が拒否するため、
//! caller は必ず段 2 を経由する。

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::duration::{parse_duration, DURATION_PATTERN};
use crate::errors::{CheaperError, ErrorCode};
use crate::js::MAX_SAFE_INTEGER_U64;
use crate::nodepath;
use crate::policy::command::resolve_command_policy;
use crate::policy::process::resolve_process_policy;
use crate::policy::security::ExecDenyList;
use crate::resources::normalization::{ResourceManifestDeclaration, ScopeDeclaration};
use crate::resources::types::ResourceScope;

/// grant provenance — authoring layer 由来の scope。
/// serde は `"trusted" | "project"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecGrantScope {
    Trusted,
    Project,
}

/// resolved unified grant — serialized shape は parity check の
/// 比較対象(declared-shape equality)。`argv` は `fixed` で意味が
/// 分岐する: fixed → 宣言 argv そのもの(task の command[1..] 等の
/// literal)、非 fixed → argv pattern(省略 = args 個数無制約)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecGrant {
    pub executable: String,
    pub fixed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argv: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,
    pub cwd: String,
    pub env: Map<String, Value>,
    /// 畳み込み済み per-grant lifecycle(origin section defaults ??
    /// per-grant override — 畳み込みは `build_exec_input` 経路で
    /// 確定済み。`exit_codes` 未宣言は `[0]`)。
    pub timeout: String,
    pub kill_grace: String,
    pub max_output_bytes: u64,
    pub exit_codes: Vec<i64>,
    pub scope: ExecGrantScope,
}

/// `heartbeat:{interval}`(旧 `ProcessHeartbeat` 改名 — 単一 heartbeat
/// はもはや process 専有ではない)。serde shape `{interval}` 不変。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecHeartbeat {
    pub interval: String,
}

/// origin section の lifecycle defaults block — `ResolvedExecPolicy`
/// 上に `command`/`process` の 2 block で保持(R3 — pool の
/// `maxConcurrent` と per-call heartbeat が origin 別)。
/// serialized shape `{maxConcurrent, timeout, maxOutputBytes,
/// killGrace, heartbeat:{interval}}` は現行 2 section と同型。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecDefaults {
    pub max_concurrent: u64,
    pub timeout: String,
    pub max_output_bytes: u64,
    pub kill_grace: String,
    pub heartbeat: ExecHeartbeat,
}

/// 統一 resolved policy — `RuntimePolicy.exec` が保持する唯一の
/// exec 集約。`grants` は name → resolved grant の単一 map
/// (command 由来・task 由来を区別しない — `fixed`/`scope` で
/// 判別)。pin 済み canonical path は載せない(declared-shape
/// parity — `ResolvedCommandPolicy` の規約を継承)。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedExecPolicy {
    pub grants: BTreeMap<String, ExecGrant>,
    /// `!fixed` grant の origin defaults(`command` section 由来)。
    pub command: ExecDefaults,
    /// `fixed` grant の origin defaults(`process` section 由来)。
    pub process: ExecDefaults,
}

impl ResolvedExecPolicy {
    /// route 参照の eligibility — `route.grant`・`grants[]` member・
    /// `substitute.grant` 全てに同 predicate を適用する単一 seam。
    pub fn is_route_eligible(grant: &ExecGrant) -> bool {
        grant.scope == ExecGrantScope::Trusted && !grant.fixed
    }

    /// route run の grant set 投影(#166 — 単一版の一般化)。`names`
    /// は `{route.grant} ∪ grants` の有効 set — 1 件でも不在・
    /// route-ineligible(非 trusted/fixed)なら false で policy を
    /// 変更しない。投影は `{names} ∪ {fixed grants}` を残す —
    /// routed workflow から task を呼べる挙動と C9 confused-deputy
    /// 防止(非 fixed の他 grant を隠す)を set でも維持する。
    pub fn project_grants(&mut self, names: &BTreeSet<String>) -> bool {
        if names.is_empty()
            || !names
                .iter()
                .all(|n| matches!(self.grants.get(n), Some(g) if Self::is_route_eligible(g)))
        {
            return false;
        }
        self.grants.retain(|n, g| names.contains(n) || g.fixed);
        true
    }
}

/// `MAX_PROCESS_OUTPUT_BYTES` (16 MiB)。
pub const MAX_PROCESS_OUTPUT_BYTES: u64 = 16_777_216;
/// `MAX_TIMER_MS` (signed 32-bit timer limit)。
pub const MAX_TIMER_MS: u64 = 2_147_483_647;

/// 両 section が共有する lifecycle default 規約。
pub(crate) const DEFAULT_TIMEOUT: &str = "2m";
pub(crate) const DEFAULT_MAX_OUTPUT_BYTES: u64 = 65_536;
pub(crate) const DEFAULT_KILL_GRACE: &str = "2s";
pub(crate) const DEFAULT_HEARTBEAT_INTERVAL: &str = "30s";

/// workspace-relative path 検査。error prefix は呼出し側の section
/// label(`"process"`/`"command"`/`"exec"`)に合わせる — "path must be
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
/// `label` は error message の section 名("process"/"command"/"exec") —
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

/// `command_invalid`/`security_invalid` と同型の error helper —
/// `"invalid exec policy: {reason}"` + `InvalidRuntimePolicy`。
fn exec_invalid(reason: &str) -> CheaperError {
    CheaperError::new(
        ErrorCode::InvalidRuntimePolicy,
        format!("invalid exec policy: {reason}"),
    )
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
/// `fixed:true` grant の `argv` は literal であり compile 対象外。
#[derive(Debug, Clone, PartialEq)]
pub struct ArgPattern {
    tokens: Vec<ArgToken>,
}

impl ArgPattern {
    /// pattern compile。最終要素以外の bare `*`・末尾以外/2 個以上の
    /// `*` を含む要素は `InvalidRuntimePolicy` で拒否(match 不能な
    /// pattern を静的に排除)。token の内容は審査しない。
    pub fn compile(pattern: &[String]) -> Result<Self, CheaperError> {
        let last = pattern.len().saturating_sub(1);
        let mut tokens = Vec::with_capacity(pattern.len());
        for (i, element) in pattern.iter().enumerate() {
            let token = match element.matches('*').count() {
                0 => ArgToken::Literal(element.clone()),
                // 最終要素の bare `*` のみ zero-or-more rest marker。
                1 if element == "*" && i == last => ArgToken::Rest,
                1 if element == "*" => {
                    return Err(exec_invalid(
                        "argv pattern rest marker is only allowed as the last element",
                    ));
                }
                // 末尾に `*` を 1 個だけ持ち prefix が非空 → prefix match。
                1 if element.ends_with('*') => {
                    ArgToken::Prefix(element[..element.len() - 1].to_string())
                }
                _ => {
                    return Err(exec_invalid(
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

/// `env` の NUL/Windows-case-dup 規則を全 grant へ適用する shared
/// helper(旧 `process.rs:255-275` 相当 — key pattern は schema が担保)。
/// `invalid` は呼出し層の error helper(`command_invalid`/`invalid`/
/// `exec_invalid`)で section prefix を attribution する。
pub(crate) fn resolve_env(
    raw: Option<&Value>,
    invalid: impl Fn(&str) -> CheaperError,
) -> Result<Map<String, Value>, CheaperError> {
    let mut env = Map::new();
    let mut seen = std::collections::HashSet::new();
    let mut env_entries: Vec<(&String, &Value)> = raw
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
    Ok(env)
}

/// defaults block の input properties — `{maxConcurrent, timeout,
/// maxOutputBytes, killGrace, heartbeat:{interval}}` のみ受理する
/// (`grants`/`tasks` key は unknown で reject)。nested `heartbeat` の
/// `additionalProperties: false` は round-trip/parity の不変条件
/// (design §(d) 実装注意)。
fn exec_defaults_properties() -> Value {
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
    })
}

/// exec-shaped doc の input properties(design §(d) verbatim)。
/// `additionalProperties: false` で authoring 形(`process.tasks`・
/// `command.grants` を内包する section そのもの)の direct 投入を
/// fail-loud に拒否 — caller は必ず `build_exec_input` を経由する。
pub fn exec_policy_properties() -> Value {
    let bounded_string = json!({"type": "string", "minLength": 1, "maxLength": 4096});
    let paths = json!({"type": "array", "maxItems": 256, "items": bounded_string});
    let defaults_schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": exec_defaults_properties(),
    });
    json!({
        "grants": {
            "type": "object",
            // 2 section × 128 の merge 上限。
            "maxProperties": 256,
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
                    "maxOutputBytes": {"type": "integer", "minimum": 1,
                                       "maximum": MAX_PROCESS_OUTPUT_BYTES},
                    "exitCodes": {"type": "array", "maxItems": 64,
                                  "items": {"type": "integer", "minimum": 0,
                                            "maximum": 4294967295u64}},
                    "scope": {"type": "string", "enum": ["trusted", "project"]},
                },
            },
        },
        "command": defaults_schema,
        "process": defaults_schema,
    })
}

fn exec_input_validator() -> &'static jsonschema::Validator {
    static V: std::sync::OnceLock<jsonschema::Validator> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        jsonschema::draft202012::new(&json!({
            "type": "object",
            "additionalProperties": false,
            "properties": exec_policy_properties(),
        }))
        .expect("exec policy input schema compiles")
    })
}

/// `command`/`process` defaults block を `ExecDefaults` へ resolve。
/// 欠落 block → 既定値。
fn resolve_exec_defaults(input: Option<&Value>) -> Result<ExecDefaults, CheaperError> {
    let empty = Map::new();
    let obj = input.and_then(Value::as_object).unwrap_or(&empty);
    Ok(ExecDefaults {
        max_concurrent: obj
            .get("maxConcurrent")
            .and_then(crate::js::safe_u64)
            .unwrap_or(1),
        timeout: bounded_duration(
            obj.get("timeout")
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_TIMEOUT),
            "timeout",
            "exec",
        )?,
        max_output_bytes: obj
            .get("maxOutputBytes")
            .and_then(crate::js::safe_u64)
            .unwrap_or(DEFAULT_MAX_OUTPUT_BYTES),
        kill_grace: bounded_duration(
            obj.get("killGrace")
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_KILL_GRACE),
            "killGrace",
            "exec",
        )?,
        heartbeat: ExecHeartbeat {
            interval: bounded_duration(
                obj.get("heartbeat")
                    .and_then(Value::as_object)
                    .and_then(|h| h.get("interval"))
                    .and_then(Value::as_str)
                    .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL),
                "heartbeat.interval",
                "exec",
            )?,
        },
    })
}

/// defaults block の serialized shape — `build_exec_input` が
/// section resolved defaults を exec-doc へ埋め込む形。
fn defaults_block(
    max_concurrent: u64,
    timeout: &str,
    max_output_bytes: u64,
    kill_grace: &str,
    interval: &str,
) -> Value {
    json!({
        "maxConcurrent": max_concurrent,
        "timeout": timeout,
        "maxOutputBytes": max_output_bytes,
        "killGrace": kill_grace,
        "heartbeat": {"interval": interval},
    })
}

/// 段 2 — synthesis: merged authoring 2 section + project 宣言 task
/// 名集合 → exec-shaped doc(`resolve_exec_policy` の入力 shape)へ畳む。
/// `command.grants` × `process.tasks` の同名衝突はここで
/// `InvalidRuntimePolicy` fail-loud(`details.{grant,task}`)。
pub fn build_exec_input(
    process: Option<&Value>,
    command: Option<&Value>,
    project_task_names: &BTreeSet<String>,
) -> Result<Map<String, Value>, CheaperError> {
    let process_section = resolve_process_policy(process)?;
    let command_section = resolve_command_policy(command)?;
    // `command.grants` keys ∩ `process.tasks` keys が非空 → collision。
    // `details.{grant,task}` に衝突名を載せ宣言位置を指す(merged-map
    // 評価のため `details.layer` は付かない — accepted gap)。
    if let Some(name) = command_section
        .grants
        .keys()
        .find(|name| process_section.tasks.contains_key(*name))
    {
        let mut details = Map::new();
        details.insert("grant".to_string(), json!(name));
        details.insert("task".to_string(), json!(name));
        return Err(
            exec_invalid("grant name collides between command.grants and process.tasks")
                .with_details(details),
        );
    }
    let mut grants = Map::new();
    // task → exec-doc grant 変換 — lifecycle は synthesis で明示値と
    // して畳み込む(origin = authoring section のため resolved grant に
    // origin field は不要 — round-trip も冪等)。
    for (name, task) in &process_section.tasks {
        grants.insert(
            name.clone(),
            json!({
                "executable": task.command.first().cloned().unwrap_or_default(),
                "fixed": true,
                "argv": task.command[1..].to_vec(),
                "inputs": task.inputs,
                "outputs": task.outputs,
                "cwd": task.cwd,
                "env": task.env,
                "timeout": process_section.timeout,
                "killGrace": process_section.kill_grace,
                "maxOutputBytes": process_section.max_output_bytes,
                "exitCodes": [0],
                "scope": if project_task_names.contains(name) {
                    "project"
                } else {
                    "trusted"
                },
            }),
        );
    }
    // command grant → exec-doc grant 変換(declared fields そのまま +
    // `fixed`/`scope`/`inputs`/`outputs` 既定値は section resolver の
    // 畳み込み済み値 — serialized `ExecGrant` がそのまま exec-doc 形)。
    for (name, grant) in &command_section.grants {
        grants.insert(
            name.clone(),
            serde_json::to_value(grant).expect("ExecGrant は必ず serialize 可能"),
        );
    }
    let mut out = Map::new();
    out.insert("grants".to_string(), Value::Object(grants));
    out.insert(
        "command".to_string(),
        defaults_block(
            command_section.max_concurrent,
            &command_section.timeout,
            command_section.max_output_bytes,
            &command_section.kill_grace,
            &command_section.heartbeat.interval,
        ),
    );
    out.insert(
        "process".to_string(),
        defaults_block(
            process_section.max_concurrent,
            &process_section.timeout,
            process_section.max_output_bytes,
            &process_section.kill_grace,
            &process_section.heartbeat.interval,
        ),
    );
    Ok(out)
}

/// 段 3 — unified resolver: exec-shaped doc(`{grants:{..},
/// command:{defaults}, process:{defaults}}` — `ResolvedExecPolicy` の
/// serialized shape と同一 input shape)を boundary validate +
/// lifecycle 畳み込み + 適用点 1(declared basename deny)で resolved
/// へ。`None`/`null` は `{}` に collapse — exec 節を持たない入力は
/// empty grants(=deny)へ normalize される(現行規約継承)。
pub fn resolve_exec_policy(
    input: Option<&Value>,
    deny: &ExecDenyList,
) -> Result<ResolvedExecPolicy, CheaperError> {
    let raw = match input {
        None | Some(Value::Null) => json!({}),
        Some(v) => v.clone(),
    };
    if !exec_input_validator().is_valid(&raw) {
        return Err(exec_invalid("missing, unknown or invalid config field"));
    }
    let empty = Map::new();
    let raw_obj: &Map<String, Value> = match &raw {
        Value::Object(m) => m,
        _ => &empty, // unreachable post-validation
    };
    let command_defaults = resolve_exec_defaults(raw_obj.get("command"))?;
    let process_defaults = resolve_exec_defaults(raw_obj.get("process"))?;
    let empty_grants = Map::new();
    let raw_grants = raw_obj
        .get("grants")
        .and_then(Value::as_object)
        .unwrap_or(&empty_grants);
    // `locale_compare` 順(現行 2 resolver と同じ整列規約)。
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
        // 適用点 1 — declared basename deny(全 grant 一律 — task 由来
        // 含む)。`details.{grant,executable}` を載せ、fixed grant は
        // `details.task` も併記(宣言位置が `process.tasks` である旨の
        // attribution)。
        if deny.is_denied(&executable) {
            let mut details = Map::new();
            details.insert("grant".to_string(), json!(name));
            details.insert("executable".to_string(), json!(executable));
            if fixed {
                details.insert("task".to_string(), json!(name));
            }
            return Err(
                exec_invalid("executable is denied by security.execDenyPatterns")
                    .with_details(details),
            );
        }
        let scope = match grant.get("scope").and_then(Value::as_str) {
            Some("project") => ExecGrantScope::Project,
            _ => ExecGrantScope::Trusted,
        };
        // `argv` の 2 意味化: `!fixed` なら `ArgPattern::compile` で
        // pattern 構造検査、`fixed` なら literal 扱いで compile しない。
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
        relative(&cwd, "exec")?;
        let inputs = grant.get("inputs").filter(|v| !v.is_null()).map(str_vec);
        let outputs = grant.get("outputs").filter(|v| !v.is_null()).map(str_vec);
        for value in inputs.iter().flatten().chain(outputs.iter().flatten()) {
            relative(value, "exec")?;
        }
        let env = resolve_env(grant.get("env"), exec_invalid)?;
        // lifecycle 欠落 field は `fixed` → `process` defaults、
        // `!fixed` → `command` defaults で畳み込み(synthesis 経路では
        // 全値明示のため dormant fallback — 直接 exec-doc 入力向け)。
        let defaults = if fixed {
            &process_defaults
        } else {
            &command_defaults
        };
        let timeout = match grant.get("timeout").and_then(Value::as_str) {
            Some(v) => bounded_duration(v, "timeout", "exec")?,
            None => defaults.timeout.clone(),
        };
        let kill_grace = match grant.get("killGrace").and_then(Value::as_str) {
            Some(v) => bounded_duration(v, "killGrace", "exec")?,
            None => defaults.kill_grace.clone(),
        };
        let max_output_bytes = grant
            .get("maxOutputBytes")
            .and_then(crate::js::safe_u64)
            .unwrap_or(defaults.max_output_bytes);
        // `exit_codes` 未宣言は `[0]`(従来挙動)。
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
                timeout,
                kill_grace,
                max_output_bytes,
                exit_codes,
                scope,
            },
        );
    }
    Ok(ResolvedExecPolicy {
        grants,
        command: command_defaults,
        process: process_defaults,
    })
}

/// surface 別の grant 解決 — `fixed` 期待値と `grant.fixed` が
/// 一致しない場合は surface 固有の unknown error を返す
/// (disjoint surface: fixed surface → "unknown or unauthorized
/// process task" + `details.task`、非 fixed surface → "unknown or
/// unauthorized command grant" + `details.grant`)。
pub fn resolve_exec_grant(
    policy: &ResolvedExecPolicy,
    name: &str,
    fixed: bool,
) -> Result<ExecGrant, CheaperError> {
    match policy.grants.get(name) {
        Some(grant) if grant.fixed == fixed => Ok(grant.clone()),
        _ => {
            let mut details = Map::new();
            if fixed {
                details.insert("task".to_string(), json!(name));
                Err(CheaperError::new(
                    ErrorCode::PolicyViolation,
                    "unknown or unauthorized process task",
                )
                .with_details(details))
            } else {
                details.insert("grant".to_string(), json!(name));
                Err(CheaperError::new(
                    ErrorCode::PolicyViolation,
                    "unknown or unauthorized command grant",
                )
                .with_details(details))
            }
        }
    }
}

/// `exec_grant_resources` — `inputs`/`outputs` のいずれかを宣言
/// する grant(`Some([])` 空配列も宣言と見なす)は per-path
/// manifest(`process_task_resources`:340-363 同等: reads=inputs・
/// writes=outputs・produces=outputs・`**` 末尾 → Subtree。
/// `Some([])` は「何も claim しない」空 manifest を生成)。
/// `inputs`/`outputs` が両方 `None` の grant は conservative
/// root manifest(`command_grant_resources`:419-429 同等 —
/// 副作用を過大に見積もる fail-safe 側の宣言)。
pub fn exec_grant_resources(
    policy: &ResolvedExecPolicy,
    name: &str,
) -> Result<ResourceManifestDeclaration, CheaperError> {
    let grant = policy.grants.get(name).ok_or_else(|| {
        let mut details = Map::new();
        details.insert("grant".to_string(), json!(name));
        CheaperError::new(
            ErrorCode::PolicyViolation,
            "unknown or unauthorized exec grant",
        )
        .with_details(details)
    })?;
    if grant.inputs.is_none() && grant.outputs.is_none() {
        // conservative root manifest — root subtree の reads+writes、
        // `produces` は宣言しない(duplicate-producer 検出との衝突回避 +
        // ownership semantics 不充足)。
        let root = || ScopeDeclaration {
            path: ".".to_string(),
            scope: ResourceScope::Subtree,
        };
        return Ok(ResourceManifestDeclaration {
            reads: Some(vec![root()]),
            writes: Some(vec![root()]),
            produces: None,
        });
    }
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
    let inputs = grant.inputs.as_deref().unwrap_or(&[]);
    let outputs = grant.outputs.as_deref().unwrap_or(&[]);
    Ok(ResourceManifestDeclaration {
        reads: Some(scopes(inputs)),
        writes: Some(scopes(outputs)),
        produces: Some(scopes(outputs)),
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
    use std::collections::BTreeSet;

    use super::*;

    fn resolve(input: &Value) -> Result<ResolvedExecPolicy, CheaperError> {
        resolve_exec_policy(Some(input), &ExecDenyList::default())
    }

    fn merged(process: Value, command: Value) -> ResolvedExecPolicy {
        let doc =
            build_exec_input(Some(&process), Some(&command), &BTreeSet::new()).expect("synthesis");
        resolve_exec_policy(Some(&Value::Object(doc)), &ExecDenyList::default())
            .expect("exec resolves")
    }

    #[test]
    fn defaults_when_absent() {
        let p = resolve_exec_policy(None, &ExecDenyList::default()).unwrap();
        assert!(p.grants.is_empty());
        assert_eq!(p.command.max_concurrent, 1);
        assert_eq!(p.command.timeout, "2m");
        assert_eq!(p.process.max_concurrent, 1);
        let p = resolve_exec_policy(Some(&Value::Null), &ExecDenyList::default()).unwrap();
        assert!(p.grants.is_empty());
    }

    /// authoring section 形(`tasks`/`grants` を内包する doc)の direct
    /// 投入は `additionalProperties: false` が拒否 — caller は必ず
    /// `build_exec_input` 経由(段 1→2→3 の 3 段強制)。
    #[test]
    fn rejects_authoring_shaped_input() {
        for doc in [
            json!({"tasks": {"t": {"command": ["cc"], "inputs": [], "outputs": []}}}),
            json!({"command": {"grants": {"g": {"executable": "git"}}}}),
            json!({"process": {"tasks": {}}}),
            json!({"bogus": 1}),
        ] {
            let e = resolve(&doc).unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{doc}");
            assert!(e.message().contains("invalid exec policy"), "{doc}");
        }
        // defaults block の nested heartbeat も `additionalProperties:
        // false` で未知 key を拒否する(parity 不変条件)。
        let e =
            resolve(&json!({"command": {"heartbeat": {"interval": "1s", "x": 1}}})).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
    }

    /// direct exec-doc(resolve_exec_policy の入力 shape)の lifecycle
    /// fold — `fixed` → `process` defaults、`!fixed` → `command`
    /// defaults。
    #[test]
    fn lifecycle_fold_by_fixed_flag() {
        let p = resolve(&json!({
            "command": {"timeout": "9m", "heartbeat": {"interval": "7s"}},
            "process": {"timeout": "3m", "killGrace": "4s"},
            "grants": {
                "route": {"executable": "git"},
                "taskish": {"executable": "cc", "fixed": true},
            },
        }))
        .unwrap();
        let route = &p.grants["route"];
        assert!(!route.fixed);
        assert_eq!(route.timeout, "9m"); // command defaults
        assert_eq!(route.kill_grace, "2s"); // global default
        let taskish = &p.grants["taskish"];
        assert!(taskish.fixed);
        assert_eq!(taskish.timeout, "3m"); // process defaults
        assert_eq!(taskish.kill_grace, "4s");
        assert_eq!(p.command.heartbeat.interval, "7s");
        // `!fixed` grant の argv は pattern、`fixed` grant の argv は
        // literal — wildcard 構造検査は !fixed のみに効く。
        assert!(resolve(&json!({
            "grants": {"g": {"executable": "git", "argv": ["a*b"]}}
        }))
        .is_err());
        let p = resolve(&json!({
            "grants": {"g": {"executable": "git", "fixed": true, "argv": ["a*b"]}}
        }))
        .unwrap();
        assert_eq!(p.grants["g"].argv, Some(vec!["a*b".to_string()]));
    }

    /// 適用点 1 — declared basename deny が全 grant(command 由来・
    /// task 由来)へ一様に発火し `InvalidRuntimePolicy`。fixed grant は
    /// `details.task` を併記する。
    #[test]
    fn declared_deny_fires_for_both_surfaces() {
        let deny = ExecDenyList::compile(&["python.*".to_string()]).unwrap();
        // command surface grant。
        let e = resolve_exec_policy(
            Some(&json!({"grants": {"py": {"executable": "python3"}}})),
            &deny,
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert_eq!(e.details().unwrap()["grant"], json!("py"));
        assert_eq!(e.details().unwrap()["executable"], json!("python3"));
        assert!(e.details().unwrap().get("task").is_none());
        // fixed(task-derived)grant — `details.task` 併記。
        let e = resolve_exec_policy(
            Some(&json!({"grants": {"py": {"executable": "python3", "fixed": true}}})),
            &deny,
        )
        .unwrap_err();
        assert_eq!(e.details().unwrap()["task"], json!("py"));
        // 既定(空)deny は受理 — 旧 builtin deny 名も素通し。
        for exe in ["python", "python3", "ssh", "tar", "env"] {
            assert!(
                resolve(&json!({"grants": {"g": {"executable": exe}}})).is_ok(),
                "{exe}"
            );
        }
    }

    /// synthesis — task → `fixed:true` grant(`argv` = command[1..]
    /// literal、lifecycle = process section resolved defaults、
    /// `inputs`/`outputs` は task decl そのまま)。project task 名集合は
    /// `scope:"project"` provenance を畳む。
    #[test]
    fn synthesis_maps_tasks_to_fixed_grants() {
        let names: BTreeSet<String> = ["proj".to_string()].into_iter().collect();
        let doc = build_exec_input(
            Some(&json!({
                "timeout": "5m",
                "tasks": {
                    "build": {"command": ["cc", "-O2", "x.c"], "inputs": ["src/**"],
                              "outputs": ["out"], "cwd": "sub", "env": {"K": "v"}},
                    "proj": {"command": ["make"], "inputs": [], "outputs": []},
                },
            })),
            Some(&json!({"grants": {"g": {"executable": "git", "argv": ["log", "*"]}}})),
            &names,
        )
        .unwrap();
        let p = resolve_exec_policy(Some(&Value::Object(doc)), &ExecDenyList::default()).unwrap();
        assert_eq!(p.grants.len(), 3);
        let build = &p.grants["build"];
        assert!(build.fixed);
        assert_eq!(build.executable, "cc");
        assert_eq!(build.argv, Some(vec!["-O2".to_string(), "x.c".to_string()]));
        assert_eq!(build.timeout, "5m"); // process section default
        assert_eq!(build.exit_codes, vec![0]);
        assert_eq!(build.scope, ExecGrantScope::Trusted);
        assert_eq!(build.cwd, "sub");
        assert_eq!(build.inputs, Some(vec!["src/**".to_string()]));
        let proj = &p.grants["proj"];
        assert_eq!(proj.scope, ExecGrantScope::Project);
        // command grant は trusted + !fixed のまま。
        let g = &p.grants["g"];
        assert!(!g.fixed);
        assert_eq!(g.scope, ExecGrantScope::Trusted);
        // serialized → 再 resolve round-trip 冪等(parity 不変条件)。
        let serialized = serde_json::to_value(&p).unwrap();
        let reparsed = resolve_exec_policy(Some(&serialized), &ExecDenyList::default()).unwrap();
        assert_eq!(p, reparsed);
    }

    /// collision — `command.grants` × `process.tasks` の同名は
    /// `build_exec_input` で `InvalidRuntimePolicy` +
    /// `details.{grant,task}` fail-loud。
    #[test]
    fn synthesis_rejects_cross_section_name_collision() {
        let e = build_exec_input(
            Some(&json!({
                "tasks": {"shared": {"command": ["cc"], "inputs": [], "outputs": []}}
            })),
            Some(&json!({"grants": {"shared": {"executable": "git"}}})),
            &BTreeSet::new(),
        )
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy);
        assert!(e.message().contains("collides"));
        assert_eq!(e.details().unwrap()["grant"], json!("shared"));
        assert_eq!(e.details().unwrap()["task"], json!("shared"));
        // 別名なら受理。
        assert!(build_exec_input(
            Some(&json!({
                "tasks": {"t": {"command": ["cc"], "inputs": [], "outputs": []}}
            })),
            Some(&json!({"grants": {"g": {"executable": "git"}}})),
            &BTreeSet::new(),
        )
        .is_ok());
    }

    /// surface disjoint — `resolve_exec_grant` は `fixed` 期待値と
    /// `grant.fixed` が一致しない grant を surface 固有の unknown
    /// error で拒否する(fixed task を `name` で呼ぶ・その逆、共に
    /// fail-closed)。
    #[test]
    fn resolve_exec_grant_enforces_disjoint_surfaces() {
        let p = merged(
            json!({"tasks": {"t": {"command": ["cc"], "inputs": [], "outputs": []}}}),
            json!({"grants": {"g": {"executable": "git"}}}),
        );
        // fixed surface: task hit、command grant は task 語彙で
        // unknown。
        assert!(resolve_exec_grant(&p, "t", true).is_ok());
        let e = resolve_exec_grant(&p, "g", true).unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
        assert!(e.message().contains("process task"));
        assert_eq!(e.details().unwrap()["task"], json!("g"));
        // !fixed surface: grant hit、task は grant 語彙で unknown。
        assert!(resolve_exec_grant(&p, "g", false).is_ok());
        let e = resolve_exec_grant(&p, "t", false).unwrap_err();
        assert!(e.message().contains("command grant"));
        assert_eq!(e.details().unwrap()["grant"], json!("t"));
        let e = resolve_exec_grant(&p, "nope", false).unwrap_err();
        assert_eq!(e.details().unwrap()["grant"], json!("nope"));
    }

    /// manifest tri-state — task-derived grant は per-path manifest、
    /// `inputs`/`outputs` 未宣言の command grant は conservative root
    /// manifest。
    #[test]
    fn exec_grant_resources_tri_state() {
        let p = merged(
            json!({"tasks": {
                "ro": {"command": ["cc"], "inputs": ["src/**"], "outputs": []},
            }}),
            json!({"grants": {
                "conservative": {"executable": "git"},
                "declared": {"executable": "cc", "outputs": ["out"]},
            }}),
        );
        let m = exec_grant_resources(&p, "ro").unwrap();
        assert_eq!(m.reads.as_ref().unwrap()[0].path, "src/**");
        assert_eq!(m.reads.as_ref().unwrap()[0].scope, ResourceScope::Subtree);
        assert_eq!(m.writes.as_ref().unwrap().len(), 0);
        // root manifest — subtree reads+writes、produces なし。
        let m = exec_grant_resources(&p, "conservative").unwrap();
        assert_eq!(m.reads.as_ref().unwrap()[0].path, ".");
        assert_eq!(m.reads.as_ref().unwrap()[0].scope, ResourceScope::Subtree);
        assert!(m.produces.is_none());
        // `outputs` だけ宣言した grant も per-path manifest。
        let m = exec_grant_resources(&p, "declared").unwrap();
        assert_eq!(m.writes.as_ref().unwrap()[0].path, "out");
        assert_eq!(m.reads.as_ref().unwrap().len(), 0);
        let e = exec_grant_resources(&p, "nope").unwrap_err();
        assert_eq!(e.code(), ErrorCode::PolicyViolation);
    }

    /// route eligibility — `Trusted && !fixed` のみ eligible。
    #[test]
    fn route_eligibility_and_project_grants() {
        let p = merged(
            json!({"tasks": {"t": {"command": ["cc"], "inputs": [], "outputs": []}}}),
            json!({"grants": {
                "route": {"executable": "git"},
                "member": {"executable": "fmt"},
                "scoped": {"executable": "x", "scope": "project"},
            }}),
        );
        assert!(ResolvedExecPolicy::is_route_eligible(&p.grants["route"]));
        assert!(ResolvedExecPolicy::is_route_eligible(&p.grants["member"]));
        assert!(!ResolvedExecPolicy::is_route_eligible(&p.grants["t"]));
        assert!(!ResolvedExecPolicy::is_route_eligible(&p.grants["scoped"]));
        // `project_grants` は `{route.grant} ∪ grants ∪ {fixed
        // grants}` を残す(#166 — task 経路は投影で失われない)。
        let mut p2 = p.clone();
        let names = BTreeSet::from(["route".to_string(), "member".to_string()]);
        assert!(p2.project_grants(&names));
        assert_eq!(p2.grants.len(), 3);
        assert!(p2.grants.contains_key("route"));
        assert!(p2.grants.contains_key("member"));
        assert!(p2.grants.contains_key("t")); // fixed は残る
        assert!(!p2.grants.contains_key("scoped"));
        // set 内 1 件でも ineligible/不在なら false で policy 不変。
        let mut p = p;
        assert!(!p.project_grants(&BTreeSet::from(["route".to_string(), "t".to_string(),])));
        assert!(!p.project_grants(&BTreeSet::from(
            ["route".to_string(), "scoped".to_string(),]
        )));
        assert!(!p.project_grants(&BTreeSet::from(
            ["route".to_string(), "absent".to_string(),]
        )));
        // 空集合も false(全 `!fixed` grant が落ちる fail-safe)。
        assert!(!p.project_grants(&BTreeSet::new()));
        assert_eq!(p.grants.len(), 4, "policy 不変");
    }

    /// `ArgPattern` の compile/match(command.rs から移設 — `!fixed`
    /// grant の argv pattern 構造検査)。
    #[test]
    fn argv_pattern_compile_and_match() {
        let p = ArgPattern::compile(&["log".to_string()]).unwrap();
        assert!(p.matches(&["log".to_string()]));
        assert!(!p.matches(&["log".to_string(), "--oneline".to_string()]));
        assert!(!p.matches(&[]));
        let p = ArgPattern::compile(&["log".to_string(), "*".to_string()]).unwrap();
        assert!(p.matches(&["log".to_string()]));
        assert!(p.matches(&["log".to_string(), "--oneline".to_string()]));
        assert!(!p.matches(&[]));
        let p = ArgPattern::compile(&["*".to_string()]).unwrap();
        assert!(p.matches(&[]));
        assert!(p.matches(&["a".to_string(), "b".to_string()]));
        let p = ArgPattern::compile(&["log".to_string(), "--format=*".to_string()]).unwrap();
        assert!(p.matches(&["log".to_string(), "--format=%H".to_string()]));
        assert!(!p.matches(&["log".to_string(), "--format".to_string()]));
        for bad in ["a*b", "**", "*a"] {
            assert!(ArgPattern::compile(&[bad.to_string()]).is_err(), "{bad}");
        }
        // 最終要素以外の bare `*` は拒否。
        assert!(ArgPattern::compile(&["*".to_string(), "log".to_string()]).is_err());
        // metachar を含む token は literal として受理される。
        for ok in ["a|b", "x;y", "v$(i)"] {
            assert!(ArgPattern::compile(&[ok.to_string()]).is_ok(), "{ok}");
        }
    }

    /// `bounded_duration`/`relative`/`str_vec` の移設先 unit
    /// 担保 — section label で error prefix が分岐する。
    #[test]
    fn shared_helpers_carry_section_label() {
        assert!(bounded_duration("1s", "timeout", "exec").is_ok());
        let e = bounded_duration("100000000000000000000h", "timeout", "exec").unwrap_err();
        assert!(e.message().contains("invalid exec policy"));
        let e = relative("../x", "command").unwrap_err();
        assert!(e.message().contains("invalid command policy"));
        assert_eq!(str_vec(&json!(["a", 1, "b"])), vec!["a", "b"]);
    }

    /// exec-doc の env/cwd/path 規則 — section 規則と同一の検査が
    /// `invalid exec policy` prefix で発火する。
    #[test]
    fn exec_doc_semantic_rules() {
        for doc in [
            json!({"grants": {"g": {"executable": "git", "cwd": "../x"}}}),
            json!({"grants": {"g": {"executable": "git", "inputs": ["a/../b"]}}}),
            json!({"grants": {"g": {"executable": "git", "env": {"K": "a\0b"}}}}),
            json!({"grants": {"g": {"executable": "git", "timeout": "5x"}}}),
            json!({"command": {"timeout": "5x"}}),
        ] {
            let e = resolve(&doc).unwrap_err();
            assert_eq!(e.code(), ErrorCode::InvalidRuntimePolicy, "{doc}");
            assert!(e.message().contains("invalid exec policy"), "{doc}");
        }
    }
}
