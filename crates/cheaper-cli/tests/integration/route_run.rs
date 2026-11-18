use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use crate::common::{env_lookup, Fixture};
use cheaper_cli::cli::CliIo;
use cheaper_cli::route::payload::{encode, EncodeInput};
use cheaper_cli::route::run::{execute, RouteRunOutput};
use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Value};

#[test]
fn command_fixture() {
    let Ok(mode) = std::env::var("CHEAPER_ROUTE_CHILD_MODE") else {
        return;
    };
    match mode.as_str() {
        "io" => {
            print!("ROUTE-OUT-MARK");
            eprint!("ROUTE-ERR-MARK");
            if let Ok(marker) = std::env::var("CHEAPER_ROUTE_CHILD_MARKER") {
                std::fs::write(&marker, "ran").unwrap();
            }
            std::process::exit(7);
        }
        "ok" => {
            print!("ROUTE-OUT-MARK");
            std::process::exit(0);
        }
        "envdump" => {
            let out = std::env::var("CHEAPER_ROUTE_CHILD_OUT").unwrap();
            let body = format!(
                "FOO={}\nKEEP={}\nSECRET={}\nCWD={}",
                std::env::var("FOO").unwrap_or_else(|_| "<unset>".into()),
                std::env::var("KEEP").unwrap_or_else(|_| "<unset>".into()),
                std::env::var("SECRET").unwrap_or_else(|_| "<unset>".into()),
                std::env::current_dir().unwrap().display(),
            );
            std::fs::write(&out, body).unwrap();
        }
        "big" => {
            let n: usize = std::env::var("CHEAPER_ROUTE_CHILD_ARG")
                .unwrap_or_else(|_| "4096".into())
                .parse()
                .unwrap();
            print!("{}", "x".repeat(n));
        }
        "tail" => {
            if let Ok(marker) = std::env::var("CHEAPER_ROUTE_CHILD_MARKER") {
                std::fs::write(&marker, "ran").unwrap();
            }
            print!("{}", "x".repeat(300));
            print!("LOG-TAIL-MARK");
        }
        "sleep" => {
            if let Ok(marker) = std::env::var("CHEAPER_ROUTE_CHILD_MARKER") {
                std::fs::write(&marker, "spawned").unwrap();
            }
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
        "e2e" => {
            if let Ok(marker) = std::env::var("CHEAPER_ROUTE_CHILD_MARKER") {
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&marker)
                    .unwrap()
                    .write_all(b"ran\n")
                    .unwrap();
            }
            let mut stdin_bytes = 0usize;
            let mut buf = [0u8; 4096];
            while let Ok(n) = std::io::Read::read(&mut std::io::stdin(), &mut buf) {
                if n == 0 {
                    break;
                }
                stdin_bytes += n;
            }
            if let Ok(out) = std::env::var("CHEAPER_ROUTE_CHILD_OUT") {
                let body = format!(
                    "FOO={}\nKEEP={}\nSECRET={}\nCWD={}\nSTDIN_BYTES={stdin_bytes}",
                    std::env::var("FOO").unwrap_or_else(|_| "<unset>".into()),
                    std::env::var("KEEP").unwrap_or_else(|_| "<unset>".into()),
                    std::env::var("SECRET").unwrap_or_else(|_| "<unset>".into()),
                    std::env::current_dir().unwrap().display(),
                );
                std::fs::write(&out, body).unwrap();
            }
            print!("ROUTE-E2E-HEAD");
            print!("{}", "x".repeat(300));
            print!("ROUTE-E2E-TAIL");
            eprint!("ROUTE-E2E-ERR");
            std::process::exit(7);
        }
        other => panic!("unknown fixture mode: {other}"),
    }
}

const CHILD_ARGV: &str = "route_run::command_fixture --exact --nocapture";

fn routed_fx(f: &Fixture, workflow: &Value, grant_env: &str, grant_extra: &str, passthrough: &str) {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        format!(
            "version: 1\nroutes:\n  - name: child-run\n    match: {{command: [routed]}}\n    target: {{kind: workflow, path: routed-exec.workflow.json}}\n    grant: child-run\n    passthroughEnv: [{passthrough}]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("routed-exec.workflow.json"),
        serde_json::to_string(workflow).unwrap(),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    child-run:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      timeout: '600s'\n{grant_env}    other:\n      executable: '{exe}'\n      argv: ['*']\n{grant_extra}"
        ),
    )
    .unwrap();
}

fn exec_only_workflow(extra_jobs: Value, extra_outputs: Value) -> Value {
    let mut jobs = json!({
        "exec": {
            "kind": "tool",
            "tool": "command",
            "input": {"name": {"$input": "grant"}, "args": {"$input": "execArgs"}}
        }
    });
    for (k, v) in extra_jobs.as_object().unwrap() {
        jobs[k] = v.clone();
    }
    let mut outputs = json!({
        "execResult": {"schema": {"type": "object"}, "value": {"$from": "exec", "path": ""}}
    });
    for (k, v) in extra_outputs.as_object().unwrap() {
        outputs[k] = v.clone();
    }
    json!({
        "schemaVersion": 1,
        "inputs": {
            "grant": {"type": "string"},
            "executable": {"type": "string"},
            "argv": {"type": "array"},
            "args": {"type": "array"},
            "execArgs": {"type": "array"},
            "cwd": {"type": "string"},
            "tokenThreshold": {"type": "number"},
            "runId": {"type": "string"}
        },
        "jobs": jobs,
        "outputs": outputs,
    })
}

fn payload_token(
    f: &Fixture,
    command: &str,
    argv: &[&str],
    ambient_env: &[(&str, &str)],
    passthrough: &[&str],
    cwd: Option<String>,
) -> String {
    encode(EncodeInput {
        command: command.to_string(),
        executable: argv.first().unwrap_or(&"").to_string(),
        argv: argv.iter().map(|s| s.to_string()).collect(),
        args: argv[1..].iter().map(|s| s.to_string()).collect(),
        cwd: cwd.unwrap_or_else(|| f.ws.path().to_string_lossy().into_owned()),
        ambient_env: ambient_env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
        passthrough_env: passthrough.iter().map(|s| s.to_string()).collect(),
    })
    .unwrap()
}

async fn run_route(
    f: &mut Fixture,
    token: &str,
) -> Result<RouteRunOutput, cheaper_cli::cli::BoxError> {
    let env_lookup = env_lookup(&f.env_map);
    let cwd = f.ws.path().to_string_lossy().into_owned();
    let mut io = CliIo {
        stdin: &mut f.stdin,
        stdout: &mut f.out,
        stderr: &mut f.err,
        env: &env_lookup,
        cwd,
    };
    execute(token, None, false, &mut io, &f.hooks, 0.0).await
}

async fn run_cli_raw(f: &mut Fixture, argv: &[&str]) -> (i32, String, String) {
    f.out.0.clear();
    f.err.0.clear();
    let env = env_lookup(&f.env_map);
    let cwd = f.ws.path().to_string_lossy().into_owned();
    let mut io = CliIo {
        stdin: &mut f.stdin,
        stdout: &mut f.out,
        stderr: &mut f.err,
        env: &env,
        cwd,
    };
    let code = cheaper_cli::cli::run_cli(
        &argv.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
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
}

fn reference_workflow() -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/workflows/routed-exec.workflow.json"
        ))
        .unwrap(),
    )
    .unwrap()
}

async fn hook_route_payload(f: &mut Fixture, raw: &str) -> String {
    f.stdin.data = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {"command": raw}
    })
    .to_string()
    .into_bytes();
    let (code, out, _err) =
        run_cli_raw(f, &["hook", "pre-tool-use", "--target", "pretooluse"]).await;
    assert_eq!(code, 0, "hook out={out}");
    let hook: Value = serde_json::from_str(&out).unwrap();
    assert!(
        hook.pointer("/hookSpecificOutput/permissionDecision")
            .is_none()
            && hook.get("decision").is_none(),
        "no approval bypass: {hook}"
    );
    let command = hook
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("updatedInput 無し: {hook}"));
    let (left, token) = command
        .rsplit_once(" route run --payload ")
        .unwrap_or_else(|| panic!("wrapper delimiter 無し: {command}"));
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        left == format!("\"{exe}\"") || left == "cheaper",
        "left token: {left}"
    );
    assert!(
        !token.is_empty()
            && token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "token must be base64url: {token}"
    );
    token.to_string()
}

fn route_record(f: &Fixture) -> Value {
    let log = f.usage_log().expect("usage.jsonl written");
    log.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["kind"] == "route")
        .expect("route record")
}

fn grant_env(mode: &str, extra: &str) -> String {
    format!("      env:\n        CHEAPER_ROUTE_CHILD_MODE: {mode}\n{extra}")
}

