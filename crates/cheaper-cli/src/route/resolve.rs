//! `route::resolve` — argv → route match → viability の純粋評価
//! 入口(design §RouteResolver、R4-R12)。hook eval・`route
//! resolve`・`route run` の 3 caller が共有する唯一の resolve
//! 実装。file I/O・policy load は caller 責務(`ResolveInput` で
//! 済み分を受ける)で、load 失敗は caller が `invalid-route`/
//! `no-grant` viability へ写像する(fail-safe、R39)。

use cheaper_core::policy::exec::ResolvedExecPolicy;
use cheaper_core::policy::security::ExecDenyList;
use cheaper_core::workspace::deny_paths::PathDenylist;
use cheaper_core::workspace::paths::SymlinkPolicy;
use cheaper_tools::tools::exec::{check_exec_execution, ExecToolOptions, RouteCommandContext};

use super::routes::{EffectiveRoutes, Route};
use crate::hook::routing_policy::{command_name, is_env_prefix, CWD_CHANGING_COMMANDS};

/// viability の 1 値(R12 の固定優先順に従い最初に成立した分類)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viability {
    Runnable,
    /// stdin 非接続でも実行可能だが、interactive intent・inline
    /// code・特権/remote 副作用を持つため承認なしには置き換えない
    /// (R33bis — 構造的に実行不能な `stdin-unsupported` とは別階層)。
    /// hook は Rewrite を emit するが bypass-不可(`permissionDecision:
    /// "allow"` 併記は行わない)ため、host 側の承認 prompt が残る。
    ApprovalRequired,
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
            Self::ApprovalRequired => "approval-required",
            Self::NoGrant => "no-grant",
            Self::MissingTarget => "missing-target",
            Self::InvalidRoute => "invalid-route",
            Self::UnsafeCommand => "unsafe-command",
            Self::StdinUnsupported => "stdin-unsupported",
        }
    }

    /// hook の route arm が Rewrite を emit しうる viability。
    /// `route run` 側の実行受理もこの集合で判定する。
    pub fn is_executable(self) -> bool {
        matches!(self, Self::Runnable | Self::ApprovalRequired)
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
    /// merge 敗北 route 名(provenance 報告)— 同一 `match.command`
    /// で下位層に負けた project route(#168 で displaced 記録は
    /// project shadow 廃止とともに除去)。
    pub suppressed: Option<String>,
    /// match した route の `passthroughEnv` 宣言 — payload encode
    /// の ambient env allowlist(`route run` の再 resolve も同じ値
    /// を使う)。no-match では空。
    pub passthrough_env: Vec<String>,
}

/// `match_command` が評価した effective route 1 行の row-level
/// sample 状態。`matched` は宣言 `match.command` が normalized argv
/// に prefix match すること、`selected` は longest-prefix winner
/// であることを表す。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct RouteCommandMatchRow<'a> {
    pub(crate) route: &'a Route,
    pub(crate) matched: bool,
    pub(crate) selected: bool,
}

/// `resolve` の viability より前の shared match phase。route check は
/// `matched_command` を effective/non-effective row の declaration に
/// 適用し、`is_selected` を effective row へ適用する。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct RouteCommandMatch<'a> {
    /// env-prefix strip 後の normalized argv。
    pub(crate) argv: Vec<String>,
    pub(crate) executable: Option<String>,
    /// effective route 全行の `matched`/`selected` 状態。
    pub(crate) rows: Vec<RouteCommandMatchRow<'a>>,
    pub(crate) selected: Option<&'a Route>,
    pub(crate) tied: bool,
    /// `RouteResolution.suppressed` と同じ route-name 値。
    pub(crate) suppressed: Option<String>,
    /// self-bypass(`cheaper ...`)では route declaration を評価しない。
    self_bypassed: bool,
}

#[allow(dead_code)]
impl RouteCommandMatch<'_> {
    /// 任意の route 宣言 command(effective/suppressed/shadowed/
    /// invalid row を問わない)が normalized argv に prefix match
    /// するか。self-bypass と空 argv は declaration を評価しない。
    pub(crate) fn matched_command(&self, command: &[String]) -> bool {
        !self.self_bypassed && !self.argv.is_empty() && is_prefix(command, &self.argv)
    }

    /// effective route が shared phase の longest-prefix winner か。
    /// merge row は `Route` clone を持つため pointer ではなく
    /// name/layer/command の identity で比較する。
    pub(crate) fn is_selected(&self, route: &Route) -> bool {
        self.selected.is_some_and(|selected| {
            selected.name == route.name
                && selected.layer == route.layer
                && selected.command == route.command
        })
    }
}

/// `resolve()` と shared match phase を合わせた結果。`route run`
/// のように `RouteResolution` と selected `Route` の両方が必要な
/// caller は二重に longest-match を組み立てない。
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct RouteResolutionWithMatch<'a> {
    pub(crate) resolution: RouteResolution,
    pub(crate) matched: RouteCommandMatch<'a>,
}

/// `passthroughEnv` 未宣言でも best-effort で routing しない
/// stdin 依存 command の分類(R33 — 網羅性ではなく構造保証補助)。
/// `Approval`: stdin 非接続でも spawn 可能だが interactive intent・
///   inline code・特権/remote 副作用を持つため承認を要する。
/// `Structural`: stdin/TTY なしでは有意に実行できない(UI app・
///   REPL bare・stdin 由来 program) — approval でも救えない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Interaction {
    Batch,
    Approval,
    Structural,
}

/// UI/TTY app — stdin 非接続では表示・対話が成立しないため
/// 常に `Structural`(R33 継承の安全側)。
const UI_STRUCTURAL_COMMANDS: &[&str] = &[
    // editors — `vim.basic` 等の Debian alternatives 名は union の
    // 解決名側で拾えないため別綴りを列挙する
    "vi", "vim", "nvim", "nano", "emacs", "code", "codium", "subl", "view", "gvim", "gview", "rvim",
    "rview", "ex", "eview", "vimdiff", "nvi", // pagers・TTY monitors
    "less", "more", "man", "top", "htop", "watch", // interactive session tools
    "ftp", "sftp", "telnet",
];

/// 特権・remote・detached/scheduled 副作用 command — batch 形状でも
/// 実行を承認へ回す。`xargs`/`parallel` のような arbitrary-command
/// forwarder は argv 内容によらず `Approval`。
const PRIVILEGED_COMMANDS: &[&str] = &[
    // privilege escalation
    "sudo", "su", "doas", "pkexec", "run0", "sudoedit", "pfexec", // remote execution / copy
    "ssh", "scp", "slogin", "rsh", "rlogin", "mosh", "expect",
    "vagrant", // exec-capable sockets / forwarders
    "nc", "ncat", "netcat", "socat", "xargs", "parallel", // detached / scheduled side effects
    "tmux", "screen", "at", "batch", "crontab",
];

