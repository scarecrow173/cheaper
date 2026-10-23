//! `cheaper policy grant {add,list,remove}` — user-scope
//! `<userConfigDir>/policy.yaml` の `command.grants` 管理 command の
//! dispatch 経路 test(design §(f)・test-spec の `policy grant` rules)。
//!
//! 本 target は dev-seam 非 gate(`required-features` 無し)の
//! utility-command test に限定する — fake-stream seam は dev-seam
//! feature 有効時のみ compile されるため model session を走らせられ
//! ず、`policy grant add/remove` 後の flag 無し run 挙動(AC6/AC7 の
//! 行為側半分)は `run_dev_seam.rs` 側が所有する。
//!
//! `common::Fixture::run` は stdout/stderr を使い回すため、同一 config
//! dir への複数 command 呼出し(round-trip)は本 file 内の
//! `run_policy`(call ごとに新規 sink)で行う。

use std::collections::HashMap;
use std::path::Path;

use crate::common;
use common::{ambient_env, PipeStdin, SharedVecSink};

use cheaper_cli::cli::{run_cli, CliHooks, CliIo};
use cheaper_core::errors::ErrorCode;
use serde_json::{json, Value};

/// `CHEAPER_CONFIG_DIR` を tempdir に向け、他は process env に
/// fallthrough。呼出しごとに新規 sink を張るため、返す stdout は当該
/// call の envelope のみ(`(exit code, stdout envelope, stderr)`)。
async fn run_policy(config_dir: &Path, argv: &[&str]) -> (i32, Value, String) {
    let injected: HashMap<String, String> = HashMap::from_iter([(
        "CHEAPER_CONFIG_DIR".to_string(),
        config_dir.to_string_lossy().into_owned(),
    )]);
    let env_lookup = |name: &str| injected.get(name).cloned().or_else(|| ambient_env(name));
    let mut stdin = PipeStdin::empty(true);
    let out = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let err = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut io = CliIo {
        stdin: &mut stdin,
        stdout: &mut SharedVecSink(out.clone()),
        stderr: &mut SharedVecSink(err.clone()),
        env: &env_lookup,
        cwd: std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    };
    let code = run_cli(
        &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        &mut io,
        &CliHooks::default(),
    )
    .await;
    drop(io);
    let stdout = String::from_utf8(out.borrow().clone()).unwrap();
    let stderr = String::from_utf8(err.borrow().clone()).unwrap();
    (
        code,
        serde_json::from_str(stdout.trim_end()).expect("stdout is a single JSON envelope"),
        stderr,
    )
}

/// built-in `resources/policy.yaml` の pin — embedded 実物を
/// `load_policy` で load し、issue #111 の既定値(maxTurns null /
/// timeout 300s の stall window)が resource 本体へ反映されていることを
/// 保証する。golden snapshot だけでは `resources/policy.yaml` 自体の
/// drift を検知できないため実 file を読む。
#[test]
fn builtin_policy_pins_nullable_max_turns_and_stall_window() {
    let builtin = common::builtin_dir().join("policy.yaml");
    let tmp = tempfile::tempdir().unwrap();
    let policy =
        cheaper_core::policy::load::load_policy(&cheaper_core::policy::load::LoadPolicyOptions {
            builtin_policy_path: &builtin.to_string_lossy(),
            user_config_dir: &tmp.path().to_string_lossy(),
            project_dir: &tmp.path().to_string_lossy(),
            invocation: None,
        })
        .unwrap();
    assert_eq!(policy.execution.max_turns, None);
    assert_eq!(policy.execution.timeout_ms, 300_000);
}

fn policy_path(config_dir: &Path) -> std::path::PathBuf {
    config_dir.join("policy.yaml")
}

fn policy_text(config_dir: &Path) -> String {
    std::fs::read_to_string(policy_path(config_dir)).unwrap()
}

