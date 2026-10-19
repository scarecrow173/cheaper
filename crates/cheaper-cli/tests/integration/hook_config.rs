//! `cheaper hook-config` の contract test(issue #107 /
//! ADR-034H2WBNtC4We8dyC5lQZJ)。
//!
//! sync/verify/remove・legacy conflict reporting・`--event-stdin`
//! の lifecycle mapping を in-process `run_cli` で検証する。
//! capability check
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

/// `.windsurf/hooks.json` に legacy cheaper 登録を残す conflict fixture。
fn write_legacy_conflict(fx: &Fixture) {
    std::fs::create_dir_all(fx.ws.path().join(".windsurf")).unwrap();
    std::fs::write(
        fx.ws.path().join(".windsurf").join("hooks.json"),
        "{\"pre_read_code\": [{\"command\": \"cheaper hook pre-read-code\"}]}\n",
    )
    .unwrap();
}

/// `.devin/config.json` を PreToolUse group 群で書き込む。
fn write_config_with_groups(fx: &Fixture, groups: &[String]) {
    std::fs::create_dir_all(fx.ws.path().join(".devin")).unwrap();
    std::fs::write(
        config_path(fx),
        format!("{{\"hooks\":{{\"PreToolUse\":[{}]}}}}\n", groups.join(",")),
    )
    .unwrap();
}

/// `.devin/` 配下に任意名の sibling file(orphan 候補)を書き込む。
/// 戻り値は作成した file の path。
fn write_orphan_file(fx: &Fixture, name: &str) -> PathBuf {
    std::fs::create_dir_all(fx.ws.path().join(".devin")).unwrap();
    let path = fx.ws.path().join(".devin").join(name);
    std::fs::write(&path, "orphan fixture\n").unwrap();
    path
}

