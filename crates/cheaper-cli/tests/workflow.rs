//! `workflow interface` / `workflow run`(T7)の in-process 結合 test。
//!
//! `packages/cli/test/workflow-run.test.ts` / `workflow-interface.test.ts`
//! の public contract に対応する:
//! - interface 射影(`{schemaVersion, inputs, outputs}` → utility envelope)
//! - literal output / `$from` / `$input` binding
//! - tool job の artifact ledger → `meta.artifacts`(preview なし)
//! - worker job の `CHEAPER_TEST_FAKE_STREAM` fixture 経路は
//!   `workflow_dev_seam.rs` へ分離(`dev-seam` cargo feature 前提、
//!   `required-features` で gate)
//! - partial failure の jobs 投影と exit code
//! - timeout parsing / policy restriction
//! - quiet が汎用 error diagnostic を抑える
//! - hooks.signal の abort 伝播
//!
//! `CHEAPER_CONFIG_DIR` を temp dir へ向けて credential store /
//! usage.jsonl を分離する。

mod common;
use common::{ambient_env, Fixture};

use cheaper_cli::cli::{run_cli, CliIo};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// workflow interface
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn workflow_interface_projects_public_shape() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {
            "target": { "type": "string" },
            "count": { "type": "number", "default": 3 }
        },
        "jobs": {},
        "outputs": {
            "ok": { "schema": { "type": "boolean" }, "value": { "$literal": true } }
        }
    }));
    let (code, env, err) = f.run(&["workflow", "interface", "workflow.json"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(env["worker"], Value::Null);
    // `projectInterface` — `{schemaVersion, inputs, outputs}`。
    assert_eq!(env["result"]["schemaVersion"], json!(1));
    assert_eq!(env["result"]["inputs"]["target"]["required"], json!(true));
    // `default` 付き input は required:false。
    assert_eq!(env["result"]["inputs"]["count"]["required"], json!(false));
    assert_eq!(
        env["result"]["outputs"]["ok"]["schema"],
        json!({"type": "boolean"})
    );
    // job は実行されない — jobs キーは result に含まれない。
    assert!(env["result"].get("jobs").is_none());
    assert!(env["meta"]["durationMs"].is_number());
}

#[tokio::test(flavor = "current_thread")]
async fn workflow_interface_missing_file() {
    let mut f = Fixture::new();
    let (code, env, _err) = f.run(&["workflow", "interface", "missing.json"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_REQUEST"));
    assert_eq!(env["error"]["message"], json!("cannot read workflow file"));
    assert_ne!(code, 0);
}

// ---------------------------------------------------------------------------
// workflow run — literal outputs / inputs
// ---------------------------------------------------------------------------

/// TS test: "omits stdin and preserves false, zero, null and tokens
/// without usage records" 相当。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_literal_outputs_success() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {},
        "outputs": {
            "falsy": { "schema": { "type": "boolean" }, "value": { "$literal": false } },
            "tokens": { "schema": { "type": "number" }, "value": { "$literal": 0 } },
            "empty": { "schema": { "type": "null" }, "value": { "$literal": null } }
        }
    }));
    // stdin 未指定でも読まれない(`args.input === undefined` →
    // `read_task_input` 非呼出し)。
    f.stdin.tty = true;
    let (code, env, err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(
        env["result"],
        json!({"falsy": false, "tokens": 0, "empty": null})
    );
    assert_eq!(env["workflow"], json!(true));
    assert_eq!(env["worker"], Value::Null);
    assert_eq!(env["jobs"], json!([]));
    assert_eq!(env["meta"]["artifacts"], json!([]));
    // workflow run は usage record 対象外。
    assert!(f.usage_log().is_none(), "usage.jsonl must not be written");
}

/// TS test: "reads explicit stdin and input files" 相当。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_input_stdin_and_file() {
    let definition = json!({
        "schemaVersion": 1,
        "inputs": { "x": { "type": "number" } },
        "jobs": {},
        "outputs": {
            "value": { "schema": { "type": "number" }, "value": { "$input": "x" } }
        }
    });
    let mut f = Fixture::new()
        .with_workflow(definition.clone())
        .with_input(r#"{"x":0}"#);
    let (code, env, err) = f
        .run(&["workflow", "run", "workflow.json", "--input", "-"])
        .await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["result"], json!({"value": 0}));

    let mut f = Fixture::new().with_workflow(definition);
    let input_file = f.ws_path().join("input.json");
    std::fs::write(&input_file, r#"{"x":7}"#).unwrap();
    let arg = input_file.to_string_lossy().into_owned();
    let (code, env, err) = f
        .run(&["workflow", "run", "workflow.json", "--input", &arg])
        .await;
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(env["result"], json!({"value": 7}));
}

