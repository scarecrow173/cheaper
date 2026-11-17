//! workflow catalog manifest の read/validate 契約
//! (docs/test-specs/workflow-catalog-manifest-and-list-contract.md)。

use cheaper_core::errors::{CheaperError, ErrorCode};
use cheaper_core::nodepath;
use cheaper_core::policy::schema::FilesystemPolicy;
use cheaper_core::workspace::paths::{realpath, SymlinkPolicy};
use serde_json::{json, Value};

use cheaper_workflow::{read_catalog, CatalogRead, DiagnosticKind, EntryStatus};

struct Workspace {
    _dir: tempfile::TempDir,
    root: String,
}

fn workspace() -> Workspace {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = realpath(&dir.path().to_string_lossy()).expect("realpath");
    Workspace { _dir: dir, root }
}

fn policy(symlinks: SymlinkPolicy, deny: &[&str]) -> FilesystemPolicy {
    FilesystemPolicy {
        symlinks,
        deny_paths: deny.iter().map(|p| (*p).to_string()).collect(),
    }
}

fn within() -> FilesystemPolicy {
    policy(SymlinkPolicy::WithinWorkspace, &[])
}

fn write_raw(root: &str, text: &str) {
    let dir = nodepath::join(&[root, ".cheaper", "workflows"]);
    std::fs::create_dir_all(&dir).expect("mkdir catalog dir");
    std::fs::write(nodepath::join(&[&dir, "catalog.json"]), text).expect("write catalog");
}

fn write_catalog(root: &str, workflows: Value) {
    let manifest = json!({ "schemaVersion": 1, "workflows": workflows });
    write_raw(root, &serde_json::to_string(&manifest).unwrap());
}

fn touch(root: &str, rel: &str) {
    let path = nodepath::join(&[root, rel]);
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).expect("mkdir parent");
    }
    std::fs::write(&path, "PRIVATE workflow body").expect("write workflow");
}

fn entry(id: &str, path: &str) -> Value {
    json!({ "id": id, "path": path, "summary": format!("{id} summary") })
}

fn read_ok(root: &str, filesystem: &FilesystemPolicy) -> CatalogRead {
    read_catalog(root, filesystem).expect("catalog read")
}

fn read_err(root: &str) -> CheaperError {
    read_catalog(root, &within()).expect_err("manifest must fail closed")
}

fn statuses(read: &CatalogRead) -> Vec<EntryStatus> {
    read.workflows.iter().map(|w| w.status).collect()
}

fn kinds(read: &CatalogRead) -> Vec<(DiagnosticKind, usize)> {
    read.diagnostics.iter().map(|d| (d.kind, d.index)).collect()
}

/// `symlink(target, link, "junction"|"dir")` 相当(loader test と同形)。
fn symlink_dir(target: &str, link: &str) {
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J", link, target])
            .output()
            .expect("cmd mklink");
        assert!(status.status.success(), "mklink /J failed");
    }
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(target, link).expect("symlink");
}

#[test]
fn absent_manifest_reads_as_empty_catalog() {
    let ws = workspace();
    let read = read_ok(&ws.root, &within());
    assert!(!read.present);
    assert!(read.workflows.is_empty());
    assert!(read.diagnostics.is_empty());
}

#[test]
fn present_empty_manifest_is_distinct_from_absent() {
    let ws = workspace();
    write_catalog(&ws.root, json!([]));
    let read = read_ok(&ws.root, &within());
    assert!(read.present);
    assert!(read.workflows.is_empty());
}

#[test]
fn manifest_corruption_fails_closed_with_invalid_input() {
    let ok = |id: &str| entry(id, "wf.json");
    let long = |n: usize| "a".repeat(n);
    let cases: Vec<(&str, String)> = vec![
        ("malformed json", "{ not json PRIVATE".to_string()),
        ("root array", "[]".to_string()),
        ("schemaVersion 2", json!({"schemaVersion": 2, "workflows": []}).to_string()),
        ("schemaVersion missing", json!({"workflows": []}).to_string()),
        ("workflows object", json!({"schemaVersion": 1, "workflows": {}}).to_string()),
        ("workflows missing", json!({"schemaVersion": 1}).to_string()),
        (
            "257 entries",
            json!({"schemaVersion": 1, "workflows": (0..257).map(|i| ok(&format!("w{i}"))).collect::<Vec<_>>()})
                .to_string(),
        ),
        (
            "id 65 chars",
            json!({"schemaVersion": 1, "workflows": [ok(&long(65))]}).to_string(),
        ),
        (
            "path 513 chars",
            json!({"schemaVersion": 1, "workflows": [entry("a", &long(513))]}).to_string(),
        ),
        (
            "summary 501 chars",
            json!({"schemaVersion": 1, "workflows": [{"id": "a", "path": "wf.json", "summary": long(501)}]})
                .to_string(),
        ),
        (
            "17 tags",
            json!({"schemaVersion": 1, "workflows": [{"id": "a", "path": "wf.json", "summary": "", "tags": vec!["t"; 17]}]})
                .to_string(),
        ),
        (
            "tag 65 chars",
            json!({"schemaVersion": 1, "workflows": [{"id": "a", "path": "wf.json", "summary": "", "tags": [long(65)]}]})
                .to_string(),
        ),
        (
            "bound exceeded on a later malformed entry",
            json!({"schemaVersion": 1, "workflows": [ok("a"), {"id": long(65)}]}).to_string(),
        ),
    ];
    for (label, text) in cases {
        let ws = workspace();
        touch(&ws.root, "wf.json");
        write_raw(&ws.root, &text);
        let err = read_err(&ws.root);
        assert_eq!(err.code(), ErrorCode::InvalidInput, "{label}");
        assert_ne!(err.exit_code(), 0, "{label}");
        assert!(
            !err.message().contains("PRIVATE"),
            "{label}: {}",
            err.message()
        );
    }
}