#[tokio::test(flavor = "current_thread")]
async fn exit_mirrored_and_log_written() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("child-marker.txt");
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env(
            "io",
            &format!(
                "        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
                marker
                    .to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "''")
            ),
        ),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 7);
    assert!(out.text.contains("ROUTE-OUT-MARK"), "text: {}", out.text);
    assert!(out.text.contains("ROUTE-ERR-MARK"), "text: {}", out.text);
    assert!(
        out.text.contains("[cheaper route log:"),
        "text: {}",
        out.text
    );
    let result = &out.envelope["result"];
    assert_eq!(result["execResult"]["details"]["exitCode"], json!(7));
    assert_eq!(result["postprocessFailures"], json!([]));
    let log_ref = result["logRef"].as_str().expect("logRef");
    let log_file = f
        .cfg
        .path()
        .join("route-logs")
        .join(format!("{log_ref}.log"));
    let log = std::fs::read_to_string(&log_file).unwrap();
    assert!(log.contains("ROUTE-OUT-MARK"), "log: {log}");
    assert!(log.contains("ROUTE-ERR-MARK"), "log: {log}");
    assert!(marker.exists());
    let record = route_record(&f);
    assert_eq!(record["exitCode"], json!(7));
    assert_eq!(record["ok"], json!(false));
    assert_eq!(record["route"], json!("child-run"));
    assert_eq!(record["logRef"], json!(log_ref));
    // `command` shell 文字列本文は record に載らない —
    // `commandHash` のみ(#166 で `executed.argv` は additive に
    // 記録されるが、それは exec argv token であり shell 本文
    // `"routed …"` ではない)。
    assert!(
        record.get("command").is_none(),
        "command plaintext must not be recorded: {record}"
    );
    // `executed` — additive attribution(#166):実 exec job が実行した
    // grant と env-prefix-strip 済み argv。
    assert_eq!(record["executed"]["grant"], json!("child-run"));
    assert_eq!(
        record["executed"]["argv"],
        json!(["route_run::command_fixture", "--exact", "--nocapture"])
    );
    assert!(result["execResult"]["details"]["outputTokens"].is_number());
    assert_eq!(result["execResult"]["details"]["logRef"], json!(log_ref));
}

#[tokio::test(flavor = "current_thread")]
async fn skipped_analyze_keeps_fallback_text() {
    let mut f = Fixture::new();
    routed_fx(
        &f,
        &exec_only_workflow(
            json!({
                "analyze": {
                    "kind": "tool",
                    "tool": "read",
                    "input": {"path": "child-marker.txt"},
                    "needs": ["exec"],
                    "when": {
                        "left": {"$from": "exec", "path": "/details/outputTokens"},
                        "op": "gt",
                        "right": {"$input": "tokenThreshold"}
                    }
                }
            }),
            json!({
                "result": {
                    "schema": {"type": ["string", "null"]},
                    "value": {"$from": "analyze", "path": ""}
                }
            }),
        ),
        &grant_env("ok", ""),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert!(out.text.contains("ROUTE-OUT-MARK"), "text: {}", out.text);
    assert_eq!(out.envelope["result"]["postprocessFailures"], json!([]));
    assert_eq!(route_record(&f)["ok"], json!(true));
}

#[tokio::test(flavor = "current_thread")]
async fn post_job_failure_keeps_exit_and_records_stage() {
    let mut f = Fixture::new();
    routed_fx(
        &f,
        &exec_only_workflow(
            json!({
                "analyze": {
                    "kind": "tool",
                    "tool": "read",
                    "input": {"path": "definitely-missing.txt"},
                    "needs": ["exec"]
                }
            }),
            json!({}),
        ),
        &grant_env("io", ""),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 7);
    assert!(out.text.contains("ROUTE-OUT-MARK"), "text: {}", out.text);
    let failures = out.envelope["result"]["postprocessFailures"]
        .as_array()
        .unwrap();
    assert!(!failures.is_empty(), "failures: {failures:?}");
    assert_eq!(
        out.envelope["result"]["execResult"]["details"]["exitCode"],
        json!(7)
    );
}

/// #166 member fixture — route は `grants: [fmt, report, strict]`
/// (anchor `child-run` 以外の delegated grant member)を宣言し、
/// policy は anchor + member 3 grant + route 非宣言の `undeclared`
/// を持つ。`fmt`/`report` は `tail` mode marker で実 spawn の証跡を
/// 残し、`strict` は argv `['allowed']` のみ許可する。
fn member_fx(f: &Fixture, workflow: &Value) -> (std::path::PathBuf, std::path::PathBuf) {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let fmt_marker = f.ws.path().join("fmt-marker.txt");
    let report_marker = f.ws.path().join("report-marker.txt");
    let esc = |p: &Path| {
        p.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "''")
    };
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: member-run\n    match: {command: [routed]}\n    target: {kind: workflow, path: member.workflow.json}\n    grant: child-run\n    grants: [fmt, report, strict]\n",
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("member.workflow.json"),
        serde_json::to_string(workflow).unwrap(),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    child-run:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      timeout: '600s'\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n    fmt:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: tail\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n    report:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: tail\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n    strict:\n      executable: '{exe}'\n      argv: ['allowed']\n    undeclared:\n      executable: '{exe}'\n      argv: ['*']\n",
            esc(&fmt_marker),
            esc(&report_marker)
        ),
    )
    .unwrap();
    (fmt_marker, report_marker)
}

/// `grants:[…]` 宣言 route の member 委譲(#166)— `job_id != "exec"`
/// の `command` job は宣言 member(`fmt`/`report`)を `input.name` で
/// 実行できる。projection 外の `undeclared` 名は job 失敗へ、member
/// argv が grant pattern 外なら job 実行時 `check_exec_execution` が
/// 拒否する — member argv 照合は viability ではなく job 実行時に
/// 行われるため route 発火自体は止まらない(O2b)。失敗は
/// `job-failed:<id>` の postprocess failure として記録される。
#[tokio::test(flavor = "current_thread")]
async fn member_grant_jobs_execute_and_undeclared_denied() {
    let mut f = Fixture::new();
    let workflow = exec_only_workflow(
        json!({
            "fmt-job": {
                "kind": "tool",
                "tool": "command",
                "input": {"name": "fmt", "args": ["route_run::command_fixture", "--exact", "--nocapture"]}
            },
            "report-job": {
                "kind": "tool",
                "tool": "command",
                "input": {"name": "report", "args": ["route_run::command_fixture", "--exact", "--nocapture"]}
            },
            "undeclared-job": {
                "kind": "tool",
                "tool": "command",
                "input": {"name": "undeclared", "args": []}
            },
            "strict-job": {
                "kind": "tool",
                "tool": "command",
                "input": {"name": "strict", "args": ["denied-arg"]}
            }
        }),
        json!({}),
    );
    let (fmt_marker, report_marker) = member_fx(&f, &workflow);
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    // anchor(ok mode)exit 0 — member job 失敗は route 発火を止めず
    // postprocess failure へ畳まれる。
    assert_eq!(out.exit_code, 0);
    // member grant job は実際に spawn された(marker 証跡)。
    assert!(fmt_marker.exists(), "fmt member job must run");
    assert!(report_marker.exists(), "report member job must run");
    let failures = out.envelope["result"]["postprocessFailures"]
        .as_array()
        .unwrap();
    // projection 外名・member argv pattern 違反は job 失敗。
    assert!(
        failures.iter().any(|s| s == "job-failed:undeclared-job"),
        "failures: {failures:?}"
    );
    assert!(
        failures.iter().any(|s| s == "job-failed:strict-job"),
        "failures: {failures:?}"
    );
    // member job 自体は失敗しない。
    assert!(!failures.iter().any(|s| s == "job-failed:fmt-job"));
    assert!(!failures.iter().any(|s| s == "job-failed:report-job"));
    // record attribution は anchor のまま — `executed` は member
    // 名ではなく anchor `child-run` と exec argv を記録する。
    let record = route_record(&f);
    assert_eq!(record["route"], json!("member-run"));
    assert_eq!(record["executed"]["grant"], json!("child-run"));
    assert_eq!(
        record["executed"]["argv"],
        json!(["route_run::command_fixture", "--exact", "--nocapture"])
    );
}

/// `job_id == "exec"` × `tool: "command"` は anchor grant + 元 argv の
/// `expected_input` に束縛される(#166 O1)— member grant 名を名乗る
/// exec job は `exec-input-mismatch` PolicyViolation で job 失敗し、
/// captured result 無しのため route run は workflow job 失敗の
/// collapsed `ToolExecutionFailure` を返す(member 名による anchor
/// impersonation は成立しない)。
#[tokio::test(flavor = "current_thread")]
async fn exec_job_named_member_is_input_mismatch() {
    let mut f = Fixture::new();
    let mut workflow = exec_only_workflow(json!({}), json!({}));
    // exec job が anchor ではなく member grant 名を名乗る。
    workflow["jobs"]["exec"]["input"] = json!({"name": "fmt", "args": ["x"]});
    member_fx(&f, &workflow);
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let err = run_route(&mut f, &token)
        .await
        .err()
        .expect("route must fail: exec job input mismatch");
    assert_eq!(
        err.downcast_ref::<CheaperError>().map(CheaperError::code),
        Some(ErrorCode::ToolExecutionFailure),
        "exec job 失敗は outcome.error の collapse code: {err}"
    );
    assert!(
        err.to_string().contains("routed command did not complete"),
        "captured()==None && outcome.error.is_some() 分岐: {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn output_schema_failure_still_mirrors_real_result() {
    let mut f = Fixture::new();
    let mut wf = exec_only_workflow(json!({}), json!({}));
    wf["outputs"]["execResult"]["schema"] = json!({"type": "number"});
    routed_fx(&f, &wf, &grant_env("io", ""), "", "");
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 7);
    assert_eq!(
        out.envelope["result"]["execResult"]["details"]["exitCode"],
        json!(7)
    );
    assert!(out.text.contains("ROUTE-OUT-MARK"), "text: {}", out.text);
}

