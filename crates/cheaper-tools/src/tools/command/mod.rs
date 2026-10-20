//! `command` canonical tool の実装(policy-gated command tool design
//! §(b)/(e)/(f)/(g))。T1 の deny-stub(`POLICY_VIOLATION
//! "command unavailable"`)を grant 経由実行の実装へ差し替えた。
//!
//! model 入力は `{name, args}` のみ — executable/cwd/env は trusted
//! policy の grant に由来し、model 入力経路には一切乗らない。
//! execute glue は grant 解決 → argv pattern match(metachar 事前
//! check)→ cwd prepare → pin 済み executable の spawn 前再検証 →
//! shared `crate::tools::proc::run` の順。pin 自体は registry の
//! `create_tool_runtime` が `names` に `Command` を含む場合のみ行う
//! (per-scope、design §(b)/(g))— 本 module の [`pin_command_grants`]
//! を呼ぶ。
//!
//! `process` tool との差分: grant(固定 task でなく動的 argv +
//! pattern)・details `{grant, argv, ..., durationMs}`・独自 pool
//! `"command"`・Exclusive root-subtree claim・writeScope 非両立
//! (registry の registration gate で拒否)。

use std::collections::BTreeMap;
use std::sync::Arc;

use cheaper_core::duration::parse_duration;
use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::policy::command::{
    is_denied_command_executable, is_relative_executable_path, resolve_command_grant, ArgPattern,
    CommandGrant, ResolvedCommandPolicy,
};
use cheaper_core::policy::process::{argv_shell_re, MAX_PROCESS_OUTPUT_BYTES};
use cheaper_core::resources::types::ClaimMode;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::{json, Map, Value};

use crate::catalog::CanonicalToolResult;
use crate::operation::AbortSignal;
use crate::scheduler::{OperationDeclaration, OperationPool};
use crate::tools::proc::executable::{
    native_executable, resolve_process_executable, ProcessExecutableOptions,
};
use crate::tools::proc::{
    aborted_err, minimal_env, prepare_cwd, prepare_exec_cwd, OnTimeout, ProcIdentity, ProcLimits,
    ProcPrepared, ProcTee,
};
use crate::ToolError;

/// `commandSchema` — model input は `{name, args}` のみ。`executable`/
/// `cwd`/`env` 等の余剰 key は `additionalProperties: false` で schema
/// 層の `INVALID_INPUT` に落ち、model 入力経路に乗らない(design §(i))。
/// `required: ["name", "args"]` — `args` 省略不可で model に argv の
/// 明示を強制する。
pub fn command_schema() -> Value {
    json!({
        "type": "object",
        "required": ["name", "args"],
        "properties": {
            "name": {
                "type": "string",
                "pattern": "^[a-z][a-z0-9_-]{0,63}$",
                "description": "Name of a policy-authorized command grant",
            },
            "args": {
                "type": "array",
                "maxItems": 256,
                "items": {"type": "string", "maxLength": 4096},
                "description": "Arguments matched against the grant argv pattern",
            },
        },
        "additionalProperties": false,
    })
}

fn stream_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "text": {"type": "string", "maxLength": MAX_PROCESS_OUTPUT_BYTES},
            "bytes": {"type": "integer", "minimum": 0},
            "capturedBytes": {"type": "integer", "minimum": 0, "maximum": MAX_PROCESS_OUTPUT_BYTES},
            "truncated": {"type": "boolean"},
        },
        "required": ["text", "bytes", "capturedBytes", "truncated"],
        "additionalProperties": false,
    })
}

/// `commandDetailsSchema` — `{grant, argv, state, exitCode, signal,
/// durationMs, stdout, stderr}`。`durationMs`/`argv` は spec が要求する
/// duration/argv 報告で、process details shape を意図的に超過する
/// spec-mandated な差分(design §(e))。
pub fn command_details_schema() -> Value {
    let stream = stream_schema();
    json!({
        "type": "object",
        "properties": {
            "grant": {"type": "string"},
            "argv": {"type": "array", "items": {"type": "string"}},
            "state": {"enum": ["completed", "failed"]},
            "exitCode": {"type": ["integer", "null"]},
            "signal": {"type": ["string", "null"]},
            "durationMs": {"type": "integer", "minimum": 0},
            "stdout": stream,
            "stderr": stream,
            "logRef": {"type": "string"},
            "outputTokens": {"type": "integer", "minimum": 0},
            "outputTokensTruncated": {"type": "boolean"},
        },
        "required": [
            "grant", "argv", "state", "exitCode", "signal", "durationMs",
            "stdout", "stderr",
        ],
        "additionalProperties": false,
    })
}

