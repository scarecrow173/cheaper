//! `cheaper run` の fake-stream fixture 経路 test(`dev-seam` cargo
//! feature 前提 — `CHEAPER_TEST_FAKE_STREAM` + `NODE_ENV=test` seam が
//! feature 有効時のみ compile される)。`required-features =
//! ["dev-seam"]` で gate 済み。
//!
//! provider 解決は fixture で行うため実 credential/network 不要。
//! `CHEAPER_CONFIG_DIR` を temp dir へ向けて credential store /
//! usage.jsonl を分離する。

mod common;
use common::Fixture;

use serde_json::json;

/// 最小 success fixture(01-success-basic 相当 — fenced `{"ok":true}`)。
const FIXTURE_BASIC: &str = r#"{
  "name": "cli-t3-basic",
  "sessions": [
    {"steps": [
      {"usage_delta": {"input": 1000, "output": 2000}},
      {"turn": {}},
      {"finish": {"text": "```json\n{\"ok\":true}\n```"}}
    ]}
  ]
}"#;

/// signal abort fixture(08-signal-abort 相当 — `hang_until_abort`)。
const FIXTURE_ABORT: &str = r#"{
  "name": "cli-t3-abort",
  "abort": {"after_ms": 10, "message": "user abort"},
  "sessions": [
    {"steps": [
      {"usage_delta": {"input": 10, "output": 1}},
      {"turn": {}},
      "hang_until_abort"
    ]}
  ]
}"#;

// ---------------------------------------------------------------------------
// fake-stream fixture 経路
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_fixture_worker_fallback_success() {
    // `fixture-worker` は builtin/user/project いずれにも存在しない
    // → fixture `worker` 節へ fallback し scripted stream を実行する。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    let (code, env, err) = f.run(&["run", "fixture-worker"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], json!("fixture-worker"));
    assert_eq!(env["result"], json!({"ok": true}));
    assert_eq!(env["meta"]["model"]["provider"], json!("cheaper-fake"));
    assert_eq!(env["meta"]["model"]["id"], json!("fake-model"));
    // stderr の model line(TS `model: <prov>/<id> (tier <t>)`)。
    assert!(err.contains("model: cheaper-fake/fake-model (tier "));
    // usage.jsonl に run record が追記される。
    let log = f.usage_log().expect("usage.jsonl must be written");
    assert!(log.contains("\"kind\":\"run\""), "usage log: {log}");
}

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_quiet_suppresses_model_line() {
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    let (code, _env, err) = f.run(&["run", "fixture-worker", "--quiet"]).await;
    assert_eq!(code, 0);
    assert_eq!(err, "");
}

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_signal_abort() {
    // fixture `abort.after_ms` → signal 側 abort → ABORTED 分類。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_ABORT)
        .with_input(r#"{"files":["a.txt"]}"#);
    let (code, env, _err) = f.run(&["run", "fixture-worker"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("ABORTED"));
    assert_eq!(env["error"]["exitCode"], json!(8));
    assert_eq!(env["error"]["details"]["reason"], json!("signal"));
    assert_eq!(code, 8);
}

#[tokio::test(flavor = "current_thread")]
async fn run_worker_file_with_fake_stream() {
    // `--worker-file` — 明示 path の worker を fixture stream で実行。
    let worker_md = r#"---
name: file-worker
description: explicit worker file
tools: [read]
cheaper:
  model: {tier: low}
  output:
    schema:
      inline: {"type":"object","required":["ok"],"properties":{"ok":{"type":"boolean"}}}
---
Do it.
"#;
    let mut f = Fixture::new().with_fake_stream(FIXTURE_BASIC);
    let wpath = f.ws_path().join("file-worker.md");
    std::fs::write(&wpath, worker_md).unwrap();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let warg = wpath.to_string_lossy().into_owned();
    let (code, env, err) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], json!("file-worker"));
    assert_eq!(env["result"], json!({"ok": true}));
}

/// main + repair 用の 2 session fixture(どちらも `{"ok":true}` を
/// 返す — bulk-reader の output schema には合わず INVALID_OUTPUT
/// まで到達する)。
const FIXTURE_TWO_BAD: &str = r#"{
  "name": "cli-t3-two-bad",
  "sessions": [
    {"steps": [{"turn": {}}, {"finish": {"text": "{\"ok\":true}"}}]},
    {"steps": [{"finish": {"text": "{\"ok\":true}"}}]}
  ]
}"#;

