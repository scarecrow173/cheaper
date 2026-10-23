//! `route::resolve` — argv → route match → viability の純粋評価
//! 入口(design §RouteResolver、R4-R12)。hook eval・`route
//! resolve`・`route run` の 3 caller が共有する唯一の resolve
//! 実装。file I/O・policy load は caller 責務(`ResolveInput` で
//! 済み分を受ける)で、load 失敗は caller が `invalid-route`/
//! `no-grant` viability へ写像する(fail-safe、R39)。

use std::path::Path;

use cheaper_core::policy::exec::ResolvedExecPolicy;
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_tools::tools::exec::{check_exec_execution, ExecToolOptions, RouteCommandContext};

use super::routes::EffectiveRoutes;
use crate::hook::routing_policy::{command_name, is_env_prefix, CWD_CHANGING_COMMANDS};

/// viability の 1 値(R12 の固定優先順に従い最初に成立した分類)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viability {
    Runnable,
    NoGrant,
    MissingTarget,
    InvalidRoute,
    UnsafeCommand,
    StdinUnsupported,
}

impl Viability {
    /// resolve 出力(R11)の wire 値。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Runnable => "runnable",
            Self::NoGrant => "no-grant",
            Self::MissingTarget => "missing-target",
            Self::InvalidRoute => "invalid-route",
            Self::UnsafeCommand => "unsafe-command",
            Self::StdinUnsupported => "stdin-unsupported",
        }
    }
}

/// `resolve` への入力 — routes/command policy は load 済みを受け
/// る純粋評価(A#10 — hook eval での grants 供給経路)。
pub struct ResolveInput<'a> {
    /// raw argv(env-prefix 含み得る — strip は resolve が担う)。
    pub argv: &'a [String],
    /// workspace root(viability の `check_command_execution`
    /// preflight・payload `cwd`・`exec_cwd` 復元の基準)。
    pub cwd: &'a str,
    /// `routes::load` 済みの effective route set。
    pub routes: &'a EffectiveRoutes,
    /// effective exec policy(`load_policy` resolved)の参照面。
    pub exec_policy: &'a ExecPolicyView<'a>,
    /// 入力 command が複数 shell segment・unsafe segment/token を
    /// 含む場合に caller が立てる flag(`route resolve --command`
    /// の tokenize 結果)。hook eval は segment 安全性を resolve
    /// 前に評価済みのため `false` を渡す。
    pub unsafe_command: bool,
}

/// viability の grant-match/preflight が参照する exec policy
/// 環境 — `check_exec_execution` と同一 predicate で評価する
/// ため `ExecToolOptions` 構築に必要な面だけを束ねる
/// (design §(j) — 旧 `CommandPolicyView`)。
#[derive(Clone, Copy)]
pub struct ExecPolicyView<'a> {
    pub policy: &'a ResolvedExecPolicy,
    /// pin 解決・`prepare_cwd` の workspace root(通常 resolve
    /// input の `cwd` と同じ値)。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    pub denylist: Option<&'a PathDenylist>,
    /// `security.execDenyPatterns` の compiled deny(issue #164)—
    /// 全 field borrowed の struct 規約に従い `&'a` で受ける
    /// (`Copy` 維持)。`pin_exec_grant`・`ExecToolOptions`
    /// の適用点 2/3 で同一 predicate を見る。
    pub exec_deny: &'a ExecDenyList,
}

/// resolve 結果(R11 の wire shape にそのまま載る)。
#[derive(Debug, Clone)]
pub struct RouteResolution {
    pub matched: bool,
    /// match した route 名。
    pub route: Option<String>,
    /// 解決済み workflow path(宣言層起点の絶対 path)。
    pub target: Option<String>,
    /// 参照 grant 名。
    pub grant: Option<String>,
    /// `argv[0]` の `command_name`。
    pub executable: Option<String>,
    /// env-prefix 除く argv 列。
    pub argv: Vec<String>,
    /// `argv[len(match.command)..]` — R4/R11 の表示用。
    pub args: Vec<String>,
    /// `argv[1..]` — spawn argv / grant argv 照合の基準(C1)。
    pub exec_args: Vec<String>,
    pub viability: Option<Viability>,
    /// trust 不在で負けた project route 名(provenance 報告)。
    pub suppressed: Option<String>,
    /// match した route の `passthroughEnv` 宣言 — payload encode
    /// の ambient env allowlist(`route run` の再 resolve も同じ値
    /// を使う)。no-match では空。
    pub passthrough_env: Vec<String>,
}