const ROUTES_YAML: &str = "version: 1\nroutes:\n  - name: child-run\n    match: {command: [routed]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: child-run\n    passthroughEnv: [FOO]\n";

fn reject_fx(f: &Fixture, routes_yaml: &str, grant_yaml_tail: &str, marker: &Path) {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(f.cfg.path().join("routes.yaml"), routes_yaml).unwrap();
    std::fs::write(
        f.cfg.path().join("routed-exec.workflow.json"),
        serde_json::to_string(&exec_only_workflow(json!({}), json!({}))).unwrap(),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    child-run:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: io\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n{grant_yaml_tail}",
            marker.to_string_lossy().replace('\\', "\\\\").replace('\'', "''")
        ),
    )
    .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn revalidation_failures_reject_without_executing() {
    let argv = [
        "routed",
        "route_run::command_fixture",
        "--exact",
        "--nocapture",
    ];
    async fn expect_reject(f: &mut Fixture, token: &str, marker: &Path) {
        let result = run_route(f, token).await;
        assert!(result.is_err(), "case must reject");
        assert!(!marker.exists(), "raw command must not run");
        let log = f.usage_log().unwrap_or_default();
        assert!(
            !log.lines().any(|l| l.contains("\"route\"")),
            "no route usage record: {log}"
        );
    }
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, ROUTES_YAML, "", &marker);
        expect_reject(&mut f, "!!!not-base64!!!", &marker).await;
    }
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, ROUTES_YAML, "", &marker);
        let token = payload_token(
            &f,
            &format!("routed {CHILD_ARGV}"),
            &["routed", "different"],
            &[],
            &[],
            None,
        );
        expect_reject(&mut f, &token, &marker).await;
    }
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, ROUTES_YAML, "", &marker);
        let token = payload_token(&f, "other arg", &["other", "arg"], &[], &[], None);
        expect_reject(&mut f, &token, &marker).await;
    }
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        let foreign = tempfile::tempdir().unwrap();
        reject_fx(&f, ROUTES_YAML, "", &marker);
        let token = payload_token(
            &f,
            &format!("routed {CHILD_ARGV}"),
            &argv,
            &[],
            &[],
            Some(foreign.path().to_string_lossy().into_owned()),
        );
        expect_reject(&mut f, &token, &marker).await;
        drop(foreign);
    }
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, ROUTES_YAML, "", &marker);
        let token = payload_token(
            &f,
            &format!("routed {CHILD_ARGV}"),
            &argv,
            &[("SECRET", "leak")],
            &["SECRET"],
            None,
        );
        expect_reject(&mut f, &token, &marker).await;
    }
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(
            &f,
            "version: 1\nroutes:\n  - name: child-run\n    match: {command: [routed]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: absent\n",
            "",
            &marker,
        );
        let token = payload_token(&f, &format!("routed {CHILD_ARGV}"), &argv, &[], &[], None);
        expect_reject(&mut f, &token, &marker).await;
    }
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, ROUTES_YAML, "", &marker);
        std::fs::remove_file(f.cfg.path().join("routed-exec.workflow.json")).unwrap();
        let token = payload_token(&f, &format!("routed {CHILD_ARGV}"), &argv, &[], &[], None);
        expect_reject(&mut f, &token, &marker).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn exec_infra_failure_returns_error() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    reject_fx(&f, ROUTES_YAML, "      timeout: '1ms'\n", &marker);
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        std::fs::read_to_string(f.cfg.path().join("policy.yaml"))
            .unwrap()
            .replace(
                "CHEAPER_ROUTE_CHILD_MODE: io",
                "CHEAPER_ROUTE_CHILD_MODE: sleep",
            ),
    )
    .unwrap();
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let err = run_route(&mut f, &token)
        .await
        .err()
        .expect("route must fail: grant timeout is 1ms");
    assert_eq!(
        err.downcast_ref::<CheaperError>().map(CheaperError::code),
        Some(ErrorCode::ToolExecutionFailure),
        "job 失敗は engine が outcome.error を ToolExecutionFailure に collapse する決定的 infra-failure code",
    );
    assert!(
        err.to_string().contains("routed command did not complete"),
        "captured()==None && outcome.error.is_some() 分岐固有の message — workflow が exec job を失敗まで実行した証拠: {err}",
    );
    let log = f.usage_log().unwrap_or_default();
    assert!(
        !log.lines().any(|l| l.contains("\"route\"")),
        "no route record: {log}"
    );
}

