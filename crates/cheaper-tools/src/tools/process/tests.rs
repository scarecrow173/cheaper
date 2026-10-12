use super::monitor::{StreamCapture, StreamCaptureResult};
use super::spawn::display;
use serde_json::json;

use super::*;

#[test]
fn capture_finish_decodes_within_byte_cap() {
    let mut capture = StreamCapture::default();
    capture.append(&[b'a'; 10], 6);
    capture.append(&[b'b'; 10], 4);
    let done = capture.finish();
    assert_eq!(done.text, "aaaaaabbbb");
    assert_eq!(done.bytes, 20);
    assert_eq!(done.captured_bytes, 10);
    assert!(done.truncated);
}

#[test]
fn capture_finish_drops_incomplete_trailing_codepoint() {
    // stream:true decode は不完全な末尾 sequence を捨てる
    // (replacement 文字すら出さない)。
    let mut capture = StreamCapture::default();
    let input = "日本語".as_bytes(); // 9 bytes / 3 chars
    capture.append(input, 7); // "日本" + 語 の先頭 1 byte
    let done = capture.finish();
    assert_eq!(done.text, "日本");
    assert!(done.truncated);
}

#[test]
fn capture_finish_marks_replacement_expansion_as_truncated() {
    let mut capture = StreamCapture::default();
    capture.append(&[0xff, 0xff], 2); // invalid → U+FFFD に拡大
    let done = capture.finish();
    assert_eq!(done.bytes, 2);
    assert_eq!(done.captured_bytes, 2);
    assert!(done.truncated); // encoded(3) > captured(2)
}

#[test]
fn display_folds_at_64_utf16_units() {
    let text = "a".repeat(130);
    let value = StreamCaptureResult {
        text,
        bytes: 130,
        captured_bytes: 130,
        truncated: false,
    };
    let shown = display(&value);
    let body = shown.split('\n').skip(1).collect::<Vec<_>>();
    assert_eq!(body.len(), 3);
    assert_eq!(body[0].len(), 64);
    assert_eq!(body[1].len(), 64);
    assert_eq!(body[2].len(), 2);
    assert!(shown.starts_with("130 bytes, 130 captured, truncated=false\n"));
}

#[test]
fn display_does_not_split_surrogate_pair() {
    // 63 ascii + 1 BMP外 char(2 units)= 65 units → pair は次行へ。
    let text = format!("{}😀x", "a".repeat(63));
    let value = StreamCaptureResult {
        text,
        bytes: 0,
        captured_bytes: 0,
        truncated: false,
    };
    let shown = display(&value);
    let body: Vec<&str> = shown.split('\n').skip(1).collect();
    assert_eq!(body[0].chars().count(), 63);
    assert_eq!(body[1], "😀x");
}

#[test]
fn minimal_env_keeps_only_explicit_on_posix() {
    if cfg!(windows) {
        return;
    }
    let mut explicit = Map::new();
    explicit.insert("ONLY_THIS".to_string(), json!("yes"));
    let env = minimal_env(&explicit);
    assert_eq!(env, vec![("ONLY_THIS".to_string(), "yes".to_string())]);
}

#[cfg(windows)]
#[test]
fn minimal_env_windows_required_env_and_systemroot() {
    let mut explicit = Map::new();
    explicit.insert("path".to_string(), json!("custom")); // → PATH へ upper
    explicit.insert("only_this".to_string(), json!("yes"));
    let env = minimal_env(&explicit);
    let get = |name: &str| env.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
    // 明示 env は upper-case 化される。
    assert_eq!(get("PATH"), Some("custom"));
    assert_eq!(get("ONLY_THIS"), Some("yes"));
    // required env は未指定時に空値で遮断される(PATH は明示済み)。
    for key in [
        "HOMEDRIVE",
        "HOMEPATH",
        "LOGONSERVER",
        "SYSTEMDRIVE",
        "TEMP",
        "USERDOMAIN",
        "USERNAME",
        "USERPROFILE",
        "WINDIR",
    ] {
        assert_eq!(get(key), Some(""), "{key}");
    }
    // SystemRoot は親値 or "C:\Windows" で補完。
    let expected = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    assert_eq!(get("SYSTEMROOT"), Some(expected.as_str()));
}