/// `python3.11`・`node20` のような version suffix 付き名を base 名
/// として認識する — suffix は `[0-9._-]` 列のみ許容し、
/// `pythonic` のような語の伸長は別名扱い。
fn is_versioned_name(name: &str, base: &str) -> bool {
    name.strip_prefix(base).is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
    })
}

fn name_is(name: &str, bases: &[&str]) -> bool {
    bases
        .iter()
        .any(|b| name == *b || is_versioned_name(name, b))
}

/// REPL/interpreter family の argv 分類規則。scan は flag 1 個で
/// early return せず全 arg の最悪 tier を採る — `-`(stdin program)
/// の `Structural` が `Approval` flag より常に優先される。
struct ReplSpec {
    /// positional arg(script/module/data file)を持つ形状の tier。
    /// `Batch`: script 実行で stdin 非依存。`Approval`: positional
    /// 自体が program 文字列(awk/sed/redis-cli)。`Structural`:
    /// positional があっても対話 session に落ちる(psql/mysql)。
    positional: Interaction,
    /// `-xyz` single-dash cluster 内で `Approval` の letter
    /// (case-sensitive — `python -I` と `-i`・`sh -e` errexit と
    /// eval `-e` は別物)。
    intent_letters: &'static str,
    /// 同・case-insensitive 追加(perl `-e`/`-E`、sqlcmd `-q`/`-Q`)。
    intent_letters_ci: &'static str,
    /// `-`/`--`/`/` prefix を剥がした lowercase 名の完全一致で
    /// `Approval`(node `--eval`・cmd `/c`・pwsh `/Command`)。
    intent_long: &'static [&'static str],
    /// pwsh 型 unambiguous-prefix parameter — arg が canonical 名の
    /// 先頭一致なら `Approval`(`-Com`…`-Command`・`-Enc`…の省略形)。
    intent_prefixes: &'static [&'static str],
    /// 同・prefix 一致で `Structural`(`-Interactive`/`-NoExit` —
    /// script 実行後も TTY に残る)。
    structural_prefixes: &'static [&'static str],
    /// 同・完全一致で `Structural`(docker 系 `--interactive`)。
    structural_long: &'static [&'static str],
    /// `-f FILE`/`--file FILE` 形式で値を取り、以降の positional を
    /// data file として `Batch` 化する script-file flag。
    file_flags: &'static [&'static str],
    /// file_flags の pwsh 型 prefix 版(`-F`…`-File`)。
    file_prefixes: &'static [&'static str],
    /// cluster 内で `Structural` の letter(php `-a`・redis-cli `-x`)。
    structural_letters: &'static str,
    /// positional token 自体が inline program 相当(deno `eval`)。
    approval_positionals: &'static [&'static str],
    /// positional token が対話 session(deno `repl`)。
    structural_positionals: &'static [&'static str],
}

#[allow(clippy::too_many_arguments)] // 表駆動 spec の宣言的な形を維持
const fn spec(
    positional: Interaction,
    intent_letters: &'static str,
    intent_letters_ci: &'static str,
    intent_long: &'static [&'static str],
    intent_prefixes: &'static [&'static str],
    structural_prefixes: &'static [&'static str],
    structural_long: &'static [&'static str],
    file_flags: &'static [&'static str],
    file_prefixes: &'static [&'static str],
    structural_letters: &'static str,
    approval_positionals: &'static [&'static str],
    structural_positionals: &'static [&'static str],
) -> ReplSpec {
    ReplSpec {
        positional,
        intent_letters,
        intent_letters_ci,
        intent_long,
        intent_prefixes,
        structural_prefixes,
        structural_long,
        file_flags,
        file_prefixes,
        structural_letters,
        approval_positionals,
        structural_positionals,
    }
}

/// shell 系 — `-c`/`/k` 相当の inline code と `-i` の interactive
/// intent が `Approval`。`-e` は errexit で eval ではないため含め
/// ない(`sh -e run.sh` は `Batch`)。
const SHELL_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "ci",
    "",
    &["command"], // fish `--command`
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
const PYTHON_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "ci", // `-c` eval・`-i` interactive;`-I`(isolated)は別物
    "",
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
const NODE_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "epi", // `-e`/`--eval`・`-p`/`--print`・`-i`
    "",
    &["eval", "print", "interactive"],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
const DENO_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "ei",
    "",
    &["eval", "print"],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &["eval"], // `deno eval` inline code subcommand
    &["repl"], // `deno repl` は対話 session
);
const RUBY_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "e",
    "",
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
/// `irb file.rb` も file 実行後に REPL へ残るため常に対話。
const IRB_SPEC: ReplSpec = spec(
    Interaction::Structural,
    "",
    "",
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
/// perl `-e`/`-E` は両方 eval(`-E` は feature 有効版)で `-pe` の
/// ような cluster + attached code が常套形。
const PERL_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "",
    "e",
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
const PHP_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "r",
    "rb", // `-r`/`-R`/`-B` code 系 flag
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    "a", // `php -a` interactive shell
    &[],
    &[],
);
const LUA_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "ei",
    "",
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
/// pwsh/powershell — parameter は case-insensitive で unambiguous
/// prefix が効く(`-Com`→`-Command`・`-Enc`→`-EncodedCommand`)。
/// `-e`/`-ec` は実装が受け付ける省略綴りとして安全側に含める。
/// `-File` prefix(`-f`…`-File`)は script file で `Batch`。
const PWSH_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "",
    "",
    &["c", "e", "ec"],
    &["command", "encodedcommand", "commandwithargs"],
    &["interactive", "noexit", "interactivewithargs"],
    &[],
    &[],
    &["file"],
    "",
    &[],
    &[],
);
/// `cmd /c`・`/k` は inline code 実行で `bash -c` 相当の `Approval`。
const CMD_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "",
    "",
    &["c", "k"],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);
