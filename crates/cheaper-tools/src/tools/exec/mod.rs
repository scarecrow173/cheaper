//! 統一 exec grant 実行層(issue #165 — design §(h))。
//!
//! `command`/`process` の 2 surface が共有する runtime 機構を、旧
//! `tools/command` の pin/verify/check/authorize 機構と旧
//! `tools/process/prepare` を unified `ExecGrant` へ汎化して集約した
//! 層。2 surface は disjoint を維持し(`command` は `!fixed` grant、
//! `process` は `fixed` grant のみ到達可能)、共通機構は本 module に
//! 単一実装として置く。
//!
//! 責務:
//! - `ExecToolOptions` — 両 surface 共有の options(表面別 field は
//!   `write_scope`(fixed のみ)・`route_context`(`!fixed` のみ)に
//!   分離コメントで示す)。
//! - `pin_exec_grant`/`pin_exec_grants` — scope 構築の per-scope
//!   executable pin(適用点 2 — resolved basename deny)。
//! - `verify_pinned_executable` — spawn 前再検証(適用点 3)。
//! - `check_exec_execution` — `command::execute` と `route::resolve`
//!   が共有する spawn 無し viability predicate(!fixed 表面)。
//! - `exec_operation_declaration` — 2 surface の op declaration を
//!   単一規則へ(ClaimMode tri-state + surface 別 pool + resources)。
//! - `prepare_exec_grant`(prepare.rs)— fixed grant の per-call
//!   前処理(cwd・manifest・declared-path deny・writeScope tri-state・
//!   pin 検索 + verify)。
//!
//! `RouteCommandContext` は route scope 構築層が注入する command
//! surface 専用 context(!fixed surface のみが読む)。

use std::collections::{BTreeMap, HashSet};

use cheaper_core::errors::{CheaperError, ErrorCode};
pub use cheaper_core::policy::exec::{exec_grant_resources, resolve_exec_grant};
use cheaper_core::policy::exec::{ArgPattern, ExecGrant, ResolvedExecPolicy};
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::resources::types::ClaimMode;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use serde_json::{json, Map, Value};

use crate::scheduler::{OperationDeclaration, OperationPool};
use crate::tools::proc::executable::{
    native_executable, resolve_process_executable, ProcessExecutableOptions,
};
use crate::tools::proc::{
    prepare_cwd, prepare_exec_cwd, ExecutionCorrelationCarrier, OnTimeout, ProcTee,
};
use crate::ToolError;

pub(crate) mod prepare;
pub(crate) use prepare::{prepare_exec_grant, PreparedExec};

// ---------------------------------------------------------------------------
// options / route context
// ---------------------------------------------------------------------------

/// unified options — surface 別 field の ownership: `write_scope` は
/// process(fixed)surface のみ、`route_context` は command(`!fixed`)
/// surface のみが読む。両 surface で共有する機構 field は 1 本に集約
/// (false abstraction 回避のため 2 struct を維持しない — design §(h))。
pub struct ExecToolOptions {
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    /// unified resolved policy — 両 surface が同一 instance を共有する
    /// (disjoint は `resolve_exec_grant` の `fixed` 期待値で強制)。
    pub exec: ResolvedExecPolicy,
    /// grant name → pin 済み canonical executable path(per-scope pin —
    /// `create_tool_runtime` が expose surface の到達可能 grant 群のみ
    /// 構築。surface をまたぐ cache はない)。
    pub pins: BTreeMap<String, String>,
    /// `onTimeout(reason)`。
    pub on_timeout: Option<OnTimeout>,
    pub denylist: Option<PathDenylist>,
    /// `security.execDenyPatterns` の compiled deny predicate
    /// (issue #164 — opt-in)。`denylist` 先例の owned field —
    /// 非 Option で空 `Default` が no-op を表す。適用点 3
    /// (`verify_pinned_executable` の spawn 前再検証)で使う。
    pub exec_deny: ExecDenyList,
    /// worker writeScope — **fixed surface のみ**が読む(`!fixed`
    /// grant は構築時 gate で writeScope 下に到達不能のため、command
    /// surface の options では常に `None`)。
    pub write_scope: Option<HashSet<String>>,
    /// route scope 構築層のみが注入する route context — **`!fixed`
    /// surface のみ**が読む(design §route context 注入)。`None` で
    /// 従来挙動不変。
    pub route_context: Option<RouteCommandContext>,
    /// managed child に渡す明示 hint。認可判断には使わない。
    pub execution_correlation: Option<ExecutionCorrelationCarrier>,
}

