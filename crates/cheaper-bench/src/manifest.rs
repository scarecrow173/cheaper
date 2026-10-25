//! manifest v2 serde 型定義 + load(design §2.1)。
//!
//! `benchmarks/scenarios/*.json` の型表現。**fail-fast validation 本体は
//! T2**(§2.6 — `REQUIRED_SCENARIO_IDS` exact coverage・arm/route/
//! comparisons の domain 規則・policy overlay 既知 section 制約)。本
//! module が行うのは serde 型への写像と `schemaVersion === 2`・
//! `fixtureRevision` 非空の構造 check 程度まで。

use std::collections::BTreeMap;
use std::path::Path;

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde::Deserialize;
use serde_json::Value;

use crate::fixture::bench_error;

/// manifest v2 root。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// `2` 完全一致(v1 manifest は reject)。
    pub schema_version: u64,
    /// fixture 意味論の revision(`"seeded-v2"` — `{node}` workload →
    /// fixture child への切替を version 付きで明示)。
    pub fixture_revision: String,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
}

impl Manifest {
    /// manifest file を load し構造 check(schemaVersion・fixtureRevision)
    /// まで行う。domain validation は T2 の validation 層。
    pub fn load(path: &Path) -> Result<Self, CheaperError> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            CheaperError::new(
                ErrorCode::InvalidRequest,
                format!("failed to read manifest {}: {e}", path.display()),
            )
        })?;
        Self::parse(&text)
            .map_err(|e| CheaperError::new(e.code(), format!("manifest {}: {e}", path.display())))
    }

    /// manifest JSON 文字列を parse する(構造 check 込み)。
    pub fn parse(text: &str) -> Result<Self, CheaperError> {
        let manifest: Manifest = serde_json::from_str(text)
            .map_err(|e| bench_error(format!("invalid manifest json: {e}")))?;
        if manifest.schema_version != 2 {
            return Err(bench_error(format!(
                "unsupported schemaVersion: {} (expected 2)",
                manifest.schema_version
            )));
        }
        if manifest.fixture_revision.is_empty() {
            return Err(bench_error("fixtureRevision must not be empty"));
        }
        Ok(manifest)
    }
}

/// scenario 1 件(§2.1)。`id`/`title`/`arms` は必須、他は省略可 —
/// 非空値・REQUIRED coverage・arm 整合等の domain 規則は T2
/// validation が担う。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scenario {
    pub id: String,
    pub title: String,
    /// 宣言 arm 名列(実装への写像は `arm_base`)。
    pub arms: Vec<String>,
    /// workspace へ materialize する `path → content`。
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// `<cfg>/policy.yaml` の `workflow.maxJobs` へ。
    #[serde(default)]
    pub workflow_max_jobs: Option<u64>,
    /// `<cfg>/policy.yaml` への section overlay(`execution`/`output`/
    /// `retry` 等の section 部分 object — section merge 規則は §2.4)。
    #[serde(default)]
    pub policy: Option<Value>,
    /// workflow definition(arm `workflow`/`dynamic-workflow` 用)。
    #[serde(default)]
    pub workflow: Option<Value>,
    /// `saved-workflow` arm の source(`benchmarks/scenarios/` 相対)。
    #[serde(default)]
    pub saved_workflow_file: Option<String>,
    /// project 層 `<ws>/.cheaper/policy.yaml` の `process` 節(v1
    /// parity — route grant とは layer/scope が異なり同 file に混ぜない)。
    /// task command の `{fixtureChild}` placeholder は `current_exe()`
    /// abs path へ展開する。
    #[serde(default)]
    pub process_policy: Option<Value>,
    /// Route scenario 宣言(§2.4 materialize)。
    #[serde(default)]
    pub route: Option<RouteSpec>,
    /// `CHEAPER_TEST_FAKE_STREAM` fixture 指定(§2.5)。未指定は
    /// `"auto"` 既定相当。
    #[serde(default)]
    pub fake_stream: Option<FakeStream>,
    /// `arm → impl` 写像(宣言 arm 名・concrete impl 名どちらへの
    /// 写像も許容、alias 連鎖不可 — §2.3)。
    #[serde(default)]
    pub arm_base: BTreeMap<String, String>,
    /// arm 単位の spawn env override。
    #[serde(default)]
    pub arm_env: BTreeMap<String, BTreeMap<String, String>>,
    /// arm 単位の期待失敗形 pin(§2.6 の語彙制約は T2)。
    #[serde(default)]
    pub arm_expect: BTreeMap<String, Value>,
    /// arm 単位の fakeStream 差し替え(失敗 matrix arm 用)。
    #[serde(default)]
    pub arm_fake_stream: BTreeMap<String, FakeStream>,
    /// direct/manual 系 arm の入力(v1 `manual` block + `child`)。
    #[serde(default)]
    pub manual: Option<ManualSpec>,
    #[serde(default)]
    pub comparisons: Vec<ComparisonSpec>,
    /// v1 parity — scenario 級の既定 profile(個別 spawn への
    /// override は `fakeStream`/`manual.responseProfiles`)。
    #[serde(default)]
    pub response_profiles: Option<Vec<String>>,
    #[serde(default)]
    pub response_delay_ms: Option<f64>,
}

