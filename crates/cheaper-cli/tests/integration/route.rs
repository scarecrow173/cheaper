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

/// `grants` member の viability fold(#166)— member 未存在・task 由来
/// fixed member は anchor 失敗と同じ `no-grant` へ畳まれ、全 member
/// eligible+pinnable なら `runnable`(O3a — member 理由を区別する
/// viability 値は増やさない)。
#[tokio::test]
async fn resolve_member_grants_viability() {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let mut f = Fixture::new();
    // closure が `f` 全体を捕捉しないよう cfg path を先に取る。
    let cfg = f.cfg.path().to_path_buf();
    let write_routes = |grants: &str| {
        std::fs::write(
            cfg.join("routes.yaml"),
            format!(
                "version: 1\nroutes:\n  - name: cargo-test\n    match: {{command: [cargo, test]}}\n    target: {{kind: workflow, path: routed-exec.workflow.json}}\n    grant: cargo-test\n    grants: {grants}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            cfg.join("routed-exec.workflow.json"),
            r#"{"version":1,"jobs":{}}"#,
        )
        .unwrap();
    };
    let write_policy = |extra: &str| {
        std::fs::write(
            cfg.join("policy.yaml"),
            format!(
                "version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: \"{exe}\"\n      argv: [test, \"*\"]\n{extra}"
            ),
        )
        .unwrap();
    };

    // 全 member eligible+pinnable → runnable。
    write_routes("[fmt, report]");
    write_policy(&format!(
        "    fmt:\n      executable: \"{exe}\"\n      argv: [\"*\"]\n    report:\n      executable: \"{exe}\"\n      argv: [\"*\"]\n"
    ));
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "cargo test"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["viability"], "runnable");

    // member 未存在 → no-grant(anchor 失敗と同一 fold)。
    write_routes("[missing]");
    write_policy("");
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "cargo test"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["viability"], "no-grant");

    // member が task 由来 fixed grant → no-grant。
    write_routes("[fmt]");
    write_policy(&format!(
        "process:\n  tasks:\n    fmt:\n      command: [\"{exe}\"]\n      inputs: []\n      outputs: []\n"
    ));
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "cargo test"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["viability"], "no-grant");
}

/// quoted metachar は route viability で `unsafe-command` に倒れない
/// (#166 R5 — token 内容の shell-metachar 正規表現分類は除去済み。
/// tokenize_shell は quote 内 metachar を data として扱うため grant
/// argv 照合まで到達する)。
#[tokio::test]
async fn resolve_quoted_metachar_reaches_runnable() {
    let mut f = routed_fixture();
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "cargo test 'a|b'"])
        .await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["viability"], "runnable");
    let (code, env, _err) = f
        .run(&["route", "resolve", "--argv", r#"["cargo","test","a;b"]"#])
        .await;
    assert_eq!(code, 0);
    assert_eq!(env["result"]["viability"], "runnable");
}

/// substitute-exec route mode(#167)— `substitute:{grant,argv}` 宣言
/// route は typed command で発火しつつ実行ペアを substitute 側へ
/// retarget する。原 exec_args `[push,origin,main]` は anchor
/// `cargo-test` の pattern `[test,*]` に一致しないが、substitute
/// ペア `{push-dry,[push,--dry-run]}` が健全なため runnable
/// (viability retarget)。`route resolve` 出力 shape は不変 —
/// `grant` は `route.grant` anchor echo で substitute echo は
/// 載らない(R11)。
#[tokio::test]
async fn resolve_substitute_retargets_viability() {
    let mut f = Fixture::new();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: push-safeguard\n    match: {command: [git, push]}\n    target: {kind: workflow, path: w.json}\n    grant: cargo-test\n    substitute: {grant: push-dry, argv: [push, --dry-run]}\n",
    )
    .unwrap();
    std::fs::write(f.cfg.path().join("w.json"), r#"{"version":1,"jobs":{}}"#).unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    cargo-test:\n      executable: \"{exe}\"\n      argv: [test, \"*\"]\n    push-dry:\n      executable: \"{exe}\"\n      argv: [push, --dry-run]\n"
        ),
    )
    .unwrap();
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "git push origin main"])
        .await;
    assert_eq!(code, 0, "{env}");
    let result = &env["result"];
    assert_eq!(result["matched"], true);
    assert_eq!(result["route"], "push-safeguard");
    // anchor echo — substitute echo は出力 shape に載らない(R11)。
    assert_eq!(result["grant"], "cargo-test");
    assert!(result.get("substitute").is_none(), "{result}");
    // substitute ペアが健全なら runnable — 原 argv の route.grant
    // pattern 照合は行わない。
    assert_eq!(result["viability"], "runnable");
    assert_eq!(result["args"], serde_json::json!(["origin", "main"]));
}

