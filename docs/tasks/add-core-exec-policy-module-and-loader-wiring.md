---
id: "TASK-034KQsi5CcOH2MsHWhWWnC"
type: "task"
status: "done"
title: "cheaper-core policy/exec module と 3 段 resolver・schema/loader 配線"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-core/src/policy/exec.rs"
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
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/process.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_process.rs"
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
    generated: []
  implements:
    - "docs/plans/unified-exec-grant-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/unified-exec-grant-model.md"
  blocks:
    - "docs/tasks/plumb-unified-exec-through-tools.md"
    - "docs/tasks/document-unified-exec-grant-model.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/unified-exec-grant-model.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/unified-exec-grant-model.md"
  references:
    - "docs/designs/unified-exec-grant-model.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/discovery/unified-exec-grant-model-issue-165.md"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-core/src/policy/process.rs"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/load.rs"
  defers: []
  deferred-by: []
---

# cheaper-core policy/exec module と 3 段 resolver・schema/loader 配線

## Work

plan `docs/plans/unified-exec-grant-model.md`
（PLAN-034KQs6XKnAqahkSVke78h）の T1。cheaper-core の policy 層に
統一 exec grant model を新設し、`policy/exec.rs`（単一 grant
namespace・argv 契約・provenance の owner）+ 3 段 resolver
（section resolvers → `build_exec_input` →
`resolve_exec_policy`）+ `RuntimePolicy.exec` + schema/loader
配線を実装する。design（DESIGN-034KP4lfgfD7DdGiDV1YuT）の
統一型 verbatim・3 段確定手順・exec-doc schema・配線表・
error 契約をそのまま実装する — 再解釈しない。

### 1. `policy/exec.rs`（新設）

design (a)/(b)/(d) の module contract 通りに集約する:

- `pub enum ExecGrantScope { Trusted, Project }` — serde
  `"trusted" | "project"`（`rename_all = "lowercase"`）・
  `Debug, Clone, Copy, PartialEq, Eq, Serialize`。
- `pub struct ExecGrant` — `rename_all = "camelCase"`・
  `{executable, fixed, argv?, inputs?, outputs?, cwd, env,
  timeout, kill_grace, max_output_bytes, exit_codes, scope}`・
  `Option` field は `skip_serializing_if`。design (a) の
  verbatim rust（serialized shape は parity check の比較対象 —
  declared-shape equality）。pin 済み canonical path は載せない。
- `pub struct ExecHeartbeat { interval: String }` — 旧
  `ProcessHeartbeat` 改名（serde shape `{interval}` 不変）。
- `pub struct ExecDefaults` — `{max_concurrent, timeout,
  max_output_bytes, kill_grace, heartbeat}`・serialized shape は
  現行 2 section と同型。
- `pub struct ResolvedExecPolicy { grants: BTreeMap<String,
  ExecGrant>, command: ExecDefaults, process: ExecDefaults }` —
  `is_route_eligible(grant)`（`scope==Trusted && !fixed`）と
  `project_grant(&mut self, name) -> bool`（design verbatim —
  eligibility 検査後 `retain(|n,g| n == name || g.fixed)`）。
- `pub fn build_exec_input(process, command,
  project_task_names: &BTreeSet<String>) -> Result<Map, CheaperError>`
  — design (b) の確定手順 6 段: (1) `resolve_process_policy`
  → section、（2) `resolve_command_policy` → section（deny 照合
  なし — 適用点 1 は段 3 へ一本化）、(3) `command.grants` keys
  ∩ `process.tasks` keys 非空で `exec_invalid("grant name
  collides between command.grants and process.tasks")` +
  `details.{grant,task}`（両方に衝突名）、(4) task → exec-doc
  grant 変換（`{executable:command[0], fixed:true, argv:
  command[1..], inputs, outputs, cwd, env, lifecycle:process
  section resolved defaults, exitCodes:[0], scope: names.
  contains(name) ? "project" : "trusted"}` — lifecycle は
  synthesis で明示値として畳み込み）、(5) command grant →
  exec-doc grant 変換（declared fields + `fixed`/`scope`/
  `inputs`/`outputs` 既定値畳み込み — `scope:"project"` は
  自己弱体化として受理）、(6) `{grants, command:{defaults},
  process:{defaults}}` を返す。
