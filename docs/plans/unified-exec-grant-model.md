---
id: "PLAN-034KQs6XKnAqahkSVke78h"
type: "plan"
status: "completed"
title: "unified exec grant model — command.grants / process.tasks の統合実装"
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
        path: "crates/cheaper-tools/src/tools/process/tests.rs"
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
        path: "crates/cheaper-cli/src/grant.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route_cmd.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_context.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/supply.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
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
        path: "crates/cheaper-workflow/tests/process.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs"
      - type: "file"
        path: "crates/cheaper-workflow/tests/filesystem_contracts/resource_inference.rs"
      - type: "file"
        path: "docs/guides/managed-process.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/guides/workflow-usage.md"
      - type: "file"
        path: "docs/guides/workers.md"
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
  implements:
    - "docs/specs/unified-exec-grant-model.md"
  implemented-by:
    - "docs/tasks/add-core-exec-policy-module-and-loader-wiring.md"
    - "docs/tasks/plumb-unified-exec-through-tools.md"
    - "docs/tasks/plumb-unified-exec-through-cli-and-runtime.md"
    - "docs/tasks/document-unified-exec-grant-model.md"
    - "docs/tasks/verify-unified-exec-grant-model.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/unified-exec-grant-model-issue-165.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/designs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/discovery/unified-exec-grant-model-issue-165.md"
  derived-by:
    - "docs/tasks/add-core-exec-policy-module-and-loader-wiring.md"
    - "docs/tasks/plumb-unified-exec-through-tools.md"
    - "docs/tasks/plumb-unified-exec-through-cli-and-runtime.md"
    - "docs/tasks/document-unified-exec-grant-model.md"
    - "docs/tasks/verify-unified-exec-grant-model.md"
  verifies: []
  verified-by:
    - "docs/test-specs/unified-exec-grant-model.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/designs/managed-process-canonical-tool.md"
    - "docs/designs/policy-gated-command-tool.md"
    - "docs/designs/invocation-scoped-command-grants.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/specs/managed-process-canonical-tool.md"
    - "docs/specs/policy-gated-command-tool.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-core/src/policy/process.rs"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/load.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/tools/process/mod.rs"
    - "crates/cheaper-tools/src/tools/exec/prepare.rs"
    - "crates/cheaper-tools/src/registry.rs"
    - "crates/cheaper-tools/src/invocation.rs"
    - "crates/cheaper-tools/src/catalog.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/grant.rs"
    - "crates/cheaper-cli/src/policy.rs"
  defers: []
  deferred-by: []
---

# unified exec grant model — command.grants / process.tasks の統合実装 Implementation Plan

## Goal

DESIGN-034KP4lfgfD7DdGiDV1YuT の実装契約に従い、`command.grants`
と `process.tasks` の二重実装を単一の exec grant model へ統合する
— 新 module `policy/exec.rs`（`ExecGrant`/`ResolvedExecPolicy`/
`ExecDefaults`/`ExecHeartbeat` + `ArgPattern`/shared helpers 移設）
と 3 段 resolver（section resolvers → `build_exec_input` →
`resolve_exec_policy`）で `RuntimePolicy.exec` を構築し、
`tools/exec/` の unified pin/verify/check/prepare 機構へ
`command`（`{name,args}`→`!fixed`）・`process`（`{task}`→`fixed`）
の disjoint thin surface を接続する。`security.execDenyPatterns`
を全 grant へ 3 適用点 uniform 適用（task 由来含む）、route は
`scope=="trusted" && !fixed` eligibility + `{route.grant} ∪
{fixed grants}` 投影、authoring surface（policy.yaml 2 節・
`policy grant` CLI・`--grant`・`tool:"process"` workflow）は
不変。

## Scope

単一の implementation stream。「統一 resolved model + 3 段
resolver + disjoint surface + unified 機構層」という 1 つの
behavioral 変更に、serialized shape churn（options/literal/
test/golden fixture/route view）が横断的に付随する構成であり、
API 形状・error 契約・適用点・churn 一覧は design が file:line
精度で固定済み — 分割するのは commit の論理境界（crate 層 +
docs + 検証）のみで、plan は 1 つに収める。

