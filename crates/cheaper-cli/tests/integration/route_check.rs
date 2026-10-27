//! `route check` の read-only diagnostics / envelope / no-mutation
//! contract test。subprocess で production embedded source と stdout
//! envelope を確認する。

use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn run_check(workspace: &Path, config: &Path, temp: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_cheaper"))
        .args(args)
        .current_dir(workspace)
        .env("CHEAPER_CONFIG_DIR", config)
        .env("CHEAPER_LOG", "debug")
        .env("CHEAPER_LOG_FORMAT", "json")
        .env("TEMP", temp)
        .env("TMP", temp)
        .env("TMPDIR", temp)
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn run_check_json(
    workspace: &Path,
    config: &Path,
    temp: &Path,
    args: &[&str],
) -> (i32, Value, String, String) {
    let (code, stdout, stderr) = run_check(workspace, config, temp, args);
    (
        code,
        serde_json::from_str(&stdout).expect("route check stdout must be JSON"),
        stdout,
        stderr,
    )
}

fn yaml_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn write_policy_grant_executable(config: &Path, executable: &str) {
    let executable = yaml_quote(executable);
    std::fs::write(
        config.join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: {executable}\n      argv: ['*']\n"
        ),
    )
    .unwrap();
}

fn write_policy_grant(config: &Path) {
    write_policy_grant_executable(config, env!("CARGO_BIN_EXE_cheaper"));
}

fn builtin_workflow() -> String {
    std::fs::read_to_string(
        crate::common::builtin_dir()
            .join("workflows")
            .join("routed-exec.workflow.json"),
    )
    .unwrap()
}

fn route_by_name<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["routes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|route| route["name"] == name)
        .unwrap_or_else(|| panic!("route {name} must be reported"))
}