#[cfg(feature = "dev-seam")]
#[tokio::test(flavor = "current_thread")]
async fn worker_parity_and_route_log_read() {
    let seeded_id = "9999999999999-1-deadbeef";
    let seeded_words = (0..300)
        .map(|i| format!("seeded-word-{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let fixture = format!(
        "{{\"name\":\"route-analyze\",\"sessions\":[{{\"steps\":[{{\"turn\":{{\"tool_calls\":true}}}},{{\"tool_calls\":[{{\"name\":\"read\",\"input\":{{\"source\":{{\"kind\":\"artifact\",\"reference\":{{\"artifact\":\"{seeded_id}\",\"runId\":\"{seeded_id}\",\"producer\":{{\"jobId\":\"exec\",\"name\":\"command\"}},\"bytes\":0,\"preview\":\"\",\"truncated\":false}}}},\"offset\":0,\"maxBytes\":32768}}}}]}},{{\"usage_delta\":{{\"input\":120,\"output\":30}}}},{{\"finish\":{{\"text\":\"{{\\\"summary\\\":\\\"FAKE-ANALYZE-MARK\\\"}}\"}}}}]}}]}}"
    );
    let mut f = Fixture::new().with_fake_stream(&fixture);
    std::fs::create_dir_all(f.cfg.path().join("route-logs")).unwrap();
    std::fs::write(
        f.cfg
            .path()
            .join("route-logs")
            .join(format!("{seeded_id}.log")),
        format!("SEEDED-WORKER-READ-MARK {seeded_words}"),
    )
    .unwrap();
    std::fs::create_dir_all(f.cfg.path().join("workers")).unwrap();
    std::fs::write(
        f.cfg.path().join("workers/route-analyzer.md"),
        "---\nname: route-analyzer\ndescription: route log analyzer\ntools: [read]\ncheaper:\n  model: {tier: low}\n  input:\n    schema:\n      inline: {\"type\":\"object\",\"required\":[\"task\"],\"properties\":{\"task\":{\"type\":\"string\"}}}\n  output:\n    schema:\n      inline: {\"type\":\"object\",\"required\":[\"summary\"],\"properties\":{\"summary\":{\"type\":\"string\"}}}\n---\nAnalyze the route log.\n",
    )
    .unwrap();
    let marker = f.ws.path().join("marker.txt");
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(f.cfg.path().join("routes.yaml"), ROUTES_YAML).unwrap();
    std::fs::write(
        f.cfg.path().join("routed-exec.workflow.json"),
        serde_json::to_string(&exec_only_workflow(
            json!({
                "readlog": {
                    "kind": "tool",
                    "tool": "read",
                    "input": {
                        "source": {
                            "kind": "artifact",
                            "reference": {
                                "artifact": {"$input": "runId"},
                                "runId": {"$input": "runId"},
                                "producer": {"jobId": "exec", "name": "command"},
                                "bytes": 0,
                                "preview": "",
                                "truncated": false
                            }
                        },
                        "offset": 0,
                        "maxBytes": 32768
                    },
                    "needs": ["exec"]
                },
                "analyze": {
                    "kind": "worker",
                    "worker": "route-analyzer",
                    "input": {"task": {"$literal": "summarize"}},
                    "needs": ["readlog"]
                }
            }),
            json!({
                "result": {
                    "schema": {"type": "string"},
                    "value": {"$from": "readlog", "path": "/content/0/text"}
                }
            }),
        ))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    child-run:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      maxOutputBytes: 64\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: tail\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
            marker.to_string_lossy().replace('\\', "\\\\").replace('\'', "''")
        ),
    )
    .unwrap();
    let token = payload_token(
        &f,
        &format!("FOO=secret-pfx routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[("FOO", "ambient-val")],
        &["FOO"],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert_eq!(out.envelope["result"]["postprocessFailures"], json!([]));
    assert!(out.text.contains("LOG-TAIL-MARK"), "text: {}", out.text);
    assert!(marker.exists(), "child executed");
    let record = route_record(&f);
    assert_eq!(record["worker"]["model"]["provider"], json!("cheaper-fake"));
    assert!(record["worker"]["usage"].is_object(), "usage: {record}");
    assert!(
        record["durationMs"]["analyze"].as_f64().unwrap_or(0.0) > 0.0,
        "analyze duration: {record}"
    );
    assert!(
        !out.text.contains("secret-pfx"),
        "prefix value leaked: {}",
        out.text
    );
    assert!(
        !record.to_string().contains("secret-pfx"),
        "record leaked prefix"
    );
    assert!(
        !record.to_string().contains("ambient-val"),
        "record leaked ambient"
    );
    // `executed`(#166)— env-prefix-strip 済みの実実行 argv を記録
    // し、`FOO=secret-pfx` 等の prefix 値は argv にも含まれない。
    assert_eq!(record["executed"]["grant"], json!("child-run"));
    assert_eq!(
        record["executed"]["argv"],
        json!(["route_run::command_fixture", "--exact", "--nocapture"])
    );
}

#[tokio::test(flavor = "current_thread")]
async fn env_prefix_beats_ambient_beats_grant() {
    let mut f = Fixture::new();
    f.env_map.insert("SECRET".to_string(), "leak".to_string());
    let dump = f.ws.path().join("env-dump.txt");
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env(
            "envdump",
            &format!(
                "        CHEAPER_ROUTE_CHILD_OUT: '{}'\n        FOO: grant-val\n        KEEP: keep\n",
                dump.to_string_lossy().replace('\\', "\\\\").replace('\'', "''")
            ),
        ),
        "",
        "FOO",
    );
    let token = payload_token(
        &f,
        &format!("FOO=prefix routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[("FOO", "ambient")],
        &["FOO"],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    let env_dump = std::fs::read_to_string(&dump).unwrap();
    let expected_cwd =
        cheaper_core::workspace::paths::canonicalize_workspace_root(&f.ws.path().to_string_lossy())
            .unwrap();
    assert!(env_dump.contains("FOO=prefix"), "env: {env_dump}");
    assert!(env_dump.contains("KEEP=keep"), "env: {env_dump}");
    assert!(env_dump.contains("SECRET=<unset>"), "env: {env_dump}");
    assert!(
        env_dump.contains(&format!("CWD={expected_cwd}")),
        "env: {env_dump}"
    );
    let record = route_record(&f);
    assert!(!record.to_string().contains("prefix"));
    assert!(!record.to_string().contains("ambient"));
}

#[tokio::test(flavor = "current_thread")]
async fn other_grant_blocked_by_projected_policy() {
    let mut f = Fixture::new();
    routed_fx(
        &f,
        &exec_only_workflow(
            json!({
                "sneaky": {
                    "kind": "tool",
                    "tool": "command",
                    "input": {"name": "other", "args": []},
                    "needs": ["exec"]
                }
            }),
            json!({}),
        ),
        &grant_env("io", ""),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 7);
    assert!(out.text.contains("ROUTE-OUT-MARK"), "text: {}", out.text);
    let failures = out.envelope["result"]["postprocessFailures"]
        .as_array()
        .unwrap();
    assert!(!failures.is_empty(), "failures: {failures:?}");
}

#[tokio::test(flavor = "current_thread")]
async fn layer_file_root_and_workspace_inference_root() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("child-marker.txt");
    std::fs::write(&marker, "WS-MARKER-CONTENT").unwrap();
    routed_fx(
        &f,
        &exec_only_workflow(
            json!({
                "readmark": {
                    "kind": "tool",
                    "tool": "read",
                    "input": {"path": "child-marker.txt"},
                    "needs": ["exec"]
                }
            }),
            json!({
                "result": {
                    "schema": {"type": "string"},
                    "value": {"$from": "readmark", "path": "/content/0/text"}
                }
            }),
        ),
        &grant_env("ok", ""),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert!(out.text.contains("WS-MARKER-CONTENT"), "text: {}", out.text);
}

#[tokio::test(flavor = "current_thread")]
async fn bounded_text_full_log() {
    let mut f = Fixture::new();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: child-run\n    match: {command: [routed]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: child-run\n",
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("routed-exec.workflow.json"),
        serde_json::to_string(&exec_only_workflow(json!({}), json!({}))).unwrap(),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    child-run:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      maxOutputBytes: 64\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: big\n        CHEAPER_ROUTE_CHILD_ARG: '4096'\n"
        ),
    )
    .unwrap();
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert!(out.text.contains("truncated: true"), "text: {}", out.text);
    assert!(out.text.len() < 4096, "text len {}", out.text.len());
    let details = &out.envelope["result"]["execResult"]["details"];
    assert_eq!(details["stdout"]["truncated"], json!(true));
    let log_ref = details["logRef"].as_str().unwrap();
    let log_file = f
        .cfg
        .path()
        .join("route-logs")
        .join(format!("{log_ref}.log"));
    let log_len = std::fs::metadata(&log_file).unwrap().len();
    assert!(log_len > 4096, "log len {log_len}");
    assert!(details["outputTokens"].as_u64().unwrap() > 0);
}

#[tokio::test(flavor = "current_thread")]
async fn tiny_count_cap_partial_count() {
    let mut f = Fixture::new();
    f.env_map
        .insert("CHEAPER_COUNT_MAX_BYTES".to_string(), "64".to_string());
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env("big", ""),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    let details = &out.envelope["result"]["execResult"]["details"];
    assert_eq!(details["outputTokensTruncated"], json!(true));
    let failures = out.envelope["result"]["postprocessFailures"]
        .as_array()
        .unwrap();
    assert!(
        failures.iter().any(|s| s.as_str() == Some("partial-count")),
        "failures: {failures:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn log_begin_failure_preserves_exit_without_logref() {
    let mut f = Fixture::new();
    std::fs::write(f.cfg.path().join("route-logs"), b"block").unwrap();
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env("ok", ""),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    assert!(out.envelope["result"].get("logRef").is_none());
    assert!(out.text.contains("unavailable"), "text: {}", out.text);
    let failures = out.envelope["result"]["postprocessFailures"]
        .as_array()
        .unwrap();
    assert!(
        failures.iter().any(|s| s.as_str() == Some("log-begin")),
        "failures: {failures:?}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn cli_run_emits_raw_text_exit_and_route_record() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env(
            "io",
            &format!(
                "        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
                marker
                    .to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "''")
            ),
        ),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let (code, out, _) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_eq!(code, 7);
    assert!(out.contains("ROUTE-OUT-MARK"), "out: {out}");
    assert!(out.contains("ROUTE-ERR-MARK"), "out: {out}");
    assert!(out.contains("[cheaper route log:"), "out: {out}");
    assert!(
        serde_json::from_str::<Value>(&out).is_err(),
        "raw text must not be a JSON envelope"
    );
    let record = route_record(&f);
    assert_eq!(record["kind"], "route");
    assert_eq!(record["exitCode"], 7);
    assert!(record["worker"].is_null() || record["worker"].is_object());
    let (code2, out2, _) = run_cli_raw(
        &mut f,
        &["route", "run", "--payload", &token, "--format", "json"],
    )
    .await;
    assert_eq!(code2, 7);
    assert!(out2.contains("ROUTE-OUT-MARK"), "out: {out2}");
    assert!(serde_json::from_str::<Value>(&out2).is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn cli_run_invalid_payload_is_error_envelope() {
    let mut f = Fixture::new();
    reject_fx(&f, ROUTES_YAML, "", &f.ws.path().join("marker.txt"));
    let (code, out, _) =
        run_cli_raw(&mut f, &["route", "run", "--payload", "!!!not-base64!!!"]).await;
    assert_ne!(code, 0);
    let envelope: Value = serde_json::from_str(&out)
        .unwrap_or_else(|_| panic!("error path must emit envelope: {out}"));
    assert!(envelope["ok"].is_boolean() || envelope["error"].is_object());
    assert!(!f.ws.path().join("marker.txt").exists());
    let log = f.usage_log().unwrap_or_default();
    assert!(!log.lines().any(|l| l.contains("\"route\"")), "{log}");
}

#[tokio::test(flavor = "current_thread")]
async fn cli_run_quiet_suppresses_usage_failure_diagnostic() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    std::fs::create_dir_all(f.cfg.path().join("usage.jsonl")).unwrap();
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env(
            "io",
            &format!(
                "        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
                marker
                    .to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "''")
            ),
        ),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let (code, out, err) =
        run_cli_raw(&mut f, &["route", "run", "--payload", &token, "--quiet"]).await;
    assert_eq!(code, 7);
    assert!(out.contains("ROUTE-OUT-MARK"), "out: {out}");
    assert!(!err.contains("usage-log"), "quiet must suppress: {err}");
    let (code, _, err) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_eq!(code, 7);
    assert!(err.contains("usage-log"), "diagnostic expected: {err}");
}

#[tokio::test(flavor = "current_thread")]
async fn cli_run_cwd_override_uses_foreign_root() {
    let mut f = Fixture::new();
    let foreign = tempfile::tempdir().unwrap();
    let dump = f.ws.path().join("env-dump.txt");
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env(
            "envdump",
            &format!(
                "        CHEAPER_ROUTE_CHILD_OUT: '{}'\n",
                dump.to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "''")
            ),
        ),
        "",
        "",
    );
    let foreign_str = foreign.path().to_string_lossy().into_owned();
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        Some(foreign_str.clone()),
    );
    let (code, _, _) = run_cli_raw(
        &mut f,
        &["route", "run", "--payload", &token, "--cwd", &foreign_str],
    )
    .await;
    assert_eq!(code, 0, "override must accept foreign cwd");
    let dumped = std::fs::read_to_string(&dump).unwrap();
    assert!(
        dumped.contains(&format!(
            "CWD={}",
            cheaper_core::workspace::paths::canonicalize_workspace_root(&foreign_str).unwrap()
        )),
        "dump: {dumped}"
    );
    let (code2, _, _) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_ne!(code2, 0, "without override the same payload must reject");
    drop(foreign);
}