/// `fakeStream`/`armFakeStream` 指定(§2.5)。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FakeStream {
    /// `"auto"` — profile 既定値で生成(他 literal は T2 validation が
    /// reject するが、生成側でも `fixture::fixture_document` が
    /// fail-fast する)。
    Literal(String),
    /// `{fixture: {…}}` — RunFixture DSL の部分 object passthrough。
    Fixture { fixture: Value },
    /// `{profiles:[…], delayMs, toolCallPath}` — v1 generator 相当の
    /// profile 駆動 session 生成。
    Profiles(ProfileSpec),
}

/// `{profiles, delayMs, toolCallPath}` 形。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProfileSpec {
    pub profiles: Option<Vec<String>>,
    pub delay_ms: Option<f64>,
    pub tool_call_path: Option<String>,
}

/// `route` block(§2.1/§2.4)。`command` 末尾 token が
/// `--fixture-child <childMode>` であること等の規則検証は T2。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteSpec {
    pub name: String,
    /// `match: {command: [literal token prefix]}`。
    #[serde(rename = "match")]
    pub route_match: RouteMatch,
    /// resolve/hook 入力の原 command — `argv[1..]` が exec_args として
    /// spawn され child dispatch の `argv[1]`/`argv[2]` になるため、
    /// mode はここに書く(grant argv は match pattern で注入器では
    /// ない — §1.4)。
    pub command: String,
    /// `<cfg>/policy.yaml` `command.grants` の key(scope==trusted
    /// でなければ route-eligible でないため user 層へ宣言 — §2.4)。
    pub grant: String,
    /// `command` 末尾 token と一致必須(T2 検証)。
    pub child_mode: String,
    /// grant `env` へ流す `CHEAPER_BENCH_CHILD_BYTES`。
    #[serde(default)]
    pub child_bytes: Option<u64>,
    /// `<cfg>/routes.yaml` `output.tokenThreshold`。
    #[serde(default)]
    pub token_threshold: Option<f64>,
    /// grant `timeout`(`DURATION_PATTERN` — 省略時は command section
    /// default `2m`、workflow deadline は grant timeout + 120s)。
    #[serde(default)]
    pub timeout: Option<String>,
    /// grant の output cap(truncation 計測)。
    #[serde(default)]
    pub max_output_bytes: Option<u64>,
    /// 先行宣言(§1.4 — 12 scenario は `[]` 固定)。
    #[serde(default)]
    pub passthrough_env: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct RouteMatch {
    /// literal token prefix のみ(`*` 不可 — T2 検証)。
    pub command: Vec<String>,
}

/// `manual` block — v1 keys + `child {mode, bytes}`(v1
/// `processScript`(node script + `process.execPath` spawn)の置換 —
/// spawn 先が node から bench bin 自身へ変わった名実一致)。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ManualSpec {
    pub kind: Option<String>,
    /// `direct-filesystem` arm の read 対象(v1 `manual.path`)。
    pub path: Option<String>,
    pub read_path: Option<String>,
    pub read_paths: Vec<String>,
    /// `run bulk-reader` の task 列(spawn i 回目に `tasks[i]`)。
    pub tasks: Vec<String>,
    /// spawn i 回目の profile(`fakeStream` 既定を上書き)。
    pub response_profiles: Option<Vec<String>>,
    pub tool_call_path: Option<String>,
    pub forward_full_results: Option<bool>,
    /// fixture child 直接 spawn(`processScript` の置換)。
    pub child: Option<ManualChildSpec>,
}