/// project 層 `substitute` 宣言は presence-first で拒否される
/// (#167 O5 — trusted layer 境界。`grants` 層拒否と同じ規約で
/// `InvalidRuntimePolicy` + `schema_error`)。
#[tokio::test]
async fn resolve_project_substitute_rejected() {
    let mut f = Fixture::new();
    let project = f.ws.path().join(".cheaper");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: proj\n    match: {command: [git, push]}\n    target: {kind: workflow, path: w.json}\n    grant: g\n    substitute: {grant: s, argv: [x]}\n",
    )
    .unwrap();
    let (code, env, _err) = f.run(&["route", "resolve", "--command", "git push"]).await;
    assert_eq!(code, 3, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "INVALID_RUNTIME_POLICY");
}

/// project 層 `trusted` key presence は `grants`/`substitute` と同じ
/// 層 gate で fail-loud 拒否される(#168 O2a — presence-first、
/// `InvalidRuntimePolicy` + `schema_error`、file 全体 load 失敗)。
/// `trusted` 非宣言の project route は同一 `match.command` 衝突で
/// 下位層 route に負け、負けた project route 名が `suppressed`
/// provenance へ記録される(spec R4 回帰固定)。
#[tokio::test]
async fn resolve_project_trusted_rejected() {
    let mut f = Fixture::new();
    let project = f.ws.path().join(".cheaper");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: proj\n    match: {command: [git, push]}\n    target: {kind: workflow, path: w.json}\n    grant: g\n    trusted: true\n",
    )
    .unwrap();
    let (code, env, _err) = f.run(&["route", "resolve", "--command", "git push"]).await;
    assert_eq!(code, 3, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "INVALID_RUNTIME_POLICY");

    // `trusted` 非宣言の project route が同一 `match.command` で user
    // 層 route に負け、負けた project route 名が suppressed へ記録
    // される(project 層は下位層を shadow しない — trust = 宣言層
    // provenance)。
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    std::fs::write(
        f.cfg.path().join("routes.yaml"),
        "version: 1\nroutes:\n  - name: user-push\n    match: {command: [git, push]}\n    target: {kind: workflow, path: user-push.workflow.json}\n    grant: push-any\n",
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("user-push.workflow.json"),
        r#"{"version":1,"jobs":{}}"#,
    )
    .unwrap();
    std::fs::write(
        project.join("routes.yaml"),
        "version: 1\nroutes:\n  - name: proj-push\n    match: {command: [git, push]}\n    target: {kind: workflow, path: proj-push.workflow.json}\n    grant: push-any\n",
    )
    .unwrap();
    std::fs::write(
        project.join("proj-push.workflow.json"),
        r#"{"version":1,"jobs":{}}"#,
    )
    .unwrap();
    std::fs::write(
        f.cfg.path().join("policy.yaml"),
        format!(
            "version: 1\ncommand:\n  grants:\n    push-any:\n      executable: \"{exe}\"\n      argv: [\"*\"]\n"
        ),
    )
    .unwrap();
    let (code, env, _err) = f
        .run(&["route", "resolve", "--command", "git push origin main"])
        .await;
    assert_eq!(code, 0, "{env}");
    let result = &env["result"];
    assert_eq!(result["matched"], true);
    // user route が勝ち、負けた project route は suppressed へ記録
    // される(displaced 下位層 route は発生しない — #168)。
    assert_eq!(result["route"], "user-push");
    assert_eq!(result["suppressed"], "proj-push");
    assert_eq!(result["viability"], "runnable");
}