#[tokio::test(flavor = "current_thread")]
async fn suppressed_project_provenance_reaches_route_record() {
    // `trusted` 非宣言の project route は同一 `match.command` で
    // builtin route に負け、負けた project route 名(`project-shadow`)
    // が resolve 出力と `kind:"route"` record の `suppressed` へ
    // 到達する(#168 — project 層は下位層を shadow しない)。
    let mut f = Fixture::new();
    routed_fx(
        &f,
        &reference_workflow(),
        "      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n",
        "",
        "FOO",
    );
    let project_dir = f.ws.path().join(".cheaper");
    std::fs::create_dir_all(&project_dir).unwrap();
    let production = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/workflows/routed-exec.workflow.json"
    ))
    .unwrap();
    std::fs::write(project_dir.join("p.json"), production).unwrap();
    std::fs::write(
        project_dir.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: project-shadow\n    match: {command: [routed]}\n    target: {kind: workflow, path: p.json}\n    grant: child-run\n",
    )
    .unwrap();
    let (code, out, err) = f
        .run(&[
            "route",
            "resolve",
            "--argv",
            "[\"routed\",\"route_run::command_fixture\",\"--exact\",\"--nocapture\"]",
        ])
        .await;
    assert_eq!(code, 0, "out={out} err={err}");
    let result = &out["result"];
    assert_eq!(result["route"], "child-run");
    assert_eq!(result["suppressed"], "project-shadow");
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let (code, _out, _err) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_eq!(code, 0, "route run must execute");
    let record = route_record(&f);
    assert_eq!(record["route"], "child-run");
    assert_eq!(record["suppressed"], "project-shadow", "{record}");
}

#[tokio::test(flavor = "current_thread")]
async fn e2e_hook_rewrite_to_cli_route_run() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    let dump = f.ws.path().join("env-dump.txt");
    routed_fx(
        &f,
        &reference_workflow(),
        &format!(
            "      env:\n        CHEAPER_ROUTE_CHILD_MODE: e2e\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n        CHEAPER_ROUTE_CHILD_OUT: '{}'\n        FOO: E2E-GRANT-FOO\n        KEEP: E2E-GRANT-KEEP\n",
            marker.to_string_lossy().replace('\'', "''"),
            dump.to_string_lossy().replace('\'', "''"),
        ),
        "",
        "FOO, KEEP",
    );
    f.env_map.insert("FOO".into(), "E2E-CAPTURED-FOO".into());
    f.env_map.insert("KEEP".into(), "E2E-CAPTURED-KEEP".into());
    f.env_map.insert("SECRET".into(), "E2E-SECRET-VALUE".into());
    let raw = format!("FOO=E2E-PREFIX-VALUE routed {CHILD_ARGV}");
    let (code, out, _err) = f.run(&["route", "resolve", "--command", &raw]).await;
    assert_eq!(code, 0, "resolve out={out}");
    let result = &out["result"];
    assert_eq!(result["matched"], true);
    assert_eq!(result["route"], "child-run");
    assert_eq!(result["grant"], "child-run");
    assert_eq!(result["viability"], "runnable");
    assert_eq!(
        result["args"],
        json!(["route_run::command_fixture", "--exact", "--nocapture"])
    );
    let token = hook_route_payload(&mut f, &raw).await;
    assert!(!marker.exists(), "hook is dry-run — no side effect");
    assert!(f.usage_log().is_none(), "no route record before run");
    let (code, out, _err) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_eq!(code, 7, "out={out}");
    assert!(out.contains("ROUTE-E2E-HEAD"), "out={out}");
    assert!(out.contains("ROUTE-E2E-TAIL"), "out={out}");
    assert!(out.contains("[cheaper route log:"), "out={out}");
    let dumped = std::fs::read_to_string(&dump).unwrap();
    assert!(dumped.contains("FOO=E2E-PREFIX-VALUE"), "{dumped}");
    assert!(dumped.contains("KEEP=E2E-CAPTURED-KEEP"), "{dumped}");
    assert!(dumped.contains("SECRET=<unset>"), "{dumped}");
    assert!(dumped.contains("STDIN_BYTES=0"), "{dumped}");
    let expected_cwd =
        cheaper_core::workspace::paths::canonicalize_workspace_root(&f.ws.path().to_string_lossy())
            .unwrap();
    assert!(dumped.contains(&format!("CWD={expected_cwd}")), "{dumped}");
    assert_eq!(
        std::fs::read_to_string(&marker).unwrap(),
        "ran\n",
        "marker appended exactly once"
    );
    let record = route_record(&f);
    assert_eq!(record["postprocessFailures"], 0);
    assert!(
        record["worker"].get("model").is_none() && record["worker"].get("usage").is_none(),
        "condition-skipped worker must not meter: {record}"
    );
    let serialized = serde_json::to_string(&record).unwrap();
    for secret in ["E2E-PREFIX-VALUE", "E2E-SECRET-VALUE", "E2E-GRANT-FOO"] {
        assert!(
            !serialized.contains(secret),
            "record leaks {secret}: {serialized}"
        );
    }
}

/// `cd <subdir> && <routable>` — hook が mint する cd unwrap
/// payload の subdir cwd で routed child が実行される観測可能な
/// 証明(envdump child が自身の cwd を file へ記録する)。
#[tokio::test(flavor = "current_thread")]
async fn e2e_cd_prefix_unwrap_runs_child_in_subdir() {
    let mut f = Fixture::new();
    let sub = f.ws.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let dump = f.ws.path().join("env-dump.txt");
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env(
            "envdump",
            &format!(
                "        CHEAPER_ROUTE_CHILD_OUT: '{}'\n",
                dump.to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "''")
            ),
        ),
        "",
        "",
    );
    // hook(pretooluse)の Rewrite command 経由で mint — payload.cwd
    // は canonical 済み subdir、payload.command は原文のまま。
    let raw = format!("cd sub && routed {CHILD_ARGV}");
    let token = hook_route_payload(&mut f, &raw).await;
    assert!(!dump.exists(), "hook is dry-run — no side effect");
    let (code, out, err) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_eq!(code, 0, "out={out} err={err}");
    let dumped = std::fs::read_to_string(&dump).unwrap();
    let expected =
        cheaper_core::workspace::paths::canonicalize_workspace_root(&sub.to_string_lossy())
            .unwrap();
    assert!(dumped.contains(&format!("CWD={expected}")), "{dumped}");
}

/// 受理外の cd 形・workspace 外/不存在の移動先は rewrite を emit
/// しない — hook は fail-open(deny ではなく command 素通し)。
#[tokio::test(flavor = "current_thread")]
async fn e2e_cd_prefix_fail_open_emits_no_rewrite() {
    let mut f = Fixture::new();
    let sub = f.ws.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    routed_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &grant_env("ok", ""),
        "",
        "",
    );
    for raw in [
        format!("cd .. && routed {CHILD_ARGV}"),
        format!("cd missing && routed {CHILD_ARGV}"),
        format!("cd sub || routed {CHILD_ARGV}"),
        format!("cd sub | routed {CHILD_ARGV}"),
        format!("cd sub && routed {CHILD_ARGV} && echo done"),
        format!("pushd sub && routed {CHILD_ARGV}"),
        format!("FOO=1 cd sub && routed {CHILD_ARGV}"),
    ] {
        f.stdin.data = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {"command": raw}
        })
        .to_string()
        .into_bytes();
        let (code, out, err) =
            run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "pretooluse"]).await;
        assert_eq!(code, 0, "{raw}: err={err}");
        assert!(!out.contains("route run --payload"), "{raw}: {out}");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn e2e_bounded_output_full_log_and_refetch() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    routed_fx(
        &f,
        &reference_workflow(),
        &format!(
            "      env:\n        CHEAPER_ROUTE_CHILD_MODE: e2e\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n      maxOutputBytes: 128\n",
            marker.to_string_lossy().replace('\'', "''"),
        ),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let (code, out, _err) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_eq!(code, 7, "out={out}");
    assert!(out.contains("ROUTE-E2E-HEAD"), "out={out}");
    assert!(
        !out.contains("ROUTE-E2E-TAIL"),
        "bounded must drop tail: {out}"
    );
    assert!(out.contains("truncated: true"), "out={out}");
    let record = route_record(&f);
    let log_ref = record["logRef"].as_str().unwrap().to_string();
    let log = std::fs::read_to_string(
        f.cfg
            .path()
            .join("route-logs")
            .join(format!("{log_ref}.log")),
    )
    .unwrap();
    assert!(log.contains("ROUTE-E2E-TAIL"), "full log keeps tail");
    assert!(log.contains("ROUTE-E2E-ERR"), "full log keeps stderr");
    let (code, page, _err) = f
        .run(&["route", "logs", &log_ref, "--max-bytes", "16"])
        .await;
    assert_eq!(code, 0, "page={page}");
    assert_eq!(page["result"]["data"].as_str().unwrap().len(), 16);
    assert_eq!(page["result"]["nextOffset"], 16);
    let refetch: Vec<_> = f
        .usage_log()
        .unwrap()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|r| r["kind"] == "route-refetch")
        .collect();
    assert_eq!(refetch.len(), 1);
    assert_eq!(refetch[0]["logRef"], log_ref);
    assert_eq!(refetch[0]["bytesRead"], 16);
}

