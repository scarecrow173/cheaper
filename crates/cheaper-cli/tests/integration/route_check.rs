//! `route check` の read-only source supply に関する subprocess
//! contract test。report 本体は後続 task が実装する。

use std::process::Command;

#[test]
fn route_check_production_builtin_source_does_not_materialize_temp_cache() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_cheaper"))
        .args(["route", "check"])
        .current_dir(ws.path())
        .env("CHEAPER_CONFIG_DIR", config.path())
        .env("CHEAPER_LOG", "debug")
        .env("TEMP", temp.path())
        .env("TMP", temp.path())
        .env("TMPDIR", temp.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(70));
    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("sanitized error envelope");
    assert_eq!(stdout["ok"], false);
    assert_eq!(stdout["error"]["code"], "INTERNAL_ERROR");
    assert_eq!(stdout["error"]["details"]["reason"], "internal-error");

    let entries: Vec<_> = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(
        !entries
            .iter()
            .any(|name| name.to_string_lossy().starts_with("cheaper-builtin-")),
        "route check must not materialize builtin resources: {entries:?}"
    );
    assert!(
        std::fs::read_dir(config.path()).unwrap().next().is_none(),
        "route check must not write user config state"
    );
    assert!(!ws.path().join(".cheaper").exists());
}