/// own entry(canonical matcher + `OWN_COMMAND`)を持つ group が
/// config の `hooks.PreToolUse` に存在することを assert する。
fn assert_own_entry_present(cfg: &Value) {
    let pre = cfg["hooks"]["PreToolUse"].as_array().unwrap();
    assert!(
        pre.iter().any(|g| {
            g["matcher"] == CANONICAL_MATCHER
                && g["hooks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|h| h["command"] == OWN_COMMAND)
        }),
        "{cfg}"
    );
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
async fn hook_config_verify_absent_config_reports_legacy_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    // config 不在 + legacy 登録残り — blocker の `absent-config` と
    // informational の `legacy-conflict:` は意図的に併存する。
    write_legacy_conflict(&fx);

    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2, "{env}");
    assert_eq!(env["result"]["clean"], false);
    assert_eq!(env["result"]["ownEntry"], "absent");
    let findings = env["result"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().contains("absent-config")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("legacy-conflict:")),
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
    // conflict 無しでは `legacyConflicts` は空 array として常時存在し、
    // `warning` は emit されない(条件付き出力を pin)。
    assert!(
        env["result"]["legacyConflicts"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{env}"
    );
    assert!(env["result"].get("warning").is_none(), "{env}");
    let cfg = read_config(&fx);
    // 無関係 key/entry は保持。
    assert_eq!(cfg["defaults"]["model"], "x");
    let pre = cfg["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 2);
    assert_eq!(pre[1]["matcher"], CANONICAL_MATCHER);
    assert_eq!(pre[1]["hooks"][0]["command"], OWN_COMMAND);

    // 冪等: 2回目は変更無し。`changed:false` 経路でも
    // `legacyConflicts`(空)は emit され `warning` は付かない。
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["changed"], false);
    assert!(
        env["result"]["legacyConflicts"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{env}"
    );
    assert!(env["result"].get("warning").is_none(), "{env}");

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
async fn hook_config_sync_warns_on_legacy_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    // legacy windsurf 登録を残す。
    write_legacy_conflict(&fx);

    // 降格後の規約: conflict は informational であり sync は upsert を
    // 継続し、`legacyConflicts` + `warning` を載せて exit 0。
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["ok"], true);
    let conflicts = env["result"]["legacyConflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1, "{env}");
    assert!(
        conflicts[0].as_str().unwrap().contains(".windsurf"),
        "{conflicts:?}"
    );
    assert!(
        !env["result"]["warning"]
            .as_str()
            .unwrap_or_default()
            .is_empty(),
        "{env}"
    );
    assert!(config_path(&fx).exists(), "sync must write on conflict");
    assert_own_entry_present(&read_config(&fx));

    // verify は conflict を informational finding として報告し clean。
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["clean"], true);
    let conflicts = env["result"]["legacyConflicts"].as_array().unwrap();
    assert_eq!(conflicts.len(), 1, "{env}");
    let findings = env["result"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("legacy-conflict:")),
        "{findings:?}"
    );
    for f in findings {
        assert!(
            !f.as_str().unwrap().contains("duplicate firing is unsafe"),
            "{f}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_target_file_conflict_warns_and_syncs() {
    let mut fx = fixture_with_cheaper_on_path();
    // target file 自身に foreign cheaper 登録(own command 以外)が残る —
    // `file == target` && `!= OWN_COMMAND` の positive branch を通す。
    // foreign entry は own でないため sync は upsert を継続する。
    write_config_with_groups(
        &fx,
        &["{\"matcher\":\"^Read$\",\"hooks\":[{\"type\":\"command\",\"command\":\"cheaper hook pre-read-code\"}]}".to_string()],
    );

    // verify は target file 自体を `legacy-conflict:` として報告する
    // (own entry 欠落の `missing:` blocker と併存)。
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2, "{env}");
    let conflicts = env["result"]["legacyConflicts"].as_array().unwrap();
    assert!(
        conflicts
            .iter()
            .any(|f| f.as_str().unwrap().contains("config.json")),
        "{conflicts:?}"
    );
    let findings = env["result"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("legacy-conflict:")),
        "{findings:?}"
    );

    // sync は foreign entry を保持しつつ own entry を upsert し、
    // conflict は warning として報告して exit 0。
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["changed"], true);
    assert!(env["result"]["warning"].is_string(), "{env}");
    let cfg = read_config(&fx);
    let pre = cfg["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 2, "{cfg}");
    assert_own_entry_present(&cfg);
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

#[tokio::test(flavor = "current_thread")]
async fn hook_config_event_stdin_converges_on_legacy_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    write_legacy_conflict(&fx);

    // post-install — conflict 有りでも sync + verify が収束して exit 0。
    fx.stdin.data = b"{\"event\":\"post-install\",\"packages\":[\"cheaper\"]}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(config_path(&fx).exists(), "post-install must sync");
    let sync_conflicts = env["result"]["sync"]["legacyConflicts"].as_array().unwrap();
    assert!(!sync_conflicts.is_empty(), "{env}");
    assert!(env["result"]["sync"]["warning"].is_string(), "{env}");
    assert_eq!(env["result"]["verify"]["clean"], true);

    // post-update — 同一 mapping の冪等経路でも conflict で落ちない。
    fx.stdin.data = b"{\"event\":\"post-update\",\"packages\":[\"cheaper\"]}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(config_path(&fx).exists(), "post-update must keep config");
    assert_own_entry_present(&read_config(&fx));
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_post_uninstall_undetermined_converges_on_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    // own entry 書込み済み + conflict-only residual。
    write_config_with_groups(&fx, &[own_entry(CANONICAL_MATCHER)]);
    write_legacy_conflict(&fx);

    // packages 判定不能 → remove せず verify passthrough。降格前は
    // conflict-only で exit 2 だったが、降格後は clean:true で exit 0。
    fx.stdin.data = b"{\"event\":\"post-uninstall\"}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["action"], "skipped");
    assert_eq!(env["result"]["verify"]["clean"], true);
    // conflict state が実際に確立されていたことを pin(informational
    // finding の前提を検証)。
    assert!(
        !env["result"]["verify"]["legacyConflicts"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{env}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_quiet_keeps_conflict_fields() {
    let mut fx = fixture_with_cheaper_on_path();
    write_legacy_conflict(&fx);
    let (code, env, _err) = fx.run(&["hook-config", "sync", "--quiet"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(
        !env["result"]["legacyConflicts"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{env}"
    );
    assert!(env["result"]["warning"].is_string(), "{env}");
    // `warning`/`legacyConflicts` は quiet gate 対象外 — `note` のみ抑制。
    assert!(env["result"].get("note").is_none(), "{env}");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_retained_errors_with_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    write_legacy_conflict(&fx);

    // malformed config — conflict 併存下でも error のまま。
    std::fs::create_dir_all(fx.ws.path().join(".devin")).unwrap();
    std::fs::write(config_path(&fx), "{ not json").unwrap();
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 2, "{env}");
    assert_eq!(env["ok"], false);

    // 未知 matcher の modified own entry。
    write_config_with_groups(&fx, &[own_entry("^Bogus$")]);
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 2, "{env}");
    assert_eq!(env["ok"], false);

    // duplicate own entry。
    write_config_with_groups(&fx, &[own_entry(CANONICAL_MATCHER), own_entry(OLD_MATCHER)]);
    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 2, "{env}");
    assert_eq!(env["ok"], false);
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_verify_blockers_retained_with_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    write_legacy_conflict(&fx);

    // own entry missing + conflict — informational finding は
    // blocker を覆わない。
    write_config_with_groups(&fx, &[]);
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2, "{env}");
    assert_eq!(env["result"]["clean"], false);
    let findings = env["result"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("missing:")),
        "{findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("legacy-conflict:")),
        "{findings:?}"
    );

    // modified own entry(未知 matcher)+ conflict。
    write_config_with_groups(&fx, &[own_entry("^Bogus$")]);
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2, "{env}");
    assert_eq!(env["result"]["clean"], false);
    let findings = env["result"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("modified:")),
        "{findings:?}"
    );

    // duplicate own entry + conflict。
    write_config_with_groups(
        &fx,
        &[own_entry(CANONICAL_MATCHER), own_entry(CANONICAL_MATCHER)],
    );
    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 2, "{env}");
    assert_eq!(env["result"]["clean"], false);
    let findings = env["result"]["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("duplicate:")),
        "{findings:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_remove_reports_residual_conflict() {
    let mut fx = fixture_with_cheaper_on_path();
    // own entry 書込み済み + legacy 登録残り。
    write_config_with_groups(&fx, &[own_entry(CANONICAL_MATCHER)]);
    write_legacy_conflict(&fx);

    let (code, env, _err) = fx.run(&["hook-config", "remove"]).await;
    assert_eq!(code, 0, "{env}");
    let residual = env["result"]["residualRegistrations"].as_array().unwrap();
    assert!(!residual.is_empty(), "{env}");
    let note = env["result"]["note"].as_str().unwrap();
    assert!(note.contains("deduplicated at eval time"), "{note}");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_sweeps_tmp_orphan() {
    let mut fx = fixture_with_cheaper_on_path();
    // atomic write 残留相当の `.tmp` orphan — sweep 対象。
    let tmp = write_orphan_file(&fx, "config.json.12345.tmp");

    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(!tmp.exists(), ".tmp orphan must be swept");
    let removed = env["result"]["orphansRemoved"].as_array().unwrap();
    assert!(
        removed
            .iter()
            .any(|v| v.as_str() == Some("config.json.12345.tmp")),
        "{env}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_preserves_bak_files() {
    let mut fx = fixture_with_cheaper_on_path();
    // `.bak` は cheaper が生成しない foreign file — sweep 対象外。
    let bak = write_orphan_file(&fx, "config.json.bak");
    let manual_bak = write_orphan_file(&fx, "config.json.manual.bak");

    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(bak.exists(), ".bak must survive sync");
    assert!(manual_bak.exists(), ".bak must survive sync");
    // `.bak` のみの環境では削除対象が無いため `orphansRemoved` key 自体
    // 不在でもよい(insert は削除 1 件以上が条件)。present なら `.bak`
    // 名を一切含まないことを pin する。
    if let Some(removed) = env["result"]["orphansRemoved"].as_array() {
        assert!(
            !removed
                .iter()
                .any(|v| v.as_str().unwrap().ends_with(".bak")),
            "{env}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_sync_sweeps_only_tmp_among_mixed() {
    let mut fx = fixture_with_cheaper_on_path();
    // `.tmp` + `.bak` 混在 — 削除されるのは `.tmp` のみ。
    let tmp = write_orphan_file(&fx, "config.json.777.tmp");
    let bak = write_orphan_file(&fx, "config.json.bak");

    let (code, env, _err) = fx.run(&["hook-config", "sync"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(!tmp.exists(), ".tmp orphan must be swept");
    assert!(bak.exists(), ".bak must survive sync");
    let removed = env["result"]["orphansRemoved"].as_array().unwrap();
    assert_eq!(removed.len(), 1, "{env}");
    assert_eq!(removed[0].as_str().unwrap(), "config.json.777.tmp");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_verify_reports_bak_orphan_informational() {
    let mut fx = fixture_with_cheaper_on_path();
    // converged config を先に用意 — config 不在だと `absent-config`
    // blocker で clean:false・exit 2 となり、`.bak` orphan が
    // informational であることの pin が立てられない。
    write_config_with_groups(&fx, &[own_entry(CANONICAL_MATCHER)]);
    let bak = write_orphan_file(&fx, "config.json.bak");

    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    // `.bak` orphan 単独では informational — clean:true・exit 0。
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["clean"], true, "{env}");
    let orphans = env["result"]["orphans"].as_array().unwrap();
    assert!(
        orphans
            .iter()
            .any(|v| v.as_str() == Some("config.json.bak")),
        "{env}"
    );
    let findings = env["result"]["findings"].as_array().unwrap();
    let bak_finding = findings
        .iter()
        .filter_map(|f| f.as_str())
        .find(|f| f.starts_with("orphan:") && f.contains("config.json.bak"))
        .unwrap_or_else(|| panic!("missing orphan finding for config.json.bak: {findings:?}"));
    assert!(
        bak_finding.contains("not managed by cheaper"),
        "{bak_finding}"
    );
    assert!(!bak_finding.contains("safe to delete"), "{bak_finding}");
    assert!(bak.exists(), "verify must not delete .bak");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_verify_tmp_orphan_wording_and_readonly() {
    let mut fx = fixture_with_cheaper_on_path();
    write_config_with_groups(&fx, &[own_entry(CANONICAL_MATCHER)]);
    let tmp = write_orphan_file(&fx, "config.json.999.tmp");

    let (code, env, _err) = fx.run(&["hook-config", "verify"]).await;
    assert_eq!(code, 0, "{env}");
    let findings = env["result"]["findings"].as_array().unwrap();
    let tmp_finding = findings
        .iter()
        .filter_map(|f| f.as_str())
        .find(|f| f.starts_with("orphan:") && f.contains("config.json.999.tmp"))
        .unwrap_or_else(|| panic!("missing orphan finding: {findings:?}"));
    // `.tmp` orphan は次回 sync が自動回収する旨の文言。
    assert!(tmp_finding.contains("auto-removed"), "{tmp_finding}");
    assert!(tmp_finding.contains("sync"), "{tmp_finding}");
    // verify は read-only — `.tmp` file は残存する。
    assert!(tmp.exists(), "verify must not delete .tmp orphan");
}

#[tokio::test(flavor = "current_thread")]
async fn hook_config_event_stdin_preserves_bak() {
    let mut fx = fixture_with_cheaper_on_path();
    let bak = write_orphan_file(&fx, "config.json.bak");

    // post-install → sync + verify — `.bak` は残存。
    fx.stdin.data = b"{\"event\":\"post-install\",\"packages\":[\"cheaper\"]}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(bak.exists(), ".bak must survive post-install sync");
    if let Some(removed) = env["result"]["sync"]["orphansRemoved"].as_array() {
        assert!(
            !removed
                .iter()
                .any(|v| v.as_str().unwrap().ends_with(".bak")),
            "{env}"
        );
    }

    // post-update — 同一 arm の contract 継承を pin。
    fx.stdin.data = b"{\"event\":\"post-update\",\"packages\":[\"cheaper\"]}".to_vec();
    let (code, env, _err) = fx.run(&["hook-config", "--event-stdin"]).await;
    assert_eq!(code, 0, "{env}");
    assert!(bak.exists(), ".bak must survive post-update sync");
    if let Some(removed) = env["result"]["sync"]["orphansRemoved"].as_array() {
        assert!(
            !removed
                .iter()
                .any(|v| v.as_str().unwrap().ends_with(".bak")),
            "{env}"
        );
    }
}