- `pub fn resolve_exec_policy(input: Option<&Value>, deny:
  &ExecDenyList) -> Result<ResolvedExecPolicy, CheaperError>` —
  design (b) の確定手順: (1) `exec_input_validator` で検査、
  (2) `command`/`process` defaults block を `ExecDefaults` へ
  resolve（欠落 → 既定値 `max_concurrent:1, timeout:"2m",
  max_output_bytes:65_536, kill_grace:"2s", heartbeat:"30s"` —
  `process.rs:53-56` の共有 const を継承）、(3) grant loop —
  `executable` 抽出直後に**適用点 1**: `deny.is_denied(
  &executable)` → `exec_invalid("executable is denied by
  security.execDenyPatterns")` + `details.{grant,executable}`
  （**fixed grant は `details.task` 併記**）、`fixed`/`scope`
  は declared 値 or default（`false`/`"trusted"`）、`argv`:
  `!fixed` なら `ArgPattern::compile`（`fixed` なら literal
  扱いで compile しない）、`cwd` は `relative()`、`inputs`/
  `outputs`・`env` は shared helper で全 grant へ適用、
  lifecycle 欠落 field は `fixed`→`process`/`!fixed`→`command`
  defaults で fold（dormant fallback）、`exit_codes` 未宣言は
  `[0]`、(4) `locale_compare` 順 → `BTreeMap`。
- `pub fn resolve_exec_grant(policy, name, fixed: bool) ->
  Result<ExecGrant, CheaperError>` — surface 別 error 語彙
  内蔵（`fixed` 期待値と `grant.fixed` が一致しない・不在:
  fixed surface → `"unknown or unauthorized process task"` +
  `details.task`、非 fixed → `"unknown or unauthorized command
  grant"` + `details.grant` → `PolicyViolation`）。
- `pub fn exec_grant_resources(policy, name) ->
  Result<ResourceManifestDeclaration, CheaperError>` —
  `inputs`/`outputs` のいずれか宣言（`Some([])` 含む）→
  per-path manifest（`process_task_resources`:340-363 同等 —
  `reads=inputs`・`writes=outputs`・`produces=outputs`・`**`
  末尾 → `Subtree`）。両方 `None` → conservative root manifest
  （`command_grant_resources`:419-429 同等）。
- `pub fn exec_policy_properties()`/`exec_defaults_properties()`
  — design (d) の verbatim json（`grants` maxProperties 256・
  name pattern・grant properties・defaults block 2 つ）。
  **`heartbeat` nested object は `{"type":"object",
  "additionalProperties":false,"properties":{"interval":…}}`
  を再現**（design (d) 実装注意 — 落とすと unknown key 混入を
  受理し round-trip/parity を破る）。
- `fn exec_invalid(reason) -> CheaperError` —
  `command_invalid`/`security_invalid` と同型 —
  `"invalid exec policy: {reason}"` + `InvalidRuntimePolicy`。
- 移設: `ArgPattern`/`ArgToken`（`command.rs:154-226`）・
  `bounded_duration`/`relative`/`str_vec`/`ProcessHeartbeat→
  ExecHeartbeat`/`DEFAULT_*`/`MAX_PROCESS_OUTPUT_BYTES`/
  `MAX_TIMER_MS`/`argv_shell_re`（`process.rs:120-149`・
  `:164-183`・`:45-56` 他）→ `policy/exec.rs`。

### 2. `policy/mod.rs`

module 一覧に `pub mod exec;` を追加（alphabetic 位置）。

### 3. `policy/process.rs` — section 型化

- `ResolvedProcessPolicy` → `ResolvedProcessSection` 改名、
  `tasks` を `Map<String,Value>` → `BTreeMap<String,ProcessTask>`
  へ typed 化（synthesis 入力）。`resolve_process_policy` の
  semantic 規則（task env/cwd/相対 path・duration bound —
  `"invalid process policy:"` prefix の authoring attribution）
  は不変。
- `process_policy_properties()`（:60-96）は**不変**。
- `resolve_process_task`/`process_task_resources`/共有 helper・
  const の exec.rs 移設。`ProcessTask` 自体は `process`
  section の authoring decl 型として残存。
- `:374-494` unit test を exec 経路へ移行。

### 4. `policy/command.rs` — section 型化 + schema 拡張