#[test]
fn route_check_production_builtin_source_reports_report_without_mutation() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (code, out, _stdout, _stderr) = run_check_json(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--log", "debug", "--log-format", "json"],
    );
    assert_eq!(code, 1);
    assert_eq!(out["ok"], true);
    assert!(out["worker"].is_null());
    let report = &out["result"];
    for key in [
        "schemaVersion",
        "sampleProvided",
        "merge",
        "layers",
        "policy",
        "routes",
        "summary",
    ] {
        assert!(report.get(key).is_some(), "missing report key {key}");
    }
    assert_eq!(report["sampleProvided"], false);
    assert_eq!(report["layers"][0]["layer"], "builtin");
    assert_eq!(report["layers"][0]["sourceKind"], "embedded");
    assert_eq!(report["layers"][0]["status"], "loaded");
    assert_eq!(report["policy"]["status"], "loaded");

    let cargo = route_by_name(report, "cargo-test");
    assert_eq!(cargo["configuration"], "active");
    assert_eq!(cargo["workflow"]["status"], "valid");
    assert_eq!(cargo["grants"]["anchor"]["status"], "missing");
    assert_eq!(cargo["sample"]["status"], "requires-sample-input");
    assert_eq!(cargo["sample"]["matched"], "not-applicable");
    let targets = cargo["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 6);
    let rewrite = |name: &str| {
        targets
            .iter()
            .find(|target| target["target"] == name)
            .unwrap_or_else(|| panic!("missing target {name}"))["rewrite"]
            .as_str()
            .unwrap()
    };
    assert_eq!(rewrite("pretooluse"), "supported");
    assert_eq!(rewrite("devin"), "supported");
    assert_eq!(rewrite("codex"), "supported");
    assert_eq!(rewrite("windsurf"), "unsupported");
    assert_eq!(rewrite("copilot"), "unsupported");
    assert_eq!(rewrite("cursor"), "unsupported");
    for target in targets {
        assert_eq!(target["registration"], "unverified");
        assert_eq!(target["approval"], "unverified");
        assert_eq!(target["execution"], "unverified");
    }
    assert!(cargo["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "route.grant-missing" && d["severity"] == "warning"));

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

#[test]
fn route_check_sample_matches_resolve_viability_without_raw_argv() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    let (code, out, stdout, _stderr) = run_check_json(
        ws.path(),
        config.path(),
        temp.path(),
        &[
            "route",
            "check",
            "--argv",
            r#"["cargo","test","SECRET_VALUE"]"#,
        ],
    );
    assert_eq!(code, 0, "stdout: {stdout}");
    assert_eq!(out["ok"], true);
    let cargo = route_by_name(&out["result"], "cargo-test");
    assert_eq!(cargo["sample"]["status"], "evaluated");
    assert_eq!(cargo["sample"]["matched"], true);
    assert_eq!(cargo["sample"]["selected"], true);
    assert_eq!(cargo["sample"]["viability"], "runnable");
    assert_eq!(cargo["grants"]["anchor"]["status"], "environment-dependent");
    assert!(!stdout.contains("SECRET_VALUE"));
    assert!(!stdout.contains("execArgs"));
}

#[test]
fn route_check_layer_failure_preserves_valid_siblings() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(config.path().join("routes.yaml"), "version: [").unwrap();
    let (code, out, _stdout, _stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    assert_eq!(code, 3);
    assert_eq!(out["ok"], true);
    let report = &out["result"];
    assert_eq!(report["layers"][1]["layer"], "user");
    assert_eq!(report["layers"][1]["status"], "failed");
    assert_eq!(report["layers"][1]["error"]["code"], "route.layer-load");
    assert_eq!(report["layers"][1]["error"]["reason"], "yaml-parse");
    assert_eq!(report["layers"][1]["error"]["source"], "user:routes.yaml");
    let cargo = route_by_name(report, "cargo-test");
    assert_eq!(cargo["configuration"], "indeterminate");
    assert!(cargo["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "route.indeterminate" && d["severity"] == "error"));
}

#[test]
fn route_check_policy_failure_reports_routes_and_unknown_readiness() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(config.path().join("policy.yaml"), "version: [").unwrap();
    let (code, out, stdout, _stderr) = run_check_json(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--argv", r#"["cargo","test"]"#],
    );
    assert_eq!(code, 3);
    assert_eq!(out["ok"], true);
    let report = &out["result"];
    assert_eq!(report["policy"]["status"], "failed");
    assert_eq!(report["policy"]["layers"]["builtin"], "loaded");
    assert_eq!(report["policy"]["layers"]["user"], "failed");
    assert_eq!(report["policy"]["error"]["code"], "route.policy-load");
    assert_eq!(report["policy"]["error"]["reason"], "yaml-parse");
    assert_eq!(report["policy"]["error"]["source"], "user:policy.yaml");
    let cargo = route_by_name(report, "cargo-test");
    assert_eq!(cargo["configuration"], "active");
    assert_eq!(cargo["sample"]["selected"], true);
    assert_eq!(cargo["sample"]["viability"], "unknown");
    assert_eq!(cargo["grants"]["status"], "unknown");
    assert_eq!(cargo["workflow"]["status"], "unknown");
    assert_eq!(cargo["workflow"]["workers"][0]["status"], "valid");
    assert!(!stdout.contains(config.path().to_string_lossy().as_ref()));
}

#[test]
fn route_check_reports_missing_worker_as_route_diagnostic() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    let workflow =
        builtin_workflow().replace("\"worker\": \"condense\"", "\"worker\": \"missing-worker\"");
    std::fs::write(config.path().join("missing-worker.workflow.json"), workflow).unwrap();
    std::fs::write(
        config.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: missing-worker\n    match: {command: [missing]}\n    target: {kind: workflow, path: missing-worker.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();
    let (code, out, _stdout, _stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    assert_eq!(code, 3);
    let route = route_by_name(&out["result"], "missing-worker");
    assert_eq!(route["configuration"], "active");
    assert_eq!(route["workflow"]["status"], "invalid");
    assert!(route["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "route.worker-missing" && d["severity"] == "error"));
}

#[test]
fn route_check_reports_shadowed_and_suppressed_rows() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    let workflow = builtin_workflow();
    std::fs::write(config.path().join("user.workflow.json"), &workflow).unwrap();
    std::fs::write(
        config.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: user-shadow\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: user.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();
    let project_dir = ws.path().join(".cheaper");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(project_dir.join("project.workflow.json"), &workflow).unwrap();
    std::fs::write(
        project_dir.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: project-suppressed\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: project.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();

    let (code, out, _stdout, _stderr) = run_check_json(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--argv", r#"["cargo","test"]"#],
    );
    assert_eq!(code, 1);
    let report = &out["result"];
    assert_eq!(report["merge"]["shadowed"], 1);
    assert_eq!(report["merge"]["suppressed"], 1);
    assert_eq!(
        route_by_name(report, "cargo-test")["configuration"],
        "shadowed"
    );
    assert_eq!(
        route_by_name(report, "cargo-test")["sample"]["matched"],
        true
    );
    assert_eq!(
        route_by_name(report, "cargo-test")["sample"]["selected"],
        false
    );
    assert_eq!(
        route_by_name(report, "user-shadow")["configuration"],
        "active"
    );
    assert_eq!(
        route_by_name(report, "user-shadow")["sample"]["selected"],
        true
    );
    assert_eq!(
        route_by_name(report, "user-shadow")["sample"]["viability"],
        "runnable"
    );
    assert_eq!(
        route_by_name(report, "user-shadow")["sample"]["suppressed"],
        "project-suppressed"
    );
    assert_eq!(
        route_by_name(report, "project-suppressed")["configuration"],
        "suppressed"
    );
    assert_eq!(
        route_by_name(report, "project-suppressed")["sample"]["matched"],
        true
    );
    assert_eq!(
        route_by_name(report, "project-suppressed")["sample"]["selected"],
        false
    );
}

#[test]
fn route_check_reports_invalid_workflow_as_route_diagnostic() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    std::fs::write(config.path().join("invalid.workflow.json"), "{").unwrap();
    std::fs::write(
        config.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: invalid-workflow\n    match: {command: [invalid]}\n    target: {kind: workflow, path: invalid.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();
    let (code, out, stdout, _stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    assert_eq!(code, 3);
    let route = route_by_name(&out["result"], "invalid-workflow");
    assert_eq!(route["workflow"]["status"], "invalid");
    assert_eq!(route["workflow"]["source"], "user:invalid.workflow.json");
    assert!(route["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "route.workflow-invalid" && d["severity"] == "error"));
    assert!(!stdout.contains(config.path().to_string_lossy().as_ref()));
}

#[test]
fn route_check_reports_worker_and_schema_failures_as_route_diagnostics() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    let workers = config.path().join("workers");
    std::fs::create_dir_all(&workers).unwrap();
    std::fs::write(
        workers.join("bad-worker.md"),
        "---\nname: other-worker\ndescription: Invalid worker.\ntools: [read]\ncheaper:\n  model: {tier: low}\n  input: {schema: {inline: {type: object}}}\n  output: {schema: {inline: {type: object}}}\n---\n\nWorker body.\n",
    )
    .unwrap();
    std::fs::write(
        workers.join("schema-worker.md"),
        "---\nname: schema-worker\ndescription: Worker with a missing schema.\ntools: [read]\ncheaper:\n  model: {tier: low}\n  input: {schema: {file: ./schemas/missing.schema.json}}\n  output: {schema: {inline: {type: object}}}\n---\n\nWorker body.\n",
    )
    .unwrap();
    let workflow = builtin_workflow();
    std::fs::write(
        config.path().join("bad-worker.workflow.json"),
        workflow.replace("\"worker\": \"condense\"", "\"worker\": \"bad-worker\""),
    )
    .unwrap();
    std::fs::write(
        config.path().join("schema-worker.workflow.json"),
        workflow.replace("\"worker\": \"condense\"", "\"worker\": \"schema-worker\""),
    )
    .unwrap();
    std::fs::write(
        config.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: invalid-worker\n    match: {command: [invalid-worker]}\n    target: {kind: workflow, path: bad-worker.workflow.json}\n    grant: cargo-test\n  - name: invalid-schema\n    match: {command: [invalid-schema]}\n    target: {kind: workflow, path: schema-worker.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();

    let (code, out, _stdout, _stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    assert_eq!(code, 3);
    let invalid_worker = route_by_name(&out["result"], "invalid-worker");
    assert_eq!(invalid_worker["workflow"]["status"], "invalid");
    assert!(invalid_worker["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "route.worker-invalid" && d["severity"] == "error"));
    let invalid_schema = route_by_name(&out["result"], "invalid-schema");
    assert_eq!(invalid_schema["workflow"]["status"], "invalid");
    assert!(invalid_schema["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "route.worker-schema" && d["severity"] == "error"));
}

#[test]
fn route_check_reports_unresolved_executable_readiness_without_raw_path() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let missing_executable = "CHEAPER_ROUTE_CHECK_MISSING_EXECUTABLE_0123456789";
    write_policy_grant_executable(config.path(), missing_executable);
    let (code, out, stdout, _stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    assert_eq!(code, 1);
    let cargo = route_by_name(&out["result"], "cargo-test");
    assert_eq!(cargo["grants"]["anchor"]["status"], "pin-unresolved");
    assert_eq!(cargo["grants"]["status"], "pin-unresolved");
    assert_eq!(cargo["execReadiness"]["status"], "pin-unresolved");
    assert!(cargo["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["code"] == "route.grant-pin-unresolved" && d["severity"] == "warning"));
    assert!(!stdout.contains(missing_executable));
}

#[test]
fn route_check_reports_project_layer_restrictions_as_layer_diagnostic() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let project_dir = ws.path().join(".cheaper");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(
        project_dir.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: project-grants\n    match: {command: [project]}\n    target: {kind: workflow, path: project.workflow.json}\n    grant: cargo-test\n    grants: [cargo-test]\n",
    )
    .unwrap();
    let (code, out, stdout, _stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    assert_eq!(code, 3);
    let report = &out["result"];
    assert_eq!(report["layers"][2]["layer"], "project");
    assert_eq!(report["layers"][2]["status"], "failed");
    assert_eq!(report["layers"][2]["error"]["code"], "route.layer-load");
    assert_eq!(report["layers"][2]["error"]["reason"], "schema");
    assert_eq!(
        report["layers"][2]["error"]["source"],
        "project:routes.yaml"
    );
    assert_eq!(
        route_by_name(report, "cargo-test")["configuration"],
        "active"
    );
    assert!(!stdout.contains(project_dir.to_string_lossy().as_ref()));
}

#[test]
fn route_check_command_unsafe_sample_stays_informational() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    let (code, out, stdout, _stderr) = run_check_json(
        ws.path(),
        config.path(),
        temp.path(),
        &[
            "route",
            "check",
            "--command",
            "SECRET_TOKEN=abc cargo test && rm -rf /tmp/x",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(out["ok"], true);
    let cargo = route_by_name(&out["result"], "cargo-test");
    assert_eq!(cargo["sample"]["viability"], "unsafe-command");
    assert!(!stdout.contains("SECRET_TOKEN"));
    assert!(!stdout.contains("abc cargo"));
    assert!(!stdout.contains("/tmp/x"));
}

#[test]
fn route_check_input_error_is_sanitized_exit_two() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let missing_cwd = ws.path().join("missing-workspace");
    for args in [
        vec!["route", "check", "--argv", "{not json"],
        vec!["route", "check", "--command", "a", "--argv", "[]"],
        vec!["route", "check", "--cwd", missing_cwd.to_str().unwrap()],
    ] {
        let (code, out, stdout, _stderr) =
            run_check_json(ws.path(), config.path(), temp.path(), &args);
        assert_eq!(code, 2, "{args:?}: {stdout}");
        assert_eq!(out["ok"], false);
        assert!(matches!(
            out["error"]["code"].as_str().unwrap(),
            "INVALID_REQUEST" | "INVALID_INPUT"
        ));
        assert_eq!(out["error"]["details"]["reason"], "invalid-input");
        assert!(!stdout.contains("not json"));
        assert!(!stdout.contains(missing_cwd.to_string_lossy().as_ref()));
    }
}

#[test]
fn route_check_text_renderer_is_dedicated_and_deterministic() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    let (code, stdout, _stderr) = run_check(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--format", "text"],
    );
    assert_eq!(code, 0);
    assert!(stdout.starts_with("route check\n"));
    assert!(stdout.contains("\nlayers:\n"));
    assert!(stdout.contains("\npolicy: loaded\n"));
    assert!(stdout.contains("\nroutes:\n"));
    assert!(stdout.contains("\nsummary: routes="));
    assert!(!stdout.contains("result:"));
    assert!(!stdout.contains("\"routes\""));
    let (repeat_code, repeat_stdout, _repeat_stderr) = run_check(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--format", "text"],
    );
    assert_eq!(repeat_code, code);
    assert_eq!(repeat_stdout, stdout);
}

#[test]
fn route_check_missing_target_sample_uses_error_severity() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    std::fs::write(
        config.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: missing-target\n    match: {command: [missing-target]}\n    target: {kind: workflow, path: absent.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();

    let (code, out, _stdout, _stderr) = run_check_json(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--argv", r#"["missing-target"]"#],
    );
    assert_eq!(code, 3);
    let route = route_by_name(&out["result"], "missing-target");
    assert_eq!(route["sample"]["selected"], true);
    assert_eq!(route["sample"]["viability"], "missing-target");
    let workflow_missing: Vec<_> = route["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["code"] == "route.workflow-missing")
        .collect();
    assert_eq!(workflow_missing.len(), 1);
    assert_eq!(workflow_missing[0]["severity"], "error");
    assert!(
        out["result"]["summary"]["diagnostics"]["error"]
            .as_u64()
            .unwrap()
            >= 1
    );
}

#[test]
fn route_check_invalid_count_counts_invalid_declarations() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    std::fs::write(
        config.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: duplicate-one\n    match: {command: [duplicate]}\n    target: {kind: workflow, path: one.workflow.json}\n    grant: cargo-test\n  - name: duplicate-two\n    match: {command: [duplicate]}\n    target: {kind: workflow, path: two.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();

    let (code, out, _stdout, _stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    assert_eq!(code, 3);
    let report = &out["result"];
    assert_eq!(report["layers"][1]["invalidCount"], 2);
    assert_eq!(report["merge"]["invalid"], 2);
    assert_eq!(
        report["routes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|route| route["configuration"] == "invalid")
            .count(),
        2
    );
}

#[test]
fn route_check_text_renderer_escapes_route_name_control_characters() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    write_policy_grant(config.path());
    std::fs::write(config.path().join("forged.workflow.json"), "{}").unwrap();
    std::fs::write(
        config.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: \"forged\\nname\"\n    match: {command: [forged]}\n    target: {kind: workflow, path: forged.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();

    let (_code, stdout, _stderr) = run_check(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--format", "text"],
    );
    assert!(!stdout.contains("forged\nname"));
    assert!(stdout.contains("forged\\nname"));
}

#[test]
fn route_check_quiet_preserves_stdout_report_and_exit_code() {
    let ws = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let (normal_code, normal, _normal_stdout, _normal_stderr) =
        run_check_json(ws.path(), config.path(), temp.path(), &["route", "check"]);
    let (quiet_code, quiet, quiet_stdout, _quiet_stderr) = run_check_json(
        ws.path(),
        config.path(),
        temp.path(),
        &["route", "check", "--quiet"],
    );
    assert_eq!(quiet_code, normal_code);
    assert_eq!(quiet["result"], normal["result"]);
    assert_eq!(quiet["ok"], true);
    assert_eq!(quiet_stdout, serde_json::to_string(&quiet).unwrap() + "\n");
}
