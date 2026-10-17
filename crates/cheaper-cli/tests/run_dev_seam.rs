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

/// `INVALID_OUTPUT` 確定時、最終 raw output の全文が
/// `<userConfigDir>/invalid-outputs/` へ保存され `details.rawOutputPath`
/// に path が返る(issue #114)。`rawOutput` excerpt は従来どおり先頭
/// 2000 文字に切り詰まるため、2000 字超の raw で file 側が非 truncate
/// であることを pin する。
#[tokio::test(flavor = "current_thread")]
async fn run_invalid_output_persists_full_raw_output() {
    // `{"ok":"…3000字…"}` — bulk-reader schema の required 欠落で
    // INVALID_OUTPUT。3000字超の raw で excerpt との区別を付ける。
    let padding = "x".repeat(3_000);
    let raw = format!("{{\"ok\":\"{padding}\"}}");
    let fixture = format!(
        r#"{{"name":"cli-invalid-output-raw","sessions":[
          {{"steps":[{{"finish":{{"text":{0}}}}}]}},
          {{"steps":[{{"finish":{{"text":{0}}}}}]}}]}}"#,
        serde_json::to_string(&raw).unwrap()
    );
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task":"t"}"#);
    let (code, env, _err) = f.run(&["run", "bulk-reader"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_OUTPUT"));
    assert_ne!(code, 0);

    // `rawOutput` excerpt は切り詰め済み、file は全文。
    let excerpt = env["error"]["details"]["rawOutput"]
        .as_str()
        .expect("rawOutput excerpt");
    assert!(excerpt.len() < raw.len(), "excerpt must be truncated");
    let path_str = env["error"]["details"]["rawOutputPath"]
        .as_str()
        .expect("rawOutputPath missing");
    let path = std::path::Path::new(path_str);
    assert_eq!(
        path.parent()
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_string()),
        Some("invalid-outputs".to_string()),
        "unexpected dir: {path_str}"
    );
    assert_eq!(
        std::fs::read_to_string(path).expect("raw output file"),
        raw,
        "file must hold the untruncated raw output"
    );
}

/// 保存失敗は best-effort — `rawOutputPath` が省略されるだけで
/// `INVALID_OUTPUT` の分類・exit code は不変(issue #114)。
/// `<cfg>/invalid-outputs` を通常 file として事前作成し、
/// `create_dir_all` のみを失敗させる(config dir 全体を壊すと先に
/// 別の error になる — 保存経路だけの失敗を隔離する)。
#[tokio::test(flavor = "current_thread")]
async fn run_invalid_output_save_failure_omits_path() {
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_TWO_BAD)
        .with_input(r#"{"task":"t"}"#);
    std::fs::write(f.cfg.path().join("invalid-outputs"), "x").unwrap();
    let (code, env, _err) = f.run(&["run", "bulk-reader"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_OUTPUT"));
    assert_ne!(code, 0);
    assert!(
        env["error"]["details"].get("rawOutputPath").is_none(),
        "rawOutputPath must be omitted on save failure: {}",
        env["error"]["details"]
    );
    assert!(env["error"]["details"]["rawOutput"].is_string());
}

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

// ---------------------------------------------------------------------------
// optionalTools — opt-in 縮退の dev-seam 統合 test(issue #102 T4、
// approved test-spec `docs/test-specs/tool-operator-built-in-worker.md`)。
// admission(`tools.allow`)と call 経路の認可機構は一切緩めない。
// ---------------------------------------------------------------------------

/// DSL opt-in worker — `worker.optional_tools`(snake_case)が
/// `to_markdown()` 経由で `cheaper.optionalTools` へ emit される経路を
/// 含めて縮退 run を検証する。`status` enum は `blocked` を受理する。
const OPT_IN_STATUS_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["status"],
  "properties": {
    "status": {"type": "string", "enum": ["success", "partial", "failed", "blocked"]}
  },
  "additionalProperties": false
}"#;

fn opt_in_fixture(finish_text: &str, tool_calls: Option<&str>) -> String {
    let calls = tool_calls
        .map(|c| format!(r#"{{"tool_calls": {c}}},"#))
        .unwrap_or_default();
    let turn = if tool_calls.is_some() {
        r#"{"turn": {"tool_calls": true}},"#
    } else {
        ""
    };
    format!(
        r#"{{"name": "opt_in", "worker": {{
          "name": "opt-worker",
          "tools": ["read", "process", "command"],
          "optional_tools": ["process", "command"],
          "write_scope": null,
          "output_schema": {OPT_IN_STATUS_SCHEMA}
        }}, "sessions": [{{"steps": [
          {calls}
          {turn}
          {{"finish": {{"text": {}}}}}]}}]}}"#,
        serde_json::to_string(finish_text).unwrap()
    )
}

