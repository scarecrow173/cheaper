---
id: "TASK-034KQsmrlonTwNkmmLJOxg"
type: "task"
status: "done"
title: "cheaper-tools tools/exec module と registry/invocation/catalog 配管"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-tools/src/tools/exec/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/exec/prepare.rs"
    modified:
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
    deleted: []
    renamed: []
    moved:
      - type: "item"
        path: "crates/cheaper-tools/src/tools/command/mod.rs::{pin_command_grant(s),verify_pinned_executable,check_command_execution,authorize_args,RouteCommandContext}"
        note: "tools/exec/mod.rs へ移設・汎化"
      - type: "item"
        path: "crates/cheaper-tools/src/tools/process/prepare.rs"
        note: "tools/exec/prepare.rs へ移設・汎化"
    generated: []
  implements:
    - "docs/plans/unified-exec-grant-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/unified-exec-grant-model.md"
    - "docs/tasks/add-core-exec-policy-module-and-loader-wiring.md"
  blocks:
    - "docs/tasks/plumb-unified-exec-through-cli-and-runtime.md"
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
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/tools/process/mod.rs"
    - "crates/cheaper-tools/src/tools/exec/prepare.rs"
    - "crates/cheaper-tools/src/registry.rs"
    - "crates/cheaper-tools/src/invocation.rs"
    - "crates/cheaper-tools/src/catalog.rs"
  defers: []
  deferred-by: []
---

# cheaper-tools tools/exec module と registry/invocation/catalog 配管

## Work

plan `docs/plans/unified-exec-grant-model.md`
（PLAN-034KQs6XKnAqahkSVke78h）の T2。T1 が追加した
`ExecGrant`/`ResolvedExecPolicy`/`resolve_exec_policy`/
`resolve_exec_grant`/`exec_grant_resources` へ cheaper-tools を
配管する — `tools/exec/` の unified pin/verify/check/prepare/
declaration 機構を新設し、`tools/command`・`tools/process` を
thin surface 化し、`registry.rs`・`invocation.rs`・
`catalog.rs` を `exec` 一本化する。design
（DESIGN-034KP4lfgfD7DdGiDV1YuT）の (g)(h)(i)(k)(l) 契約・
call site 配線・error 契約をそのまま実装する — 再解釈しない。

### 1. `tools/exec/mod.rs`（新設）

design (h) の API 形状通りに集約する（現 `tools/command` の
pin 機構 + `tools/process` の shared 責務を汎化移設）:

- `pub struct ExecToolOptions` — `{root, symlink_policy, exec:
  ResolvedExecPolicy, pins: BTreeMap<String,String>, on_timeout,
  denylist, exec_deny, write_scope: Option<HashSet<String>>（fixed
  surface のみ読む）, route_context: Option<RouteCommandContext>
  （!fixed surface のみ読む）}` — 共有機構 field は 1 本に集約
  （2 struct 維持しない — false abstraction 回避）。
- `pub fn pin_exec_grant(name, grant: &ExecGrant, root,
  symlink_policy, denylist, exec_deny: &ExecDenyList) ->
  Result<String, ToolError>` — `resolve_process_executable` +
  **deny 適用点 2**: `exec_deny.is_denied(&resolved)` →
  `grant_violation(name, "resolved executable is denied by
  security.execDenyPatterns")`（`PolicyViolation` +
  `details.grant`）。`route::resolve` の viability 事前評価も
  同一 predicate を呼ぶ seam。
- `pub(crate) fn pin_exec_grants(policy: &ResolvedExecPolicy,
  fixed: bool, root, symlink_policy, denylist, exec_deny) ->
  Result<BTreeMap<String,String>, ToolError>` — `fixed` filter
  で surface subset の grant 群のみ pin（registry が surface 別
  union で呼ぶ）。
- `fn verify_pinned_executable(name, pinned, exec_deny:
  &ExecDenyList) -> Result<(), ToolError>` — **適用点 3**:
  pin-equality（`resolved != pinned`）直後に同一 deny predicate
  を再適用（B1/B2 + uniform 契約）。
