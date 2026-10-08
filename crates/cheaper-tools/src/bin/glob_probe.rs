//! glob 差分プローブ: stdin の JSONL `{ "p": path, "g": pattern }` を
//! `glob::matches_glob` で評価し、各行に `1`/`0` を出力する。
//! `node probe-glob.cjs` と組み合わせて `path.posix.matchesGlob` と比較する。

use cheaper_tools::glob::matches_glob;
use std::io::{BufRead, BufWriter, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        let p = v["p"].as_str().unwrap();
        let g = v["g"].as_str().unwrap();
        let v = match matches_glob(p, g) {
            Ok(true) => "1",
            Ok(false) => "0",
            // Node が throw する case(over-long pattern 等)は driver 側で
            // 比較対象外になるため `-1` で印を付ける。
            Err(_) => "-1",
        };
        writeln!(out, "{}", v).unwrap();
    }
}
