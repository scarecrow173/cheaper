---
id: "DESIGN-034KP4lfgfD7DdGiDV1YuT"
type: "design"
status: "approved"
title: "unified exec grant model — command.grants / process.tasks の統合"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-core/src/policy/exec.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/exec/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/exec/prepare.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/mod.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/process.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/schema.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/load.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/capability.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/restrict.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/catalog.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/prepare.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/tests.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_context.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/grant.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/supply.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route_cmd.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/run.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/fixture.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/tests/adapter.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/process.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_process.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_process.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_runtime.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_degrade.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/characterization.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/resource_inference.rs"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/managed-process.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/guides/workflow-usage.md"
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/specs/unified-exec-grant-model.md"
    deleted: []
    renamed: []
    moved:
      - type: "item"
        path: "crates/cheaper-core/src/policy/command.rs::ArgPattern"
        note: "policy/exec.rs へ移設（argv pattern は grant 契約概念）"
      - type: "item"
        path: "crates/cheaper-core/src/policy/process.rs::{bounded_duration,relative,str_vec,ProcessHeartbeat,MAX_*,DEFAULT_*}"
        note: "policy/exec.rs へ移設（section 共有 helper/const）"
      - type: "item"
        path: "crates/cheaper-core/src/policy/process.rs::argv_shell_re"
        note: "policy/exec.rs へ移設（route/resolve.rs の参照先を更新）"
      - type: "item"
        path: "crates/cheaper-tools/src/tools/command/mod.rs::{pin_command_grant(s),verify_pinned_executable,check_command_execution,authorize_args,RouteCommandContext}"
        note: "tools/exec/mod.rs へ移設・汎化"
      - type: "item"
        path: "crates/cheaper-tools/src/tools/process/prepare.rs"
        note: "tools/exec/prepare.rs へ移設・汎化"
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../designs/opt-in-exec-deny-patterns.md"
    - "../designs/exec-contract-content-audit-removal.md"
    - "../designs/policy-gated-command-tool.md"
    - "../designs/managed-process-canonical-tool.md"
    - "../designs/invocation-scoped-command-grants.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/unified-exec-grant-model.md"
    - "../adr/unified-exec-grant-model.md"
    - "../discovery/unified-exec-grant-model-issue-165.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../specs/unified-exec-grant-model.md"
    - "../adr/unified-exec-grant-model.md"
    - "../discovery/unified-exec-grant-model-issue-165.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../specs/opt-in-exec-deny-patterns.md"
    - "../adr/opt-in-exec-deny-patterns.md"
    - "../specs/managed-process-canonical-tool.md"
    - "../specs/policy-gated-command-tool.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
  defers: []
  deferred-by: []
---

# unified exec grant model — command.grants / process.tasks の統合

## Context

issue #165（#151 実装分解 3/7）は、`command.grants` と
`process.tasks` — 「宣言済み executable の許可 + argv 契約
（固定 or pattern）+ policy 管理」の同一概念の二重実装 — を単一の
exec grant model へ統合する。spec（SPEC-034KOpKfZpegX2MLPD0uq6・
proposed）と ADR（ADR-034KOpPArDTHQ8W1c81Q0B・proposed — O1-O10
全採用）で方針は確定済みであり、discovery（DISC-034KNX4OP8iBFRKTREaigz・
resolved — D1-D12 確定 + 実測 inventory + line drift 訂正表）が
移行対象の全接点を file:line 精度で固定している。親 design
（DESIGN-034KuDkWOmNXiBddEQ4XWK）の Option D capability-grant
統合と Merge Fidelity M1-M5 が前提条件。

本 design の責務は「統一型の serde/schema 契約・loader 写像の
具体的な分割（section resolver → `build_exec_input` →
`resolve_exec_policy` の 3 段）・tool surface disjoint の実装形・
pin/verify/prepare の単一 predicate・writeScope/deny/route の
error 境界・全 churn の inventory」を実装契約として固定すること。
引用する file:line は全て branch
`feature/issue-165-unified-exec-grant-model` の現行 code で実測済み
（discovery の drift 訂正表に従い `registry.rs:1150-1159`・
`process.rs:340-363`・`command.rs:419-429` 等の訂正済み値を使用）。

統合の方針（ADR Decision Outcome 確定）:

- **統一対象は resolved model と機構層** — authoring surface
  （policy.yaml の `process`/`command` 節・`policy grant` CLI・
  `--grant`・`tool:"process"` workflow）は不変。
- `process.tasks` は loader が `fixed:true` + `scope` provenance
  付き grant へ写像し、`command.grants` と同じ単一 map へ畳む。
- `command`（`{name,args}` → `!fixed`）が canonical exec surface、
  `process`（`{task}` → `fixed`）は canonical vocabulary に残す
  実装層 compat alias。
- `security.execDenyPatterns` を unified grant 全体へ 3 適用点で
  uniform 適用（#164 が deferred した task 適用を吸収）。
- route 参照は `scope=="trusted" && !fixed` のみ解決、
  `project_grant` 投影は `{route.grant} ∪ {fixed grants}`。

## Scope

### In Scope

- 新 module `crates/cheaper-core/src/policy/exec.rs` — `ExecGrant`・
  `ExecGrantScope`・`ExecDefaults`・`ExecHeartbeat`・
  `ResolvedExecPolicy`・`resolve_exec_policy`・`build_exec_input`・
  `resolve_exec_grant`・`exec_grant_resources`・
  `exec_policy_properties`（exec-shaped input schema）・
  `ArgPattern`/`argv_shell_re`/shared helpers の移設。
- `process`/`command` section resolver の section 型化
  （`ResolvedProcessSection`/`ResolvedCommandSection` — authoring
  境界 validation + defaults 供給の中間段として存続、ADR O9a/R11）。
- `load_policy` への project task 名捕捉 + `normalize_policy` への
  provenance 配管 + `validate_policy_schema` への unified
  collision/deny 接続（同一経路で全入口一致）。
- `RuntimePolicy.exec` 新設（`process`/`command` field 除去）と
  `ToolRuntimeOptions`/`InvocationToolRuntimeOptions`/
  `ToolCtx`/`InvocationInner`/`ToolCatalog`/route view/各
  serialize 配管の `exec` 一本化。
- `crates/cheaper-tools/src/tools/exec/` 新設 — 統一
  `pin_exec_grant(s)`/`verify_pinned_executable`/
  `check_exec_execution`/`prepare_exec_grant`/
  `exec_operation_declaration`/`ExecToolOptions`（現
  `tools/command` の pin 機構 + 現 `tools/process/prepare` を
  汎化移設）。
- `tools/command`・`tools/process` の thin surface 化（schema・
  description・details schema・ProcIdentity を surface 別に保持し、
  内部は unified exec 機構へ接続）。
- writeScope 構築時 gate の surface 別維持 + per-call coverage の
  unified prepare 移設（M4）。
- route 層 — `CommandPolicyView` → `ExecPolicyView` 改名・
  eligibility（`scope=="trusted" && !fixed`）・`project_grant`
  一般化・`policy.exec` 参照への書き換え。
- test・golden fixture・docs の移行（benchmarks は authoring
  surface 不変のため無改訂のまま動く回帰 AC）。

### Out of Scope

- `process`/`command` policy.yaml 節の名称・shape 変更（authoring
  面は不変 — `process.tasks` の `command` 配列・`command.grants`
  の field 集合に新 optional field を**追加**するのみ）。
- `policy grant` CLI・`--grant` flag・route `grant` 用語の改名。
- route grant-set 投影（`grants[]`・issue #166）・substitute exec
  mode（issue 5）・trusted flag 発行主体（issue 6）・viability
  条件付き弱化（issue 4）— 本 design はそれらが乗る単一モデルの
  seam のみ定義。
- `process` tool を `TOOL_ALIASES` へ移す vocabulary alias 化
  （ADR O5c で却下済み — canonical 名維持）。
- tool 数・capability 体系の変更（`process`/`command` の 2 名は
  canonical vocabulary として不変）。
- 文書全体の正直性 sweep（issue #169 の scope — 本 design の
  Docs 更新契約は本 change に必要な範囲に限定）。

## Components and Boundaries

### (a) 統一型 — `crates/cheaper-core/src/policy/exec.rs`（新設）

単一 grant namespace・argv 契約・provenance の owner。`command.rs`
の resolved grant 規約（serde-native・`PartialEq`・source string
保持で invocation parity が効く）を全 grant へ一般化する。

```rust
/// grant provenance — authoring layer 由来の scope。
/// serde は `"trusted" | "project"`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecGrantScope { Trusted, Project }

/// resolved unified grant — serialized shape は parity check の
/// 比較対象（declared-shape equality）。`argv` は `fixed` で意味が
/// 分岐する: fixed → 宣言 argv そのもの（task の command[1..] 等の
/// literal）、非 fixed → argv pattern（省略 = args 個数無制約）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecGrant {
    pub executable: String,
    pub fixed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argv: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<String>>,
    pub cwd: String,
    pub env: Map<String, Value>,
    /// 畳み込み済み per-grant lifecycle（origin section defaults
    /// ?? per-grant override — 畳み込みは `build_exec_input` 経路で
    /// 確定済み。`exit_codes` 未宣言は `[0]`）。
    pub timeout: String,
    pub kill_grace: String,
    pub max_output_bytes: u64,
    pub exit_codes: Vec<i64>,
    pub scope: ExecGrantScope,
}

/// `heartbeat:{interval}`（旧 `ProcessHeartbeat` 改名 — 単一 heartbeat
/// はもはや process 専有ではない）。serde shape `{interval}` 不変。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecHeartbeat { pub interval: String }

/// origin section の lifecycle defaults block — `ResolvedExecPolicy`
/// 上に `command`/`process` の 2 block で保持（R3 — pool の
/// `maxConcurrent` と per-call heartbeat が origin 別）。
/// serialized shape `{maxConcurrent, timeout, maxOutputBytes,
/// killGrace, heartbeat:{interval}}` は現行 2 section と同型。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecDefaults {
    pub max_concurrent: u64,
    pub timeout: String,
    pub max_output_bytes: u64,
    pub kill_grace: String,
    pub heartbeat: ExecHeartbeat,
}

/// 統一 resolved policy — `RuntimePolicy.exec` が保持する唯一の
/// exec 集約。`grants` は name → resolved grant の単一 map
/// （command 由来・task 由来を区別しない — `fixed`/`scope` で
/// 判別）。pin 済み canonical path は載せない（declared-shape
/// parity — `ResolvedCommandPolicy` の規約を継承）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedExecPolicy {
    pub grants: BTreeMap<String, ExecGrant>,
    /// `!fixed` grant の origin defaults（`command` section 由来）。
    pub command: ExecDefaults,
    /// `fixed` grant の origin defaults（`process` section 由来）。
    pub process: ExecDefaults,
}
```

`ResolvedExecPolicy` の method:

```rust
/// route 参照の eligibility — `route.grant`・将来の `grants[]`・
/// `substitute.grant` 全てに同 predicate を適用する単一 seam。
pub fn is_route_eligible(grant: &ExecGrant) -> bool {
    grant.scope == ExecGrantScope::Trusted && !grant.fixed
}