/// `passthroughEnv` 未宣言でも best-effort で routing しない
/// stdin 依存 command の curated list(R33 — 網羅性ではなく
/// stdin 非接続の構造保証を補助する分類)。editors・pagers・
/// REPL・ssh・sudo・login shell。
const INTERACTIVE_COMMANDS: &[&str] = &[
    // editors
    "vi",
    "vim",
    "nvim",
    "nano",
    "emacs",
    "code",
    "subl",
    // pagers
    "less",
    "more",
    // REPL/interpreters(bare invocation は stdin 駆動)
    "python",
    "python3",
    "node",
    "ruby",
    "irb",
    "perl",
    "psql",
    "mysql",
    "sqlite3",
    "redis-cli",
    // remote/credential prompts
    "ssh",
    "sudo",
    "su",
    "ftp",
    "sftp",
    "telnet",
    // login shells
    "bash",
    "zsh",
    "sh",
    "fish",
    "csh",
    "tcsh",
    "pwsh",
    "powershell",
    "cmd",
    // misc TTY consumers
    "top",
    "htop",
    "watch",
    "man",
];

/// `docker`/`kubectl`/`podman` の interactive flag — `-it`・`-i`・
/// `--interactive`/`--tty` を argv に含む場合 stdin 依存と見なす。
const INTERACTIVE_FLAG_COMMANDS: &[&str] = &["docker", "kubectl", "podman"];

fn is_interactive(executable: &str, exec_args: &[String]) -> bool {
    let name = executable.to_ascii_lowercase();
    if INTERACTIVE_COMMANDS.contains(&name.as_str()) {
        return true;
    }
    INTERACTIVE_FLAG_COMMANDS.contains(&name.as_str())
        && exec_args
            .iter()
            .any(|a| matches!(a.as_str(), "-it" | "-i" | "--interactive" | "--tty" | "-t"))
}

/// 先頭の `KEY=VALUE` token 列を env-prefix として分離する
/// (`extract_read_candidates` と同じ skip 規約)。戻り値は
/// `(env-prefix pair 列, 残り argv)`。
pub fn split_env_prefix(argv: &[String]) -> (Vec<(String, String)>, &[String]) {
    let mut i = 0;
    let mut pairs = Vec::new();
    while let Some(token) = argv.get(i) {
        if !is_env_prefix(token) {
            break;
        }
        if let Some((k, v)) = token.split_once('=') {
            pairs.push((k.to_string(), v.to_string()));
        }
        i += 1;
    }
    (pairs, &argv[i..])
}

fn no_match(
    argv: Vec<String>,
    executable: Option<String>,
    suppressed: Option<String>,
) -> RouteResolution {
    RouteResolution {
        matched: false,
        route: None,
        target: None,
        grant: None,
        executable,
        argv,
        args: Vec::new(),
        exec_args: Vec::new(),
        viability: None,
        suppressed,
        passthrough_env: Vec::new(),
    }
}

/// route match → viability の純粋評価。`ResolveInput.argv` は
/// env-prefix を含み得る raw 列(`--argv` arm も同じく先頭 skip で
/// 正規化されるよう resolve 側で strip する — 2 経路で結果が
/// 乖離しない)。
pub fn resolve(input: &ResolveInput<'_>) -> RouteResolution {
    let (_, argv) = split_env_prefix(input.argv);
    let suppressed = |command: &[String]| {
        input
            .routes
            .suppressed
            .iter()
            .find(|s| is_prefix(&s.command, command))
            .map(|s| s.name.clone())
    };
    if argv.is_empty() {
        return no_match(Vec::new(), None, None);
    }
    let executable = command_name(&argv[0]);
    // 1. self-bypass(R9)— cheaper 自身の command は match 評価
    // しない(recursion 防止)。
    if executable.eq_ignore_ascii_case("cheaper") {
        return no_match(Vec::new(), Some(executable), None);
    }
    // 2. argv prefix match(R4)+ longest match(R5)。
    let mut matches: Vec<&super::routes::Route> = input
        .routes
        .routes
        .iter()
        .filter(|r| is_prefix(&r.command, argv))
        .collect();
    if matches.is_empty() {
        return no_match(argv.to_vec(), Some(executable), suppressed(argv));
    }
    matches.sort_by_key(|r| r.command.len());
    let best = matches.pop().expect("nonempty");
    let tied = matches
        .last()
        .is_some_and(|r| r.command.len() == best.command.len());
    let argv = argv.to_vec();
    let args = argv[best.command.len()..].to_vec();
    let exec_args = argv[1..].to_vec();
    let mut out = RouteResolution {
        matched: true,
        route: Some(best.name.clone()),
        target: Some(best.target_path.display().to_string()),
        grant: Some(best.grant.clone()),
        executable: Some(executable.clone()),
        argv,
        args,
        exec_args,
        viability: None,
        suppressed: suppressed(&best.command),
        passthrough_env: best.passthrough_env.clone(),
    };
    // 3. viability 固定優先順(R12)— 最初に成立した 1 値。
    out.viability = Some(viability(input, best, &out, tied));
    out
}

fn is_prefix(prefix: &[String], argv: &[String]) -> bool {
    prefix.len() <= argv.len() && argv[..prefix.len()] == *prefix
}

