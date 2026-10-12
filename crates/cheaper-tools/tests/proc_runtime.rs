//! `packages/tool-runtime/test/runtime.test.ts` 対応の integration test。
//!
//! `handlers`(write/edit/ls の injected handler)は Rust 版に存在しない
//! (全 8 tool が native)ため、同じ contract は native tool 実行で検証する:
//! - "rejects unregistered write/edit/ls" は「`names` 未登録 tool の
//!   execute が `UNKNOWN_TOOL`」として置き換える。
//! - budget 競合は native `write`(`write_scope` + manifest 認可)で検証する。
//!
//! grep eval は `proc_process.rs` と同じく、この test binary 自身を
//! `CHEAPER_GREP_EVAL` env marker 付きで re-exec する fixture 構成。

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use cheaper_core::errors::ErrorCode;
use cheaper_core::token::count::count_text_tokens;
use cheaper_core::tools::vocabulary::ToolName;
use cheaper_core::worker::schema::{compile_schema, SchemaKind};
use cheaper_core::workspace::deny_paths::create_path_denylist;
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::journal::MutationJournal;
use cheaper_tools::operation::OperationState;
use cheaper_tools::registry::{create_tool_runtime, ToolRuntimeOptions};
use cheaper_tools::tools::grep_eval::{EvalSpawn, HELPER_ENV};
use cheaper_tools::ToolError;
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// grep eval fixture(re-exec of this test binary)
// ---------------------------------------------------------------------------

/// `grep_eval::test_support::helper_entry` 相当(lib の test_support は
/// `cfg(test)` のため integration binary からは呼べない)。env marker が
/// 無い通常の `cargo test` では即 return する。
#[test]
fn grep_eval_helper_entry() {
    if std::env::var_os(HELPER_ENV).is_none() {
        return;
    }
    std::process::exit(cheaper_tools::tools::grep_eval::helper_main());
}

fn fixture_eval_spawn() -> EvalSpawn {
    EvalSpawn {
        program: std::env::current_exe().expect("current exe"),
        args: vec![
            "grep_eval_helper_entry".to_string(),
            "--exact".to_string(),
            "--test-threads=1".to_string(),
        ],
        envs: vec![(HELPER_ENV.to_string(), "1".to_string())],
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-runtime-it-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }

    fn root(&self) -> String {
        canonicalize_workspace_root(self.0.to_str().unwrap()).unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn options(
    root: String,
    names: Vec<ToolName>,
    on_limit: Box<dyn Fn() + Send + Sync>,
) -> ToolRuntimeOptions {
    ToolRuntimeOptions {
        resource_manifest: None,
        write_scope: None,
        read_strictness: None,
        artifacts: None,
        process: None,
        command: None,
        root,
        symlink_policy: SymlinkPolicy::Deny,
        names,
        max_tool_calls: 100,
        on_limit,
        journal: None,
        denylist: None,
        parent_lease: None,
        signal: None,
        abort_grace_ms: None,
        concurrency: None,
        eval_spawn: Some(fixture_eval_spawn()),
    }
}

fn code(err: &ToolError) -> Option<ErrorCode> {
    cheaper_tools::tools::process::find_cheaper_error(err.as_ref() as &dyn std::error::Error)
        .map(|e| e.code())
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

/// "exposes eight schemas without a workspace or handlers"。
#[test]
fn exposes_eight_schemas() {
    let catalog = cheaper_tools::catalog::create_tool_catalog(None).unwrap();
    let names: Vec<&str> = catalog.list().iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "read",
            "write",
            "edit",
            "ls",
            "grep",
            "find",
            "count_tokens",
            "process",
        ]
    );
    assert!(catalog.get("missing").is_none());
    for t in catalog.list() {
        assert_eq!(t.input_schema["type"], json!("object"));
        assert_eq!(t.output_schema["type"], json!("object"));
    }
}

/// "executes owned tools and retains schema defaults in handlers"。
#[tokio::test]
async fn executes_owned_tools() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "hello world\nsecond\n").unwrap();
    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![
                ToolName::Read,
                ToolName::Grep,
                ToolName::Find,
                ToolName::CountTokens,
            ],
            Box::new(|| {}),
        ),
        None,
    )
    .unwrap();
    let result = r
        .execute("read", &json!({"path": "a.txt"}), None)
        .await
        .unwrap();
    assert_eq!(result.content[0].text, "hello world\nsecond\n");
    let result = r
        .execute("grep", &json!({"pattern": "hello"}), None)
        .await
        .unwrap();
    assert!(result.content[0].text.contains("a.txt:1:"));
    let result = r
        .execute("find", &json!({"pattern": "*.txt"}), None)
        .await
        .unwrap();
    assert_eq!(result.content[0].text, "a.txt");
    let counted = r
        .execute("count_tokens", &json!({"path": "a.txt"}), None)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_str(&counted.content[0].text).unwrap();
    assert_eq!(counted.details.as_ref(), Some(&parsed));
    let output = catalog_output_schema(&r, "count_tokens");
    let compiled = compile_schema(output, SchemaKind::Output).unwrap();
    assert!(compiled
        .validate(&serde_json::to_value(&counted).unwrap())
        .is_ok());
    r.dispose().await;
}