/// psql/mysql 系 — positional(db 名)だけでは対話 session に落ちる。
/// `-c`/`-e`/`-q` inline query は `Approval`、`-f FILE` は `Batch`。
const DB_SESSION_SPEC: ReplSpec = spec(
    Interaction::Structural,
    "ce",
    "q",
    &["command", "execute", "query", "eval"],
    &[],
    &[],
    &[],
    &["f", "file"],
    &[],
    "",
    &[],
    &[],
);
/// positional が query/command 文字列そのものの DB CLI。
const DB_COMMAND_SPEC: ReplSpec = spec(
    Interaction::Approval,
    "ce",
    "q",
    &["command", "execute", "query", "eval"],
    &[],
    &[],
    &[],
    &["f", "file"],
    &[],
    "",
    &[],
    &[],
);
/// redis-cli — `-x` は最後の arg を stdin から読む。
const REDIS_SPEC: ReplSpec = spec(
    Interaction::Approval,
    "",
    "",
    &[],
    &[],
    &[],
    &[],
    &[],
    &[],
    "x",
    &[],
    &[],
);
/// awk 系 — positional は program 文字列(`system()` で command
/// 実行可能)で `Approval`。`-f FILE` で file 化すれば `Batch`。
const AWK_SPEC: ReplSpec = spec(
    Interaction::Approval,
    "e",
    "",
    &["file", "exec"], // gawk `-e`/`--exec` program text
    &[],
    &[],
    &[],
    &["f", "file"],
    &[],
    "",
    &[],
    &[],
);
const SED_SPEC: ReplSpec = spec(
    Interaction::Approval,
    "e",
    "",
    &["expression"],
    &[],
    &[],
    &[],
    &["f", "file"],
    &[],
    "",
    &[],
    &[],
);
/// 汎用 REPL/jit — `-e`/`--eval`・`-c` 系 eval flag と `-i` を
/// `Approval`、positional script は `Batch`、bare は `Structural`。
const GENERIC_REPL_SPEC: ReplSpec = spec(
    Interaction::Batch,
    "cei",
    "e",
    &["eval", "exec", "execute", "command"],
    &[],
    &[],
    &[],
    &[],
    &[],
    "",
    &[],
    &[],
);

/// 名 → REPL spec。version suffix は `is_versioned_name` で吸収
/// する(`python3.11`・`node20`・`ruby3.2`・`lua5.4` 等)。
fn repl_spec(name: &str) -> Option<ReplSpec> {
    if [
        "bash", "sh", "zsh", "fish", "dash", "ksh", "mksh", "ash", "csh", "tcsh", "posh", "yash",
    ]
    .contains(&name)
    {
        return Some(SHELL_SPEC);
    }
    if name_is(name, &["python", "pypy"]) {
        return Some(PYTHON_SPEC);
    }
    if name_is(name, &["node", "nodejs"]) {
        return Some(NODE_SPEC);
    }
    if ["deno", "bun"].contains(&name) {
        return Some(DENO_SPEC);
    }
    if name_is(name, &["ruby", "jruby"]) {
        return Some(RUBY_SPEC);
    }
    if name == "irb" {
        return Some(IRB_SPEC);
    }
    if name_is(name, &["perl"]) {
        return Some(PERL_SPEC);
    }
    if name_is(name, &["php"]) {
        return Some(PHP_SPEC);
    }
    if name_is(name, &["lua", "luajit"]) {
        return Some(LUA_SPEC);
    }
    if name_is(name, &["pwsh", "powershell"])
        || ["pwsh-preview", "powershell-preview"].contains(&name)
    {
        return Some(PWSH_SPEC);
    }
    if name == "cmd" {
        return Some(CMD_SPEC);
    }
    if [
        "psql", "mysql", "mariadb", "mongosh", "mongo", "sqlcmd", "influx",
    ]
    .contains(&name)
    {
        return Some(DB_SESSION_SPEC);
    }
    if ["sqlite3", "duckdb"].contains(&name) {
        return Some(DB_COMMAND_SPEC);
    }
    if name == "redis-cli" {
        return Some(REDIS_SPEC);
    }
    if ["awk", "gawk", "mawk", "nawk"].contains(&name) {
        return Some(AWK_SPEC);
    }
    if ["sed", "gsed", "ssed"].contains(&name) {
        return Some(SED_SPEC);
    }
    if name_is(name, &["julia", "rscript"])
        || [
            "r",
            "ipython",
            "groovy",
            "osascript",
            "tclsh",
            "wish",
            "ghci",
            "runghc",
            "runhaskell",
            "nu",
            "xonsh",
            "iex",
            "elixir",
            "swift",
            "scala",
            "clj",
            "clojure",
            "octave",
            "racket",
            "gjs",
            "qjs",
            "jsc",
            "v8",
            "d8",
        ]
        .contains(&name)
    {
        return Some(GENERIC_REPL_SPEC);
    }
    None
}

/// 別 command へ実行を委譲する launcher/forwarder — inner command
/// を再帰分類する(`env sudo x`・`timeout 5 bash -c x` で argv0 名
/// だけ見ると危険側が `Batch` に漏れる)。`command`/`builtin`/`exec`
/// は shell builtin だが argv0 として現れ得るため含める。
const LAUNCHER_COMMANDS: &[&str] = &[
    "env",
    "nohup",
    "nice",
    "ionice",
    "stdbuf",
    "setsid",
    "command",
    "builtin",
    "exec",
    "linux32",
    "linux64",
    "setarch",
    "arch",
    "unshare",
    "nsenter",
    "chroot",
    "timeout",
    "taskset",
    "flock",
    "busybox",
    "toybox",
    "firejail",
    "bwrap",
    "flatpak-spawn",
    "systemd-run",
];

/// launcher の「値を取る」flag — 値 token を inner command 候補から
/// 外すための主要 flag の superset(per-command 厳密 parse は
/// しない — bool flag を値取り扱いすると実 inner を食い違えて
/// 検査が潜る側に倒れるため、確実に値を取るものだけを列挙する。
/// 値を positional と誤認しても tail-scan が残りを拾う)。
const LAUNCHER_VALUE_FLAGS: &[&str] = &[
    "-u",
    "--unset",
    "-C",
    "--chdir",
    "-S",
    "--split-string",
    "-s",
    "--signal",
    "-k",
    "--kill-after",
    "-n",
    "--adjustment",
    "-c",
    "--cpu-list",
    "-w",
    "--wait",
    "-t",
    "--target",
    "-p",
    "--pid",
    "-a",
    "--argv0",
    "-d",
    "--dir",
    "-f",
    "--file",
    "--block-signal",
    "--default-signal",
    "--ignore-signal",
];

/// `docker`/`kubectl`/`podman` 系 container CLI — stdin/TTY flag の
/// 綴り(permutation・`--x=y`・`--stdin`)を argv から拾う。
const CONTAINER_COMMANDS: &[&str] = &[
    "docker",
    "kubectl",
    "podman",
    "nerdctl",
    "buildah",
    "crictl",
    "oc",
    "docker-compose",
    "podman-compose",
];