- `pub fn check_exec_execution(options: &ExecToolOptions, name,
  args: &[String]) -> Result<CheckedExec, ToolError>` —
  `check_command_execution`（:402-439）の一般化（route 共有
  seam B#3）— `!fixed` grant 解決 → argv pattern 照合 → cwd
  （route `exec_cwd` override 込み）→ pin map 検索 + verify。
  `command::execute` と `route::resolve` が呼ぶ。
- `pub fn exec_operation_declaration(policy, name, fixed) ->
  Result<OperationDeclaration, CheaperError>` —
  `process_operation_declaration`(:103-118) +
  `command_operation_declaration`(:125-144) の置換。ClaimMode:
  `outputs:Some([])` → `Shared`、その他（`Some(nonempty)`・
  `None`）→ `Exclusive`。pool: `fixed` → `{"process",
  policy.process.max_concurrent}`、`!fixed` → `{"command",
  policy.command.max_concurrent}`（R3/D11 budget 保存）。
  `declaration.resources` は全 grant で設定（宣言 inputs/
  outputs → per-path、無ければ root manifest — fixed grant への
  resources 付与は新規だが prepare が同じ manifest を claim
  するため一貫）。
- 移設: `authorize_args`/`RouteCommandContext`/`CheckedCommand→
  CheckedExec`/`grant_violation`/`verify_pinned_executable` →
  `tools/exec/mod.rs`。

### 2. `tools/exec/prepare.rs`（新設）

現 `tools/process/prepare.rs`（:53-138）の責務を grant 化して
移設 — `pub(super) fn prepare_exec_grant(options:
&ExecToolOptions, name, grant: &ExecGrant) -> Result<
PreparedExec, ToolError>`:

- cwd 解決 → （`inputs`/`outputs` 宣言 grant のみ）manifest
  normalize → declared-path deny → normalized claim deny →
  writeScope coverage（`write_scope` 存在時、`outputs ⊆
  writeScope` でないと `POLICY_VIOLATION` `"process outputs
  exceed worker writeScope"` — 文言不変）→ pin 検索 +
  `verify_pinned_executable` → `PreparedExec{executable, cwd}`。
- **writeScope tri-state（M4 確定分岐）**: `outputs:Some(
  nonempty)` → manifest normalize + `claim_contains` 検査。
  `outputs:Some([])` → 空 manifest = 副作用非宣言契約で
  coverage trivially 成立・exec 可。`outputs:None` →
  `options.write_scope` が `Some` なら証明不能で
  `POLICY_VIOLATION`（fail-closed — C5）、`None` なら
  coverage 検査不要で通常実行。
- per-call lazy executable resolve（現 `prepare.rs:127-136`）は
  **除去** — task executable は scope 構築時 pin 済み（D9）。
- `!fixed` grant は構築時 gate で writeScope 下に到達不能
  （防御分岐は残さない — unreachable state）。
- `options.write_scope`/`denylist` は `ExecToolOptions` field
  参照へ。

### 3. `tools/command/mod.rs`・`tools/process/mod.rs` — thin surface 化

design (i) の対照表通り:

- pin/verify/check/authorize/`RouteCommandContext`/
  `CheckedExec`/`grant_violation`/`CommandToolOptions`/
  `ProcessToolOptions` を `tools/exec` へ移設。module は
  schema・description・details schema・`ProcIdentity`・
  definition builder・`execute` glue のみ所有。
- `command` surface — `{name,args}` input schema
  （`command_schema():51-70` 不変）・`resolve_exec_grant(
  policy, input.name, fixed:false)`・`check_exec_execution` →
  `ProcPrepared{argv: args}` → `proc::run`（identity:
  `{grant,argv}`・`"command {name}"`・`report_duration_ms:
  true`）。route_context 注入可（現行不変）。