fn catalog_output_schema(
    r: &cheaper_tools::registry::ToolRuntime,
    name: &str,
) -> Map<String, Value> {
    r.catalog()
        .get(name)
        .unwrap_or_else(|| panic!("missing {name} output schema"))
        .output_schema
        .as_object()
        .expect("object schema")
        .clone()
}

/// "rejects malformed %s at the canonical entrance" — INVALID_INPUT は
/// budget 予約・operation 作成の前に throw する。
#[tokio::test]
async fn rejects_malformed_at_entrance() {
    let dir = TempDir::new();
    let cases: [(&str, Value); 5] = [
        ("read", json!({"path": 1})),
        ("read", json!({"path": "a.txt", "limit": 2001})),
        ("grep", json!({"pattern": []})),
        ("find", json!({})),
        ("count_tokens", json!({"path": false})),
    ];
    for (name, input) in cases {
        let r = create_tool_runtime(
            options(
                dir.root(),
                vec![
                    ToolName::Read,
                    ToolName::Grep,
                    ToolName::Find,
                    ToolName::CountTokens,
                ],
                Box::new(|| {}),
            ),
            None,
        )
        .unwrap();
        let err = r.execute(name, &input, None).await.unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::InvalidInput), "{name} {input}");
        assert_eq!(r.budget().tool_calls(), 0, "{name} {input}");
        assert!(r.operations().is_empty(), "{name} {input}");
        r.dispose().await;
    }
}

/// "rejects unregistered write/edit/ls" — TS では handler 未注入の
/// write/edit/ls が `UNKNOWN_TOOL`。Rust は native tool のみのため、
/// `names` 未登録 tool の `execute` が `UNKNOWN_TOOL` になることを検証する。
#[tokio::test]
async fn rejects_unregistered_tool() {
    let dir = TempDir::new();
    for name in ["write", "edit", "ls"] {
        let r = create_tool_runtime(
            options(dir.root(), vec![ToolName::Read], Box::new(|| {})),
            None,
        )
        .unwrap();
        let err = r.execute(name, &json!({}), None).await.unwrap_err();
        assert_eq!(code(&err), Some(ErrorCode::UnknownTool), "{name}");
        assert!(r.operations().is_empty(), "{name}");
        r.dispose().await;
    }
}

/// "reserves sibling budget once and meters successful text once"。
/// `accounting.run` の同期 admission が sibling の await より先に
/// `toolCalls` を予約する(`join!` は宣言順に poll するため 1 件目が
/// fulfilled / 2 件目が limit reject になる)。
#[tokio::test]
async fn reserves_sibling_budget_once_and_meters_once() {
    let dir = TempDir::new();
    let on_limit_calls = Arc::new(AtomicUsize::new(0));
    let on_limit = {
        let calls = Arc::clone(&on_limit_calls);
        Box::new(move || {
            calls.fetch_add(1, Ordering::SeqCst);
        }) as Box<dyn Fn() + Send + Sync>
    };
    let mut opts = options(dir.root(), vec![ToolName::Write], on_limit);
    opts.max_tool_calls = 1;
    opts.write_scope = Some(vec!["a.txt".to_string()]);
    opts.resource_manifest = Some(json!({
        "writes": [{"path": "a.txt", "scope": "exact"}]
    }));
    let r = create_tool_runtime(opts, None).unwrap();
    let input_x = json!({"path": "a.txt", "content": "x"});
    let input_y = json!({"path": "a.txt", "content": "y"});
    let (a, b) = tokio::join!(
        r.execute("write", &input_x, None),
        r.execute("write", &input_y, None),
    );
    let result = a.expect("first call fulfilled");
    let err = b.expect_err("second call rejected");
    assert!(
        err.to_string().contains("maxToolCalls"),
        "expected maxToolCalls rejection, got {err}"
    );
    assert_eq!(r.budget().tool_calls(), 1);
    assert_eq!(r.budget().rejected_tool_calls(), 1);
    let expected = count_text_tokens(&result.content[0].text).unwrap();
    assert_eq!(r.meter().source_tokens(), expected);
    assert_eq!(on_limit_calls.load(Ordering::SeqCst), 1);
    let states: Vec<OperationState> = r.operations().iter().map(|op| op.state).collect();
    assert_eq!(states, [OperationState::Completed]);
    r.dispose().await;
}

/// "counts access rejects once without output tokens"。
#[tokio::test]
async fn counts_access_rejects_once() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "hello world\nsecond\n").unwrap();
    let mut opts = options(dir.root(), vec![ToolName::Read], Box::new(|| {}));
    opts.denylist = Some(create_path_denylist(&["a.txt"]));
    opts.journal = Some(Arc::new(MutationJournal::default()));
    let r = create_tool_runtime(opts, None).unwrap();
    let err = r
        .execute("read", &json!({"path": "a.txt"}), None)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("Access denied"),
        "expected access denied, got {err}"
    );
    assert_eq!(r.budget().tool_calls(), 1);
    assert_eq!(r.budget().rejected_tool_calls(), 1);
    assert_eq!(r.meter().source_tokens(), 0);
    let ops = r.operations();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].state, OperationState::Failed);
    let message = ops[0]
        .error
        .as_ref()
        .map(|e| e.message.clone())
        .unwrap_or_default();
    assert!(message.contains("Access denied"), "{message}");
    r.dispose().await;
}
