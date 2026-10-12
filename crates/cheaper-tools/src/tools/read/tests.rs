use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use cheaper_core::workspace::deny_paths::{create_path_denylist, PathDenylist};
use cheaper_core::workspace::paths::{canonicalize_workspace_root, SymlinkPolicy};
use serde_json::json;

use super::*;
use crate::artifact::{
    ArtifactData, ArtifactPut, ArtifactReadRequest, ArtifactReadResult, ArtifactReader,
    ArtifactReference, ArtifactSource,
};

fn tmpdir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cheaper-read-{}", fastrand::u64(..)));
    fs::create_dir_all(&dir).expect("tmpdir");
    dir
}

fn read_opts<'a>(root: &'a str, denylist: Option<&'a PathDenylist>) -> ReadToolOptions<'a> {
    ReadToolOptions {
        artifacts: None,
        owner_id: None,
        observe_access: None,
        root,
        symlink_policy: SymlinkPolicy::Deny,
        denylist,
        max_bytes: None,
        chunk_bytes: None,
    }
}

fn text_of(result: &CanonicalToolResult) -> &str {
    &result.content[0].text
}

#[test]
fn fs_read_lines_offset_limit() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    let body = (1..=10)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(dir.join("f.txt"), &body).unwrap();
    let tool = create_read_tool(read_opts(&root, None));
    let result = tool
        .execute(&json!({"path": "f.txt", "offset": 3, "limit": 2}), None)
        .unwrap();
    assert_eq!(
        text_of(&result),
        "line3\nline4\n[Showing lines 3-4. Use offset=5 to continue.]"
    );
    let d = result.details.as_ref().unwrap();
    assert_eq!(d["startLine"], 3);
    assert_eq!(d["endLine"], 4);
    assert_eq!(d["eof"], false);
    assert_eq!(d["truncatedBy"], "limit");
    assert_eq!(d["continuation"]["offset"], 5);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_eof_total_lines() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join("f.txt"), "a\nb\n").unwrap();
    let tool = create_read_tool(read_opts(&root, None));
    let result = tool.execute(&json!({"path": "f.txt"}), None).unwrap();
    // split("\n") 意味論: "a\nb\n" → ["a","b",""] の末尾 empty は 3 行目。
    assert_eq!(text_of(&result), "a\nb\n");
    let d = result.details.as_ref().unwrap();
    assert_eq!(d["eof"], true);
    assert_eq!(d["totalLines"], 3);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_offset_beyond_eof_reports_total() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join("f.txt"), "a\nb\n").unwrap();
    let tool = create_read_tool(read_opts(&root, None));
    let err = tool
        .execute(&json!({"path": "f.txt", "offset": 9}), None)
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "Offset 9 is beyond end of file (3 lines total)"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_oversized_line_continuation() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    let long = "x".repeat(100);
    fs::write(dir.join("f.txt"), &long).unwrap();
    let mut opts = read_opts(&root, None);
    opts.max_bytes = Some(30);
    let tool = create_read_tool(opts);
    let result = tool.execute(&json!({"path": "f.txt"}), None).unwrap();
    let d = result.details.as_ref().unwrap();
    assert_eq!(d["truncatedBy"], "bytes");
    let byte_in_line = d["continuation"]["byteInLine"].as_u64().unwrap() as usize;
    assert!(byte_in_line > 0);
    let emitted = &result.content[0].text;
    let (body, trailer) = emitted.split_once('\n').unwrap();
    assert_eq!(body.len(), byte_in_line);
    assert!(trailer.contains(&format!("byteInLine={byte_in_line}")));
    // byteInLine 継続で残りを読む(marker も byte 予算に計上される
    // ため、十分な cap の tool を別建てる)。
    let mut opts2 = read_opts(&root, None);
    opts2.max_bytes = Some(4096);
    let tool2 = create_read_tool(opts2);
    let result2 = tool2
        .execute(
            &json!({"path": "f.txt", "offset": 1, "byteInLine": byte_in_line}),
            None,
        )
        .unwrap();
    let d2 = result2.details.as_ref().unwrap();
    assert_eq!(d2["eof"], true);
    assert_eq!(d2["totalLines"], 1);
    let text2 = text_of(&result2);
    let marker = format!("[partial line 1: continuing from byte {byte_in_line}]");
    let expected_rest = "x".repeat(100 - byte_in_line);
    assert_eq!(text2, format!("{marker}\n{expected_rest}"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_rejects_directory_and_image() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("img.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();
    let tool = create_read_tool(read_opts(&root, None));
    let err = tool.execute(&json!({"path": "sub"}), None).unwrap_err();
    assert_eq!(err.to_string(), "read: not a regular file");
    let err = tool.execute(&json!({"path": "img.png"}), None).unwrap_err();
    assert_eq!(err.to_string(), "read does not support image files");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_rejects_binary_nul_sniff() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join("b.bin"), [0x61u8, 0x00, 0x62]).unwrap();
    let tool = create_read_tool(read_opts(&root, None));
    let err = tool.execute(&json!({"path": "b.bin"}), None).unwrap_err();
    assert_eq!(
        err.to_string(),
        "read: binary file (NUL byte within first 8KiB)"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_invalid_utf8_becomes_replacement_char() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join("bad.txt"), [0x66, 0x80, 0x67]).unwrap();
    let tool = create_read_tool(read_opts(&root, None));
    let result = tool.execute(&json!({"path": "bad.txt"}), None).unwrap();
    assert_eq!(text_of(&result), "f\u{FFFD}g");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_denied_and_escape_errors() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join(".env"), "SECRET=1").unwrap();
    let denylist = create_path_denylist(&[]);
    let tool = create_read_tool(read_opts(&root, Some(&denylist)));
    let err = tool.execute(&json!({"path": ".env"}), None).unwrap_err();
    assert_eq!(err.to_string(), crate::walker::POLICY_DENIED_MESSAGE);
    let tool2 = create_read_tool(read_opts(&root, None));
    let err = tool2.execute(&json!({"path": "../x"}), None).unwrap_err();
    assert_eq!(err.to_string(), crate::walker::ACCESS_DENIED_MESSAGE);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_aborted_signal() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join("f.txt"), "a").unwrap();
    let controller = crate::operation::AbortController::new();
    controller.abort(crate::operation::AbortReason::msg("stop"));
    let signal = controller.signal();
    let tool = create_read_tool(read_opts(&root, None));
    let err = tool
        .execute(&json!({"path": "f.txt"}), Some(&signal))
        .unwrap_err();
    assert_eq!(err.to_string(), "Operation aborted");
    fs::remove_dir_all(&dir).unwrap();
}