#[test]
fn manifest_byte_bound_is_64_kib() {
    let ws = workspace();
    let base = json!({"schemaVersion": 1, "workflows": []}).to_string();
    let padded = |len: usize| format!("{base}{}", " ".repeat(len - base.len()));
    write_raw(&ws.root, &padded(64 * 1024));
    assert!(read_ok(&ws.root, &within()).present);
    write_raw(&ws.root, &padded(64 * 1024 + 1));
    assert_eq!(read_err(&ws.root).code(), ErrorCode::InvalidInput);
}

#[test]
fn field_bounds_accept_their_maximum() {
    let ws = workspace();
    touch(&ws.root, "wf.json");
    let tags: Vec<String> = (0..16).map(|i| format!("{i:0>64}")).collect();
    let mut workflows = vec![
        json!({
            "id": "i".repeat(64),
            "path": "wf.json",
            "summary": "s".repeat(500),
            "tags": tags,
        }),
        // 512 文字 path は manifest として受理される(file の有無は別判定)。
        entry("long-path", &"p".repeat(512)),
    ];
    workflows.extend((2..256).map(|i| entry(&format!("w{i}"), "wf.json")));
    write_catalog(&ws.root, Value::Array(workflows));
    let read = read_ok(&ws.root, &within());
    assert_eq!(read.workflows.len(), 256);
    assert_eq!(read.workflows[0].status, EntryStatus::Ok);
    assert_eq!(read.workflows[0].tags.len(), 16);
    assert_eq!(read.workflows[1].path.as_deref().map(str::len), Some(512));
}

#[test]
fn valid_entry_projects_manifest_fields_only() {
    let ws = workspace();
    // 本文は JSON ですらない — catalog read は workflow file を読まない。
    touch(&ws.root, "flows/review.json");
    write_catalog(
        &ws.root,
        json!([
            {
                "id": "review.v1",
                "path": "flows/review.json",
                "summary": "Review a change",
                "tags": ["review", "git"],
                "jobs": {"private": {"prompt": "PRIVATE"}}
            },
            entry("plain", "flows/review.json")
        ]),
    );
    let read = read_ok(&ws.root, &within());
    assert!(read.present);
    assert!(read.diagnostics.is_empty());
    let projected = serde_json::to_value(&read.workflows).unwrap();
    assert_eq!(
        projected,
        json!([
            {
                "id": "review.v1",
                "path": "flows/review.json",
                "summary": "Review a change",
                "tags": ["review", "git"],
                "status": "ok"
            },
            {
                "id": "plain",
                "path": "flows/review.json",
                "summary": "plain summary",
                "tags": [],
                "status": "ok"
            }
        ])
    );
}

#[test]
fn missing_file_is_reported_as_missing_path() {
    let ws = workspace();
    write_catalog(&ws.root, json!([entry("gone", "deleted.json")]));
    let read = read_ok(&ws.root, &within());
    assert_eq!(statuses(&read), [EntryStatus::Missing]);
    let diagnostic = &read.diagnostics[0];
    assert_eq!(diagnostic.kind, DiagnosticKind::MissingPath);
    assert_eq!(diagnostic.index, 0);
    assert_eq!(diagnostic.id.as_deref(), Some("gone"));
    assert_eq!(diagnostic.path.as_deref(), Some("deleted.json"));
}

