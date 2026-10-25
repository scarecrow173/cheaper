//! temp workspace / cfgDir の materialize(design §2.4・Non-Functional
//! Constraints)— `benchmarks/lib/cli.mjs` `prepareWorkspace` の Rust
//! port。
//!
//! - ws・cfgDir は **observation(計測 spawn)毎に新規生成**する粒度 —
//!   scenario/arm/rep 間で使い回さない(usage.jsonl・route-logs・
//!   hook-eval・seam registry 等の全副作用を observation 単位で
//!   リセット。route-logs arm の setup→計測 2 段で同一 cfgDir を共有
//!   するのは T2 の measure 側責務)。
//! - `<cfg>/policy.yaml` は **section merge** 組立て —
//!   `workflowMaxJobs`(`workflow.maxJobs`)・scenario `policy`
//!   overlay・route `command.grants` が同じ file を書きに来るため、
//!   `{version:1, …merge 済み sections…}` の 1 document として
//!   組み立てる(後勝ち上書きではない — A7)。
//! - `processPolicy` は project 層 `<ws>/.cheaper/policy.yaml` の
//!   `process` 節へ(v1 parity — route grant と layer/scope が異なる
//!   ため同 file に混ぜない。同名 grant/task は `build_exec_input`
//!   が InvalidRuntimePolicy で fail-loud)。
//! - file 出力は YAML ではなく JSON を書く — YAML 1.2 は JSON
//!   superset で `parse_yaml` が受理する(v1 `prepareWorkspace` が
//!   `JSON.stringify` 出力を `policy.yaml` へ書いていた規約の継承)。

use std::path::{Path, PathBuf};

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde_json::{json, Map, Value};
use tempfile::TempDir;

use crate::manifest::{RouteSpec, Scenario};

/// `{fixtureChild}` placeholder — `processPolicy.tasks.*.command` 内で
/// `current_exe()` abs path へ展開される token(v1 node process
/// fixture spawn の置換 — spawn 先が node から bench bin 自身へ
/// 変わった名実一致)。
pub const FIXTURE_CHILD_PLACEHOLDER: &str = "{fixtureChild}";

/// `<ws>/.cheaper/models.yaml` — `cheaper-fake/fake-model` を low tier
/// へ override する固定内容(v1 parity)。
const MODELS_YAML: &str = "version: 1\noverrides:\n  cheaper-fake/fake-model: low\n";

/// `<cfg>` の dir 名(v1 `.benchmark-config` parity)。
const CONFIG_DIR_NAME: &str = ".benchmark-config";

/// `CHEAPER_TEST_FAKE_STREAM` が指す生成 fixture file 名。
const FIXTURE_FILE_NAME: &str = "fake-stream-fixture.json";

/// routed-exec workflow の copy 元(cheaper-cli embedded resource と
/// 同じ定義 — `routed-exec.workflow.json` builtin)。crate dir から
/// repo 相対で導く(env!("CARGO_MANIFEST_DIR") 基準)。
const ROUTED_EXEC_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../cheaper-cli/resources/workflows/routed-exec.workflow.json"
);

/// routed-exec copy 先の `<cfg>` 内 file 名(routes.yaml の
/// `target.path` と一致必須 — 層起点相対)。
const ROUTED_EXEC_FILE_NAME: &str = "routed-exec.workflow.json";

/// observation 1 回分の隔離 workspace。`TempDir` drop で ws ごと
/// 掃除される(cfgDir は ws 内 `.benchmark-config`)。
pub struct BenchWorkspace {
    dir: TempDir,
    config_dir: PathBuf,
    fixture_path: PathBuf,
}

impl BenchWorkspace {
    /// workspace root。
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// `CHEAPER_CONFIG_DIR` に渡す user 層 config dir。
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// `CHEAPER_TEST_FAKE_STREAM` に渡す fixture JSON path
    /// (内容は spawn ごとに `fixture::write_fixture` が書き換える)。
    pub fn fixture_path(&self) -> &Path {
        &self.fixture_path
    }
}