// ---------------------------------------------------------------------------
// tool job + artifact ledger
// ---------------------------------------------------------------------------

/// TS test: "publishes bounded artifact metadata as unavailable without
/// a top-level preview" 相当。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_tool_artifact_metadata() {
    let mut f = Fixture::new();
    std::fs::write(f.ws_path().join("published.txt"), "PRIVATE_ARTIFACT_BODY").unwrap();
    f = f.with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "publish": {
                "kind": "tool",
                "tool": "read",
                "input": { "path": "published.txt", "artifact": "source" }
            }
        },
        "outputs": {
            "reference": {
                "schema": { "type": "object" },
                "value": { "$from": "publish", "path": "/details/artifact" }
            }
        }
    }));
    let (code, env, err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["ok"], json!(true));
    assert_eq!(
        env["result"]["reference"]["producer"],
        json!({"jobId": "publish", "name": "source"})
    );
    // artifact metadata は ledger 経由 — `availability: unavailable`
    // (invocation scope は既に dispose 済み)、`preview` key なし。
    let artifacts = env["meta"]["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0]["availability"], json!("unavailable"));
    assert_eq!(
        artifacts[0]["producer"],
        json!({"jobId": "publish", "name": "source"})
    );
    assert!(artifacts[0].get("preview").is_none());
    // result 側の artifact reference は preview を保持する。
    assert_eq!(
        env["result"]["reference"]["preview"],
        json!("PRIVATE_ARTIFACT_BODY")
    );
}

/// TS test: "collects independent success and hides private results in
/// JSON and text failures" 相当。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_partial_failure_tool_jobs() {
    let mut f = Fixture::new();
    std::fs::write(f.ws_path().join("private.txt"), "PRIVATE_SENTINEL").unwrap();
    f = f.with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "bad": { "kind": "tool", "tool": "read", "input": { "path": "missing.txt" } },
            "good": { "kind": "tool", "tool": "read", "input": { "path": "private.txt" } }
        },
        "outputs": {
            "surviving": { "schema": { "type": "boolean" }, "value": { "$literal": false } }
        }
    }));
    let (code, env, err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_ne!(code, 0, "expected failure exit code; envelope: {env}");
    assert_eq!(env["ok"], json!(false));
    // 成功側 output は収集される(TS `result` assert と同じ)。
    assert_eq!(env["result"], json!({"surviving": false}));
    let jobs = env["jobs"].as_array().unwrap();
    let find = |id: &str| jobs.iter().find(|j| j["id"] == json!(id)).unwrap();
    assert_eq!(find("bad")["status"], json!("failed"));
    assert_eq!(find("bad")["meta"]["toolCalls"], json!(1));
    assert_eq!(find("good")["status"], json!("succeeded"));
    // job result の private 内容は envelope/stderr に漏れない。
    let stdout = serde_json::to_string(&env).unwrap();
    assert!(
        !(stdout + &err).contains("PRIVATE_SENTINEL"),
        "private content leaked"
    );
    // workflow error は `error` field に出る。
    assert!(env["error"]["code"].is_string());
    assert!(env["error"]["exitCode"].is_number());
}

// ---------------------------------------------------------------------------
// error paths / flags / abort
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn workflow_run_invalid_timeout_invalid_request() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {},
        "outputs": {}
    }));
    let (code, env, err) = f
        .run(&["workflow", "run", "workflow.json", "--timeout", "bogus"])
        .await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_REQUEST"));
    assert_eq!(
        env["error"]["message"],
        json!("invalid --timeout duration: bogus")
    );
    assert_ne!(code, 0);
    // engine 起動前 error の汎用 diagnostic(non-quiet)。
    assert!(err.contains("error INVALID_REQUEST:"));
}