- `command_policy_properties()`（:86-128）の grant spec
  `properties` に 4 field 追加（design (c) verbatim —
  `"fixed":{"type":"boolean"}`・`"inputs":paths`・
  `"outputs":paths`・`"scope":{"type":"string","enum":
  ["trusted","project"]}` — `process tasks` と同じ paths
  schema）。
- `ResolvedCommandPolicy` → `ResolvedCommandSection` 改名、
  `grants` を `BTreeMap<String,ExecGrant>`（`fixed`/`scope`/
  `inputs`/`outputs` 既定値畳み込み済み）へ。
- `resolve_command_policy`（:239-396）— **`deny` param を除去**
  （適用点 1 は `resolve_exec_policy` へ一本化。適用点 1 挿入
  位置:295-312 の deny check も除去）。
- `CommandGrant`/`resolve_command_grant`/`command_grant_resources`/
  `project_grant`/`ArgPattern` の exec.rs 移設。
- `:431-736` unit test を exec 経路へ移行。

### 5. `policy/schema.rs` — `RuntimePolicy.exec` + unified 配線

- `RuntimePolicy`（:129-149）— `process`/`command` field 除去、
  `workflow` 直後に `pub exec: ResolvedExecPolicy`（serialized
  key 順 `workflow` → `exec` → `route` → `security` → …）。
- `validate_policy_schema`（:437-446）— 個別再 resolve を
  `build_exec_input(raw.get("process"), raw.get("command"),
  &BTreeSet::new())` + `resolve_exec_policy(&exec_doc,
  &exec_deny)` へ置換。`resolve_exec_deny_list` の無条件
  compile（:443）は維持 — **collision・declared deny・全
  semantic 規則が validate 経路で発火する入口**（`policy
  grant` `write_validated` もこの経路を共有）。
- `normalize_policy`（:516-639）— signature を
  `(raw: &Map<String,Value>, project_task_names: &BTreeSet<
  String>)` へ変更。内部で `build_exec_input(raw.get("process"),
  raw.get("command"), project_task_names)?` →
  `resolve_exec_policy(&exec_doc, &exec_deny)?` →
  `RuntimePolicy.exec` 格納（`security` resolve + compile は
  維持、`process`/`command` 個別 resolve は除去）。
- import を `use crate::policy::exec::{…}` へ更新。
- `project_task_names` は provenance stamp のみ（validation 成否
  へ影響しない — `validate_policy_schema` は ∅ で呼ぶ）。

### 6. `policy/load.rs` — merge 前捕捉 + normalize 呼出し

- :237-256 — project merge の `validate_policy_schema
  (ProjectPartial)` 直後・`merge_policy_layers` 直前に
  `project_task_names` を捕捉（validated project doc の
  `process.tasks` keys を `BTreeSet` へ）。**merge 前捕捉が
  必須** — RestrictConcurrency deep-merge で同名 task が
  project field で override された後では provenance を導出
  不能。
- :198-220 — section re-embed は現行構造維持（complete schema
  の section required key を満たす目的）。deny compile 維持、
  `resolve_command_policy` 呼出しは deny param 除去へ追随
  （deny 注入は除去 — 適用点 1 は段 3 へ移動）。
- :257-258 — `normalize_policy(&completed, &project_task_names)`
  呼出し追随。
- :120-189 — invocation guard（`command.grants` key 制約）は
  **不変**。
- attribution 規約（不変）: invocation 層 `command.grants` と
  merged `process.tasks` の衝突 error は `details.layer` を
  持たない（merged-map 評価の帰属不能 — accepted gap）。

### 7. `policy/{capability,restrict}.rs` — literal churn

- `capability.rs`(:86-118)・`restrict.rs`(:110-150) の
  `RuntimePolicy` literal を `exec:` 形状へ。
  `restrict_policy` 本体は `..policy.clone()` で無変更。

### 8. unit test（src 内 `#[cfg(test)]` — TSPEC Rules 対応）