/// scenario の workspace/cfgDir を新規生成し files・models.yaml・
/// policy.yaml(section merge)・route materialize を行う。
///
/// `fixture_child_exe` は `{fixtureChild}` placeholder と route grant
/// `executable` に展開される bench bin abs path(caller は
/// `std::env::current_exe()` を渡す — 引数化は unit test が
/// current_exe に依存しないため)。
pub fn prepare(
    scenario: &Scenario,
    fixture_child_exe: &Path,
) -> Result<BenchWorkspace, CheaperError> {
    let dir = TempDir::new().map_err(|e| workspace_error(format!("tempdir create failed: {e}")))?;
    let ws = dir.path();

    // scenario `files` の materialize — workspace escape は拒否
    // (v1 `path.resolve` の lexical normalize + `startsWith` 検査
    // parity。`Path::starts_with` は `..` を normalize しないため
    // component 単位で検査する)。
    for (relative, content) in &scenario.files {
        let Some(rel_path) = workspace_relative(relative) else {
            return Err(workspace_error(format!(
                "fixture path escapes workspace: {relative}"
            )));
        };
        let target = ws.join(rel_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                workspace_error(format!("file dir create failed {}: {e}", parent.display()))
            })?;
        }
        std::fs::write(&target, content)
            .map_err(|e| workspace_error(format!("file write failed {}: {e}", target.display())))?;
    }

    // `.cheaper/models.yaml` — `cheaper-fake/fake-model: low` override。
    let project_config = ws.join(".cheaper");
    std::fs::create_dir_all(&project_config).map_err(|e| {
        workspace_error(format!(
            ".cheaper dir create failed {}: {e}",
            project_config.display()
        ))
    })?;
    std::fs::write(project_config.join("models.yaml"), MODELS_YAML)
        .map_err(|e| workspace_error(format!("models.yaml write failed: {e}")))?;

    // `processPolicy` → project 層 `<ws>/.cheaper/policy.yaml` の
    // `process` 節(`{fixtureChild}` を bench bin abs path へ展開)。
    if let Some(process_policy) = &scenario.process_policy {
        let expanded = expand_fixture_child(process_policy, fixture_child_exe);
        let doc = json!({"version": 1, "process": expanded});
        write_json(
            &project_config.join("policy.yaml"),
            &doc,
            ".cheaper/policy.yaml",
        )?;
    }

    // user 層 cfgDir。
    let config_dir = ws.join(CONFIG_DIR_NAME);
    std::fs::create_dir_all(&config_dir).map_err(|e| {
        workspace_error(format!(
            "cfg dir create failed {}: {e}",
            config_dir.display()
        ))
    })?;

    // `<cfg>/policy.yaml` — section merge 組立て。生成 section が
    // 無ければ file を置かない(absent = policy default)。
    let user_policy = build_user_policy(scenario, fixture_child_exe);
    if user_policy.len() > 1 {
        write_json(
            &config_dir.join("policy.yaml"),
            &Value::Object(user_policy),
            "policy.yaml",
        )?;
    }

    // `route` block の materialize。
    if let Some(route) = &scenario.route {
        let doc = routes_document(route);
        write_json(&config_dir.join("routes.yaml"), &doc, "routes.yaml")?;
        std::fs::copy(ROUTED_EXEC_SOURCE, config_dir.join(ROUTED_EXEC_FILE_NAME)).map_err(|e| {
            workspace_error(format!(
                "routed-exec.workflow.json copy failed ({} → <cfg>): {e}",
                ROUTED_EXEC_SOURCE
            ))
        })?;
    }

    Ok(BenchWorkspace {
        config_dir: config_dir.clone(),
        fixture_path: config_dir.join(FIXTURE_FILE_NAME),
        dir,
    })
}

/// `<cfg>/policy.yaml` の **section merge** 組立て — spawn を伴わない
/// 純粋関数(merge 規則の unit test 対象)。
///
/// `{version:1}` 基底へ `workflowMaxJobs` → `{"workflow":{"maxJobs"}}`、
/// scenario `policy` overlay、route `command.grants` を deep merge する
/// — 同一 section 内の key は共存し、後勝ち上書きで一方が消えない
/// (§2.4/A7)。
pub fn build_user_policy(scenario: &Scenario, fixture_child_exe: &Path) -> Map<String, Value> {
    let mut doc = Map::new();
    doc.insert("version".to_string(), json!(1));
    if let Some(max_jobs) = scenario.workflow_max_jobs {
        merge_sections(
            &mut doc,
            as_overlay(&json!({"workflow": {"maxJobs": max_jobs}})),
        );
    }
    if let Some(Value::Object(overlay)) = &scenario.policy {
        merge_sections(&mut doc, overlay);
    }
    if let Some(route) = &scenario.route {
        let grant = route_grant(route, fixture_child_exe);
        merge_sections(
            &mut doc,
            as_overlay(&json!({"command": {"grants": {route.grant.clone(): grant}}})),
        );
    }
    doc
}

