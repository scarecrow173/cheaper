//! `cheaper route resolve` — dry-run resolve の dispatch 経路 test
//! (B2・route-resolution test spec)。envelope shape(R11)・env-prefix
//! 正規化・`--argv` 入力・no-match 出力を `run_cli` 経由で検証する。

use crate::common;
use common::Fixture;

/// user 層 route + grant を `<cfg>` へ配置する fixture。
/// grant executable は test binary 自身(native exe)を使い、
/// `argv: ["test","*"]` で `cargo test …` の exec_args を受理する。
fn routed_fixture() -> Fixture {
    let f = Fixture::new();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: cargo-test\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: routed-exec.workflow.json}\n    grant: cargo-test\n",
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("routed-exec.workflow.json"),
        r#"{"version":1,"jobs":{}}"#,
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: \"{}\"\n      argv: [test, \"*\"]\n",
            exe.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    )
    .unwrap();
    f
}

#[tokio::test]
async fn resolve_no_match_omits_route_fields() {
    let mut f = Fixture::new();
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "git status"])
        .await;
    assert_eq!(code, 0);
    let result = &env["result"];
    assert_eq!(result["matched"], false);
    // no-match でも正規化 argv/executable は報告する。
    assert_eq!(result["argv"], serde_json::json!(["git", "status"]));
    assert_eq!(result["executable"], "git");
    // R11 — route/target/grant/viability/suppressed は載せない。
    for key in ["route", "target", "grant", "viability", "suppressed"] {
        assert!(result.get(key).is_none(), "{key} should be absent");
    }
}

#[tokio::test]
async fn resolve_matched_runnable_reports_r11_shape() {
    let mut f = routed_fixture();
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "cargo test --workspace"])
        .await;
    assert_eq!(code, 0);
    let result = &env["result"];
    assert_eq!(result["matched"], true);
    assert_eq!(result["route"], "cargo-test");
    assert_eq!(result["grant"], "cargo-test");
    assert_eq!(result["executable"], "cargo");
    assert_eq!(result["viability"], "runnable");
    assert_eq!(
        result["argv"],
        serde_json::json!(["cargo", "test", "--workspace"])
    );
    // args = post-prefix、exec basis(argv[1..])は露出しない。
    assert_eq!(result["args"], serde_json::json!(["--workspace"]));
    let target = result["target"].as_str().unwrap();
    assert!(target.ends_with("routed-exec.workflow.json"), "{target}");
}

#[tokio::test]
async fn resolve_argv_applies_env_prefix_normalization() {
    let mut f = routed_fixture();
    let (code, env, _err) = f
        .run(&[
            "route",
            "resolve",
            "--argv",
            r#"["CI=1","cargo","test","-p","x"]"#,
        ])
        .await;
    assert_eq!(code, 0);
    let result = &env["result"];
    assert_eq!(result["matched"], true);
    // env-prefix は strip されて match/argv に入る。
    assert_eq!(
        result["argv"],
        serde_json::json!(["cargo", "test", "-p", "x"])
    );
    assert_eq!(result["args"], serde_json::json!(["-p", "x"]));
    assert_eq!(result["viability"], "runnable");
}

#[tokio::test]
async fn resolve_no_grant_reports_viability() {
    // grant を policy に宣言しない route — viability: no-grant。
    let mut f = Fixture::new();
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: cargo-test\n    match: {command: [cargo, test]}\n    target: {kind: workflow, path: w.json}\n    grant: absent\n",
    )
    .unwrap();
    std::fs::write(f.cfg.path().join("w.json"), "{}").unwrap();
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "cargo test"])
        .await;
    assert_eq!(code, 0);
    let result = &env["result"];
    assert_eq!(result["matched"], true);
    assert_eq!(result["viability"], "no-grant");
}

#[tokio::test]
async fn resolve_usage_log_not_written() {
    // utility command 規約 — usage.jsonl を記録しない。
    let mut f = Fixture::new();
    let (code, _env, _err) = f
        .run(&["route", "resolve", "--command", "git status"])
        .await;
    assert_eq!(code, 0);
    assert!(f.usage_log().is_none());
}
