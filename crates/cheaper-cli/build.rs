//! `CARGO_PKG_VERSION` から SEA VFS `version.json` 相当を生成する。
//! crate version は `version.workspace = true` で root Cargo.toml の
//! `[workspace.package].version`(canonical version source)を継承する
//! ため、binary の `--version` 出力と同じ宣言に由来する
//! (`scripts/check-version.mjs` の contract と整合)。
//!
//! 生成物は `OUT_DIR/version.json` で、`src/embedded.rs` が
//! `include_str!` で埋め込む。SEA bundle が `version.json` を優先する
//! TS `packageVersion()` の解決順(version.json → package.json)を
//! Rust binary でも再現するためのもの。

use std::path::PathBuf;

fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION");
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // workspace manifest 上の canonical version が変わった際に build
    // script を再実行させる(version.workspace 継承元)。
    let workspace_manifest = manifest_dir.join("../../Cargo.toml");
    println!("cargo:rerun-if-changed={}", workspace_manifest.display());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(
        out_dir.join("version.json"),
        format!("{{\"version\":\"{version}\"}}\n"),
    )
    .expect("write generated version.json");
}
