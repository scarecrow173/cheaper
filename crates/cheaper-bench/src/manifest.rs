//! manifest v2 serde 型定義 + load + fail-fast validation(design §2.1/§2.6)。
//!
//! `benchmarks/scenarios/*.json` の型表現と domain invariant の強制。
//! `Manifest::parse` は構造 check(`schemaVersion === 2`・`fixtureRevision`
//! 非空)に加えて [`Manifest::validate`] の全 invariant を fail-fast で
//! 検証する — `REQUIRED_SCENARIO_IDS` exact coverage・arm 解決・route
//! block 規則・policy overlay 既知 section・comparisons/armExpect の
//! 語彙制約。

use std::collections::{BTreeMap, BTreeSet};

use cheaper_core::errors::{CheaperError, ErrorCode};
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

use crate::child::{ChildMode, FIXTURE_CHILD_ARGV};
use crate::fixture::bench_error;

/// required scenario 集合(design §2.2 の 12 件 exact coverage —
/// missing/unexpected 双方拒否)。
pub const REQUIRED_SCENARIO_IDS: &[&str] = &[
    "single-simple-tool-call",
    "independent-fanout-worker-fanin",
    "tool-to-worker-chain",
    "process-and-analysis",
    "large-intermediate-process-analysis",
    "dynamic-vs-saved-workflow",
    "definition-vs-compact-interface",
    "route-resolve-and-rewrite",
    "route-routed-exec",
    "route-condense-and-refetch",
    "worker-failure-modes",
    "route-child-nonzero-exit",
];

/// arm 実装語彙 13 種(design §2.3)— `armBase` 解決後の名前はこの
/// 集合に含まれること。失敗系は impl ではなく fixture + expect で
/// 表現するため語彙に含めない。
pub const IMPL_ARMS: &[&str] = &[
    "workflow",
    "dynamic-workflow",
    "saved-workflow",
    "direct-filesystem",
    "direct-worker-tool-call",
    "direct-worker-sequence",
    "full-definition",
    "compact-interface",
    "route-resolve",
    "route-rewrite",
    "route-run",
    "route-logs",
    "direct-child-exec",
];

/// workflow 系 impl(workflow definition が必須の arm class — §2.6
/// 「workflow 系 arm を含む scenario は非空 `workflow.jobs`」)。
const WORKFLOW_IMPLS: &[&str] = &["workflow", "dynamic-workflow", "saved-workflow"];

/// scenario `policy` overlay の既知 top-level section(design §2.6 —
/// `<cfg>/policy.yaml` merge へ流せる policy.yaml 語彙のみ受理)。
/// policy schema の section 一覧(`crates/cheaper-core/src/policy/`
/// の `RuntimePolicy` field 名)に対応する。
const KNOWN_POLICY_SECTIONS: &[&str] = &[
    "workflow",
    "execution",
    "output",
    "retry",
    "tools",
    "filesystem",
    "model",
    "route",
    "security",
    "command",
    "process",
];

/// `armExpect` top-level key(§2.6 の確定集合 — `{envelopeOk,
/// errorCode, exitCode, success}` + `route`/`resolve`/`metrics` block)。
const ARM_EXPECT_TOP_KEYS: &[&str] = &[
    "envelopeOk",
    "errorCode",
    "exitCode",
    "success",
    "route",
    "resolve",
    "metrics",
];

/// `armExpect.route` block の field 語彙(§3.3 route block と同一)。
const ARM_EXPECT_ROUTE_KEYS: &[&str] = &[
    "runId",
    "condenseFired",
    "truncated",
    "rewritten",
    "payloadBytes",
    "logRefetchBytes",
    "originalTokens",
    "returnedTokens",
    "childOutputBytes",
    "returnedBodyBytes",
];

/// `armExpect.resolve` block の field 語彙(route-resolve arm の
/// verdict pin)。
const ARM_EXPECT_RESOLVE_KEYS: &[&str] =
    &["matched", "route", "viability", "executable", "suppressed"];