#[tokio::test(flavor = "current_thread")]
async fn e2e_stale_grant_fails_closed_after_rewrite() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    routed_fx(
        &f,
        &reference_workflow(),
        &format!(
            "      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
            marker.to_string_lossy().replace('\'', "''"),
        ),
        "",
        "",
    );
    let token = hook_route_payload(&mut f, &format!("routed {CHILD_ARGV}")).await;
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    other:\n      executable: '{exe}'\n      argv: ['*']\n"
        ),
    )
    .unwrap();
    let (code, out, _err) = run_cli_raw(&mut f, &["route", "run", "--payload", &token]).await;
    assert_ne!(code, 0);
    let envelope: Value = serde_json::from_str(&out)
        .unwrap_or_else(|_| panic!("error path must emit envelope: {out}"));
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "POLICY_VIOLATION", "{envelope}");
    assert!(!marker.exists(), "raw command must not run");
    let log = f.usage_log().unwrap_or_default();
    assert!(
        !log.lines().any(|l| l.contains("\"route\"")),
        "no route record: {log}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn routed_binary_does_not_forward_parent_stdin() {
    let f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    let dump = f.ws.path().join("env-dump.txt");
    routed_fx(
        &f,
        &reference_workflow(),
        &format!(
            "      env:\n        CHEAPER_ROUTE_CHILD_MODE: e2e\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n        CHEAPER_ROUTE_CHILD_OUT: '{}'\n",
            marker.to_string_lossy().replace('\'', "''"),
            dump.to_string_lossy().replace('\'', "''"),
        ),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_cheaper"))
        .args(["route", "run", "--payload", &token])
        .current_dir(f.ws.path())
        .env_clear()
        .envs(&f.env_map)
        .env(
            "SystemRoot",
            std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into()),
        )
        .envs(
            ["TEMP", "TMP", "HOME", "USERPROFILE"]
                .iter()
                .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v))),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"MUST-NOT-REACH-CHILD")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(7),
        "stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(
        std::fs::read_to_string(&marker).unwrap(),
        "ran\n",
        "exactly one child execution"
    );
    let dumped = std::fs::read_to_string(&dump).unwrap();
    assert!(dumped.contains("STDIN_BYTES=0"), "{dumped}");
}

#[tokio::test(flavor = "current_thread")]
async fn devin_hook_rewrite_emits_nested_updated_input() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    reject_fx(&f, ROUTES_YAML, "", &marker);
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    child-run:\n      executable: '{}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      timeout: '600s'\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n",
            std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .replace('\'', "''"),
        ),
    )
    .unwrap();
    f.stdin.data = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "exec",
        "tool_input": {"command": format!("routed {CHILD_ARGV}")}
    })
    .to_string()
    .into_bytes();
    let (code, hook_out, _err) =
        run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "devin"]).await;
    assert_eq!(code, 0, "hook out={hook_out}");
    let hook: Value = serde_json::from_str(&hook_out).unwrap();
    assert!(
        hook.get("decision").is_none(),
        "top decision must be absent: {hook}"
    );
    assert!(
        hook.get("updatedInput").is_none(),
        "top updatedInput absent: {hook}"
    );
    assert_eq!(
        hook.pointer("/hookSpecificOutput/hookEventName")
            .and_then(Value::as_str),
        Some("PreToolUse")
    );
    let command = hook
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("devin nested updatedInput 無し: {hook}"));
    assert!(command.contains(" route run --payload "), "{command}");
    assert!(hook
        .pointer("/hookSpecificOutput/permissionDecision")
        .is_none());
    assert!(!marker.exists());
}

/// codex — user policy `route.rewrite.approvalBypass: [codex]` + grant
/// で `--target codex` firing が `permissionDecision:"allow"` +
/// `updatedInput.command`(`route run --payload` 形)を emit し exit 0
/// (R6 — bypassing emit は flag opt-in 時のみ)。
#[tokio::test(flavor = "current_thread")]
async fn codex_hook_rewrite_with_approval_bypass_emits_allow_updated_input() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    reject_fx(&f, ROUTES_YAML, "", &marker);
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\nroute:\n  rewrite:\n    approvalBypass: [codex]\ncommand:\n  grants:\n    child-run:\n      executable: '{}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      timeout: '600s'\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n",
            std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .replace('\'', "''"),
        ),
    )
    .unwrap();
    f.stdin.data = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {"command": format!("routed {CHILD_ARGV}")}
    })
    .to_string()
    .into_bytes();
    let (code, hook_out, _err) =
        run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "codex"]).await;
    assert_eq!(code, 0, "hook out={hook_out}");
    let hook: Value = serde_json::from_str(&hook_out).unwrap();
    assert!(
        hook.get("decision").is_none() && hook.get("updatedInput").is_none(),
        "top-level fields absent: {hook}"
    );
    assert_eq!(
        hook.pointer("/hookSpecificOutput/hookEventName")
            .and_then(Value::as_str),
        Some("PreToolUse")
    );
    assert_eq!(
        hook.pointer("/hookSpecificOutput/permissionDecision")
            .and_then(Value::as_str),
        Some("allow")
    );
    let command = hook
        .pointer("/hookSpecificOutput/updatedInput/command")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("codex bypassing emit 無し: {hook}"));
    assert!(command.contains(" route run --payload "), "{command}");
    assert!(!marker.exists());
}

/// codex — flag 未設定・空配列・codex 非含有のとき codex render は
/// fallback verdict のみを返し `hookSpecificOutput` に `updatedInput`
/// を含まない(R7 — `builtin_cargo_test_route_registered_but_no_grant`
/// の「no-grant は rewrite しない」と対になる「flag 無しは rewrite
/// しない」pin)。
#[tokio::test(flavor = "current_thread")]
async fn codex_hook_rewrite_without_flag_renders_fallback_only() {
    for route_yaml in [
        "",
        "route:\n  rewrite:\n    approvalBypass: []\n",
        "route:\n  rewrite:\n    approvalBypass: [cursor]\n",
    ] {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, ROUTES_YAML, "", &marker);
        std::fs::write(
            f.cfg.path().join("policy.yaml"),
            format!(
                "version: 1\n{route_yaml}command:\n  grants:\n    child-run:\n      executable: '{}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      timeout: '600s'\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n",
                std::env::current_exe()
                    .unwrap()
                    .to_string_lossy()
                    .replace('\'', "''"),
            ),
        )
        .unwrap();
        f.stdin.data = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {"command": format!("routed {CHILD_ARGV}")}
        })
        .to_string()
        .into_bytes();
        let (code, hook_out, _err) =
            run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "codex"]).await;
        assert_eq!(code, 0, "hook out={hook_out}");
        let hook: Value = serde_json::from_str(&hook_out).unwrap_or(Value::Null);
        assert!(
            hook.pointer("/hookSpecificOutput/updatedInput").is_none(),
            "flag 無し({route_yaml:?})は rewrite emit しない: {hook_out}"
        );
        assert!(
            !hook_out.contains("\"permissionDecision\":\"allow\""),
            "bypassing emit 無し: {hook_out}"
        );
        assert!(!marker.exists());
    }
}

