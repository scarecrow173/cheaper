//! `pricing.rs` — `load_pricing_snapshot` の fail-soft contract test
//! (`packages/cli/test/pricing.test.ts` の port)。

use std::path::Path;

use cheaper_cli::pricing::{load_pricing_snapshot, PRICING_FILENAME};

fn write_pricing(dir: &Path, body: &str) {
    std::fs::write(dir.join(PRICING_FILENAME), body).unwrap();
}

#[test]
fn returns_none_when_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load_pricing_snapshot(dir.path()).is_none());
}

#[test]
fn returns_none_for_malformed_json_or_missing_fields() {
    let dir = tempfile::tempdir().unwrap();
    write_pricing(dir.path(), "{ not json");
    assert!(load_pricing_snapshot(dir.path()).is_none());
    write_pricing(dir.path(), r#"{"version":"2026-09-18"}"#);
    assert!(load_pricing_snapshot(dir.path()).is_none());
    write_pricing(
        dir.path(),
        r#"{"version":"v","updatedAt":"2026-09-18","models":"nope"}"#,
    );
    assert!(load_pricing_snapshot(dir.path()).is_none());
    // 非 object(array / scalar)も None。
    write_pricing(dir.path(), "[1,2]");
    assert!(load_pricing_snapshot(dir.path()).is_none());
    write_pricing(dir.path(), "null");
    assert!(load_pricing_snapshot(dir.path()).is_none());
}

#[test]
fn parses_snapshot_matching_core_contract() {
    let dir = tempfile::tempdir().unwrap();
    write_pricing(
        dir.path(),
        r#"{"version":"2026-09-18","updatedAt":"2026-09-18T00:00:00.000Z",
           "models":{"ollama/qwen3.5:9b":{"input":0.1,"output":0.3,
           "cacheRead":0.05,"cacheWrite":0.2,"updatedAt":"2026-09-18"}}}"#,
    );
    let snap = load_pricing_snapshot(dir.path()).unwrap();
    assert_eq!(snap.version, "2026-09-18");
    assert_eq!(snap.updated_at, "2026-09-18T00:00:00.000Z");
    assert_eq!(snap.models.len(), 1);
    let (key, e) = &snap.models[0];
    assert_eq!(key, "ollama/qwen3.5:9b");
    assert_eq!(e.input, 0.1);
    assert_eq!(e.output, 0.3);
    assert_eq!(e.cache_read, Some(0.05));
    assert_eq!(e.cache_write, Some(0.2));
    assert_eq!(e.updated_at.as_deref(), Some("2026-09-18"));
}

#[test]
fn skips_invalid_entries_but_keeps_valid() {
    let dir = tempfile::tempdir().unwrap();
    write_pricing(
        dir.path(),
        r#"{"version":"v1","updatedAt":"2026-09-18","models":{
           "a/b":{"input":1,"output":2},
           "c/d":{"input":-1,"output":2},
           "e/f":{"input":1},
           "g/h":"garbage"}}"#,
    );
    let snap = load_pricing_snapshot(dir.path()).unwrap();
    assert_eq!(snap.models.len(), 1);
    assert_eq!(snap.models[0].0, "a/b");
    assert_eq!(snap.models[0].1.input, 1.0);
    assert_eq!(snap.models[0].1.output, 2.0);
    assert_eq!(snap.models[0].1.cache_read, None);
}

#[test]
fn preserves_model_key_insertion_order() {
    // `resolve_pricing` の first-match 走査は Object.entries 順に依存。
    let dir = tempfile::tempdir().unwrap();
    write_pricing(
        dir.path(),
        r#"{"version":"v","updatedAt":"u","models":{
           "z/z":{"input":1,"output":1},
           "a/a":{"input":2,"output":2},
           "m/m":{"input":3,"output":3}}}"#,
    );
    let snap = load_pricing_snapshot(dir.path()).unwrap();
    let keys: Vec<&str> = snap.models.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["z/z", "a/a", "m/m"]);
}