/// builtin default policy(`process.tasks: {}` / `command.grants: {}`)
/// では opt-in した `process`/`command` が未登録へ縮退し、run は
/// `POLICY_VIOLATION` で死なず model session まで到達して完了する。
/// model が `status: "blocked"` を返しても run-level error ではなく
/// worker result として envelope へ載る(AC4/AC8)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_opt_in_degraded_run_completes_with_blocked_result() {
    let fixture = opt_in_fixture(r#"{"status":"blocked"}"#, None);
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    let (code, env, _) = f.run(&["run", "opt-worker"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], json!("opt-worker"));
    assert_eq!(env["result"], json!({"status": "blocked"}));
    // 縮退 tool は実行経路を持たない — spawn/計上ともに起きない。
    assert_eq!(env["meta"]["toolCalls"], json!(0));
    // honest metadata — 宣言された mutating tool により
    // `directlyComparable` は `process`/`command` 縮退後も false。
    assert_eq!(env["meta"]["savings"]["directlyComparable"], json!(false));
    assert_eq!(env["meta"]["changes"]["files"], json!([]));
}

/// 縮退した `process`/`command` と worker が宣言していない
/// `write`/`edit` は同じく「未登録」として call を fail させる —
/// いずれも metering/accounting に到達しない(`meta.toolCalls` は
/// 実在登録 tool `read` の 1 件のみ)。`UNKNOWN_TOOL` の literal
/// assert は runtime 層(`crates/cheaper-runtime/tests/run_dev_seam.rs`)
/// の tee-session test が担う。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_degraded_and_undeclared_calls_have_no_exec_path() {
    let calls = r#"[
      {"name": "command", "input": {"name": "self", "args": ["x"]}},
      {"name": "process", "input": {"task": "probe"}},
      {"name": "write", "input": {"path": "x.txt", "content": "x"}},
      {"name": "edit", "input": {"path": "x.txt", "oldText": "a", "newText": "b"}},
      {"name": "read", "input": {"path": "seed.txt"}}
    ]"#;
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, Some(calls));
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    std::fs::write(f.ws_path().join("seed.txt"), "seed\n").unwrap();
    let (code, env, _) = f.run(&["run", "opt-worker"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["ok"], json!(true));
    // read のみ実行 — degraded/undeclared call は全て fail-closed。
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    // write/edit call は実行されていない — audited changes は空のまま。
    assert_eq!(env["meta"]["changes"]["files"], json!([]));
    assert!(!f.ws_path().join("x.txt").exists());
}

/// opt-in worker 用 `--worker-file` — grants 非空で `command` が
/// 実登録される経路を検証する(`writeScope` なし — 不併存 rule 不発)。
const OPT_IN_COMMAND_WORKER_MD: &str = r#"---
name: opt-command-worker
description: opt-in command worker
tools: [read, command]
cheaper:
  optionalTools: [command]
  model:
    tier: low
  output:
    schema:
      inline:
        type: object
        required: [ok]
        properties:
          ok:
            type: boolean
---
Use command when granted.
"#;

/// opt-in しても grants 非空では `command` は縮退せず実登録・実 spawn
/// する — argv pattern + executable pin の authorization path は
/// 非 opt-in と同一(design §(j) / test-spec AC4)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_opt_in_command_grant_executes_and_marks_not_comparable() {
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
    let worker_path = f.ws_path().join("opt-command-worker.md");
    std::fs::write(&worker_path, OPT_IN_COMMAND_WORKER_MD).unwrap();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let warg = worker_path.to_string_lossy().into_owned();
    let (code, env, _) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    // real subprocess spawn — child が marker を実ファイルへ書く。
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
    // honest metadata — child process 副作用は audited change に混ぜない。
    assert_eq!(env["meta"]["changes"]["files"], json!([]));
    assert_eq!(env["meta"]["savings"]["directlyComparable"], json!(false));
}

/// `tools.allow` が declared tool を除外した場合は `optionalTools`
/// 宣言と無関係に admission(`check_capabilities`)で fail-closed —
/// `POLICY_VIOLATION` + `details.missing`(test-spec: admission と
/// content-gate の分離)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_opt_in_capability_denied_is_policy_violation() {
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, None);
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    // user(trusted)layer の `tools.allow` は builtin を replace —
    // opt-in 宣言は `tools.allow` を広げない。
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        "version: 1\ntools: {allow: [read]}\n",
    )
    .unwrap();
    let (code, env, _) = f.run(&["run", "opt-worker"]).await;
    assert_eq!(code, 4);
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("POLICY_VIOLATION"));
    assert_eq!(
        env["error"]["details"]["missing"],
        json!(["process", "command"]),
        "{env}"
    );
}