/// `armExpect.metrics` block および `comparisons.reduction` が参照
/// できる既知 metric/dimension 名(§3.3 record field・metrics・
/// §4.2 delta dim の union — stableValue 対象の同名 key)。
const KNOWN_METRIC_FIELDS: &[&str] = &[
    // §3.3 metrics/comparable dims
    "jobCount",
    "workerJobCount",
    "toolOnlyJobCount",
    "workerTurns",
    "toolCalls",
    "retainedBytes",
    "retainedResultBytes",
    "artifactBytes",
    "materializedPromptBytes",
    "criticalPathMs",
    "maxConcurrency",
    "directToolOperations",
    "finalResultBytes",
    "repairs",
    "transientRetries",
    "postprocessFailures",
    "logRefetches",
    "workerStreamConcurrency",
    "interfaceBytes",
    "definitionReadBytes",
    "nodeCount",
    "toolOperationConcurrency",
    "cliReportedDurationMs",
    "childInvocations",
    // usage 分解(§4.2 delta dim)
    "usage",
    "usageInput",
    "usageOutput",
    "usageCachedInput",
    "usageCachedOutput",
    "usageTotal",
    // §3.3 top-level observation field(§4.2 delta dim)
    "wallTimeMs",
    "exitCode",
    "errorCode",
    "directUsageRecordCount",
    "directInvocationTurnsProxy",
    "workflowDefinitionBytes",
    "parentVisibleInterfaceBytes",
    "interfaceDiscoveryWallTimeMs",
    "definitionGenerationMs",
    // §3.3 route block(stable 値として参照 — §4.2)
    "payloadBytes",
    "condenseFired",
    "truncated",
    "rewritten",
    "runId",
    "logRefetchBytes",
    "originalTokens",
    "returnedTokens",
    "childOutputBytes",
    "returnedBodyBytes",
    // comparisons.reduction 派生
    "parentVisibleReductionBytes",
    "parentVisibleReductionRatio",
    "reductionBytes",
    "reductionRatio",
];

/// manifest v2 root。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// `2` 完全一致(v1 manifest は reject)。
    pub schema_version: u64,
    /// fixture 意味論の revision(`"seeded-v2"` — v1 process fixture
    /// workload → fixture child への切替を version 付きで明示)。
    pub fixture_revision: String,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
}

impl Manifest {
    /// manifest file を load し構造 check + domain validation
    /// ([`Self::validate`])まで行う。
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

    /// manifest JSON 文字列を parse する(構造 check + domain validation
    /// 込み — §2.6 の全 invariant を fail-fast で検証する)。
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
        manifest.validate()?;
        Ok(manifest)
    }

    /// §2.6 domain validation — REQUIRED coverage・arm 解決・route
    /// block・policy overlay・comparisons/armExpect 語彙を fail-fast で
    /// 検証する(最初の違反で error)。
    fn validate(&self) -> Result<(), CheaperError> {
        // scenario id 重複。
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for scenario in &self.scenarios {
            if !seen.insert(scenario.id.as_str()) {
                return Err(bench_error(format!(
                    "duplicate scenario id: {}",
                    scenario.id
                )));
            }
        }
        // REQUIRED exact coverage(missing/unexpected 双方拒否)。
        let required: BTreeSet<&str> = REQUIRED_SCENARIO_IDS.iter().copied().collect();
        let missing: Vec<&str> = required.difference(&seen).copied().collect();
        if !missing.is_empty() {
            return Err(bench_error(format!(
                "missing required scenarios: {}",
                missing.join(", ")
            )));
        }
        let unexpected: Vec<&str> = seen.difference(&required).copied().collect();
        if !unexpected.is_empty() {
            return Err(bench_error(format!(
                "unexpected scenarios (not in required set): {}",
                unexpected.join(", ")
            )));
        }
        for scenario in &self.scenarios {
            scenario.validate()?;
        }
        Ok(())
    }
}

