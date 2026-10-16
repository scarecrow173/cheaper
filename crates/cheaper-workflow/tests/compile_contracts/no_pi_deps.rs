//! Port of `packages/workflow/test/no-pi-deps.test.ts`。
//!
//! TS の forbidden specifier:
//! - `@earendil-works/*`(Rig packages)→ Rust では `rig-*` crate(`rig_core`
//!   等、ident では `rig_` prefix)/`poc-rig`
//! - `@cheaper/runtime-pi` → `cheaper-runtime`(runtime 層)
//! - `cheaper`(cli package 名、exact match)→ `cheaper-cli`/`cheaper`
//! - `cheaper-rig-adapter` も runtime 置き換え対象層として禁止。
//!
//! `cheaper-core`/`cheaper-tools` は許可された依存(TS の
//! `@cheaper/core`/`@cheaper/tool-runtime` 相当)。

use std::fs;
use std::path::{Path, PathBuf};

/// Cargo.toml dependency name ベースの判定。
fn is_forbidden(name: &str) -> bool {
    name == "rig"
        || name.starts_with("rig-")
        || [
            "cheaper",
            "cheaper-cli",
            "cheaper-runtime",
            "cheaper-rig-adapter",
            "poc-rig",
        ]
        .contains(&name)
}

/// Rust ident(`_` 区切り)ベースの判定。
fn is_forbidden_ident(ident: &str) -> bool {
    is_forbidden(&ident.replace('_', "-"))
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn cargo_toml_dependencies_contain_no_forbidden_entries() {
    let manifest = crate_dir().join("Cargo.toml");
    let text = fs::read_to_string(&manifest).expect("read Cargo.toml");
    let mut in_deps = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line.contains("dependencies]");
            continue;
        }
        if !in_deps || line.starts_with('#') {
            continue;
        }
        let Some((name, _)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_matches('"');
        let name = name.split('.').next().unwrap_or(name);
        assert!(
            !is_forbidden(name),
            "forbidden dependency in {}: {name}",
            manifest.display()
        );
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readdir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `use <ident>`/`extern crate <ident>`/`<ident>::` path head の
/// specifier 検査(TS の `from "X"`/`import "X"` 相当)。
#[test]
fn src_files_use_no_forbidden_crate_specifiers() {
    let src = crate_dir().join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(!files.is_empty());
    for file in files {
        let text = fs::read_to_string(&file).expect("read src file");
        for line in text.lines() {
            let line = line.trim_start();
            for keyword in ["use ", "extern crate "] {
                let Some(rest) = line.strip_prefix(keyword) else {
                    continue;
                };
                // first path segment を crate ident として検査。
                let ident: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                assert!(
                    !is_forbidden_ident(&ident),
                    "{}: forbidden specifier `{keyword}{ident}`",
                    file.display()
                );
            }
        }
        // `use` 文を通さない qualified path(`cheaper_runtime::x` 等)。
        for ident in [
            "cheaper",
            "cheaper_cli",
            "cheaper_runtime",
            "cheaper_rig_adapter",
            "poc_rig",
            "rig_core",
        ] {
            let pattern = format!("{ident}::");
            assert!(
                !text.contains(pattern.as_str()),
                "{}: forbidden path head {pattern}",
                file.display()
            );
        }
    }
}