// ---------------------------------------------------------------------------
// operation declaration
// ---------------------------------------------------------------------------

/// `commandOperationDeclaration`: grant 解決 → Exclusive access + 独自
/// pool `"command"`(`command.maxConcurrent` cap、process pool と独立 —
/// design §(f))+ `command_grant_resources` の保守的 manifest
/// (workspace root subtree の reads+writes、`produces` なし)。
/// 未知 grant は `resolve_command_grant` の `POLICY_VIOLATION` で
/// fail-closed する(schedule 前の op declaration 段階で拒否)。
pub fn command_operation_declaration(
    policy: &ResolvedCommandPolicy,
    grant: &str,
) -> Result<OperationDeclaration, CheaperError> {
    resolve_command_grant(policy, grant)?;
    let manifest = cheaper_core::policy::command::command_grant_resources();
    let scopes = |items: &Option<Vec<cheaper_core::resources::normalization::ScopeDeclaration>>| {
        serde_json::to_value(items).expect("ScopeDeclaration は必ず serialize 可能")
    };
    let mut declaration = OperationDeclaration::new(ClaimMode::Exclusive);
    declaration.pool = Some(OperationPool {
        name: "command".to_string(),
        max_concurrent: policy.max_concurrent as i64,
    });
    declaration.resources = Some(json!({
        "reads": scopes(&manifest.reads),
        "writes": scopes(&manifest.writes),
    }));
    Ok(declaration)
}

// ---------------------------------------------------------------------------
// pin / spawn-time re-verification(design §(b))
// ---------------------------------------------------------------------------

/// grant 文脈の `POLICY_VIOLATION` — `resolve_command_grant` の
/// `{grant: name}` details 規約に揃える。
fn grant_violation(name: &str, reason: &str) -> ToolError {
    let mut details = Map::new();
    details.insert("grant".to_string(), json!(name));
    CheaperError::new(
        ErrorCode::PolicyViolation,
        format!("command grant \"{name}\": {reason}"),
    )
    .with_details(details)
    .into()
}

/// pin 済み canonical path が workspace root 内かの判定。
/// `root`・`resolved` は共に realpath 済み canonical path 前提 —
/// `nodepath::relative` が `..` segment を含むか absolute なら root 外。
fn within_workspace(root: &str, resolved: &str) -> bool {
    let rel = nodepath::relative(root, resolved);
    !nodepath::is_absolute(&rel) && !rel.split(['/', '\\']).any(|s| s == "..")
}

/// `create_tool_runtime` 境界の per-scope pin(design §(b)/(g))。
/// `names` に `Command` を含む runtime 構築時のみ registry から呼ばれ、
/// 各 grant の declared executable を ambient PATH で解決して realpath
/// 済み canonical path へ pin する。workspace-resident executable は
/// 二段で拒否: (a) absolute でない separator 付き declared path は
/// `resolve_process_executable` の workspace-relative 解決へ着地する
/// ため委譲前に `POLICY_VIOLATION`、(b) `native()` realpath 後の
/// canonical path が workspace root 内なら同じく `POLICY_VIOLATION`
/// (absolute path / PATH 上の symlink が workspace 内へ着地する迂回
/// を塞ぐ)。pin 不能・deny 該当 grant も `POLICY_VIOLATION` で runtime
/// 構築自体を fail させる。
pub(crate) fn pin_command_grants(
    policy: &ResolvedCommandPolicy,
    root: &str,
    symlink_policy: SymlinkPolicy,
    denylist: Option<&PathDenylist>,
) -> Result<BTreeMap<String, String>, ToolError> {
    let mut pins = BTreeMap::new();
    for (name, grant) in &policy.grants {
        let resolved = pin_command_grant(name, grant, root, symlink_policy, denylist)?;
        pins.insert(name.clone(), resolved);
    }
    Ok(pins)
}