/// REPL family の argv scan — `-` stdin program は `Structural`、
/// intent flag/綴りは `Approval`、positional は spec 階層。file
/// flag(script file 指定)があれば positional を data file として
/// `Batch` 化。positional・file 指定のどちらも無い形状は対話
/// session 前提で `Structural`。全 arg の最悪 tier を返す。
fn scan_repl_args(spec: &ReplSpec, exec_args: &[String]) -> Interaction {
    let mut tier = Interaction::Batch;
    let mut positional = false;
    let mut file_program = false;
    let mut saw_intent = false;
    let mut after_ddash = false;
    let mut i = 0;
    // intent/file flag は値を別 arg で取る — 次 arg を値として消費
    // する(`psql -c 'sql'` の `'sql'` を positional と誤認しない)。
    // `-`/`--` は消費しない(stdin program の `Structural` 判定を
    // 守る)。
    let consume_value = |exec_args: &[String], i: &mut usize| {
        if exec_args.get(*i).is_some_and(|n| n != "-" && n != "--") {
            *i += 1;
        }
    };
    while let Some(arg) = exec_args.get(i) {
        i += 1;
        if after_ddash {
            positional = true;
            continue;
        }
        let a = arg.as_str();
        if a == "--" {
            after_ddash = true;
            continue;
        }
        if a == "-" {
            return Interaction::Structural;
        }
        let stripped = a.trim_start_matches(['-', '/']);
        if stripped.len() != a.len() && !stripped.is_empty() {
            let lower = stripped.to_ascii_lowercase();
            let (name_part, value) = match lower.split_once('=') {
                Some((n, v)) => (n, Some(v)),
                None => (lower.as_str(), None),
            };
            let disabled = matches!(value, Some("false" | "0" | "no"));
            if spec.file_flags.contains(&name_part)
                || spec.file_prefixes.iter().any(|p| p.starts_with(name_part))
            {
                file_program = true;
                if value.is_none() {
                    consume_value(exec_args, &mut i);
                }
                continue;
            }
            if !disabled {
                if spec.intent_long.contains(&name_part)
                    || spec
                        .intent_prefixes
                        .iter()
                        .any(|p| p.starts_with(name_part))
                {
                    tier = tier.max(Interaction::Approval);
                    saw_intent = true;
                    if value.is_none() {
                        consume_value(exec_args, &mut i);
                    }
                }
                if spec.structural_long.contains(&name_part)
                    || spec
                        .structural_prefixes
                        .iter()
                        .any(|p| p.starts_with(name_part))
                {
                    tier = tier.max(Interaction::Structural);
                }
            }
            // single-dash cluster(`-lc`・`-pe`・`-c'code'` 等)の
            // letter scan。`--x`・`/x` は cluster にしない。
            if a.starts_with('-') && !a.starts_with("--") {
                let mut cluster_intent = false;
                for ch in stripped.chars() {
                    if spec.intent_letters.contains(ch)
                        || spec.intent_letters_ci.contains(ch.to_ascii_lowercase())
                    {
                        tier = tier.max(Interaction::Approval);
                        cluster_intent = true;
                    }
                    if spec.structural_letters.contains(ch) {
                        tier = tier.max(Interaction::Structural);
                    }
                }
                // cluster 内の intent flag は値を次 arg から取る
                // (`bash -c x`・`perl -pe 'code'`・`python -i`)。
                // bool 系(`-i` 等)でも値を消費しても tier は変わら
                // ないため、区別せず消費する。
                saw_intent |= cluster_intent;
                if cluster_intent && value.is_none() {
                    consume_value(exec_args, &mut i);
                }
            }
            continue;
        }
        // positional token — spec.positional tier は scan 完了後に
        // 適用する(file flag が後から来ても positional を data file
        // と誤認しないよう遅延する)。
        let lower = a.to_ascii_lowercase();
        if spec.approval_positionals.contains(&lower.as_str()) {
            tier = tier.max(Interaction::Approval);
            saw_intent = true;
        } else if spec.structural_positionals.contains(&lower.as_str()) {
            tier = tier.max(Interaction::Structural);
        } else {
            positional = true;
        }
    }
    if file_program {
        // script file 指定済み — positional は data file で
        // spec.positional を適用しない(`awk -f prog data`・
        // `psql -f x.sql` は `Batch`)。flag 由来の tier は残す
        // (`pwsh -NoExit -File x` は `-NoExit` で `Structural`)。
        return tier;
    }
    match (positional, tier) {
        // intent flag 検出済みの positional は flag 値・db 名等の
        // 残り引数で spec.positional に畳まない(`psql -c x db` は
        // session でなく inline query 実行 — `Approval` のまま)。
        (true, t) if saw_intent => t,
        (true, t) => t.max(spec.positional),
        (false, Interaction::Batch) => Interaction::Structural,
        (false, t) => t,
    }
}

/// launcher argv を走査し、value-flag pair と `KEY=VALUE` 代入を除く
/// 各 positional token を「inner command + 以降の argv」として再帰
/// 分類する。positional が無ければ `Batch`(`env` 単独等)。
fn classify_launcher_args(exec_args: &[String]) -> Interaction {
    let mut tier = Interaction::Batch;
    let mut i = 0;
    while let Some(arg) = exec_args.get(i) {
        i += 1;
        let a = arg.as_str();
        if LAUNCHER_VALUE_FLAGS.contains(&a) {
            i += 1;
            continue;
        }
        if a.starts_with('-') {
            continue;
        }
        if is_env_prefix(a) {
            continue;
        }
        // inner command 候補 — 以降の argv を伴って再帰分類。
        tier = tier.max(classify_interaction(&command_name(a), &exec_args[i..]));
    }
    tier
}

/// container CLI の argv scan — `--` 以降は container 内 command の
/// 引数であり local の stdin/TTY 意味論を持たないため flag 部分の
/// みを見る。single-dash cluster の `i`/`t`(`-it`・`-ti`・`-itd`)
/// と long flag(`--interactive`・`--tty`・`--stdin`、`=false` は
/// 無効扱い)で `Structural`。
fn scan_container_args(exec_args: &[String]) -> Interaction {
    for arg in exec_args.iter().take_while(|a| a.as_str() != "--") {
        let a = arg.as_str();
        if let Some(rest) = a.strip_prefix("--") {
            let name = rest.split('=').next().unwrap_or_default();
            let enabled = !a.contains("=false") && !a.contains("=0");
            if enabled && matches!(name, "interactive" | "tty" | "stdin") {
                return Interaction::Structural;
            }
        } else if a.starts_with('-')
            && a.len() > 1
            && a[1..].chars().any(|c| matches!(c, 'i' | 't'))
        {
            return Interaction::Structural;
        }
    }
    Interaction::Batch
}