#[tokio::test(flavor = "current_thread")]
async fn workflow_run_timeout_over_policy_policy_violation() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {},
        "outputs": {}
    }));
    let (code, env, _err) = f
        .run(&["workflow", "run", "workflow.json", "--timeout", "9999s"])
        .await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("POLICY_VIOLATION"));
    assert_ne!(code, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn workflow_run_unknown_worker_invalid_request() {
    // `resolveWorker` 失敗(存在しない worker)は preload で
    // `WORKER_NOT_FOUND`/`INVALID_WORKER_NAME` のみ無視 — その他は
    // compile 時に `unknown worker` として伝播する。
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "a": { "kind": "worker", "worker": "no-such-worker", "input": {} }
        },
        "outputs": {}
    }));
    let (code, env, _err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("INVALID_REQUEST"));
    assert!(env["error"]["message"]
        .as_str()
        .unwrap()
        .contains("unknown worker: no-such-worker"));
    assert_ne!(code, 0);
}

/// 存在するが不正な worker `.md` — `resolve_worker` の
/// `INVALID_WORKER_DEFINITION`(NOT_FOUND/INVALID_NAME 以外)は
/// `failures` に積まれ、compile の `workers.get(name)` が
/// `SharedToolError` として再 throw する(TS は同一 error object を
/// 再 throw)。envelope は wrapper を剥がして内側の code/details/
/// exitCode を使う — 以前は `INTERNAL_ERROR`(70)に畳まれていた。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_invalid_worker_definition_preserves_error() {
    let mut f = Fixture::new();
    let workers = f.cfg.path().join("workers");
    std::fs::create_dir_all(&workers).unwrap();
    // frontmatter は parse できるが `description` 欠落 →
    // `invalid worker definition: description-missing`
    // (INVALID_WORKER_DEFINITION、details に reason/sourcePath)。
    std::fs::write(workers.join("broken.md"), "---\nname: broken\n---\nbody\n").unwrap();
    f = f.with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "a": { "kind": "worker", "worker": "broken", "input": {} }
        },
        "outputs": {}
    }));
    let (code, env, _err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(
        env["error"]["code"],
        json!("INVALID_WORKER_DEFINITION"),
        "envelope: {env}"
    );
    assert_eq!(env["error"]["exitCode"], json!(3));
    assert_eq!(code, 3, "envelope: {env}");
    // validator details は envelope 側でも保持される。
    assert_eq!(
        env["error"]["details"]["reason"],
        json!("description-missing")
    );
    assert!(env["error"]["details"]["sourcePath"]
        .as_str()
        .unwrap()
        .contains("broken.md"));
}

/// `ensureModel()` 失敗(`load_model_config` の
/// `INVALID_MODEL_CONFIG` + `with_cause` 付き)は `SharedToolError` で
/// memoize され worker job executor が再 throw する。job diagnostic は
/// `find_cheaper_error` の cause chain walk で内側の `CheaperError`
/// code を拾う — 以前は `SharedToolError::source()` が内側 error 自身
/// を skip し、`TOOL_EXECUTION_FAILURE` に畳まれていた。cause 側の
/// yaml error ではなく外側の `CheaperError` code が選ばれることを
/// 確認する。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_ensure_model_failure_preserves_code() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "a": { "kind": "worker", "worker": "bulk-reader", "input": { "task": "x" } }
        },
        "outputs": {}
    }));
    // user config の models.yaml を壊す — `load_model_config` が
    // `INVALID_MODEL_CONFIG` + `with_cause`(yaml parse error)を返す。
    std::fs::write(f.cfg.path().join("models.yaml"), "broken: [unclosed\n").unwrap();
    let (code, env, _err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(env["ok"], json!(false), "envelope: {env}");
    let jobs = env["jobs"].as_array().unwrap();
    let job = jobs.iter().find(|j| j["id"] == json!("a")).unwrap();
    assert_eq!(job["status"], json!("failed"), "job: {job}");
    assert_eq!(
        job["error"]["code"],
        json!("INVALID_MODEL_CONFIG"),
        "job: {job}"
    );
    // workflow 全体の error は job 失敗の集約
    // (`TOOL_EXECUTION_FAILURE`、"workflow job failed"、exit 6)。
    assert_eq!(env["error"]["code"], json!("TOOL_EXECUTION_FAILURE"));
    assert_eq!(code, 6, "envelope: {env}");
}