/// approval-required tier — inline code flag(`bash -c`)を持つ typed
/// command は `approval-required` viability で rewrite emit されるが、
/// `route.rewrite.approvalBypass` opt-in の有無に関わらず
/// `permissionDecision:"allow"` を併記しない(host 承認 prompt が残る、
/// R33bis)。codex は「承認付き置換」を表現できないため fallback
/// verdict へ倒れる。bare `bash`(positional 無し)は structural
/// `stdin-unsupported` で emit されない。
#[tokio::test(flavor = "current_thread")]
async fn approval_tier_rewrite_never_emits_bypass_decision() {
    const BASH_ROUTES_YAML: &str = "version: 1\nroutes:\n  - name: child-run\n    match: {command: [bash]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: child-run\n";
    let policy = |bypass: &str| {
        format!(
            "version: 1\nroute:\n  rewrite:\n    approvalBypass: [{bypass}]\ncommand:\n  grants:\n    child-run:\n      executable: '{}'\n      argv: ['*']\n      timeout: '600s'\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n",
            std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .replace('\'', "''"),
        )
    };
    // devin — bypass opt-in ありでも approval tier は
    // `permissionDecision` を併記せず updatedInput のみ emit。
    for bypass in ["", "devin"] {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, BASH_ROUTES_YAML, "", &marker);
        std::fs::write(f.cfg.path().join("policy.yaml"), policy(bypass)).unwrap();
        f.stdin.data = serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "exec",
            "tool_input": {"command": "bash -c 'echo hi'"}
        })
        .to_string()
        .into_bytes();
        let (code, hook_out, _err) =
            run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "devin"]).await;
        assert_eq!(code, 0, "hook out={hook_out}");
        let hook: Value = serde_json::from_str(&hook_out).unwrap();
        let command = hook
            .pointer("/hookSpecificOutput/updatedInput/command")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("bypass={bypass:?}: updatedInput 無し: {hook}"));
        assert!(command.contains(" route run --payload "), "{command}");
        assert!(
            hook.pointer("/hookSpecificOutput/permissionDecision")
                .is_none(),
            "bypass={bypass:?}: approval tier で allow 併記は不可: {hook}"
        );
        assert!(!marker.exists());
    }
    // codex — bypass opt-in ありでも approval tier は fallback
    // verdict(render 不可能な emit shape)へ倒れる。
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, BASH_ROUTES_YAML, "", &marker);
        std::fs::write(f.cfg.path().join("policy.yaml"), policy("codex")).unwrap();
        f.stdin.data = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {"command": "bash -c 'echo hi'"}
        })
        .to_string()
        .into_bytes();
        let (code, hook_out, _err) =
            run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "codex"]).await;
        assert_eq!(code, 0, "hook out={hook_out}");
        let hook: Value = serde_json::from_str(&hook_out).unwrap_or(Value::Null);
        assert!(
            hook.pointer("/hookSpecificOutput/updatedInput").is_none(),
            "codex approval tier は updatedInput を emit しない: {hook_out}"
        );
        assert!(!marker.exists());
    }
    // devin — bare `bash` は structural `stdin-unsupported` で
    // rewrite emit されない。
    {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("marker.txt");
        reject_fx(&f, BASH_ROUTES_YAML, "", &marker);
        std::fs::write(f.cfg.path().join("policy.yaml"), policy("devin")).unwrap();
        f.stdin.data = serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "exec",
            "tool_input": {"command": "bash"}
        })
        .to_string()
        .into_bytes();
        let (code, hook_out, _err) =
            run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "devin"]).await;
        assert_eq!(code, 0, "hook out={hook_out}");
        let hook: Value = serde_json::from_str(&hook_out).unwrap_or(Value::Null);
        assert!(
            hook.pointer("/hookSpecificOutput/updatedInput").is_none(),
            "structural shape は emit しない: {hook_out}"
        );
        assert!(!marker.exists());
    }
}

/// codex — untrusted project 層 `.cheaper/policy.yaml` の `route` key は
/// project partial schema 違反で `load_policy` 全体を失敗させ、
/// `load_or_none` → `None` → route arm 不活性 + bypass 無効となる。
/// user 層 policy に flag があっても hook は rewrite を発行しない
/// (R3/R15 — project は flag を「消せる」が「有効化」は構造的に不能
/// の end-to-end 側)。
#[tokio::test(flavor = "current_thread")]
async fn codex_hook_project_route_key_fails_closed() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("marker.txt");
    reject_fx(&f, ROUTES_YAML, "", &marker);
    // user 層には flag + grant を正しく置く — project 層の `route` key
    // だけが load を壊す差分。
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\nroute:\n  rewrite:\n    approvalBypass: [codex]\ncommand:\n  grants:\n    child-run:\n      executable: '{}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      timeout: '600s'\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n",
            std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .replace('\'', "''"),
        ),
    )
    .unwrap();
    let project_dir = f.ws.path().join(".cheaper");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(
        project_dir.join("policy.yaml"),
        "version: 1\nroute:\n  rewrite:\n    approvalBypass: [codex]\n",
    )
    .unwrap();
    f.stdin.data = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {"command": format!("routed {CHILD_ARGV}")}
    })
    .to_string()
    .into_bytes();
    let (code, hook_out, _err) =
        run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "codex"]).await;
    assert_eq!(code, 0, "hook out={hook_out}");
    let hook: Value = serde_json::from_str(&hook_out).unwrap_or(Value::Null);
    assert!(
        hook.pointer("/hookSpecificOutput/updatedInput").is_none(),
        "project `route` key は load error → rewrite 不発行(fail-closed): {hook_out}"
    );
    assert!(!marker.exists());
}

#[tokio::test(flavor = "current_thread")]
async fn builtin_cargo_test_route_registered_but_no_grant() {
    let mut f = Fixture::new();
    let (code, out, _err) = f
        .run(&["route", "resolve", "--argv", "[\"cargo\",\"test\"]"])
        .await;
    assert_eq!(code, 0, "out={out}");
    let result = &out["result"];
    assert_eq!(result["matched"], true);
    assert_eq!(result["route"], "cargo-test");
    assert_eq!(result["viability"], "no-grant");
    let raw = "cargo test";
    f.stdin.data = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": {"command": raw}
    })
    .to_string()
    .into_bytes();
    let (code, hook_out, _err) =
        run_cli_raw(&mut f, &["hook", "pre-tool-use", "--target", "pretooluse"]).await;
    assert_eq!(code, 0, "hook out={hook_out}");
    let hook: Value = serde_json::from_str(&hook_out).unwrap_or(Value::Null);
    assert!(
        hook.pointer("/hookSpecificOutput/updatedInput/command")
            .is_none(),
        "no-grant must not rewrite: {hook}"
    );
}

// ---- substitute-exec route mode(#167)----

/// substitute fixture — `route.grant` は `anchor`(firing gate /
/// attribution anchor・member 級検査のみで実行されない)、実実行は
/// `substitute: {grant: sub-exec, argv: [child argv]}`。`anchor` に
/// `env` を置くのは「anchor env が子へ届かない」証明のため。
/// `sub_env` は `sub-exec` grant の `      env:` block 全体、
/// `grants_extra` は追加 grant 宣言、`route_extra` は追加 route key。
fn substitute_fx(
    f: &Fixture,
    workflow: &Value,
    sub_env: &str,
    grants_extra: &str,
    route_extra: &str,
) {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        format!(
            "version: 1\nroutes:\n  - name: sub-run\n    match: {{command: [routed]}}\n    target: {{kind: workflow, path: sub.workflow.json}}\n    grant: anchor\n    substitute: {{grant: sub-exec, argv: ['route_run::command_fixture', '--exact', '--nocapture']}}\n{route_extra}"
        ),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("sub.workflow.json"),
        serde_json::to_string(workflow).unwrap(),
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    anchor:\n      executable: '{exe}'\n      argv: ['*']\n      env:\n        FOO: anchor-val\n        KEEP: anchor-keep\n    sub-exec:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      timeout: '600s'\n{sub_env}{grants_extra}"
        ),
    )
    .unwrap();
}

const SUB_ARGV: &[&str] = &["route_run::command_fixture", "--exact", "--nocapture"];

/// exec-input 再 anchor・exit mirror・record shape(#167 AC5/AC6/
/// AC8)— typed command `routed anything --else` で route は発火
/// するが exec job が実行するのは `{sub-exec, substitute.argv}`。
/// 原 exec_args は実行対象ではなく record も `executed` に
/// substitute ペアを auto-capture する(`route`/`commandHash` の
/// 宣言 identity は維持・schemaVersion 不変・新規 field なし)。
#[tokio::test(flavor = "current_thread")]
async fn substitute_exec_pair_runs_and_mirrors_exit() {
    let mut f = Fixture::new();
    let marker = f.ws.path().join("sub-marker.txt");
    substitute_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &format!(
            "      env:\n        CHEAPER_ROUTE_CHILD_MODE: io\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
            marker
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('\'', "''")
        ),
        "",
        "",
    );
    let token = payload_token(
        &f,
        "routed anything --else",
        &["routed", "anything", "--else"],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    // substitute child(io mode → exit 7)をそのまま mirror。
    assert_eq!(out.exit_code, 7);
    assert!(out.text.contains("ROUTE-OUT-MARK"), "text: {}", out.text);
    assert!(marker.exists(), "substitute pair spawned child");
    let details = &out.envelope["result"]["execResult"]["details"];
    assert_eq!(details["exitCode"], json!(7));
    // `command::execute` details が substitute ペアを auto-capture。
    assert_eq!(details["grant"], json!("sub-exec"), "{details}");
    assert_eq!(details["argv"], json!(SUB_ARGV), "{details}");
    let record = route_record(&f);
    // 宣言 anchor attribution + substitute 実行 attribution —
    // `route`/`commandHash` は原 identity、`executed` は substitute
    // ペア。record shape は schemaVersion:1 不変・新規 field 無し。
    assert_eq!(record["schemaVersion"], json!(1));
    assert_eq!(record["kind"], json!("route"));
    assert_eq!(record["route"], json!("sub-run"));
    assert!(record["commandHash"].is_string(), "{record}");
    assert_eq!(record["executed"]["grant"], json!("sub-exec"));
    assert_eq!(record["executed"]["argv"], json!(SUB_ARGV));
    assert_eq!(record["exitCode"], json!(7));
    assert_eq!(record["ok"], json!(false));
}