| In | Out |
|---|---|
| 新 module `crates/cheaper-core/src/policy/exec.rs` — `ExecGrant`/`ExecGrantScope`/`ExecDefaults`/`ExecHeartbeat`/`ResolvedExecPolicy`（`is_route_eligible`/`project_grant`）・`resolve_exec_policy`・`build_exec_input`・`resolve_exec_grant`・`exec_grant_resources`・`exec_policy_properties` + `ArgPattern`/`argv_shell_re`/shared helper/const 移設 | `process`/`command` policy.yaml 節の名称・shape 変更（authoring 面は不変 — `command.grants` への 4 optional field 追加のみ） |
| section resolver の section 型化（`ResolvedProcessSection`/`ResolvedCommandSection` — authoring 境界 validation + defaults 供給の中間段）と `resolve_command_policy` の `deny` param 除去（適用点 1 は段 3 へ一本化） | `policy grant` CLI・`--grant` flag・route `grant` 用語の改名 |
| `load_policy` project task 名捕捉（merge 前）+ `normalize_policy` provenance 配管 + `validate_policy_schema` unified collision/deny 接続（全入口同一経路） | route grant-set 投影（`grants[]`、#166）・substitute exec mode（issue 5）・trusted flag 発行主体（issue 6）・viability 条件付き弱化（issue 4） |
| `RuntimePolicy.exec` 新設（`process`/`command` field 除去）と options/`ToolCtx`/`ToolCatalog`/route view/serialize の `exec` 一本化 | `process` tool の `TOOL_ALIASES` 化（ADR O5c 却下 — canonical 名維持）・tool 数・capability 体系の変更 |
| `tools/exec/` 新設 — `pin_exec_grant(s)`/`verify_pinned_executable`/`check_exec_execution`/`prepare_exec_grant`/`exec_operation_declaration`/`ExecToolOptions` + `tools/command`・`tools/process` thin surface 化 | `declaration.resources` の PerResource demotion 挙動変化の最終調整（実装時検証 flag — 意図しない並行度変化は test で検出） |
| writeScope 構築 gate 維持 + per-call coverage tri-state の unified prepare 移設（M4）、pin-fail×degradable 両 surface 対称（R11-(e) 意図的拡張） | 文書全体の正直性 sweep（issue #169）・migration guide 本文（issue 7） |
| route 層 `ExecPolicyView`・`is_route_eligible`・`project_grant` 一般化・`policy.exec` 参照 + CLI 入口 `fixed`/`scope` reject | |
| test・golden fixture・docs の移行（benchmarks 無改訂回帰） | |

## Upstream Documents

- `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6、approved — R1-R11・Acceptance
  Criteria・破壊的変更一覧 R11-(a)-(e)）
- `docs/adr/unified-exec-grant-model.md`
  （ADR-034KOpPArDTHQ8W1c81Q0B、accepted — O1a+O2a+O3a+O4a+
  O5a+O6a+O7a+O8a+O9a+O10a 全採用: 単一 grants map・`fixed`/
  `scope` flags・O4a loader 写像・disjoint surface・deny
  uniform・route eligibility・`{route.grant}∪{fixed}` 投影・
  legacy 型置換・authoring 不変 migration）
- `docs/discovery/unified-exec-grant-model-issue-165.md`
  （DISC-034KNX4OP8iBFRKTREaigz、resolved — D1-D12 確定 +
  実測 inventory + line drift 訂正表の ground truth）

## Design Inputs

- `docs/designs/unified-exec-grant-model.md`
  （DESIGN-034KP4lfgfD7DdGiDV1YuT、approved — AUTHORITATIVE な
  HOW: (a) 統一型 verbatim rust・(b) 3 段 resolver 確定手順・
  (c) authoring schema 拡張 + CLI 入口 reject・(d) exec-doc
  input schema verbatim json・(e) validate/load 配線表・(f)
  deny 3 適用点表・(g) `create_tool_runtime` 段別置換表 +
  pin-fail×degradable 確定規則・(h) `tools/exec` API 形状 +
  manifest/writeScope tri-state + op declaration + lifecycle/pool
  意図的非対称 + scheduler 相互作用 flag・(i) disjoint surface
  対照表・(j) route 層置換表・(k) invocation 配管表・(l)
  signature 契約表・(m) call site 配線全一覧・(n) error 契約表・
  Churn 一覧・Docs 更新契約・Verification Notes）。
  本 plan・task は design の行指定を転記するだけで、API 形状・
  error 文言・適用位置を再解釈しない。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-core/src/policy/exec.rs` | **新設** — `ExecGrantScope`・`ExecGrant`・`ExecHeartbeat`・`ExecDefaults`・`ResolvedExecPolicy`（`is_route_eligible`/`project_grant`）・`resolve_exec_policy`・`build_exec_input`・`resolve_exec_grant`・`exec_grant_resources`・`exec_policy_properties`/`exec_defaults_properties`・`exec_invalid` + `ArgPattern`/`ArgToken`/`argv_shell_re`/`bounded_duration`/`relative`/`str_vec`/`DEFAULT_*`/`MAX_PROCESS_OUTPUT_BYTES`/`MAX_TIMER_MS` 移設 | T1。単一 grant namespace・argv 契約・provenance の owner |
