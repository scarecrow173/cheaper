use std::collections::BTreeMap;
use std::path::Path;

use crate::common::{env_lookup, Fixture};
use cheaper_cli::cli::CliIo;
use cheaper_cli::route::payload::{encode, EncodeInput};
use cheaper_cli::route::run::{execute, RouteRunOutput};
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
    execute(token, &mut io, &f.hooks, 0.0).await
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
    assert!(!record.to_string().contains(CHILD_ARGV));
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
    assert!(run_route(&mut f, &token).await.is_err());
    assert!(
        marker.exists(),
        "child was spawned through the grant then killed"
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
