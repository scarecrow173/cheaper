//! `cheaper hook-config` の contract test(issue #107 /
//! ADR-034H2WBNtC4We8dyC5lQZJ)。
//!
//! sync/verify/remove・legacy gate・`--event-stdin` の lifecycle
//! mapping を in-process `run_cli` で検証する。capability check
//! (PATH 上の cheaper)には `CARGO_BIN_EXE` の copy を PATH shim
//! として使う。

use std::path::PathBuf;

use crate::common::Fixture;
use serde_json::Value;

const OWN_COMMAND: &str = "cheaper hook pre-tool-use --target devin";
const CANONICAL_MATCHER: &str = "^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Ee][Xx][Ee][Cc])$";
const OLD_MATCHER: &str = "^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp])$";

/// `cheaper` binary を PATH shim dir へコピーし、`PATH` を
/// `<shim>` のみに固定した Fixture を返す。
fn fixture_with_cheaper_on_path() -> Fixture {
    let fx = Fixture::new();
    let shim = fx.ws.path().join("bin");
    std::fs::create_dir_all(&shim).unwrap();
    let src = PathBuf::from(env!("CARGO_BIN_EXE_cheaper"));
    let name = if cfg!(windows) {
        "cheaper.exe"
    } else {
        "cheaper"
    };
    let dst = shim.join(name);
    std::fs::copy(&src, &dst).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&dst).unwrap().permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(&dst, p).unwrap();
    }
    let mut fx = fx;
    fx.env_map
        .insert("PATH".to_string(), shim.to_string_lossy().into_owned());
    fx
}

fn config_path(fx: &Fixture) -> PathBuf {
    fx.ws.path().join(".devin").join("config.json")
}

fn read_config(fx: &Fixture) -> Value {
    serde_json::from_str(&std::fs::read_to_string(config_path(fx)).unwrap()).unwrap()
}

