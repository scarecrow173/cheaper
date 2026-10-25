//! `cheaper-bench` process entry — 薄い entry(design §1.1/§1.4)。
//!
//! 責務は 2 分岐のみ:
//! 1. `argv[1] == "--fixture-child"` を**何より先に**判定し
//!    `child::dispatch` へ委譲 — fixture child は harness 本体・
//!    arg parse へ進まず、child 処理後 `process::exit(code)` で抜ける。
//!    route 経路では typed command の `argv[1..]`(exec_args)がそのまま
//!    `argv[1]`/`argv[2]` になる(§1.4 — grant `argv` は match
//!    pattern であり注入器ではない)。
//! 2. 通常経路は `cli::run` へ(arg parse → manifest load → run loop)。

fn main() {
    let argv: Vec<String> = std::env::args().collect();

    if argv.get(1).map(String::as_str) == Some(cheaper_bench::child::FIXTURE_CHILD_ARGV) {
        std::process::exit(cheaper_bench::child::dispatch(&argv));
    }

    std::process::exit(cheaper_bench::cli::run(&argv[1..]));
}