/// mixed policy(project layer で `process.tasks` 非空 × `command.grants`
/// 空)では failed-gate の `command` のみ縮退し、`process` は実登録・実
/// spawn する(縮退は per-tool — test-spec Rule)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_opt_in_mixed_policy_only_command_degrades() {
    let calls = r#"[
      {"name": "process", "input": {"task": "probe"}},
      {"name": "command", "input": {"name": "self", "args": ["x"]}}
    ]"#;
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, Some(calls));
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    let exe = std::env::current_exe().unwrap();
    let marker = f.ws_path().join("proc-out.txt");
    // project layer は `command` section を持てない(Option A 信頼境界)
    // — `process.tasks` のみ seed し grants は空のままにする。
    let project_policy = format!(
        "version: 1\nprocess:\n  tasks:\n    probe:\n      command: [{:?}, \"run_dev_seam_command_fixture_child\", \"--exact\", \"--nocapture\"]\n      inputs: []\n      outputs: []\n      env: {{CHEAPER_PROC_CHILD_MODE: \"probe\", CHEAPER_PROC_CHILD_ARG: {:?}}}\n",
        exe.to_string_lossy(),
        marker.to_string_lossy()
    );
    std::fs::create_dir_all(f.ws_path().join(".cheaper")).unwrap();
    std::fs::write(f.ws_path().join(".cheaper/policy.yaml"), project_policy).unwrap();
    let (code, env, _) = f.run(&["run", "opt-worker"]).await;
    assert_eq!(code, 0, "{env}");
    // process task が real spawn で marker を書き、command call は
    // 未登録で metering/accounting に到達しない。
    assert!(marker.exists());
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    assert_eq!(env["meta"]["changes"]["files"], json!([]));
}

/// mixed policy 逆方向(user layer で `command.grants` 非空 ×
/// `process.tasks` 空)では `process` のみ縮退し、`command` は
/// authorization path(grant 解決 + argv pattern + executable pin)を
/// そのまま通って実登録・実 spawn する。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_opt_in_mixed_policy_only_process_degrades() {
    let calls = r#"[
      {"name": "command", "input": {"name": "self",
        "args": ["run_dev_seam_command_fixture_child", "--exact", "--nocapture"]}},
      {"name": "process", "input": {"task": "probe"}}
    ]"#;
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, Some(calls));
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
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
    let (code, env, _) = f.run(&["run", "opt-worker"]).await;
    assert_eq!(code, 0, "{env}");
    // command grant spawn は marker を書き、degraded process call は
    // 計上されない。
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    assert_eq!(env["meta"]["changes"]["files"], json!([]));
    assert_eq!(env["meta"]["savings"]["directlyComparable"], json!(false));
}

// ---------------------------------------------------------------------------
// built-in `tool-operator` — alias 解約・status contract・bounded result・
// honest metadata・usage record(approved test-spec AC1-AC10)。
// ---------------------------------------------------------------------------

/// builtin `tool-operator` 用 fake stream — sessions のみ(worker 節は
/// builtin resolution が先に勝つため不要)。
fn tool_operator_fixture(status: &str) -> String {
    let result = serde_json::json!({
        "status": status,
        "summary": format!("run finished with {status}"),
        "findings": [{
            "title": "checked the target",
            "detail": "looked at the requested inputs",
            "evidence": [{"kind": "note", "detail": "e"}]
        }],
        "unresolved": if status == "success" { Vec::<&str>::new() } else { vec!["command unavailable"] },
        "nextActions": if status == "success" { Vec::<&str>::new() } else { vec!["grant command policy"] }
    });
    format!(
        r#"{{"name": "tool_op_{status}", "sessions": [{{"steps": [
          {{"usage_delta": {{"input": 120, "output": 40}}}},
          {{"turn": {{}}}},
          {{"finish": {{"text": {}}}}}]}}]}}"#,
        serde_json::to_string(&serde_json::to_string(&result).unwrap()).unwrap()
    )
}