/// `pin_command_grants` の単一 grant 版 — `route::resolve` の
/// viability 事前評価(`check_command_execution`)が scope pin 未
/// 構築の状態でも同一 predicate を使うための共有 seam。
/// workspace-resident executable は二段で拒否: (a) absolute でない
/// separator 付き declared path は `resolve_process_executable` の
/// workspace-relative 解決へ着地するため委譲前に
/// `POLICY_VIOLATION`、(b) `native()` realpath 後の canonical path
/// が workspace root 内なら同じく `POLICY_VIOLATION`。
/// pin 不能・deny 該当 grant は `POLICY_VIOLATION` で返す。
pub fn pin_command_grant(
    name: &str,
    grant: &CommandGrant,
    root: &str,
    symlink_policy: SymlinkPolicy,
    denylist: Option<&PathDenylist>,
) -> Result<String, ToolError> {
    // (a) workspace-relative declared executable の pre-filter。
    if is_relative_executable_path(&grant.executable) {
        return Err(grant_violation(
            name,
            "workspace-resident executables are not grantable",
        ));
    }
    let resolved = resolve_process_executable(
        &grant.executable,
        &ProcessExecutableOptions {
            root,
            symlink_policy,
            denylist,
            search_path: None,
        },
        "command grant",
    )?;
    // resolved basename の deny 再検査(design §(d) 適用点 2 —
    // alias/symlink が denied executable へ解決する迂回を塞ぐ)。
    if is_denied_command_executable(&resolved) {
        return Err(grant_violation(
            name,
            "resolved executable is interpreter or exec-capable",
        ));
    }
    // (b) realpath 後の workspace 内 canonical path を拒否。
    if within_workspace(root, &resolved) {
        return Err(grant_violation(
            name,
            "workspace-resident executables are not grantable",
        ));
    }
    Ok(resolved)
}

