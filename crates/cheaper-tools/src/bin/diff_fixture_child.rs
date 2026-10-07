//! `tests/differential.rs` 用 fixture child — `node -e` task script の
//! 代替実行体。設計固定ルールで Rust runtime は `node` を PATH 解決しない
//! ため、fixture の `<NODE>` task はこの bin へ翻訳される。
//!
//! libtest re-exec(proc_process.rs の確立パターン)は `running N tests`
//! banner を stdout に書くため byte 一致の `details.stdout` 比較に使え
//! ない。clean stdout が必要な fixture child は bin target に分離し、
//! test 側は `env!("CARGO_BIN_EXE_diff_fixture_child")` で参照する。
//!
//! mode は `CHEAPER_DIFF_CHILD_MODE` env で切替え、payload args は
//! argv[1..](`node -e` の `process.argv.slice(1)` 相当)。各 mode の
//! 出力 byte は dump script の `NODE_TASKS` script と同期を保つ。

use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

fn main() {
    let mode = std::env::var("CHEAPER_DIFF_CHILD_MODE").unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    match mode.as_str() {
        "hi" => println!("hello process"),
        "fail" => {
            println!("before failure");
            eprintln!("failure detail");
            std::process::exit(7);
        }
        "output" => println!("{}", "a".repeat(1000)),
        "sleep" => loop {
            std::thread::sleep(Duration::from_millis(50));
        },
        "inspect" => {
            // `node -e` script と同一 JSON(key 順も固定 —
            // details.stdout.text は byte 一致比較される)。
            // `process.env.PATH?process.env.PATH:null` は falsy(`""`)
            // でも null を返すため nonempty filter を掛ける。
            let cwd = std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let cwd_tail = cwd.rsplit('/').next().unwrap_or_default().to_string();
            let explicit = std::env::var("ONLY_THIS").ok();
            let path = std::env::var("PATH").ok().filter(|s| !s.is_empty());
            println!(
                "{{\"cwdTail\":{},\"args\":{},\"explicit\":{},\"path\":{}}}",
                serde_json::to_string(&cwd_tail).unwrap(),
                serde_json::to_string(&args).unwrap(),
                serde_json::to_string(&explicit).unwrap(),
                serde_json::to_string(&path).unwrap(),
            );
        }
        "hold" => {
            // `HOLD` script: gate file が現れるまで 20ms 毎に poll、
            // 20s の safety timeout で exit(3)。
            let gate = args.first().expect("hold gate arg");
            let start = Instant::now();
            while !Path::new(gate).exists() {
                if start.elapsed() > Duration::from_secs(20) {
                    std::process::exit(3);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            println!("released");
        }
        other => {
            eprintln!("unknown fixture child mode: {other}");
            std::process::exit(2);
        }
    }
    std::io::stdout().flush().unwrap();
}