/// route scope の command tool 注入 context(design §route context
/// 注入、R18/R26/C2)。旧 `tools/command` から unified 層へ移設。
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

// ---------------------------------------------------------------------------
// error conventions
// ---------------------------------------------------------------------------

/// grant 文脈の `POLICY_VIOLATION` — unified contract `exec grant
/// "{name}": {reason}` + `details.grant`(design §(n))。pin・verify・
/// argv 拒否など実行時 policy violation の単一成形点。
pub(crate) fn grant_violation(name: &str, reason: &str) -> ToolError {
    let mut details = Map::new();
    details.insert("grant".to_string(), json!(name));
    CheaperError::new(
        ErrorCode::PolicyViolation,
        format!("exec grant \"{name}\": {reason}"),
    )
    .with_details(details)
    .into()
}

// ---------------------------------------------------------------------------
// pin / spawn-time re-verification(design §(b)・適用点 2/3)
// ---------------------------------------------------------------------------

/// surface subset への複数 grant 版 — registry が `fixed` filter で
/// 呼ぶ(`names` 含 surface の到達可能 grant 群のみ pin)。
/// `create_tool_runtime` 境界の per-scope pin(design §(b)/(g))。
/// 各 grant の declared executable を ambient PATH で解決して
/// realpath 済み canonical path へ pin する。pin 不能の grant は
/// `POLICY_VIOLATION` で呼出し側の fail/縮退判定へ返す。
/// registry を bypass する直接 `create_process_tool`/`create_command_tool`
/// 経路(integration test・route resolve)も同一 seam で pin map を
/// 構築する。
pub fn pin_exec_grants(
    policy: &ResolvedExecPolicy,
    fixed: bool,
    root: &str,
    symlink_policy: SymlinkPolicy,
    denylist: Option<&PathDenylist>,
    exec_deny: &ExecDenyList,
) -> Result<BTreeMap<String, String>, ToolError> {
    let mut pins = BTreeMap::new();
    for (name, grant) in policy.grants.iter().filter(|(_, g)| g.fixed == fixed) {
        let resolved = pin_exec_grant(name, grant, root, symlink_policy, denylist, exec_deny)?;
        pins.insert(name.clone(), resolved);
    }
    Ok(pins)
}

/// `pin_exec_grants` の単一 grant 版 — grant name → pin 済み
/// canonical path。`route::resolve` の viability 事前評価
/// (`check_exec_execution`)が scope pin 未構築の状態でも同一
/// predicate を使うための共有 seam。
/// pin 不能の grant は `POLICY_VIOLATION` で返す。
/// resolved executable が `security.execDenyPatterns` に match
/// する場合も `POLICY_VIOLATION`(適用点 2 — declared spec を
/// PATH 解決した実 basename で deny を再適用)。
pub fn pin_exec_grant(
    name: &str,
    grant: &ExecGrant,
    root: &str,
    symlink_policy: SymlinkPolicy,
    denylist: Option<&PathDenylist>,
    exec_deny: &ExecDenyList,
) -> Result<String, ToolError> {
    let resolved = match resolve_process_executable(
        &grant.executable,
        &ProcessExecutableOptions {
            root,
            symlink_policy,
            denylist,
            search_path: None,
        },
        "exec grant",
    ) {
        Ok(resolved) => resolved,
        Err(err) => {
            // `PolicyViolation`(unsupported)は unified contract
            // `exec grant "{name}": {reason}` + `details.grant` へ
            // 射影する(design §(n))。`WorkspaceEscape` 等の別種
            // error はそのまま伝播する。
            let cheaper =
                crate::tools::proc::find_cheaper_error(err.as_ref() as &dyn std::error::Error);
            match cheaper {
                Some(e) if e.code() == ErrorCode::PolicyViolation => {
                    let reason = e
                        .message()
                        .strip_prefix("exec grant ")
                        .unwrap_or(e.message())
                        .to_string();
                    return Err(grant_violation(name, &reason));
                }
                _ => return Err(err),
            }
        }
    };
    // 適用点 2 — resolved basename で deny を再評価(declared 名が
    // shim/wrapper 名でも resolved 実体が deny 対象なら拒否)。
    if exec_deny.is_denied(&resolved) {
        return Err(grant_violation(
            name,
            "resolved executable is denied by security.execDenyPatterns",
        ));
    }
    Ok(resolved)
}