/// `cheaper tool-operate` alias が `run tool-operator` と等価に builtin
/// worker を解決し、builtin default policy(tasks/grants 空)で
/// `process`/`command` が縮退しても run は model session まで到達して
/// 完了する(AC1/AC2/AC4)。`toolCalls == 0`・`meta.changes` 空・
/// `directlyComparable == false`・usage record を併せて pin する。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_tool_operate_alias_completes_degraded() {
    let fixture = tool_operator_fixture("success");
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "inspect the workspace"}"#);
    let (code, env, stderr) = f.run(&["tool-operate"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["ok"], json!(true));
    // alias は `tool-operator` へ解約される。
    assert_eq!(env["worker"], json!("tool-operator"));
    assert_eq!(env["result"]["status"], json!("success"));
    assert!(stderr.contains("model: cheaper-fake/fake-model"));
    // 縮退 tool は実行されていない。
    assert_eq!(env["meta"]["toolCalls"], json!(0));
    // honest metadata — child process 副作用は audited change に
    // 混ぜず、mutating tool 宣言で directlyComparable は false。
    assert_eq!(env["meta"]["changes"]["files"], json!([]));
    assert_eq!(env["meta"]["savings"]["directlyComparable"], json!(false));
    // usage.jsonl に run record が書かれる。
    let log = f.usage_log().expect("usage.jsonl written");
    assert!(log.contains(r#""kind":"run""#), "{log}");
    assert!(log.contains(r#""worker":"tool-operator""#), "{log}");
    assert!(log.contains(r#""directlyComparable":false"#), "{log}");
}

/// `partial` / `failed` / `blocked` はいずれも worker result であり
/// run-level error ではない — envelope `ok: true` で `result.status`
/// として返る(AC2/AC3/AC4)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_tool_operator_non_success_statuses_are_results() {
    for status in ["partial", "failed", "blocked"] {
        let fixture = tool_operator_fixture(status);
        let mut f = Fixture::new()
            .with_fake_stream(&fixture)
            .with_input(r#"{"task": "t"}"#);
        let (code, env, _) = f.run(&["run", "tool-operator"]).await;
        assert_eq!(code, 0, "{status}: {env}");
        assert_eq!(env["ok"], json!(true), "{status}");
        assert_eq!(env["result"]["status"], json!(status), "{status}");
        // non-success でも error envelope ではなく result。
        assert!(env.get("error").is_none(), "{status}: {env}");
    }
}

/// 大きな tool output は result に生転載されない — envelope の
/// `result` は compact structured output のみで、`sourceTokens` が
/// `returnedTokens` を上回る offload 実測として savings に出る(AC5)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_tool_operator_returns_bounded_result_only() {
    let calls = r#"[{"name": "read", "input": {"path": "big.txt"}}]"#;
    let result = serde_json::json!({
        "status": "success",
        "summary": "file inspected",
        "findings": [{"title": "t", "detail": "d", "evidence": [{"kind": "note", "detail": "e"}]}],
        "unresolved": [],
        "nextActions": []
    });
    let fixture = format!(
        r#"{{"name": "tool_op_bounded", "sessions": [{{"steps": [
          {{"tool_calls": {calls}}},
          {{"turn": {{"tool_calls": true}}}},
          {{"finish": {{"text": {}}}}}]}}]}}"#,
        serde_json::to_string(&serde_json::to_string(&result).unwrap()).unwrap()
    );
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "inspect big.txt"}"#);
    std::fs::write(f.ws_path().join("big.txt"), "A".repeat(64_000)).unwrap();
    let (code, env, _) = f.run(&["run", "tool-operator"]).await;
    assert_eq!(code, 0, "{env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    assert_eq!(env["result"], result);
    // result は bounded — read の生 output は envelope に現れない。
    assert!(!env.to_string().contains("AAAA"));
    let savings = &env["meta"]["savings"];
    assert!(
        savings["sourceTokens"].as_u64().unwrap() > 1_000,
        "{savings}"
    );
    assert!(
        savings["sourceTokens"].as_u64().unwrap() > savings["returnedTokens"].as_u64().unwrap(),
        "{savings}"
    );
}

// ---------------------------------------------------------------------------
// `--grant` — invocation-scoped command grant(issue #105 T5、
// approved test-spec `docs/test-specs/invocation-scoped-command-grants.md`)。
// flag 由来 grant は合成 `invocation` layer として user merge 直後に
// 畳まれ、当該 run で user-policy grant と同一の runtime 挙動を持つ一方
// policy file に一切永続化されない。
// ---------------------------------------------------------------------------

/// `--grant` flag 値(JSON)— `command_user_policy` と同型の
/// self-reshim grant(executable = test binary 自身、literal argv pin、
/// `env` で fixture mode + marker path を渡す)を `name` 内包の flag
/// JSON として組み立てる。
fn command_grant_flag(marker: &str) -> String {
    serde_json::json!({
        "name": "self",
        "executable": std::env::current_exe().unwrap().to_string_lossy(),
        "argv": ["run_dev_seam_command_fixture_child", "--exact", "--nocapture"],
        "env": {
            "CHEAPER_PROC_CHILD_MODE": "probe",
            "CHEAPER_PROC_CHILD_ARG": marker,
        }
    })
    .to_string()
}

