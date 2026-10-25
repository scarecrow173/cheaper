//! fixture child mode 実装(design §1.4)— bench bin 自身の自己再実行。
//!
//! v1 process fixture workload・route grant 対象・nonzero exit の
//! 供給源として、bench bin が `current_exe()` + 固定 argv marker で
//! 自ら fixture child になる
//! (`CHEAPER_ROUTE_CHILD_MODE`/`CHEAPER_PROC_CHILD_MODE`/
//! `workflow_dev_seam_command_fixture_child` で実証済みの pattern)。
//!
//! - dispatch 規約: `argv[1] == "--fixture-child"`・`argv[2]` が mode。
//!   `main()` が何より先に分岐し、child 処理後 `process::exit(code)`。
//! - route 経路では typed command の `argv[1..]`(=`exec_args`)がそのまま
//!   `argv[1]`/`argv[2]` になる — `command = "benchrun --fixture-child
//!   <mode>"` の mode は typed argv に書く。grant `argv` は ArgPattern
//!   の match pattern であって spawn argv への注入器ではない(§1.4)。
//! - spawn 経路: `processPolicy.tasks.*.command` の `{fixtureChild}` は
//!   `current_exe()` abs path へ展開済み argv(workspace.rs)、
//!   `direct-child-exec`/`direct-worker-sequence` manual 側は harness が
//!   `Command::new(current_exe())` を直接 spawn。いずれも shell 非経由・
//!   `Command` API のみ(Windows/Unix parity — PATH 解決を通らない)。

use std::io::Write;

/// 固定 argv marker — `argv[1]` がこれに一致すると fixture child。
pub const FIXTURE_CHILD_ARGV: &str = "--fixture-child";

/// `emit` mode の stdout byte 数を渡す env(既定 [`DEFAULT_EMIT_BYTES`])。
pub const ENV_CHILD_BYTES: &str = "CHEAPER_BENCH_CHILD_BYTES";
/// `fail` mode の exit code を渡す env(既定 [`DEFAULT_FAIL_EXIT`])。
pub const ENV_CHILD_EXIT: &str = "CHEAPER_BENCH_CHILD_EXIT";
/// 先行宣言 — `envdump` mode が観測 env を書き出す file path
/// (`passthroughEnv` 到達検証用、§1.4 A11。required 12 scenario は
/// 消費しない予約語彙)。
pub const ENV_CHILD_OUT: &str = "CHEAPER_BENCH_CHILD_OUT";

/// `emit` 既定 byte 数(v1 `emit.mjs`/`big` mode の 4096 相当)。
pub const DEFAULT_EMIT_BYTES: usize = 4096;
/// `fail` 既定 exit code(`io`/`e2e` mode 相当の 7)。
pub const DEFAULT_FAIL_EXIT: i32 = 7;
/// mode 引数の欠落・未知語彙・予約 mode 到達時の exit(EX_USAGE 系)。
pub const USAGE_EXIT: i32 = 64;

/// `fail` mode の stdout/stderr marker(観測側 pin 語彙)。
pub const FAIL_STDOUT_MARKER: &str = "CHEAPER-BENCH-FAIL-OUT-MARK";
pub const FAIL_STDERR_MARKER: &str = "CHEAPER-BENCH-FAIL-ERR-MARK";

/// fixture child mode 語彙(§1.4)。`Sleep`/`Envdump` は先行宣言 —
/// required 12 scenario は消費しない予約語彙として enum に宣言するのみ
/// (process timeout 計測・passthroughEnv 到達検証の将来拡張)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildMode {
    /// `CHEAPER_BENCH_CHILD_BYTES` byte の seeded 高エントロピ印字可能列を
    /// stdout へ書き exit 0(condense `outputTokens > tokenThreshold`
    /// 評価の決定性を担保する既知 token 密度 — §1.4)。
    Emit,
    /// stdout/stderr marker を書き `CHEAPER_BENCH_CHILD_EXIT` で exit
    /// (nonzero exit・exit-code mirror 計測用)。
    Fail,
    /// 先行宣言(process timeout 計測用)— 到達時は未対応 exit。
    Sleep,
    /// 先行宣言(`CHEAPER_BENCH_CHILD_OUT` への env dump —
    /// `passthroughEnv` 到達検証用)— 到達時は未対応 exit。
    Envdump,
}

impl ChildMode {
    /// `argv[2]` の mode 文字列 → mode。未知語彙は `None`。
    pub fn parse(token: &str) -> Option<Self> {
        match token {
            "emit" => Some(Self::Emit),
            "fail" => Some(Self::Fail),
            "sleep" => Some(Self::Sleep),
            "envdump" => Some(Self::Envdump),
            _ => None,
        }
    }

    /// manifest `route.childMode`/`manual.child.mode` が取り得る語彙名
    /// (route materialize・validation 側の既知語彙判定用)。
    pub fn name(self) -> &'static str {
        match self {
            Self::Emit => "emit",
            Self::Fail => "fail",
            Self::Sleep => "sleep",
            Self::Envdump => "envdump",
        }
    }
}