/// 最小 publisher: bytes を Vec に保持する fake。
struct FakePublisher {
    cap: u64,
}

impl crate::artifact::ArtifactPublisher for FakePublisher {
    fn max_artifact_bytes(&self) -> u64 {
        self.cap
    }
    fn put(&self, value: ArtifactPut) -> Result<ArtifactReference, ToolError> {
        let bytes = match &value.data {
            ArtifactData::Bytes(b) => b.len() as u64,
            ArtifactData::Text(t) => t.len() as u64,
        };
        Ok(ArtifactReference {
            artifact: "test-artifact".to_string(),
            run_id: "run-1".to_string(),
            producer: value.producer,
            bytes,
            preview: String::new(),
            truncated: false,
        })
    }
}

#[test]
fn fs_read_artifact_publication() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join("big.txt"), "data").unwrap();
    let publisher = Arc::new(FakePublisher { cap: 1024 });
    let ports = ArtifactPorts {
        reader: None,
        publisher: Some(publisher),
    };
    let mut opts = read_opts(&root, None);
    opts.artifacts = Some(&ports);
    let tool = create_read_tool(opts);
    let result = tool
        .execute(&json!({"path": "big.txt", "artifact": "blob"}), None)
        .unwrap();
    assert!(result.content.is_empty());
    let artifact = &result.details.as_ref().unwrap()["artifact"];
    assert_eq!(artifact["artifact"], "test-artifact");
    assert_eq!(artifact["producer"]["name"], "blob");
    assert_eq!(artifact["producer"]["jobId"], "tool-runtime");
    assert_eq!(artifact["bytes"], 4);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn fs_read_artifact_publication_cap() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    fs::write(dir.join("big.txt"), "0123456789").unwrap();
    let publisher = Arc::new(FakePublisher { cap: 4 });
    let ports = ArtifactPorts {
        reader: None,
        publisher: Some(publisher),
    };
    let mut opts = read_opts(&root, None);
    opts.artifacts = Some(&ports);
    let tool = create_read_tool(opts);
    let err = tool
        .execute(&json!({"path": "big.txt", "artifact": "blob"}), None)
        .unwrap_err();
    assert_eq!(err.to_string(), "artifact storage limit exceeded");
    fs::remove_dir_all(&dir).unwrap();
}

/// 最小 reader。
struct FakeReader {
    data: Vec<u8>,
}

impl ArtifactReader for FakeReader {
    fn read(
        &self,
        _source: &ArtifactSource,
        request: &ArtifactReadRequest,
    ) -> Result<ArtifactReadResult, ToolError> {
        let offset = request.offset.min(self.data.len() as u64) as usize;
        let end = (offset + request.max_bytes as usize).min(self.data.len());
        Ok(ArtifactReadResult {
            data: self.data[offset..end].to_vec(),
            offset: offset as u64,
            next_offset: end as u64,
            eof: end >= self.data.len(),
        })
    }
}

#[test]
fn fs_read_artifact_bytes() {
    let dir = tmpdir();
    let root = canonicalize_workspace_root(dir.to_str().unwrap()).unwrap();
    let reader = Arc::new(FakeReader {
        data: b"hello world".to_vec(),
    });
    let ports = ArtifactPorts {
        reader: Some(reader),
        publisher: None,
    };
    let mut opts = read_opts(&root, None);
    opts.artifacts = Some(&ports);
    let tool = create_read_tool(opts);
    let source = json!({
        "kind": "artifact",
        "reference": {
            "artifact": "a",
            "runId": "r",
            "producer": {"jobId": "j", "name": "n"},
            "bytes": 11,
            "preview": "",
            "truncated": false,
        },
    });
    let result = tool
        .execute(&json!({"source": source, "offset": 6, "maxBytes": 5}), None)
        .unwrap();
    assert_eq!(text_of(&result), "world");
    let d = result.details.as_ref().unwrap();
    assert_eq!(d["offset"], 6);
    assert_eq!(d["nextOffset"], 11);
    assert_eq!(d["eof"], true);
    fs::remove_dir_all(&dir).unwrap();
}