/// `job_id == "exec"` は `{substitute.grant, substitute.argv}` の
/// `expected_input` に束縛される(#167 AC5)— `route.grant` + 原
/// exec_args・substitute 名 + 原 exec_args・member 名のいずれも
/// `exec-input-mismatch` で job 失敗し(anchor 名による exec
/// impersonation は成立しない)、captured 無しの workflow 失敗と
/// して collapsed `ToolExecutionFailure` を返す。
#[tokio::test(flavor = "current_thread")]
async fn substitute_exec_job_input_mismatch() {
    for (case, input) in [
        // route.grant(anchor)名 + 原 exec_args — byte-matched 時代
        // の旧 pair は substitute mode では受理されない。
        (
            "anchor+original-args",
            json!({"name": "anchor", "args": ["anything", "--else"]}),
        ),
        // substitute 名でも原 exec_args(typed argv 由来)は不一致。
        (
            "substitute+original-args",
            json!({"name": "sub-exec", "args": ["anything", "--else"]}),
        ),
        // member 名も exec job では名乗れない。
        (
            "member-name",
            json!({"name": "fmt", "args": ["route_run::command_fixture", "--exact", "--nocapture"]}),
        ),
    ] {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("sub-marker.txt");
        let mut workflow = exec_only_workflow(json!({}), json!({}));
        workflow["jobs"]["exec"]["input"] = input;
        let exe = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        substitute_fx(
            &f,
            &workflow,
            &format!(
                "      env:\n        CHEAPER_ROUTE_CHILD_MODE: io\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
                marker
                    .to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "''")
            ),
            &format!(
                "    fmt:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n"
            ),
            "    grants: [fmt]\n",
        );
        let token = payload_token(
            &f,
            "routed anything --else",
            &["routed", "anything", "--else"],
            &[],
            &[],
            None,
        );
        let err = run_route(&mut f, &token)
            .await
            .err()
            .unwrap_or_else(|| panic!("{case}: route must fail on exec-input-mismatch"));
        assert_eq!(
            err.downcast_ref::<CheaperError>().map(CheaperError::code),
            Some(ErrorCode::ToolExecutionFailure),
            "{case}: exec job 失敗は outcome.error の collapse code: {err}"
        );
        assert!(
            err.to_string().contains("routed command did not complete"),
            "{case}: captured()==None && outcome.error.is_some() 分岐: {err}"
        );
        assert!(!marker.exists(), "{case}: mismatched job must not spawn");
    }
}

/// lifecycle anchor(#167 AC7)— exec job は `substitute.grant` の宣言
/// env で spawn され、`route.grant`(anchor)の env は子へ届かない
/// (`FOO` は sub-exec 側値、`KEEP` は anchor のみの宣言のため
/// unset)。deadline も substitute.grant timeout 由来(run.rs unit で
/// pin 済み)。
#[tokio::test(flavor = "current_thread")]
async fn substitute_grant_env_reaches_child_not_anchor() {
    let mut f = Fixture::new();
    let dump = f.ws.path().join("env-dump.txt");
    substitute_fx(
        &f,
        &exec_only_workflow(json!({}), json!({})),
        &format!(
            "      env:\n        CHEAPER_ROUTE_CHILD_MODE: envdump\n        CHEAPER_ROUTE_CHILD_OUT: '{}'\n        FOO: sub-val\n",
            dump.to_string_lossy().replace('\\', "\\\\").replace('\'', "''")
        ),
        "",
        "",
    );
    let token = payload_token(
        &f,
        &format!("routed {CHILD_ARGV}"),
        &[
            "routed",
            "route_run::command_fixture",
            "--exact",
            "--nocapture",
        ],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    assert_eq!(out.exit_code, 0);
    let dumped = std::fs::read_to_string(&dump).unwrap();
    // substitute.grant env が適用され、anchor env は届かない。
    assert!(dumped.contains("FOO=sub-val"), "{dumped}");
    assert!(dumped.contains("KEEP=<unset>"), "{dumped}");
    let record = route_record(&f);
    assert_eq!(record["ok"], json!(true));
    assert_eq!(record["executed"]["grant"], json!("sub-exec"));
}

/// member 併用(#167 AC10)— `grants[]` member job(job_id != "exec")
/// は member 規約で実行でき、`input.name = substitute.grant` の
/// member job も投影経由で到達できる(`grant_set` が substitute
/// 名を含む)。投影外名は従来どおり job 失敗へ畳まれる。
#[tokio::test(flavor = "current_thread")]
async fn substitute_member_jobs_reach_members_and_substitute() {
    let mut f = Fixture::new();
    let fmt_marker = f.ws.path().join("fmt-marker.txt");
    let workflow = exec_only_workflow(
        json!({
            "fmt-job": {
                "kind": "tool",
                "tool": "command",
                "input": {"name": "fmt", "args": ["route_run::command_fixture", "--exact", "--nocapture"]}
            },
            "sub-job": {
                "kind": "tool",
                "tool": "command",
                "input": {"name": "sub-exec", "args": ["route_run::command_fixture", "--exact", "--nocapture"]}
            },
            "undeclared-job": {
                "kind": "tool",
                "tool": "command",
                "input": {"name": "undeclared", "args": []}
            }
        }),
        json!({}),
    );
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    substitute_fx(
        &f,
        &workflow,
        "      env:\n        CHEAPER_ROUTE_CHILD_MODE: ok\n",
        &format!(
            "    fmt:\n      executable: '{exe}'\n      argv: ['route_run::command_fixture', '--exact', '--nocapture']\n      env:\n        CHEAPER_ROUTE_CHILD_MODE: tail\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n    undeclared:\n      executable: '{exe}'\n      argv: ['*']\n",
            fmt_marker
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('\'', "''")
        ),
        "    grants: [fmt]\n",
    );
    let token = payload_token(
        &f,
        "routed anything --else",
        &["routed", "anything", "--else"],
        &[],
        &[],
        None,
    );
    let out = run_route(&mut f, &token).await.unwrap();
    // exec job は substitute ペア(ok mode → exit 0)。
    assert_eq!(out.exit_code, 0);
    // member job は実 spawn(fmt marker)+ substitute.grant 名の
    // member job も member 規約で到達(失敗しない)。
    assert!(fmt_marker.exists(), "fmt member job must run");
    let failures = out.envelope["result"]["postprocessFailures"]
        .as_array()
        .unwrap();
    assert!(
        failures.iter().any(|s| s == "job-failed:undeclared-job"),
        "failures: {failures:?}"
    );
    assert!(
        !failures.iter().any(|s| s == "job-failed:fmt-job"),
        "failures: {failures:?}"
    );
    // `input.name = substitute.grant` の member job — 投影経由で
    // 到達できる(AC gap 項目の pin)。
    assert!(
        !failures.iter().any(|s| s == "job-failed:sub-job"),
        "failures: {failures:?}"
    );
    let record = route_record(&f);
    assert_eq!(record["executed"]["grant"], json!("sub-exec"));
}

/// Option E 境界(#167 AC11)— exec job 不在・`outputs.execResult`
/// 不在の workflow は substitute 宣言の有無に関わらず routed
/// convention 違反で `InvalidRuntimePolicy`(`validate_routed_
/// convention` は不変 — exec-less route は別設計)。
#[tokio::test(flavor = "current_thread")]
async fn substitute_routed_convention_still_enforced() {
    for (case, raw_wf) in [
        (
            "missing-exec-job",
            json!({"schemaVersion": 1, "inputs": {}, "jobs": {}, "outputs": {}}),
        ),
        (
            "missing-execResult",
            json!({
                "schemaVersion": 1,
                "inputs": {},
                "jobs": {"exec": {"kind": "tool", "tool": "command", "input": {}}},
                "outputs": {}
            }),
        ),
    ] {
        let mut f = Fixture::new();
        let marker = f.ws.path().join("sub-marker.txt");
        substitute_fx(
            &f,
            &raw_wf,
            &format!(
                "      env:\n        CHEAPER_ROUTE_CHILD_MODE: io\n        CHEAPER_ROUTE_CHILD_MARKER: '{}'\n",
                marker
                    .to_string_lossy()
                    .replace('\\', "\\\\")
                    .replace('\'', "''")
            ),
            "",
            "",
        );
        let token = payload_token(
            &f,
            &format!("routed {CHILD_ARGV}"),
            &[
                "routed",
                "route_run::command_fixture",
                "--exact",
                "--nocapture",
            ],
            &[],
            &[],
            None,
        );
        let err = run_route(&mut f, &token)
            .await
            .err()
            .unwrap_or_else(|| panic!("{case}: convention violation must reject"));
        assert_eq!(
            err.downcast_ref::<CheaperError>().map(CheaperError::code),
            Some(ErrorCode::InvalidRuntimePolicy),
            "{case}: routed convention は InvalidRuntimePolicy: {err}"
        );
        assert!(!marker.exists(), "{case}: rejected before exec spawn");
    }
}