#[test]
fn operation_declaration_derives_access_from_outputs() {
    let policy = cheaper_core::policy::process::resolve_process_policy(Some(&json!({
        "maxConcurrent": 3,
        "tasks": {
            "ro": {"command": ["cc"], "inputs": [], "outputs": []},
            "wr": {"command": ["cc"], "inputs": [], "outputs": ["out"]},
        },
    })))
    .unwrap();
    let ro = process_operation_declaration(&policy, "ro").unwrap();
    assert_eq!(ro.access, cheaper_core::resources::types::ClaimMode::Shared);
    let pool = ro.pool.as_ref().expect("process pool");
    assert_eq!(pool.name, "process");
    assert_eq!(pool.max_concurrent, 3);
    let wr = process_operation_declaration(&policy, "wr").unwrap();
    assert_eq!(
        wr.access,
        cheaper_core::resources::types::ClaimMode::Exclusive
    );
    let err = process_operation_declaration(&policy, "nope").unwrap_err();
    assert_eq!(err.code(), ErrorCode::PolicyViolation);
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "cheaper-proc-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }

    fn root(&self) -> String {
        cheaper_core::workspace::paths::canonicalize_workspace_root(self.0.to_str().unwrap())
            .unwrap()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn policy(tasks: Value) -> ResolvedProcessPolicy {
    cheaper_core::policy::process::resolve_process_policy(Some(&json!({
        "tasks": tasks,
    })))
    .unwrap()
}

fn tool(root: String, tasks: Value) -> ProcessTool {
    create_process_tool(ProcessToolOptions {
        root,
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(tasks),
        on_timeout: None,
        denylist: None,
        write_scope: None,
    })
}

fn cheaper_code(err: &ToolError) -> Option<ErrorCode> {
    find_cheaper_error(err.as_ref()).map(|e| e.code())
}

#[tokio::test]
async fn unknown_task_is_policy_violation_before_spawn() {
    let dir = TempDir::new();
    let tool = tool(
        dir.root(),
        json!({"check": {"command": ["cc"], "inputs": [], "outputs": []}}),
    );
    let err = tool
        .execute(&json!({"task": "missing"}), None)
        .await
        .unwrap_err();
    assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
}

#[tokio::test]
async fn prepare_rejects_missing_cwd() {
    let dir = TempDir::new();
    let tool = tool(
        dir.root(),
        json!({"check": {
            "command": ["cc"], "inputs": [], "outputs": [], "cwd": "missing",
        }}),
    );
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    // TS parity: `stat` の ENOENT は CheaperError で wrap せず raw
    // error で伝播する(isDirectory=false のみ INVALID_REQUEST)。
    assert_eq!(cheaper_code(&err), None);
    assert!(err.is::<std::io::Error>() || err.source().is_some());
}

#[tokio::test]
async fn prepare_rejects_denied_output_and_write_scope_excess() {
    let dir = TempDir::new();
    let root = dir.root();
    // declared output が denylist に一致 → deny。
    let denylist = cheaper_core::workspace::deny_paths::create_path_denylist(&[".cheaper/**"]);
    let tool = create_process_tool(ProcessToolOptions {
        root: root.clone(),
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(json!({"check": {
            "command": ["cc"], "inputs": [], "outputs": [".cheaper/out"],
        }})),
        on_timeout: None,
        denylist: Some(denylist),
        write_scope: None,
    });
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
    // writeScope 外の output → POLICY_VIOLATION。
    let tool = create_process_tool(ProcessToolOptions {
        root,
        symlink_policy: SymlinkPolicy::Deny,
        process: policy(json!({"check": {
            "command": ["cc"], "inputs": [], "outputs": ["out"],
        }})),
        on_timeout: None,
        denylist: None,
        write_scope: Some(HashSet::from(["allowed".to_string()])),
    });
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    assert_eq!(cheaper_code(&err), Some(ErrorCode::PolicyViolation));
}

/// `spawn_code`: raw OS errno → Node/libuv `error.code` 名の写像。
/// unmapped errno と errno 無しの構築 error は kind fallback へ。
#[test]
fn spawn_code_maps_raw_errno_to_libuv_names() {
    #[cfg(unix)]
    {
        for (errno, want) in [
            (libc::ENOENT, "ENOENT"),
            (libc::EACCES, "EACCES"),
            (libc::ENOEXEC, "ENOEXEC"),
            (libc::E2BIG, "E2BIG"),
            (libc::ENXIO, "ENXIO"),
            (libc::ETXTBSY, "ETXTBSY"),
            // mapping に無い errno は kind fallback(ENOTSUP →
            // Uncategorized → SPAWN_ERROR)。
            (libc::ENOTSUP, "SPAWN_ERROR"),
        ] {
            assert_eq!(
                spawn_code(&std::io::Error::from_raw_os_error(errno)),
                want,
                "errno {errno}"
            );
        }
    }
    #[cfg(windows)]
    {
        for (errno, want) in [
            (2, "ENOENT"),     // ERROR_FILE_NOT_FOUND(Node 実測 async error)
            (3, "ENOENT"),     // ERROR_PATH_NOT_FOUND
            (5, "EACCES"),     // ERROR_ACCESS_DENIED
            (32, "EACCES"),    // ERROR_SHARING_VIOLATION
            (193, "EFTYPE"),   // ERROR_BAD_EXE_FORMAT(Node 実測)
            (216, "UNKNOWN"),  // ERROR_EXE_MACHINE_TYPE_MISMATCH(実測)
            (267, "ENOTDIR"),  // ERROR_DIRECTORY
            (9999, "UNKNOWN"), // unmapped win32 → UV__UNKNOWN
        ] {
            assert_eq!(
                spawn_code(&std::io::Error::from_raw_os_error(errno)),
                want,
                "errno {errno}"
            );
        }
    }
    // raw errno を持たない構築 error: kind による fallback。
    assert_eq!(
        spawn_code(&std::io::Error::new(std::io::ErrorKind::NotFound, "x")),
        "ENOENT"
    );
    assert_eq!(
        spawn_code(&std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "x"
        )),
        "EACCES"
    );
    assert_eq!(spawn_code(&std::io::Error::other("x")), "SPAWN_ERROR");
}

/// `spawn_failure_is_async`: unix は exec 失敗が error pipe 経由で
/// 常に非同期(`"managed process failed"` shape)。win32 は ENOENT の
/// み非同期(Node 実測)、その他は sync throw 経路。
#[test]
fn spawn_failure_async_shape_follows_node() {
    #[cfg(unix)]
    {
        for errno in [libc::ENOENT, libc::EACCES, libc::ENOEXEC] {
            assert!(spawn_failure_is_async(&std::io::Error::from_raw_os_error(
                errno
            )));
        }
    }
    #[cfg(windows)]
    {
        assert!(spawn_failure_is_async(&std::io::Error::from_raw_os_error(
            2
        )));
        assert!(!spawn_failure_is_async(&std::io::Error::from_raw_os_error(
            216
        )));
        assert!(!spawn_failure_is_async(&std::io::Error::from_raw_os_error(
            193
        )));
        assert!(!spawn_failure_is_async(&std::io::Error::from_raw_os_error(
            5
        )));
    }
    assert!(!spawn_failure_is_async(&std::io::Error::other("x")));
}

/// magic check(MZ)を通すが exec 不可能な executable。
/// `ERROR_EXE_MACHINE_TYPE_MISMATCH`(216)→ libuv 写像 `"UNKNOWN"`、
/// Node は非 ENOENT error を sync throw するため
/// `"managed process spawn failed"` + `spawnCode` + cause 相当。
/// (unix 側は `execvpe` の ENOEXEC→`/bin/sh` fallback が干渉するため
/// execute-level 検証は win32 のみ — mapping/shape 規則は unit test
/// で別途担保する。)
#[cfg(windows)]
#[tokio::test]
async fn spawn_errno_failure_reports_libuv_style_code() {
    let dir = TempDir::new();
    let bogus = dir.0.join("bogus.exe");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&bogus).unwrap();
        f.write_all(b"MZnot-a-real-pe").unwrap();
    }
    let tool = tool(
        dir.root(),
        json!({"check": {"command": ["./bogus.exe"], "inputs": [], "outputs": []}}),
    );
    let err = tool
        .execute(&json!({"task": "check"}), None)
        .await
        .unwrap_err();
    let ce = find_cheaper_error(err.as_ref()).expect("CheaperError");
    assert_eq!(ce.code(), ErrorCode::ToolExecutionFailure);
    let details = ce.details().expect("details");
    assert_eq!(ce.message(), "managed process spawn failed");
    // 216(machine type mismatch)は libuv の unmapped → "UNKNOWN"。
    assert_eq!(details["spawnCode"], json!("UNKNOWN"));
    assert_eq!(details["task"], json!("check"));
    assert_eq!(details["state"], json!("failed"));
}

#[tokio::test]
async fn pre_spawn_abort_rejects_with_outer_reason() {
    let dir = TempDir::new();
    let tool = tool(
        dir.root(),
        json!({"check": {"command": ["cc"], "inputs": [], "outputs": []}}),
    );
    let controller = AbortController::new();
    controller.abort(AbortReason::from(CheaperError::new(
        ErrorCode::Aborted,
        "outer wins",
    )));
    let err = tool
        .execute(&json!({"task": "check"}), Some(controller.signal()))
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "outer wins");
    assert_eq!(cheaper_code(&err), Some(ErrorCode::Aborted));
}