- `exec.rs` — 統一 schema（`fixed`/`scope`/`inputs`/`outputs`/
  畳み込み lifecycle 保持・`fixed:true` は `resolve_exec_grant
  (!fixed)` で不可視・`resolve_exec_grant(fixed)` で解決）、
  `argv` 2 意味化（fixed literal は compile しない）、defaults
  block 既定値・lifecycle deterministic fold、exec-doc 形状
  fail-loud（`additionalProperties:false`・nested `heartbeat`
  未知 key・`None`/`null` collapse）、collision（同名交差 →
  `InvalidRuntimePolicy` + `details.{grant,task}`）、
  task→grant 変換（`fixed:true`/`argv:command[1..]`/
  `exitCodes:[0]`/`scope` stamp）、round-trip 冪等
  （serialized → re-resolve → 同一）、deny 適用点 1
  （declared basename — fixed grant の `details.task` 併記）、
  `exec_grant_resources` tri-state（per-path/`Some([])` 空
  manifest/root）、`project_grant`（`{name}∪{fixed}` 投影 +
  eligibility 内蔵）、`is_route_eligible`。
- `schema.rs` — `validate_policy_schema` 経由の collision（各
  mode で同一 `InvalidRuntimePolicy`）・unified resolve 接続・
  `RuntimePolicy.exec` serialized shape（`:1063` test 追随）。
- `load.rs` — project task 名の `scope:"project"` stamp・
  deep-merge shadow（project 名 → `scope:"project"` で保存）・
  merged collision の `details.layer` 無し pin・invocation
  guard 不変（`:419-471` invocation test 追随）。

### 9. golden fixture（`crates/cheaper-core/tests/golden/`）

- `fixtures/policy/{load,schema,restrict,process}.json` —
  normalized `RuntimePolicy` 出力を `{"exec":{grants,command,
  process}}` 形状へ再採取 + `fixed`/`scope`/collision case を
  追加。
- `merge.json`（raw map merge）・`workflow.json`（process/
  command 参照なし）は不変確認。
- `policy_process.rs` driver（:1-3,:17,:26,:31）—
  `resolve_process_task`/`process_task_resources`/
  `resolve_process_policy` 呼出しを `resolve_exec_grant`/
  `exec_grant_resources`/`resolve_exec_policy` の unified op へ
  写像（fixture case の op 名は維持可）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/unified-exec-grant-model.md`
  （PLAN-034KQs6XKnAqahkSVke78h — T1）
- `docs/designs/unified-exec-grant-model.md`
  （DESIGN-034KP4lfgfD7DdGiDV1YuT — (a) 統一型 verbatim・(b)
  3 段手順・(c)(d) schema・(e) 配線表・(f) 適用点 1・(l)
  signature・(m) 配線・(n) error 契約が AUTHORITATIVE）
- `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6 — R1/R2/R3/R4/R5/R10/R11）
- `docs/test-specs/unified-exec-grant-model.md`
  （TSPEC-034KQs1NIL8ahirht6QteV — 統一 schema/`fixed`・
  exec-doc fail-loud・build_exec_input provenance+collision・
  round-trip 冪等・deny 適用点 1・manifest tri-state・
  serialized 形状 Rules）

## Dependencies

- なし（起点 task）。design の行指定は現行コードと一致済み
  （branch `feature/issue-165-unified-exec-grant-model` で
  実測 — discovery drift 訂正表適用済み）。
- 既知の transient: 本 task で `resolve_command_policy` の
  `deny` param 除去・`ResolvedProcessPolicy`/
  `ResolvedCommandPolicy` の section 型改名・`RuntimePolicy`
  field 変更・`resolve_process_task`/`process_task_resources`/
  `resolve_command_grant`/`command_grant_resources`/`ArgPattern`/
  `argv_shell_re` の移設により、cheaper-tools（`registry.rs`・
  `invocation.rs`・`catalog.rs`・`tools/command`・
  `tools/process`）・cheaper-cli（`route/resolve.rs`・
  `route/supply.rs`・`route/run.rs`・`hook/routing_policy.rs`・
  `workflow_context.rs`）・cheaper-runtime（`dev/fixture.rs`）・
  cheaper-workflow/rig-adapter test が T2/T3 の追随まで
  compile 不可。本 task の検証は `cargo test -p cheaper-core`
  に scope する（cheaper-core は単独で compile・test 可能）。

## Affected Files