| `crates/cheaper-core/src/policy/mod.rs` | `pub mod exec;` を module 一覧へ追加 | T1 |
| `crates/cheaper-core/src/policy/process.rs` | `ResolvedProcessPolicy`→`ResolvedProcessSection` 改名・`tasks` を `BTreeMap<String,ProcessTask>` typed 化・`resolve_process_task`/`process_task_resources`/helper/const の exec.rs 移設・`process_policy_properties()` 不変・unit test 移行 | T1。`ProcessTask` は authoring decl 型として残存 |
| `crates/cheaper-core/src/policy/command.rs` | `command_policy_properties()`(:86-128) へ `fixed`/`inputs`/`outputs`/`scope` 4 field 追加・`ResolvedCommandPolicy`→`ResolvedCommandSection`・**`deny` param 除去**（適用点 1 は段 3 移動）・`CommandGrant`/`resolve_command_grant`/`command_grant_resources`/`project_grant`/`ArgPattern` の exec.rs 移設・unit test 移行 | T1 |
| `crates/cheaper-core/src/policy/schema.rs` | `RuntimePolicy`(:129-149) — `process`/`command` 除去・`workflow` 直後に `exec: ResolvedExecPolicy`。`validate_policy_schema`(:437-446) — 個別再 resolve を `build_exec_input(.., &BTreeSet::new())` + `resolve_exec_policy(&doc, &exec_deny)` へ置換（`resolve_exec_deny_list` 無条件 compile 維持）。`normalize_policy`(:516-639) — `project_task_names` param 追加 + `build_exec_input` → `resolve_exec_policy` → `exec` 格納。import 更新 | T1 |
| `crates/cheaper-core/src/policy/load.rs` | :237-256 — ProjectPartial validate 直後・`merge_policy_layers` 直前に `project_task_names` を `BTreeSet` 捕捉（**merge 前捕捉必須**）。:198-220 — section re-embed 維持（`resolve_command_policy` deny param 除去の呼出し追随）。:257-258 — `normalize_policy(&completed, &project_task_names)`。:120-189 invocation guard 不変 | T1 |
| `crates/cheaper-core/src/policy/{capability,restrict}.rs` | `RuntimePolicy` literal を `exec:` 形状へ（`restrict_policy` 本体は `..policy.clone()` で無変更） | T1 |
| `crates/cheaper-core/tests/golden/fixtures/policy/{load,schema,restrict,process}.json` | normalized `RuntimePolicy` 出力の `{"exec":{grants,command,process}}` 形状へ再採取 + `fixed`/`scope`/collision case 追加（`merge.json`/`workflow.json` は不変確認） | T1 |
| `crates/cheaper-core/tests/golden/policy_process.rs` | driver の `resolve_process_task`/`process_task_resources`/`resolve_process_policy` 呼出しを `resolve_exec_grant`/`exec_grant_resources`/`resolve_exec_policy` へ写像（fixture case の op 名は維持） | T1 |
| `crates/cheaper-tools/src/tools/exec/mod.rs` | **新設** — `ExecToolOptions`・`resolve_exec_grant`（surface 別 error 語彙内蔵）・`pin_exec_grant`/`pin_exec_grants`・`verify_pinned_executable`・`check_exec_execution`/`CheckedExec`・`exec_operation_declaration` + `authorize_args`/`RouteCommandContext`/`grant_violation` 移設 | T2。deny 適用点 2 は `pin_exec_grant`、適用点 3 は `verify_pinned_executable`（pin-equality 直後） |
| `crates/cheaper-tools/src/tools/exec/prepare.rs` | **新設** — `prepare_exec_grant`（現 `process/prepare.rs` の grant 化: cwd 解決 → manifest normalize → declared-path deny → normalized claim deny → writeScope coverage tri-state → pin + verify → `PreparedExec`） | T2 |
| `crates/cheaper-tools/src/tools/{command,process}/mod.rs` | thin surface 化 — schema/details/description/`ProcIdentity`/re-export path は surface 別保持、内部は unified exec 機構へ接続（`command_tool_definition(ExecToolOptions)`/`process_tool_definition(ExecToolOptions)`） | T2 |
| `crates/cheaper-tools/src/tools/process/{prepare.rs,tests.rs}` | `prepare.rs` → `tools/exec/prepare.rs` 移設・`tests.rs` の `process_operation_declaration`/`resolve_process_policy`/`ProcessToolOptions` → unified 版 | T2 |
| `crates/cheaper-tools/src/registry.rs` | `ToolRuntimeOptions.{process,command}` → `exec: Option<Value>`（:97-108）・`create_tool_runtime`(:1095-1238) — `resolve_exec_policy` 単一 resolve・content gate（Process→`any(fixed)`/Command→`any(!fixed)`）・writeScope gate・surface subset pin + degradable 縮退・`ToolCatalog::from_resolved_policy(exec)`・`ToolCtx.{exec,exec_pins,exec_deny}`・dispatch(:878-897) `exec_operation_declaration`・tool import rename | T2 |
| `crates/cheaper-tools/src/invocation.rs` | `InvocationToolRuntimeOptions.{process,command}` → `exec`（:62-82）・`InvocationInner.exec`・parity 3 check → 2 check（`invalid_binding("exec policy mismatch")`・inner code 保持）・scope 再注入 `exec: Some(to_value(&inner.exec))`・`create_invocation_tool_runtime`(:423-436) 単一 resolve | T2 |
| `crates/cheaper-tools/src/catalog.rs` | `ToolCatalog.policy: ResolvedExecPolicy`・`from_resolved_policy(exec)`・Process arm → `resolve_exec_grant(fixed:true)` + `exec_grant_resources`（error 伝播維持）・Command arm → literal 名 + `!fixed` resolve のみ `exec_grant_resources`、非 literal/未知/fixed 一致は root manifest fallback（error 非伝播） | T2 |
| `crates/cheaper-tools/tests/{proc_command,proc_process,proc_invocation,proc_runtime,proc_degrade,characterization,differential}.rs` | options literal `exec:` 化 + `ResolvedExecPolicy` literal + unified resolver 呼出し追随 + disjoint surface・deny・pin-fail×degradable・writeScope tri-state・parity の新規 test | T2。`differential.rs::runtime_options`(:646-698) に `spec.process` → `build_exec_input` → `resolve_exec_policy` → `to_value` → `options.exec` の **harness conversion seam**（fixture json 不変） |
| `crates/cheaper-cli/src/route/resolve.rs` | `CommandPolicyView` → `ExecPolicyView`（`policy: &ResolvedExecPolicy`）・viability の `is_route_eligible` 検査 + `pin_exec_grant`/`ExecToolOptions`/`check_exec_execution`・`argv_shell_re` import を `policy::exec` へ・test literal | T3 |
| `crates/cheaper-cli/src/route/{supply,run}.rs` | `command_view()` → `exec_view()`（`policy: &self.policy.exec`）・`RouteEvalInput.command_policy` → `exec_policy`・`project_grant(&route.grant)` 投影一般化・grant timeout `policy.exec.grants`（`expected_input` 不変） | T3 |
| `crates/cheaper-cli/src/{route_cmd,hook/routing_policy}.rs` | `exec_view()`/`exec_policy` rename 追随 + test literal の `resolve_command_policy` → `build_exec_input`+`resolve_exec_policy` 経由 | T3 |
| `crates/cheaper-cli/src/{grant,policy}.rs` | `build_invocation_layer`(:49-104) と `parse_grant_spec`(:189-205) に `fixed`/`scope` key reject（`InvalidRequest`）+ `write_validated` module doc へ unified collision 追記 + module doc 更新 | T3 |
| `crates/cheaper-cli/src/workflow_{run,context}.rs` | `shared.policy.{process,command,security}` → `exec`+`security` 2 serialize・`process:None`/`command:None` → `exec:None`（parity skip + 再注入同型）・`ToolCatalog::from_resolved_policy(policy.exec.clone())`（`tool:"process"` admission 不変） | T3 |
| `crates/cheaper-runtime/src/{run/mod.rs,dev/fixture.rs}` + `tests/{run_dev_seam,ollama_e2e}.rs` | options `exec:` 一本化・`RuntimePolicy` literal `exec:` 形状・`p.process.tasks.insert` → `p.exec.grants.insert` | T3 |
| `crates/cheaper-{workflow,rig-adapter}/tests/*` | `ToolRuntimeOptions`/`InvocationToolRuntimeOptions` literal `exec:` 化 + `resource_inference.rs` に Command arm 条件付き解決 pin case 追加（cheaper-workflow src 不変 — M5） | T3 |
| `crates/cheaper-cli/tests/integration/{policy,route_run,run}.rs` | collision `InvalidRuntimePolicy` + `parse_grant_spec`/`--grant` の `fixed`/`scope` reject + route run 投影の新規 case | T3 |
| `docs/guides/{managed-process,command,configuration,usage,workflow-usage,workers}.md` | design Docs 更新契約（task→fixed grant 写像・構築時 pin・deny task 適用・collision・新 field・writeScope/degradable 規則・`tool:"process"` compat） | T4。T1 の semantics 確定後に T2/T3 と並列可能。`workers.md` は既に部分更新済み — 整合確認から着手 |
| `crates/cheaper-cli/resources/policy.yaml` | 節構成不変（authoring 不変 — 無改訂が正しい状態）。`security` 節 comment の task 適用補足は任意 | T4 |

