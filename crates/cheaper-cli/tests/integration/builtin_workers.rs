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
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::resolve::resolve_worker;
use cheaper_core::worker::validate::{validate_worker_input, WorkerDefinition};
use serde_json::json;

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
    assert!(
        names.contains(&"condense".to_string()),
        "condense.md missing from builtin workers: {names:?}"
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

/// builtin worker schema は magnitude（文字数・件数）ではなく構造のみを
/// 検証する（issue #113/#114 — spec `worker-schema-structure-not-magnitude`）。
/// 旧上限値を超える入力/出力の受理と、構造制約の存続を pin する。
#[test]
fn builtin_schemas_validate_structure_not_magnitude() {
    let builtin = common::builtin_dir();
    let tmp = tempfile::tempdir().unwrap();
    let dirs = ConfigDirs {
        user_config_dir: tmp.path().join("user"),
        project_dir: tmp.path().join("project"),
        builtin_dir: builtin,
    };
    let resolve = |name: &str| -> WorkerDefinition {
        resolve_worker(name, &dirs).unwrap_or_else(|e| panic!("resolve {name}: {e}"))
    };
    let input_ok = |def: &WorkerDefinition, input: serde_json::Value| {
        validate_worker_input(def, &input)
            .unwrap_or_else(|e| panic!("{}: input rejected: {e:?}", def.name))
    };
    let input_err = |def: &WorkerDefinition, input: serde_json::Value| {
        assert!(
            validate_worker_input(def, &input).is_err(),
            "{}: input unexpectedly accepted: {input}",
            def.name
        )
    };

    // input magnitude 撤廃 — 旧上限(4000/6000)を超える task を受理。
    let long_task = "t".repeat(10_000);
    let bulk = resolve("bulk-reader");
    input_ok(&bulk, json!({"task": long_task}));
    input_ok(
        &bulk,
        json!({"task": "t", "paths": vec![json!("src/a.rs"); 100]}),
    );
    // `maxFindings` — 省略 / null / 大きな値を受理、0 は minimum 違反。
    input_ok(&bulk, json!({"task": "t", "maxFindings": null}));
    input_ok(&bulk, json!({"task": "t", "maxFindings": 500}));
    input_err(&bulk, json!({"task": "t", "maxFindings": 0}));

    let code = resolve("code-writer");
    input_ok(
        &code,
        json!({"task": "t".repeat(10_000), "targets": [{"path": "src/a.rs", "intent": "modify"}]}),
    );
    let doc = resolve("doc-writer");
    input_ok(
        &doc,
        json!({"task": "t".repeat(10_000), "documentType": "guide", "targets": [{"path": "docs/a.md", "intent": "create"}]}),
    );

    let tool = resolve("tool-operator");
    input_ok(
        &tool,
        json!({"task": "t".repeat(10_000), "output": {"maxFindings": null}}),
    );
    input_ok(&tool, json!({"task": "t", "output": {"maxFindings": 100}}));
    input_err(&tool, json!({"task": "t", "output": {"maxFindings": 0}}));

    // 構造制約は維持 — 絶対 path pattern・`additionalProperties`・`minLength`。
    input_err(
        &code,
        json!({"task": "t", "targets": [{"path": "/abs/x.rs", "intent": "modify"}]}),
    );
    input_err(&bulk, json!({"task": "t", "bogus": 1}));
    input_err(&bulk, json!({"task": ""}));

    // output magnitude 撤廃 — 旧上限(120/1200/600/30件/1500)超過を受理。
    let finding = json!({
        "title": "t".repeat(200),
        "detail": "d".repeat(5_000),
        "confidence": "high",
        "locations": [{"path": "src/a.rs", "excerpt": "e".repeat(2_000)}],
    });
    bulk.output_schema
        .validate(&json!({
            "summary": "s".repeat(4_000),
            "findings": vec![finding; 40],
            "relevantFiles": vec![json!({"path": "src/a.rs", "reason": "r"}); 50],
            "suggestedReads": [],
            "openQuestions": [],
        }))
        .unwrap_or_else(|e| panic!("bulk-reader output rejected: {e:?}"));

    // 構造制約は維持 — required 欠落・enum 違反。
    assert!(bulk
        .output_schema
        .validate(&json!({
            "summary": "s",
            "findings": [],
            "relevantFiles": [],
            "suggestedReads": [],
        }))
        .is_err());
    assert!(bulk
        .output_schema
        .validate(&json!({
            "summary": "s",
            "findings": [{
                "title": "t", "detail": "d",
                "confidence": "bogus", "locations": [],
            }],
            "relevantFiles": [],
            "suggestedReads": [],
            "openQuestions": [],
        }))
        .is_err());

    for name in ["code-writer", "doc-writer"] {
        let def = resolve(name);
        def.output_schema
            .validate(&json!({
                "summary": "s".repeat(4_000),
                "notes": vec![json!("n".repeat(2_000)); 40],
                "unresolved": [],
            }))
            .unwrap_or_else(|e| panic!("{name} output rejected: {e:?}"));
    }

    tool.output_schema
        .validate(&json!({
            "status": "success",
            "summary": "s".repeat(4_000),
            "findings": [],
            "unresolved": [],
            "nextActions": [],
        }))
        .unwrap_or_else(|e| panic!("tool-operator output rejected: {e:?}"));
    // `grantRequests` の policy 連動上限は維持。
    assert!(tool
        .output_schema
        .validate(&json!({
            "status": "success",
            "summary": "s",
            "findings": [],
            "unresolved": [],
            "nextActions": [],
            "grantRequests": vec![json!({"executable": "sh"}); 21],
        }))
        .is_err());
    assert!(tool
        .output_schema
        .validate(&json!({
            "status": "success",
            "summary": "s",
            "findings": [],
            "unresolved": [],
            "nextActions": [],
            "grantRequests": [{"executable": "sh", "name": "Bad-Name!"}],
        }))
        .is_err());

    let condense = resolve("condense");
    assert_eq!(condense.tools, vec![ToolName::Read]);
    input_ok(
        &condense,
        json!({
            "logRef": "1700000000000-1234-deadbeef",
            "originalTokens": 100,
            "command": ["cargo", "test"],
        }),
    );
    input_err(
        &condense,
        json!({
            "logRef": "not-a-run-id",
            "originalTokens": 100,
            "command": ["cargo", "test"],
        }),
    );
    input_err(
        &condense,
        json!({
            "logRef": "1700000000000-1234-deadbeef",
            "originalTokens": 100,
            "command": ["cargo", "test"],
            "env": {"SECRET": "x"},
        }),
    );
    condense
        .output_schema
        .validate(&json!({"summary": "s"}))
        .unwrap_or_else(|e| panic!("condense output rejected: {e:?}"));
    assert!(condense.output_schema.validate(&json!({})).is_err());
}