/// file 上の grants を read path 経由で読む(`read_policy_yaml` —
/// write 内容の検証は serialize 文字列ではなく parse 後の値で行う)。
fn read_grants(config_dir: &Path) -> serde_json::Map<String, Value> {
    let policy = cheaper_core::policy::load::read_policy_yaml(
        &policy_path(config_dir).to_string_lossy(),
        false,
    )
    .unwrap()
    .expect("policy.yaml exists");
    policy
        .get("command")
        .and_then(Value::as_object)
        .and_then(|c| c.get("grants"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn assert_error(env: &Value, code: ErrorCode) {
    assert_eq!(env["ok"], json!(false), "{env}");
    assert_eq!(env["error"]["code"], json!(code.as_str()), "{env}");
    let exit = env["error"]["exitCode"].as_i64().unwrap();
    assert_eq!(exit, i64::from(code.exit_code()), "{env}");
}

/// add → list → remove の round-trip — file 永続化・name 順 sort・
/// `{grant:{name,executable}}` / `{grants:[...]}` envelope。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_add_then_list_then_remove_round_trip() {
    let cfg = tempfile::tempdir().unwrap();

    // file absent の add が新規 file を作る(version は command が
    // authoring する — `{"version":1}` base 経路)。
    let (code, env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"git","argv":["log","*"]}"#,
        ],
    )
    .await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["result"]["grant"]["name"], json!("git-log"));
    assert_eq!(env["result"]["grant"]["executable"], json!("git"));
    let grants = read_grants(cfg.path());
    assert_eq!(grants["git-log"]["executable"], json!("git"));
    assert_eq!(grants["git-log"]["argv"], json!(["log", "*"]));

    // 2 件目 — list は name 順 sort。
    let (code, _env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "cargo-fmt",
            r#"{"executable":"cargo","argv":["fmt"]}"#,
        ],
    )
    .await;
    assert_eq!(code, 0);
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "list"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(
        env["result"]["grants"],
        json!([
            {"name": "cargo-fmt", "executable": "cargo"},
            {"name": "git-log", "executable": "git"},
        ])
    );

    // remove → list が残り 1 件。削除側も merged full-object
    // validation + atomic write を通る。
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "remove", "git-log"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["grant"]["name"], json!("git-log"));
    assert_eq!(env["result"]["grant"]["executable"], json!("git"));
    let grants = read_grants(cfg.path());
    assert!(!grants.contains_key("git-log"));
    assert!(grants.contains_key("cargo-fmt"));
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "list"]).await;
    assert_eq!(code, 0);
    assert_eq!(
        env["result"]["grants"],
        json!([{"name": "cargo-fmt", "executable": "cargo"}])
    );
}

/// file absent の list は空配列。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_list_on_absent_file_is_empty() {
    let cfg = tempfile::tempdir().unwrap();
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "list"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["result"]["grants"], json!([]));
}

/// invalid spec(duration 規則違反・argv compile 失敗・schema 違反)は
/// `InvalidRuntimePolicy`(exit 3)で file を一切変更しない。
/// malformed JSON・`name` key 混入・非 object spec は `InvalidRequest`
/// (exit 2)。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_add_rejects_invalid_spec_without_writing() {
    let cfg = tempfile::tempdir().unwrap();
    std::fs::write(policy_path(cfg.path()), "version: 1\n").unwrap();
    let before = policy_text(cfg.path());

    // shared validation 由来 — `resolve_command_policy` 系統の失敗は
    // `InvalidRuntimePolicy`。
    for spec in [
        // bounded duration 規則違反(非 duration literal)。
        r#"{"executable":"git","timeout":"5x"}"#,
        // argv pattern compile 失敗(interior wildcard)。
        r#"{"executable":"git","argv":["a*b"]}"#,
        // schema 違反(`executable` required 欠落)。
        r#"{"argv":["log"]}"#,
    ] {
        let (code, env, _err) =
            run_policy(cfg.path(), &["policy", "grant", "add", "bad-grant", spec]).await;
        assert_eq!(code, 3, "{spec}: {env}");
        assert_error(&env, ErrorCode::InvalidRuntimePolicy);
        assert_eq!(policy_text(cfg.path()), before, "{spec}");
    }

    // 入力構造の失敗 — `InvalidRequest`。
    for spec in [
        "{not json",
        "[1,2]",
        "\"str\"",
        r#"{"name":"x","executable":"git"}"#,
    ] {
        let (code, env, _err) =
            run_policy(cfg.path(), &["policy", "grant", "add", "bad-grant", spec]).await;
        assert_eq!(code, 2, "{spec}: {env}");
        assert_error(&env, ErrorCode::InvalidRequest);
        assert_eq!(policy_text(cfg.path()), before, "{spec}");
    }
}