/// `main()` からの分岐先 — `argv` は `std::env::args()` 全列
/// (`argv[1] == "--fixture-child"` は caller が判定済み)。
/// `argv[2]` が mode。戻り値は `process::exit` へ渡す exit code。
pub fn dispatch(argv: &[String]) -> i32 {
    let Some(token) = argv.get(2) else {
        eprintln!("cheaper-bench fixture child: missing mode (expected: emit|fail|sleep|envdump)");
        return USAGE_EXIT;
    };
    let Some(mode) = ChildMode::parse(token) else {
        eprintln!("cheaper-bench fixture child: unknown mode: {token}");
        return USAGE_EXIT;
    };
    match mode {
        ChildMode::Emit => emit(),
        ChildMode::Fail => fail(),
        ChildMode::Sleep | ChildMode::Envdump => {
            // 予約語彙への到達 — 宣言のみで未実装。未対応を示す exit
            // (黙って成功に見せない)。
            eprintln!(
                "cheaper-bench fixture child: mode '{}' is declared but not implemented",
                mode.name()
            );
            USAGE_EXIT
        }
    }
}

fn env_parse<T: std::str::FromStr>(name: &str, default: T) -> Result<T, ()> {
    match std::env::var(name) {
        Ok(raw) => raw.parse::<T>().map_err(|_| ()),
        Err(_) => Ok(default),
    }
}

/// `emit` — `CHEAPER_BENCH_CHILD_BYTES`(既定 4096)byte を stdout へ
/// 書き exit 0。内容は固定 seed PRNG 由来の deterministic 高エントロピ
/// 印字可能列(`'L'.repeat(n)` 型の低エントロピ反復は o200k BPE で高
/// 圧縮され condense 評価の決定性を失うため禁止 — §1.4)。
fn emit() -> i32 {
    let bytes = match env_parse(ENV_CHILD_BYTES, DEFAULT_EMIT_BYTES) {
        Ok(n) => n,
        Err(()) => {
            eprintln!("cheaper-bench fixture child: {ENV_CHILD_BYTES} is not a usize");
            return USAGE_EXIT;
        }
    };
    let out = crate::fixture::seeded_emit_bytes(bytes);
    let mut stdout = std::io::stdout().lock();
    if stdout.write_all(&out).is_err() || stdout.flush().is_err() {
        eprintln!("cheaper-bench fixture child: stdout write failed");
        return USAGE_EXIT;
    }
    0
}

/// `fail` — stdout/stderr marker を書き `CHEAPER_BENCH_CHILD_EXIT`
/// (既定 7)で exit。nonzero exit・exit-code mirror 計測用
/// (`io`/`e2e` mode 相当)。
fn fail() -> i32 {
    let code = match env_parse(ENV_CHILD_EXIT, DEFAULT_FAIL_EXIT) {
        Ok(code) => code,
        Err(()) => {
            eprintln!("cheaper-bench fixture child: {ENV_CHILD_EXIT} is not an i32");
            return USAGE_EXIT;
        }
    };
    let _ = writeln!(std::io::stdout().lock(), "{FAIL_STDOUT_MARKER}");
    let _ = writeln!(std::io::stderr().lock(), "{FAIL_STDERR_MARKER}");
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_vocabulary() {
        assert_eq!(ChildMode::parse("emit"), Some(ChildMode::Emit));
        assert_eq!(ChildMode::parse("fail"), Some(ChildMode::Fail));
        // 予約語彙は parse できる(dispatch 側で未対応 exit へ写す)。
        assert_eq!(ChildMode::parse("sleep"), Some(ChildMode::Sleep));
        assert_eq!(ChildMode::parse("envdump"), Some(ChildMode::Envdump));
        assert_eq!(ChildMode::parse("bogus"), None);
        assert_eq!(ChildMode::parse(""), None);
    }

    #[test]
    fn reserved_modes_exit_unimplemented() {
        for mode in ["sleep", "envdump"] {
            let argv = vec![
                "cheaper-bench".to_string(),
                FIXTURE_CHILD_ARGV.to_string(),
                mode.to_string(),
            ];
            assert_eq!(dispatch(&argv), USAGE_EXIT);
        }
    }

    #[test]
    fn missing_or_unknown_mode_exits_usage() {
        let argv = vec!["cheaper-bench".to_string(), FIXTURE_CHILD_ARGV.to_string()];
        assert_eq!(dispatch(&argv), USAGE_EXIT);
        let argv = vec![
            "cheaper-bench".to_string(),
            FIXTURE_CHILD_ARGV.to_string(),
            "bogus".to_string(),
        ];
        assert_eq!(dispatch(&argv), USAGE_EXIT);
    }

    #[test]
    fn fail_mode_uses_default_exit() {
        // env 未設定時は既定 7(env は read 側 default。test 並列の
        // env 競合を避けるため env unset は保証しない — 他 crate test
        // が同名 env を設定しない語彙のため実質 deterministic)。
        if std::env::var(ENV_CHILD_EXIT).is_err() {
            let argv = vec![
                "cheaper-bench".to_string(),
                FIXTURE_CHILD_ARGV.to_string(),
                "fail".to_string(),
            ];
            assert_eq!(dispatch(&argv), DEFAULT_FAIL_EXIT);
        }
    }
}