/// route run の単一 grant 投影（O8a — 現 `project_grant`:73-79 の
/// 一般化）。`name` が route-eligible でなければ false。投影は
/// `{name} ∪ {fixed grants}` を残す — routed workflow から task
/// を呼べる現行挙動を保存しつつ、非 fixed の他 grant を隠す C9
/// confused-deputy 防止を維持する。
pub fn project_grant(&mut self, name: &str) -> bool {
    match self.grants.get(name) {
        Some(g) if is_route_eligible(g) => {}
        _ => return false,
    }
    self.grants.retain(|n, g| n == name || g.fixed);
    true
}
```

`RuntimePolicy`（`schema.rs:129-149`）から `process`/`command`
field を除去し、`workflow` の直後に `pub exec: ResolvedExecPolicy`
を置く（serialized 出力の key 順は `workflow` → `exec` → `route`
→ `security` → … — serde 宣言順）。

### (b) resolver 3 段構成 — section → synthesis → unified

ADR O4a/D4 の確定形。責務を「authoring 境界 validation」
「namespace 統合」「exec-shaped 境界 validation」の 3 段に分離し、
全入口（load・`validate_policy_schema`・invocation parity・
`policy grant` write・runtime options）が同一経路を通る。

```rust
/// 段 1 — section resolver（既存・責務据え置き）:
/// `resolve_process_policy(input: Option<&Value>)
///     -> Result<ResolvedProcessSection, CheaperError>`
/// `resolve_command_policy(input: Option<&Value>)
///     -> Result<ResolvedCommandSection, CheaperError>`
///
/// 段 2 — synthesis（新設）:
/// merged authoring 2 section + project 宣言 task 名集合 →
/// exec-shaped doc（`resolve_exec_policy` の入力 shape）へ畳む。
/// `command.grants` × `process.tasks` の同名衝突はここで
/// `InvalidRuntimePolicy` fail-loud（`details.{grant,task}`）。
pub fn build_exec_input(
    process: Option<&Value>,
    command: Option<&Value>,
    project_task_names: &BTreeSet<String>,
) -> Result<Map<String, Value>, CheaperError>;