/// issue #165 — `fixed:true`・明示 `scope` を含む spec は CLI 入口で
/// `InvalidRequest`(exit 2、file 不変)。trusted YAML への直接記述は
/// 合法のままだが、`policy grant add` は `!fixed` command grant のみ
/// 登録する契約(design §(c))。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_add_rejects_fixed_and_scope() {
    let cfg = tempfile::tempdir().unwrap();
    std::fs::write(policy_path(cfg.path()), "version: 1\n").unwrap();
    let before = policy_text(cfg.path());

    for spec in [
        r#"{"executable":"git","fixed":true}"#,
        r#"{"executable":"git","scope":"trusted"}"#,
        r#"{"executable":"git","scope":"project"}"#,
    ] {
        let (code, env, _err) =
            run_policy(cfg.path(), &["policy", "grant", "add", "bad-grant", spec]).await;
        assert_eq!(code, 2, "{spec}: {env}");
        assert_error(&env, ErrorCode::InvalidRequest);
        assert_eq!(policy_text(cfg.path()), before, "{spec}");
    }

    // `fixed:false`・`inputs`/`outputs` 宣言は reject 対象外(schema 合法)。
    let (code, env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"git","fixed":false,"outputs":["out/**"]}"#,
        ],
    )
    .await;
    assert_eq!(code, 0, "{env}");
    assert!(policy_text(cfg.path()).contains("git-log"));
}

/// 既存 grant name への add は `InvalidRequest` + `details.grant`
/// (collision と同じ fail-loud 規約 — 置き換えは remove → add)。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_add_rejects_existing_name() {
    let cfg = tempfile::tempdir().unwrap();
    let (code, _env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"git"}"#,
        ],
    )
    .await;
    assert_eq!(code, 0);
    let before = policy_text(cfg.path());
    let (code, env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"cargo"}"#,
        ],
    )
    .await;
    assert_eq!(code, 2, "{env}");
    assert_error(&env, ErrorCode::InvalidRequest);
    assert_eq!(env["error"]["details"]["grant"], json!("git-log"));
    assert_eq!(policy_text(cfg.path()), before);
}

/// `command` 以外の section は rewrite 後も保存される。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_add_preserves_other_sections() {
    let cfg = tempfile::tempdir().unwrap();
    std::fs::write(
        policy_path(cfg.path()),
        "version: 1\ntools: {allow: [read, grep]}\nexecution: {timeout: 10m}\n",
    )
    .unwrap();
    let (code, env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"git"}"#,
        ],
    )
    .await;
    assert_eq!(code, 0, "{env}");
    let policy = cheaper_core::policy::load::read_policy_yaml(
        &policy_path(cfg.path()).to_string_lossy(),
        false,
    )
    .unwrap()
    .unwrap();
    assert_eq!(policy["tools"]["allow"], json!(["read", "grep"]));
    assert_eq!(policy["execution"]["timeout"], json!("10m"));
    assert_eq!(
        policy["command"]["grants"]["git-log"]["executable"],
        json!("git")
    );
}