/// pin 済み executable の spawn 前再検証(design §(b)) — pin 後の
/// file 差替えに対する fail-closed。pin 時と同じ candidate 検証
/// (realpath → shell/shim → is_file → ext → X_OK → native magic)を
/// 再適用し、deny basename・workspace residency を再評価する。
/// pin 済み path は canonical のため、realpath が変わっていた場合は
/// symlink 差替えとして reject する。
fn verify_pinned_executable(name: &str, pinned: &str, root: &str) -> Result<(), ToolError> {
    let Some(resolved) = native_executable(pinned) else {
        return Err(grant_violation(
            name,
            "pinned executable no longer resolves to a supported native executable",
        ));
    };
    if resolved != pinned {
        return Err(grant_violation(
            name,
            "pinned executable path changed since pin",
        ));
    }
    if is_denied_command_executable(&resolved) {
        return Err(grant_violation(
            name,
            "resolved executable is interpreter or exec-capable",
        ));
    }
    if within_workspace(root, &resolved) {
        return Err(grant_violation(
            name,
            "workspace-resident executables are not grantable",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// tool surface
// ---------------------------------------------------------------------------

/// `Options`(TS `interface Options` 相当)。`pins` は
/// `create_tool_runtime` の per-scope pin map(grant name → canonical
/// path)をそのまま受ける — model/scope をまたぐ cache はない。
pub struct CommandToolOptions {
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    pub command: ResolvedCommandPolicy,
    pub pins: BTreeMap<String, String>,
    /// `onTimeout(reason)`。
    pub on_timeout: Option<OnTimeout>,
    pub denylist: Option<PathDenylist>,
    /// route scope 構築層のみが注入する route context(design
    /// §route context 注入)。`None` で従来挙動不変。
    pub route_context: Option<RouteCommandContext>,
}

/// route scope の command tool 注入 context(design §route context
/// 注入、R18/R26/C2)。
#[derive(Clone, Default)]
pub struct RouteCommandContext {
    /// `env-prefix` + ambient allowlist 合成の子 env 上乗せ分。
    /// 優先順位 `env-prefix > ambient` の合成は注入側で済み、
    /// tool は `grant.env` への override として畳むのみ
    /// (= extra_env > grant.env)。
    pub extra_env: Map<String, Value>,
    /// payload `cwd` の子 process cwd override(R18)— grant.cwd
    /// 経路を bypass し、workspace containment は課さない
    /// (存在確認 + denylist 検査のみ)。
    pub exec_cwd: Option<String>,
    /// 任意の completed exit を result 化(exit-code mirror、C2)—
    /// `accepted_exit_codes: None` へ投影する。spawn 失敗・timeout・
    /// abort は従来どおり Err。
    pub accept_any_exit_code: bool,
    /// 全量 output tee sink(R26)— tee 内部の write 失敗は sink が
    /// 自身へ記録し capture を失敗させない(postprocessFailure
    /// 縮退、R29)。
    pub log_tee: Option<ProcTee>,
}

/// `createCommandTool` が返す canonical definition。
pub struct CommandTool {
    description: String,
    options: CommandToolOptions,
}

/// `createCommandTool`。description に authorized grant 名一覧を
/// 埋め込み、model が grant 名を discovery できるようにする
/// (`create_process_tool` の task 名一覧先例)。
pub fn create_command_tool(options: CommandToolOptions) -> CommandTool {
    let grants: Vec<&str> = options.command.grants.keys().map(String::as_str).collect();
    CommandTool {
        description: format!(
            "Run a policy-authorized command grant: {}. Only granted executables with matching argv patterns run.",
            grants.join(", ")
        ),
        options,
    }
}

impl CommandTool {
    pub fn name(&self) -> &'static str {
        "command"
    }

    pub fn label(&self) -> &'static str {
        "Command"
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    /// `parameters` JSON schema(`validateToolInput` 接合対象)。
    pub fn parameters(&self) -> Value {
        command_schema()
    }

    /// `execute(input, signal)`。`input.name`/`input.args` の schema
    /// 検証は catalog 側責務(TS 同様)。`signal` は
    /// `signal ?? new AbortController().signal`。
    pub async fn execute(
        &self,
        input: &Value,
        signal: Option<AbortSignal>,
    ) -> Result<CanonicalToolResult, ToolError> {
        execute(
            &self.options,
            input,
            signal.unwrap_or_else(AbortSignal::detached),
        )
        .await
    }
}

/// `createCommandTool` の `CanonicalToolDefinition` ラッパ。
/// `process_tool_definition` と同じく tool は `Arc` 共有。
pub fn command_tool_definition(
    options: CommandToolOptions,
) -> crate::definition::CanonicalToolDefinition<'static> {
    let tool = Arc::new(create_command_tool(options));
    crate::definition::define_canonical_tool(crate::definition::CanonicalToolDefinition {
        name: cheaper_core::tools::vocabulary::ToolName::Command,
        label: tool.label().to_string(),
        description: tool.description().to_string(),
        parameters: command_schema().as_object().expect("object schema").clone(),
        prepare_arguments: None,
        execute: Box::new(move |input, signal| {
            let tool = Arc::clone(&tool);
            Box::pin(async move { tool.execute(&input, signal).await })
        }),
    })
}

// ---------------------------------------------------------------------------
// execute(grant 解決 → argv match → prepare → pin 再検証 → proc::run)
// ---------------------------------------------------------------------------

/// grant argv pattern match + metachar 検査。metachar check は全
/// token(pattern 適合・rest 残余を問わず)に match 前の独立 check
/// として適用する(design §(a))。pattern 不一致・metachar 含有は
/// `POLICY_VIOLATION`。
/// `check_command_execution` の検証済み出力 — spawn に必要な
/// grant・cwd・executable を保持する(predicate 結果の共有
/// transport。`command::execute` はこの 3 値をそのまま使い、
/// `route::resolve` は viability の成否のみ見る)。
pub struct CheckedCommand {
    pub grant: CommandGrant,
    pub cwd: String,
    /// 検証済み executable — scope pin があれば pin 済み path の
    /// 再検証結果、無ければ declared executable のその場解決(pin
    /// と同一 predicate: `pin_command_grant`)。
    pub executable: String,
}

/// spawn 無しの事前 check(design §viability — B#3 共有 seam)。
/// `command::execute` が実 spawn 前に課す全 gate を同一 predicate
/// で評価する: grant 解決 → argv pattern/metachar → cwd
/// (route context の `exec_cwd` override を含む)→ executable
/// 検証(pin 済み path の spawn 前再検証、または pin 未構築時は
/// declared executable のその場 pin 解決 — `route::resolve` の
/// viability 経路)。predicate の複製による drift を防ぐため
/// `command::execute` もこの entry を呼ぶ。
pub fn check_command_execution(
    options: &CommandToolOptions,
    name: &str,
    args: &[String],
) -> Result<CheckedCommand, ToolError> {
    let grant =
        resolve_command_grant(&options.command, name).map_err(|e| Box::new(e) as ToolError)?;
    authorize_args(name, &grant, args)?;
    let route_ctx = options.route_context.as_ref();
    let cwd = match route_ctx.and_then(|r| r.exec_cwd.as_deref()) {
        Some(exec_cwd) => prepare_exec_cwd(exec_cwd, options.denylist.as_ref())?,
        None => prepare_cwd(
            &options.root,
            &grant.cwd,
            options.symlink_policy,
            options.denylist.as_ref(),
            "command",
        )?,
    };
    // executable 事前検証 — pin 済み path がある scope(registry
    // 構築経路)は spawn 前再検証、pin 未構築の dry-run 経路
    // (`route::resolve` viability)は declared executable をその場
    // 解決する。いずれも deny/residency の同一 predicate。
    let executable = match options.pins.get(name) {
        Some(pinned) => {
            verify_pinned_executable(name, pinned, &options.root)?;
            pinned.clone()
        }
        None => pin_command_grant(
            name,
            &grant,
            &options.root,
            options.symlink_policy,
            options.denylist.as_ref(),
        )?,
    };
    Ok(CheckedCommand {
        grant,
        cwd,
        executable,
    })
}

fn authorize_args(name: &str, grant: &CommandGrant, args: &[String]) -> Result<(), ToolError> {
    for token in args {
        if argv_shell_re().is_match(token) {
            return Err(grant_violation(
                name,
                "argument contains forbidden shell metacharacters",
            ));
        }
    }
    if let Some(pattern) = &grant.argv {
        // pattern は policy resolve で compile 検証済み — resolved
        // grant が source string を保持するため match 時に再 compile
        // する(design §(a))。compile 失敗は到達不能の defense。
        let pattern = ArgPattern::compile(pattern).map_err(|e| Box::new(e) as ToolError)?;
        if !pattern.matches(args) {
            return Err(grant_violation(
                name,
                "arguments do not match the grant argv pattern",
            ));
        }
    }
    Ok(())
}

async fn execute(
    options: &CommandToolOptions,
    input: &Value,
    outer: AbortSignal,
) -> Result<CanonicalToolResult, ToolError> {
    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }
    let name = input
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    // schema 検証済みのため全要素 string — 非 string は捨てて
    // pattern match 側の個数不一致で fail-closed に落ちる。
    let args: Vec<String> = input
        .get("args")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    // 事前 check は `check_command_execution` へ集約(design
    // §viability — `route::resolve` と同一 predicate)。pin map に
    // 無い direct 構築は declared executable のその場解決へ倒れ、
    // deny/residency の検証は共通のまま。
    let checked = check_command_execution(options, &name, &args)?;
    let grant = checked.grant;
    let cwd = checked.cwd;
    let pinned = checked.executable;
    let route_ctx = options.route_context.as_ref();
    if let Some(reason) = outer.reason() {
        return Err(aborted_err(reason));
    }

    // per-grant override 畳み込み済みの effective limit を
    // `ProcLimits` へ流す(`parse_duration` は policy resolve で
    // 検証済みのため到達不能想定)。heartbeat は section-level のみ。
    let limits = ProcLimits {
        timeout_ms: parse_duration(&grant.timeout)?,
        kill_grace_ms: parse_duration(&grant.kill_grace)?,
        max_output_bytes: grant.max_output_bytes,
        heartbeat_interval_ms: parse_duration(&options.command.heartbeat.interval)?,
        // route context の `accept_any_exit_code` は declared grant の
        // `exitCodes` を上書きして任意の completed exit を result 化
        // する(C2 — routed scope でのみ注入される)。
        accepted_exit_codes: if route_ctx.is_some_and(|r| r.accept_any_exit_code) {
            None
        } else {
            Some(grant.exit_codes.clone())
        },
    };
    // `extra_env` は `grant.env` への override — 合成済み map を
    // `minimal_env` へ渡し正規化を一箇所へ集約する。
    let mut declared_env = grant.env.clone();
    if let Some(extra) = route_ctx.map(|r| &r.extra_env) {
        for (key, value) in extra {
            declared_env.insert(key.clone(), value.clone());
        }
    }
    let prepared = ProcPrepared {
        executable: pinned.clone(),
        argv: args.clone(),
        cwd,
        env: minimal_env(&declared_env),
    };
    // `command` の identity: details `{grant: name, argv: args}`・
    // display `"command {name}"`・diagnostics event prefix `"command"`。
    // `durationMs` は details schema required のため identity flag で
    // proc 側 details 組み立てに載せる。
    let identity = ProcIdentity {
        details: vec![
            ("grant".to_string(), json!(name)),
            ("argv".to_string(), json!(args)),
        ],
        label: name.clone(),
        display: format!("command {name}"),
        event_prefix: "command",
        report_duration_ms: true,
    };
    crate::tools::proc::run(
        &prepared,
        &limits,
        &identity,
        outer,
        options.on_timeout.as_ref(),
        route_ctx.and_then(|r| r.log_tee.clone()),
    )
    .await
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use cheaper_core::policy::command::resolve_command_policy;
    use cheaper_core::workspace::paths::{canonicalize_workspace_root, realpath};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!(
                "cheaper-cmd-{}-{}",
                std::process::id(),
                fastrand::u64(..)
            ));
            std::fs::create_dir_all(&base).unwrap();
            Self(base)
        }

        /// Canonical workspace root(dunce realpath 済み)。
        fn root(&self) -> String {
            canonicalize_workspace_root(self.0.to_str().unwrap()).unwrap()
        }

        fn join(&self, p: &str) -> PathBuf {
            self.0.join(p)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write(path: &Path, bytes: &[u8]) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(bytes).unwrap();
    }

    fn native_bytes() -> Vec<u8> {
        if cfg!(windows) {
            b"MZbinary".to_vec()
        } else {
            vec![0x7f, 0x45, 0x4c, 0x46]
        }
    }

    fn set_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let _ = path;
    }

    fn policy(command: Value) -> ResolvedCommandPolicy {
        resolve_command_policy(Some(&command)).unwrap()
    }

    fn policy_violation(err: &ToolError) -> bool {
        crate::tools::proc::find_cheaper_error(err.as_ref() as &dyn std::error::Error)
            .is_some_and(|e| e.code() == ErrorCode::PolicyViolation)
    }

    fn grant<'a>(policy: &'a ResolvedCommandPolicy, name: &str) -> &'a CommandGrant {
        policy.grants.get(name).expect("grant")
    }

    #[test]
    fn description_embeds_authorized_grant_names() {
        let policy = policy(json!({
            "grants": {
                "git-read": {"executable": "git"},
                "cargo-build": {"executable": "cargo"},
            }
        }));
        let tool = create_command_tool(CommandToolOptions {
            root: ".".to_string(),
            symlink_policy: SymlinkPolicy::Deny,
            command: policy,
            pins: BTreeMap::new(),
            on_timeout: None,
            denylist: None,
            route_context: None,
        });
        assert!(tool.description().contains("cargo-build"));
        assert!(tool.description().contains("git-read"));
    }

    #[test]
    fn operation_declaration_is_exclusive_with_own_command_pool() {
        let policy = policy(json!({
            "maxConcurrent": 3,
            "grants": {"git-read": {"executable": "git"}}
        }));
        let declaration = command_operation_declaration(&policy, "git-read").unwrap();
        assert_eq!(declaration.access, ClaimMode::Exclusive);
        let pool = declaration.pool.unwrap();
        assert_eq!(pool.name, "command");
        assert_eq!(pool.max_concurrent, 3);
        // 保守的 manifest: root subtree reads+writes、produces は
        // 宣言しない(design §(f))。
        let resources = declaration.resources.unwrap();
        assert_eq!(
            resources,
            json!({
                "reads": [{"path": ".", "scope": "subtree"}],
                "writes": [{"path": ".", "scope": "subtree"}],
            })
        );
    }

    #[test]
    fn operation_declaration_rejects_unknown_grant() {
        let policy = policy(json!({"grants": {"git-read": {"executable": "git"}}}));
        let err = command_operation_declaration(&policy, "missing").unwrap_err();
        assert_eq!(err.code(), ErrorCode::PolicyViolation);
    }

    #[test]
    fn authorize_args_rejects_metachar_before_pattern_match() {
        let policy = policy(json!({
            "grants": {"git-read": {"executable": "git", "argv": ["log", "*"]}}
        }));
        let grant = grant(&policy, "git-read");
        // pattern 適合でも metachar token は match 前の独立 check で
        // `POLICY_VIOLATION`。
        for token in ["a|b", "x;y", "$(id)", "`id`", "a>b", "line\nfeed"] {
            let err = authorize_args("git-read", grant, &["log".to_string(), token.to_string()])
                .unwrap_err();
            assert!(policy_violation(&err), "{token}");
        }
        // pattern 不一致(超過・不足・別 token)は `POLICY_VIOLATION`。
        for args in [
            vec!["diff".to_string()],
            vec![],
            // trailing `*` が無い pattern ではないので個数無制約では
            // ないが、`log` 前置 token が違えば reject。
            vec!["branch".to_string(), "x".to_string()],
        ] {
            let err = authorize_args("git-read", grant, &args).unwrap_err();
            assert!(policy_violation(&err), "{args:?}");
        }
        // rest marker の残余 token は metachar だけが制約。
        authorize_args(
            "git-read",
            grant,
            &[
                "log".to_string(),
                "--oneline".to_string(),
                "HEAD~3".to_string(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn authorize_args_without_pattern_accepts_any_count() {
        let policy = policy(json!({"grants": {"g": {"executable": "git"}}}));
        let grant = grant(&policy, "g");
        authorize_args("g", grant, &[]).unwrap();
        authorize_args("g", grant, &["status".to_string(), "--short".to_string()]).unwrap();
        // metachar は argv 省略時も拒否。
        let err = authorize_args("g", grant, &["a;b".to_string()]).unwrap_err();
        assert!(policy_violation(&err));
    }

    #[test]
    fn pin_rejects_relative_executable_path_before_resolution() {
        let dir = TempDir::new();
        let root = dir.root();
        let policy = policy(json!({
            "grants": {"g": {"executable": "bin/tool"}}
        }));
        let err = pin_command_grants(&policy, &root, SymlinkPolicy::Deny, None).unwrap_err();
        assert!(policy_violation(&err));
        assert!(err.to_string().contains("workspace-resident"), "{err}");
    }

    #[test]
    fn pin_rejects_absolute_path_inside_workspace() {
        let dir = TempDir::new();
        let root = dir.root();
        // workspace 内に native binary を置き、absolute path grant が
        // realpath 後の inside-workspace 検査で拒否されることを確認。
        let bin = dir.join(if cfg!(windows) { "tool.exe" } else { "tool" });
        write(&bin, &native_bytes());
        set_executable(&bin);
        let policy = policy(json!({
            "grants": {"g": {"executable": bin.to_str().unwrap()}}
        }));
        let err = pin_command_grants(&policy, &root, SymlinkPolicy::Deny, None).unwrap_err();
        assert!(policy_violation(&err));
        assert!(err.to_string().contains("workspace-resident"), "{err}");
    }

    #[test]
    fn pin_fails_closed_on_unresolvable_or_denied_executable() {
        let dir = TempDir::new();
        let root = dir.root();
        // ambient PATH で解決不能 → resolve_process_executable の
        // unsupported `POLICY_VIOLATION`。
        let policy = policy(json!({
            "grants": {"g": {"executable": "definitely-missing-tool-xyz"}}
        }));
        let err = pin_command_grants(&policy, &root, SymlinkPolicy::Deny, None).unwrap_err();
        assert!(policy_violation(&err));
        // denied executable は policy resolve 段階で拒否されるため、
        // ここでは resolved basename 再検査の対象外 — shell/shim だけ
        // 同じく resolve で拒否されることを確認(design §(d) 適用点 1)。
        assert!(resolve_command_policy(Some(&json!({
            "grants": {"g": {"executable": "python3"}}
        })))
        .is_err());
    }

    #[test]
    fn pin_maps_grant_to_resolved_canonical_path() {
        let dir = TempDir::new();
        let root = dir.root();
        // `current_exe` は workspace 外の native binary — absolute
        // path grant として pin できる。
        let exe = std::env::current_exe().unwrap();
        let exe = exe.to_str().unwrap();
        let policy = policy(json!({
            "grants": {"self": {"executable": exe}}
        }));
        let pins = pin_command_grants(&policy, &root, SymlinkPolicy::Deny, None).unwrap();
        assert_eq!(pins.get("self").unwrap(), &realpath(exe).unwrap());
    }

    #[tokio::test]
    async fn verify_pinned_rejects_missing_swapped_or_workspace_paths() {
        let dir = TempDir::new();
        let root = dir.root();
        // workspace 外の native binary は verify 通過。
        let exe = std::env::current_exe().unwrap();
        let pinned = realpath(exe.to_str().unwrap()).unwrap();
        verify_pinned_executable("g", &pinned, &root).unwrap();
        // 存在しない path → native candidate 不成立で fail-closed。
        let missing = dir.join(if cfg!(windows) { "gone.exe" } else { "gone" });
        let err = verify_pinned_executable("g", missing.to_str().unwrap(), &root).unwrap_err();
        assert!(policy_violation(&err));
        // workspace 内 native file → residency 検査で reject。
        let inside = dir.join(if cfg!(windows) { "in.exe" } else { "in" });
        write(&inside, &native_bytes());
        set_executable(&inside);
        let inside = realpath(inside.to_str().unwrap()).unwrap();
        let err = verify_pinned_executable("g", &inside, &root).unwrap_err();
        assert!(policy_violation(&err));
        assert!(err.to_string().contains("workspace-resident"), "{err}");
        // pin map に無い grant は execute で fail-closed
        // (verify まで到達しない)。
        let options = CommandToolOptions {
            root: root.clone(),
            symlink_policy: SymlinkPolicy::Deny,
            command: policy(json!({"grants": {"g": {"executable": "git"}}})),
            pins: BTreeMap::new(),
            on_timeout: None,
            denylist: None,
            route_context: None,
        };
        let tool = create_command_tool(options);
        let err = tool
            .execute(&json!({"name": "g", "args": []}), None)
            .await
            .unwrap_err();
        assert!(policy_violation(&err));
        assert!(err.to_string().contains("no pinned executable"), "{err}");
    }

    #[test]
    fn within_workspace_boundary() {
        let dir = TempDir::new();
        let root = dir.root();
        // `within_workspace` は canonical 同士の lexical 判定 —
        // file の実在は問わない。
        let inside = nodepath::join(&[&root, "sub", "tool"]);
        assert!(within_workspace(&root, &inside));
        let outside = realpath(std::env::current_exe().unwrap().to_str().unwrap()).unwrap();
        assert!(!within_workspace(&root, &outside));
        // sibling dir は `..` segment 経由で root 外。
        let sibling = format!("{root}-sibling");
        assert!(!within_workspace(&root, &sibling));
        // `..foo` のような prefix 名は `..` segment ではなく root 内
        // として扱う。
        let prefix = nodepath::join(&[&root, "..tool"]);
        assert!(within_workspace(&root, &prefix));
    }
}