- `process` surface — `{task}` input schema
  （`process_schema():51-66` 不変）・`resolve_exec_grant(
  policy, input.task, fixed:true)`・`prepare_exec_grant` →
  `ProcPrepared{argv: grant.argv（fixed literal）}` →
  `proc::run`（identity: `{task}`・`"process {name}"`・
  `report_duration_ms:false`）。route_context 注入不可
  （`None` 固定）。
- `process/mod.rs` — `TOOL_ALIASES` 追加なし（O5c）、canonical
  `ToolName::Process`・`process_schema`・details schema・
  `find_cheaper_error`/`OnTimeout`/`StreamCaptureResult`/
  `MAX_PROCESS_OUTPUT_BYTES` re-export path（:40-45）を全て
  維持 — `MAX_PROCESS_OUTPUT_BYTES` は
  `pub use cheaper_core::policy::exec::…` 経由で path 維持。
- `tools/process/tests.rs` — `process_operation_declaration`/
  `resolve_process_policy`/`ProcessToolOptions` →
  `exec_operation_declaration`/`resolve_exec_policy`/
  `ExecToolOptions`（:15-37,:64-75）。

### 4. `registry.rs` — `exec` 一本化（design (g) 段別置換表）

- `ToolRuntimeOptions`（:97-108）— `process`/`command` field を
  `exec: Option<Value>` 一本へ（`security` 不変 — caller は
  `serde_json::to_value(&policy.exec)` を渡す機械的
  serialize）。
- `create_tool_runtime`（:1095-）— `resolve_exec_deny_list`（維持）
  + `resolve_exec_policy(options.exec, &exec_deny)` 単一
  resolve（`resolve_process_policy`/`resolve_command_policy` の
  個別 resolve は除去）。
- content gate — `names` 含 `Process` + `!exec.grants.values().
  any(|g| g.fixed)` → degrade/`POLICY_VIOLATION "process
  exposure requires nonempty authorized tasks"`（文言不変）。
  `names` 含 `Command` + `!any(|g| !g.fixed)` →
  `"command exposure requires nonempty authorized grants"`。
- writeScope gate — `names` 含 `Command` + `!fixed` grant 非空
  + `write_scope` → `POLICY_VIOLATION "command is incompatible
  with worker writeScope"`（:1150-1159 同位置・同文言 — M4）。
- pin — `names` 含 `Command` → 全 `!fixed` grant を
  `pin_exec_grants`、`names` 含 `Process` → 全 `fixed` grant
  を同関数で pin。**task executable が pin されるのは本 issue
  が初**。**pin-fail×degradable（確定規則）**: 失敗 grant の
  surface（`fixed`/`!fixed`）に対応する tool が `degradable`
  宣言されていれば当該 tool のみ `names` から除去（opt-in
  縮退の既存機構再利用）、含まれなければ
  `POLICY_VIOLATION` で構築 fail。**Process・Command 両 surface
  に同規則を適用**（Command 側は opt-in 宣言時のみ差分の
  意図的拡張 — R11-(e)。非 opt-in 経路は現行無条件 hard fail
  不変）。
- `ToolCatalog::from_resolved_policy(exec)`（:1238）。
- `ToolCtx`（:220-243）— `process`/`command`/`command_pins`/
  `exec_deny` → `exec: ResolvedExecPolicy`・`exec_pins:
  BTreeMap<String,String>`・`exec_deny` へ一本化。
- dispatch（:878-897）— `exec_operation_declaration` へ置換。
- options builder（:633-643）・tool import（:57-59 command・
  :66-68 process）・`ToolCtx` literal（:1316-1332）の rename/
  型追随。

### 5. `invocation.rs` — `exec` 一本化 + parity 2 check（design (k)）

- `InvocationToolRuntimeOptions`（:62-82）— `process`/`command`
  field → `exec: Option<Value>` 一本（`security` 不変）。
- `InvocationInner` — `inner.process`/`inner.command` →
  `inner.exec: ResolvedExecPolicy`（`security`/`exec_deny`
  不変）。