/// `policy grant add <name> <spec-json>` 用の spec JSON —
/// `command.grants` の map value shape(`name` key を含まない)。
fn command_grant_spec(marker: &str) -> String {
    let flag: serde_json::Value = serde_json::from_str(&command_grant_flag(marker)).unwrap();
    let mut spec = flag.as_object().unwrap().clone();
    spec.remove("name");
    serde_json::Value::Object(spec).to_string()
}

/// opt-in `command` worker を `--worker-file` で配置し、`(worker arg,
/// marker path)` を返す helper。grant run 群が共有する setup。
fn opt_in_command_worker(f: &Fixture) -> (String, String) {
    let marker = f
        .ws_path()
        .join("cmd-marker")
        .to_string_lossy()
        .into_owned();
    let worker_path = f.ws_path().join("opt-command-worker.md");
    std::fs::write(&worker_path, OPT_IN_COMMAND_WORKER_MD).unwrap();
    (worker_path.to_string_lossy().into_owned(), marker)
}

/// AC1 — opt-in 縮退した `command` を `--grant` flag JSON 経由の
/// invocation grant が un-degrade し、scripted `command` call が実
/// subprocess を spawn する。grant の runtime 挙動(argv pattern・
/// executable pin・grant env 伝播)は user-policy grant と同一。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_grant_flag_un_degrades_command_and_spawns() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_COMMAND_CALL);
    let (warg, marker) = opt_in_command_worker(&f);
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let grant = command_grant_flag(&marker);
    let (code, env, err) = f
        .run(&["run", "--worker-file", &warg, "--grant", &grant])
        .await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    // 実 subprocess spawn — grant env(minimal env)経由で子が marker
    // を workspace 内へ書いた。
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
}

/// AC2 — `--grant` は一切永続化しない。granted run の後に flag 無しで
/// 同一 run を再実行すると `command` は再び縮退し(degraded call は
/// `meta.toolCalls` に計上されない)、`<cfg>/policy.yaml` は run の
/// どの時点でも書き込まれない。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_grant_flag_is_not_persisted() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_COMMAND_CALL);
    let (warg, marker) = opt_in_command_worker(&f);
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let grant = command_grant_flag(&marker);
    let (code, env, err) = f
        .run(&["run", "--worker-file", &warg, "--grant", &grant])
        .await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
    std::fs::remove_file(&marker).unwrap();

    // flag 無し再実行 — `PipeStdin::read_all` が stdin を消費するため
    // 再装填する(Fixture::run の doc comment 参照)。
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let (code, env, err) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    // 再縮退 — command call は未登録で metering/accounting に到達
    // しない。
    assert_eq!(env["meta"]["toolCalls"], json!(0));
    assert!(!std::path::Path::new(&marker).exists());
    // `--grant` は policy file を一切書かない。
    assert!(!f.cfg.path().join("policy.yaml").exists());
}

/// AC3 — deny list 該当 executable(`python` — interpreter)を含む
/// `--grant` spec は run 開始前に reject される。shared validation
/// 由来として inner code `INVALID_RUNTIME_POLICY`(exit 3)を保持し
/// `details.layer == "invocation"` を持つ — invocation 経路は deny
/// list を迂回しない。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_grant_deny_listed_executable_rejected() {
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, None);
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    let (code, env, _err) = f
        .run(&[
            "run",
            "opt-worker",
            "--grant",
            r#"{"name":"x","executable":"python"}"#,
        ])
        .await;
    assert_eq!(env["ok"], json!(false), "envelope: {env}");
    assert_eq!(env["error"]["code"], json!("INVALID_RUNTIME_POLICY"));
    assert_eq!(
        env["error"]["details"]["layer"],
        json!("invocation"),
        "envelope: {env}"
    );
    assert_eq!(code, 3, "envelope: {env}");
}

/// AC4 — flag 由来 grant が builtin/user policy の同名 grant と衝突
/// する場合、run 開始前に `INVALID_REQUEST`(exit 2) +
/// `details.grant` + `details.layer="invocation"` で fail する。
/// silent 上書き・last-wins は存在しない。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_grant_collides_with_user_policy() {
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, None);
    let mut f = Fixture::new().with_fake_stream(&fixture);
    let marker = f
        .ws_path()
        .join("cmd-marker")
        .to_string_lossy()
        .into_owned();
    // user(trusted)layer に同名 grant `self` を seed。
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        command_user_policy(&marker),
    )
    .unwrap();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let grant = command_grant_flag(&marker);
    let (code, env, _err) = f.run(&["run", "opt-worker", "--grant", &grant]).await;
    assert_eq!(env["ok"], json!(false), "envelope: {env}");
    assert_eq!(env["error"]["code"], json!("INVALID_REQUEST"));
    assert_eq!(env["error"]["details"]["grant"], json!("self"));
    assert_eq!(env["error"]["details"]["layer"], json!("invocation"));
    assert_eq!(code, 2, "envelope: {env}");
}