fn viability(
    input: &ResolveInput<'_>,
    route: &super::routes::Route,
    out: &RouteResolution,
    tied: bool,
) -> Viability {
    // unsafe-command — caller 報告の unsafe/複数 segment 入力、
    // または executable が cwd 変化 command。tokenize 済み argv の
    // metachar 内容は分類しない — authorization は grant argv 契約が
    // 担い routing 層は内容審査を持たない(D2)。
    let name = out.executable.as_deref().unwrap_or_default();
    if input.unsafe_command || CWD_CHANGING_COMMANDS.contains(&name.to_lowercase().as_str()) {
        return Viability::UnsafeCommand;
    }
    // invalid-route — longest match の同 token tie(構築時重複検査
    // と同じ domain)、または invalid group が argv prefix に match。
    if tied
        || input
            .routes
            .invalid
            .iter()
            .any(|g| is_prefix(&g.command, &out.argv))
    {
        return Viability::InvalidRoute;
    }
    // missing-target — 解決済み target.path の存在/load 確認。
    if !Path::new(&route.target_path).is_file() {
        return Viability::MissingTarget;
    }
    // no-grant — `command::execute` の事前 check 全通過が必須
    // (routing≠authorization、D2)。`check_exec_execution` と
    // 同一 predicate で、exec_cwd=payload cwd・accept_any_exit
    // の routed scope 前提で評価する。route eligibility —
    // `scope==trusted && !fixed` の grant のみ route 対象
    // (`fixed`・`scope:"project"`・未知名は全て NoGrant —
    // `is_route_eligible` が単一 seam、design §(j))。実行ペアは
    // `exec_pair` が選ぶ: byte-matched は {route.grant, 原 exec_args}、
    // substitute mode は {substitute.grant, substitute.argv}(O2a/R3)。
    // substitute argv は route 解決時点で確定済みの宣言値であり member の
    // 「job 実行時へ遅延」とは質的が異なる — 「宣言通り実行する
    // command」への完全検査が意味を持つ。
    let (exec_name, exec_args) = route.exec_pair(&out.exec_args);
    let Some(grant) = input
        .exec_policy
        .policy
        .grants
        .get(exec_name)
        .filter(|g| ResolvedExecPolicy::is_route_eligible(g))
    else {
        return Viability::NoGrant;
    };
    let Ok(pinned) = cheaper_tools::tools::exec::pin_exec_grant(
        exec_name,
        grant,
        input.exec_policy.root,
        input.exec_policy.symlink_policy,
        input.exec_policy.denylist,
        input.exec_policy.exec_deny,
    ) else {
        return Viability::NoGrant;
    };
    let options = ExecToolOptions {
        root: input.exec_policy.root.to_string(),
        symlink_policy: input.exec_policy.symlink_policy,
        exec: input.exec_policy.policy.clone(),
        pins: std::collections::BTreeMap::from([(exec_name.to_string(), pinned.clone())]),
        on_timeout: None,
        denylist: input.exec_policy.denylist.cloned(),
        exec_deny: input.exec_policy.exec_deny.clone(),
        write_scope: None,
        route_context: Some(RouteCommandContext {
            exec_cwd: Some(input.cwd.to_string()),
            accept_any_exit_code: true,
            ..RouteCommandContext::default()
        }),
    };
    if check_exec_execution(&options, exec_name, exec_args).is_err() {
        return Viability::NoGrant;
    }
    // no-grant (b) — member 級検査: 存在 + is_route_eligible +
    // pin 解決のみ(argv 照合は job 実行時の check_exec_execution へ
    // 遅延 — direct `workflow run --grant` と同じ検査位置、O2a)。
    // substitute mode では `route.grant` も member 級へ降格する(R4 —
    // firing gate/attribution anchor/投影 member の役割は残るが実行
    // 対象ではない。原 argv への `route.grant` argv 照合は行わない —
    // 実行されない command の形骸化検査であり、「認可したくない
    // command こそ置き換えたい」用途を壊す)。失敗は anchor 失敗と
    // 同じ NoGrant へ fold する。
    let demoted = route.substitute.is_some().then_some(&route.grant);
    for member in demoted.into_iter().chain(&route.grants) {
        let Some(member_grant) = input
            .exec_policy
            .policy
            .grants
            .get(member)
            .filter(|g| ResolvedExecPolicy::is_route_eligible(g))
        else {
            tracing::debug!(route = %route.name, member, "route grant member ineligible");
            return Viability::NoGrant;
        };
        if cheaper_tools::tools::exec::pin_exec_grant(
            member,
            member_grant,
            input.exec_policy.root,
            input.exec_policy.symlink_policy,
            input.exec_policy.denylist,
            input.exec_policy.exec_deny,
        )
        .is_err()
        {
            tracing::debug!(route = %route.name, member, "route grant member unpinnable");
            return Viability::NoGrant;
        }
    }
    // stdin-unsupported — interactive command curated list(typed 側
    // invariant 維持: interactive な typed command は substitute
    // 宣言の有無に関わらず routing で置き換えない)。
    if is_interactive(name, &out.exec_args) {
        return Viability::StdinUnsupported;
    }
    // substitute 側 — stdin 非接続で実際に spawn されるのは
    // substitute ペアであり、interactive な substitute は発火後に
    // grant timeout まで hang してから失敗する。分類名は宣言
    // executable 名と pin 解決済み実体名の union — 片側だけだと
    // canonical 名ずらし(`python3`→`python3.11`・`vim`→`vim.basic`・
    // `vi`→`nvi`)や shim 名で系統的に回避される(O6a/R10)。
    if let Some(substitute) = &route.substitute {
        if is_interactive(&command_name(&grant.executable), &substitute.argv)
            || is_interactive(&command_name(&pinned), &substitute.argv)
        {
            return Viability::StdinUnsupported;
        }
    }
    Viability::Runnable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::routes::{
        EffectiveRoutes, InvalidRouteGroup, Route, RouteLayer, RouteSubstitute, SuppressedRoute,
    };
    use cheaper_core::policy::exec::resolve_exec_policy;
    use cheaper_core::policy::security::ExecDenyList;
    use serde_json::json;

    fn route(name: &str, command: &[&str], grant: &str, target: &str) -> Route {
        Route {
            name: name.to_string(),
            command: command.iter().map(|s| s.to_string()).collect(),
            target_path: std::path::PathBuf::from(target),
            grant: grant.to_string(),
            grants: Vec::new(),
            substitute: None,
            token_threshold: None,
            passthrough_env: Vec::new(),
            trusted: false,
            layer: RouteLayer::Builtin,
        }
    }

    /// `grants` member を持つ route 宣言 — member 検査は
    /// `route.grants`(anchor を含まない member のみ)を走査する。
    fn route_with_grants(
        name: &str,
        command: &[&str],
        grant: &str,
        grants: &[&str],
        target: &str,
    ) -> Route {
        Route {
            grants: grants.iter().map(|s| s.to_string()).collect(),
            ..route(name, command, grant, target)
        }
    }

    fn policy_view<'a>(
        policy: &'a ResolvedExecPolicy,
        root: &'a str,
        exec_deny: &'a ExecDenyList,
    ) -> ExecPolicyView<'a> {
        ExecPolicyView {
            policy,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            denylist: None,
            exec_deny,
        }
    }

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    fn grant_policy() -> ResolvedExecPolicy {
        // pin 解決可能な executable(test binary)で grant を宣言し、
        // `check_exec_execution` が受理する形を作る。exec-doc
        // shape(`{"grants":{…}}`)を `resolve_exec_policy` へ
        // 直接渡す。
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {
                        "executable": exe,
                        "argv": ["test", "*"],
                    }
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap()
    }

    #[test]
    fn self_bypass_and_env_prefix_strip() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let routes = EffectiveRoutes {
            routes: vec![route(
                "cargo-test",
                &["cargo", "test"],
                "cargo-test",
                &target.to_string_lossy(),
            )],
            ..Default::default()
        };
        let policy = grant_policy();
        let deny = ExecDenyList::default();
        let view = policy_view(&policy, &root, &deny);
        // self-bypass — cheaper argv[0] は match しない。
        let input = ResolveInput {
            argv: &argv(&["cheaper", "run", "w"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: false,
        };
        assert!(!resolve(&input).matched);
        // env-prefix は strip されて match/eval に入る。
        let input = ResolveInput {
            argv: &argv(&["CI=1", "cargo", "test", "--workspace"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: false,
        };
        let out = resolve(&input);
        assert!(out.matched, "expected matched: {out:?}");
        assert_eq!(out.route.as_deref(), Some("cargo-test"));
        assert_eq!(out.argv, argv(&["cargo", "test", "--workspace"]));
        // args は post-prefix(R4)、exec_args は argv[1..](C1)。
        assert_eq!(out.args, argv(&["--workspace"]));
        assert_eq!(out.exec_args, argv(&["test", "--workspace"]));
        // grant argv ["test","*"] と exec_args が適合し runnable。
        assert_eq!(out.viability, Some(Viability::Runnable));
    }

    #[test]
    fn viability_order_and_branches() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let policy = grant_policy();
        let deny = ExecDenyList::default();
        let view = policy_view(&policy, &root, &deny);
        let run = |routes: &EffectiveRoutes, tokens: &[&str]| {
            resolve(&ResolveInput {
                argv: &argv(tokens),
                cwd: &root,
                routes,
                exec_policy: &view,
                unsafe_command: false,
            })
        };
        // unsafe-command — caller 報告 flag と cwd 変化 executable
        // の 2 条件のみ(#166 — tokenize 済み argv token の metachar
        // 内容は分類しない、D2)。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "cargo-test", &target_str)],
            ..Default::default()
        };
        let out = resolve(&ResolveInput {
            argv: &argv(&["cargo", "test", "a;rm"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: true,
        });
        assert_eq!(out.viability, Some(Viability::UnsafeCommand));
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cd"], "cargo-test", &target_str)],
            ..Default::default()
        };
        let out = run(&routes, &["cd", "sub"]);
        assert_eq!(out.viability, Some(Viability::UnsafeCommand));
        // quoted 相当の metachar 含 token(`a;rm`)は unsafe でなく
        // grant argv pattern 照合まで到達し runnable になり得る
        // (#166 R5 — token 内容の shell-metachar 正規表現分類は
        // viability から除去済み)。`\0` 含 arg も到達する
        // (spawn 時の bounded job error — O5b 不採用の帰結)。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "cargo-test", &target_str)],
            ..Default::default()
        };
        let out = run(&routes, &["cargo", "test", "a;rm"]);
        assert_eq!(out.viability, Some(Viability::Runnable));
        let out = run(&routes, &["cargo", "test", "a\0b"]);
        assert_eq!(out.viability, Some(Viability::Runnable));
        // invalid-route — invalid group が argv prefix に match。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "cargo-test", &target_str)],
            invalid: vec![InvalidRouteGroup {
                command: argv(&["cargo"]),
                names: vec!["a".into(), "b".into()],
                layer: RouteLayer::Project,
            }],
            ..Default::default()
        };
        let out = run(&routes, &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::InvalidRoute));
        // missing-target — target file 不在。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "cargo-test", "/nonexistent/w.json")],
            ..Default::default()
        };
        let out = run(&routes, &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::MissingTarget));
        // no-grant — grant 名が policy に無い。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "absent-grant", &target_str)],
            ..Default::default()
        };
        let out = run(&routes, &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // stdin-unsupported — interactive executable(bash)。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["bash"], "cargo-test", &target_str)],
            ..Default::default()
        };
        // exec_args が grant pattern に適合する形で stdin-unsupported
        // を単独検証する(no-grant が優先されるため)。
        let out = run(&routes, &["bash", "test", "-c", "x"]);
        assert_eq!(out.viability, Some(Viability::StdinUnsupported));
        // suppressed provenance — effective 無しで suppressed match。
        let routes = EffectiveRoutes {
            suppressed: vec![SuppressedRoute {
                name: "p-route".into(),
                command: argv(&["cargo"]),
            }],
            ..Default::default()
        };
        let out = run(&routes, &["cargo", "test"]);
        assert!(!out.matched);
        assert_eq!(out.suppressed.as_deref(), Some("p-route"));
    }

    /// #166 member viability — `grants` member の検査は 存在 +
    /// `is_route_eligible` + `pin_exec_grant` のみで、失敗は anchor
    /// 失敗と同じ `NoGrant` へ fold する(O2a/O3a — member への
    /// `check_exec_execution`・cwd prepare は呼ばない。新規
    /// viability 値なし)。
    #[test]
    fn member_grants_fold_into_no_grant() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let deny = ExecDenyList::default();
        let run = |policy: &ResolvedExecPolicy, members: &[&str], tokens: &[&str]| {
            let view = policy_view(policy, &root, &deny);
            resolve(&ResolveInput {
                argv: &argv(tokens),
                cwd: &root,
                routes: &EffectiveRoutes {
                    routes: vec![route_with_grants(
                        "r",
                        &["cargo", "test"],
                        "cargo-test",
                        members,
                        &target_str,
                    )],
                    ..Default::default()
                },
                exec_policy: &view,
                unsafe_command: false,
            })
        };
        let anchor = json!({"executable": exe, "argv": ["test", "*"]});
        // member 全通過 → Runnable(anchor `cargo-test` + member
        // `fmt` ともに pin 可能な test binary を指す)。
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": anchor.clone(),
                    "fmt": {"executable": exe, "argv": ["*"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(&policy, &["fmt"], &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::Runnable));
        // member 未存在 → NoGrant。
        let out = run(&policy, &["missing-member"], &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // member scope:"project" → NoGrant。
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": anchor.clone(),
                    "scoped": {"executable": exe, "argv": ["*"], "scope": "project"},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(&policy, &["scoped"], &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // member fixed(task 由来)→ NoGrant。
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": anchor.clone(),
                    "taskish": {"executable": exe, "argv": ["x"], "fixed": true},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(&policy, &["taskish"], &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // member pin 不能(unresolvable executable)→ NoGrant。
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": anchor.clone(),
                    "unpinnable": {"executable": "./does-not-exist-xyz", "argv": ["*"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(&policy, &["unpinnable"], &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // member 失敗は stdin-unsupported より先に fold される
        // (member ループは stdin 分岐より前)。
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": anchor.clone(),
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let routes = EffectiveRoutes {
            routes: vec![route_with_grants(
                "r",
                &["bash"],
                "cargo-test",
                &["missing-member"],
                &target_str,
            )],
            ..Default::default()
        };
        let view = policy_view(&policy, &root, &deny);
        let out = resolve(&ResolveInput {
            argv: &argv(&["bash", "test", "-c", "x"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: false,
        });
        assert_eq!(out.viability, Some(Viability::NoGrant));
    }

    #[test]
    fn longest_match_wins() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let policy = grant_policy();
        let deny = ExecDenyList::default();
        let view = policy_view(&policy, &root, &deny);
        let routes = EffectiveRoutes {
            routes: vec![
                route("short", &["cargo"], "cargo-test", &target_str),
                route("long", &["cargo", "test"], "cargo-test", &target_str),
            ],
            ..Default::default()
        };
        let out = resolve(&ResolveInput {
            argv: &argv(&["cargo", "test", "-p", "x"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: false,
        });
        assert_eq!(out.route.as_deref(), Some("long"));
        assert_eq!(out.args, argv(&["-p", "x"]));
    }

    /// `security.execDenyPatterns`(issue #164)— grant の declared
    /// 名は deny を潜るが resolved basename が match する場合、
    /// viability は error ではなく `Viability::NoGrant` に畳まれる
    /// (unpinnable grant と同じ分類 — routing≠authorization)。
    /// declared ≠ resolved は symlink のみで構成できるため unix
    /// gate(declared match は `load_policy` 段階で全体失敗し
    /// viability へ到達しない)。
    #[cfg(unix)]
    #[test]
    fn denied_resolved_grant_is_no_grant() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        // 実体 `deny-me` ← symlink `helper`(declared)。
        let bin = dir.path().join("deny-me");
        std::fs::copy(std::env::current_exe().unwrap(), &bin).unwrap();
        let link = dir.path().join("helper");
        std::os::unix::fs::symlink(&bin, &link).unwrap();
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {"helper": {"executable": "./helper", "argv": ["*"]}}
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let deny = ExecDenyList::compile(&["deny-me".to_string()]).unwrap();
        let view = policy_view(&policy, &root, &deny);
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["helper"], "helper", &target_str)],
            ..Default::default()
        };
        let out = resolve(&ResolveInput {
            argv: &argv(&["helper", "x"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: false,
        });
        assert!(out.matched);
        assert_eq!(out.viability, Some(Viability::NoGrant));
    }

    fn sub(grant: &str, args: &[&str]) -> RouteSubstitute {
        RouteSubstitute {
            grant: grant.to_string(),
            argv: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// substitute mode の viability retarget(#167 O2a/R3-R4)—
    /// anchor 完全検査の対象は `{substitute.grant, substitute.argv}`
    /// であり、原 argv の `route.grant` pattern 照合は行わない
    /// (実行されない command の形骸化検査の除去)。`route.grant` は
    /// member 級(存在 + `is_route_eligible` + pin)へ降格し、
    /// substitute 側・member 級 `route.grant`・`grants[]` member の
    /// 失敗は全て `NoGrant` 一種へ fold する(新規 viability 値なし)。
    #[test]
    fn substitute_retargets_viability_and_demotes_anchor() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let deny = ExecDenyList::default();
        let run = |policy: &ResolvedExecPolicy, r: Route, tokens: &[&str]| {
            let view = policy_view(policy, &root, &deny);
            resolve(&ResolveInput {
                argv: &argv(tokens),
                cwd: &root,
                routes: &EffectiveRoutes {
                    routes: vec![r],
                    ..Default::default()
                },
                exec_policy: &view,
                unsafe_command: false,
            })
        };
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                    "dry-run": {"executable": exe, "argv": ["push", "--dry-run"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let sub_route = |grant: &str, args: &[&str]| {
            let mut r = route("r", &["git", "push"], "cargo-test", &target_str);
            r.substitute = Some(sub(grant, args));
            r
        };
        // retarget — 原 exec_args `["push","origin","main"]` は
        // `route.grant` pattern `["test","*"]` に一致しないが、
        // substitute ペア `{dry-run, [push,--dry-run]}` が健全なら
        // Runnable(AC4 — 原 argv 照合を行わない)。
        let out = run(
            &policy,
            sub_route("dry-run", &["push", "--dry-run"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::Runnable));
        // `RouteResolution` shape 不変 — `grant` は `route.grant`
        // anchor echo で substitute echo は載らない(R11)。
        assert_eq!(out.grant.as_deref(), Some("cargo-test"));
        assert_eq!(out.exec_args, argv(&["push", "origin", "main"]));
        // substitute argv が grant pattern 不一致 → NoGrant(AC3)。
        let out = run(
            &policy,
            sub_route("dry-run", &["push", "--force"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // substitute.grant 未存在 → NoGrant。
        let out = run(
            &policy,
            sub_route("absent", &["push", "--dry-run"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // substitute.grant scope:"project" → NoGrant。
        let scoped = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                    "scoped": {"executable": exe, "argv": ["*"], "scope": "project"},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(
            &scoped,
            sub_route("scoped", &["push", "--dry-run"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // substitute.grant fixed → NoGrant。
        let fixed = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                    "fixed": {"executable": exe, "argv": ["*"], "fixed": true},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(
            &fixed,
            sub_route("fixed", &["push", "--dry-run"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // substitute.grant pin 不能 → NoGrant。
        let unpinnable = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                    "unpinnable": {"executable": "./does-not-exist-xyz", "argv": ["*"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(
            &unpinnable,
            sub_route("unpinnable", &["push", "--dry-run"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // member 級降格 — `route.grant` 未存在 → NoGrant(substitute
        // ペアは健全でも member 検査で落ちる)。
        let out = run(
            &policy,
            {
                let mut r = sub_route("dry-run", &["push", "--dry-run"]);
                r.grant = "absent-anchor".to_string();
                r
            },
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // `route.grant` ineligible(scope:"project")→ NoGrant。
        let scoped_anchor = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"], "scope": "project"},
                    "dry-run": {"executable": exe, "argv": ["push", "--dry-run"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(
            &scoped_anchor,
            sub_route("dry-run", &["push", "--dry-run"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // `route.grant` pin 不能 → NoGrant(member 級 — argv 照合は
        // 行わず存在 + eligibility + pin のみ)。
        let unpinnable_anchor = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": "./does-not-exist-xyz", "argv": ["test", "*"]},
                    "dry-run": {"executable": exe, "argv": ["push", "--dry-run"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(
            &unpinnable_anchor,
            sub_route("dry-run", &["push", "--dry-run"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // `grants[]` member 失敗も substitute mode で NoGrant。
        let out = run(
            &policy,
            {
                let mut r = sub_route("dry-run", &["push", "--dry-run"]);
                r.grants = vec!["absent-member".to_string()];
                r
            },
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::NoGrant));
    }

    /// stdin-unsupported の両側分類(#167 O6a/R10)— typed 側は
    /// substitute 宣言の有無に関わらず interactive なら不発
    /// (「interactive typed command は routing で置き換えない」
    /// invariant)。substitute 側は宣言 executable 名と pin 解決済み
    /// 実体名の union + `substitute.argv` の interactive flag で
    /// 分類する。
    #[test]
    fn substitute_interactive_union() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let deny = ExecDenyList::default();
        // `native()` は win32 で `.exe`/`.com` suffix を要求するため
        // fixture 名は platform で分ける(どちらでも command_name が
        // dir+ext を剥がし basename は同じ)。`std::fs::copy` は +x
        // bit を保持する(unix の X_OK 判定対応)。
        #[cfg(windows)]
        let (vim, docker) = ("vim.exe", "docker.exe");
        #[cfg(not(windows))]
        let (vim, docker) = ("vim", "docker");
        std::fs::copy(std::env::current_exe().unwrap(), dir.path().join(vim)).unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), dir.path().join(docker)).unwrap();
        let run = |policy: &ResolvedExecPolicy, r: Route, tokens: &[&str]| {
            let view = policy_view(policy, &root, &deny);
            resolve(&ResolveInput {
                argv: &argv(tokens),
                cwd: &root,
                routes: &EffectiveRoutes {
                    routes: vec![r],
                    ..Default::default()
                },
                exec_policy: &view,
                unsafe_command: false,
            })
        };
        let sub_route = |grant: &str, args: &[&str]| {
            let mut r = route("r", &["git", "push"], "cargo-test", &target_str);
            r.substitute = Some(sub(grant, args));
            r
        };
        // interactive executable を pin する substitute.grant →
        // StdinUnsupported(宣言名側 — resolve 名も同じ basename)。
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                    "vi-sub": {"executable": format!("./{vim}"), "argv": ["*"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(
            &policy,
            sub_route("vi-sub", &["x"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::StdinUnsupported));
        // `substitute.argv` の interactive flag(docker + `-it`)→
        // StdinUnsupported。非 flag argv なら substitute 側は非該当。
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                    "docker-sub": {"executable": format!("./{docker}"), "argv": ["*"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let out = run(
            &policy,
            sub_route("docker-sub", &["run", "-it", "img"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::StdinUnsupported));
        let out = run(
            &policy,
            sub_route("docker-sub", &["ps"]),
            &["git", "push", "origin", "main"],
        );
        assert_eq!(out.viability, Some(Viability::Runnable));
        // typed 側 interactive — substitute 宣言があっても
        // `bash` typed command は不発(invariant 維持)。
        let mut r = route("r", &["bash"], "cargo-test", &target_str);
        r.substitute = Some(sub("docker-sub", &["ps"]));
        let out = run(&policy, r, &["bash", "test", "-c", "x"]);
        assert_eq!(out.viability, Some(Viability::StdinUnsupported));
    }

    /// canonical 名ずらし・shim 名の union 両側 pin(#167 AC9 —
    /// declared ≠ resolved は symlink のみで構成できるため unix
    /// gate、`SymlinkPolicy::WithinWorkspace` で pin 解決を通す)。
    /// (a) 宣言名 `python3` が list 内・解決先 `python3.11` が list
    /// 外でも宣言名側で `StdinUnsupported`、(b) 宣言名 `shim-python`
    /// が list 外でも解決済み実体名 `python3` が list 内なら解決
    /// 名側で `StdinUnsupported`。
    #[cfg(unix)]
    #[test]
    fn substitute_interactive_union_canonical_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let deny = ExecDenyList::default();
        // (a) 用: 宣言 `python3` → 実体 `python3.11`。
        let real_versioned = dir.path().join("python3.11");
        std::fs::copy(std::env::current_exe().unwrap(), &real_versioned).unwrap();
        std::os::unix::fs::symlink(&real_versioned, dir.path().join("python3")).unwrap();
        // (b) 用: 宣言 `shim-python` → 実体 `real-bins/python3`。
        let real_bins = dir.path().join("real-bins");
        std::fs::create_dir(&real_bins).unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), real_bins.join("python3")).unwrap();
        std::os::unix::fs::symlink(real_bins.join("python3"), dir.path().join("shim-python"))
            .unwrap();
        // 対照用: 宣言 `mypython` → 実体 `python3-real`(どちらも
        // list 外)。
        let real_plain = dir.path().join("python3-real");
        std::fs::copy(std::env::current_exe().unwrap(), &real_plain).unwrap();
        std::os::unix::fs::symlink(&real_plain, dir.path().join("mypython")).unwrap();
        let run = |policy: &ResolvedExecPolicy, r: Route| {
            let view = ExecPolicyView {
                policy,
                root: &root,
                symlink_policy: SymlinkPolicy::WithinWorkspace,
                denylist: None,
                exec_deny: &deny,
            };
            resolve(&ResolveInput {
                argv: &argv(&["git", "push", "origin", "main"]),
                cwd: &root,
                routes: &EffectiveRoutes {
                    routes: vec![r],
                    ..Default::default()
                },
                exec_policy: &view,
                unsafe_command: false,
            })
        };
        let sub_route = |grant: &str| {
            let mut r = route("r", &["git", "push"], "cargo-test", &target_str);
            r.substitute = Some(sub(grant, &["x"]));
            r
        };
        let policy = |name: &str, sub_exe: &str| {
            resolve_exec_policy(
                Some(&json!({
                    "grants": {
                        "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                        name: {"executable": sub_exe, "argv": ["*"]},
                    }
                })),
                &ExecDenyList::default(),
            )
            .unwrap()
        };
        // (a) 宣言 `python3`(list 内)→ 解決 `python3.11`(list 外)。
        let p = policy("versioned", "./python3");
        assert_eq!(
            run(&p, sub_route("versioned")).viability,
            Some(Viability::StdinUnsupported)
        );
        // (b) 宣言 `shim-python`(list 外)→ 解決 `python3`(list 内)。
        let p = policy("shimmed", "./shim-python");
        assert_eq!(
            run(&p, sub_route("shimmed")).viability,
            Some(Viability::StdinUnsupported)
        );
        // 非 interactive 宣言+解決は Runnable(union 非該当の対照)。
        let p = policy("plain", "./mypython");
        assert_eq!(
            run(&p, sub_route("plain")).viability,
            Some(Viability::Runnable)
        );
    }

    /// match precedence interplay(known semantics — design §Risks
    /// review P1-1)— longest `match.command` の best 1 件のみが
    /// viability 評価され、non-runnable best から shorter match への
    /// fall-through はない。
    #[test]
    fn non_runnable_best_does_not_fall_back_to_shorter() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let deny = ExecDenyList::default();
        // shorter `[git,push]` は健全な substitute route、longer
        // `[git,push,origin]` は ineligible な `grant:` で NoGrant。
        let mut short = route("short", &["git", "push"], "cargo-test", &target_str);
        short.substitute = Some(sub("dry-run", &["push", "--dry-run"]));
        let long = route(
            "long",
            &["git", "push", "origin"],
            "absent-grant",
            &target_str,
        );
        let routes = EffectiveRoutes {
            routes: vec![short, long],
            ..Default::default()
        };
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "cargo-test": {"executable": exe, "argv": ["test", "*"]},
                    "dry-run": {"executable": exe, "argv": ["push", "--dry-run"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let view = policy_view(&policy, &root, &deny);
        let out = resolve(&ResolveInput {
            argv: &argv(&["git", "push", "origin", "main"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: false,
        });
        // best(longer)のみ評価 — shorter substitute route へ
        // fall-through しない。
        assert_eq!(out.route.as_deref(), Some("long"));
        assert_eq!(out.viability, Some(Viability::NoGrant));
    }
}