## Implementation Sequence

- [x] TASK-034KQsi5CcOH2MsHWhWWnC (T1): cheaper-core の
      `policy/exec.rs` 新設 + section resolver 型化
      （`process.rs`/`command.rs` — `deny` param 除去・
      `command_policy_properties` 4 field 追加・helper/const/
      `ArgPattern` 移設）+ `schema.rs`（`RuntimePolicy.exec`・
      `validate_policy_schema` unified 経路・`normalize_policy`
      provenance）+ `load.rs`（merge 前 `project_task_names`
      捕捉）+ `capability.rs`/`restrict.rs` literal + core unit
      test + golden fixture 4 件再採取 + `policy_process.rs`
      driver 移行。`cargo test -p cheaper-core` で閉じる
- [x] TASK-034KQsmrlonTwNkmmLJOxg (T2): cheaper-tools の
      `tools/exec/` 新設（pin/verify/check/prepare/declaration/
      options 統一）+ `tools/command`・`tools/process` thin 化 +
      `registry.rs`（options/gate/pin/degradable/`ToolCtx`/
      dispatch）+ `invocation.rs`（options/parity 2 check/再注入）
      + `catalog.rs`（2 arm）+ tools test churn + differential
      harness seam + 新規 test。`cargo test -p cheaper-tools` で
      閉じる
