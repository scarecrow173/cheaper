//! Characterization test — T11 構造分割(tools/{process,grep,read} の
//! module 分割)に先行して、canonical `ToolRuntime::execute` JSON 経路の
//! 観測可能な契約を pin する。
//!
//! 対象は分割後に壊れやすい境界の挙動:
//! - grep の走査停止理由 footer marker(`[truncated: ...]`)
//! - read の pagination details(`truncatedBy`/`continuation`)と
//!   binary/escape の拒否
//! - access boundary(denylist/write scope)の tool-level 拒否
//! - abort signal の pre-aborted settlement(cancellation settle)
//!
//! 内部 unit test(src/*/tests mod)・differential fixture が既に pin
//! している詳細(operation 遷移表・lease counter・spawn errno 等)は
//! ここでは重複しない。

use std::path::PathBuf;

use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use cheaper_tools::operation::AbortController;
use cheaper_tools::registry::{create_tool_runtime, ToolCallContext, ToolRuntimeOptions};
use cheaper_tools::tools::grep_eval::{EvalSpawn, HELPER_ENV};
use serde_json::json;

/// grep eval はこの test binary 自身を `CHEAPER_GREP_EVAL` marker 付きで
/// re-exec する(proc_runtime.rs と同じ fixture 構成)。
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

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-chr-{}-{}",
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
    names: Vec<cheaper_core::tools::vocabulary::ToolName>,
) -> ToolRuntimeOptions {
    ToolRuntimeOptions {
        resource_manifest: None,
        write_scope: None,
        read_strictness: None,
        artifacts: None,
        process: None,
        root,
        symlink_policy: SymlinkPolicy::Deny,
        names,
        max_tool_calls: 100,
        on_limit: Box::new(|| {}),
        journal: None,
        denylist: None,
        parent_lease: None,
        signal: None,
        abort_grace_ms: None,
        concurrency: None,
        eval_spawn: Some(fixture_eval_spawn()),
    }
}

// ---------------------------------------------------------------------------
// grep — stop reason footer markers(TS `stopReason` → text output)
// ---------------------------------------------------------------------------

/// `maxResults` 超過は `[truncated: more matches]` で閉じ、結果自体は
/// 表示行 format(file:line: text)を保つ。
#[tokio::test]
async fn grep_result_cap_emits_more_matches_marker() {
    let dir = TempDir::new();
    let body: Vec<String> = (0..10).map(|i| format!("m{i}")).collect();
    std::fs::write(dir.0.join("many.txt"), body.join("\n") + "\n").unwrap();

    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Grep],
        ),
        None,
    )
    .unwrap();
    let out = r
        .execute("grep", &json!({"pattern": "m\\d", "maxResults": 3}), None)
        .await
        .unwrap();
    let text = &out.content[0].text;
    assert_eq!(
        text,
        "many.txt:1: m0\nmany.txt:2: m1\nmany.txt:3: m2\n[truncated: more matches]"
    );
    r.dispose().await;
}

/// output byte cap(256KiB)超過は `[truncated: output limit reached]`。
#[tokio::test]
async fn grep_output_cap_emits_output_limit_marker() {
    let dir = TempDir::new();
    // 各行は 500 utf16 unit に fold(GREP_OUTPUT_LINE_MAX_CHARS)される
    // ため、表示行あたり ~530byte。maxResults 上限(500)まで出すと
    // 500 × ~530 ≒ 265KiB > GREP_OUTPUT_MAX_BYTES(256KiB) で
    // Output stop reason が ResultCap に先行する。
    let line = "x".repeat(600);
    let body = (0..600)
        .map(|_| line.clone())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(dir.0.join("big.txt"), body).unwrap();

    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Grep],
        ),
        None,
    )
    .unwrap();
    let out = r
        .execute("grep", &json!({"pattern": "x+", "maxResults": 500}), None)
        .await
        .unwrap();
    let text = &out.content[0].text;
    assert!(
        text.contains("[truncated: output limit reached]"),
        "...{} (len={})",
        &text[text.len().saturating_sub(300)..],
        text.len()
    );
    r.dispose().await;
}

/// 500 utf16 unit 超の行は `... [truncated]` 付きで fold される。
#[tokio::test]
async fn grep_long_line_is_folded_with_truncated_marker() {
    let dir = TempDir::new();
    let long = "a".repeat(600);
    std::fs::write(dir.0.join("long.txt"), format!("{long}\n")).unwrap();

    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Grep],
        ),
        None,
    )
    .unwrap();
    let out = r
        .execute("grep", &json!({"pattern": "a+"}), None)
        .await
        .unwrap();
    let text = &out.content[0].text;
    assert!(text.contains("... [truncated]"), "{text}");
    assert!(!text.contains(&long), "folded line must not emit verbatim");
    r.dispose().await;
}