/// scenario 1 件(§2.1)。`id`/`title`/`arms` は必須、他は省略可。
/// 非空値・arm 整合・route 規則等の domain invariant は
/// [`Scenario::validate`](§2.6)が担う。
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
/// `processScript`(node script spawn)の置換 — spawn 先が node
/// から bench bin 自身へ変わった名実一致)。
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

/// 宣言 arm 名 → concrete impl 名(`armBase` 写像、未指定は宣言名その
/// まま — alias 連鎖は写像側が 1 hop 固定なので連鎖しない)。
impl Scenario {
    /// 宣言 arm `arm` の実装名を解決する(`armBase[arm]` があればその値、
    /// 無ければ `arm` 自身 — §2.3)。
    pub fn resolve_arm<'a>(&'a self, arm: &'a str) -> &'a str {
        self.arm_base.get(arm).map(String::as_str).unwrap_or(arm)
    }

    /// §2.6 の scenario 単位 domain 規則(title 非空・arms 宣言・arm
    /// key/value・route・policy・comparisons・armExpect・fakeStream)。
    fn validate(&self) -> Result<(), CheaperError> {
        let ctx = || format!("scenario {}: ", self.id);
        if self.title.trim().is_empty() {
            return Err(bench_error(format!("{}title must not be empty", ctx())));
        }
        if self.arms.is_empty() {
            return Err(bench_error(format!("{}arms must not be empty", ctx())));
        }
        let declared: BTreeSet<&str> = self.arms.iter().map(String::as_str).collect();
        if declared.len() != self.arms.len() {
            return Err(bench_error(format!("{}duplicate declared arm name", ctx())));
        }

        // armBase/armEnv/armExpect/armFakeStream の key は宣言 arm のみ。
        for (keys, name) in [
            (self.arm_base.keys().collect::<Vec<_>>(), "armBase"),
            (self.arm_env.keys().collect::<Vec<_>>(), "armEnv"),
            (self.arm_expect.keys().collect::<Vec<_>>(), "armExpect"),
            (
                self.arm_fake_stream.keys().collect::<Vec<_>>(),
                "armFakeStream",
            ),
        ] {
            for key in keys {
                if !declared.contains(key.as_str()) {
                    return Err(bench_error(format!(
                        "{}{name} key is not a declared arm: {key}",
                        ctx()
                    )));
                }
            }
        }

        // armBase 値は宣言 arm または concrete impl 語彙のみ(alias 先に
        // 宣言 arm を指す写像は 1 hop で impl へ解決するため chain せず、
        // `arm → arm → arm` の連鎖は resolve 結果が宣言 arm 名 = impl 語彙
        // 外であれば reject される)。
        for (arm, base) in &self.arm_base {
            if !declared.contains(base.as_str()) && !IMPL_ARMS.contains(&base.as_str()) {
                return Err(bench_error(format!(
                    "{}armBase[{arm}] resolves to neither a declared arm nor a known impl: {base}",
                    ctx()
                )));
            }
        }
        // 解決後の名前は impl 語彙に含まれること(alias 連鎖 — 宣言 arm
        // 名への写像はその先が再度解決されないため impl 語彙外として
        // ここで reject される)。
        for arm in &self.arms {
            let resolved = self.resolve_arm(arm);
            if !IMPL_ARMS.contains(&resolved) {
                return Err(bench_error(format!(
                    "{}arm {arm} resolves to unknown impl: {resolved} (alias chains are not allowed)",
                    ctx()
                )));
            }
        }

        // workflow 系 arm を含む scenario は非空 workflow.jobs が必須
        // (definition `jobs` は job id → def の object map — array 形
        // ではない。saved-workflow arm は `savedWorkflowFile` 側が
        // definition を持つが、v1 parity で scenario.workflow も要求
        // する — interface discovery の共有 definition)。
        let has_workflow_arm = self
            .arms
            .iter()
            .any(|a| WORKFLOW_IMPLS.contains(&self.resolve_arm(a)));
        if has_workflow_arm {
            let jobs_len = self
                .workflow
                .as_ref()
                .and_then(|w| w.get("jobs"))
                .and_then(Value::as_object)
                .map(serde_json::Map::len)
                .unwrap_or(0);
            if jobs_len == 0 {
                return Err(bench_error(format!(
                    "{}workflow-class arm declared but workflow.jobs is missing or empty",
                    ctx()
                )));
            }
        }
        // saved-workflow arm は savedWorkflowFile が必須。
        if self
            .arms
            .iter()
            .any(|a| self.resolve_arm(a) == "saved-workflow")
            && self
                .saved_workflow_file
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(bench_error(format!(
                "{}saved-workflow arm requires non-empty savedWorkflowFile",
                ctx()
            )));
        }