/// executable 名 + argv 形状から routed exec の stdin 依存階層を
/// 分類する。判定順: UI/TTY(`Structural`)→ 特権/remote/forwarder
/// (`Approval`)→ `find -exec` 系(`Approval`)→ launcher(inner
/// command 再帰)→ REPL/interpreter family(scan) → container flag
/// → `Batch`。family 判定は version suffix 付き名
/// (`python3.11`・`node20`)を吸収する — 未列挙の新規名は安全側
/// ではなく `Batch` に倒れる既知の残余リスク(curated-list の
/// 限界、design doc 参照)。
pub(crate) fn classify_interaction(executable: &str, exec_args: &[String]) -> Interaction {
    let name = executable.to_ascii_lowercase();
    if UI_STRUCTURAL_COMMANDS.contains(&name.as_str()) {
        return Interaction::Structural;
    }
    if PRIVILEGED_COMMANDS.contains(&name.as_str()) {
        return Interaction::Approval;
    }
    if name == "find" {
        return if exec_args
            .iter()
            .any(|a| matches!(a.as_str(), "-exec" | "-execdir" | "-ok" | "-okdir"))
        {
            Interaction::Approval
        } else {
            Interaction::Batch
        };
    }
    if LAUNCHER_COMMANDS.contains(&name.as_str()) {
        return classify_launcher_args(exec_args);
    }
    if let Some(spec) = repl_spec(&name) {
        return scan_repl_args(&spec, exec_args);
    }
    if CONTAINER_COMMANDS.contains(&name.as_str()) {
        return scan_container_args(exec_args);
    }
    Interaction::Batch
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

/// env-prefix strip・self-bypass・prefix match・longest/tie・
/// suppressed attribution を行う shared match phase。`argv` は
/// env-prefix を含み得る raw 列、`routes` は shared merge 済みの
/// effective/suppressed/invalid set。
pub(crate) fn match_command<'a>(
    argv: &[String],
    routes: &'a EffectiveRoutes,
) -> RouteCommandMatch<'a> {
    let (_, argv) = split_env_prefix(argv);
    let argv = argv.to_vec();
    let executable = argv.first().map(|token| command_name(token));
    let self_bypassed = executable
        .as_deref()
        .is_some_and(|name| name.eq_ignore_ascii_case("cheaper"));
    let suppressed = |command: &[String]| {
        routes
            .suppressed
            .iter()
            .find(|s| is_prefix(&s.command, command))
            .map(|s| s.name.clone())
    };

    let mut matched_indices = Vec::new();
    if !argv.is_empty() && !self_bypassed {
        matched_indices.extend(
            routes
                .routes
                .iter()
                .enumerate()
                .filter(|(_, route)| is_prefix(&route.command, &argv))
                .map(|(index, _)| index),
        );
    }
    if matched_indices.is_empty() {
        let suppressed = if argv.is_empty() || self_bypassed {
            None
        } else {
            suppressed(&argv)
        };
        return RouteCommandMatch {
            argv,
            executable,
            rows: routes
                .routes
                .iter()
                .map(|route| RouteCommandMatchRow {
                    route,
                    matched: false,
                    selected: false,
                })
                .collect(),
            selected: None,
            tied: false,
            suppressed,
            self_bypassed,
        };
    }

    matched_indices.sort_by_key(|index| routes.routes[*index].command.len());
    let selected_index = matched_indices.pop().expect("nonempty");
    let selected = &routes.routes[selected_index];
    let tied = matched_indices
        .last()
        .is_some_and(|index| routes.routes[*index].command.len() == selected.command.len());
    RouteCommandMatch {
        argv,
        executable,
        rows: routes
            .routes
            .iter()
            .enumerate()
            .map(|(index, route)| RouteCommandMatchRow {
                route,
                matched: matched_indices.contains(&index) || index == selected_index,
                selected: index == selected_index,
            })
            .collect(),
        selected: Some(selected),
        tied,
        suppressed: suppressed(&selected.command),
        self_bypassed,
    }
}

/// route match → viability の純粋評価。`ResolveInput.argv` は
/// env-prefix を含み得る raw 列(`--argv` arm も同じく先頭 skip で
/// 正規化されるよう resolve 側で strip する — 2 経路で結果が
/// 乖離しない)。
pub(crate) fn resolve_with_target_exists<'a, F>(
    input: &ResolveInput<'a>,
    target_exists: F,
) -> RouteResolutionWithMatch<'a>
where
    F: Fn(&Route) -> bool,
{
    let matched = match_command(input.argv, input.routes);
    let resolution = resolve_matched(input, &matched, &target_exists);
    RouteResolutionWithMatch {
        resolution,
        matched,
    }
}

fn resolve_matched<F>(
    input: &ResolveInput<'_>,
    matched: &RouteCommandMatch<'_>,
    target_exists: &F,
) -> RouteResolution
where
    F: Fn(&Route) -> bool,
{
    if matched.argv.is_empty() {
        return no_match(Vec::new(), None, None);
    }
    // 1. self-bypass(R9)— cheaper 自身の command は match 評価
    // しない(recursion 防止)。
    if matched.self_bypassed {
        return no_match(Vec::new(), matched.executable.clone(), None);
    }
    let Some(best) = matched.selected else {
        return no_match(
            matched.argv.clone(),
            matched.executable.clone(),
            matched.suppressed.clone(),
        );
    };
    // 2. argv prefix match(R4)+ longest match(R5) は shared phase
    // が確定済み。ここでは resolution projection のみを組み立てる。
    let argv = matched.argv.clone();
    let args = argv[best.command.len()..].to_vec();
    let exec_args = argv[1..].to_vec();
    let mut out = RouteResolution {
        matched: true,
        route: Some(best.name.clone()),
        target: Some(best.target_path.display().to_string()),
        grant: Some(best.grant.clone()),
        executable: matched.executable.clone(),
        argv,
        args,
        exec_args,
        viability: None,
        suppressed: matched.suppressed.clone(),
        passthrough_env: best.passthrough_env.clone(),
    };
    // 3. viability 固定優先順(R12)— 最初に成立した 1 値。
    out.viability = Some(viability(input, best, &out, matched.tied, target_exists));
    out
}

/// 既存 caller 向け compatibility wrapper — target existence は
/// 従来どおり filesystem `Path::is_file` と同値。
pub fn resolve(input: &ResolveInput<'_>) -> RouteResolution {
    resolve_with_target_exists(input, |route| route.target_path.is_file()).resolution
}