- `create_invocation_tool_runtime`（:423-436）—
  `resolve_exec_policy(options.exec.as_ref(), &exec_deny)` 単一
  resolve。
- parity check（:228-260）— `options.runtime.exec` が `Some`
  なら `resolve_exec_policy(Some(raw), &inner.exec_deny)` →
  `inner.exec` と `PartialEq`、不一致は
  `invalid_binding("exec policy mismatch")`（`InvalidRequest`
  + `details.reason`）。caller 供給 exec の declared
  executable が inner deny に match した error は
  `InvalidRuntimePolicy`（inner code 保持 — security parity
  と同じ `load.rs:92-98` 先例規約）。**3 check（process/
  command/security）→ 2 check（exec/security）へ集約**。
- scope 再注入（:271-304）— `exec: Some(serde_json::to_value(
  &inner.exec).expect("ResolvedExecPolicy は必ず serialize
  可能"))` を runtime options へ再注入（`inner.security` と
  同型規約 — caller の `None`/任意 raw に関わらず scope は常に
  invocation-resolved の exec を受け取る）。
- `ToolCatalog::from_resolved_policy(self.inner.exec.clone())`
  （:172）。

### 6. `catalog.rs` — policy 型 + 2 arm（design (k)）

- `ToolCatalog.policy: ResolvedExecPolicy`・
  `from_resolved_policy(exec)`（:8-10,:42-43,:57-76,:363-364,
  :489-490,:587-597）。
- `Process` arm — `input.task` → `resolve_exec_grant(fixed:
  true)` → `exec_grant_resources`（宣言 inputs/outputs →
  per-path、無ければ root fallback）。未知名・surface 不一致
  は `POLICY_VIOLATION` を伝播（現行規約不変 —
  `named_task` 検査が literal 名を既要求）。
- `Command` arm — **`input.name` が literal string で `!fixed`
  grant へ resolve する場合のみ** `exec_grant_resources`。
  非 literal（`{$input}` 等 dynamic binding の非 string）・
  未知名・`fixed` grant 一致は全て input 非依存の **root
  manifest fallback**（error 非伝播 — 現行
  `command_grant_resources`:419-429 と同値。`{$input}`
  workflow の破壊防止 + root manifest は sound な overclaim —
  design Risks 確定）。

### 7. tools test churn + 新規 test（TSPEC Rules 対応）

| file | 変更 |
|------|------|
| `tests/proc_command.rs` | `ToolRuntimeOptions{process,command}` → `exec`・`ResolvedCommandPolicy` literal → `ResolvedExecPolicy`・`resolve_command_policy` 呼出し → exec resolver 経由・deny 適用点 2/3・disjoint surface（`!fixed` 名の fixed surface 参照拒否）・pin-fail×degradable Command 側 pin |
| `tests/proc_process.rs` | options literal `exec:` 化・`ResolvedProcessPolicy` → `ResolvedExecPolicy`・disjoint surface（`fixed` 名の `!fixed` surface 参照拒否）・task pin・writeScope tri-state・deny task 適用 |
| `tests/proc_invocation.rs` | `InvocationToolRuntimeOptions{process,command}` → `exec`・parity 2 check・`invalid_binding("exec policy mismatch")`・inner deny match の `InvalidRuntimePolicy`・scope 再注入 `inner.exec`・round-trip 冪等（`options.exec`→`inner.exec` serialize→再 resolve→`PartialEq`） |
| `tests/proc_runtime.rs` | options literal `exec:` 化 |
| `tests/proc_degrade.rs` | options literal + pin-fail×degradable **両 surface** pin（Process・Command の degradable 宣言時縮退・未宣言時 hard fail） |
| `tests/characterization.rs` | options literal `exec:` 化 |
| `tests/differential.rs` | options literal + **harness conversion seam（確定）** — `runtime_options`（:646-698）の `spec.process`（authoring shape）→ `build_exec_input` → `resolve_exec_policy` → `to_value` → `options.exec` 変換を harness 内に追加。`tests/fixtures/process.json`/`runtime.json` は**不変**（authoring-shaped spec を fixture rewrite なしで受理し synthesis 経路を全 case で回帰） |
| `tools/process/tests.rs` | `process_operation_declaration`/`resolve_process_policy`/`ProcessToolOptions` → unified 版 |