/// TS test: "preserves format and quiet for errors before engine
/// execution" 相当 — `--quiet` が汎用 error diagnostic を抑える
/// (`suppressErrorDiagnostic = kind === "workflow-run" && quiet`)。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_quiet_suppresses_error_diagnostic() {
    let mut f = Fixture::new().with_workflow(json!({ "schemaVersion": 2 }));
    let (code, out, err) = {
        let env_map = &f.env_map;
        let env_lookup = move |name: &str| env_map.get(name).cloned().or_else(|| ambient_env(name));
        let cwd = f.ws.path().to_string_lossy().into_owned();
        let mut io = CliIo {
            stdin: &mut f.stdin,
            stdout: &mut f.out,
            stderr: &mut f.err,
            env: &env_lookup,
            cwd,
        };
        let code = run_cli(
            &[
                "workflow".to_string(),
                "run".to_string(),
                "workflow.json".to_string(),
                "--format".to_string(),
                "text".to_string(),
                "--quiet".to_string(),
            ],
            &mut io,
            &f.hooks,
        )
        .await;
        drop(io);
        (
            code,
            String::from_utf8_lossy(&f.out.0).into_owned(),
            String::from_utf8_lossy(&f.err.0).into_owned(),
        )
    };
    // `schemaVersion: 2` は definition schema で拒否 —
    // INVALID_REQUEST(exit 2)。
    assert_eq!(code, 2, "stdout: {out}");
    assert!(out.contains("INVALID_REQUEST"), "stdout: {out}");
    assert_eq!(err, "", "quiet は stderr diagnostic を抑える");
}

/// `hooks.signal` の abort 伝播 — abort 済み signal で起動すると
/// engine は即 `workflow aborted` で失敗する。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_pre_aborted_signal() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "a": { "kind": "tool", "tool": "read", "input": { "path": "x.txt" } }
        },
        "outputs": {}
    }));
    let controller = cheaper_tools::operation::AbortController::new();
    controller.abort(cheaper_tools::operation::AbortReason::aborted());
    f.hooks.signal = Some(controller.signal());
    let (code, env, _err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(env["ok"], json!(false));
    assert_eq!(env["error"]["code"], json!("ABORTED"));
    assert_ne!(code, 0);
}

/// `interface`/`run` 共通 — `--format text` でも envelope 構造が
/// `formatText` を通じて出力される(JSON ではなく text 形)。
#[tokio::test(flavor = "current_thread")]
async fn workflow_interface_text_format() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {},
        "outputs": {
            "ok": { "schema": { "type": "boolean" }, "value": { "$literal": true } }
        }
    }));
    let (code, out, err) = {
        let env_map = &f.env_map;
        let env_lookup = move |name: &str| env_map.get(name).cloned().or_else(|| ambient_env(name));
        let cwd = f.ws.path().to_string_lossy().into_owned();
        let mut io = CliIo {
            stdin: &mut f.stdin,
            stdout: &mut f.out,
            stderr: &mut f.err,
            env: &env_lookup,
            cwd,
        };
        let code = run_cli(
            &[
                "workflow".to_string(),
                "interface".to_string(),
                "workflow.json".to_string(),
                "--format".to_string(),
                "text".to_string(),
            ],
            &mut io,
            &f.hooks,
        )
        .await;
        drop(io);
        (
            code,
            String::from_utf8_lossy(&f.out.0).into_owned(),
            String::from_utf8_lossy(&f.err.0).into_owned(),
        )
    };
    assert_eq!(code, 0, "stderr: {err}");
    assert!(out.contains("ok"), "stdout: {out}");
}

// ---------------------------------------------------------------------------
// `kind: tool` + `command` — policy-gated grant execution(T5)
// ---------------------------------------------------------------------------

/// `command` grant が spawn する fixture child — `workflow run` の tool
/// job がこの test binary 自身を再実行する。`CHEAPER_PROC_CHILD_MODE`
/// 未設定(通常の `cargo test`)なら no-op。`probe` mode は
/// `CHEAPER_PROC_CHILD_ARG` へ marker file を書く — command の spawn
/// env は minimal env(grant.env のみ)のため、marker 到達は実 spawn +
/// grant env 伝播の両方を証明する。
#[test]
fn workflow_command_fixture_child() {
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
        "version: 1\ncommand:\n  grants:\n    self:\n      executable: '{exe}'\n      argv: ['workflow_command_fixture_child', '--exact', '--nocapture']\n      env:\n        CHEAPER_PROC_CHILD_MODE: probe\n        CHEAPER_PROC_CHILD_ARG: '{marker}'\n"
    )
}

