---
id: "IMPL-034KThYLdekX1TvTeqCQhr"
type: "impl"
status: "completed"
title: "Unified exec grant model issue 165"
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
        path: "crates/cheaper-core/src/policy/security.rs"
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
        path: "crates/cheaper-core/tests/golden/policy_restrict.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_schema.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/catalog.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/mod.rs"
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
        path: "crates/cheaper-tools/tests/fixtures/process.json"
      - type: "file"
        path: "crates/cheaper-tools/tests/fixtures/runtime.json"
      - type: "file"
        path: "crates/cheaper-cli/src/grant.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route_cmd.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/supply.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_context.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
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
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/managed-process.md"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/guides/workflow-usage.md"
      - type: "file"
        path: "docs/specs/opt-in-exec-deny-patterns.md"
      - type: "file"
        path: "docs/tasks/remove-tools-content-audit.md"
    deleted: []
    renamed: []
    moved:
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/prepare.rs -> crates/cheaper-tools/src/tools/exec/prepare.rs"
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
    generated: []
  implements:
    - "docs/tasks/add-core-exec-policy-module-and-loader-wiring.md"
    - "docs/tasks/plumb-unified-exec-through-tools.md"
    - "docs/tasks/plumb-unified-exec-through-cli-and-runtime.md"
    - "docs/tasks/document-unified-exec-grant-model.md"
    - "docs/tasks/verify-unified-exec-grant-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/unified-exec-grant-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/unified-exec-grant-model-issue-165.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/unified-exec-grant-model.md"
    - "docs/designs/unified-exec-grant-model.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/unified-exec-grant-model.md"
  references:
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/designs/opt-in-exec-deny-patterns.md"
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/specs/managed-process-canonical-tool.md"
    - "docs/specs/policy-gated-command-tool.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Unified exec grant model issue 165

## Summary

issue #165(#151 実装分解 3/7)— `command.grants` と `process.tasks`
(「宣言済み executable の許可 + argv 契約(固定 or pattern) +
policy 管理」の同一概念の二重実装)を単一の exec grant model へ
統合した(ADR-034KOpPArDTHQ8W1c81Q0B — O1-O10 全採用・Option D
capability-grant 統合 + Merge Fidelity M1-M5)。authoring
surface(policy.yaml の `process`/`command` 節・`policy grant`
CLI・`--grant`・`tool:"process"` workflow)は不変で、統一対象は
resolved model と機構層のみ — `process.tasks` は loader が
`fixed:true` + `scope` provenance 付き grant へ写像し
`command.grants` と同じ単一 map へ畳む。`command`(`{name,args}`
→ `!fixed`)が canonical exec surface、`process`(`{task}` →
`fixed`)は canonical vocabulary に残す実装層 compat alias。

1. `cheaper-core`(T1): `policy/exec.rs` を新設 —
   `ExecGrant`({executable,fixed,argv?,inputs?,outputs?,cwd,env,
   lifecycle{timeout,killGrace,maxOutputBytes},exitCodes,scope}・
   serde-native declared-shape 保持)・`ExecGrantScope`
   (Trusted/Project)・`ResolvedExecPolicy`({grants:BTreeMap,
   command:ExecDefaults,process:ExecDefaults}・`is_route_eligible`・
   `project_grant`)・`ExecDefaults`・`ExecHeartbeat`・
   `exec_policy_properties`(exec-doc input schema)。3 段 resolver —
   段 1 section resolver(`resolve_process_policy` →
   `ResolvedProcessSection`・`resolve_command_policy` →
   `ResolvedCommandSection`・deny param 除去)・段 2
   `build_exec_input`(task→fixed grant synthesis + cross-section
   collision `InvalidRuntimePolicy` + `scope` provenance stamp)・
   段 3 `resolve_exec_policy`(exec-shaped doc boundary validate +
   lifecycle 畳み込み + deny 適用点 1 — serialized round-trip
   冪等で invocation parity の不変条件)。`RuntimePolicy.exec` を
   `process`/`command` field と置換、`load_policy` の merge 前
   `project_task_names` 捕捉、`validate_policy_schema` の unified
   経路一本化、`ArgPattern`/`argv_shell_re`/shared helper の移設、
   golden fixture 4 件再採取。
2. `cheaper-tools`(T2): `tools/exec/{mod,prepare}.rs` を新設 —
   `ExecToolOptions`・`pin_exec_grant(s)`(deny 適用点 2)・
   `verify_pinned_executable`(pin-equality + deny 適用点 3)・
   `check_exec_execution`・`prepare_exec_grant`(manifest
   normalize → declared-path deny → normalized claim deny →
   writeScope coverage)・`exec_operation_declaration`・
   `exec_grant_resources`・`resolve_exec_grant`(disjoint
   surface: command→`!fixed`・process→`fixed`、surface 別
   unknown 語彙)。`tools/command`・`tools/process` を thin
   surface 化、`tools/process/prepare.rs` を `tools/exec/
   prepare.rs` へ移設・汎化。`registry.rs`/`invocation.rs`/
   `catalog.rs` の options・gate・pin・parity・`ToolCtx` を
   `exec` 一本化 — pin-fail×degradable の対称規則(Command 側
   意図的拡張・R11-(e))・catalog `Command` arm の条件付き解決
   (literal 名のみ per-path・非 literal/未知名/fixed 一致は
   root manifest fallback・error 非伝播)。
   `differential.rs::runtime_options` に authoring spec →
   `build_exec_input` → `resolve_exec_policy` → `options.exec`
   の harness 変換 seam を追加(fixture json 不変)。