- `exec/mod.rs` unit — disjoint surface resolution（`fixed`
  期待値不一致・不在の surface 別 error 語彙）・deny 適用点
  2/3（tempdir 実在 executable で `pin_exec_grant`/
  `verify_pinned_executable` → `PolicyViolation` +
  `details.grant`）・`exec_operation_declaration`（ClaimMode
  tri-state・pool origin 分離・全 grant resources）。
- `exec/prepare.rs`/`proc_process.rs` — writeScope tri-state
  （`Some(nonempty)` coverage・`Some([])` trivially 成立・
  `None`+`Some(write_scope)` fail-closed・`None`+`None` 通常
  実行）・pin 後 PATH 差替えの spawn 前拒否。
- **scheduler 相互作用 flag** — fixed grant への per-path
  `declaration.resources` 付与と `scheduler.rs:227-233`
  PerResource demotion の相互作用を `proc_*`/scheduler test で
  確認（意図しない並行度変化の回帰観点 — design (h) 実装時
  検証 flag）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/unified-exec-grant-model.md`
  （PLAN-034KQs6XKnAqahkSVke78h — T2）
- `docs/designs/unified-exec-grant-model.md`
  （DESIGN-034KP4lfgfD7DdGiDV1YuT — (g) 段別置換表・(h)
  `tools/exec` API + manifest/writeScope tri-state + op
  declaration + lifecycle/pool 非対称 + scheduler flag・(i)
  disjoint surface 対照表・(k) invocation/catalog・(l)
  signature・(n) error 契約が AUTHORITATIVE）
- `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6 — R2/R5/R6/R7/R8/R10/R11）
- `docs/test-specs/unified-exec-grant-model.md`
  （TSPEC-034KQs1NIL8ahirht6QteV — disjoint surface・deny
  適用点 2/3・manifest/writeScope tri-state・pin-fail×
  degradable・catalog fallback・parity・harness seam Rules）
- `docs/tasks/add-core-exec-policy-module-and-loader-wiring.md`
  （TASK-034KQsi5CcOH2MsHWhWWnC — 前提）

## Dependencies

- `docs/tasks/add-core-exec-policy-module-and-loader-wiring.md`
  （T1） — `ExecGrant`/`ResolvedExecPolicy`/
  `resolve_exec_policy`/`resolve_exec_grant`/
  `exec_grant_resources`/`ArgPattern`/`argv_shell_re`/shared
  helper が必要。
- 既知の transient: 本 task で `pin_command_grant(s)`/
  `verify_pinned_executable`/`check_command_execution`/
  `CommandToolOptions`/`ProcessToolOptions`/`ToolCtx`/
  `ToolRuntimeOptions`/`InvocationToolRuntimeOptions`/
  `ToolCatalog` が unified 置換されるため、cheaper-cli
  （`route/resolve.rs`・`route/supply.rs`・`workflow_run.rs`・
  `workflow_context.rs`）・cheaper-runtime（`run/mod.rs`）・
  workflow/rig-adapter test literal が T3 の追随まで compile
  不可。本 task の検証は `cargo test -p cheaper-tools` に
  scope する。

## Affected Files

- `crates/cheaper-tools/src/tools/exec/mod.rs`（add）
- `crates/cheaper-tools/src/tools/exec/prepare.rs`（add）
- `crates/cheaper-tools/src/registry.rs`（modify）
- `crates/cheaper-tools/src/invocation.rs`（modify）
- `crates/cheaper-tools/src/catalog.rs`（modify）
- `crates/cheaper-tools/src/tools/command/mod.rs`（modify）
- `crates/cheaper-tools/src/tools/process/mod.rs`（modify）
- `crates/cheaper-tools/src/tools/process/prepare.rs`（modify —
  `tools/exec/prepare.rs` へ移設）