fn is_prefix(prefix: &[String], argv: &[String]) -> bool {
    prefix.len() <= argv.len() && argv[..prefix.len()] == *prefix
}

fn viability<F>(
    input: &ResolveInput<'_>,
    route: &Route,
    out: &RouteResolution,
    tied: bool,
    target_exists: &F,
) -> Viability
where
    F: Fn(&Route) -> bool,
{
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
    // filesystem default か source-aware diagnostics predicate かは
    // caller が注入し、優先順位置は R12 のまま変えない。
    if !target_exists(route) {
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
        execution_correlation: None,
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
    // stdin 分類 — typed argv0 名・grant 宣言 executable 名・pin
    // 解決済み実体名の union を実行 argv で分類し、最悪階層を採る。
    // 片側だけだと canonical 名ずらし(`python3`→`python3.11`・
    // `vim`→`vim.basic`・`vi`→`nvi`)や「route 名と grant executable
    // が別名」な byte-matched 経路(`match:[mypy]`+executable:python3
    // で `mypy -c` が `python3 -c` として spawn されるのに typed 名
    // 側しか見ない漏れ)で系統的に回避される — substitute 側だけで
    // なく byte-matched 側にも同じ union を掛ける(O6a/R10)。
    // `exec_pair` は byte-matched では {route.grant, 原 exec_args}、
    // substitute では {substitute.grant, substitute.argv} を返す
    // ため、exec_args に対する宣言/解決名の union は両 mode で同一
    // 形になる。`Structural` は stdin/TTY なしに実行不能なため不発、
    // `Approval` は実行可能だが承認必須、`Batch` のみ素通しで
    // routable(typed 側 invariant 維持: structural な typed
    // command は substitute 宣言の有無に関わらず置き換えない)。
    let tier = [Interaction::Batch]
        .into_iter()
        .chain([classify_interaction(name, &out.exec_args)])
        .chain([
            classify_interaction(&command_name(&grant.executable), exec_args),
            classify_interaction(&command_name(&pinned), exec_args),
        ])
        .max()
        .unwrap_or(Interaction::Batch);
    match tier {
        Interaction::Structural => Viability::StdinUnsupported,
        Interaction::Approval => Viability::ApprovalRequired,
        Interaction::Batch => Viability::Runnable,
    }
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
        // stdin-unsupported — structural executable(vim)。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["vim"], "cargo-test", &target_str)],
            ..Default::default()
        };
        // exec_args が grant pattern に適合する形で stdin-unsupported
        // を単独検証する(no-grant が優先されるため)。
        let out = run(&routes, &["vim", "test", "x"]);
        assert_eq!(out.viability, Some(Viability::StdinUnsupported));
        // approval-required — batch 形状でも inline code flag(`-c`)
        // を持つ shell は承認必須。
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["bash"], "cargo-test", &target_str)],
            ..Default::default()
        };
        let out = run(&routes, &["bash", "test", "-c", "x"]);
        assert_eq!(out.viability, Some(Viability::ApprovalRequired));
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

    #[test]
    fn shared_match_reports_row_level_state_and_declaration_predicate() {
        let target = tempfile::tempdir().unwrap().path().join("w.json");
        let target = target.to_string_lossy().into_owned();
        let short = route("short", &["cargo"], "cargo-test", &target);
        let long = route("long", &["cargo", "test"], "cargo-test", &target);
        let unrelated = route("unrelated", &["git"], "cargo-test", &target);
        let routes = EffectiveRoutes {
            routes: vec![short.clone(), long.clone(), unrelated],
            suppressed: vec![SuppressedRoute {
                name: "suppressed-cargo".to_string(),
                command: argv(&["cargo"]),
            }],
            ..Default::default()
        };

        let matched = match_command(&argv(&["CI=1", "cargo", "test", "-p", "x"]), &routes);
        assert_eq!(matched.argv, argv(&["cargo", "test", "-p", "x"]));
        assert_eq!(matched.executable.as_deref(), Some("cargo"));
        assert_eq!(matched.rows.len(), 3);
        assert!(matched.rows[0].matched);
        assert!(!matched.rows[0].selected);
        assert!(matched.rows[1].matched);
        assert!(matched.rows[1].selected);
        assert!(!matched.rows[2].matched);
        assert!(!matched.rows[2].selected);
        assert_eq!(matched.selected.unwrap().name, "long");
        assert!(!matched.tied);
        assert_eq!(matched.suppressed.as_deref(), Some("suppressed-cargo"));

        // effective/non-effective row の declaration command は同じ
        // predicate で評価する。clone 済み merge row でも selected は
        // pointer ではなく route identity で判定できる。
        assert!(matched.matched_command(&argv(&["cargo"])));
        assert!(matched.matched_command(&argv(&["cargo", "test"])));
        assert!(!matched.matched_command(&argv(&["cargo", "check"])));
        assert!(matched.is_selected(&long));
        assert!(!matched.is_selected(&short));

        let bypass = match_command(&argv(&["cheaper", "run", "x"]), &routes);
        assert!(bypass.rows.iter().all(|row| !row.matched && !row.selected));
        assert!(!bypass.matched_command(&argv(&["cheaper"])));
        assert!(bypass.selected.is_none());
        assert!(bypass.suppressed.is_none());
    }

    #[test]
    fn resolve_with_target_exists_injects_only_missing_target_predicate() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let missing_target = dir.path().join("missing.workflow.json");
        let routes = EffectiveRoutes {
            routes: vec![route(
                "r",
                &["cargo", "test"],
                "absent-grant",
                &missing_target.to_string_lossy(),
            )],
            ..Default::default()
        };
        let policy = grant_policy();
        let deny = ExecDenyList::default();
        let view = policy_view(&policy, &root, &deny);
        let input = ResolveInput {
            argv: &argv(&["cargo", "test"]),
            cwd: &root,
            routes: &routes,
            exec_policy: &view,
            unsafe_command: false,
        };

        // filesystem default は従来どおり MissingTarget。
        assert_eq!(resolve(&input).viability, Some(Viability::MissingTarget));
        // diagnostics predicate が true を返せば missing-target を通過し、
        // 次の no-grant へ進む。
        let evaluated =
            resolve_with_target_exists(&input, |route| route.layer == RouteLayer::Builtin);
        assert_eq!(
            evaluated.resolution.viability,
            Some(Viability::NoGrant),
            "injected target_exists must affect only missing-target"
        );
        assert_eq!(evaluated.matched.selected.unwrap().name, "r");
        let missing = resolve_with_target_exists(&input, |_| false);
        assert_eq!(missing.resolution.viability, Some(Viability::MissingTarget));
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
    /// substitute 宣言の有無に関わらず `Structural` なら不発
    /// (「stdin 依存 typed command は routing で置き換えない」
    /// invariant)。substitute 側は宣言 executable 名と pin 解決済み
    /// 実体名の union + `substitute.argv` の flag で tri-state に
    /// 分類し、最悪階層(Structural > Approval > Batch)を採る。
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
        // typed 側 structural — substitute 宣言があっても `vim`
        // typed command は不発(invariant 維持)。`bash -c` は
        // approval tier へ降格されるため別 tier で検証済み。
        let mut r = route("r", &["vim"], "cargo-test", &target_str);
        r.substitute = Some(sub("docker-sub", &["ps"]));
        let out = run(&policy, r, &["vim", "test", "x"]);
        assert_eq!(out.viability, Some(Viability::StdinUnsupported));
    }

    /// canonical 名ずらし・shim 名の union 両側 pin(#167 AC9 —
    /// declared ≠ resolved は symlink のみで構成できるため unix
    /// gate、`SymlinkPolicy::WithinWorkspace` で pin 解決を通す)。
    /// substitute argv を空(bare invocation = `Structural`)にした
    /// 場合、(a) 宣言名 `python3` が REPL list 内・解決先
    /// `python3.11` が list 外でも宣言名側で `StdinUnsupported`、
    /// (b) 宣言名 `shim-python` が list 外でも解決済み実体名
    /// `python3` が list 内なら解決名側で `StdinUnsupported`。
    /// positional arg を持つ `python3 x` は `Batch` で routable。
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
        let sub_route = |grant: &str, args: &[&str]| {
            let mut r = route("r", &["git", "push"], "cargo-test", &target_str);
            r.substitute = Some(sub(grant, args));
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
        // bare argv で `Structural`。
        let p = policy("versioned", "./python3");
        assert_eq!(
            run(&p, sub_route("versioned", &[])).viability,
            Some(Viability::StdinUnsupported)
        );
        // positional arg の batch 形状は `Runnable` — declared 側が
        // REPL でも script を持つ spawn は stdin 非依存。
        assert_eq!(
            run(&p, sub_route("versioned", &["x.py"])).viability,
            Some(Viability::Runnable)
        );
        // (b) 宣言 `shim-python`(list 外)→ 解決 `python3`(list 内)。
        let p = policy("shimmed", "./shim-python");
        assert_eq!(
            run(&p, sub_route("shimmed", &[])).viability,
            Some(Viability::StdinUnsupported)
        );
        // 非 interactive 宣言+解決は Runnable(union 非該当の対照)。
        let p = policy("plain", "./mypython");
        assert_eq!(
            run(&p, sub_route("plain", &["x"])).viability,
            Some(Viability::Runnable)
        );
    }

    /// byte-matched 経路でも grant 宣言 executable 名・pin 解決済み
    /// 実体名を実行 argv で分類する(substitute 側と同じ union)。
    /// route 名と spawn executable が別名の構成(`match:[mypy]` +
    /// `executable:python3`)で typed 名だけ見ると `mypy -c x` が
    /// `Batch` に漏れる — spawn される `python3 -c x` の意味で
    /// `Approval` になることを pin する。grant executable は pin
    /// 解決が要るため test binary を interpreter 名で複写する
    /// (`command_name` は dir+`.exe` を剥がすため basename で
    /// 分類される)。
    #[test]
    fn byte_match_classifies_grant_executable_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().into_owned();
        let target = dir.path().join("w.json");
        std::fs::write(&target, "{}").unwrap();
        let target_str = target.to_string_lossy().into_owned();
        let deny = ExecDenyList::default();
        #[cfg(windows)]
        let pyname = "python3.exe";
        #[cfg(not(windows))]
        let pyname = "python3";
        std::fs::copy(std::env::current_exe().unwrap(), dir.path().join(pyname)).unwrap();
        let policy = resolve_exec_policy(
            Some(&json!({
                "grants": {
                    "py": {"executable": format!("./{pyname}"), "argv": ["*"]},
                }
            })),
            &ExecDenyList::default(),
        )
        .unwrap();
        let view = policy_view(&policy, &root, &deny);
        let routes = EffectiveRoutes {
            routes: vec![route("r", &["mypy"], "py", &target_str)],
            ..Default::default()
        };
        let run = |tokens: &[&str]| {
            resolve(&ResolveInput {
                argv: &argv(tokens),
                cwd: &root,
                routes: &routes,
                exec_policy: &view,
                unsafe_command: false,
            })
            .viability
        };
        // 宣言 executable 名 `python3` + `-c` inline code →
        // Approval(typed `mypy` 名だけ見ると `Batch` に漏れる)。
        assert_eq!(run(&["mypy", "-c", "x"]), Some(Viability::ApprovalRequired));
        // 宣言 executable が bare REPL 相当でも同じ union で
        // Structural — typed positional は substitute の有無に
        // 関わらず `python3` argv として解釈されるため。
        assert_eq!(run(&["mypy"]), Some(Viability::StdinUnsupported));
        // script positional 相当の exec_args は `python3 x.py`
        // として `Batch`。
        assert_eq!(run(&["mypy", "x.py"]), Some(Viability::Runnable));
    }

    /// `classify_interaction` の分類行列 — Batch(自動 route
    /// 可)/Approval(承認必須で route 可)/Structural(routing 不可)
    /// の境界を pin する。
    #[test]
    fn classify_interaction_matrix() {
        let classify = |exe: &str, args: &[&str]| {
            classify_interaction(exe, &args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        };
        // Batch — interpreter + script、task runner、plain command、
        // file flag(script file 指定)、errexit 等の benign flag。
        for (exe, args) in [
            ("python", &["check.py"][..]),
            ("python3", &["-u", "check.py", "--flag"][..]),
            ("python3.11", &["check.py"][..]),
            ("node", &["tools/build.js"][..]),
            ("ruby", &["gen.rb"][..]),
            ("pwsh", &["-File", "check.ps1"][..]),
            ("pwsh", &["-f", "check.ps1"][..]),
            ("powershell", &["-File", "check.ps1"][..]),
            ("pwsh", &["-NonInteractive", "-File", "x.ps1"][..]),
            ("bash", &["run.sh"][..]),
            ("sh", &["deploy.sh", "prod"][..]),
            // `-e` は errexit(eval ではない)— 偽陽性にならない。
            ("sh", &["-e", "deploy.sh"][..]),
            ("python", &["-I", "x.py"][..]),
            ("npm", &["run", "inspect"][..]),
            ("cargo", &["test"][..]),
            ("git", &["push", "origin", "main"][..]),
            ("docker", &["run", "img"][..]),
            ("docker", &["run", "-d", "img"][..]),
            ("kubectl", &["get", "pods"][..]),
            ("docker", &["exec", "ctr", "ls"][..]),
            ("psql", &["-f", "migration.sql"][..]),
            ("awk", &["-f", "prog.awk", "data.txt"][..]),
            ("sed", &["-f", "script.sed", "f.txt"][..]),
            ("env", &["FOO=bar", "python", "x.py"][..]),
            ("timeout", &["30", "cargo", "test"][..]),
            ("nice", &["-n", "5", "make"][..]),
        ] {
            assert_eq!(classify(exe, args), Interaction::Batch, "{exe} {args:?}");
        }
        // Approval — interactive intent / inline code flag、特権・
        // remote・detached 副作用 command、launcher 経由の inner
        // command、positional program 文字列(awk/sed/DB CLI)。
        for (exe, args) in [
            ("python", &["-i", "x.py"][..]),
            ("python", &["-c", "print(1)"][..]),
            // version suffix 付き名・attached code も同じ tier。
            ("python3.11", &["-c", "x"][..]),
            ("node", &["-e", "console.log(1)"][..]),
            ("node", &["--eval", "x"][..]),
            ("node", &["-pe", "x"][..]),
            ("deno", &["eval", "x"][..]),
            ("bash", &["-c", "echo hi"][..]),
            ("sh", &["-c", "ls"][..]),
            // grouped short flag cluster(`-lc`・`-ec`)。
            ("bash", &["-lc", "id"][..]),
            ("sh", &["-ec", "x"][..]),
            ("zsh", &["-fc", "x"][..]),
            ("perl", &["-pe", "code", "f"][..]),
            ("ruby", &["-e", "x"][..]),
            ("pwsh", &["-Command", "Get-Date"][..]),
            ("powershell", &["-Command", "Get-Date"][..]),
            // pwsh の unambiguous-prefix・encoded・`/` 綴り。
            ("pwsh", &["-Com", "x"][..]),
            ("pwsh", &["-EncodedCommand", "b64"][..]),
            ("pwsh", &["-enc", "b64"][..]),
            ("pwsh", &["-ec", "b64"][..]),
            ("pwsh", &["/Command", "x"][..]),
            ("powershell", &["/EncodedCommand", "b64"][..]),
            // `cmd /c`・`/k` は inline code 実行で `bash -c` 相当。
            ("cmd", &["/c", "dir"][..]),
            ("cmd", &["/k", "x"][..]),
            ("node", &["--interactive", "x.js"][..]),
            ("sudo", &["systemctl", "restart", "x"][..]),
            ("ssh", &["host", "uptime"][..]),
            ("su", &["-", "root"][..]),
            ("doas", &["x"][..]),
            ("pkexec", &["x"][..]),
            ("tmux", &["new", "-d", "x"][..]),
            ("crontab", &["-e"][..]),
            ("xargs", &["rm"][..]),
            // launcher 経由の inner command 再帰。
            ("env", &["sudo", "rm", "-rf", "/"][..]),
            ("env", &["FOO=x", "bash", "-c", "id"][..]),
            ("env", &["-i", "bash", "-i"][..]),
            ("timeout", &["5", "bash", "-i"][..]),
            ("exec", &["bash", "-i"][..]),
            ("find", &[".", "-exec", "bash", "-c", "id", "+"][..]),
            // positional program 文字列。
            ("awk", &["{print $1}", "f"][..]),
            ("sed", &["s/a/b/", "f"][..]),
            ("redis-cli", &["SET", "k", "v"][..]),
            ("sqlite3", &["db", "DELETE FROM t"][..]),
            ("psql", &["-c", "sql"][..]),
            ("mysql", &["-e", "sql"][..]),
        ] {
            assert_eq!(classify(exe, args), Interaction::Approval, "{exe} {args:?}");
        }
        // Structural — stdin/TTY なしには成立しない: UI app、bare
        // REPL、stdin program(`-`)、対話 session 化・container
        // interactive flag の全綴り。
        for (exe, args) in [
            ("vim", &["file.txt"][..]),
            ("nvim", &[""][..]),
            ("less", &["out.log"][..]),
            ("top", &[][..]),
            ("watch", &["ls"][..]),
            ("telnet", &["host", "80"][..]),
            ("python", &[][..]),
            ("python3.12", &[][..]),
            ("node", &[][..]),
            ("bash", &[][..]),
            ("python", &["-V"][..]),
            ("python", &["-"][..]),
            ("sh", &["-"][..]),
            // `-` は `Approval` flag より常に優先(worst-tier scan)。
            ("python", &["-i", "-"][..]),
            ("python", &["-", "-i"][..]),
            ("irb", &["x.rb"][..]),
            ("psql", &["mydb"][..]),
            ("mysql", &["-u", "root", "db"][..]),
            ("deno", &["repl"][..]),
            ("pwsh", &["-NoExit", "-File", "x.ps1"][..]),
            ("pwsh", &["-Interactive", "-File", "x.ps1"][..]),
            ("docker", &["run", "-it", "img"][..]),
            ("docker", &["run", "-ti", "img"][..]),
            ("docker", &["exec", "-i", "ctr", "sh"][..]),
            ("kubectl", &["exec", "--tty", "pod"][..]),
            ("kubectl", &["exec", "--stdin", "pod", "--", "x"][..]),
            ("docker", &["run", "--interactive=true", "img"][..]),
            ("nerdctl", &["run", "-it", "img"][..]),
        ] {
            assert_eq!(
                classify(exe, args),
                Interaction::Structural,
                "{exe} {args:?}"
            );
        }
        // 判定順 — `python -i x.py` は positional を持っても
        // Approval(flag が先に評価される)。`vim` は argv 形状を
        // 見ず常に Structural。
        assert_eq!(classify("python", &["x.py", "-i"]), Interaction::Approval);
        assert_eq!(classify("vim", &[]), Interaction::Structural);
        // case 不問 — `VIM`・`-FILE` は lowercase 正規化後も同 tier。
        assert_eq!(classify("VIM", &["x"]), Interaction::Structural);
        assert_eq!(
            classify("powershell", &["-FILE", "x.ps1"]),
            Interaction::Batch
        );
        // `--interactive=false` は disabled 扱い — positional があれば
        // Batch、無ければ flag のみ形状で Structural(安全側)。
        assert_eq!(
            classify("docker", &["run", "--interactive=false", "img"]),
            Interaction::Batch
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