/// AC13 — 同一 invocation 内の同名 `--grant` 重複も `INVALID_REQUEST`
/// + `details.grant` + `details.layer` で fail(宣言順 push の暗黙
/// last-wins を許さない)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_grant_duplicate_name_rejected() {
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, None);
    let mut f = Fixture::new().with_fake_stream(&fixture);
    let marker = f
        .ws_path()
        .join("cmd-marker")
        .to_string_lossy()
        .into_owned();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let grant = command_grant_flag(&marker);
    let (code, env, _err) = f
        .run(&["run", "opt-worker", "--grant", &grant, "--grant", &grant])
        .await;
    assert_eq!(env["ok"], json!(false), "envelope: {env}");
    assert_eq!(env["error"]["code"], json!("INVALID_REQUEST"));
    assert_eq!(env["error"]["details"]["grant"], json!("self"));
    assert_eq!(env["error"]["details"]["layer"], json!("invocation"));
    assert_eq!(code, 2, "envelope: {env}");
}

/// AC5 — gated tool 欠如由来の `blocked` で tool-operator が
/// `grantRequests[]` を emit した場合、success envelope `result` から
/// parse でき宣言 output schema(`required: ["executable"]` +
/// optional `argvHint`/`name`・`additionalProperties: false`)に適合
/// する。success envelope への到達自体が field 宣言の証明(strict
/// schema validation が非適合を `INVALID_OUTPUT` に落とすため)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_tool_operator_blocked_emits_grant_requests() {
    let result = serde_json::json!({
        "status": "blocked",
        "summary": "command unavailable under current policy",
        "findings": [{
            "title": "checked the target",
            "detail": "looked at the requested inputs",
            "evidence": [{"kind": "note", "detail": "e"}]
        }],
        "unresolved": ["command unavailable"],
        "nextActions": ["grant command policy"],
        "grantRequests": [{"executable": "rg", "argvHint": ["--files"], "name": "rg"}]
    });
    let fixture = format!(
        r#"{{"name": "tool_op_grant_requests", "sessions": [{{"steps": [
          {{"usage_delta": {{"input": 120, "output": 40}}}},
          {{"turn": {{}}}},
          {{"finish": {{"text": {}}}}}]}}]}}"#,
        serde_json::to_string(&serde_json::to_string(&result).unwrap()).unwrap()
    );
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    let (code, env, err) = f.run(&["run", "tool-operator"]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    // envelope `result` から machine-readable に parse できる。
    assert_eq!(env["result"]["status"], json!("blocked"));
    assert_eq!(
        env["result"]["grantRequests"],
        json!([{"executable": "rg", "argvHint": ["--files"], "name": "rg"}]),
        "envelope: {env}"
    );
}

/// AC6(行為側半分)— `policy grant add` で user policy へ永続化した
/// grant が、後続の flag 無し opt-in run でそのまま効く(command が
/// 実登録・実 spawn)。同一 `Fixture` の連続 `f.run` —
/// `tests/common/mod.rs` の multi-call 対応が前提。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_policy_grant_add_then_run_executes() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_COMMAND_CALL);
    let (warg, marker) = opt_in_command_worker(&f);
    let spec = command_grant_spec(&marker);
    let (code, env, err) = f.run(&["policy", "grant", "add", "self", &spec]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["result"]["grant"]["name"], json!("self"));
    assert!(f.cfg.path().join("policy.yaml").exists());

    // flag 無しの run が user policy の grant で command を実行する。
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let (code, env, err) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
}