/// 破損した既存 file(`version` 欠落・`version` ≠1・非 object の
/// `command`/`command.grants`)への add/remove は
/// `InvalidRuntimePolicy` で fail し file を変更しない —
/// `version: 1` の silent 書き足し rewrite は行わない。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_fails_on_corrupt_existing_file() {
    for body in [
        // `version` 欠落(merged object の Partial validate で捕捉)。
        "command: {}\n",
        // `version` ≠1。
        "version: 2\ncommand: {}\n",
        // 非 object `command`。
        "version: 1\ncommand: 42\n",
        // 非 object `command.grants`。
        "version: 1\ncommand: {grants: 42}\n",
        // yaml parse 不能。
        "::: not yaml :::[",
    ] {
        let cfg = tempfile::tempdir().unwrap();
        std::fs::write(policy_path(cfg.path()), body).unwrap();
        let (code, env, _err) = run_policy(
            cfg.path(),
            &[
                "policy",
                "grant",
                "add",
                "git-log",
                r#"{"executable":"git"}"#,
            ],
        )
        .await;
        assert_eq!(code, 3, "{body}: {env}");
        assert_error(&env, ErrorCode::InvalidRuntimePolicy);
        assert_eq!(policy_text(cfg.path()), body, "{body}");
    }

    // remove も同じ構造前提 pre-check + full-object validation を通る
    // — 破損 file では name 存在に関わらず loud に失敗する。
    let cfg = tempfile::tempdir().unwrap();
    let body = "version: 1\ncommand: {grants: 42}\n";
    std::fs::write(policy_path(cfg.path()), body).unwrap();
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "remove", "git-log"]).await;
    assert_eq!(code, 3, "{env}");
    assert_error(&env, ErrorCode::InvalidRuntimePolicy);
    assert_eq!(policy_text(cfg.path()), body);
}

/// 存在しない name の remove・file absent の remove は
/// `InvalidRequest`(exit 2)。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_remove_missing_name_is_invalid_request() {
    let cfg = tempfile::tempdir().unwrap();
    // file absent。
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "remove", "git-log"]).await;
    assert_eq!(code, 2, "{env}");
    assert_error(&env, ErrorCode::InvalidRequest);
    assert_eq!(env["error"]["details"]["grant"], json!("git-log"));

    // file は存在するが name が無い。
    let (code, _env, _err) = run_policy(
        cfg.path(),
        &["policy", "grant", "add", "other", r#"{"executable":"git"}"#],
    )
    .await;
    assert_eq!(code, 0);
    let before = policy_text(cfg.path());
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "remove", "git-log"]).await;
    assert_eq!(code, 2, "{env}");
    assert_error(&env, ErrorCode::InvalidRequest);
    assert_eq!(env["error"]["details"]["grant"], json!("git-log"));
    assert_eq!(policy_text(cfg.path()), before);
}

/// utility command 規約 — success envelope の `result` に top-level
/// `tokens` key を含めない(AC16。`usage_log::build_usage_record` の
/// count-record 判定との衝突回避)。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_result_has_no_tokens_key() {
    let cfg = tempfile::tempdir().unwrap();
    let (code, env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"git"}"#,
        ],
    )
    .await;
    assert_eq!(code, 0, "{env}");
    assert!(env["result"].get("tokens").is_none(), "{env}");
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "list"]).await;
    assert_eq!(code, 0);
    assert!(env["result"].get("tokens").is_none(), "{env}");
    let (code, env, _err) = run_policy(cfg.path(), &["policy", "grant", "remove", "git-log"]).await;
    assert_eq!(code, 0);
    assert!(env["result"].get("tokens").is_none(), "{env}");
}

/// `record_usage` 非呼出し — add/list/remove のいずれも
/// `usage.jsonl` に record を残さない(AC9)。
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_commands_do_not_record_usage() {
    let cfg = tempfile::tempdir().unwrap();
    for argv in [
        vec![
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"git"}"#,
        ],
        vec!["policy", "grant", "list"],
        vec!["policy", "grant", "remove", "git-log"],
    ] {
        let (code, env, _err) = run_policy(cfg.path(), &argv).await;
        assert_eq!(code, 0, "{argv:?}: {env}");
    }
    assert!(
        !cfg.path().join("usage.jsonl").exists(),
        "policy command must not append to usage.jsonl"
    );
}

/// POSIX write-mode `0600` — grant の `env` 値が secret を保持しうる
/// ため owner-only readable(design §(g))。
#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn policy_grant_add_writes_owner_only_file_mode() {
    use std::os::unix::fs::PermissionsExt;
    let cfg = tempfile::tempdir().unwrap();
    let (code, env, _err) = run_policy(
        cfg.path(),
        &[
            "policy",
            "grant",
            "add",
            "git-log",
            r#"{"executable":"git"}"#,
        ],
    )
    .await;
    assert_eq!(code, 0, "{env}");
    let mode = std::fs::metadata(policy_path(cfg.path()))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "policy.yaml must be owner-only readable");
}