- [x] TASK-034KQsrpnzHdeRKf5M3KiN (T3): cheaper-cli/runtime
      配管 — `route/resolve.rs`（`ExecPolicyView`+eligibility）・
      `route/supply.rs`（`exec_view`）・`route/run.rs`
      （`project_grant` 投影）・`route_cmd.rs`・
      `hook/routing_policy.rs`・`grant.rs`/`policy.rs` の
      `fixed`/`scope` reject・`workflow_run.rs`/
      `workflow_context.rs`・`runtime/run/mod.rs`・
      `dev/fixture.rs` + runtime/workflow/rig-adapter/cli
      integration test churn。workspace compile 回復 + cli test
      target で閉じる
- [x] TASK-034KQsvyJ7l63g3i4Gjnwl (T4): docs —
      `managed-process.md`・`command.md`・`configuration.md`・
      `usage.md`・`workflow-usage.md`・`workers.md`（部分更新
      済み — 整合確認）・`resources/policy.yaml` comment。T1 後
      なら T2/T3 と並列実行可能
- [x] TASK-034KQsAwXBVQJ56nGgagYT (T5): 最終検証 — targeted
      cargo tests・`node scripts/check-doc-ids.mjs`・
      `audit_docs`・`mise run check`・status 更新

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 core exec + schema/loader | — | T2, T4 |
| T2 tools unified 機構 | T1 | T3 |
| T3 cli/runtime 配管 | T2 | T5 |
| T4 docs | T1 | T5 |
| T5 最終検証 | T3, T4 | — |

critical path は T1 → T2 → T3 → T5。T4 は T1 完了後に T2/T3 と
並列実行可能（docs は compile 依存を持たない）。

既知の transient compile 断:

- T1 境界 — `resolve_command_policy` の `deny` param 除去・
  `ResolvedProcessPolicy`/`ResolvedCommandPolicy` の
  section 型改名・`RuntimePolicy` field 変更・
  `resolve_process_task`/`process_task_resources`/
  `resolve_command_grant`/`command_grant_resources`/`ArgPattern`/
  `argv_shell_re` の移設で cheaper-tools（`registry.rs`・
  `invocation.rs`・`catalog.rs`・`tools/command`・
  `tools/process`）・cheaper-cli（`route/resolve.rs`・
  `route/supply.rs`・`route/run.rs`・`hook/routing_policy.rs`・
  `workflow_context.rs`）・cheaper-runtime（`dev/fixture.rs`）・
  cheaper-workflow/rig-adapter test が compile 不可。T1 の検証は
  `cargo test -p cheaper-core` に scope。
- T2 境界 — `pin_command_grant`/`verify_pinned_executable`/
  `check_command_execution`/`CommandToolOptions`/
  `ProcessToolOptions`/`ToolCtx`/`ToolRuntimeOptions`/
  `InvocationToolRuntimeOptions`/`ToolCatalog` の unified 置換で
  cheaper-cli（`route/resolve.rs`・`route/supply.rs`・
  `workflow_run.rs`・`workflow_context.rs`）・cheaper-runtime
  （`run/mod.rs`）・workflow/rig-adapter test literal が compile
  不可。T2 の検証は `cargo test -p cheaper-tools` に scope し、
  T3 が cli/runtime call site を追随させて workspace compile を
  回復する。

各 task は 1 commit で、file-local の unit test・doc comment は
同 commit で追随する。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | T1/T2 commit 境界で downstream crate が compile 不可（signature/type churn — 必発の transient） | high(必発) | low | 既知の transient — 各 task の検証を単一 crate test に scope し直列実行。T3 で workspace compile 回復後に cli test を実測 |
| 2 | `project_task_names` の merge 後捕捉・`validate_policy_schema` への layer 情報流出 — provenance 誤 stamp または layer 依存 validation | med | high | design 確定の merge 前捕捉（ProjectPartial validate 直後・merge 直前）+ `validate_policy_schema` は ∅ names — TSPEC の provenance Rule が pin |
| 3 | exec-doc boundary の緩和 — `additionalProperties:false`・nested `heartbeat` 検査の脱落で authoring 形 direct 投入・unknown key 混入を受理し round-trip/parity が破れる | med | high | design (d) の verbatim schema（nested `additionalProperties:false` 再現の実装注意明記）+ exec-doc fail-loud・冪等性 Rule の pin |
| 4 | fixed grant の command surface・route 参照への漏洩 — argv 供給を受けない grant が `args`/exec_args で実行され semantics が汚れる | med | high | disjoint surface（`resolve_exec_grant` の surface 別 error）+ `is_route_eligible` + schema `additionalProperties:false` の 3 層 pin |
| 5 | cross-section collision の fail 面 — project layer 単体では valid な policy が trusted grant 名と衝突して load fail（意図的 fail-loud） | high(必発・受理済み) | low | ADR/design 受理済みの新規制約 — `details.{grant,task}` の衝突名 echo（trusted 宣言内容は漏れない）を pin・docs 明記 |
| 6 | deny の task 拡大適用による load fail — opt-in `execDenyPatterns` を持つ operator の denied task executable で policy load 失敗（意図的強化 R11-(b)） | med | med | ADR O6a 確定の意図的強化 — `details.task` 併記 attribution + managed-process.md/configuration.md 明記 |
| 7 | task pin の fail-closed 化 — per-call ambient PATH 解決から構築時 pin へ（unresolvable task が per-call error → 構築 error/degrade の回帰的強化 R11-(c)） | med | med | B1/B2 整合として design 受容 — pin-fail×degradable 両 surface 対称規則を `proc_degrade.rs` で pin（Command 側は R11-(e) の意図的拡張） |
| 8 | pin-fail×degradable の Command 側拡張が非 opt-in 経路へ漏れる — degradable 未宣言の pin fail が縮退へ変わる回帰 | low | med | 「degradable 宣言時のみ」の条件分岐を pin — 非 opt-in 経路は現行無条件 hard fail 不変 |
| 9 | catalog `Command` arm の error 伝播 — `{$input}` 非 literal 名で `POLICY_VIOLATION` が `resources()` から伝播し workflow を破壊 | med | high | design Risks 確定の「literal + `!fixed` resolve のみ `exec_grant_resources`、それ以外 root manifest fallback（error 非伝播）」— `resource_inference.rs` pin + `:496-586` 回帰維持 |
| 10 | scheduler PerResource demotion 相互作用 — fixed grant への per-path `declaration.resources` で `Exclusive` → `Shared` demote の可能性 | low | med | design (h) 実装時検証 flag — `proc_*`・scheduler test で意図しない並行度変化を確認（回帰観点として記録済み） |
| 11 | golden fixture 更新漏れ — `RuntimePolicy` serialize の `exec` key で load/schema/restrict/process の expected が stale | high(必発) | low | churn 一覧に file 網羅済み — `cargo test -p cheaper-core` の golden target が即検出（`merge.json`/`workflow.json` は不変確認） |
| 12 | data-loss / breaking-change | — | — | authoring surface 不変のため既存 policy file・workflow・benchmark は無改訂で動く。破壊的変更は R11-(a)-(e) の 5 項目に限定（collision・deny task 適用・task pin・serialized 形状・Command degradable）— 全て意図的で migration guide（issue 7）対象 |
| 13 | external dependency | — | — | 新規 crate 依存ゼロ（`regex`・`serde_json` は workspace dep 済み — design 明記） |