- `crates/cheaper-core/src/policy/exec.rs`（add）
- `crates/cheaper-core/src/policy/mod.rs`（modify）
- `crates/cheaper-core/src/policy/process.rs`（modify）
- `crates/cheaper-core/src/policy/command.rs`（modify）
- `crates/cheaper-core/src/policy/schema.rs`（modify）
- `crates/cheaper-core/src/policy/load.rs`（modify）
- `crates/cheaper-core/src/policy/capability.rs`（modify）
- `crates/cheaper-core/src/policy/restrict.rs`（modify）
- `crates/cheaper-core/tests/golden/fixtures/policy/load.json`
  （modify）
- `crates/cheaper-core/tests/golden/fixtures/policy/schema.json`
  （modify）
- `crates/cheaper-core/tests/golden/fixtures/policy/process.json`
  （modify）
- `crates/cheaper-core/tests/golden/fixtures/policy/restrict.json`
  （modify）
- `crates/cheaper-core/tests/golden/policy_process.rs`（modify）

## Done When

- [x] `policy/exec.rs` が design の module contract 通りの pub
      API（`ExecGrantScope`・`ExecGrant`・`ExecHeartbeat`・
      `ExecDefaults`・`ResolvedExecPolicy` +
      `is_route_eligible`/`project_grant`・`resolve_exec_policy`・
      `build_exec_input`・`resolve_exec_grant`・
      `exec_grant_resources`・`exec_policy_properties`）を持ち、
      `mod.rs` が `pub mod exec;` を宣言する
- [x] `process.rs` が `ResolvedProcessSection` +
      `BTreeMap<String,ProcessTask>`、`command.rs` が
      `ResolvedCommandSection` + `BTreeMap<String,ExecGrant>` +
      `deny` param 除去 + 4 field schema 拡張を持ち、
      `ArgPattern`/shared helper/const が `exec.rs` へ移設されて
      いる
- [x] `RuntimePolicy.exec` が `workflow` 直後の field として
      normalize され（`process`/`command` 除去）、serialized
      出力が `{"exec":{grants,command,process}}` 形状を持つ
- [x] `validate_policy_schema` が `build_exec_input(..,∅)` +
      `resolve_exec_policy(..,&exec_deny)` 経由で collision・
      declared deny・全 semantic 規則を全 mode で発火する
- [x] `load_policy` が merge 前に `project_task_names` を捕捉し、
      `normalize_policy(.., &project_task_names)` が project
      task を `fixed:true`+`scope:"project"` grant へ写像する
      （deep-merge shadow が `scope:"project"` で保存される）
- [x] `capability.rs`・`restrict.rs` の `RuntimePolicy` literal
      が `exec:` 形状を持つ
- [x] exec/schema/load の unit test が TSPEC Rules に対応して
      存在し、golden fixture 4 件が再採取され（`merge`/
      `workflow` 不変確認）、`policy_process.rs` driver が
      unified op へ写像され `cargo test -p cheaper-core` が
      pass

## Out of Scope

- `crates/cheaper-tools` 側の unified 機構・options 配管・
  test churn（T2）
- `crates/cheaper-cli`/`cheaper-runtime`/`cheaper-workflow`/
  `cheaper-rig-adapter` 側の call site・route 層・literal
  churn（T3）
- docs（`managed-process.md`・`command.md`・
  `configuration.md`・`usage.md`・`workflow-usage.md`・
  `workers.md`・`resources/policy.yaml`）（T4 — T1 完了後に
  並列着手可）
- `--grant`/`policy grant add` の `fixed`/`scope` reject（T3 —
  CLI 入口 validation）
- authoring surface（policy.yaml 節名・shape）の変更・
  `policy grant`/`--grant`/route `grant` 用語の改名
- route grant-set 投影（`grants[]`、#166）・substitute exec
  mode（issue 5）・trusted flag 発行主体（issue 6）

## Verification

- [x] `mise exec -- cargo test -p cheaper-core` exits 0
      （unit test + golden target 全 pass — single-crate では
      dev-seam feature 適用外のため workspace feature 指定なし。
      実測 244 passed — `057d22b`）
- [x] `mise exec -- cargo clippy -p cheaper-core --all-targets`
      exits 0（dead import・dead code なし — `mise run check`
      の clippy stage で workspace `--all-targets` として実測）
- [x] `grep -n "exec" crates/cheaper-core/src/policy/mod.rs`
      が `pub mod exec;` を返す（mod.rs:5 で実測）
