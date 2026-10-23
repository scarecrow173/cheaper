---
id: "TASK-034KQsrpnzHdeRKf5M3KiN"
type: "task"
status: "todo"
title: "cheaper-cli/runtime/route/invocation/catalog の unified exec 追随と workspace 復帰"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
  changes:
    added: []
    modified:
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
        path: "crates/cheaper-cli/src/grant.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_context.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/fixture.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/common/mod.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/resource_inference.rs"
      - type: "file"
        path: "crates/cheaper-workflow/src/engine.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/process.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/common/mod.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/resource_inference.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_fixture_test.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/tests/adapter.rs"
    deleted: []
    renamed:
      - type: "item"
        path: "crates/cheaper-cli/src/route/resolve.rs::CommandPolicyView"
        note: "ExecPolicyView へ改名（view は grant 単位の route 判定 surface）"
    moved: []
    generated: []
  implements:
    - "docs/plans/unified-exec-grant-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/unified-exec-grant-model.md"
    - "docs/tasks/plumb-unified-exec-through-tools.md"
  blocks:
    - "docs/tasks/verify-unified-exec-grant-model.md"
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
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/grant.rs"
    - "crates/cheaper-cli/src/policy.rs"
    - "crates/cheaper-runtime/src/run/mod.rs"
    - "crates/cheaper-workflow/src/resource_inference.rs"
  defers: []
  deferred-by: []
---

# cheaper-cli/runtime/route/invocation/catalog の unified exec 追随と workspace 復帰

## Work

plan `docs/plans/unified-exec-grant-model.md`
（PLAN-034KQs6XKnAqahkSVke78h）の T3。T1（core 型）+
T2（`tools/exec` 機構・`ToolCtx.exec`・`ToolRuntimeOptions.
exec`）の API 変更へ cheaper-cli・cheaper-runtime・
cheaper-workflow・cheaper-rig-adapter の call site・route 層・
test literal を追随させ、workspace 全 compile を復帰する。
route viability は `trusted && !fixed` 前提、`project_grant`
投影・`ExecPolicyView`・`--grant`/`policy grant add` の
`fixed`/`scope` reject・`tool:"process"` canonical 互換を
design（DESIGN-034KP4lfgfD7DdGiDV1YuT）の (e)(f)(j)(k)(l)(n)
契約通り実装する — 再解釈しない。

### 1. `route/resolve.rs` — `ExecPolicyView` + trusted/!fixed 前提

- `CommandPolicyView` → `ExecPolicyView` 改名（view は grant
  単位の route 判定 surface。`grant_names`/`grants` accessor・
  shape は不変、`project_grant` 呼出しの receiver は
  `ResolvedExecPolicy`）。
- `resolve`/`evaluate`/`viability`/`select` の policy source を
  `policy.exec` へ。
- **route eligibility** — route 対象は
  `policy.exec.is_route_eligible(grant)`（`scope==Trusted &&
  !fixed`）を満たす grant のみ。`fixed`・`scope:"project"`・
  未知名は `Viability::NoGrant`（`"route grant not found or
  denied"` + `details.grant` — 文言不変。route supply 層での
  `fixed`/`project` reject は不変 — 重複 reject なし）。
- `pin_exec_grant`（T2 tools/exec API）を viability 事前評価
  seam として呼ぶ（同一 predicate — pin-able でない grant は
  route 不能）。`route_context`/`argv_shell_re` の参照先を
  `cheaper_tools::tools::exec::{…}` ・
  `cheaper_core::policy::exec::argv_shell_re` へ更新。
- `project_grant(&mut policy.exec, name)` — `!fixed` route
  grant 本体 + 全 `fixed` grant を残す投影（design verbatim
  `retain`）。投影後の `exec` が `ExecToolOptions` へ渡る。
- viability 分類 pin（`Viability::{NoGrant,Denied,Pinned}`）・
  precedence・`reason`/`details` 語彙・`supply` 層 interface
  は不変。

### 2. `route/supply.rs`・`route/run.rs`・`route_cmd.rs` — 型追随

- `RouteSupply`/`run_route`/`route_cmd` の
  `CommandPolicyView`/`CommandToolOptions`/`policy.command`/
  `command_pins` 参照を `ExecPolicyView`/`ExecToolOptions`/
  `policy.exec`/`exec_pins` へ。
- `route_cmd` — route grant 注入用の grant object 構築は
  authoring shape（`command.grants` 型）を維持し、runtime
  options へ渡す前に `serde_json::to_value(&policy.exec)` 経路
  （route supply が resolved policy を保持する現行構造）へ。