/// AC7(行為側半分)— `policy grant remove` で削除した grant は後続の
/// flag 無し run で効かなくなる(opt-in `command` が縮退へ復帰)。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_policy_grant_remove_then_run_degrades() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_COMMAND_CALL);
    let (warg, marker) = opt_in_command_worker(&f);
    // add → remove(write path 双方を通す)。
    let spec = command_grant_spec(&marker);
    let (code, env, err) = f.run(&["policy", "grant", "add", "self", &spec]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    let (code, env, err) = f.run(&["policy", "grant", "remove", "self"]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");

    // flag 無し run は再び縮退 — command call は未登録で計上されない。
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let (code, env, err) = f.run(&["run", "--worker-file", &warg]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["meta"]["toolCalls"], json!(0));
    assert!(!std::path::Path::new(&marker).exists());
}

/// AC8 — invocation-sourced grant が適用された run は stderr
/// diagnostic 行 `grant: <names>`(宣言順、`model:` line 規約に一致)
/// で audit surface を持ち、`--quiet` で抑制される。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_grant_emits_stderr_diagnostic() {
    let fixture = opt_in_fixture(r#"{"status":"success"}"#, None);
    let marker_hint = "unused-marker";
    let grant = command_grant_flag(marker_hint);

    // 非 quiet — `grant: self` 行が stderr に出る。
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    let (code, env, err) = f.run(&["run", "opt-worker", "--grant", &grant]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert!(err.contains("grant: self\n"), "stderr: {err}");

    // `--quiet` — model line・warning と同規約で抑制される。
    let mut f = Fixture::new()
        .with_fake_stream(&fixture)
        .with_input(r#"{"task": "t"}"#);
    let (code, env, err) = f
        .run(&["run", "opt-worker", "--quiet", "--grant", &grant])
        .await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(err, "", "stderr: {err}");
}

/// 非露出 warn — `command` を宣言しない worker(`--worker-file` 含む)
/// への `--grant` は warn-and-continue: stderr に warning 行を出して
/// run は続行する。`tools.allow` 除外・pin 失敗は hard
/// `POLICY_VIOLATION` のまま(warning へ変換しない)— 後者は既存
/// `run_fake_stream_opt_in_capability_denied_is_policy_violation` の
/// 無変更 green で固定。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_grant_on_worker_without_command_warns() {
    // `fixture-worker`(default tools `[read, write]` — `command` 非宣言)。
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["a.txt"]}"#);
    let grant = serde_json::json!({
        "name": "self",
        "executable": std::env::current_exe().unwrap().to_string_lossy(),
    })
    .to_string();
    let (code, env, err) = f.run(&["run", "fixture-worker", "--grant", &grant]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert!(err.contains("grant: self\n"), "stderr: {err}");
    assert!(
        err.contains("warning: --grant supplied but worker does not declare the command tool"),
        "stderr: {err}"
    );
}

/// AC10 — `tool-operate` alias 経由でも `--grant` が同等に効く
/// (`ParsedCli::Run` fold parity)。builtin `tool-operator` は
/// `optionalTools: [process, command]` opt-in 宣言済みのため flag
/// grant で `command` が実登録され、scripted call が spawn する。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_tool_operate_alias_accepts_grant() {
    let result = serde_json::json!({
        "status": "success",
        "summary": "ran the granted command",
        "findings": [{
            "title": "t", "detail": "d",
            "evidence": [{"kind": "note", "detail": "e"}]
        }],
        "unresolved": [],
        "nextActions": []
    });
    let fixture = format!(
        r#"{{"name": "tool_op_grant", "sessions": [{{"steps": [
          {{"tool_calls": [{{"name": "command", "input": {{"name": "self", "args": ["run_dev_seam_command_fixture_child", "--exact", "--nocapture"]}}}}]}},
          {{"turn": {{"tool_calls": true}}}},
          {{"usage_delta": {{"input": 100, "output": 20}}}},
          {{"finish": {{"text": {}}}}}]}}]}}"#,
        serde_json::to_string(&serde_json::to_string(&result).unwrap()).unwrap()
    );
    let mut f = Fixture::new().with_fake_stream(&fixture);
    let marker = f
        .ws_path()
        .join("cmd-marker")
        .to_string_lossy()
        .into_owned();
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let grant = command_grant_flag(&marker);
    let (code, env, err) = f.run(&["tool-operate", "--grant", &grant]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    // alias は `tool-operator` へ解約される。
    assert_eq!(env["worker"], json!("tool-operator"));
    assert_eq!(env["result"]["status"], json!("success"));
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
}

/// AC15 — repeatable `--grant` で複数 grant が 1 つの run に同時注入
/// される。2 つの grant が effective `command.grants` に共存し(pin
/// される)、stderr diagnostic の `grant:` 行に宣言順で列挙される。
#[tokio::test(flavor = "current_thread")]
async fn run_fake_stream_multiple_grants_injected() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_COMMAND_CALL);
    let (warg, marker) = opt_in_command_worker(&f);
    f.stdin.data = br#"{"task":"t"}"#.to_vec();
    let grant_self = command_grant_flag(&marker);
    // 2 件目 — 同じ pin 先(test binary)を持つ未 call の grant。
    let grant_other = serde_json::json!({
        "name": "other",
        "executable": std::env::current_exe().unwrap().to_string_lossy(),
    })
    .to_string();
    let (code, env, err) = f
        .run(&[
            "run",
            "--worker-file",
            &warg,
            "--grant",
            &grant_self,
            "--grant",
            &grant_other,
        ])
        .await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    // scripted call(grant `self`)が実 spawn する。
    assert_eq!(env["meta"]["toolCalls"], json!(1));
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
    // `grant:` 行は宣言順に両名を列挙する。
    assert!(err.contains("grant: self, other\n"), "stderr: {err}");
}