/// `manual.child` — `{mode, bytes}`(fixture child spawn 宣言)。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualChildSpec {
    /// child mode 語彙(`emit`/`fail`/`sleep`/`envdump` — §1.4)。
    pub mode: String,
    /// `CHEAPER_BENCH_CHILD_BYTES` override。
    #[serde(default)]
    pub bytes: Option<u64>,
}

/// `comparisons` entry(§4.2 の domain 規則は T2)。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonSpec {
    /// 宣言 arm 名。
    pub left: String,
    pub right: String,
    #[serde(default)]
    pub reduction: Option<ReductionSpec>,
}

/// `reduction` — `{left: <metric>, right: <metric>}`(route byte/token
/// 軸・definition-vs-interface を同じ機構で表現 — §4.2)。
#[derive(Debug, Deserialize)]
pub struct ReductionSpec {
    pub left: String,
    pub right: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_minimal_v2_manifest() {
        let text = r#"{
            "schemaVersion": 2,
            "fixtureRevision": "seeded-v2",
            "scenarios": [{"id": "s1", "title": "t", "arms": ["workflow"]}]
        }"#;
        let manifest = Manifest::parse(text).unwrap();
        assert_eq!(manifest.schema_version, 2);
        assert_eq!(manifest.fixture_revision, "seeded-v2");
        assert_eq!(manifest.scenarios.len(), 1);
        assert!(manifest.scenarios[0].route.is_none());
    }

    #[test]
    fn rejects_v1_schema_version() {
        let text = r#"{"schemaVersion": 1, "fixtureRevision": "seeded-v1", "scenarios": []}"#;
        let e = Manifest::parse(text).unwrap_err();
        assert!(e.message().contains("schemaVersion"));
    }

    #[test]
    fn rejects_empty_fixture_revision() {
        let text = r#"{"schemaVersion": 2, "fixtureRevision": "", "scenarios": []}"#;
        assert!(Manifest::parse(text).is_err());
    }

    #[test]
    fn fake_stream_variants_round_trip() {
        let lit: FakeStream = serde_json::from_value(json!("auto")).unwrap();
        assert!(matches!(lit, FakeStream::Literal(ref s) if s == "auto"));
        let profiles: FakeStream = serde_json::from_value(json!({
            "profiles": ["compact", "large"], "delayMs": 5, "toolCallPath": "a.txt"
        }))
        .unwrap();
        let FakeStream::Profiles(spec) = profiles else {
            panic!("expected Profiles");
        };
        assert_eq!(spec.profiles.unwrap().len(), 2);
        assert_eq!(spec.delay_ms, Some(5.0));
        // `{fixture: …}` object は Profiles ではなく Fixture へ解決する
        // (Profiles の全 key optional と誤判定されない — untagged の
        //  variant 順序が契約)。
        let passthrough: FakeStream = serde_json::from_value(json!({
            "fixture": {"sessions": [{"steps": [{"finish": "done"}]}]}
        }))
        .unwrap();
        assert!(matches!(passthrough, FakeStream::Fixture { .. }));
    }

    #[test]
    fn route_block_deserializes() {
        let scenario: Scenario = serde_json::from_value(json!({
            "id": "route-routed-exec",
            "title": "t",
            "arms": ["route-run"],
            "route": {
                "name": "bench-route",
                "match": {"command": ["benchrun"]},
                "command": "benchrun --fixture-child emit",
                "grant": "bench-child",
                "childMode": "emit",
                "childBytes": 4096,
                "tokenThreshold": 8192,
                "timeout": "30s",
                "maxOutputBytes": 64,
                "passthroughEnv": []
            }
        }))
        .unwrap();
        let route = scenario.route.unwrap();
        assert_eq!(route.route_match.command, vec!["benchrun"]);
        assert_eq!(route.child_mode, "emit");
        assert_eq!(route.child_bytes, Some(4096));
        assert_eq!(route.max_output_bytes, Some(64));
        assert!(route.passthrough_env.is_empty());
    }
}
