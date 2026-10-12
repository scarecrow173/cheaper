//! `tools/process/tests.rs` から移設した shared 機構の unit test 群
//! (`capture_finish_*`・`display_*`・`minimal_env_*`・`spawn_code_*`/
//! `spawn_failure_*`)。process 固有 glue の test は `tools::process::tests`
//! に残す。

use serde_json::{json, Map};

use super::minimal_env;
use super::monitor::{StreamCapture, StreamCaptureResult};
use super::spawn::{display, spawn_code, spawn_failure_is_async};

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