// ---------------------------------------------------------------------------
// issue #111 — `--resume` checkpoint 経路(CLI 面)
// ---------------------------------------------------------------------------

/// 検証済み schemaVersion=1 の checkpoint を `<ws>/<name>.json` へ書き、
/// その path を返す。
fn write_checkpoint(f: &Fixture, name: &str, worker: &str) -> String {
    let path = f.ws_path().join(name);
    std::fs::write(
        &path,
        serde_json::json!({
            "schemaVersion": 1,
            "worker": worker,
            "input": {"files": ["a.txt"]},
            "abortReason": "maxTurns",
            "turns": 2,
            "toolCalls": 1,
            "rejectedToolCalls": 0,
            "observationsTruncated": false,
            "observations": [
                {"tool": "read", "args": {"path": "a.txt"}, "resultExcerpt": "alpha"}
            ],
        })
        .to_string(),
    )
    .unwrap();
    path.to_string_lossy().into_owned()
}

/// `--resume <path>` — 有効な checkpoint で run が完走する。
/// `--input` 非指定時は checkpoint stored input を使い stdin を読まない
/// (stdin 空でも成功することが fallback を pin する)。
#[tokio::test(flavor = "current_thread")]
async fn run_resume_uses_checkpoint_input_and_completes() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_BASIC);
    let cp = write_checkpoint(&f, "cp.json", "fixture-worker");
    // stdin は空のまま — checkpoint input が使われる。
    let (code, env, err) = f.run(&["run", "fixture-worker", "--resume", &cp]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], json!("fixture-worker"));
    assert_eq!(env["result"], json!({"ok": true}));
}

/// `--resume` + 明示 `--input -` — stdin が優先される(explicit
/// input は stored input を上書きする)。
#[tokio::test(flavor = "current_thread")]
async fn run_resume_explicit_input_overrides_checkpoint() {
    let mut f = Fixture::new()
        .with_fake_stream(FIXTURE_BASIC)
        .with_input(r#"{"files":["b.txt"]}"#);
    // stored input は worker の input 検証を通る shape だが、
    // explicit `--input` の方が優先される(args.rs の parse test で
    // `input_explicit` 検知を pin — ここでは組合せ成立を確認)。
    let cp = write_checkpoint(&f, "cp.json", "fixture-worker");
    let (code, env, err) = f
        .run(&["run", "fixture-worker", "--resume", &cp, "--input", "-"])
        .await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
}

/// 不正 checkpoint(malformed JSON / version 不一致 / worker 不一致)
/// は `INVALID_INPUT`。
#[tokio::test(flavor = "current_thread")]
async fn run_resume_invalid_checkpoint_invalid_input() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_BASIC);

    // malformed JSON。
    let bad = f.ws_path().join("bad.json");
    std::fs::write(&bad, "{ not json").unwrap();
    let (code, env, _err) = f
        .run(&["run", "fixture-worker", "--resume", &bad.to_string_lossy()])
        .await;
    assert_eq!(code, 2);
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_INPUT"));

    // version 不一致。
    let old = f.ws_path().join("old.json");
    std::fs::write(
        &old,
        serde_json::json!({
            "schemaVersion": 2,
            "worker": "fixture-worker",
            "input": {},
            "observations": [],
        })
        .to_string(),
    )
    .unwrap();
    let (_code, env, _err) = f
        .run(&["run", "fixture-worker", "--resume", &old.to_string_lossy()])
        .await;
    assert_eq!(env["error"]["code"], json!("INVALID_INPUT"));

    // 存在しない path。
    let missing = f.ws_path().join("missing.json");
    let (_code, env, _err) = f
        .run(&[
            "run",
            "fixture-worker",
            "--resume",
            &missing.to_string_lossy(),
        ])
        .await;
    assert_eq!(env["error"]["code"], json!("INVALID_INPUT"));
}

/// checkpoint.worker ≠ resolved worker 名は `INVALID_INPUT`。
#[tokio::test(flavor = "current_thread")]
async fn run_resume_worker_mismatch_invalid_input() {
    let mut f = Fixture::new().with_fake_stream(FIXTURE_BASIC);
    let cp = write_checkpoint(&f, "cp.json", "other-worker");
    let (code, env, _err) = f.run(&["run", "fixture-worker", "--resume", &cp]).await;
    assert_eq!(code, 2);
    assert_eq!(env["error"]["code"], json!("INVALID_INPUT"));
    assert!(env["error"]["message"]
        .as_str()
        .unwrap()
        .contains("worker mismatch"));
}