        self.validate_route()?;
        self.validate_policy_overlay()?;
        self.validate_comparisons(&declared)?;
        self.validate_expect(&declared)?;
        Ok(())
    }

    /// `route` block 規則(§2.6)— route 宣言時は route-* arm が 1 つ
    /// 以上、match.command literal・自 bypass・canonical argv 形・
    /// timeout duration 語彙を強制する。
    fn validate_route(&self) -> Result<(), CheaperError> {
        let ctx = || format!("scenario {}: ", self.id);
        let route_arm_count = self
            .arms
            .iter()
            .filter(|a| self.resolve_arm(a).starts_with("route-"))
            .count();
        let Some(route) = &self.route else {
            if route_arm_count > 0 {
                return Err(bench_error(format!(
                    "{}route-* arm declared without a route block",
                    ctx()
                )));
            }
            return Ok(());
        };
        if route_arm_count == 0 {
            return Err(bench_error(format!(
                "{}route block declared without any route-* arm",
                ctx()
            )));
        }
        // childMode は既知 mode 語彙のみ(sleep/envdump は宣言のみ許容 —
        // 実行は emit/fail のみだが mode 語彙としては受理する)。
        let Some(child_mode) = ChildMode::parse(&route.child_mode) else {
            return Err(bench_error(format!(
                "{}route.childMode is not a known mode: {}",
                ctx(),
                route.child_mode
            )));
        };

        // match.command — literal token prefix のみ(`*` 不可・
        // executable は `cheaper` 自 bypass 不可)。
        if route.route_match.command.is_empty() {
            return Err(bench_error(format!(
                "{}route.match.command must not be empty",
                ctx()
            )));
        }
        for token in &route.route_match.command {
            if token.contains('*') {
                return Err(bench_error(format!(
                    "{}route.match.command token must be literal (no wildcard): {token}",
                    ctx()
                )));
            }
        }
        if route.route_match.command[0].eq_ignore_ascii_case("cheaper") {
            return Err(bench_error(format!(
                "{}route.match.command[0] must not be `cheaper` (self-bypass prohibition)",
                ctx()
            )));
        }

        // route.command — tokenize 後に単一 segment・env-prefix strip
        // 後 argv が match.command を literal prefix として持ち、
        // executable 直後の argv tail がちょうど
        // `["--fixture-child", <childMode>]` であること。
        let argv = single_segment_argv(&route.command).ok_or_else(|| {
            bench_error(format!(
                "{}route.command must tokenize to a single safe segment: {:?}",
                ctx(),
                route.command
            ))
        })?;
        let (env_prefix, argv) = cheaper_cli::route::resolve::split_env_prefix(&argv);
        let _ = env_prefix;
        if argv.is_empty() {
            return Err(bench_error(format!(
                "{}route.command has no argv after env-prefix strip",
                ctx()
            )));
        }
        let match_len = route.route_match.command.len();
        if argv.len() < match_len || argv[..match_len] != route.route_match.command[..] {
            return Err(bench_error(format!(
                "{}route.command argv must have route.match.command as a literal prefix",
                ctx()
            )));
        }
        let expected_tail = [
            FIXTURE_CHILD_ARGV.to_string(),
            child_mode.name().to_string(),
        ];
        if argv[1..] != expected_tail[..] {
            return Err(bench_error(format!(
                "{}route.command argv tail must be exactly [`{FIXTURE_CHILD_ARGV}`, `{}`]",
                ctx(),
                child_mode.name()
            )));
        }

        // timeout — duration 語彙(DURATION_PATTERN)。
        if let Some(timeout) = &route.timeout {
            cheaper_core::duration::parse_duration(timeout).map_err(|_| {
                bench_error(format!(
                    "{}route.timeout is not a valid duration (expected ^[1-9][0-9]*(ms|s|m|h)$): {timeout}",
                    ctx()
                ))
            })?;
        }
        Ok(())
    }

    /// `policy` overlay の top-level key は既知 section のみ(§2.6 —
    /// `<cfg>/policy.yaml` へ merge するため未知 section は typo として
    /// reject)。
    fn validate_policy_overlay(&self) -> Result<(), CheaperError> {
        let ctx = || format!("scenario {}: ", self.id);
        let Some(policy) = &self.policy else {
            return Ok(());
        };
        let Some(obj) = policy.as_object() else {
            return Err(bench_error(format!(
                "{}policy must be an object of section overlays",
                ctx()
            )));
        };
        for section in obj.keys() {
            if !KNOWN_POLICY_SECTIONS.contains(&section.as_str()) {
                return Err(bench_error(format!(
                    "{}policy contains unknown section: {section} (known: {})",
                    ctx(),
                    KNOWN_POLICY_SECTIONS.join(", ")
                )));
            }
        }
        Ok(())
    }

    /// `comparisons` の left/right は宣言 arm、reduction field は既知
    /// metric 名(§2.6)。
    fn validate_comparisons(&self, declared: &BTreeSet<&str>) -> Result<(), CheaperError> {
        let ctx = || format!("scenario {}: ", self.id);
        for comp in &self.comparisons {
            for (side, name) in [(&comp.left, "left"), (&comp.right, "right")] {
                if !declared.contains(side.as_str()) {
                    return Err(bench_error(format!(
                        "{}comparisons.{name} is not a declared arm: {side}",
                        ctx()
                    )));
                }
            }
            if let Some(reduction) = &comp.reduction {
                for (side, field) in [(&reduction.left, "left"), (&reduction.right, "right")] {
                    if !KNOWN_METRIC_FIELDS.contains(&side.as_str()) {
                        return Err(bench_error(format!(
                            "{}comparisons.reduction.{field} is not a known metric field: {side}",
                            ctx()
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// `armExpect` の key/value 語彙(§2.6)— top-level・route・
    /// resolve・metrics の field 名を固定集合へ縛る。
    fn validate_expect(&self, declared: &BTreeSet<&str>) -> Result<(), CheaperError> {
        let ctx = || format!("scenario {}: ", self.id);
        for (arm, expect) in &self.arm_expect {
            if !declared.contains(arm.as_str()) {
                // key の宣言 arm 照合は上位で済 — 到達しないが保険。
                return Err(bench_error(format!(
                    "{}armExpect key is not a declared arm: {arm}",
                    ctx()
                )));
            }
            let Some(obj) = expect.as_object() else {
                return Err(bench_error(format!(
                    "{}armExpect[{arm}] must be an object",
                    ctx()
                )));
            };
            for key in obj.keys() {
                if !ARM_EXPECT_TOP_KEYS.contains(&key.as_str()) {
                    return Err(bench_error(format!(
                        "{}armExpect[{arm}] contains unknown key: {key}",
                        ctx()
                    )));
                }
            }
            let resolved = self.resolve_arm(arm);
            let is_envelope_arm = matches!(
                resolved,
                "workflow"
                    | "dynamic-workflow"
                    | "saved-workflow"
                    | "direct-worker-tool-call"
                    | "direct-worker-sequence"
                    | "compact-interface"
                    | "route-resolve"
                    | "route-logs"
            );
            // envelopeOk/errorCode は envelope arm 以外では null のみ
            // 宣言可(意味を持たない pin を値付きで置かせない)。
            for key in ["envelopeOk", "errorCode"] {
                if let Some(value) = obj.get(key) {
                    if !is_envelope_arm && !value.is_null() {
                        return Err(bench_error(format!(
                            "{}armExpect[{arm}].{key} is only meaningful on envelope arms (use null)",
                            ctx()
                        )));
                    }
                }
            }
            if let Some(route_expect) = obj.get("route") {
                let Some(route_obj) = route_expect.as_object() else {
                    return Err(bench_error(format!(
                        "{}armExpect[{arm}].route must be an object",
                        ctx()
                    )));
                };
                for key in route_obj.keys() {
                    if !ARM_EXPECT_ROUTE_KEYS.contains(&key.as_str()) {
                        return Err(bench_error(format!(
                            "{}armExpect[{arm}].route contains unknown key: {key}",
                            ctx()
                        )));
                    }
                }
            }
            if let Some(resolve_expect) = obj.get("resolve") {
                let Some(resolve_obj) = resolve_expect.as_object() else {
                    return Err(bench_error(format!(
                        "{}armExpect[{arm}].resolve must be an object",
                        ctx()
                    )));
                };
                for key in resolve_obj.keys() {
                    if !ARM_EXPECT_RESOLVE_KEYS.contains(&key.as_str()) {
                        return Err(bench_error(format!(
                            "{}armExpect[{arm}].resolve contains unknown key: {key}",
                            ctx()
                        )));
                    }
                }
            }
            if let Some(metrics_expect) = obj.get("metrics") {
                let Some(metrics_obj) = metrics_expect.as_object() else {
                    return Err(bench_error(format!(
                        "{}armExpect[{arm}].metrics must be an object",
                        ctx()
                    )));
                };
                for key in metrics_obj.keys() {
                    if !KNOWN_METRIC_FIELDS.contains(&key.as_str()) {
                        return Err(bench_error(format!(
                            "{}armExpect[{arm}].metrics contains unknown metric: {key}",
                            ctx()
                        )));
                    }
                }
            }
        }
        Ok(())
    }
}

/// `route.command` を `tokenize_shell` し、単一安全 segment の token
/// text 列を argv として返す(複数 segment・unsafe segment/token・
/// tokenize 失敗は `None` — §2.6 の canonical 形要件)。
fn single_segment_argv(command: &str) -> Option<Vec<String>> {
    let segments = cheaper_cli::hook::routing_policy::tokenize_shell(command)?;
    if segments.len() != 1 {
        return None;
    }
    let segment = &segments[0];
    if segment.unsafe_segment {
        return None;
    }
    let mut argv = Vec::with_capacity(segment.tokens.len());
    for token in &segment.tokens {
        if token.unsafe_token {
            return None;
        }
        argv.push(token.text.clone());
    }
    if argv.is_empty() {
        return None;
    }
    Some(argv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserializes_minimal_v2_shape() {
        // serde 構造層のみの確認(domain validation は
        // `Manifest::parse` → validate が担い required coverage を強制
        // するため "s1" 単独 scenario は `from_str` で deserialize)。
        let text = r#"{
            "schemaVersion": 2,
            "fixtureRevision": "seeded-v2",
            "scenarios": [{"id": "s1", "title": "t", "arms": ["workflow"]}]
        }"#;
        let manifest: Manifest = serde_json::from_str(text).unwrap();
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