3. `cheaper-cli`/`cheaper-runtime`/`cheaper-workflow`/
   `rig-adapter`(T3): `CommandPolicyView` → `ExecPolicyView`
   (全 borrowed・`Copy` 維持)、route viability に
   `is_route_eligible`(scope==Trusted && !fixed →
   project/fixed/不在は `Viability::NoGrant`)、
   `route/run.rs` の `project_grant` → `{route.grant} ∪
   {fixed grants}` 投影、`exec_view()`/`RouteEvalInput.
   exec_policy` 改名、`runtime/run/mod.rs`・`workflow_run.rs`
   の options `exec:` 一本化(invocation parity は
   `invalid_binding("exec policy mismatch")` の 1 check 化)。
   `grant.rs::build_invocation_layer` と
   `policy.rs::parse_grant_spec` へ `fixed:true`/明示 `scope`
   の `InvalidRequest` entry reject(`8ea2071` — schema は受理
   するが CLI 契約は `!fixed`+`scope:"trusted"` 供給専用)。
4. docs(T4): `managed-process.md`・`command.md`・
   `configuration.md`・`usage.md`・`workflow-usage.md`・
   `workers.md`・`parent-agent-integration.md`・
   `resources/policy.yaml` を unified model・新 grant field・
   collision・deny task 適用・writeScope tri-state・
   pin-fail×degradable・route eligibility へ整合。
5. 検証(T5): targeted test 全 pass・`check-doc-ids` 1144 files
   OK・`audit_docs` 新規 file blocking finding 0・
   `mise run check` exit 0・plan/task/test-spec status 更新。

## Context