Rollback strategy: 全変更は git commit 単位の通常 revert で戻せる。
不可逆 step（migration・外部 format 固定）は無い — authoring
surface が不変のため既存 policy file は revert 後も valid。

## Task Breakdown

- T1 `docs/tasks/add-core-exec-policy-module-and-loader-wiring.md`
  （`TASK-034KQsi5CcOH2MsHWhWWnC`） — `policy/exec.rs` 新設・
  `mod.rs`/`process.rs`/`command.rs`/`schema.rs`/`load.rs`/
  `capability.rs`/`restrict.rs` 配線・golden fixture 4 件・
  `policy_process.rs` driver・core unit test。
  verified by `docs/test-specs/unified-exec-grant-model.md`
- T2 `docs/tasks/plumb-unified-exec-through-tools.md`
  （`TASK-034KQsmrlonTwNkmmLJOxg`） — `tools/exec/` 新設・
  `tools/command`/`tools/process` thin 化・`registry.rs`・
  `invocation.rs`・`catalog.rs` 配管・tools test churn +
  differential harness seam + 新規 test。
  verified by `docs/test-specs/unified-exec-grant-model.md`
- T3 `docs/tasks/plumb-unified-exec-through-cli-and-runtime.md`
  （`TASK-034KQsrpnzHdeRKf5M3KiN`） — `route/resolve.rs`・
  `route/supply.rs`・`route/run.rs`・`route_cmd.rs`・
  `hook/routing_policy.rs`・`grant.rs`・`policy.rs`・
  `workflow_run.rs`・`workflow_context.rs`・`runtime/run/mod.rs`・
  `dev/fixture.rs` + runtime/workflow/rig-adapter/cli
  integration test churn。
  verified by `docs/test-specs/unified-exec-grant-model.md`
- T4 `docs/tasks/document-unified-exec-grant-model.md`
  （`TASK-034KQsvyJ7l63g3i4Gjnwl`） — `managed-process.md`・
  `command.md`・`configuration.md`・`usage.md`・
  `workflow-usage.md`・`workers.md`・`resources/policy.yaml`。
  verified by `docs/test-specs/unified-exec-grant-model.md`
- T5 `docs/tasks/verify-unified-exec-grant-model.md`
  （`TASK-034KQsAwXBVQJ56nGgagYT`） — targeted tests・doc-id・
  `audit_docs`・`mise run check`・status 更新。

## Verification

全項目を 2026-10-23 に実測済み。実装 commit:
T1 = `057d22b`・T2 = `85571c7`・T3 = `eceaa6e` +
`8ea2071`（CLI entry reject）・T4 = `912987f`・
T5 = 本 status/evidence 更新 commit。

- [x] `mise exec -- cargo test -p cheaper-core` exits 0 —
      exec.rs/process.rs/command.rs/schema.rs/load.rs unit test +
      golden target（load/schema/restrict/process.json 再採取 +
      merge/workflow 不変）全 pass（244 passed — T1、`057d22b`）