/// `json!` literal を section overlay `Map` として借用する unwrap
/// 省略 helper(生成 fragment は常に object — caller 構造保証)。
fn as_overlay(value: &Value) -> &Map<String, Value> {
    value.as_object().expect("generated overlay is an object")
}

/// object + object の deep merge — section 内 key は共存し、同名 leaf
/// は overlay 側が勝つ(object 同士は再帰 merge)。「後勝ち上書きでは
/// なく section merge」の規則を体現する唯一の場所。
fn merge_sections(base: &mut Map<String, Value>, overlay: &Map<String, Value>) {
    for (key, value) in overlay {
        match (base.get_mut(key), value) {
            (Some(Value::Object(b)), Value::Object(o)) => merge_sections(b, o),
            (Some(slot), v) => *slot = v.clone(),
            (None, v) => {
                base.insert(key.clone(), v.clone());
            }
        }
    }
}

/// route `command.grants.<grant>` entry(§2.4)— `argv` は literal
/// `ArgPattern`(`["--fixture-child", childMode]` で typed command の
/// exec_args と個数・literal 一致)、`env` は spawn env 注入面
/// (`CHEAPER_BENCH_CHILD_BYTES` 等)、`timeout`/`maxOutputBytes` は
/// 宣言時のみ emit。
fn route_grant(route: &RouteSpec, fixture_child_exe: &Path) -> Value {
    let mut grant = Map::new();
    grant.insert(
        "executable".to_string(),
        json!(fixture_child_exe.to_string_lossy()),
    );
    grant.insert(
        "argv".to_string(),
        json!([crate::child::FIXTURE_CHILD_ARGV, route.child_mode]),
    );
    if let Some(bytes) = route.child_bytes {
        grant.insert(
            "env".to_string(),
            json!({crate::child::ENV_CHILD_BYTES: bytes.to_string()}),
        );
    }
    if let Some(max) = route.max_output_bytes {
        grant.insert("maxOutputBytes".to_string(), json!(max));
    }
    if let Some(timeout) = &route.timeout {
        grant.insert("timeout".to_string(), json!(timeout));
    }
    Value::Object(grant)
}

/// `<cfg>/routes.yaml` document — `version:1` + route 1 件
/// (name/match/target routed-exec.workflow.json/grant/
/// output.tokenThreshold/passthroughEnv)。`substitute`/`trusted` は
/// emit しない(typed command に mode を載せる方式 — §1.4/§2.4)。
fn routes_document(route: &RouteSpec) -> Value {
    let mut entry = Map::new();
    entry.insert("name".to_string(), json!(route.name));
    entry.insert(
        "match".to_string(),
        json!({"command": route.route_match.command}),
    );
    entry.insert(
        "target".to_string(),
        json!({"kind": "workflow", "path": ROUTED_EXEC_FILE_NAME}),
    );
    entry.insert("grant".to_string(), json!(route.grant));
    if let Some(threshold) = route.token_threshold {
        entry.insert("output".to_string(), json!({"tokenThreshold": threshold}));
    }
    entry.insert("passthroughEnv".to_string(), json!(route.passthrough_env));
    json!({"version": 1, "routes": [Value::Object(entry)]})
}

