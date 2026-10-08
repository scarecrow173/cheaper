//! `packages/cli/package.json` の version から SEA VFS `version.json`
//! 相当を生成する(canonical version source は TS 側の package.json の
//! まま — `scripts/check-version.mjs` の contract と整合)。
//!
//! 生成物は `OUT_DIR/version.json` で、`src/embedded.rs` が
//! `include_str!` で埋め込む。SEA bundle が `version.json` を優先する
//! TS `packageVersion()` の解決順(version.json → package.json)を
//! Rust binary でも再現するためのもの。

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pkg_path = manifest_dir.join("../../packages/cli/package.json");
    println!("cargo:rerun-if-changed={}", pkg_path.display());
    let text =
        std::fs::read_to_string(&pkg_path).expect("read packages/cli/package.json for version");
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("packages/cli/package.json is valid JSON");
    let version = parsed
        .get("version")
        .and_then(serde_json::Value::as_str)
        .expect("packages/cli/package.json has a string version");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(
        out_dir.join("version.json"),
        format!("{{\"version\":\"{version}\"}}\n"),
    )
    .expect("write generated version.json");
}