/// pin 済み executable の spawn 前再検証(design §(b)) — pin 後の
/// file 差替えに対する fail-closed。`native_executable` で spawn
/// viability(realpath → is_file → win32 ext → X_OK)を再適用し、
/// resolved path が pin 済み canonical path と一致するかを検査する。
/// pin 済み path は canonical のため、realpath が変わっていた場合は
/// symlink 差替えとして reject する。pin-equality 直後に
/// `security.execDenyPatterns` を同一 predicate で再適用する
/// (適用点 3 — 全適用点で同一 predicate を再適用する uniform
/// 契約)。
fn verify_pinned_executable(
    name: &str,
    pinned: &str,
    exec_deny: &ExecDenyList,
) -> Result<(), ToolError> {
    let Some(resolved) = native_executable(pinned) else {
        return Err(grant_violation(
            name,
            "pinned executable no longer resolves to a spawnable executable",
        ));
    };
    if resolved != pinned {
        return Err(grant_violation(
            name,
            "pinned executable path changed since pin",
        ));
    }
    // 適用点 3 — spawn 前に resolved basename で deny を再適用。
    if exec_deny.is_denied(&resolved) {
        return Err(grant_violation(
            name,
            "resolved executable is denied by security.execDenyPatterns",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// check_exec_execution(!fixed surface 共有 viability predicate — B#3)
// ---------------------------------------------------------------------------

/// `check_exec_execution` の検証済み出力 — spawn に必要な
/// grant・cwd・executable を保持する(predicate 結果の共有
/// transport。`command::execute` はこの 3 値をそのまま使い、
/// `route::resolve` は viability の成否のみ見る)。
#[derive(Debug)]
pub struct CheckedExec {
    pub grant: ExecGrant,
    pub cwd: String,
    /// 検証済み executable — scope pin map の pin 済み path を
    /// spawn 前に再検証した結果(`verify_pinned_executable`)。
    /// pin map に grant が無ければここへは到達しない。
    pub executable: String,
}

/// spawn 無しの事前 check(design §viability — B#3 共有 seam)。
/// `command::execute` が実 spawn 前に課す全 gate を同一 predicate
/// で評価する: `!fixed` grant 解決 → argv pattern 照合 → cwd
/// (route context の `exec_cwd` override を含む)→ executable
/// 検証(per-scope pin map の pin 済み path の再検証。pin 不在は
/// `POLICY_VIOLATION` — resolver は選択 grant を `pin_exec_grant`
/// で pre-pin してから本 seam を呼ぶ)。predicate の複製による
/// drift を防ぐため `command::execute` もこの entry を呼ぶ。
pub fn check_exec_execution(
    options: &ExecToolOptions,
    name: &str,
    args: &[String],
) -> Result<CheckedExec, ToolError> {
    let grant =
        resolve_exec_grant(&options.exec, name, false).map_err(|e| Box::new(e) as ToolError)?;
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
    // executable 事前検証 — pin 済み path は scope 構築時点の
    // canonical へ immutable に固定され、spawn 前にここで
    // `native_executable` 再解決 + path 同一性を再検証する
    // (verify_pinned)。pin map 無しの経路は fail-closed。
    let executable = match options.pins.get(name) {
        Some(pinned) => {
            verify_pinned_executable(name, pinned, &options.exec_deny)?;
            pinned.clone()
        }
        None => {
            return Err(grant_violation(name, "no pinned executable"));
        }
    };
    Ok(CheckedExec {
        grant,
        cwd,
        executable,
    })
}

/// grant argv pattern match。`grant.argv` が `None` なら無条件受理、
/// `Some` なら compile + `matches` 照合のみ — 不一致は
/// `POLICY_VIOLATION`。argv token の内容は審査しない。
/// `!fixed` grant のみが呼ぶ(fixed の `argv` は literal であり
/// pattern 照合しない)。
fn authorize_args(name: &str, grant: &ExecGrant, args: &[String]) -> Result<(), ToolError> {
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

// ---------------------------------------------------------------------------
// operation declaration
// ---------------------------------------------------------------------------

/// `execOperationDeclaration` — 2 surface の op declaration を単一
/// 規則へ置換する(design §(h)):
///
/// - grant 解決: `resolve_exec_grant(policy, name, fixed)` — surface
///   不一致・未知名は `POLICY_VIOLATION`(schedule 前に fail-closed)。
/// - ClaimMode: `outputs: Some([])` → Shared、その他(`Some(nonempty)`
///   ・`None` 未宣言 = 未知副作用を保守的に)→ Exclusive(R2 —
///   manifest/writeScope と同一 tri-state 規約)。
/// - pool: `fixed` → `{"process", policy.process.max_concurrent}`、
///   `!fixed` → `{"command", policy.command.max_concurrent}`(R3/D11)。
/// - `declaration.resources` は全 grant で設定(宣言 inputs/outputs
///   があれば per-path、無ければ root manifest — `exec_grant_resources`
///   の reads+writes を claim として載せる)。
pub fn exec_operation_declaration(
    policy: &ResolvedExecPolicy,
    name: &str,
    fixed: bool,
) -> Result<OperationDeclaration, CheaperError> {
    let grant = resolve_exec_grant(policy, name, fixed)?;
    let manifest = exec_grant_resources(policy, name)?;
    let scopes = |items: &Option<Vec<cheaper_core::resources::normalization::ScopeDeclaration>>| {
        serde_json::to_value(items).expect("ScopeDeclaration は必ず serialize 可能")
    };
    // `outputs: Some([])` = 「副作用を宣言しない」契約 → Shared。
    // `Some(nonempty)`・`None` は未知副作用を保守的に Exclusive。
    let access = match &grant.outputs {
        Some(outputs) if outputs.is_empty() => ClaimMode::Shared,
        _ => ClaimMode::Exclusive,
    };
    let mut declaration = OperationDeclaration::new(access);
    // pool は reach surface で引く(fixed → process、!fixed →
    // command)— lifecycle は authoring origin defaults、pool は
    // surface 経路の並行性 budget という意図的非対称(D11)。
    let defaults = if fixed {
        &policy.process
    } else {
        &policy.command
    };
    declaration.pool = Some(OperationPool {
        name: if fixed { "process" } else { "command" }.to_string(),
        max_concurrent: defaults.max_concurrent as i64,
    });
    declaration.resources = Some(json!({
        "reads": scopes(&manifest.reads),
        "writes": scopes(&manifest.writes),
    }));
    Ok(declaration)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use cheaper_core::policy::exec::build_exec_input;
    use cheaper_core::workspace::paths::{canonicalize_workspace_root, realpath};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!(
                "cheaper-exec-{}-{}",
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

    /// 実在 executable file の fixture bytes — 内容は検査対象では
    /// なく、file の実在性と実行 bit だけが意味を持つ。
    fn executable_file_bytes() -> Vec<u8> {
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

    /// authoring `command` section → unified policy への seam 変換。
    fn exec_policy(command: Value) -> ResolvedExecPolicy {
        let doc = build_exec_input(None, Some(&command), &BTreeSet::new()).expect("synthesis");
        cheaper_core::policy::exec::resolve_exec_policy(
            Some(&Value::Object(doc)),
            &ExecDenyList::default(),
        )
        .unwrap()
    }

    fn policy_violation(err: &ToolError) -> bool {
        crate::tools::proc::find_cheaper_error(err.as_ref() as &dyn std::error::Error)
            .is_some_and(|e| e.code() == ErrorCode::PolicyViolation)
    }

    fn grant<'a>(policy: &'a ResolvedExecPolicy, name: &str) -> &'a ExecGrant {
        policy.grants.get(name).expect("grant")
    }

    fn exec_options(root: &str, exec: ResolvedExecPolicy) -> ExecToolOptions {
        ExecToolOptions {
            root: root.to_string(),
            symlink_policy: SymlinkPolicy::Deny,
            exec,
            pins: BTreeMap::new(),
            on_timeout: None,
            denylist: None,
            exec_deny: ExecDenyList::default(),
            write_scope: None,
            route_context: None,
            execution_correlation: None,
        }
    }

    #[test]
    fn operation_declaration_is_exclusive_with_own_command_pool() {
        let policy = exec_policy(json!({
            "maxConcurrent": 3,
            "grants": {"git-read": {"executable": "git"}}
        }));
        let declaration = exec_operation_declaration(&policy, "git-read", false).unwrap();
        assert_eq!(declaration.access, ClaimMode::Exclusive);
        let pool = declaration.pool.unwrap();
        assert_eq!(pool.name, "command");
        assert_eq!(pool.max_concurrent, 3);
        // `inputs`/`outputs` 未宣言 → 保守的 manifest: root subtree
        // reads+writes、produces は宣言しない(design §(f)/(h))。
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
    fn operation_declaration_declared_paths_narrow_manifest() {
        // `inputs`/`outputs` 宣言 grant は per-path manifest へ
        // narrowing される(declared-contract 規約 — design §(h))。
        let policy = exec_policy(json!({
            "grants": {"build": {
                "executable": "cc",
                "inputs": ["src/**"],
                "outputs": ["dist"],
            }}
        }));
        let declaration = exec_operation_declaration(&policy, "build", false).unwrap();
        assert_eq!(declaration.access, ClaimMode::Exclusive);
        let resources = declaration.resources.unwrap();
        assert_eq!(
            resources,
            json!({
                "reads": [{"path": "src/**", "scope": "subtree"}],
                "writes": [{"path": "dist", "scope": "exact"}],
            })
        );
        // `outputs: Some([])` は Shared(副作用を宣言しない契約)。
        let policy = exec_policy(json!({
            "grants": {"ro": {"executable": "cc", "outputs": []}}
        }));
        let declaration = exec_operation_declaration(&policy, "ro", false).unwrap();
        assert_eq!(declaration.access, ClaimMode::Shared);
        // surface 不一致 — fixed grant を !fixed で解決すると
        // unknown command grant。
        let policy = exec_policy(json!({
            "grants": {"fx": {"executable": "cc", "fixed": true}}
        }));
        let err = exec_operation_declaration(&policy, "fx", false).unwrap_err();
        assert_eq!(err.code(), ErrorCode::PolicyViolation);
    }

    #[test]
    fn authorize_args_matches_pattern_only() {
        let policy = exec_policy(json!({
            "grants": {"git-read": {"executable": "git", "argv": ["log", "*"]}}
        }));
        let grant = grant(&policy, "git-read");
        // metachar token は内容審査されず literal argv として受理される。
        for token in ["a|b", "x;y", "$(id)", "`id`", "a>b", "line\nfeed"] {
            authorize_args("git-read", grant, &["log".to_string(), token.to_string()])
                .unwrap_or_else(|e| panic!("{token}: accepted: {e}"));
        }
        // pattern 不一致(超過・不足・別 token)は `POLICY_VIOLATION`。
        for args in [
            vec!["diff".to_string()],
            vec![],
            vec!["branch".to_string(), "x".to_string()],
        ] {
            let err = authorize_args("git-read", grant, &args).unwrap_err();
            assert!(policy_violation(&err), "{args:?}");
        }
        // rest marker の残余 token は pattern 照合のみが制約。
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
        let policy = exec_policy(json!({"grants": {"g": {"executable": "git"}}}));
        let grant = grant(&policy, "g");
        authorize_args("g", grant, &[]).unwrap();
        authorize_args("g", grant, &["status".to_string(), "--short".to_string()]).unwrap();
        // metachar token も argv 省略時は受理される。
        authorize_args("g", grant, &["a;b".to_string()]).unwrap();
    }

    #[test]
    fn pin_resolves_workspace_relative_executable_path() {
        let dir = TempDir::new();
        let root = dir.root();
        // workspace 内の実在 file への relative declared executable は
        // `resolve_workspace_path` の workspace 境界内で解決され pin
        // される。
        let declared = if cfg!(windows) {
            "bin\\tool.exe"
        } else {
            "bin/tool"
        };
        let bin = dir.join(if cfg!(windows) {
            "bin/tool.exe"
        } else {
            "bin/tool"
        });
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        write(&bin, &executable_file_bytes());
        set_executable(&bin);
        let policy = exec_policy(json!({
            "grants": {"g": {"executable": declared}}
        }));
        let pins = pin_exec_grants(
            &policy,
            false,
            &root,
            SymlinkPolicy::Deny,
            None,
            &ExecDenyList::default(),
        )
        .unwrap();
        assert_eq!(
            pins.get("g").unwrap(),
            &realpath(bin.to_str().unwrap()).unwrap()
        );
    }

    #[test]
    fn pin_accepts_absolute_path_inside_workspace() {
        let dir = TempDir::new();
        let root = dir.root();
        // workspace 内 absolute path の executable も pin される。
        let bin = dir.join(if cfg!(windows) { "tool.exe" } else { "tool" });
        write(&bin, &executable_file_bytes());
        set_executable(&bin);
        let policy = exec_policy(json!({
            "grants": {"g": {"executable": bin.to_str().unwrap()}}
        }));
        let pins = pin_exec_grants(
            &policy,
            false,
            &root,
            SymlinkPolicy::Deny,
            None,
            &ExecDenyList::default(),
        )
        .unwrap();
        assert_eq!(
            pins.get("g").unwrap(),
            &realpath(bin.to_str().unwrap()).unwrap()
        );
    }

    #[test]
    fn pin_fails_closed_on_unresolvable_executable() {
        let dir = TempDir::new();
        let root = dir.root();
        // ambient PATH で解決不能 → `exec grant "{name}": {reason}` の
        // `POLICY_VIOLATION` + `details.grant`(design §(n))。
        let policy = exec_policy(json!({
            "grants": {"g": {"executable": "definitely-missing-tool-xyz"}}
        }));
        let err = pin_exec_grants(
            &policy,
            false,
            &root,
            SymlinkPolicy::Deny,
            None,
            &ExecDenyList::default(),
        )
        .unwrap_err();
        assert!(policy_violation(&err));
        assert!(
            err.to_string()
                .contains("exec grant \"g\": could not be resolved to a spawnable executable"),
            "{err}"
        );
        let e =
            crate::tools::proc::find_cheaper_error(err.as_ref() as &dyn std::error::Error).unwrap();
        assert_eq!(e.details().unwrap()["grant"], json!("g"));
    }

    #[test]
    fn pin_maps_grant_to_resolved_canonical_path() {
        let dir = TempDir::new();
        let root = dir.root();
        // `current_exe` は workspace 外の native binary — absolute
        // path grant として pin できる。
        let exe = std::env::current_exe().unwrap();
        let exe = exe.to_str().unwrap();
        let policy = exec_policy(json!({
            "grants": {"self": {"executable": exe}}
        }));
        let pins = pin_exec_grants(
            &policy,
            false,
            &root,
            SymlinkPolicy::Deny,
            None,
            &ExecDenyList::default(),
        )
        .unwrap();
        assert_eq!(pins.get("self").unwrap(), &realpath(exe).unwrap());
    }

    /// `fixed` filter — `pin_exec_grants(.., true)` は fixed grant のみ
    /// pin し `!fixed` を、`(.., false)` は `!fixed` のみ pin し
    /// `fixed` を skip する(disjoint surface の pin 分担)。
    #[test]
    fn pin_filters_grants_by_fixed_surface() {
        let dir = TempDir::new();
        let root = dir.root();
        let exe = std::env::current_exe().unwrap();
        let exe = exe.to_str().unwrap();
        let doc = json!({
            "grants": {
                "cmd": {"executable": exe},
                "task": {"executable": exe, "fixed": true, "argv": ["--x"]},
            },
        });
        let policy =
            cheaper_core::policy::exec::resolve_exec_policy(Some(&doc), &ExecDenyList::default())
                .unwrap();
        let fixed = pin_exec_grants(
            &policy,
            true,
            &root,
            SymlinkPolicy::Deny,
            None,
            &ExecDenyList::default(),
        )
        .unwrap();
        assert_eq!(fixed.keys().collect::<Vec<_>>(), ["task"]);
        let dynamic = pin_exec_grants(
            &policy,
            false,
            &root,
            SymlinkPolicy::Deny,
            None,
            &ExecDenyList::default(),
        )
        .unwrap();
        assert_eq!(dynamic.keys().collect::<Vec<_>>(), ["cmd"]);
    }

    /// 適用点 2(issue #164)— resolved executable の basename が
    /// `exec_deny` に match する grant は `pin_exec_grant` /
    /// `pin_exec_grants` で `PolicyViolation` +
    /// `details.grant`(blast radius — deny grant を含む policy
    /// 全体の pin が fail する)。
    #[test]
    fn pin_rejects_resolved_executable_matching_exec_deny() {
        let dir = TempDir::new();
        let root = dir.root();
        let bin = dir.join(if cfg!(windows) {
            "deny-me.exe"
        } else {
            "deny-me"
        });
        write(&bin, &executable_file_bytes());
        set_executable(&bin);
        // declared 名は workspace 内 absolute path — 適用点 1 は
        // default(空)deny で素通しさせ、適用点 2 だけを検査する。
        let deny_policy = exec_policy(json!({
            "grants": {
                "safe": {"executable": "git"},
                "denied": {"executable": bin.to_str().unwrap()},
            }
        }));
        let deny = ExecDenyList::compile(&["deny-me".to_string()]).unwrap();
        // 単一 grant — resolved basename `deny-me` が pattern match。
        let err = pin_exec_grant(
            "denied",
            grant(&deny_policy, "denied"),
            &root,
            SymlinkPolicy::Deny,
            None,
            &deny,
        )
        .unwrap_err();
        assert!(policy_violation(&err), "{err}");
        assert!(
            err.to_string()
                .contains("resolved executable is denied by security.execDenyPatterns"),
            "{err}"
        );
        let e =
            crate::tools::proc::find_cheaper_error(err.as_ref() as &dyn std::error::Error).unwrap();
        assert_eq!(e.details().unwrap()["grant"], json!("denied"));
        // blast radius — denied resolved grant を含む policy 全体の
        // pin が Err(順序非依存で最初の deny で fail-closed)。
        let err = pin_exec_grants(&deny_policy, false, &root, SymlinkPolicy::Deny, None, &deny)
            .unwrap_err();
        assert!(policy_violation(&err), "{err}");
        // 既定(空)deny では両 grant とも pin を試行する(deny 非発火
        // — `safe` は PATH 上の `git` に依存するため、ここでは
        // `denied` grant のみを含む policy で空 deny の受理側を pin)。
        let default_deny_policy = exec_policy(json!({
            "grants": {"g": {"executable": bin.to_str().unwrap()}}
        }));
        let pins = pin_exec_grants(
            &default_deny_policy,
            false,
            &root,
            SymlinkPolicy::Deny,
            None,
            &ExecDenyList::default(),
        )
        .unwrap();
        assert_eq!(
            pins.get("g").unwrap(),
            &realpath(bin.to_str().unwrap()).unwrap()
        );
    }

    #[tokio::test]
    async fn verify_pinned_rejects_missing_accepts_workspace_paths() {
        let dir = TempDir::new();
        let root = dir.root();
        // spawn 可能な executable は verify 通過。
        let exe = std::env::current_exe().unwrap();
        let pinned = realpath(exe.to_str().unwrap()).unwrap();
        verify_pinned_executable("g", &pinned, &ExecDenyList::default()).unwrap();
        // 存在しない path → candidate 不成立で fail-closed。
        let missing = dir.join(if cfg!(windows) { "gone.exe" } else { "gone" });
        let err =
            verify_pinned_executable("g", missing.to_str().unwrap(), &ExecDenyList::default())
                .unwrap_err();
        assert!(policy_violation(&err));
        assert!(err.to_string().contains("no longer resolves"), "{err}");
        // workspace 内 file も `resolved == pinned` で検査通過。
        let inside = dir.join(if cfg!(windows) { "in.exe" } else { "in" });
        write(&inside, &executable_file_bytes());
        set_executable(&inside);
        let inside = realpath(inside.to_str().unwrap()).unwrap();
        verify_pinned_executable("g", &inside, &ExecDenyList::default()).unwrap();
        // 適用点 3 — pin-equality 通過後も resolved basename が
        // `exec_deny` に match すれば `PolicyViolation`(同一
        // predicate の uniform 再適用)。
        let stem = cheaper_core::policy::security::executable_stem(&inside);
        let deny = ExecDenyList::compile(&[stem]).unwrap();
        let err = verify_pinned_executable("g", &inside, &deny).unwrap_err();
        assert!(policy_violation(&err), "{err}");
        assert!(
            err.to_string()
                .contains("resolved executable is denied by security.execDenyPatterns"),
            "{err}"
        );
        let e =
            crate::tools::proc::find_cheaper_error(err.as_ref() as &dyn std::error::Error).unwrap();
        assert_eq!(e.details().unwrap()["grant"], json!("g"));
        // pin map に無い grant は check で fail-closed
        // (verify まで到達しない)。
        let exe = std::env::current_exe().unwrap();
        let exe = realpath(exe.to_str().unwrap()).unwrap();
        let options = exec_options(
            &root,
            exec_policy(json!({"grants": {"g": {"executable": exe}}})),
        );
        let err = check_exec_execution(&options, "g", &[]).unwrap_err();
        assert!(policy_violation(&err));
        assert!(err.to_string().contains("no pinned executable"), "{err}");
    }

    /// disjoint surface — `check_exec_execution` は `!fixed` grant
    /// のみ解決し、`fixed` grant 名は unknown command grant として
    /// `POLICY_VIOLATION`(design §(i))。
    #[test]
    fn check_rejects_fixed_grant_on_command_surface() {
        let dir = TempDir::new();
        let root = dir.root();
        let exe = std::env::current_exe().unwrap();
        let exe = exe.to_str().unwrap();
        let options = exec_options(
            &root,
            cheaper_core::policy::exec::resolve_exec_policy(
                Some(&json!({
                    "grants": {"task": {"executable": exe, "fixed": true}}
                })),
                &ExecDenyList::default(),
            )
            .unwrap(),
        );
        let err = check_exec_execution(&options, "task", &[]).unwrap_err();
        assert!(policy_violation(&err));
        assert!(
            err.to_string()
                .contains("unknown or unauthorized command grant"),
            "{err}"
        );
    }
}