fn own_entry(matcher: &str) -> String {
    format!(
        "{{\"matcher\":{},\"hooks\":[{{\"type\":\"command\",\"command\":{}}}]}}",
        serde_json::to_string(matcher).unwrap(),
        serde_json::to_string(OWN_COMMAND).unwrap()
    )
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_verify_reports_absent_config() {
    let mut fx = fixture_with_cheaper_on_path();
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], true);
    assert_eq!(env["result"]["ownEntry"], "absent");
    assert_eq!(env["result"]["clean"], false);
    let findings = env["result"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().contains("absent-config")),
        "{findings:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_creates_and_is_idempotent() {
    let mut fx = fixture_with_cheaper_on_path();
    // 既存の無関係 config を残しておく(保持されること)。
    std::fs::create_dir_all(fx.ws.path().join(".devin")).unwrap();
    std::fs::write(
        config_path(&fx),
        "{\n  \"defaults\": {\"model\": \"x\"},\n  \"hooks\": {\"PreToolUse\": [{\"matcher\": \"^Write$\", \"hooks\": [{\"type\": \"command\", \"command\": \"other-tool\"}]}]}}\n",
    )
    .unwrap();

    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["changed"], true);
    let cfg = read_config(&fx);
    // 無関係 key/entry は保持。
    assert_eq!(cfg["defaults"]["model"], "x");
    let pre = cfg["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 2);
    assert_eq!(pre[1]["matcher"], CANONICAL_MATCHER);
    assert_eq!(pre[1]["hooks"][0]["command"], OWN_COMMAND);

    // 冪等: 2回目は変更無し。
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["changed"], false);

    // verify は clean。
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["clean"], true);
    assert_eq!(env["result"]["ownEntry"], "present");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_converges_old_matcher() {
    let mut fx = fixture_with_cheaper_on_path();
    std::fs::create_dir_all(fx.ws.path().join(".devin")).unwrap();
    std::fs::write(
        config_path(&fx),
        format!(
            "{{\"hooks\":{{\"PreToolUse\":[{}]}}}}\n",
            own_entry(OLD_MATCHER)
        ),
    )
    .unwrap();
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["changed"], true);
    let cfg = read_config(&fx);
    assert_eq!(cfg["hooks"]["PreToolUse"][0]["matcher"], CANONICAL_MATCHER);
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_fails_on_legacy_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    // legacy windsurf 登録を残す。
    std::fs::create_dir_all(fx.ws.path().join(".windsurf")).unwrap();
    std::fs::write(
        fx.ws.path().join(".windsurf").join("hooks.json"),
        "{\"pre_read_code\": [{\"command\": \"cheaper hook pre-read-code\"}]}\n",
    )
    .unwrap();
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], false);
    assert!(
        !config_path(&fx).exists(),
        "sync must not write on conflict"
    );

    // verify は conflict を報告する。
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2);
    let conflicts = env["result"]["legacyConflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1, "{env}");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_remove_only_own_entry() {
    let mut fx = fixture_with_cheaper_on_path();
    std::fs::create_dir_all(fx.ws.path().join(".devin")).unwrap();
    std::fs::write(
        config_path(&fx),
        format!(
            "{{\"hooks\":{{\"PreToolUse\":[{{\"matcher\":\"^Write$\",\"hooks\":[{{\"type\":\"command\",\"command\":\"other\"}}]}},{}]}}}}\n",
            own_entry(CANONICAL_MATCHER)
        ),
    )
    .unwrap();
    let (code, env, _err) = fx.run(&["hook-config", "remove"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["changed"], true);
    let cfg = read_config(&fx);
    let pre = cfg["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 1);
    assert_eq!(pre[0]["hooks"][0]["command"], "other");

    // 冪等: 2回目は no-op。
    let (code, env, _err) = fx.run(&["hook-config", "remove"]).await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["changed"], false);
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_event_stdin_lifecycle_mapping() {
    let mut fx = fixture_with_cheaper_on_path();
    // post-install → sync + verify。
    fx.stdin.data = b"{\"event\":\"post-install\",\"packages\":[\"cheaper\"]}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(config_path(&fx).exists(), "post-install must sync");
    assert!(env["result"]["verify"]["clean"].as_bool().unwrap());

    // 非 cheaper package の post-uninstall → noop(remove しない)。
    fx.stdin.data = b"{\"event\":\"post-uninstall\",\"packages\":[\"other-pkg\"]}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["action"], "noop");
    assert!(config_path(&fx).exists(), "must not remove");

    // cheaper の post-uninstall → remove。
    fx.stdin.data =
        b"{\"event\":\"post-uninstall\",\"packages\":[\"scarecrow173/cheaper\"]}".to_vec();
    let (code, _env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0);
    let cfg = read_config(&fx);
    let pre = cfg["hooks"]["PreToolUse"].as_array().unwrap();
    assert!(pre.is_empty(), "own entry must be removed: {cfg}");

    // pre-* → noop。
    fx.stdin.data = b"{\"event\":\"pre-install\"}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["action"], "noop");

    // malformed payload → error。
    fx.stdin.data = b"not json".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], false);
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_rejects_malformed_and_unknown_args() {
    let mut fx = fixture_with_cheaper_on_path();
    // 未知 action。
    let (code, env, _err) = fx.run(&["hook-config", "bogus"]).await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], false);
    // action 無し(event-stdin 無し)。
    let (code, env, _err) = fx.run(&["hook-config"]).await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], false);
    // 未知 scope。
    let (code, env, _err) = fx
        .run(&["hook-config", "verify", "--scope", "system"])
        .await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], false);
    // malformed JSON config は verify で finding、sync では error。
    std::fs::create_dir_all(fx.ws.path().join(".devin")).unwrap();
    std::fs::write(config_path(&fx), "{ not json").unwrap();
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2);
    assert!(
        env["result"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f.as_str().unwrap().contains("config-unreadable")),
        "{env}"
    );
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], false);
}