TASK-034KQsi5CcOH2MsHWhWWnC(T1)・TASK-034KQsmrlonTwNkmmLJOxg(T2)・
TASK-034KQsrpnzHdeRKf5M3KiN(T3)・TASK-034KQsvyJ7l63g3i4Gjnwl(T4)・
TASK-034KQsAwXBVQJ56nGgagYT(T5)。PLAN-034KQs6XKnAqahkSVke78h /
DESIGN-034KP4lfgfD7DdGiDV1YuT / SPEC-034KOpKfZpegX2MLPD0uq6 /
ADR-034KOpPArDTHQ8W1c81Q0B / TSPEC-034KQs1NIL8ahirht6QteV に従う。
統合対象の全接点(2 系統 schema/resolve・pin/verify/prepare・
manifest・op declaration・route・invocation parity・catalog・
CLI)の実測 inventory と issue 本文の line drift 訂正表は
DISC-034KNX4OP8iBFRKTREaigz(resolved — D1-D12 確定)で固定済み。
#163 が「宣言された実行契約のみ強制」の mandatory 層を確立し、
#164 が opt-in `security.execDenyPatterns` を 3 適用点で
`command.grants` のみに配管した — 本 issue はその deny を
task 由来 grant を含む unified grant 全体へ uniform 適用する
(O6a・#164 の task 除外を吸収)。本 IR は `derives-from`/
`implements`/`verified-by`/`depends-on` を chain 内 doc 集合で
閉じ、cross-chain doc(#151 design/spec・#163 design/ADR・
#164 design/spec/ADR/IR・canonical tool spec 群)は
`related`/`references` でのみ参照する — lineage edge は持たない。

## Implementation

### Changes

- `crates/cheaper-core/src/policy/exec.rs`: **新設**。単一 grant
  namespace・argv 契約・provenance の owner。
  `ExecGrant`(:53 — `executable`・`fixed`・`argv?`・`inputs?`・
  `outputs?`・`cwd`・`env`・畳み込み済み lifecycle
  {`timeout`/`kill_grace`/`max_output_bytes`}・`exit_codes`・
  `scope`、serde-native + `PartialEq` で parity check の
  declared-shape equality 対象)。`ExecGrantScope`(:42 —
  Trusted/Project、serde `"trusted"|"project"`)。
  `ExecDefaults`(:88 — origin section 別 defaults block、
  serialized shape `{maxConcurrent,timeout,maxOutputBytes,
  killGrace,heartbeat:{interval}}` は旧 2 section と同型)。
  `ExecHeartbeat`(:77 — 旧 `ProcessHeartbeat` 改名、serde shape
  不変)。`ResolvedExecPolicy`(:103 — `grants` 単一 map +
  `command`/`process` defaults、`is_route_eligible`:114・
  `project_grant`:123 の 2 method)。`ArgPattern`/`ArgToken`・
  `argv_shell_re`(:147)・`relative`/`str_vec`/
  `bounded_duration`/`resolve_env`・`MAX_PROCESS_OUTPUT_BYTES`/
  `MAX_TIMER_MS`/`DEFAULT_*` を `command.rs`/`process.rs` から
  移設。`exec_policy_properties`(:350 — exec-doc input
  schema、`grants` maxProperties 256・`additionalProperties:
  false`・defaults block は `grants`/`tasks` key 混入を reject
  — heartbeat nested object にも `additionalProperties:false`
  を再現して round-trip 不変条件を保全)。
  `build_exec_input`(:473 — 段 2: 両 section resolver 呼出し・
  `command.grants` ∩ `process.tasks` collision を
  `exec_invalid("grant name collides between command.grants and
  process.tasks")` + `details.{grant,task}` で fail-loud・
  task→`{fixed:true,argv:command[1..],exitCodes:[0],
  scope:project|trusted}` synthesis — lifecycle は synthesis で
  明示値畳み込み、provenance stamp は merge 前捕捉の
  `project_task_names` で決定)。`resolve_exec_policy`(:563 —
  段 3: exec-doc validator → defaults resolve(欠落 →
  `max_concurrent:1,timeout:"2m",max_output_bytes:65536,
  kill_grace:"2s",heartbeat:"30s"`)→ per-grant: 適用点 1 deny
  (`details.task` 併記 for fixed)・`!fixed` は `ArgPattern::
  compile`・cwd/env/relative 規則・lifecycle fallback
  (`fixed`→process block・`!fixed`→command block)・
  `exit_codes` 未宣言 `[0]`・`locale_compare` 整列)。
  `resolve_exec_grant`(:698 — surface disjoint 解決、
  `fixed` 期待値不一致を surface 別 unknown 語彙
  `"unknown or unauthorized command grant"`/`"unknown or
  unauthorized process task"` で `PolicyViolation`)。
  `exec_grant_resources`(:734 — `inputs`/`outputs` 宣言
  (`Some([])` 含む)→ per-path manifest、両方 `None` →
  conservative root manifest — 宣言は authoritative contract
  で underclaim 責任は宣言者側)。
- `crates/cheaper-core/src/policy/process.rs`・`command.rs`:
  `ResolvedProcessPolicy`→`ResolvedProcessSection`・
  `ResolvedCommandPolicy`→`ResolvedCommandSection` 改名(段 1
  resolver は authoring 境界 validation + defaults 供給の中間
  段として存続 — ADR O9a)。`resolve_process_policy` の
  `tasks` を `BTreeMap<String,ProcessTask>` typed 化、
  `resolve_command_policy` から `deny` param 除去(適用点 1 は
  段 3 へ一本化)。`command_policy_properties()` の grant spec
  へ `fixed`/`inputs`/`outputs`/`scope` optional field 追加
  (trusted 層が fixed argv grant・resource 宣言・自己弱体化
  scope を authoring 可 — R4)。`process_policy_properties()`
  不変。
- `crates/cheaper-core/src/policy/schema.rs`: `RuntimePolicy` の
  `process`/`command` field を除去し `workflow` 直後へ
  `pub exec: ResolvedExecPolicy`(serialized key 順
  `workflow` → `exec` → `route` → `security` → …)。
  `validate_policy_schema` の個別 re-resolve を
  `build_exec_input(process, command, &BTreeSet::new())` +
  `resolve_exec_policy(&exec_doc, &exec_deny)` へ置換 —
  collision・declared deny が全 mode で発火する入口(`policy
  grant add` の `write_validated` も同一経路)。
  `normalize_policy(raw, &project_task_names)` へ signature
  変更し unified resolve 結果を `RuntimePolicy.exec` へ格納。
- `crates/cheaper-core/src/policy/load.rs`: project merge の
  `validate_policy_schema(ProjectPartial)` 直後・
  `merge_policy_layers` 直前に validated project doc の
  `process.tasks` keys を `BTreeSet` へ捕捉(**merge 前捕捉が
  必須** — RestrictConcurrency deep-merge 後では provenance
  を導出不能)。typed re-resolve 段は serialized section
  shape の `with_defaults` re-embed を維持(deny 注入は除去)。
- `crates/cheaper-core/src/policy/capability.rs`・`restrict.rs`
  ・`security.rs`: `RuntimePolicy` literal を `exec:` 形状へ
  (`restrict_policy` 本体は `..policy.clone()` で無変更)、
  `security.rs` の doc comment を `CommandGrant` → `ExecGrant`
  へ追随。
- `crates/cheaper-core/tests/golden/`: `load.json`・
  `schema.json`・`process.json`・`restrict.json` の normalized
  `RuntimePolicy` 出力を `{"exec":{grants,command,process}}`
  形状へ再採取(`merge.json`/`workflow.json` は不変 — plan
  risk #11 対応確認済み)。driver `policy_process.rs` を
  `resolve_exec_grant`/`exec_grant_resources`/
  `resolve_exec_policy` の unified op 呼出しへ書き換え
  (fixture case の op 名は維持)。`policy_restrict.rs`・
  `policy_schema.rs` の `RuntimePolicy` literal 追随。
- `crates/cheaper-tools/src/tools/exec/mod.rs`: **新設**。
  `tools/command` の pin 機構を unified grant へ汎化集約。
  `ExecToolOptions`(:57 — `root`/`symlink_policy`/`exec`/
  `pins`/`on_timeout`/`denylist`/`exec_deny` 共有 field +
  `write_scope`(fixed surface のみ)・`route_context`
  (`!fixed` surface のみ)の surface 別 ownership)。
  `pin_exec_grants`(:139 — surface subset 複数版・registry
  が `fixed` filter で呼ぶ)・`pin_exec_grant`(:163 —
  `resolve_process_executable` + deny 適用点 2・route
  viability の事前評価も同一 predicate を呼ぶ)・
  `verify_pinned_executable`(:222 — pin-equality B1/B2 +
  deny 適用点 3、両 surface 共有)・`CheckedExec`(:258)・
  `check_exec_execution`(:275 — `check_command_execution`
  一般化: `!fixed` grant 解決 → argv pattern 照合 → cwd
  (`exec_cwd` override 込み)→ pin map + verify)・
  `exec_operation_declaration`(:352 — ClaimMode:
  `outputs:Some([])`→Shared・その他→Exclusive の tri-state・
  pool `{"process",process.max_concurrent}`/`{"command",
  command.max_concurrent}` origin 別・`declaration.resources`
  全 grant 設定)。`RouteCommandContext`・`grant_violation`・
  `authorize_args` も移設。
- `crates/cheaper-tools/src/tools/exec/prepare.rs`: **新設**
  (`tools/process/prepare.rs` 移設・汎化)。
  `prepare_exec_grant`(:68 — fixed grant の per-call 前処理:
  cwd 解決 → manifest normalize → declared-path deny →
  normalized claim deny → writeScope coverage(`outputs`
  tri-state: `Some(nonempty)` は `outputs ⊆ writeScope`・
  `Some([])` は空 manifest で trivially 成立・`None` かつ
  `write_scope:Some` は証明不能で `POLICY_VIOLATION`
  fail-closed)→ pin 検索 + verify → `PreparedExec`)。
  `options.write_scope`/`denylist` は `ExecToolOptions` field
  参照。`!fixed` grant は構築時 gate で writeScope 下到達不能
  のため unreachable(unreachable-state 分岐は残さない)。
- `crates/cheaper-tools/src/tools/command/mod.rs`・
  `tools/process/mod.rs`: thin surface 化 — pin/verify/check/
  authorize/`RouteCommandContext`/`CheckedExec`/
  `CommandToolOptions`/`ProcessToolOptions`/`prepare` を
  `tools/exec` へ移設し、surface 固有の input schema
  (`{name,args}`/`{task}` 不変)・details schema・description
  (surface subset のみ列挙)・`ProcIdentity`・`execute` glue
  のみ保持。`tools::process::find_cheaper_error`/`OnTimeout`/
  `StreamCaptureResult`/`MAX_PROCESS_OUTPUT_BYTES`・
  `tools::process::{executable,tree}` の re-export path 維持
  (`engine.rs`・rig-adapter が参照 — M5)。`tools/mod.rs` に
  `pub mod exec;` 追加。
- `crates/cheaper-tools/src/registry.rs`:
  `ToolRuntimeOptions` の `process`/`command` field を
  `exec: Option<Value>` 一本化(`security` 不変)。
  `create_tool_runtime` は `resolve_exec_deny_list` +
  `resolve_exec_policy(options.exec, &exec_deny)` 単一
  resolve。content gate を `names` 含 `Process` && 全 grant
  に `fixed` 無し / 含 `Command` && `!fixed` 無し →
  degrade/`POLICY_VIOLATION`(文言不変)。writeScope gate を
  `names` 含 `Command` && `!fixed` grant 非空 &&
  `write_scope` → `POLICY_VIOLATION "command is incompatible
  with worker writeScope"`(同文言 — M4 維持)。pin は surface
  別 union: `names` 含 `Command` → 全 `!fixed` grant、含
  `Process` → 全 `fixed` grant を `pin_exec_grants`
  (**task executable の構築時 pin は本 issue が初** —
  `prepare.rs` の lazy ambient PATH resolve を除去)。
  **pin-fail×degradable 対称規則**: pin 失敗 grant の
  surface(`fixed`/`!fixed`)に対応する tool が `degradable`
  宣言されていれば当該 tool のみ `names` から除去、
  無ければ構築 `POLICY_VIOLATION` — Command 側は
  degradable 宣言時に pin fail で縮退する**新規 semantics**
  (現行無条件 hard fail からの意図的拡張・R11-(e))。
  `ToolCtx` を `{exec:ResolvedExecPolicy, exec_pins:
  BTreeMap, exec_deny}` へ一本化、dispatch は
  `exec_operation_declaration` へ置換。
- `crates/cheaper-tools/src/invocation.rs`:
  `InvocationToolRuntimeOptions` の `process`/`command` →
  `exec: Option<Value>`、`InvocationInner` を `inner.exec:
  ResolvedExecPolicy` + `security`/`exec_deny` へ。parity
  check を 3 check → 2 check(exec/security)へ集約 —
  `options.runtime.exec` `Some` → `resolve_exec_policy` →
  `inner.exec` と `PartialEq`、不一致は
  `invalid_binding("exec policy mismatch")`。scope 再注入は
  `exec: Some(to_value(&inner.exec))`。
- `crates/cheaper-tools/src/catalog.rs`: `ToolCatalog.policy:
  ResolvedExecPolicy`、`from_resolved_policy(exec)`。
  `Process` arm — `input.task` → `resolve_exec_grant(fixed:
  true)` → `exec_grant_resources`(宣言 inputs/outputs →
  per-path・無ければ root fallback、error 伝播は現行規約
  不変)。`Command` arm — **`input.name` が literal string
  で `!fixed` grant へ resolve する場合のみ**
  `exec_grant_resources` へ委譲し、非 literal(`{$input}`
  等 binding 未解決)・未知名・`fixed` grant 一致は input
  非依存の **root manifest fallback**(error 非伝播の
  fail-safe 側 — P1-1 review 確定、Risks 参照)。
- `crates/cheaper-tools/tests/differential.rs`:
  `runtime_options`(:646-698 相当)に `spec.process`
  (authoring shape)→ `build_exec_input` →
  `resolve_exec_policy` → `to_value` → `options.exec` の
  harness 変換 seam を追加 — `tests/fixtures/process.json`/
  `runtime.json` の case spec は**不変**(synthesis 経路自体を
  fixture 全 case で回帰。`expectRust.state.operations.
  resourceClaims` 期待差分のみ追記 — op declaration が fixed
  grant へ `resources` を付与するため)。
- `crates/cheaper-cli/src/route/resolve.rs`:
  `CommandPolicyView` → `ExecPolicyView`(:68 — 全 field
  borrowed・`Copy` 維持、`policy: &ResolvedExecPolicy`)。
  viability に `is_route_eligible` を挿入(:314-320 —
  `scope=="project"`・`fixed`・不在は全て `Viability::
  NoGrant`、routing≠authorization)。`pin_command_grant` →
  `pin_exec_grant`・`CommandToolOptions` → `ExecToolOptions`
  (`exec:` field)・`check_command_execution` →
  `check_exec_execution`。
- `crates/cheaper-cli/src/route/supply.rs`: `command_view()` →
  `exec_view()`(:45 — `policy: &self.policy.exec`)、
  `RouteEvalInput.command_policy` → `exec_policy`。
- `crates/cheaper-cli/src/route/run.rs`: `supply.policy.exec.
  project_grant(&route.grant)`(:121 — `{route.grant} ∪
  {fixed grants}` 投影。routed workflow から task を呼べる
  現行挙動を保存しつつ他 `!fixed` grant を隠す C9
  confused-deputy 防止を維持)。`expected_input`(:176 —
  `{"name":grant,"args":exec_args}`)・grant timeout 参照は
  `policy.exec` へ(`expected_input` の byte 一致 anchor は
  不変)。
- `crates/cheaper-cli/src/route_cmd.rs`・`hook/routing_policy.rs`:
  `exec_view()`/`exec_policy` field 名の rename 追随 +
  test literal の unified resolver 経路更新。
- `crates/cheaper-cli/src/grant.rs`: `build_invocation_layer`
  (:86-104)に `--grant` spec の `fixed:true`・明示 `scope` を
  `InvalidRequest` で reject — schema は合法 field として受理
  するが、`--grant` は route-eligible な `!fixed` command
  grant 供給の CLI 契約であり、`fixed:true`(両 surface 不可視
  の inert grant)・`scope`(route 不可視)は利用者の意図と
  矛盾するため CLI 入口 validation で拒否(layer 非依存の
  schema では CLI 由来 spec を識別不能 — trusted YAML での
  直接 authoring は合法のまま)。module doc 追記。
- `crates/cheaper-cli/src/policy.rs`: `parse_grant_spec`
  (:205-217 — 既存 `name` key reject と同じ CLI 入口
  validation)に同じく `fixed:true`/`scope` の
  `InvalidRequest` reject を追加。`write_validated` の
  `validate_policy_schema(Partial)` が unified collision +
  deny を発火するため `policy grant add` 時点で
  `command.grants`×`process.tasks` 衝突が
  `InvalidRuntimePolicy` になる新規挙動(module doc 追記)。
- `crates/cheaper-runtime/src/run/mod.rs`: invocation/worker
  scope の `ToolRuntimeOptions`/`InvocationToolRuntimeOptions`
  を `process:`/`command:` → `exec: Some(to_value(&policy.
  exec))` 一本化(`security` 同型維持)。
- `crates/cheaper-cli/src/workflow_run.rs`・
  `workflow_context.rs`: `shared.policy.{process,command}` →
  `exec`+`security` の 2 serialize、`kind: tool` worker scope
  は `exec:None`/`security:None`(parity skip + inner 再注入
  — `command:None` と同型)。`ToolCatalog::from_resolved_
  policy(policy.exec.clone())`。`tool:"process"` の
  `ToolName::from_canonical`/`tools.allow` admission・
  `ports.tools.get("process")`・`named_task` 検査は canonical
  名で**不変**(M5)。
- `crates/cheaper-runtime/src/dev/fixture.rs`・
  `tests/run_dev_seam.rs`: `RuntimePolicy` literal `exec:`
  形状、`p.process.tasks.insert` → `p.exec.grants.insert`
  (fixed grant)への test 置換。
- `crates/cheaper-cli/resources/policy.yaml`: `security` 節
  comment へ deny が task executable にも及ぶ旨を補足
  (authoring surface 不変のため節構成は無改訂)。
- `docs/guides/managed-process.md`: task→`fixed:true` grant
  写像・構築時 pin(unresolvable → 構築 fail/opt-in
  degradable 時は tool 縮退)・deny の task 適用拡大・
  cross-section collision を記述。`command.md`: 新 optional
  field `fixed`/`inputs`/`outputs`/`scope`・disjoint
  surface・collision・deny uniform を追記。
  `configuration.md`: policy.yaml reference へ新 field・
  loader 写像(project task → `scope:"project"`)・collision・
  deny 拡大を追記。`usage.md`: `policy grant` CLI 節へ
  collision・`fixed`/`scope` reject を追記。
  `workflow-usage.md`: `tool:"process"` compat alias 不変を
  明記。`workers.md`: writeScope 下 `outputs ⊆ writeScope`
  の fixed grant のみ exec 可・`!fixed` は構築時
  `POLICY_VIOLATION`・degradable 宣言 exec surface tool の
  pin 失敗時縮退(両 surface)を明記。
  `parent-agent-integration.md`: route `grant:` の
  eligibility(`scope:"trusted" && !fixed`)を追記。
- test churn + 新規 pin: core(exec.rs unit・schema/load/
  collision/provenance/round-trip/parity/deny 適用点 1)、
  tools(exec unit・proc_command/proc_process 適用点 2/3・
  writeScope tri-state・pin-fail×degradable・disjoint
  surface・parity・harness seam 経由全 case)、cli
  (`policy.rs` collision/`fixed`/`scope` reject、
  `run.rs`/`route_run.rs`/`run_dev_seam.rs` 回帰)、
  workflow(`resource_inference.rs` の catalog fallback pin・
  options literal)、rig-adapter/runtime literal churn。

### Decisions During Implementation

- **3 段 resolver の責務分離** — 「authoring 境界 validation」
  (段 1 section resolver 存続・`"invalid process policy:"`/
  `"invalid command policy:"` の attribution prefix 維持)・
  「namespace 統合」(段 2 `build_exec_input` の synthesis +
  collision + provenance)・「exec-shaped 境界 validation」
  (段 3 `resolve_exec_policy` — serialized resolved shape と
  同一 input shape、round-trip 冪等で parity 不変条件)を分離。
  authoring section を段 3 へ direct 投入する誤用は
  `additionalProperties:false` + defaults block への
  `grants`/`tasks` 混入 reject で fail-loud に構造防止(段 2
  経由を強制)。全入口(load・`validate_policy_schema`・
  invocation parity・`policy grant` write・runtime options)
  が同一経路を通る。
- **provenance は load 経路固有 param 1 本** —
  `project_task_names` は `merge_policy_layers` **前**に
  validated project doc から捕捉(RestrictConcurrency
  deep-merge 後では shadow 元を導出不能)。layer 非依存の
  `validate_policy_schema` には到達させず(∅ で呼び per-doc
  collision のみ検査)、`normalize_policy` への channel は
  本 param のみ — validation 成否へ影響しない(scope stamp
  は `details.task` attribution の診断性向上と #166 投影
  seam の基盤であり、route guard/権限強化機構ではない)。
- **lifecycle/pool の意図的非対称** — `command.grants` で
  `fixed:true` を authoring した grant は per-grant lifecycle
  を **command section defaults** で畳み込み(authoring
  origin)、op declaration の pool/heartbeat は `fixed` flag
  で **process block** を引く(`{"process",policy.process.
  max_concurrent}`)。「lifecycle は宣言位置の既定値、pool は
  reach surface の並行性 budget」の分離は一貫(単一化すると
  budget 意味が変わる — D11)。
- **review 反映(P1-1)catalog `Command` arm の条件付き解決** —
  `input.name` が literal string で `!fixed` grant へ resolve
  する場合のみ `exec_grant_resources` へ委譲、それ以外
  (binding 未解決の非 string 名・未知名・`fixed` 一致)は
  input 非依存の root manifest fallback で error を伝播
  しない。根拠: (a) `create_tool_catalog(None)` 空 catalog
  + literal 名で root manifest を期待する現行契約
  (`resource_inference.rs:496-586`)を保存、(b) `resources(
  &job.input)?` の error 伝播は `{$input}` 使用 workflow を
  破壊し得る、(c) literal 未知名でも root manifest は
  sound な overclaim(fail-safe)— 実 authorization は実行時
  `resolve_exec_grant` が担保し、inference 層の
  `POLICY_VIOLATION` は false positive を生むだけ。Process
  arm との非対称(error 伝播)は意図的 — `named_task` 検査が
  literal 名を既に要求しているため現行規約不変。
- **review 反映(P1-2)pin-fail×degradable の対称規則** —
  現行 command は pin 失敗を常に hard fail(`registry.rs` の
  `?`)。本実装は surface 対称の uniform 規則: pin 失敗
  grant の surface に対応する tool が `degradable` 宣言
  されれば当該 tool のみ `names` から除去(opt-in 縮退の
  既存機構を再利用)、無ければ `POLICY_VIOLATION` で構築
  fail。Command 側の「degradable 宣言時に pin fail で縮退」
  は**意図的 semantics 拡張** — spec R7/AC/R11-(e) 破壊的
  変更として明文化・登録済み(非 opt-in 経路の挙動は不変、
  migration guide は issue 7 対象)。
- **`--grant`/`policy grant add` の `fixed`/`scope` entry
  reject** — schema は `fixed`/`inputs`/`outputs`/`scope` を
  trusted 層 field として受理するが、`build_invocation_layer`
  (`--grant`)と `parse_grant_spec`(`policy grant add`)の両
  CLI 入口で `fixed:true`・明示 `scope` を `InvalidRequest`
  で拒否。schema validate は layer 非依存で CLI 由来 spec を
  識別不能のため拒否点は schema ではなく各 CLI 入口 —
  `fixed:true` は両 surface 不可視の inert grant、
  `scope:"project"` は route 不可視で誤解を招く。trusted
  YAML での直接 authoring は合法のまま。
- **fixture 方針 — harness 変換 seam で json 不変** —
  `differential.rs::runtime_options` に authoring spec →
  `build_exec_input` → `resolve_exec_policy` → `options.exec`
  の変換を harness 内へ追加し、`process.json`/`runtime.json`
  の ~60 spec block を exec 形状へ rewrite する案は不採用
  (機械的 churn + authoring 経路 coverage 喪失。synthesis
  経路自体を fixture 全 case で回帰させ、`options.exec` の
  exec-shaped 境界契約を壊さず production
  `to_value(&policy.exec)` と同じ材料を供給)。fixed grant
  への `declaration.resources` 付与による期待差分のみ
  `expectRust.state.operations.resourceClaims` で追記。
- **deny の uniform 3 適用点(task 由来含む)** — #164 が
  `command.grants` のみに配管した opt-in deny を unified
  grant 全体へ拡大: 適用点 1(`resolve_exec_policy` —
  declared basename、fixed grant は `details.task` 併記)・
  適用点 2(`pin_exec_grant` — resolved basename)・適用点 3
  (`verify_pinned_executable` — spawn 前再検証)。denied
  project task executable は policy load fail(意図的強化 —
  project task が deny executable を走らせる hole を塞ぐ、
  ADR O6a)。merged deny error の `details.layer` 無しは
  #164 と同構造の accepted gap。
- **semantic guarantee の保存** — pin-equality(spawn 前
  `verify_pinned_executable` の canonical path 同一性 +
  deny 再適用)・symlink policy・`PathDenylist`・
  `env_clear` + `stdin(Stdio::null)`(`tools/proc` spawn
  不変)・lifecycle 畳み込み(task → `exitCodes:[0]`・
  `!fixed` → command defaults・`fixed` → process defaults)・
  fs boundary(execution の fs 接地面は `tools/proc` 共有層
  不変)・`tool:"process"` canonical 名 compat(M5 —
  `TOOL_ALIASES` 非採用、O5c)・route viability 不成立 →
  `Viability::NoGrant` 畳み込み(error 化しない分類維持)。
- **T1→T2→T3 の compile transient は scoped verification** —
  T1 が core signature を、T2 が tools options/pin を、
  T3 が cli/runtime/workspace を変えるため各 task を単一
  crate test で閉じ T3 で workspace compile を回復(#163/
  #164 と同じ既知の直列依存処理 — design で予告済み)。

## Related Experiments

### Adopted

なし(known-solution task。design が統一型 API・3 段 resolver
手順・exec-doc schema・signature 契約・options 配管・disjoint
surface・deny/writeScope error 文言・pin-fail×degradable 規則・
catalog fallback 条件・churn 一覧・docs 契約を file:line 精度で
固定済みのため Experiment Log は未作成)。

### Rejected

なし。

## Validation

targeted cargo test(2026-10-23 実測、全 pass / 0 failed —
plan Verification・T5 task 記録):

- `mise exec -- cargo test -p cheaper-core`: exit 0 — 244 pass
  (exec.rs/process.rs/command.rs/schema.rs/load.rs unit test +
  golden target — load/schema/restrict/process.json 再採取・
  merge/workflow 不変確認。T1・`057d22b`)
- `mise exec -- cargo test -p cheaper-tools`: exit 0 — 全 target
  green(exec/mod.rs・exec/prepare.rs unit・proc_*
  integration・characterization・differential — harness seam
  経由の全 case。T2・`85571c7`。**並列負荷下で
  `proc_process.rs::node_task_resolves_via_path` が 1 度だけ
  helper exit 255 の transient fail** — isolated rerun で
  pass、process-tree cleanup の既知 flake であり unified
  policy の regression ではない)
- `mise exec -- cargo test --workspace --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam`: exit 0 —
  全 50 test target pass(cli integration/regression/
  run_dev_seam/workflow/workflow_dev_seam・runtime
  contracts/run_dev_seam・workflow compile_contracts/
  filesystem_contracts・rig-adapter を含む — T3、`eceaa6e`
  + `8ea2071`)
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
  integration route_run::`: exit 0 — 28 tests(route
  eligibility/`project_grant` 投影/`ExecPolicyView` 回帰)
- spec Acceptance Criteria 全項目に対応する test が存在し
  pass(TSPEC Rules ↔ 統一 schema/`fixed` 意味論・exec-doc
  fail-loud・`build_exec_input` provenance+collision・
  round-trip 冪等・disjoint surface・deny 3 適用点・
  manifest/writeScope tri-state・pin-fail×degradable・
  catalog fallback・route eligibility/`project_grant`・
  `tool:"process"` compat・invocation parity・CLI reject・
  serialized 形状・harness seam・docs — 上記全 target +
  `mise run check` の workspace test で実測)(T5)
- `node scripts/check-doc-ids.mjs`: exit 0(1144 markdown
  files scanned — T5)
- `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all --json`: exit 0 — 本 plan 群の新規 file で
  blocking finding 0(`tools/exec/prepare.rs` 移設に伴う
  frontmatter path を plan・T2 task・test-spec・
  `remove-tools-content-audit.md` で修正済み。repo 全体の
  pre-existing reciprocal-relation warning 677 件は残置 —
  issue #169 sweep 対象)
- `mise run check`: exit 0(biome・`cargo fmt --check`・
  clippy `--all-targets` + dev-seam・version/doc-id check・
  `cargo test --workspace`・`node --test`・lockfile・APM
  payload 全 stage pass — T5 実測)

証跡 commit: planning chain `cce7bb2`(discovery seed)→
`5725ca7`/`1c00476`(briefing — discovery resolved 昇格 +
spec/ADR proposed)→ `69055cd`(design)→ `4e75fc9`(design
review 指摘反映 — adversarial P1×2 修正: catalog `Command`
arm fallback・pin-fail×degradable 対称 + cooperative P1×6
反映)→ `fc28446`(design approved 昇格)→ `8b7b8fe`(plan +
test-spec + tasks)→ 実装 `057d22b`(T1 core)→ `85571c7`
(T2 tools)→ `eceaa6e` + `8ea2071`(T3 cli/runtime +
`--grant`/`policy grant` entry reject)→ `912987f`(T4 docs)→
`e72a253`(T5 検証記録 + plan/task/test-spec status 更新)。
本 IR はその後に追加する。

## Risks

- **cross-section collision(新規 fail-loud)** —
  `command.grants` と `process.tasks` の同名は現行は別
  namespace で共存可能だったが単一 map 化で
  `InvalidRuntimePolicy`(load・`validate_policy_schema`・
  `policy grant add` 全入口で同一 failure)。
  `details.{grant,task}` で両宣言位置を attribution。
- **cross-layer collision の fail 面** — merged 後の単一
  map で評価されるため trusted `command.grants.x` ×
  project `process.tasks.x` の cross-layer 同名でも load
  fail し得る(silent shadow を許さない意図的 fail-loud —
  `details` が echo するのは衝突名のみで trusted 側宣言
  内容は漏れない accepted surface)。
- **deny の task 拡大適用(意図的強化)** — opt-in
  `execDenyPatterns` 宣言 operator は denied task
  executable で policy load fail し得る(#164 の task
  除外を吸収 — deny の効果が exec surface 全体に及ぶ
  operator 契約と一致)。
- **task pin の fail-closed 化** — per-call ambient PATH
  解決から構築時 pin へ。unresolvable task は per-call
  error → 構築 error/degrade へ変わる回帰的強化(B1/B2
  との整合・pin 機構単一化として受容 — degradable 宣言
  時は縮退)。pin 後の executable 差替え・session 中の
  ambient PATH 変化は spawn 前 `verify_pinned_executable`
  で `POLICY_VIOLATION` fail-closed。
- **scheduler PerResource demotion 相互作用** — fixed
  grant への per-path `declaration.resources` 付与は新規
  であり、`scheduler.rs` の PerResource mode(
  `resource_scoped || resources.is_some()` で snapshot
  `Shared` demotion)との相互作用で PerResource mode 下の
  fixed grant op が従来の `Exclusive` → `Shared` へ
  demote される可能性 — design の実装時検証 flag として
  `proc_*`/scheduler test で意図しない並行度変化を確認
  する回帰観点として残置(事前確定仕様ではない)。
- **declared `inputs`/`outputs` は authoritative contract**
  — `!fixed` grant の宣言は op declaration/catalog claim
  を root manifest から narrowing するが、argv 供給で宣言
  外 path へ write する分岐を runtime が bound する機構で
  はない(#163 declared-contract 規約 — underclaim の責任は
  宣言者側)。
- **serialized shape churn** — `RuntimePolicy` の
  `process`/`command` → `exec` は内部 wire の破壊的変更
  (R11-(d))。golden fixture・options field・
  `InvocationInner`・route view・catalog が一括 churn
  済み — 外部 authoring surface は不変。
- **merged 評価の attribution gap** — collision/merged
  deny error は `details.layer` を持たず denied 元層
  (builtin/user)の帰属が取れない(#164 と同構造の
  accepted gap — test pin で誤修正を防止)。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #165)由来の未分類・未処理の follow-up はない。下記は
全て spec「Out of Scope」・design「Deferred Design Concerns」・
各 task「Out of Scope」で scope 外確定・記録済みの defer、または
本 issue 由来でない観測の記録であり、新規 chain は起こさない。

- **route grant-set `grants[]` 投影** — issue #166(#151 分解
  4)。本 issue の `is_route_eligible`/`project_grant` が投影
  seam だが set 投影自体は #166 の scope。
- **`substitute.grant` の fixed grant 参照** — route
  eligibility の `!fixed` 制約で NoGrant になるため問題化
  しないが、固定 argv への substitute は issue 5(substitute
  exec mode)側の設計論点。
- **trusted flag 発行主体** — issue 6 の scope。
- **`policy exec` への CLI 改名** — 現行 `policy grant` の
  用語維持で十分(authoring 破壊的変更を本 issue に持ち込ま
  ない)。用語統一需要が発生した場合に再着手(#169 sweep
  論点として残置)。
- **`process`/`command` section heartbeat の単一化** — origin
  別保存(D11)で維持。単一 heartbeat への統一は pool
  budget 意味が変わるため対象外。
- **段 1 resolver(`resolve_process_policy`/
  `resolve_command_policy`)の最終帰結** — ADR O9a で存続
  決定。将来 authoring shape が変わる変更が入った場合に
  統合先を再検討。
- **repo-wide reciprocal-relation warning(677 件)** —
  `audit_docs --type all` の pre-existing warning(#165 新規
  file では error 0)。docs 全体の正直性 sweep と合わせて
  issue #169 へ defer。
- **triage note — `proc_process.rs::node_task_resolves_via_
  path`** が 2026-10-23 の `cargo test -p cheaper-tools`
  実測で並列負荷下 1 度だけ helper exit 255 の transient
  fail(isolated rerun + `mise run check` 内の再実行では
  pass)。process-tree cleanup の既知 flake であり unified
  policy 非依存・本 issue 由来の regression ではない —
  `followup-terminal` 候補(別途 flake 調査 issue の起票は
  所有者判断)。