- [x] `mise exec -- cargo test -p cheaper-tools` exits 0 —
      exec/mod.rs・exec/prepare.rs unit・proc_* integration・
      characterization・differential（harness seam 経由の全 case）
      pass（T2、`85571c7`。全 target green。並列負荷下で
      `proc_process.rs::node_task_resolves_via_path` が 1 度だけ
      helper exit 255 の transient fail を起こしたが isolated
      rerun で pass — process-tree cleanup の flake であり
      unified policy の regression ではない）
- [x] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0 —
      全 50 test target pass（cli integration/regression/
      run_dev_seam/workflow/workflow_dev_seam・runtime contracts/
      run_dev_seam・workflow compile_contracts/
      filesystem_contracts・rig-adapter を含む — T3、`eceaa6e`
      + `8ea2071`）
- [x] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      integration route_run::` exits 0 — 28 tests pass（route
      eligibility/`project_grant` 投影の回帰 — T3/T5）
- [x] spec Acceptance Criteria 全項目に対応する test が存在し pass
      （TSPEC の Rules — 統一 schema/`fixed` 意味論・exec-doc
      fail-loud・build_exec_input provenance+collision・round-trip
      冪等・disjoint surface・deny 3 適用点・manifest/writeScope
      tri-state・pin-fail×degradable・catalog fallback・route
      eligibility/`project_grant`・`tool:"process"` compat・
      invocation parity・CLI reject・serialized 形状・harness
      seam・docs — 上記全 target + `mise run check` の workspace
      test で実測）（T5）
- [x] `node scripts/check-doc-ids.mjs` exits 0（1144 markdown
      files scanned — T5）
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js
      --type all --json` exits 0 — 本 plan 群の新規 file で
      blocking finding 0（T5。`tools/exec/prepare.rs` 移設に
      伴う frontmatter path 3 file + `remove-tools-content-
      audit.md` を修正済み。repo 全体の pre-existing
      reciprocal-relation warning は残置 — issue #169 sweep
      対象）
- [x] `mise run check` exits 0 — biome・cargo fmt・cargo clippy
      （dev-seam features）・version/doc-id check・workspace test・
      node test・lockfile・APM payload 全 stage pass（T5）

## Open Questions / Gaps

- なし — 統一型 API・3 段 resolver 手順・exec-doc schema・
  signature 契約・options 配管・disjoint surface・deny/writeScope
  error 文言・pin-fail×degradable 規則・catalog fallback 条件・
  Churn 一覧・docs 契約の全 site は design が file:line 精度で
  固定済み。実装者の判断余地は新規 test/case の命名と restate 後
  の docs 文言のみ。
- merged deny・collision error の `details.layer` 無しは design
  accepted gap — `load.rs` の既存 attribution gap と同構造と
  して pin・docs 明記するが、layer provenance の後付けは行わ
  ない。
- scheduler PerResource demotion と fixed grant resources の
  相互作用は design の実装時検証 flag — 意図しない並行度変化を
  `proc_*`/scheduler test で確認する回帰観点として残置
  （事前確定仕様ではない）。

## Self-Review

- [x] Every spec requirement maps to a task.（R1/R2/R3/R5 → T1
      schema+resolver、R4 → T1 load.rs provenance + T3 CLI reject、
      R6 → T2 disjoint surface + T3 `tool:"process"` compat 確認、
      R7/R8 → T2 pin/prepare/writeScope、R9 → T3 route 層、R10 →
      T1 適用点 1 + T2 適用点 2/3、R11 → T1-T3 全体の型置換 +
      churn、AC → T5 対応確認）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are
      consistent（file:line は design の実測値を転記 — branch
      `feature/issue-165-unified-exec-grant-model` 現行 code
      基準、discovery drift 訂正表適用済み）。

## Implementation Handoff

- Recommended mode: subagent-capable implementation
- Independent work streams: T4（docs）は T1 完了後に T2/T3 と並列
  実行可能。T1→T2→T3→T5 は直列（compile 依存）。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute
  task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- 前提: design（DESIGN-034KP4lfgfD7DdGiDV1YuT）は approved 済み
  — 本 plan は機械的分解であり再審査不要。
- reviewer の焦点: T1/T2 の transient compile 境界（task の
  verification scope が単一 crate test に限定されること）、
  `project_task_names` の merge 前捕捉と `validate_policy_schema`
  の ∅ names（layer 非依存性）、exec-doc boundary の
  `additionalProperties:false` + nested `heartbeat` 再現、
  catalog `Command` arm の error 非伝播 fallback、pin-fail×
  degradable の Command 側意図的拡張（R11-(e)）、merged error
  の `details.layer` 無し pin。