- `--route`（route supply CLI 入口）で route 名解決は
  `!fixed` grant 前提（route visibility と一致）。

### 3. `hook/routing_policy.rs`・`workflow_context.rs`・`workflow_run.rs`

- `routing_policy` — hook route policy の policy source/
  grant lookup を `exec` へ追随（route 名 → `!fixed` grant）。
- `workflow_context` — worker spec → `ToolRuntimeOptions`
  構築で `options.exec = serde_json::to_value(&policy.exec)`
  注入（`process`/`command` 個別 field 除去）。`writeScope`/
  `deny`/`security` の既存注入規約は不変。
- `workflow_run` — `tool:"process"`/`tool:"command"` 実行の
  options 供給を `exec` 一本化へ追随。`tool:"process"` は
  canonical vocabulary のまま（`TOOL_ALIASES` 追加なし、
  workflow `tool` field・error 文言・`process` detail schema
  不変 — M5）。

### 4. `grant.rs`・`policy.rs` — CLI 入口の fixed/scope reject

- `--grant`/`--grant-file`/`policy grant add` の **grant spec
  object を受理する入口**で `fixed:true`・`scope` key を
  reject（`PolicyViolation`/`InvalidRequest` — CLI/
  invocation 供給 grant は常に trusted・`!fixed` — O7）。
  `grant.rs::build_invocation_layer` が組立てる invocation
  layer は authoring shape `{"version":1,"command":{"grants":
  {…}}}` のまま維持し、spec parse 段（name strip・pattern
  検査と同位置）で `fixed`/`scope` を reject する。
  `policy grant list`/`remove` は grant object を受理しない
  （name positional）ため reject 対象外。
- trusted policy YAML 内の `command.grants[].{fixed,scope}`
  宣言は引き続き authoring surface として受理（T1 schema
  拡張 — CLI reject は entry-point validation であって
  authoring schema 制約ではない）。
- `policy grant` の write_validated は `validate_policy_schema`
  経路（T1 unified resolve）を共有 — collision・declared
  deny・semantic 規則が同経路で発火。

### 5. `runtime/src/run/mod.rs`・`dev/fixture.rs`

- `run/mod.rs` — runtime options literal の `process:`/
  `command:` → `exec:`（`to_value(&policy.exec)`）。
- `dev/fixture.rs` — dev fixture の `RuntimePolicy` literal・
  `ToolRuntimeOptions` literal を `exec` 形状へ（dev-seam
  feature gate 配下 — `cargo test` には dev-seam flag 要）。

### 6. `cheaper-workflow` — resource inference・engine

- `resource_inference.rs` — tool resource 推論で
  `tool:"process"`/`tool:"command"` の grant 参照を `exec`
  型へ追随（`process.tasks` → `exec.grants[fixed]`・
  `command.grants` → `exec.grants[!fixed]`。`exec_grant_resources`
  経由の per-path/root fallback を共用 — ToolCatalog
  `resources` callback が `exec_grant_resources` 委譲へ変わる
  ため）。
- `engine.rs` — `ToolCtx` field・`find_cheaper_error`
  re-export path（`tools::process` 経由・維持）の型追随。
  `compiler.rs` は tool option/policy 型を参照しないため
  変更なし。
- `tool:"process"`/`tool:"command"` 名・workflow schema・error
  文言は不変（M5 canonical 互換）。

### 7. test churn（CLI/runtime/workflow/rig-adapter）

compile 境界（`cargo build --workspace --all-targets`）で
`RuntimePolicy`/`ToolRuntimeOptions`/`CommandPolicyView`/
`CommandToolOptions`/`ProcessToolOptions`/`policy.{process,
command}`/`command_pins` 参照の**完全な集合が compile error
として surface** する。下表は代表 file で、churn は機械的
literal 追随（`process:`/`command:` → `exec:`・`to_value(
&policy.exec)`）+ 新規 case 追加。