#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_named_worker_takes_precedence_over_fallback() {
    // 実在する builtin worker(`bulk-reader`)が resolve される限り
    // fixture `worker` fallback は発火しない — bulk-reader の output
    // schema(`summary` 等必須)は `{"ok":true}` で失敗し、repair も
    // 同じ応答で exhausted → INVALID_OUTPUT に到達すれば named
    // resolution 優先が確認できる。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_TWO_BAD)
        .with_input(r#"{"task":"t"}"#);
    let (code, env, _err) = f.run(&["run", "bulk-reader"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["worker"], json!("bulk-reader"));
    assert_eq!(env["error"]["code"], json!("INVALID_OUTPUT"));
    assert_ne!(code, 0);
}

// ---------------------------------------------------------------------------
// catalog error の stderr surfacing(spec Req 4)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn run_emits_catalog_warnings_for_malformed_plugin_catalog() {
    // spec Req 4 — `run` も catalog error を `warning:` 行として stderr
    // へ出す(emit は `run_worker` より先)。envelope・exit code 不変。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    f.write_malformed_devin_catalog();
    let (code, env, err) = f.run(&["run", "fixture-worker"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["result"], json!({"ok": true}));
    assert!(
        err.contains("warning: Provider \"devin\":"),
        "stderr: {err}"
    );
    assert!(err.contains("plugin catalog"), "stderr: {err}");
    // 既存の stderr model line は変わらない。
    assert!(
        err.contains("model: cheaper-fake/fake-model"),
        "stderr: {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn run_quiet_suppresses_catalog_warnings() {
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    f.write_malformed_devin_catalog();
    let (code, env, err) = f.run(&["run", "fixture-worker", "--quiet"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    // warning・model line ともに抑制される。
    assert!(!err.contains("warning:"), "stderr: {err}");
    assert_eq!(err, "");
}

// ---------------------------------------------------------------------------
// `command` tool — worker 経路の grant execution(T5)
// ---------------------------------------------------------------------------

/// `command` grant が spawn する fixture child — `cheaper run` の
/// worker tool scope がこの test binary 自身を再実行する。
/// `CHEAPER_PROC_CHILD_MODE` 未設定(通常の `cargo test`)なら no-op。
/// `probe` は `CHEAPER_PROC_CHILD_ARG` へ marker file を書く —
/// command の spawn env は minimal env(grant.env のみ)のため、marker
/// 到達は実 spawn + grant env 伝播の両方を証明する。
#[test]
fn run_dev_seam_command_fixture_child() {
    let Ok(mode) = std::env::var("CHEAPER_PROC_CHILD_MODE") else {
        return;
    };
    match mode.as_str() {
        "probe" => {
            std::fs::write(std::env::var("CHEAPER_PROC_CHILD_ARG").unwrap(), "ran").unwrap();
        }
        other => panic!("unknown fixture mode: {other}"),
    }
}

/// `tools: [command]` の worker file — writeScope 無し(`command` との
/// 併存は scope 生成で拒否されるため)。
const COMMAND_WORKER_MD: &str = r#"---
name: cmd-worker
description: command tool worker
tools: [command]
cheaper:
  model: {tier: low}
  output:
    schema:
      inline: {"type":"object","required":["ok"],"properties":{"ok":{"type":"boolean"}}}
---
Do it.
"#;

/// `command` tool call を 1 件 emit してから正常終了する fixture。
const FIXTURE_COMMAND_CALL: &str = r#"{
  "name": "cli-t5-command",
  "sessions": [
    {"steps": [
      {"tool_calls": [{"name": "command", "input": {"name": "self", "args": ["run_dev_seam_command_fixture_child", "--exact", "--nocapture"]}}]},
      {"turn": {"tool_calls": true}},
      {"usage_delta": {"input": 100, "output": 20}},
      {"finish": {"text": "{\"ok\":true}"}}
    ]}
  ]
}"#;

/// `command` 用の trusted user layer `policy.yaml` — grant executable
/// はこの test binary 自身(workspace 外のため workspace-resident 拒否
/// に掛からず、ambient PATH 非依存で pin できる)。`argv` は literal
/// pin、`env` で fixture mode + marker path を渡す。
fn command_user_policy(marker: &str) -> String {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    format!(
        "version: 1\ncommand:\n  grants:\n    self:\n      executable: '{exe}'\n      argv: ['run_dev_seam_command_fixture_child', '--exact', '--nocapture']\n      env:\n        CHEAPER_PROC_CHILD_MODE: probe\n        CHEAPER_PROC_CHILD_ARG: '{marker}'\n"
    )
}

/// worker 経路の `command` — `tools: [command]` worker の tool scope
/// が trusted user layer grant で実 subprocess を spawn する
/// (model→tool_call→`tool_runtime.execute`→managed subprocess の全
/// 経路)。`meta.savings.directlyComparable == false`(command は
/// mutating tool — child mutation 未観測のため estimated 母数に
/// 参加させない)と `meta.changes` が journal 実測のみ(子が実際に
/// 書いた marker を audited change として主張しない)を固定する。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_command_grant_executes_and_marks_not_comparable() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_COMMAND_CALL);
    let marker = f
        .ws_path()
        .join("cmd-marker")
        .to_string_lossy()
        .into_owned();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        command_user_policy(&marker),
    )
    .unwrap();
    let wpath = f.ws_path().join("cmd-worker.md");
    std::fs::write(&wpath, COMMAND_WORKER_MD).unwrap();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let warg = wpath.to_string_lossy().into_owned();
    let (code, env, err) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["result"], json!({"ok": true}));
    // 実 subprocess spawn の証跡 — grant env(minimal env)経由で子が
    // workspace 内に marker を書いた。
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    // `command` は mutating tool — `is_directly_comparable_worker` が
    // false → `savings.directlyComparable == false`。
    assert_eq!(env["meta"]["savings"]["directlyComparable"], json!(false));
    // journal 実測のみ — 子が書いた marker は audited change として
    // 報告されない(spec §meta.changes)。
    assert_eq!(env["meta"]["changes"]["files"], json!([]));
}

/// `tools: [command]` worker + grants 無し(builtin default)は
/// `create_worker_scope` の nonempty grants gate で `POLICY_VIOLATION`
/// — session 開始前に fail-closed(exit 4)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_command_worker_denied_without_grants() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_COMMAND_CALL);
    let wpath = f.ws_path().join("cmd-worker.md");
    std::fs::write(&wpath, COMMAND_WORKER_MD).unwrap();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let warg = wpath.to_string_lossy().into_owned();
    let (code, env, _err) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(env["ok"], json!(false), "envelope: {env}");
    assert_eq!(
        env["error"]["code"],
        json!("POLICY_VIOLATION"),
        "envelope: {env}"
    );
    assert!(env["error"]["message"]
        .as_str()
        .unwrap()
        .contains("nonempty authorized grants"));
    assert_eq!(code, 4, "envelope: {env}");
}