/// 段 3 — unified resolver（新設）:
/// exec-shaped doc（`{grants:{..}, command:{defaults},
/// process:{defaults}}` — `ResolvedExecPolicy` の serialized
/// shape と同一 input shape）を boundary validate + lifecycle
/// 畳み込み + 適用点 1（declared basename deny）で resolved へ。
/// `None`/`null` は `{}` に collapse — exec 節を持たない入力は
/// empty grants（=deny）へ normalize される（現行規約継承）。
pub fn resolve_exec_policy(
    input: Option<&Value>,
    deny: &ExecDenyList,
) -> Result<ResolvedExecPolicy, CheaperError>;
```

**`build_exec_input` の確定手順**:

1. `resolve_process_policy(process)` → `ResolvedProcessSection
   {defaults, tasks}`（task env/cwd/相対 path 規則・duration bound は
   section resolver の現行検査を維持 — `"invalid process policy:"`
   prefix で operator の authoring 位置を attribution）。
2. `resolve_command_policy(command)` → `ResolvedCommandSection
   {defaults, grants}`（`"invalid command policy:"` 同上。
   **deny 照合は行わない** — uniform 化のため適用点 1 は段 3 に
   一本化する）。
3. `command.grants` keys ∩ `process.tasks` keys が非空 →
   `exec_invalid("grant name collides between command.grants and
   process.tasks")` + `details.{grant,task}`（両方に衝突名を載せ
   宣言位置を指す）。
4. task → exec-doc grant 変換（`{executable: command[0], fixed:true,
   argv: command[1..], inputs, outputs, cwd, env, timeout/killGrace/
   maxOutputBytes: process section resolved defaults, exitCodes:[0],
   scope: names.contains(name) ? "project" : "trusted"}`）—
   **lifecycle は synthesis で明示値として畳み込む**（origin =
   authoring section のため resolved grant に origin field は不要
   — round-trip も冪等）。
5. `command` grant → exec-doc grant 変換（declared fields そのまま +
   `fixed`/`scope`/`inputs`/`outputs` 未宣言は既定値 — per-grant
   lifecycle は section resolver の畳み込み済み値）。
6. `{grants: {synth}, command: {defaults}, process: {defaults}}` を
   返す（defaults block は resolved section defaults の serialize）。

**`resolve_exec_policy` の確定手順**:

1. `exec_input_validator`（exec-shaped schema — 下記 (d)）で検査。
2. `command`/`process` defaults block を `ExecDefaults` へ resolve
   （欠落 block → 既定値 `max_concurrent:1, timeout:"2m",
   max_output_bytes:65_536, kill_grace:"2s", heartbeat:"30s"` —
   `process.rs:53-56` の共有 const を継承）。
3. 各 grant: `executable`（必須・内容不審査）→ **適用点 1**:
   `deny.is_denied(&executable)` →
   `exec_invalid("executable is denied by security.execDenyPatterns")`
   + `details.{grant,executable}`（fixed grant は `details.task` も
   併記 — 宣言位置が `process.tasks` である旨の attribution）。
   `fixed`/`scope` は declared 値 or default（`false`/`trusted`）。
   `argv`: `!fixed` なら `ArgPattern::compile` で pattern 構造検査
   （`fixed` なら literal 扱いで compile しない）。`cwd` は
   `relative()`、`inputs`/`outputs` は宣言時のみ同規則、
   `env` は NUL/Windows-case-dup 規則（`process.rs:127-149`・
   `:255-275` 相当を exec.rs の shared helper で全 grant へ適用）。
   lifecycle: 欠落 field は `fixed` → `process` defaults、
   `!fixed` → `command` defaults で畳み込み（synthesis 経路では
   全値明示のため dormant fallback — 直接 exec-doc 入力向け）。
   `exit_codes` 未宣言は `[0]`。
4. name は `locale_compare` 順 → `BTreeMap`（現行 2 resolver と
   同じ整列規約）。

**round-trip 冪等性**（R5 AC）: serialized `ResolvedExecPolicy` は
exec-doc input schema に適合し、re-resolve で「畳み込み済み
lifecycle・`fixed`/`scope`/`inputs`/`outputs` 明示値 → per-grant
明示値がそのまま採用 → 同一結果」。`options.exec` raw JSON は
`to_value(&policy.exec)` の機械的 serialize のみを受理する契約と
なり、parity check（serialize → 再 resolve → `PartialEq`）が
成立する。

### (c) authoring schema — `command_policy_properties()` の拡張

`command.rs:86-128` の grant spec `properties` に 4 field を追加
（R4 — trusted 層が `fixed`/`inputs`/`outputs`/`scope` を直接
authoring できる経路。`scope:"project"` は自己弱体化として受理）:

```rust
"fixed": {"type": "boolean"},
"inputs": paths,   // process tasks と同じ paths schema（maxItems 256、bounded string）
"outputs": paths,  // 同上
"scope": {"type": "string", "enum": ["trusted", "project"]},
```

**CLI 入口での `fixed`/`scope` 拒否（確定）** — schema はこれらの
field を受理するが、`--grant`（`grant.rs::build_invocation_layer`
:49-104 — spec を `command.grants` へ verbatim 挿入する唯一の
合成点）と `policy grant add`（`policy.rs::parse_grant_spec`
:189-205 — 既存 `name` key reject と同じ CLI 入口 validation）
では `fixed:true`・明示 `scope` を含む spec を `InvalidRequest` で
拒否する。理由: `--grant`/`policy grant` は route-eligible な
`!fixed` command grant を供給する CLI 契約であり、`fixed:true`
（command surface・route 双方に不可視の inert grant）や
`scope:"project"`（route 不可視）は利用者の意図と矛盾し誤解を
招く。schema validate は layer 非依存で CLI 由来 spec を識別
できず、trusted YAML での直接 authoring は合法のままにするため
拒否点は schema ではなく各 CLI 入口に置く。

`process_policy_properties()`（`process.rs:60-96`）は**不変** —
`tasks` の required `command`/`inputs`/`outputs`・shape・name
pattern は authoring 互換の対象外。complete/partial schema の
`process`/`command` 節配置（`schema.rs:310-313`・`:331-335`）と
project partial の `command` 節除外（`:351-366` — A3
invariant）も不変。

### (d) exec-doc input schema — `exec_policy_properties()`

```rust
json!({
    "grants": {
        "type": "object",
        "maxProperties": 256,          // 2 section × 128 の merge 上限
        "propertyNames": {"pattern": "^[a-z][a-z0-9_-]{0,63}$"},
        "additionalProperties": {
            "type": "object",
            "additionalProperties": false,
            "required": ["executable"],
            "properties": {
                "executable": bounded_string,
                "fixed": {"type": "boolean"},
                "argv": {"type": "array", "maxItems": 256, "items": bounded_string},
                "inputs": paths,
                "outputs": paths,
                "cwd": bounded_string,
                "env": {/* grant env schema — command_policy_properties() と同型 */},
                "timeout": {"type": "string", "pattern": DURATION_PATTERN},
                "killGrace": {"type": "string", "pattern": DURATION_PATTERN},
                "maxOutputBytes": {"type": "integer", "minimum": 1,
                                   "maximum": MAX_PROCESS_OUTPUT_BYTES},
                "exitCodes": {"type": "array", "maxItems": 64,
                              "items": {"type": "integer", "minimum": 0,
                                        "maximum": 4294967295u64}},
                "scope": {"type": "string", "enum": ["trusted", "project"]},
            },
        },
    },
    "command": object_schema(&[], exec_defaults_properties()),
    "process": object_schema(&[], exec_defaults_properties()),
})
```

- `additionalProperties: false` で authoring 形（`process.tasks`・
  `command.grants` を内包する section そのもの）の direct 投入を
  fail-loud に拒否 — caller は必ず `build_exec_input` を経由する。
- defaults block は `{"maxConcurrent","timeout","maxOutputBytes",
  "killGrace","heartbeat":{"interval"}}` のみ受理（`grants`/`tasks`
  key は unknown で reject）。実装上の注意:
  `exec_defaults_properties()` の `heartbeat` nested object は
  既存 section schema と同じく
  `{"type":"object","additionalProperties":false,"properties":
  {"interval":…}}` を再現する必要がある（`process.rs:68-72`・
  `command.rs:93-97` の形状 — nested `additionalProperties:false`
  を落とすと `{"heartbeat":{"interval":…,"x":…}}` の unknown key
  混入を受理してしまい round-trip/parity の不変条件を破る）。

### (e) `validate_policy_schema` / `load_policy` 配線

| site | lines（現行） | 変更 |
|------|------------|------|
| `schema.rs` `validate_policy_schema` | :437-446 | `resolve_process_policy`/`resolve_command_policy` の個別再 resolve を `build_exec_input(process, command, &BTreeSet::new())` + `resolve_exec_policy(&exec_doc, &exec_deny)` へ置換。`resolve_exec_deny_list` の無条件 compile（:443）は維持。**collision・declared deny・全 semantic 規則が validate 経路で発火する入口** — `policy grant` の `write_validated`（`cli/policy.rs:254-282`）もこの経路を共有するため add 時点で collision が `InvalidRuntimePolicy` になる |
| `load.rs` `load_policy` | :237-256 | project merge の `validate_policy_schema(ProjectPartial)` 直後・`merge_policy_layers` 直前に `project_task_names` を捕捉（validated project doc の `process.tasks` keys を `BTreeSet` へ）。**merge 前捕捉が必須** — RestrictConcurrency deep-merge で同名 task が project field で override された後では provenance を導出できない |
| `load.rs` `load_policy` | :198-220 | typed re-resolve 段は `resolve_process_policy`/`resolve_command_policy` を serialized section shape で `with_defaults` へ re-embed する現行構造を維持（complete schema の section required key を満たす目的 — 統一後も section 再 resolve は必要）。deny compile は維持するが `resolve_command_policy` への deny 注入は除去（適用点 1 は段 3 へ移動） |
| `load.rs` `load_policy` | :257-258 | `normalize_policy(&completed, &project_task_names)` へ signature 変更 |

`normalize_policy` の signature を
`(raw: &Map<String, Value>, project_task_names: &BTreeSet<String>)
-> Result<RuntimePolicy, CheaperError>` へ変更し、内部で
`build_exec_input(raw.get("process"), raw.get("command"),
project_task_names)?` → `resolve_exec_policy(&exec_doc,
&exec_deny)?` → `RuntimePolicy.exec` へ格納する（`schema.rs:585-590`
の `security` resolve + compile は維持、`process`/`command` の
個別 resolve は除去）。`project_task_names` は layer 非依存
resolver（`validate_policy_schema`）には到達しない — provenance
は load 経路固有の情報であり、validation の成否に影響しない
（scope stamp は normalize 段でのみ意味を持つ）。なお
`scope:"project"` stamp の行動効果は `details.task` attribution
の診断性向上と #166 grant-set 投影 seam の基盤であり、project
scope が何かを構造的に強化する機構ではない — route 参照の拒否は
`is_route_eligible` が別途担う既存 predicate であり、本 mechanism
は route guard ではない（権限を広げる方向には一切働かない）。

**attribution 規約（不変）**: invocation 層の `command.grants` と
merged `process.tasks` の衝突は、段 2 の collision check が
`InvalidRuntimePolicy` で発火するが `details.layer` は付かない —
merged-map 評価の帰属不能は deny の merged 評価（`load.rs:206-212`
コメント）と同じ構造の accepted gap。

### (f) deny uniform 適用 — 3 適用点（D6/O6a）

| 適用点 | site（移行後） | 現行 site | error |
|--------|--------------|-----------|-------|
| 1. declared basename | `exec.rs` `resolve_exec_policy` grant loop | `command.rs:295-312`（grants のみ — tasks は対象外だった） | `exec_invalid("executable is denied by security.execDenyPatterns")` → `InvalidRuntimePolicy` + `details.{grant,executable}`（fixed grant は `details.task` 併記）。**task 由来 executable も対象** — denied project task は policy load fail（意図的強化） |
| 2. resolved basename | `tools/exec/mod.rs` `pin_exec_grant` | `command/mod.rs:190-217` `pin_command_grant`（deny :208-215） | `PolicyViolation` + `details.grant`（`exec grant "{name}": resolved executable is denied by security.execDenyPatterns`） |
| 3. spawn 前再検証 | `tools/exec/mod.rs` `verify_pinned_executable` | `command/mod.rs:228-253` | 同上（pin-equality 直後に同一 predicate を再適用 — uniform 契約維持） |

既定（空 `ExecDenyList`）では全適用点が no-op — #164 後の既定
挙動不変（R10）。deny の layer-agnostic 性も継承: invocation 合成
grant・project task 由来 grant を含む全 grant に merged effective
policy の deny が及び、deny 元 layer との帰属 gap は現行と同じ
accepted limitation。

### (g) runtime 構築 — `registry.rs` `create_tool_runtime`（:1095-）

| 段 | 現行 | 移行後 |
|----|------|--------|
| options field | `process`/`command`/`security` の raw JSON 3 field（:97-108） | `exec`/`security` の 2 field — `exec` は `to_value(&policy.exec)` の機械的 serialize（parity check 入力でもある） |
| resolve | `resolve_process_policy`（:1112）+ `resolve_exec_deny_list`（:1129）+ `resolve_command_policy`（:1131） | `resolve_exec_deny_list`（維持）+ `resolve_exec_policy(options.exec, &exec_deny)` 単一 |
| `Process` content gate | `names` 含 `Process` && `process.tasks.is_empty()` → degrade/`POLICY_VIOLATION`（:1113-1123） | `names` 含 `Process` && `!exec.grants.values().any(|g| g.fixed)` → degrade/`POLICY_VIOLATION "process exposure requires nonempty authorized tasks"`（文言不変） |
| `Command` content gate | `names` 含 `Command` && `command.grants.is_empty()` → degrade/fail（:1133-1149） | `names` 含 `Command` && `!exec.grants.values().any(|g| !g.fixed)` → degrade/`POLICY_VIOLATION "command exposure requires nonempty authorized grants"`（文言不変） |
| writeScope gate | `names` 含 `Command` && grants 非空 && `write_scope` → `POLICY_VIOLATION`（:1150-1159） | `names` 含 `Command` && `!fixed` grant 非空 && `write_scope` → `POLICY_VIOLATION "command is incompatible with worker writeScope"`（同位置・同文言 — M4 維持） |
| pin | `names` 含 `Command` → `pin_command_grants`（:1225-1234） | surface 別 union: `names` 含 `Command` → 全 `!fixed` grant を `pin_exec_grants`、`names` 含 `Process` → 全 `fixed` grant を同関数で pin。**task executable が pin されるのは本 issue が初**（D9 — `prepare.rs:127-136` の lazy resolve は除去）。pin 失敗は構築 `POLICY_VIOLATION` — ただし当該 surface tool が `degradable` 宣言されている場合は当該 surface のみ縮退（下記 trade-off 参照） |
| catalog | `ToolCatalog::from_resolved_policy(process)`（:1238） | `ToolCatalog::from_resolved_policy(exec)` |
| `ToolCtx` | `process`/`command`/`command_pins`/`exec_deny`（:220-243） | `exec: ResolvedExecPolicy`・`exec_pins: BTreeMap<String,String>`・`exec_deny` へ一本化 |

**pin-fail × degradable（確定 — 意図的 semantics 拡張）**: 現行
command は pin 失敗を常に hard fail させる（`:1225-1234` の `?`）。
本 design は surface 対称の uniform 規則とする:
`pin_exec_grants` 失敗時、失敗した grant の surface
（`fixed`/`!fixed`）に対応する tool が `degradable` に含まれれば
その tool のみ `names` から除去（opt-in 縮退の既存機構を再利用）、
含まれなければ `POLICY_VIOLATION` で構築 fail。Command 側は
「degradable 宣言時に pin fail で縮退する」**新規 semantics**
であり現行の無条件 hard fail からの意図的拡張 — spec AC 本文
（当初 `Process` のみ言及）を両 surface 適用へ改訂済み
（`docs/specs/unified-exec-grant-model.md` R7/AC/R11）、破壊的変更
一覧にも登録した（R11-(e)・後述 Non-Functional Constraints）。
opt-in tool の利用不能を fail ではなく縮退で表す既存 degradable
契約の意図と整合する統一であり、非 opt-in 経路の挙動は不変。

### (h) unified tools 層 — `crates/cheaper-tools/src/tools/exec/`（新設）

`tools/command` の pin 機構と `tools/process/prepare` を
unified grant へ汎化して集約する。

```rust
/// unified options — surface 別 field の ownership:
/// `write_scope` は process（fixed）surface のみ、`route_context`
/// は command（!fixed）surface のみが読む。両 surface で共有する
/// 機構 field は 1 本に集約（false abstraction 回避のため 2 struct
/// を維持しない）。
pub struct ExecToolOptions {
    pub root: String,
    pub symlink_policy: SymlinkPolicy,
    pub exec: ResolvedExecPolicy,
    pub pins: BTreeMap<String, String>,
    pub on_timeout: Option<OnTimeout>,
    pub denylist: Option<PathDenylist>,
    pub exec_deny: ExecDenyList,
    pub write_scope: Option<HashSet<String>>,      // fixed surface のみ
    pub route_context: Option<RouteCommandContext>, // !fixed surface のみ
}

/// surface 別の grant 解決 — `fixed` 期待値と `grant.fixed` が
/// 一致しない場合は surface 固有の unknown error を返す
/// （disjoint surface: fixed surface → "unknown or unauthorized
/// process task" + `details.task`、非 fixed surface → "unknown or
/// unauthorized command grant" + `details.grant`）。
pub fn resolve_exec_grant(
    policy: &ResolvedExecPolicy,
    name: &str,
    fixed: bool,
) -> Result<ExecGrant, CheaperError>;

/// grant name → pin 済み canonical path（現 `pin_command_grant`
/// 相当 — `resolve_process_executable` + deny 適用点 2）。
/// `route::resolve` の viability 事前評価も同一 predicate を呼ぶ。
pub fn pin_exec_grant(
    name: &str, grant: &ExecGrant, root: &str,
    symlink_policy: SymlinkPolicy,
    denylist: Option<&PathDenylist>, exec_deny: &ExecDenyList,
) -> Result<String, ToolError>;

/// surface subset への複数 grant 版（registry が `fixed` filter で
/// 呼ぶ — `names` 含 surface 到達可能な grant 群のみ pin）。
pub(crate) fn pin_exec_grants(
    policy: &ResolvedExecPolicy, fixed: bool, root: &str,
    symlink_policy: SymlinkPolicy,
    denylist: Option<&PathDenylist>, exec_deny: &ExecDenyList,
) -> Result<BTreeMap<String, String>, ToolError>;

/// spawn 前再検証（現行同一 — B1/B2 + deny 適用点 3）。
fn verify_pinned_executable(
    name: &str, pinned: &str, exec_deny: &ExecDenyList,
) -> Result<(), ToolError>;

/// `check_command_execution` の一般化（route 共有 seam B#3）—
/// `!fixed` grant 解決 → argv pattern 照合 → cwd（route
/// `exec_cwd` override 込み）→ pin map 検索 + verify。
/// `command::execute` と `route::resolve` が呼ぶ。
pub fn check_exec_execution(
    options: &ExecToolOptions, name: &str, args: &[String],
) -> Result<CheckedExec, ToolError>;

/// unified prepare — fixed grant（process surface）の per-call
/// 前処理: cwd 解決 → （`inputs`/`outputs` 宣言 grant のみ）
/// manifest normalize → declared-path deny → normalized claim
/// deny → writeScope coverage（`write_scope` 存在時、
/// `outputs ⊆ writeScope` でないと `POLICY_VIOLATION`）→
/// pin 検索 + verify → `PreparedExec{executable, cwd}`。
/// 現 `process/prepare.rs:53-138` の責務を grant 化して移設。
pub(super) fn prepare_exec_grant(
    options: &ExecToolOptions, name: &str, grant: &ExecGrant,
) -> Result<PreparedExec, ToolError>;
```

**manifest 規則（M1/R2）**:

```rust
/// `exec_grant_resources` — `inputs`/`outputs` のいずれかを宣言
/// する grant（`Some([])` 空配列も宣言と見なす）は per-path
/// manifest（`process_task_resources`:340-363 同等: reads=inputs・
/// writes=outputs・produces=outputs・`**` 末尾 → Subtree。
/// `Some([])` は「何も claim しない」空 manifest を生成）。
/// `inputs`/`outputs` が両方 `None` の grant は conservative
/// root manifest（`command_grant_resources`:419-429 同等 —
/// 副作用を過大に見積もる fail-safe 側の宣言）。
pub fn exec_grant_resources(
    policy: &ResolvedExecPolicy, name: &str,
) -> Result<ResourceManifestDeclaration, CheaperError>;
```

**宣言 `inputs`/`outputs` は authoritative contract（#163
declared-contracts 哲学）** — `!fixed` grant が `inputs`/`outputs`
を宣言した場合、op declaration と catalog `Command` arm の
resource claim は常時 root manifest から宣言範囲へ narrowing
される。これらの field は operator が署名する「この grant はこの
範囲でのみ使う」という authoritative な宣言契約であり、argv
供給で宣言外 path へ write する分岐を runtime が bound・検査する
機構ではない（内容審査除去後の declared-contract 規約 —
underclaim の責任は宣言者側）。

**writeScope coverage の確定分岐**（M4 — `prepare.rs:96-122` 移設）:
`outputs` の tri-state を `prepare_exec_grant` で以下の通り処理する
（`exec_grant_resources`/`exec_operation_declaration` と同一の
tri-state 規約 — `None` と `Some([])` は意味が異なる）:

- `outputs: Some(paths)`（非空）— manifest normalize +
  `claim_contains` 検査で `outputs ⊆ writeScope` を検査
  （現行同一）。非対応なら `POLICY_VIOLATION`。
- `outputs: Some([])` — 空 per-path manifest = 「副作用を
  宣言しない」契約。coverage は trivially 成立し exec 可
  （writeScope 下で副作用を起こさない grant として受理）。
- `outputs: None`（未宣言）— `options.write_scope` が `Some`
  のとき coverage を証明不能（副作用が resource 層へ不可視 —
  C5）として `POLICY_VIOLATION` で fail-closed。
  `options.write_scope` が `None` なら coverage 検査は不要で
  通常実行（writeScope 制約なし — manifest 側は保守的 root
  manifest を宣言）。

`!fixed` grant は構築時 gate（(g)）で既に writeScope 下に到達
不能のため到達しない（defense として分岐自体は残さない —
unreachable state）。

**op declaration**（registry dispatch `:878-897` の置換）:

```rust
/// ClaimMode: `outputs: Some([])` → Shared、その他 → Exclusive
/// （`None` = undeclared = 未知副作用を保守的に Exclusive —
/// 現行 command の常時 Exclusive と process の outputs-empty
/// 規則を単一規則へ写す、R2 — `Some([])`/`Some(nonempty)`/`None`
/// の tri-state は上記 manifest/writeScope と同一規約）。
/// pool: `fixed` → `{"process", policy.process.max_concurrent}`、
/// `!fixed` → `{"command", policy.command.max_concurrent}`（R3/D11）。
/// `declaration.resources` は全 grant で設定（宣言 inputs/outputs
/// があれば per-path、無ければ root manifest — fixed grant への
/// resources 付与は新規だが、prepare が同じ manifest を claim
/// するため一貫）。
pub fn exec_operation_declaration(
    policy: &ResolvedExecPolicy, name: &str, fixed: bool,
) -> Result<OperationDeclaration, CheaperError>;
```

**lifecycle/pool の意図的非対称（確定）** — `command.grants` で
`fixed:true` を authoring した grant は、per-grant lifecycle
field を **command section defaults** で畳み込む（authoring origin
= `command` — synthesis で `command` resolver の defaults が適用
される）一方、op declaration の pool と heartbeat は `fixed` flag
で **process block** を引く（`{"process", policy.process.
max_concurrent}`）。「lifecycle は authoring 起源の section
defaults、pool/heartbeat は reach surface（`fixed` = process
surface 経由の実行）」という意図的な非対称 — pool は surface
経路の並行性 budget であり lifecycle は宣言位置の既定値なので
分離は一貫する（単一化すると `command` section の heartbeat 設定
が process 到達 grant へ漏れる等、budget 意味が変わる — D11）。

**scheduler 相互作用（実装時検証 flag）** — fixed grant への
per-path `declaration.resources` 付与は新規であり、
`scheduler.rs:227-233`（`mutations == PerResource` かつ
`declaration.resource_scoped || resources.is_some()` のとき
snapshot 宣言へ `ClaimMode::Shared` demotion）との相互作用で、
PerResource mode 下の fixed grant op が従来の `Exclusive` から
snapshot ベース `Shared` へ demote される可能性がある。
conflict/demotion 挙動の変化は実装時に `proc_*`・scheduler test
で確認する（意図しない並行度変化を検出する回帰観点として記録）。

### (i) disjoint surface — `tools/command`・`tools/process` thin 化

| item | `command` surface（canonical） | `process` surface（compat alias） |
|------|------------------------------|----------------------------------|
| input schema | `{name,args}` — `command_schema():51-70` 不変 | `{task}` — `process_schema():51-66` 不変 |
| grant 解決 | `resolve_exec_grant(policy, input.name, fixed:false)` | `resolve_exec_grant(policy, input.task, fixed:true)` |
| 実行 path | `check_exec_execution` → `ProcPrepared{argv: args}` → `proc::run` | `prepare_exec_grant` → `ProcPrepared{argv: grant.argv(fixed literal)}` → `proc::run` |
| details schema | `{grant,argv,state,exitCode,signal,durationMs,stdout,stderr,logRef?,...}` 不変（:90-113） | `{task,state,exitCode,signal,stdout,stderr}` 不変（:83-98） |
| `ProcIdentity` | `{grant,argv}`・`"command {name}"`・`report_duration_ms:true` | `{task}`・`"process {name}"`・`report_duration_ms:false` |
| description | task/grant 名一覧埋め込み（surface subset のみ列挙 — `!fixed` 名のみ） | 同上（`fixed` 名のみ） |
| `accepted_exit_codes` | `grant.exit_codes`（route context で `None` 上書き可能 — 現行不変） | `grant.exit_codes`（task 由来は常に `[0]`） |
| route_context | 注入可（`extra_env`/`exec_cwd`/`accept_any_exit_code`/`log_tee` — 現行不変） | 注入不可（`None` 固定） |
| write_scope | 構築 gate で拒否（到達不能） | per-call coverage（(h) の確定分岐） |

`process` は canonical `ToolName::Process`・`process_schema`・
details schema・`find_cheaper_error`/`OnTimeout`/`MAX_PROCESS_
OUTPUT_BYTES` の re-export path（`process/mod.rs:40-45`）を全て
保持 — `TOOL_ALIASES`（`vocabulary.rs:81-88`）には追加しない
（O5c — worker `tools:`/`tools.allow`/`optionalTools` の literal
照合を維持）。`is_scope_gated_tool`（`vocabulary.rs:99-101`）・
`is_mutating_tool`（`worker/validate.rs:326-331`）・
`check_capabilities`/`normalize_policy_tools` も不変。

### (j) route 層 — `ExecPolicyView` + eligibility + 投影

| site | lines（現行） | 変更 |
|------|------------|------|
| `route/resolve.rs` `CommandPolicyView` | :70-82 | `ExecPolicyView` へ改名、`policy: &ResolvedCommandPolicy` → `policy: &ResolvedExecPolicy`（root/symlink/denylist/`&ExecDenyList` field 不変） |
| `route/resolve.rs` viability | :310-343 | `policy.grants.get(&route.grant)` 後に `is_route_eligible(grant)` を検査 — `scope=="project"`・`fixed`・不在は全て `Viability::NoGrant`（routing≠authorization、D2）。`pin_command_grant`→`pin_exec_grant`、`CommandToolOptions`→`ExecToolOptions`、`check_command_execution`→`check_exec_execution`（`command:` field → `exec:`） |
| `route/supply.rs` `command_view()` | :45-53 | `exec_view()` へ改名、`policy: &self.policy.exec`。`RouteEvalInput.command_policy` field も `exec_policy` へ改名（`hook/routing_policy.rs:140-142` の型のみ追随） |
| `route/run.rs` 投影 | :121-123 | `supply.policy.exec.project_grant(&route.grant)` — `{route.grant} ∪ {fixed grants}` 投影へ一般化（O8a） |
| `route/run.rs` expected input | :176 | **不変** — `{"name": grant, "args": exec_args}`（route 入力 anchor は command surface 契約） |
| `route/run.rs` grant timeout | :356-364 | `policy.exec.grants.get(&route.grant)` へ |
| `hook/routing_policy.rs` | :1467,:2055-2106 | `CommandPolicyView`/`RouteEvalInput` field 名の type 追随 + test literal の `resolve_command_policy` → `build_exec_input`+`resolve_exec_policy` 経由への更新 |

### (k) invocation parity / 配管 — `exec` 一本化

| site | lines（現行） | 変更 |
|------|------------|------|
| `invocation.rs` `InvocationToolRuntimeOptions` | :62-82 | `process`/`command` field → `exec: Option<Value>` 一本（`security` 不変） |
| `invocation.rs` `InvocationInner` | （`inner.process`/`inner.command`/`inner.security`/`inner.exec_deny`） | `inner.exec: ResolvedExecPolicy` + `security`/`exec_deny` 不変 |
| `invocation.rs` parity check | :228-260 | `options.runtime.exec` `Some` → `resolve_exec_policy(Some(raw), &inner.exec_deny)` → `inner.exec` と `PartialEq`、不一致は `invalid_binding("exec policy mismatch")`。caller 供給 exec の declared executable が inner deny に match した error は `InvalidRuntimePolicy`（`security` parity と同じ inner-code 規約）。**3 check（process/command/security）→ 2 check（exec/security）へ集約** |
| `invocation.rs` scope 再注入 | :271-304 | `exec: Some(to_value(&inner.exec))` を runtime options へ再注入（`inner.security` と同型規約） |
| `invocation.rs` `create_invocation_tool_runtime` | :423-436 | `resolve_exec_policy(options.exec.as_ref(), &exec_deny)` 単一 resolve |
| `registry.rs` `ToolRuntimeOptions` | :97-108 | `process`/`command` → `exec: Option<Value>`（`security` 不変） |
| `runtime/run/mod.rs` | :337-344,:496-500 | `process:`/`command:` → `exec: Some(to_value(&policy.exec))` 一本 |
| `cli/workflow_run.rs` | :241-263 | `shared.policy.{process,command,security}` → `exec`+`security` の 2 serialize |
| `cli/workflow_run.rs` | :506-511 | `process:None`/`command:None`/`security:None` → `exec:None`/`security:None`（parity skip + 再注入 — 同型） |
| `cli/workflow_context.rs` | :224,:315-318 | `ToolCatalog::from_resolved_policy(policy.exec.clone())` |
| `catalog.rs` | :8-10,:42-43,:57-76,:363-364,:489-490,:587-597 | `ToolCatalog.policy: ResolvedExecPolicy`。`Process` arm — `input.task` → `resolve_exec_grant(fixed:true)` → `exec_grant_resources`（宣言 inputs/outputs → per-path、無ければ root fallback）。未知名・surface 不一致は `resolve_exec_grant` の `POLICY_VIOLATION` を伝播（現行規約不変 — `tool:"process"` job は inference 側の `named_task` 検査で literal 名を要求済み、`resource_inference.rs:131-138`）。`Command` arm — **`input.name` が literal string で `!fixed` grant へ resolve する場合のみ** `exec_grant_resources`（宣言 inputs/outputs → per-path、無ければ同関数内で root manifest）。非 literal（`{$input}` 等の dynamic binding で非 string）・未知名・`fixed` grant 一致は全て **input 非依存の root manifest fallback** に倒す（現行 `command_grant_resources`:419-429 と同値 — error を伝播しない。根拠は Risks「catalog `Command` arm」参照） |

### (l) signature 契約（変更後）

| function | 変更後 signature | 備考 |
|----------|------------------|------|
| `resolve_process_policy` | `(input: Option<&Value>) -> Result<ResolvedProcessSection, CheaperError>` | `process.rs:188-300`。戻り型を `ResolvedProcessPolicy` → `ResolvedProcessSection` へ改名、`tasks` を `Map<String,Value>` → `BTreeMap<String,ProcessTask>` へ typed 化（synthesis 入力）。semantic 規則（env/cwd/path/duration）は不変 |
| `resolve_command_policy` | `(input: Option<&Value>) -> Result<ResolvedCommandSection, CheaperError>` | `command.rs:239-396`。**`deny` param を除去**（適用点 1 は `resolve_exec_policy` へ一本化）。戻り型を `ResolvedCommandPolicy` → `ResolvedCommandSection` へ改名、`grants` を `BTreeMap<String,ExecGrant>`（`fixed`/`scope`/`inputs`/`outputs` 既定値畳み込み済み）へ |
| `build_exec_input` | `(process, command, project_task_names) -> Result<Map, CheaperError>` | exec.rs 新設（段 2 — collision + provenance + task→grant 変換） |
| `resolve_exec_policy` | `(input: Option<&Value>, deny: &ExecDenyList) -> Result<ResolvedExecPolicy, CheaperError>` | exec.rs 新設（段 3 — exec-doc boundary + deny 適用点 1 + lifecycle fallback） |
| `resolve_exec_grant` | `(policy, name, fixed: bool) -> Result<ExecGrant, CheaperError>` | surface disjoint 解決（surface 別 error 語彙内蔵） |
| `exec_grant_resources` | `(policy, name) -> Result<ResourceManifestDeclaration, CheaperError>` | `process_task_resources`/`command_grant_resources` を置換 |
| `normalize_policy` | `(raw: &Map, project_task_names: &BTreeSet<String>) -> Result<RuntimePolicy, CheaperError>` | `schema.rs:516-639` — provenance param 追加 |
| `pin_exec_grant(s)` | `pin_command_grant(s)` から `&CommandGrant`→`&ExecGrant` へ | `command/mod.rs:168-217` → `tools/exec/mod.rs` |
| `check_exec_execution` | `(options: &ExecToolOptions, name, args) -> Result<CheckedExec, ToolError>` | `check_command_execution`:402-439 の一般化 |
| `exec_operation_declaration` | `(policy, name, fixed) -> Result<OperationDeclaration, CheaperError>` | `process_operation_declaration`:103-118 + `command_operation_declaration`:125-144 を置換 |
| `ResolvedExecPolicy::project_grant` | `(&mut self, name) -> bool` | `project_grant`:73-79 の `{name}∪{fixed}` 一般化 + eligibility 内蔵 |
| `process_tool_definition`/`command_tool_definition` | `(options: ExecToolOptions) -> CanonicalToolDefinition` | options 型を surface 別 struct → `ExecToolOptions` へ |

**除去される型/function**: `ResolvedProcessPolicy`・
`ResolvedCommandPolicy`（top-level resolved model として —
`ResolvedProcessSection`/`ResolvedCommandSection` へ改名）・
`CommandGrant`・`ProcessTask` 由来の `resolve_process_task`・
`process_task_resources`・`resolve_command_grant`・
`command_grant_resources`・`ToolRuntimeOptions.process`/
`.command`・`InvocationToolRuntimeOptions.process`/`.command`・
`ToolCtx.process`/`.command`/`.command_pins`・
`InvocationInner.process`/`.command`・`CommandPolicyView`・
`CommandToolOptions`・`ProcessToolOptions`（全て unified 版へ置換 —
ADR O9a/R11）。`ProcessTask` 自体は `process` section の authoring
decl 型として `process.rs` に残存。

**移設**: `ArgPattern`/`ArgToken`（`command.rs:154-226`）・
`bounded_duration`/`relative`/`str_vec`/`ProcessHeartbeat→ExecHeartbeat`/
`DEFAULT_*`/`MAX_PROCESS_OUTPUT_BYTES`/`MAX_TIMER_MS`/
`argv_shell_re`（`process.rs:120-149`・`:164-183`・`:45-56` 他）→
`policy/exec.rs`。`verify_pinned_executable`/`authorize_args`/
`RouteCommandContext`/`CheckedCommand→CheckedExec`/`grant_violation` →
`tools/exec/mod.rs`。`prepare.rs` → `tools/exec/prepare.rs`。
`MAX_PROCESS_OUTPUT_BYTES` は `tools/process/mod.rs:23` の
re-export path を `pub use cheaper_core::policy::exec::…` 経由で維持。

### (m) call site 配線（全一覧）

| site | lines（現行） | 変更内容 |
|------|------------|----------|
| `schema.rs` | :10-15 | `use policy::exec::{…}` へ import 更新（`ResolvedProcessPolicy`/`ResolvedCommandPolicy` 参照を `exec` 型へ） |
| `schema.rs` | :129-149 | `RuntimePolicy` — `process`/`command` field 除去、`exec: ResolvedExecPolicy` 追加（`workflow` 直後） |
| `schema.rs` | :437-446 | `validate_policy_schema` — unified build+resolve へ置換（(e) 表参照） |
| `schema.rs` | :585-590 | `normalize_policy` — `exec` resolve + struct literal、`project_task_names` param 追加 |
| `load.rs` | :30,:32,:198-220 | `resolve_command_policy`/`resolve_process_policy` は section re-embed 用途で維持（`deny` param 除去の呼出し追随） |
| `load.rs` | :237-258 | `project_task_names` 捕捉 + `normalize_policy` 呼出し追随 |
| `load.rs` | :120-189 | **不変** — invocation guard（`command.grants` key 制約）と同名 collision check は authoring 層規約として存続 |
| `capability.rs` | :86-118 | test literal `RuntimePolicy{…}` — `process:`/`command:` → `exec:` 形状へ |
| `restrict.rs` | :110-150 | 同上（`restrict_policy` 本体は `..policy.clone()` で無変更） |
| `merge.rs` | — | **不変** — merge は authoring 層の raw map 規約（`process.*`/`command.*` の RestrictConcurrency field 一覧は不変） |
| `registry.rs` | :19-21,:57-59,:66-68,:97-108,:220-338,:633-643,:878-897,:1095-1238,:1316-1332 | (g) 表の gate/pin/catalog/`ToolCtx`/options builder/dispatch 置換（`:57-59` command・`:66-68` process の tool import も `tools::exec` 経由へ rename 追随） |
| `invocation.rs` | :11-12,:62-82,:172,:229-304,:423-436 | (k) 表の options/parity/re-inject/resolve 置換 |
| `catalog.rs` | :8-10,:42-43,:57-76,:363-364,:489-490,:587-597 | (k) 表の policy 型・arm 解決・`from_resolved_policy` 置換 |
| `tools/command/mod.rs` | 全 file | thin surface 化 — pin/verify/check/authorize/`RouteCommandContext`/`CheckedExec`/`grant_violation`/`CommandToolOptions` を `tools/exec` へ移設、module は schema/details/description/`command_tool_definition(ExecToolOptions)`/`execute` glue（`check_exec_execution` → `ProcPrepared` → `proc::run` + route context 合成 :516-521）を保持 |
| `tools/process/mod.rs` | 全 file | thin surface 化 — `resolve_process_task` 参照を `resolve_exec_grant(fixed:true)` へ、`prepare` を `prepare_exec_grant` へ、`ProcessToolOptions` を `ExecToolOptions` へ。`process_operation_declaration`→`exec_operation_declaration`。re-export 群（:40-45）は path 維持 |
| `tools/process/prepare.rs` | 全 file | `tools/exec/prepare.rs` へ移設・`prepare_exec_grant` へ汎化（`options.write_scope`/`denylist` は `ExecToolOptions` field 参照） |
| `cli/run.rs` | :178-217 | **不変** — `build_invocation_layer` は `command.grants` authoring layer を合成し続ける（warning 文言も不変 — `--grant` は `!fixed` grant として `command` surface 到達） |
| `cli/grant.rs` | :44-104 | `build_invocation_layer` に grant spec の `fixed`/`scope` key reject を追加（`InvalidRequest` + `details.{layer:"invocation",grant}` — `--grant` は `!fixed`+`scope:"trusted"` 合成に限定。`fixed:true` は `command` surface・route 双方に不可視で `--grant` の意図と矛盾する inert grant、`scope:"project"` も route 不可視で誤解を招くため、CLI 契約として spec 挿入点で拒否）。module doc に unified model 参照を追記 |
| `cli/policy.rs` | :118-172,:189-205,:254-282 | `write_validated` の `validate_policy_schema(Partial)` が unified collision + deny を発火するため `policy grant add` 時点で `command.grants`×`process.tasks` 衝突が `InvalidRuntimePolicy` になる新規挙動（module doc :5-15 の検査列挙へ追記）。併せて `parse_grant_spec`（:189-205 — 既存 `name` key reject と同じ CLI 入口 validation）に `fixed`/`scope` key reject を追加 — `validate_policy_schema` は layer 非依存で `policy grant add` 由来 spec を識別不能（trusted YAML の `fixed:true`/`scope` 宣言は合法 field のため schema 側で拒否不可） |
| `cli/workflow_run.rs` | :241-263,:440-560,:506-511,:753 | (k) 表の exec options 置換。`ToolJobExecutor` の `ToolName::from_canonical("process")`/`tools.allow` admission（:458-476,:604-621）は **不変**（M5 — `tool:"process"` は canonical 名で到達） |
| `cli/workflow_context.rs` | :224,:315-318 | `ToolCatalog::from_resolved_policy(policy.exec.clone())` |
| `route/supply.rs` | :23,:29-53,:92-134 | `ExecPolicyView`/`exec_view()` 改名 + `policy.exec` 参照 |
| `route/resolve.rs` | :10-17,:49-82,:291,:310-343,:357-… | `ExecPolicyView`・`argv_shell_re` import を `policy::exec` へ・eligibility 追加・`pin_exec_grant`/`check_exec_execution`/`ExecToolOptions`・test literal |
| `route/run.rs` | :121-123,:176,:356-364 | `policy.exec` 参照（`expected_input` 不変） |
| `route_cmd.rs` | :96,:101 | `supply.command_view()` → `exec_view()`・`ResolveInput{command_policy}` → `exec_policy`（rename 追随のみ — resolve 呼出し構造は不変） |
| `hook/routing_policy.rs` | :140-142,:1467,:2055-2106 | `RouteEvalInput.exec_policy` field 名・型追随 + test literal |
| `runtime/run/mod.rs` | :335-344,:487-500 | `exec:` options field 一本化（`security` 同型維持） |
| `runtime/dev/fixture.rs` | :58-65,:225-231,:759-817 | `RuntimePolicy` literal `exec:` 形状（`default_policy()` の 2 section literal → `exec` literal） |
| `runtime/run_dev_seam.rs` | :178-219,:1046-1252 | `RuntimePolicy` literal `exec:` + `p.exec.grants.insert(name, fixed-grant)` への test 置換（`p.process.tasks.insert` 経路の置き換え） |
| `workflow/` | `schema.rs`:259-291,`types.rs`:249-303,`compiler.rs`:439,`resource_inference.rs`:123-180,`engine.rs`:374 | **code 不変** — `ports.tools.get("process")`・`item.job.name=="process"`・`ProcessResourceDeclaration`・`known_process`・`tools::process::find_cheaper_error` re-export は全て M5 で維持（回帰確認のみ — `tools.get("process").resources({"task":…})` が unified catalog 経由で動く） |
| `rig-adapter` | `tools.rs`:58,`tests/adapter.rs`:319-320,:599-642 | `tools::process::find_cheaper_error` re-export で import 不変。test literal の `ToolRuntimeOptions{process:None,command:None}` → `exec:None` |
| `benchmarks` | `scenarios/scenarios.json`:190-305,`lib/cli.mjs`:58-70 | **不変** — `processPolicy`（project `process.tasks` authoring）+ `tool:"process"` + `input.task:"emit"` は M5 で維持（無改訂で動く回帰 AC） |

### (n) error 契約

| 条件 | code | message / details | boundary |
|------|------|-------------------|----------|
| `command.grants`×`process.tasks` 同名 | `InvalidRuntimePolicy` | `"invalid exec policy: grant name collides between command.grants and process.tasks"` + `details.{grant,task}`（両方に衝突名） | `build_exec_input` — `validate_policy_schema`（全 mode）・`load_policy` normalize・`policy grant add` `write_validated` で同一 failure |
| project task 名が trusted task 名を shadow | —（error なし） | 従来 deep-merge 規約（merged spec → `scope:"project"`） | `merge_policy_layers` + 段 2 scope stamp |
| route grant 参照が `scope=="project"`/`fixed`/不在 | `Viability::NoGrant`（enum — error ではない）→ route run では `denied("no-grant")` | eligibility predicate 不成立 | `route/resolve.rs` precheck + `project_grant` 内の eligibility 二重防御 |
| `command` surface が `fixed` grant 名を指定 | `PolicyViolation` | `"unknown or unauthorized command grant"` + `details.grant` | `resolve_exec_grant(fixed:false)` — tool execute・`check_exec_execution`（catalog `Command` arm は root manifest fallback で error を伝播しない — §(k)） |
| `process` surface が `!fixed` grant 名を指定 | `PolicyViolation` | `"unknown or unauthorized process task"` + `details.task` | `resolve_exec_grant(fixed:true)` — tool execute・catalog `Process` arm resources（現行規約不変） |
| `process` surface の extra key（`args` 等） | schema reject | `process_schema` `additionalProperties:false`（不変 — M2 「`{name}` のみ」は input schema で強制） | catalog `validateToolInput` |
| declared deny（全 grant 一律 — task 由来含む） | `InvalidRuntimePolicy` | `"invalid exec policy: executable is denied by security.execDenyPatterns"` + `details.{grant,executable}`（fixed は `details.task` 併記） | `resolve_exec_policy` 適用点 1 — load/`validate_policy_schema`/`policy grant add` 全入口 |
| resolved/spawn deny | `PolicyViolation` | `exec grant "{name}": resolved executable is denied by security.execDenyPatterns` + `details.grant` | `pin_exec_grant`（適用点 2）/`verify_pinned_executable`（適用点 3） |
| pin 不能（全 grant — task 由来は初） | `PolicyViolation` | `exec grant "{name}": {reason}` + `details.grant`（degradable 宣言時は surface 縮退） | scope 構築 `pin_exec_grants` |
| writeScope × `!fixed` | `PolicyViolation` | `"command is incompatible with worker writeScope"`（不変） | scope 構築 gate |
| writeScope coverage 超過 | `PolicyViolation` | `"process outputs exceed worker writeScope"`（不変 — fixed grant の `outputs ⊆ writeScope` 不成立または `outputs` 未宣言） | unified prepare per-call |
| content gate（surface 到達可能 grant 0 件） | `PolicyViolation` | `"process exposure requires nonempty authorized tasks"` / `"command exposure requires nonempty authorized grants"`（文言不変） | scope 構築 gate（degradable 時は縮退） |
| exec-doc 形状不正（unknown key・`tasks`/`grants` を defaults block へ混入等） | `InvalidRuntimePolicy` | `"invalid exec policy: missing, unknown or invalid config field"` | `resolve_exec_policy` validator — runtime options/`options.exec` 境界 |
| invocation parity mismatch | `InvalidRequest` | `invalid_binding("exec policy mismatch")` + `details.reason` | `create_worker_scope` parity check |
| `tool:"process"` 経路の tool 解決 | —（UnknownTool 化しない） | `ToolName::from_canonical("process")`・`tools.allow` admission・`ports.tools.get("process")` は canonical 名で到達（M5） | workflow schema/executor/catalog 全経路 |
| unknown tool（`tool:"xyz"`） | `UnknownTool`/`PolicyViolation` | 現行 `"unknown tool: {name}"`/`"tool not allowed"` 不変 | workflow schema・job executor |

## Data and Control Flow

```
policy.yaml layers              load 経路                                runtime 経路
────────────────────────────────────────────────────────────────────────────────────
builtin (Complete) ──┐
user (Partial)       ──┼─ validate_policy_schema(mode)
                     │     ├─ schema(section shape) — authoring 不変
                     │     ├─ resolve_exec_deny_list(security)  … 無条件
                     │     ├─ build_exec_input(process, command, ∅)      … collision
                     │     └─ resolve_exec_policy(exec_doc, &deny)      … 適用点1(全 grant)
invocation --grant   ──┤   (構造 guard: keys ⊆ {version,command.grants} — 不変)
                     ├─ merge_policy_layers(Replace ×2)
                     ├─ project (ProjectPartial — command 節 schema reject)
                     │     └─ project_task_names 捕捉(merge 前)  … provenance
                     ├─ merge_policy_layers(RestrictConcurrency)
                     └─ normalize_policy(completed, &project_task_names)
                           ├─ build_exec_input(…, names)   … task→fixed grant
                           │      {fixed:true, argv:command[1..], exitCodes:[0],
                           │       scope:project|trusted}
                           └─ resolve_exec_policy → RuntimePolicy.exec
                                  {grants:BTreeMap<ExecGrant>,
                                   command:ExecDefaults, process:ExecDefaults}

RuntimePolicy.exec ── to_value ──→ ToolRuntimeOptions.exec /
                                   InvocationToolRuntimeOptions.exec
   create_invocation_tool_runtime: resolve_exec_policy → inner.exec
   create_worker_scope: parity(inner.exec) → runtime_options.exec 再注入
   create_tool_runtime: resolve_exec_policy(options.exec, &exec_deny)
     ├─ content gate: Process→any(fixed) / Command→any(!fixed)
     ├─ writeScope gate: Command+!fixed 非空+writeScope → POLICY_VIOLATION
     ├─ pin: names 含 Command→!fixed grant 群 / 含 Process→fixed grant 群
     │        … 適用点2 + (degradable → surface 縮退)
     └─ ToolCtx.{exec, exec_pins, exec_deny} → ExecToolOptions
          ├─ command surface: {name,args} → resolve_exec_grant(!fixed)
          │    → check_exec_execution → proc::run(identity:command)
          ├─ process surface: {task} → resolve_exec_grant(fixed)
          │    → prepare_exec_grant(cwd/manifest/deny/writeScope/pin)
          │    → proc::run(identity:process)
          └─ verify_pinned_executable … 適用点3(両 surface 共有)
   route: RouteSupply → ExecPolicyView{policy:&policy.exec}
        ├─ viability: eligibility(scope==trusted && !fixed)
        │    → pin_exec_grant → check_exec_execution … NoGrant
        └─ run: policy.exec.project_grant(route.grant)
             → 投影 {route.grant} ∪ {fixed grants} → routed workflow
             → expected_input {"name":grant,"args":exec_args} … byte 一致
```

## Non-Functional Constraints

- **fail-closed 原則** — collision・deny・pin 不能・unknown
  grant/task・route eligibility 違反は全て error または
  `Viability::NoGrant` で fail-loud。silent shadow・承認の silent
  喪失は生じさせない（ADR Decision Drivers）。project task による
  trusted task 名の shadow は従来 deep-merge 規約の保存であり
  silent 喪失ではない（route 参照不能な project scope でのみ
  上書きが効くため A3 を弱めない — spec R4）。
- **serialized round-trip 冪等性** — `to_value(&policy.exec)` →
  `resolve_exec_policy` が同値を返すことが invocation parity の
  不変条件。resolved grant の全 field（`fixed`/`scope`/
  `inputs`/`outputs`/畳み込み済み lifecycle）を入力 schema が
  受理し、absent 値に対する fallback fold が deterministic
  （`fixed` → process block、`!fixed` → command block）でなければ
  ならない。
- **per-call PATH 変化の吸収喪失（意図的強化）** — task
  executable の構築時 pin により、session 中の ambient PATH 変化は
  spawn 前 `verify_pinned_executable` の path 同一性 + deny 再適用
  でのみ検査される（B1/B2）。pin 後の executable 差替えは
  `POLICY_VIOLATION` で fail-closed。
- **budget 意味の保存** — pool `maxConcurrent`・heartbeat
  interval は origin 別（`fixed` → `process` block、`!fixed` →
  `command` block）に保存。単一 pool への統一は budget 意味が
  変わるため行わない（D11）。
- **表現ドリフトの排除** — 同一概念の型は 1 箇所で定義:
  `ExecGrant`/`ResolvedExecPolicy`/`ExecDefaults`/`ExecHeartbeat`/
  `ArgPattern`/`argv_shell_re`/shared helpers は `policy/exec.rs`、
  pin/verify/check/prepare は `tools/exec/`。`tools/command`・
  `tools/process` は surface 固有の schema/description/details/
  identity のみ所有する薄い層（spec R7）。
- **後方互換（authoring）** — policy.yaml の `process`/`command`
  節・`policy grant` CLI・`--grant` flag・`tool:"process"`
  workflow・worker `tools:`/`optionalTools` literal は不変。破壊的
  変更は (a) cross-section collision fail-loud、(b) deny の task
  executable 拡大適用、(c) task executable の構築時 pin
  fail-closed 化、(d) `RuntimePolicy` serialized 形状（内部
  wire）、(e) `degradable` 宣言 `Command` surface の pin 失敗が
  縮退へ変わる（現行は無条件 hard fail — opt-in 宣言時のみ差分、
  (g) の uniform 規則による意図的 semantics 拡張）に限定
  （spec R11 — migration guide は issue 7 対象）。
- **re-export path 維持** — `tools::process::find_cheaper_error`/
  `OnTimeout`/`StreamCaptureResult`/`MAX_PROCESS_OUTPUT_BYTES`・
  `tools::process::{executable,tree}`（`process/mod.rs:40-45`）は
  `engine.rs:374`・`rig-adapter/tools.rs:58` が参照するため
  path 維持。

## Risks and Trade-offs

- **cross-section collision（新規 fail-loud）** — `command.grants`
  と `process.tasks` の同名は現行は別 namespace で共存可能だった
  が、単一 map 化で `InvalidRuntimePolicy` になる。load・
  `validate_policy_schema`・`policy grant add` の全入口で同一
  failure を観測できるよう段 2（`build_exec_input`）に置く。
  `details.{grant,task}` で両宣言位置を attribution（spec R4）。
- **cross-layer collision の fail 面** — collision は merged 後の
  単一 map で評価されるため、trusted `command.grants.x` ×
  project `process.tasks.x` の cross-layer 同名でも
  `InvalidRuntimePolicy` で落ちる。project layer 単体では valid
  な policy が、project 側から見えない trusted 層の grant 名と
  衝突して load fail し得る（silent shadow を許さない意図的な
  fail-loud — 衝突名の明示で operator が rename 可能）。
  `details.{grant,task}` が echo するのは衝突名のみで、trusted
  側の宣言内容（executable 等）は漏れない — project author が
  既に知っている自前の名のみが返るため情報露出は生じない
  （accepted surface）。
- **deny の task 拡大適用（意図的強化）** — opt-in
  `execDenyPatterns` を持つ operator は denied task executable で
  policy load が fail し得る。#164 の task 除外を統合で吸収する
  ADR O6a 確定 — deny の効果が exec surface 全体に及ぶ operator
  契約と一致し、project task が deny executable を走らせる hole
  を塞ぐ。
- **task pin の fail-closed 化** — per-call ambient PATH 解決
  （`prepare.rs:127-136`）から構築時 pin へ。unresolvable task は
  per-call error → 構築 error/degrade へ変わる回帰的強化だが、
  B1/B2 との整合・pin 機構の単一化として受容（degradable
  宣言時は縮退 — (g) の uniform 規則）。**確定した意図的拡張**:
  `degradable` 宣言 tool の pin fail が縮退になる規則は
  Command surface にも適用する（現行は degradable に関わらず
  無条件 hard fail — `registry.rs:1225-1234`）— spec R7/AC/R11
  に両 surface 適用として明文化済み、破壊的変更 R11-(e) として
  登録。非 opt-in worker の挙動は不変。
- **exec-doc 形状の二重入口** — `resolve_exec_policy` は
  serialized resolved shape と同一 input shape を受理するため、
  authoring section を direct 投入する誤用を
  `additionalProperties:false` + defaults block への
  `grants`/`tasks` 混入 reject で fail-loud に防ぐ
  （段 2 経由を構造的に強制）。
- **catalog `Command` arm の条件付き input 解決** — 現行は input
  非依存の root manifest 固定（`catalog.rs:75`）。宣言
  `inputs`/`outputs` を持つ `!fixed` grant の per-path 化のため、
  `input.name` が **literal string で `!fixed` grant へ resolve
  する場合のみ** `exec_grant_resources` へ委譲し、それ以外
  （`{$input}` 等 dynamic binding の非 string 名・未知名・`fixed`
  grant 一致）は root manifest へ fallback する。error を伝播
  しない選択の根拠: (a) `create_tool_catalog(None)` の空 catalog
  上で `tool:"command"` job が root manifest を期待する現行契約
  （`cheaper-workflow/tests/filesystem_contracts/
  resource_inference.rs:496-586` が到達）を保存、
  (b) workflow の `input.name` は inference 時点で binding 未解決
  になり得る（generic path は `resources(&job.input)?` で error
  伝播するため fail-loud は `{$input}` 使用 workflow を破壊する）、
  (c) literal 未知名に対しても root manifest は sound な overclaim
  であり fail-safe — 実 authorization は実行時
  `resolve_exec_grant` が担保するため、inference 層での
  `POLICY_VIOLATION` は false positive を生むだけ（認可境界を
  緩めない保守側の選択）。Process arm との非対称（同 arm は
  error 伝播）は意図的 — `tool:"process"` は inference 側
  `named_task` 検査が literal 名を既に要求しており規約を変え
  ない（`resource_inference.rs:131-138`）。
- **serialized shape churn** — `RuntimePolicy` の
  `process`/`command` → `exec` で golden fixture・options
  field・`InvocationInner`・route view・catalog が一括 churn
  （下表）。機能差分は unified 化自体で、残りは機械的な型追随。
- **`normalize_policy` の layer 非依存性破壊の回避** —
  `project_task_names` param は provenance stamp のみに使い
  validation 成否へ影響させない（`validate_policy_schema` は
  ∅ で呼び per-doc collision のみ検査）。layer-aware な情報を
  normalize へ流す channel は本 param 1 本に限定する。

### Churn 一覧（実測 — signature/field 追随）

- **core policy** — `policy/exec.rs` 新設。`process.rs`:
  `ResolvedProcessPolicy`→`ResolvedProcessSection`・`tasks` を
  `BTreeMap<String,ProcessTask>` 化・`resolve_process_task`/
  `process_task_resources`/helper/const の exec.rs 移設・
  `:374-494` unit test の exec 化。`command.rs`:
  `command_policy_properties()` へ `fixed`/`inputs`/`outputs`/
  `scope` 追加・`ResolvedCommandPolicy`→`ResolvedCommandSection`・
  `deny` param 除去・`CommandGrant`/`resolve_command_grant`/
  `command_grant_resources`/`project_grant`/`ArgPattern` の
  exec.rs 移設・`:431-736` unit test の移行。`schema.rs`:
  `RuntimePolicy`(:129-149)・`validate_policy_schema`(:437-446)・
  `normalize_policy`(:516-639)・`:1063` test。`load.rs`:
  project_task_names 捕捉(:237-255)・`normalize_policy` 呼出し
  (:258)・`:419-471` invocation test。`capability.rs`(:86-118)・
  `restrict.rs`(:110-150) の `RuntimePolicy` literal。
- **golden fixture** — `policy/load.json`・`policy/schema.json`・
  `policy/restrict.json`・`policy/process.json` の normalized
  `RuntimePolicy` 出力が `{"exec":{grants,command,process}}` 形状へ
  変わる（再採取）。`policy/merge.json` は raw map merge のため
  変更不要（collision case を追加してよい）。`policy/workflow.json`
  は `process`/`command` 参照なしで不変。driver
  `tests/golden/policy_process.rs`（:1-3,:17,:26,:31 —
  `resolve_process_task`/`process_task_resources`/
  `resolve_process_policy` を呼ぶ）は
  `resolve_exec_grant`/`exec_grant_resources`/`resolve_exec_policy`
  の unified op へ書き換え（fixture case の op 名は維持可 —
  driver 側で unified 呼出しへ写像）。
- **tools** — `tools/exec/{mod,prepare}.rs` 新設。
  `registry.rs`（options/gate/pin/`ToolCtx`/builder/dispatch/
  literal :1316-1332）。`invocation.rs`（options/inner/parity/
  re-inject/resolve）。`catalog.rs`（policy 型・2 arm・
  `from_resolved_policy`）。`tools/command/mod.rs`・
  `tools/process/mod.rs` thin 化。`tools/process/prepare.rs` →
  `tools/exec/prepare.rs`。`tools/process/tests.rs`（:15-37 の
  `process_operation_declaration`/`resolve_process_policy` →
  `exec_operation_declaration`/`resolve_exec_policy`、:64-75 の
  `resolve_process_policy`/`ProcessToolOptions` →
  `resolve_exec_policy`/`ExecToolOptions`）。
  test: `proc_command.rs`・`proc_process.rs`・`proc_invocation.rs`・
  `proc_runtime.rs`・`proc_degrade.rs`・`characterization.rs`・
  `differential.rs`（`ToolRuntimeOptions{process,command}` →
  `exec`・`ResolvedProcessPolicy`/`ResolvedCommandPolicy` literal →
  `ResolvedExecPolicy`・`resolve_command_policy` test 呼出し →
  exec resolver 経由）。**fixture harness seam（確定）**:
  `differential.rs::runtime_options`（:646-698）の
  `spec.process`（authoring shape）→ `build_exec_input` →
  `resolve_exec_policy` → `to_value` → `options.exec` 変換を
  harness 内に追加する。`tests/fixtures/process.json`/
  `runtime.json` は **不変** — authoring-shaped `runtime.process`
  spec を fixture rewrite なしで受理し、synthesis 経路自体を
  fixture 全 case で回帰させる（`options.exec` の exec-shaped
  境界契約を壊さず、production `to_value(&policy.exec)` と同じ
  材料を供給する選択 — fixture を exec 形状へ rewrite する案は
  ~60 spec block の機械的 churn + authoring 経路 coverage 喪失の
  ため不採用）。
- **cli** — `route/resolve.rs`(:10-17,:49-82,:310-343,:357-…・
  view/test)・`route/supply.rs`(:23,:45-53,:126)・
  `route/run.rs`(:121,:176,:356-364)・`route_cmd.rs`
  (`supply.command_view()` :96 → `exec_view()`・
  `ResolveInput{command_policy}` :101 → `exec_policy` — rename
  追随のみ)・`hook/routing_policy.rs`(:140-142,:1467,:2055-2106)・
  `workflow_run.rs`(:241-263,:499-529,:753)・
  `workflow_context.rs`(:224,:315-318)・
  `policy.rs`(module doc :5-15 + `parse_grant_spec` :189-205 の
  `fixed`/`scope` reject — §(m) 参照)・
  `grant.rs`(module doc :1-7 + `build_invocation_layer` :49-104 の
  `fixed`/`scope` reject — §(m) 参照)。
- **runtime** — `run/mod.rs`(:337-344,:496-500)・
  `dev/fixture.rs`(:58-65,:759-817)・`tests/run_dev_seam.rs`
  (:178-219,:1046-1252 — `p.process.tasks.insert` →
  `p.exec.grants.insert`)・`tests/ollama_e2e.rs`(:219 付近
  literal)。
- **workflow / rig-adapter** — `cheaper-workflow` src は code
  不変（M5）。test: `tests/process.rs`(:32-,:108-)・
  `filesystem_contracts/artifacts.rs`(:69-,:102-) の
  `ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal・
  `filesystem_contracts/resource_inference.rs` — `Command` arm の
  条件付き解決を pin する case 追加（literal 名 + 宣言
  inputs/outputs を持つ `!fixed` grant → per-path、非 literal・
  未知名・`fixed` 名 → root manifest fallback。
  `:496-586` の既存 case は空 catalog + literal 名で fallback
  を既に pin 済み — 回帰維持）。
  `rig-adapter/tests/adapter.rs`(:319-320) 同 literal。
- **cli integration** — `tests/integration/policy.rs`（collision
  case 追加 — `command.grants`×`process.tasks` 同名 add が
  `InvalidRuntimePolicy` + `parse_grant_spec` の `fixed`/`scope`
  reject）・`tests/integration/route_run.rs`・
  `tests/integration/run.rs`（`--grant` 経路 — `fixed:true`/`scope`
  宣言値の reject 回帰を含む）。

## Docs 更新契約

- `docs/guides/managed-process.md` — `process.tasks` が内部で
  `fixed:true` grant へ写像されること・task executable が scope
  構築時に pin されること（unresolvable task は構築 fail /
  opt-in degradable 時は tool 縮退）・`execDenyPatterns` が
  task executable にも効くこと（opt-in profile の適用拡大）・
  `command.grants`×`process.tasks` 同名の collision が
  `InvalidRuntimePolicy` であることを記述する。
- `docs/guides/command.md` — grant spec に新 optional field
  `fixed`/`inputs`/`outputs`/`scope` の意味（trusted 層で fixed
  argv grant・resource 宣言・自己弱体化 scope を authoring 可 —
  `scope:"project"` は route 参照不能）を追記。`process` との
  関係（disjoint surface — `command` は `!fixed` のみ、
  `process` は `fixed` のみ解決）・collision 制約・deny uniform
  適用を明記。
- `docs/guides/configuration.md` — policy.yaml reference に
  `command.grants` 新 field・`process` 節の loader 写像
  （project 宣言 task は `scope:"project"`）・collision 制約・
  `execDenyPatterns` の task 適用拡大を追記。trusted 層の
  `process` 節は従来どおり fixed task を供給する（authoring
  不変 — 「trusted layer で `process` section を書ける」挙動は
  `partial_schema():332` で既に受理済みであり、本 change で
  追加制約は入れない）。
- `docs/guides/usage.md` — `policy grant` CLI 節に collision
  制約（`process.tasks` と同名の grant は add 時点で
  `InvalidRuntimePolicy`）と、spec への `fixed`/`scope` 宣言が
  `policy grant add`・`--grant` 両入口で `InvalidRequest` reject
  されること（これら CLI は `!fixed`+`scope:"trusted"` の
  command grant 供給専用 — fixed grant は YAML 直接 authoring）
  を追記。
- `docs/guides/workflow-usage.md` — `tool:"process"` job が
  `input.task` → `fixed` grant 名解決の compat alias として
  不変で動くこと（`tools.allow` への `process` literal・
  named-task input 必須は不変）を明記。
- `docs/guides/workers.md` — `optionalTools`/writeScope 節に
  「writeScope 下では `outputs ⊆ writeScope` の fixed grant のみ
  exec 可、`!fixed`（argv 供給）grant は構築時
  `POLICY_VIOLATION`」の規則を明記（現行 process writeScope
  記述を unified 用語へ）。併せて `optionalTools` 節へ
  「degradable 宣言された exec surface tool（`process`/`command`）
  は、surface に到達可能な grant の executable pin 失敗時も
  scope 構築を error にせず当該 tool のみ未登録へ縮退する」
  ことを明記する（pin-fail×degradable の両 surface 適用 —
  `command` は opt-in 宣言時の新規挙動、現行は無条件
  `POLICY_VIOLATION`）。
- `crates/cheaper-cli/resources/policy.yaml` — authoring surface
  不変のため節構成は変更しない（`process:`/`command:` 節を
  そのまま保持 — 無改訂が正しい状態）。`security` 節の comment
  で deny が task executable にも及ぶ旨を補足してよい。

## Deferred Design Concerns

- **`policy exec` への CLI 改名** — 現行 `policy grant` の用語
  維持で十分（authoring 破壊的変更を本 issue に持ち込まない）。
  用語の統一需要が発生した場合に再着手（#169 の sweep 論点）。
- **`process` の `TOOL_ALIASES` 化** — ADR O5c で却下済み。
  vocabulary alias へ移すと worker `tools:`/`tools.allow`/
  `optionalTools` の literal 照合が `command` 名へ変わり
  M5 互換が破れる。canonical 名のまま実装層 alias で維持する
  決定は本 design で確定。
- **`substitute.grant` の fixed grant 参照** — route eligibility
  の `!fixed` 制約で NoGrant になるため問題化しないが、固定
  argv への substitute は issue 5（substitute exec mode）側の
  設計論点として残置（spec Deferred と同一）。
- **route grant-set `grants[]` 投影（issue #166）** — 本 design
  の `is_route_eligible`/`project_grant` がその投影 seam と
  なるが、set 投影自体は #166 の scope。
- **trusted `!fixed` grant の `inputs`/`outputs` declare と
  writeScope** — schema は受理するが M4 の argv 供給拒否で
  writeScope 下には到達不能（spec Deferred と同一）。resource
  推論は per-path manifest を使うため宣言自体は有用。
- **`process`/`command` section heartbeat の単一化** — origin 別
  保存（D11）で維持。単一 heartbeat への統一は pool budget
  意味が変わるため対象外。
- **`resolve_process_policy`/`resolve_command_policy` の段 1
  resolver の最終的な帰結** — ADR O9a で存続決定（authoring
  境界 validation + section defaults 供給）。将来 authoring
  shape が変わる変更が入った場合に統合先を再検討。

## Verification Notes

test 計画（spec Acceptance Criteria への対応）:

- [ ] **統一 schema** — `resolve_exec_policy` が `fixed`/
  `scope`/optional `inputs`/`outputs`/畳み込み lifecycle を保持、
  `fixed:true` grant は `resolve_exec_grant(!fixed)` で不可視・
  `resolve_exec_grant(fixed)` で解決される（exec.rs unit test。
  M1/M2 AC）。
- [ ] **loader 写像 + provenance** — project `process.tasks` が
  `fixed:true`+`scope:"project"` grant へ写像され、`scope` が
  serialized `RuntimePolicy.exec` に載る（load.rs test。M3 AC）。
  project task 名による trusted task 名の deep-merge shadow が
  `scope:"project"` で保存されることも pin。
- [ ] **collision fail-loud** — `command.grants`×`process.tasks`
  同名で `InvalidRuntimePolicy` + `details.{grant,task}` が
  `validate_policy_schema`（各 mode）・`load_policy`・
  `policy grant add` `write_validated` で同一 failure（schema.rs
  unit + cli integration `policy.rs`）。
- [ ] **route eligibility** — `scope:"project"`/`fixed`/不在の
  grant 参照が `Viability::NoGrant`、trusted+`!fixed` は解決
  （`route/resolve.rs` test）。`project_grant` が
  `{route.grant} ∪ {fixed grants}` を残す（core unit + route run
  経路）。
- [ ] **writeScope** — `outputs ⊆ writeScope` の fixed grant は
  writeScope 下で exec 可、`outputs` 超過・`!fixed` grant は
  拒否（`prepare_exec_grant`/`registry` test。M4 AC）。構築時
  gate（`Command`+`!fixed` 非空+writeScope → `POLICY_VIOLATION`）
  の維持。
- [ ] **`tool:"process"` workflow** — `{"kind":"tool",
  "tool":"process","input":{"task":…}}` を持つ既存 workflow が
  `UnknownTool` なしで動く（`benchmarks/scenarios/scenarios.json`
  の `processPolicy` scenario・`cheaper-workflow/tests/process.rs`
  回帰 — M5 AC）。
- [ ] **deny uniform** — `execDenyPatterns` match の task
  executable が policy load で `InvalidRuntimePolicy`
  （declared）、pin・spawn 前再検証も全 grant へ効く（exec.rs +
  `proc_*` test。適用点 1-3）。
- [ ] **task pin** — unresolvable task executable の scope が
  構築時 `POLICY_VIOLATION`（当該 surface tool が degradable
  宣言時は縮退 — `Process` と `Command` の両 surface で同規則、
  `proc_degrade.rs` で Command 側縮退も pin）。task pin 後の
  PATH 差替えが spawn 前 `verify_pinned_executable` で拒否される。
- [ ] **catalog `Command` arm** — literal 名 + 宣言
  `inputs`/`outputs` を持つ `!fixed` grant で per-path manifest、
  非 literal 名・未知名・`fixed` 名で root manifest fallback
  （error 非伝播）を `resource_inference.rs` で pin（§(k) の
  条件付き解決 — 既存 `:496-586` case の回帰維持）。
- [ ] **invocation parity** — `to_value(&policy.exec)` →
  `resolve_exec_policy` → `PartialEq` が成立（`proc_invocation.rs`
  — `options.exec`→`inner.exec` round-trip）。
- [ ] **`--grant`/`policy grant`** — invocation 合成 grant が
  `!fixed`+`scope:"trusted"` で `command` surface 到達・collision
  規則が `policy grant add` で発火・`fixed:true`/明示 `scope`
  宣言 spec が `--grant`（`build_invocation_layer`）と
  `policy grant add`（`parse_grant_spec`）の両入口で
  `InvalidRequest` reject（`grant.rs`/`policy.rs`/integration
  test 回帰）。
- [ ] **既存契約 test 回帰** — pin・env_clear・stdin null・
  process group・lifecycle（timeout/kill grace/heartbeat）・
  exit code 契約・fs boundary・denylist・writeScope が全て
  `proc_*`/`characterization`/`differential`/`contracts` で pass。
- [ ] **serialized 形状** — `RuntimePolicy` の `exec` key 化で
  `policy/{load,schema,restrict,process}.json` golden が期待どおり
  再採取される（`merge.json`/`workflow.json` は不変確認）。