| file | 変更 |
|------|------|
| `cli/tests/integration/route_run.rs` | `CommandPolicyView`/`CommandToolOptions`/`policy.command` literal → `ExecPolicyView`/`ExecToolOptions`/`policy.exec`。route eligibility 新規 case（`fixed`→`NoGrant`・`scope:"project"`→`NoGrant`・`!fixed`+`trusted`→route 可）。`project_grant` 投影（route grant+全 fixed grant）pin |
| `cli/tests/integration/route.rs`・`run.rs` | options literal `exec:` 化 + `--grant` の `fixed:true`/`scope` reject case |
| `cli/tests/integration/policy.rs` | `policy grant add` の `fixed`/`scope` reject + authoring YAML の `fixed`/`scope` 受理（write_validated 経路） |
| `cli/tests/integration/args.rs`・`common/mod.rs` | `--grant` spec literal・helper の `exec` 化・reject case helper |
| `cli/tests/integration/{hook,hook_config,builtin_workers,usage_log}.rs`・`workflow.rs`・`{run,workflow,observability}_dev_seam.rs` | runtime options/policy literal の `exec:` 機械追随（compile error で残集合を確定） |
| `cli/tests/regression/{cases,expected}/run-grant-*.json` | `--grant` accept/collide/duplicate/invalid-spec の expected output を unified 経路の error/echo 形状へ再採取（fail-loud 文言・`details` 形状差分を反映） |
| `workflow/src/resource_inference.rs`（unit）・`tests/{process,engine,compile_contracts/*,filesystem_contracts/*}` | `tool:"process"`/`tool:"command"` の `exec` 型推論・per-path/root fallback・options literal `exec:` 化・`{$input}` root fallback・common helper |
| `runtime/tests/{run_dev_seam,run_fixture_test,ollama_e2e}.rs`・`dev/fixture` 消費側 | `exec` literal 化（dev-seam feature） |
| `rig-adapter/tests/adapter.rs` | `ToolRuntimeOptions` literal `exec:` 化（`src/tools.rs` は `find_cheaper_error` re-export のみ・変更不要） |

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/unified-exec-grant-model.md`
  （PLAN-034KQs6XKnAqahkSVke78h — T3）
- `docs/designs/unified-exec-grant-model.md`
  （DESIGN-034KP4lfgfD7DdGiDV1YuT — (e) route eligibility・
  (f) `project_grant`・(j) `ExecPolicyView`/route 層・(k)
  `--grant`/invocation parity・(l) 配線・(n) error 契約が
  AUTHORITATIVE）
- `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6 — R6/R7/R8/R9/R11）
- `docs/test-specs/unified-exec-grant-model.md`
  （TSPEC-034KQs1NIL8ahirht6QteV — route eligibility・
  `project_grant`・invocation parity・`--grant` reject・
  `tool:"process"` canonical・`{$input}` fallback Rules）
- `docs/tasks/plumb-unified-exec-through-tools.md`
  （TASK-034KQsmrlonTwNkmmLJOxg — 前提）

## Dependencies

- `docs/tasks/plumb-unified-exec-through-tools.md`（T2） —
  `tools/exec` API（`ExecToolOptions`・`pin_exec_grant`・
  `check_exec_execution`・`exec_grant_resources`）・
  `ToolCtx.exec`・`ToolRuntimeOptions.exec`・
  `InvocationToolRuntimeOptions.exec`・`ToolCatalog`
  型が必要。
- T1（`add-core-exec-policy-module-and-loader-wiring`）の
  transitive 前提（`policy.exec`/`is_route_eligible`/
  `project_grant`/`argv_shell_re`）。
- 既知の transient 解消: 本 task で workspace 全 crate が
  `exec` 一本化へ追随し、T1/T2 で発生した transient compile
  失敗が解消される。T5 までに `cargo build --workspace
  --all-targets`・`cargo test --workspace` が pass する
  状態を復帰する。

## Affected Files

- `crates/cheaper-cli/src/route/resolve.rs`（modify —
  `ExecPolicyView` 改名 + trusted/`!fixed` eligibility +
  `pin_exec_grant`/`exec_grant_resources`/`project_grant`
  seam）
- `crates/cheaper-cli/src/route/supply.rs`（modify）
- `crates/cheaper-cli/src/route/run.rs`（modify）
- `crates/cheaper-cli/src/route_cmd.rs`（modify）
- `crates/cheaper-cli/src/hook/routing_policy.rs`（modify）
- `crates/cheaper-cli/src/grant.rs`（modify — `--grant`/
  `--grant-file` の `fixed`/`scope` reject）
- `crates/cheaper-cli/src/policy.rs`（modify — `policy grant
  add`/`list`/`remove` の `fixed`/`scope` entry reject +
  write_validated 経路）
- `crates/cheaper-cli/src/workflow_context.rs`（modify —
  `options.exec` 注入）