/// `kind: tool` + `command` の end-to-end — trusted user layer の
/// grant が `tools.allow` admission → `create_worker_scope` の
/// re-injected command policy(pin + nonempty grants gate)→
/// `scope.execute` の実 subprocess まで貫通する。marker file は実
/// spawn の証跡、managed details は `$from` binding 経由で出力へ出る。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_command_grant_end_to_end() {
    let mut f = Fixture::new();
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
    f = f.with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "cmd": {
                "kind": "tool",
                "tool": "command",
                "input": {
                    "name": "self",
                    "args": ["workflow_command_fixture_child", "--exact", "--nocapture"]
                }
            }
        },
        "outputs": {
            "state": { "schema": { "type": "string" }, "value": { "$from": "cmd", "path": "/details/state" } },
            "exit": { "schema": { "type": "number" }, "value": { "$from": "cmd", "path": "/details/exitCode" } }
        }
    }));
    let (code, env, err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(code, 0, "stderr: {err}; envelope: {env}");
    assert_eq!(env["result"], json!({"state": "completed", "exit": 0}));
    // 実 subprocess spawn の証跡 — grant env(minimal env)経由で子が
    // workspace 内に marker を書いた。
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "ran");
    let jobs = env["jobs"].as_array().unwrap();
    let job = jobs.iter().find(|j| j["id"] == json!("cmd")).unwrap();
    assert_eq!(job["status"], json!("succeeded"));
    assert_eq!(job["meta"]["toolCalls"], json!(1));
    // command は journal しない — `meta.changes` に子の副作用(実際に
    // 書かれた marker)が audited change として載らない。
    assert_eq!(job["meta"]["changes"]["files"], json!([]));
}

/// grants 無し(builtin default)の `command` job は `create_worker_scope`
/// の nonempty grants gate で fail-closed — job diagnostic は
/// `POLICY_VIOLATION`、envelope は `TOOL_EXECUTION_FAILURE`(exit 6)。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_command_denied_without_grants() {
    let mut f = Fixture::new().with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "cmd": {
                "kind": "tool",
                "tool": "command",
                "input": { "name": "self", "args": [] }
            }
        },
        "outputs": {}
    }));
    let (code, env, _err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(env["ok"], json!(false), "envelope: {env}");
    let jobs = env["jobs"].as_array().unwrap();
    let job = jobs.iter().find(|j| j["id"] == json!("cmd")).unwrap();
    assert_eq!(job["status"], json!("failed"), "job: {job}");
    assert_eq!(
        job["error"]["code"],
        json!("POLICY_VIOLATION"),
        "job: {job}"
    );
    assert!(job["error"]["message"]
        .as_str()
        .unwrap()
        .contains("nonempty authorized grants"));
    assert_eq!(env["error"]["code"], json!("TOOL_EXECUTION_FAILURE"));
    assert_eq!(code, 6, "envelope: {env}");
}

/// `tools.allow` から `command` が外れた policy では admission が
/// `POLICY_VIOLATION "tool not allowed: command"` で弾く — scope 生成・
/// subprocess の手前で止まる。
#[tokio::test(flavor = "current_thread")]
async fn workflow_run_command_tool_not_allowed() {
    let mut f = Fixture::new();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        "version: 1\ntools:\n  allow: [read]\n",
    )
    .unwrap();
    f = f.with_workflow(json!({
        "schemaVersion": 1,
        "inputs": {},
        "jobs": {
            "cmd": {
                "kind": "tool",
                "tool": "command",
                "input": { "name": "self", "args": [] }
            }
        },
        "outputs": {}
    }));
    let (code, env, _err) = f.run(&["workflow", "run", "workflow.json"]).await;
    assert_eq!(env["ok"], json!(false), "envelope: {env}");
    let jobs = env["jobs"].as_array().unwrap();
    let job = jobs.iter().find(|j| j["id"] == json!("cmd")).unwrap();
    assert_eq!(job["status"], json!("failed"), "job: {job}");
    assert_eq!(
        job["error"]["code"],
        json!("POLICY_VIOLATION"),
        "job: {job}"
    );
    assert!(job["error"]["message"]
        .as_str()
        .unwrap()
        .contains("tool not allowed: command"));
    assert_eq!(env["error"]["code"], json!("TOOL_EXECUTION_FAILURE"));
    assert_eq!(code, 6, "envelope: {env}");
}