- `crates/cheaper-tools/src/tools/process/tests.rs`（modify）
- `crates/cheaper-tools/tests/proc_command.rs`（modify）
- `crates/cheaper-tools/tests/proc_process.rs`（modify）
- `crates/cheaper-tools/tests/proc_invocation.rs`（modify）
- `crates/cheaper-tools/tests/proc_runtime.rs`（modify）
- `crates/cheaper-tools/tests/proc_degrade.rs`（modify）
- `crates/cheaper-tools/tests/characterization.rs`（modify）
- `crates/cheaper-tools/tests/differential.rs`（modify）

## Done When

- [x] `tools/exec/mod.rs` が design (h) の pub API
      （`ExecToolOptions`・`resolve_exec_grant`・
      `pin_exec_grant`/`pin_exec_grants`・
      `verify_pinned_executable`・`check_exec_execution`・
      `exec_operation_declaration`）を持ち、適用点 2（resolved
      basename deny）・適用点 3（pin-equality 直後の再検証）が
      `grant_violation` 規約で `PolicyViolation` を返す
- [x] `tools/exec/prepare.rs` が cwd/manifest/deny/writeScope
      tri-state/pin+verify の unified prepare を持ち、per-call
      lazy executable resolve が除去されている
- [x] `tools/command`・`tools/process` が surface 固有の
      schema/details/description/identity のみ所有する thin
      surface となり、`process` の re-export path・canonical
      vocabulary が維持されている
- [x] `ToolRuntimeOptions.exec`・`ToolCtx.{exec,exec_pins,
      exec_deny}` が存在し、`create_tool_runtime` が
      `resolve_exec_policy` 単一 resolve・surface 別 content
      gate/writeScope gate・pin-fail×degradable 両 surface 対称
      で動く
- [x] `InvocationToolRuntimeOptions.exec`・`inner.exec` が
      保持され、parity 2 check（`invalid_binding("exec policy
      mismatch")`・inner deny match の `InvalidRuntimePolicy`
      inner code 保持）・scope 再注入 `inner.exec` が動く
- [x] `ToolCatalog` が `ResolvedExecPolicy` を持ち、Process arm
      が error 伝播・Command arm が literal+`!fixed` 限定の
      `exec_grant_resources` 委譲 + root manifest fallback
      （error 非伝播）で動く
- [x] tools test churn + differential harness seam（fixture
      json 不変の `spec.process` → `options.exec` 変換）+ 新規
      test（disjoint surface・deny・writeScope tri-state・
      pin-fail×degradable・parity）が存在し `cargo test -p
      cheaper-tools` が pass

## Out of Scope

- `crates/cheaper-cli`/`cheaper-runtime`/`cheaper-workflow`/
  `cheaper-rig-adapter` 側の call site・route 層・literal
  churn（T3 — `route/resolve.rs` の `ExecPolicyView`/
  `pin_exec_grant`/`ExecToolOptions`/`check_exec_execution`
  追随を含む）
- docs（T4）
- `tools::proc` 層（`ProcLimits`/`ProcPrepared`/`ProcIdentity`/
  `minimal_env`/`prepare_cwd`/`prepare_exec_cwd`/tree cleanup/
  bounded output）の変更 — 既存共有のまま
- authoring surface・workflow `tool:"process"` の canonical 名
  変更（M5 不変）
- scheduler PerResource demotion 自体の設計変更（実装時検証
  flag のみ — 意図しない並行度変化は test で検出）

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools` exits 0
      （unit test + proc_* integration + characterization +
      differential 全 pass — single-crate では dev-seam
      feature 適用外のため flag 無し。`85571c7` で実測。
      並列負荷下の `node_task_resolves_via_path` helper
      exit 255 は isolated rerun pass の flake — regression
      ではない）
- [x] `mise exec -- cargo clippy -p cheaper-tools --all-targets`
      exits 0（`mise run check` の clippy stage で workspace
      `--all-targets` として実測）
