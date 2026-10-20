//! `route::resolve` — argv → route match → viability の純粋評価
//! 入口(design §RouteResolver、R4-R12)。hook eval・`route
//! resolve`・`route run` の 3 caller が共有する唯一の resolve
//! 実装。file I/O・policy load は caller 責務(`ResolveInput` で
//! 済み分を受ける)で、load 失敗は caller が `invalid-route`/
//! `no-grant` viability へ写像する(fail-safe、R39)。

use std::path::Path;

use cheaper_core::policy::command::ResolvedCommandPolicy;
use cheaper_core::policy::process::argv_shell_re;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_tools::tools::command::{
    check_command_execution, CommandToolOptions, RouteCommandContext,
};

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
    /// effective command policy(`load_policy` resolved)の参照面。
    pub command_policy: &'a CommandPolicyView<'a>,
    /// 入力 command が複数 shell segment・unsafe segment/token を
    /// 含む場合に caller が立てる flag(`route resolve --command`
    /// の tokenize 結果)。hook eval は segment 安全性を resolve
    /// 前に評価済みのため `false` を渡す。
    pub unsafe_command: bool,
}

/// viability の grant-match/preflight が参照する command policy
/// 環境 — `check_command_execution` と同一 predicate で評価する
/// ため `CommandToolOptions` 構築に必要な面だけを束ねる。
#[derive(Clone, Copy)]
pub struct CommandPolicyView<'a> {
    pub policy: &'a ResolvedCommandPolicy,
    /// pin 解決・`prepare_cwd` の workspace root(通常 resolve
    /// input の `cwd` と同じ値)。
    pub root: &'a str,
    pub symlink_policy: SymlinkPolicy,
    pub denylist: Option<&'a PathDenylist>,
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
    // executable が cwd 変化 command、または exec_args に shell
    // metachar(tokenize 済み argv でも grant 側は metachar を
    // 拒否するため安全性分類へ倒す)。
    let name = out.executable.as_deref().unwrap_or_default();
    if input.unsafe_command
        || CWD_CHANGING_COMMANDS.contains(&name.to_lowercase().as_str())
        || out.exec_args.iter().any(|a| argv_shell_re().is_match(a))
    {
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
    // (routing≠authorization、D2)。`check_command_execution` と
    // 同一 predicate で、exec_cwd=payload cwd・accept_any_exit
    // の routed scope 前提で評価する。
    let Some(grant) = input.command_policy.policy.grants.get(&route.grant) else {
        return Viability::NoGrant;
    };
    let Ok(pinned) = cheaper_tools::tools::command::pin_command_grant(
        &route.grant,
        grant,
        input.command_policy.root,
        input.command_policy.symlink_policy,
        input.command_policy.denylist,
    ) else {
        return Viability::NoGrant;
    };
    let options = CommandToolOptions {
        root: input.command_policy.root.to_string(),
        symlink_policy: input.command_policy.symlink_policy,
        command: input.command_policy.policy.clone(),
        pins: std::collections::BTreeMap::from([(route.grant.clone(), pinned)]),
        on_timeout: None,
        denylist: input.command_policy.denylist.cloned(),
        route_context: Some(RouteCommandContext {
            exec_cwd: Some(input.cwd.to_string()),
            accept_any_exit_code: true,
            ..RouteCommandContext::default()
        }),
    };
    if check_command_execution(&options, &route.grant, &out.exec_args).is_err() {
        return Viability::NoGrant;
    }
    // stdin-unsupported — interactive command curated list。
    if is_interactive(name, &out.exec_args) {
        return Viability::StdinUnsupported;
    }
    Viability::Runnable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route::routes::{
        EffectiveRoutes, InvalidRouteGroup, Route, RouteLayer, SuppressedRoute,
    };
    use cheaper_core::policy::command::resolve_command_policy;
    use serde_json::json;

    fn route(name: &str, command: &[&str], grant: &str, target: &str) -> Route {
        Route {
            name: name.to_string(),
            command: command.iter().map(|s| s.to_string()).collect(),
            target_path: std::path::PathBuf::from(target),
            grant: grant.to_string(),
            token_threshold: None,
            passthrough_env: Vec::new(),
            trusted: false,
            layer: RouteLayer::Builtin,
        }
    }

    fn policy_view<'a>(policy: &'a ResolvedCommandPolicy, root: &'a str) -> CommandPolicyView<'a> {
        CommandPolicyView {
            policy,
            root,
            symlink_policy: SymlinkPolicy::Deny,
            denylist: None,
        }
    }

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    fn grant_policy() -> ResolvedCommandPolicy {
        // pin 解決可能な executable(test binary)で grant を宣言し、
        // `check_command_execution` が受理する形を作る。
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        resolve_command_policy(Some(&json!({
            "grants": {
                "cargo-test": {
                    "executable": exe,
                    "argv": ["test", "*"],
                }
            }
        })))
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
        let view = policy_view(&policy, &root);
        // self-bypass — cheaper argv[0] は match しない。
        let input = ResolveInput {
            argv: &argv(&["cheaper", "run", "w"]),
            cwd: &root,
            routes: &routes,
            command_policy: &view,
            unsafe_command: false,
        };
        assert!(!resolve(&input).matched);
        // env-prefix は strip されて match/eval に入る。
        let input = ResolveInput {
            argv: &argv(&["CI=1", "cargo", "test", "--workspace"]),
            cwd: &root,
            routes: &routes,
            command_policy: &view,
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
        let view = policy_view(&policy, &root);
        let run = |routes: EffectiveRoutes, tokens: &[&str]| {
            resolve(&ResolveInput {
                argv: &argv(tokens),
                cwd: &root,
                routes: &routes,
                command_policy: &view,
                unsafe_command: false,
            })
        };
        // unsafe-command — exec_args の metachar。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "cargo-test", &target_str)],
            ..Default::default()
        };
        let out = run(routes, &["cargo", "test", "a;rm"]);
        assert_eq!(out.viability, Some(Viability::UnsafeCommand));
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
        let out = run(routes, &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::InvalidRoute));
        // missing-target — target file 不在。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "cargo-test", "/nonexistent/w.json")],
            ..Default::default()
        };
        let out = run(routes, &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::MissingTarget));
        // no-grant — grant 名が policy に無い。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["cargo"], "absent-grant", &target_str)],
            ..Default::default()
        };
        let out = run(routes, &["cargo", "test"]);
        assert_eq!(out.viability, Some(Viability::NoGrant));
        // stdin-unsupported — interactive executable(bash)。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["bash"], "cargo-test", &target_str)],
            ..Default::default()
        };
        // exec_args が grant pattern に適合する形で stdin-unsupported
        // を単独検証する(no-grant が優先されるため)。
        let out = run(routes, &["bash", "test", "-c", "x"]);
        assert_eq!(out.viability, Some(Viability::StdinUnsupported));
        // suppressed provenance — effective 無しで suppressed match。
        let routes = EffectiveRoutes {
            suppressed: vec![SuppressedRoute {
                name: "p-route".into(),
                command: argv(&["cargo"]),
            }],
            ..Default::default()
        };
        let out = run(routes, &["cargo", "test"]);
        assert!(!out.matched);
        assert_eq!(out.suppressed.as_deref(), Some("p-route"));
    }

    #[test]
    fn longest_match_wins() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let policy = grant_policy();
        let view = policy_view(&policy, &root);
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
            command_policy: &view,
            unsafe_command: false,
        });
        assert_eq!(out.route.as_deref(), Some("long"));
        assert_eq!(out.args, argv(&["-p", "x"]));
    }
}