/// workspace 相対 path の lexical 検査 + normalize — `..`・absolute・
/// prefix(Windows `C:`)component を含む宣言を `None` で拒否する
/// (file 非存在でも決まる lexical 判定。canonicalize は不要)。
fn workspace_relative(relative: &str) -> Option<PathBuf> {
    use std::path::Component;
    let mut out = PathBuf::new();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            // ParentDir/RootDir/Prefix — 層境界を潜る escape の拒否。
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// `processPolicy` 内の `{fixtureChild}` placeholder を
/// `fixture_child_exe` abs path へ展開する — `process.tasks.*.command`
/// の各 element が placeholder と完全一致した場合のみ置換(v1 node
/// placeholder 完全一致置換と同規約)。command 以外の節はそのまま返す。
fn expand_fixture_child(process_policy: &Value, fixture_child_exe: &Path) -> Value {
    let mut expanded = process_policy.clone();
    let exe = fixture_child_exe.to_string_lossy().into_owned();
    if let Some(tasks) = expanded.get_mut("tasks").and_then(Value::as_object_mut) {
        for task in tasks.values_mut() {
            if let Some(command) = task.get_mut("command").and_then(Value::as_array_mut) {
                for arg in command.iter_mut() {
                    if arg.as_str() == Some(FIXTURE_CHILD_PLACEHOLDER) {
                        *arg = json!(exe);
                    }
                }
            }
        }
    }
    expanded
}

/// JSON document を file へ書く(YAML superset — `parse_yaml` が受理
/// する v1 parity 規約)。
fn write_json(path: &Path, doc: &Value, label: &str) -> Result<(), CheaperError> {
    let mut text = serde_json::to_string_pretty(doc)
        .map_err(|e| workspace_error(format!("{label} serialize failed: {e}")))?;
    text.push('\n');
    std::fs::write(path, &text)
        .map_err(|e| workspace_error(format!("{label} write failed {}: {e}", path.display())))
}

fn workspace_error(message: impl Into<String>) -> CheaperError {
    CheaperError::new(ErrorCode::InvalidRequest, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scenario(patch: Value) -> Scenario {
        let mut base = json!({"id": "s", "title": "t", "arms": ["route-run"]});
        base.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        serde_json::from_value(base).unwrap()
    }

    #[test]
    fn user_policy_sections_merge_and_coexist() {
        // spawn-free 純粋関数テスト — `workflowMaxJobs`・scenario
        // `policy` overlay section・route `command.grants` が同一
        // `<cfg>/policy.yaml` document 上で共存 merge されること
        // (後勝ち上書きで一方が消えない — design §2.4)。
        let exe = Path::new("/bench/cheaper-bench");
        let s = scenario(json!({
            "workflowMaxJobs": 3,
            "policy": {
                "execution": {"timeout": "500ms"},
                "output": {"repairAttempts": 1},
                "command": {"timeout": "90s", "grants": {"preexisting": {"executable": "/x"}}}
            },
            "route": {
                "name": "bench-route",
                "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child emit",
                "grant": "bench-child",
                "childMode": "emit",
                "childBytes": 4096,
                "tokenThreshold": 8192,
                "timeout": "30s",
                "maxOutputBytes": 64
            }
        }));
        let doc = build_user_policy(&s, exe);

        // workflow section — maxJobs が overlay と共存。
        assert_eq!(doc["workflow"]["maxJobs"], json!(3));
        // scenario overlay sections が残る。
        assert_eq!(doc["execution"]["timeout"], json!("500ms"));
        assert_eq!(doc["output"]["repairAttempts"], json!(1));
        // `command` section — overlay の `timeout`/既存 grant と route
        // grant が**同じ `command.grants` 内で共存**する(section
        // merge。後勝ちで grants map ごと消えない)。
        assert_eq!(doc["command"]["timeout"], json!("90s"));
        assert_eq!(
            doc["command"]["grants"]["preexisting"]["executable"],
            json!("/x")
        );
        let grant = &doc["command"]["grants"]["bench-child"];
        assert_eq!(grant["executable"], json!("/bench/cheaper-bench"));
        assert_eq!(grant["argv"], json!(["--fixture-child", "emit"]));
        assert_eq!(grant["env"]["CHEAPER_BENCH_CHILD_BYTES"], json!("4096"));
        assert_eq!(grant["maxOutputBytes"], json!(64));
        assert_eq!(grant["timeout"], json!("30s"));
        assert_eq!(doc["version"], json!(1));
    }

    #[test]
    fn user_policy_minimal_when_nothing_declared() {
        let exe = Path::new("/bench/cheaper-bench");
        let s = scenario(json!({}));
        let doc = build_user_policy(&s, exe);
        // version のみ — file を置かない判定材料(prepare 側)。
        assert_eq!(doc.len(), 1);
    }

    #[test]
    fn route_grant_omits_absent_optional_fields() {
        let exe = Path::new("/bench/cheaper-bench");
        let s = scenario(json!({
            "route": {
                "name": "r", "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child fail",
                "grant": "bench-child", "childMode": "fail"
            }
        }));
        let doc = build_user_policy(&s, exe);
        let grant = &doc["command"]["grants"]["bench-child"];
        assert_eq!(grant["argv"], json!(["--fixture-child", "fail"]));
        // childBytes/maxOutputBytes/timeout 未宣言 → key 自体を emit
        // しない(grant schema は presence 意味論)。
        assert!(grant.get("env").is_none());
        assert!(grant.get("maxOutputBytes").is_none());
        assert!(grant.get("timeout").is_none());
    }

    #[test]
    fn routes_document_shape() {
        let s = scenario(json!({
            "route": {
                "name": "bench-route", "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child emit",
                "grant": "bench-child", "childMode": "emit",
                "tokenThreshold": 1, "passthroughEnv": []
            }
        }));
        let doc = routes_document(s.route.as_ref().unwrap());
        assert_eq!(doc["version"], json!(1));
        let route = &doc["routes"][0];
        assert_eq!(route["name"], json!("bench-route"));
        assert_eq!(route["match"]["command"], json!(["benchrun"]));
        assert_eq!(
            route["target"],
            json!({"kind": "workflow", "path": "routed-exec.workflow.json"})
        );
        assert_eq!(route["grant"], json!("bench-child"));
        assert_eq!(route["output"]["tokenThreshold"], json!(1.0));
        assert_eq!(route["passthroughEnv"], json!(Vec::<String>::new()));
    }

    #[test]
    fn fixture_child_placeholder_expands_in_task_command() {
        let policy = json!({
            "maxConcurrent": 1,
            "tasks": {
                "emit": {
                    "command": ["{fixtureChild}", "--fixture-child", "emit"],
                    "inputs": [], "outputs": []
                }
            }
        });
        let expanded = expand_fixture_child(&policy, Path::new("/bench/cheaper-bench"));
        assert_eq!(
            expanded["tasks"]["emit"]["command"],
            json!(["/bench/cheaper-bench", "--fixture-child", "emit"])
        );
        assert_eq!(expanded["maxConcurrent"], json!(1));
    }

    #[test]
    fn prepare_materializes_files_models_and_policies() {
        let exe = Path::new("/bench/cheaper-bench");
        let s = scenario(json!({
            "files": {"dir/source.txt": "seed\n"},
            "workflowMaxJobs": 3,
            "processPolicy": {
                "tasks": {"emit": {"command": ["{fixtureChild}", "--fixture-child", "emit"],
                                   "inputs": [], "outputs": []}}
            },
            "route": {
                "name": "bench-route", "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child emit",
                "grant": "bench-child", "childMode": "emit", "childBytes": 4096
            }
        }));
        let ws = prepare(&s, exe).unwrap();
        // files
        assert_eq!(
            std::fs::read_to_string(ws.path().join("dir/source.txt")).unwrap(),
            "seed\n"
        );
        // .cheaper/models.yaml
        let models = std::fs::read_to_string(ws.path().join(".cheaper/models.yaml")).unwrap();
        assert!(models.contains("cheaper-fake/fake-model: low"));
        // project 層 policy.yaml — process 節 + placeholder 展開。
        let project_policy: Value = serde_json::from_str(
            &std::fs::read_to_string(ws.path().join(".cheaper/policy.yaml")).unwrap(),
        )
        .unwrap();
        assert_eq!(project_policy["version"], json!(1));
        assert_eq!(
            project_policy["process"]["tasks"]["emit"]["command"][0],
            json!("/bench/cheaper-bench")
        );
        // <cfg>/policy.yaml — section merge 済み 1 document。
        let user_policy: Value = serde_json::from_str(
            &std::fs::read_to_string(ws.config_dir().join("policy.yaml")).unwrap(),
        )
        .unwrap();
        assert_eq!(user_policy["workflow"]["maxJobs"], json!(3));
        assert_eq!(
            user_policy["command"]["grants"]["bench-child"]["argv"],
            json!(["--fixture-child", "emit"])
        );
        // routes.yaml + routed-exec copy。
        let routes: Value = serde_json::from_str(
            &std::fs::read_to_string(ws.config_dir().join("routes.yaml")).unwrap(),
        )
        .unwrap();
        assert_eq!(routes["routes"][0]["name"], json!("bench-route"));
        let copied =
            std::fs::read_to_string(ws.config_dir().join("routed-exec.workflow.json")).unwrap();
        assert!(copied.contains("\"schemaVersion\": 1"));
        assert!(ws.fixture_path().ends_with("fake-stream-fixture.json"));
    }

    #[test]
    fn prepare_rejects_workspace_escape() {
        let exe = Path::new("/bench/cheaper-bench");
        let s = scenario(json!({"files": {"../escape.txt": "x"}}));
        assert!(prepare(&s, exe).is_err());
    }
}