#[test]
fn escape_deny_and_non_file_paths_are_invalid() {
    let ws = workspace();
    touch(&ws.root, ".env");
    touch(&ws.root, "blocked/wf.json");
    std::fs::create_dir_all(nodepath::join(&[&ws.root, "flows"])).unwrap();
    write_catalog(
        &ws.root,
        json!([
            entry("escape", "../outside.json"),
            entry("builtin-deny", ".env"),
            entry("policy-deny", "blocked/wf.json"),
            entry("dir", "flows"),
        ]),
    );
    let read = read_ok(
        &ws.root,
        &policy(SymlinkPolicy::WithinWorkspace, &["blocked"]),
    );
    assert_eq!(statuses(&read), [EntryStatus::Invalid; 4]);
    assert_eq!(
        kinds(&read),
        [
            (DiagnosticKind::PathOutOfWorkspace, 0),
            (DiagnosticKind::DeniedPath, 1),
            (DiagnosticKind::DeniedPath, 2),
            (DiagnosticKind::NotRegularFile, 3),
        ]
    );
}

#[test]
fn symlink_policy_and_canonical_deny_apply_to_entry_paths() {
    let ws = workspace();
    touch(&ws.root, "target/wf.json");
    symlink_dir(
        &nodepath::join(&[&ws.root, "target"]),
        &nodepath::join(&[&ws.root, "alias"]),
    );
    write_catalog(&ws.root, json!([entry("aliased", "alias/wf.json")]));

    let read = read_ok(&ws.root, &within());
    assert_eq!(statuses(&read), [EntryStatus::Ok]);

    let read = read_ok(&ws.root, &policy(SymlinkPolicy::Deny, &[]));
    assert_eq!(statuses(&read), [EntryStatus::Invalid]);
    assert_eq!(kinds(&read), [(DiagnosticKind::PathOutOfWorkspace, 0)]);

    for denied in ["alias", "target"] {
        let read = read_ok(&ws.root, &policy(SymlinkPolicy::WithinWorkspace, &[denied]));
        assert_eq!(kinds(&read), [(DiagnosticKind::DeniedPath, 0)], "{denied}");
    }
}

#[test]
fn duplicate_ids_mark_every_entry_without_last_wins() {
    let ws = workspace();
    touch(&ws.root, "a.json");
    touch(&ws.root, "b.json");
    write_catalog(
        &ws.root,
        json!([
            entry("same", "a.json"),
            entry("other", "a.json"),
            entry("same", "b.json")
        ]),
    );
    let read = read_ok(&ws.root, &within());
    assert_eq!(
        statuses(&read),
        [
            EntryStatus::Duplicate,
            EntryStatus::Ok,
            EntryStatus::Duplicate
        ]
    );
    assert_eq!(
        kinds(&read),
        [
            (DiagnosticKind::DuplicateId, 0),
            (DiagnosticKind::DuplicateId, 2)
        ]
    );
    assert_eq!(read.workflows[2].path.as_deref(), Some("b.json"));
}

#[test]
fn malformed_entries_are_invalid_and_keep_their_slot() {
    let ws = workspace();
    touch(&ws.root, "wf.json");
    write_catalog(
        &ws.root,
        json!([
            "not an object",
            {"id": "", "path": "wf.json", "summary": ""},
            {"id": "-dash-first", "path": "wf.json", "summary": ""},
            {"id": "has space", "path": "wf.json", "summary": ""},
            {"id": "no-path", "summary": ""},
            {"id": "empty-path", "path": "", "summary": ""},
            {"id": "no-summary", "path": "wf.json"},
            {"id": "bad-tags", "path": "wf.json", "summary": "", "tags": [1]},
            {"id": "null-tags", "path": "wf.json", "summary": "", "tags": null},
            entry("fine", "wf.json"),
        ]),
    );
    let read = read_ok(&ws.root, &within());
    let mut expected = vec![EntryStatus::Invalid; 9];
    expected.push(EntryStatus::Ok);
    assert_eq!(statuses(&read), expected);
    assert_eq!(
        kinds(&read),
        (0..9)
            .map(|i| (DiagnosticKind::MalformedEntry, i))
            .collect::<Vec<_>>()
    );
    // 非 string field は射影しない。
    assert_eq!(read.workflows[0].id, None);
    assert_eq!(read.workflows[4].path, None);
    assert!(read.workflows[7].tags.is_empty());
}

#[test]
fn absolute_entry_path_is_malformed() {
    let ws = workspace();
    touch(&ws.root, "wf.json");
    let absolute = nodepath::join(&[&ws.root, "wf.json"]);
    write_catalog(&ws.root, json!([entry("abs", &absolute)]));
    let read = read_ok(&ws.root, &within());
    assert_eq!(kinds(&read), [(DiagnosticKind::MalformedEntry, 0)]);
}

#[test]
fn duplicate_check_precedes_path_checks() {
    let ws = workspace();
    // duplicate は path 検査より先に判定される(missing path でも duplicate)。
    write_catalog(
        &ws.root,
        json!([entry("dup", "gone.json"), entry("dup", "../escape.json")]),
    );
    let read = read_ok(&ws.root, &within());
    assert_eq!(
        kinds(&read),
        [
            (DiagnosticKind::DuplicateId, 0),
            (DiagnosticKind::DuplicateId, 1)
        ]
    );
}