/// 不一致は `(no matches)` marker。
#[tokio::test]
async fn grep_no_matches_emits_marker() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "nothing here\n").unwrap();

    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Grep],
        ),
        None,
    )
    .unwrap();
    let out = r
        .execute("grep", &json!({"pattern": "zzz"}), None)
        .await
        .unwrap();
    assert_eq!(out.content[0].text, "(no matches)");
    r.dispose().await;
}

// ---------------------------------------------------------------------------
// read — pagination details / binary・escape 拒否
// ---------------------------------------------------------------------------

/// limit 打ち切りは `truncatedBy: "limit"` + `continuation.offset` +
/// `[Showing lines N-M. Use offset=... to continue.]` trailer を伴い、
/// 継続 offset からの read が次窓を返す。
#[tokio::test]
async fn read_limit_truncation_details_and_continuation() {
    let dir = TempDir::new();
    let body: Vec<String> = (1..=10).map(|i| format!("line{i}")).collect();
    std::fs::write(dir.0.join("f.txt"), body.join("\n") + "\n").unwrap();

    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Read],
        ),
        None,
    )
    .unwrap();
    let first = r
        .execute("read", &json!({"path": "f.txt", "limit": 3}), None)
        .await
        .unwrap();
    let d = first.details.as_ref().unwrap();
    assert_eq!(d["truncatedBy"], json!("limit"));
    assert_eq!(d["continuation"]["offset"], json!(4));
    assert!(
        first.content[0]
            .text
            .contains("[Showing lines 1-3. Use offset=4 to continue.]"),
        "{}",
        first.content[0].text
    );

    let second = r
        .execute(
            "read",
            &json!({"path": "f.txt", "offset": 4, "limit": 2}),
            None,
        )
        .await
        .unwrap();
    assert!(
        second.content[0].text.starts_with("line4"),
        "{}",
        second.content[0].text
    );
    r.dispose().await;
}

/// 先頭 8KiB に NUL を含む file は binary error。
#[tokio::test]
async fn read_binary_nul_rejected() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("b.bin"), b"a\0b").unwrap();

    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Read],
        ),
        None,
    )
    .unwrap();
    let err = r
        .execute("read", &json!({"path": "b.bin"}), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("binary file"), "{err}");
    r.dispose().await;
}

// ---------------------------------------------------------------------------
// access boundary — workspace escape は tool-level で拒否
// ---------------------------------------------------------------------------

/// `path` が workspace 外を指す read は access boundary で拒否される
/// (escape は観測されない — observation ではなく denial)。
#[tokio::test]
async fn read_workspace_escape_is_denied() {
    let dir = TempDir::new();
    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Read],
        ),
        None,
    )
    .unwrap();
    let err = r
        .execute("read", &json!({"path": "../outside.txt"}), None)
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
    r.dispose().await;
}

/// grep でも同じく workspace 外 path は拒否される。
#[tokio::test]
async fn grep_workspace_escape_is_denied() {
    let dir = TempDir::new();
    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Grep],
        ),
        None,
    )
    .unwrap();
    let err = r
        .execute("grep", &json!({"pattern": "x", "path": ".."}), None)
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
    r.dispose().await;
}

// ---------------------------------------------------------------------------
// cancellation settlement — pre-aborted signal は実行せず settle する
// ---------------------------------------------------------------------------

/// pre-aborted signal を載せた execute は tool 本体を走らせず
/// abort 理由付きで失敗する(first-reason semantics の境界)。
#[tokio::test]
async fn pre_aborted_signal_settles_without_executing() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("a.txt"), "hello\n").unwrap();

    let r = create_tool_runtime(
        options(
            dir.root(),
            vec![cheaper_core::tools::vocabulary::ToolName::Read],
        ),
        None,
    )
    .unwrap();
    let controller = AbortController::new();
    controller.abort(cheaper_tools::operation::AbortReason::aborted());
    let err = r
        .execute(
            "read",
            &json!({"path": "a.txt"}),
            Some(ToolCallContext {
                signal: Some(controller.signal()),
            }),
        )
        .await
        .unwrap_err();
    assert!(!err.to_string().is_empty());
    r.dispose().await;
}