- `crates/cheaper-cli/src/workflow_run.rs`（modify）
- `crates/cheaper-runtime/src/run/mod.rs`（modify）
- `crates/cheaper-runtime/src/dev/fixture.rs`（modify — dev-seam）
- `crates/cheaper-cli/tests/integration/route_run.rs`（modify）
- `crates/cheaper-cli/tests/integration/route.rs`（modify）
- `crates/cheaper-cli/tests/integration/run.rs`（modify）
- `crates/cheaper-cli/tests/integration/policy.rs`（modify）
- `crates/cheaper-cli/tests/integration/args.rs`（modify）
- `crates/cheaper-cli/tests/common/mod.rs`（modify）
- `crates/cheaper-cli/tests/integration/{hook,hook_config,
  builtin_workers,usage_log}.rs`・`tests/{workflow,
  run_dev_seam,workflow_dev_seam,observability_dev_seam}.rs`・
  `tests/regression/{cases,expected}/run-grant-*.json`
  （modify — compile error で残集合を確定する機械 literal
  追随 + grant regression fixture 再採取）
- `crates/cheaper-workflow/src/resource_inference.rs`（modify）
- `crates/cheaper-workflow/src/engine.rs`（modify —
  `compiler.rs` は型非参照で変更なし）
- `crates/cheaper-workflow/tests/process.rs`・`tests/engine.rs`・
  `tests/common/mod.rs`・`tests/compile_contracts/*`・
  `tests/filesystem_contracts/*`（modify — `exec` 型推論・
  options literal）
- `crates/cheaper-runtime/tests/run_dev_seam.rs`（modify）
- `crates/cheaper-runtime/tests/run_fixture_test.rs`（modify）
- `crates/cheaper-runtime/tests/ollama_e2e.rs`（modify）
- `crates/cheaper-rig-adapter/tests/adapter.rs`（modify —
  `src/tools.rs` は re-export のみで変更なし）

## Done When

- [ ] `ExecPolicyView` が存在し、route 解決が `policy.exec` +
      `is_route_eligible`（`scope==Trusted && !fixed`）前提で
      `fixed`/`scope:"project"`/未知名を `Viability::NoGrant`
      へ分類する（`pin_exec_grant` seam で viability 事前評価）
- [ ] `project_grant(&mut policy.exec, name)` が route grant +
      全 `fixed` grant を残す投影として `ExecToolOptions` へ
      渡る
- [ ] `route/supply.rs`・`route/run.rs`・`route_cmd.rs`・
      `hook/routing_policy.rs`・`workflow_context.rs`・
      `workflow_run.rs` が `ExecToolOptions`/`exec`/`exec_pins`
      型へ追随し、`workflow_context` が `options.exec =
      to_value(&policy.exec)` を注入する
- [ ] `--grant`/`--grant-file`/`policy grant add` が grant
      spec object の `fixed:true`・`scope` key を spec parse
      段で reject し、authoring policy.yaml の
      `command.grants[].{fixed,scope}` は `validate_policy_schema`
      経路で受理される（invocation layer は `command.grants`
      authoring shape を維持）
- [ ] `cheaper-runtime`（`run/mod.rs`・`dev/fixture.rs`）・
      `cheaper-workflow`（`resource_inference`/`engine`）・
      `cheaper-rig-adapter` の call site・test literal が
      `exec` 型へ追随し、`tool:"process"`/
      `tool:"command"` canonical 互換が保持される
- [ ] 全 test literal が `exec` 形状へ churn され、route
      eligibility・`project_grant`・invocation parity・
      `--grant` reject の新規 case が存在する
- [ ] workspace 全 crate が compile し（T1/T2 transient 解消）、
      `cargo test --workspace --features cheaper-runtime/
      dev-seam,cheaper-cli/dev-seam` が pass

## Out of Scope

- `policy/exec.rs`・`tools/exec` 本体の実装（T1/T2）
- docs（T4 — T1 後に並列可・本 task の compile 依存なし）
- route grant-set 投影 `grants[]`（#166 — `project_grant` の
  `{route}∪{fixed}` 投影は本 task、`grants[]` 複数 route grant
  化は別 issue）
- substitute exec mode（issue 5）・trusted flag 発行主体の
  多層化（issue 6）
- workflow `tool` field・`tool:"process"`/`tool:"command"` 名・
  error 文言・`TOOL_ALIASES` の変更（M5 不変）

## Verification

- [ ] `mise exec -- cargo build --workspace --all-targets
      --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
      exits 0（transient 解消の宣言境界）
- [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      integration route_run::` exits 0（route eligibility/
      `project_grant`/`ExecPolicyView` 回帰）
- [ ] `mise exec -- cargo clippy --workspace --all-targets
      --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
      exits 0
