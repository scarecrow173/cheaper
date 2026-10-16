//! builtin worker 定義の load test — `resources/workers/*.md` 全件が
//! `resolve_worker`(worker parse + input/output schema compile を含む)
//! で解決できることを pin する。
//!
//! `tool-operator` を含む builtin class 全体を対象にするため、新規
//! worker の追加が frontmatter strict-key や schema の strict-mode
//! 規約を破るとここで fail する(T3→T1 依存 — `optionalTools` key
//! 未実装だと strict-key reject で load 不能 — を機械的に enforce
//! する側でもある)。

use crate::common;

use cheaper_core::config::dirs::ConfigDirs;
use cheaper_core::worker::resolve::resolve_worker;

#[test]
fn all_builtin_workers_resolve_with_schemas() {
    let builtin = common::builtin_dir();
    let workers_dir = builtin.join("workers");
    let mut names: Vec<String> = std::fs::read_dir(&workers_dir)
        .unwrap_or_else(|e| panic!("cannot list {}: {e}", workers_dir.display()))
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            path.is_file()
                .then(|| {
                    (path.extension().is_some_and(|ext| ext == "md"))
                        .then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
                })
                .flatten()
        })
        .collect();
    names.sort();
    assert!(
        names.contains(&"tool-operator".to_string()),
        "tool-operator.md missing from builtin workers: {names:?}"
    );

    // project/user dir は空 — builtin 解決のみを検証する。
    let tmp = tempfile::tempdir().unwrap();
    let dirs = ConfigDirs {
        user_config_dir: tmp.path().join("user"),
        project_dir: tmp.path().join("project"),
        builtin_dir: builtin,
    };

    for name in names {
        let def = resolve_worker(&name, &dirs)
            .unwrap_or_else(|e| panic!("builtin worker {name} failed to resolve: {e}"));
        // `resolve_worker` 内部で input/output schema の load +
        // strict-mode compile 済み(output は必須、input は宣言時のみ)。
        assert_eq!(def.name.as_str(), name);
        if def.cheaper.input.is_some() {
            assert!(def.input_schema.is_some(), "{name}: input schema");
        }
    }
}
