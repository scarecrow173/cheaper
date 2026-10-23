---
id: "TSPEC-034KQs1NIL8ahirht6QteV"
type: "test-spec"
status: "approved"
title: "unified exec grant model の検証契約 (issue #165)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/unified-exec-grant-model.md"
    - "docs/discovery/unified-exec-grant-model-issue-165.md"
    - "docs/test-specs/opt-in-exec-deny-patterns.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/designs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
  derived-by: []
  verifies:
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/designs/unified-exec-grant-model.md"
  verified-by: []
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/opt-in-exec-deny-patterns.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/specs/managed-process-canonical-tool.md"
    - "docs/specs/policy-gated-command-tool.md"
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

# unified exec grant model の検証契約 (issue #165)

## Purpose

issue #165（#151 実装分解 3/7）で確定した「`command.grants` と
`process.tasks` の単一 exec grant model への統合」を、統一
schema・loader 写像・disjoint surface・pin/prepare・writeScope・
deny uniform・route 境界・serialized parity・catalog manifest・
CLI 入口 reject の 10 面で固定する。SPEC-034KOpKfZpegX2MLPD0uq6
R1-R11・Acceptance Criteria と DESIGN-034KP4lfgfD7DdGiDV1YuT の
Verification Notes が authoritative。

保つべき意図は 4 つ。(1) **単一 resolved model**:
`RuntimePolicy.exec` が `BTreeMap<String, ExecGrant>` 1 本の grant
map を持ち、`process.tasks` は `fixed:true` + `scope` provenance
付き grant へ畳まれる — authoring surface（policy.yaml の 2 節・
`policy grant` CLI・`--grant`・`tool:"process"`）は不変。
(2) **fail-closed の一貫性**: collision・deny・pin 不能・unknown
grant/task・route eligibility 違反・exec-doc 形状不正は全て
fail-loud（`InvalidRuntimePolicy`/`PolicyViolation`/
`InvalidRequest`/`Viability::NoGrant`）であり silent shadow・
承認の silent 喪失は生じない。(3) **surface disjoint**:
`command` は `!fixed` のみ・`process` は `fixed` のみ解決し、
argv 供給と fixed argv の混線を surface error 語彙で拒否する。
(4) **round-trip 冪等性**: `to_value(&policy.exec)` →
`resolve_exec_policy` → `PartialEq` が invocation parity の
不変条件として成立し、options 経路は serialized resolved shape
のみを受理する。

退役条件: `RuntimePolicy.exec` の構造・`fixed`/`scope` 契約・
disjoint surface・3 段 resolver 構成が変わるまで存続する。
#166 の route grant-set 投影が導入されても本 TSPEC の核心
（`is_route_eligible`/`project_grant` seam・deny uniform・
writeScope tri-state）は変わらない。

## Feature

`policy/exec.rs` の `ExecGrant`/`ResolvedExecPolicy`（単一 grant
map + origin 別 `ExecDefaults` 2 block）へ `command.grants`・
`process.tasks` を `build_exec_input`（collision + provenance）
→ `resolve_exec_policy`（exec-doc boundary + deny 適用点 1 +
lifecycle fallback）で畳み込み、`tools/exec/` の pin/verify/
check/prepare 単一機構へ `command`（`{name,args}`→`!fixed`）・
`process`（`{task}`→`fixed`）の thin surface が接続する。
`security.execDenyPatterns` は全 grant へ 3 適用点で uniform に
効き、route は `scope=="trusted" && !fixed` のみ解決して
`{route.grant} ∪ {fixed grants}` 投影する。

## Rules

- Rule: **統一 schema と `fixed` 意味論**（spec R1/R6 —
  `policy/exec.rs` unit test）
  - `resolve_exec_policy` が `{executable, fixed, argv?, inputs?,
    outputs?, cwd, env, timeout, killGrace, maxOutputBytes,
    exitCodes, scope}` の resolved grant を `BTreeMap`（name
    `locale_compare` 順）へ normalize する。
  - `argv` の 2 意味化: `fixed:true` は宣言 argv literal
    （`ArgPattern::compile` しない）、`fixed:false` は argv
    pattern（省略 = args 個数無制約）。
  - `fixed`/`scope`/`inputs`/`outputs` 未宣言は `false`/
    `"trusted"`/`None` に collapse。`exit_codes` 未宣言は `[0]`。
  - lifecycle 欠落 field は `fixed` → `process` defaults、
    `!fixed` → `command` defaults で deterministic fold
    （synthesis 経路では全値明示 — dormant fallback として pin）。
  - defaults block 欠落時の既定値
    `max_concurrent:1, timeout:"2m", max_output_bytes:65_536,
    kill_grace:"2s", heartbeat:"30s"`。

- Rule: **exec-doc 形状の fail-loud**（design (d) —
  `exec.rs` unit test）
  - `exec_policy_properties()` の `additionalProperties:false`
    が authoring 形（`tasks`/`grants` key を defaults block へ
    混入した object、未知 key）を `InvalidRuntimePolicy`
    （`"invalid exec policy: missing, unknown or invalid config
    field"`）で拒否する — caller は必ず `build_exec_input` 経由。
  - defaults block の `heartbeat` nested object は
    `{"type":"object","additionalProperties":false,"properties":
    {"interval":…}}` を再現し `{"heartbeat":{"interval":…,"x":…}}`
    の未知 key を拒否する（round-trip 不変条件の pin）。
  - grant `executable` 必須・name pattern
    `^[a-z][a-z0-9_-]{0,63}$`・`maxProperties:256`。
  - `None`/`null` 入力は `{}` に collapse → empty grants
    （=deny）へ normalize（現行規約継承）。

- Rule: **`build_exec_input` — provenance + collision**
  （spec R4 — `exec.rs`/`load.rs`/`schema.rs` test）
  - `command.grants` × `process.tasks` の同名交差が非空で
    `exec_invalid("grant name collides between command.grants and
    process.tasks")` → `InvalidRuntimePolicy` +
    `details.{grant,task}`（両方に衝突名 — 宣言位置 attribution）。
  - collision が `validate_policy_schema`（Complete/Partial/
    ProjectPartial 各 mode — ∅ names）・`load_policy`
    normalize・`policy grant add` `write_validated` の全入口で
    同一 `InvalidRuntimePolicy` を返す。
  - task → grant 変換: `{executable:command[0], fixed:true,
    argv:command[1..], inputs, outputs, cwd, env,
    lifecycle:process section resolved defaults, exitCodes:[0],
    scope: names.contains(name) ? "project" : "trusted"}`。
  - command grant → grant 変換: declared fields そのまま +
    `fixed`/`scope`/`inputs`/`outputs` 既定値畳み込み
    （`scope:"project"` は自己弱体化として受理）。
  - **merge 前捕捉**: `load_policy` が ProjectPartial validate
    直後・RestrictConcurrency merge 直前に project doc の
    `process.tasks` keys を `BTreeSet` へ捕捉する — deep-merge
    で同名 task が project field で override された後では
    provenance を導出不能（merge 後捕捉の bug を pin）。
  - project task 名による trusted task 名の deep-merge shadow は
    merged spec → `scope:"project"` で保存（A3 不変式を弱めない
    — route 参照不能な scope でのみ上書きが効く pin）。
  - `project_task_names` は provenance stamp のみに使い
    validation 成否へ影響しない（`validate_policy_schema` は
    ∅ で呼び per-doc collision のみ検査 — layer 非依存性 pin）。

- Rule: **`resolve_exec_policy` round-trip 冪等性**（spec R5 —
  `exec.rs`/`proc_invocation.rs`）
  - serialized `ResolvedExecPolicy`（畳み込み済み lifecycle・
    `fixed`/`scope`/`inputs`/`outputs` 明示値）が exec-doc input
    schema に適合し、re-resolve で同一結果を返す（invocation
    parity の不変条件）。
  - `options.exec` raw JSON は `to_value(&policy.exec)` の機械的
    serialize のみを受理する契約 — parity check（serialize →
    再 resolve → `PartialEq`）が `proc_invocation.rs` で成立。

- Rule: **disjoint surface resolution**（spec R6 —
  `tools/exec/mod.rs` unit + `proc_process.rs`/`proc_command.rs`）
  - `resolve_exec_grant(policy, name, fixed:false)` は `!fixed`
    grant のみ解決 — `fixed` grant 一致・不在は
    `PolicyViolation` `"unknown or unauthorized command grant"` +
    `details.grant`。
  - `resolve_exec_grant(policy, name, fixed:true)` は `fixed`
    grant のみ解決 — `!fixed` 一致・不在は `PolicyViolation`
    `"unknown or unauthorized process task"` + `details.task`。
  - `command` surface input `{name,args}` は
    `command_schema():51-70` 不変、`process` surface `{task}` は
    `process_schema():51-66` 不変 — `args` 等の extra key は
    `additionalProperties:false` で schema reject（fixed grant
    が argv 供給を受けない M2 契約）。
  - details schema・`ProcIdentity`（`{grant,argv}`/`"command
    {name}"`/`report_duration_ms:true` vs `{task}`/`"process
    {name}"`/`false`）・accepted_exit_codes・route_context
    注入可否・description の surface subset 列挙は surface 別に
    保存。

- Rule: **deny uniform 3 適用点 — task 由来含む**（spec R10/D6
  — `exec.rs` + `tools/exec/mod.rs` + `proc_*`）
  - 適用点 1（declared basename）: `resolve_exec_policy` grant
    loop で `deny.is_denied(&executable)` →
    `InvalidRuntimePolicy` `"invalid exec policy: executable is
    denied by security.execDenyPatterns"` + `details.{grant,
    executable}`。**fixed grant は `details.task` 併記**
    （宣言位置が `process.tasks` である attribution）— task 由来
    executable への deny 適用は本 issue が初（#164 の task 除外
    吸収）、denied project task は policy load fail。
  - 適用点 2（resolved basename）: `pin_exec_grant` で
    `PolicyViolation` `exec grant "{name}": resolved executable
    is denied by security.execDenyPatterns` + `details.grant`。
  - 適用点 3（spawn 前再検証）: `verify_pinned_executable` で
    pin-equality 直後に同一 predicate を再適用 — 全 grant・
    両 surface で共有。
  - 既定（空 `ExecDenyList`）は全適用点 no-op — #164 後の既定
    挙動不変。invocation 合成 grant・project task 由来を含む全
    grant に merged effective deny が及ぶ（layer-agnostic）。

- Rule: **manifest tri-state + op declaration**（spec R2 —
  `exec.rs` unit + `catalog.rs`/`resource_inference.rs`）
  - `exec_grant_resources`: `inputs`/`outputs` のいずれかを宣言
    （`Some([])` 空配列も宣言と見なす）→ per-path manifest
    （`reads=inputs`・`writes=outputs`・`produces=outputs`・
    `**` 末尾 → `Subtree`）。`Some([])` は「何も claim しない」
    空 manifest。両方 `None` → conservative root manifest。
  - `exec_operation_declaration`: ClaimMode は `outputs:
    Some([])` → `Shared`、その他（`Some(nonempty)`・`None`）→
    `Exclusive`。pool は `fixed` → `{"process",
    policy.process.max_concurrent}`、`!fixed` → `{"command",
    policy.command.max_concurrent}`（R3/D11 budget 保存 —
    lifecycle は origin defaults・pool/heartbeat は reach
    surface の意図的非対称を pin）。`declaration.resources` は
    全 grant で設定（宣言があれば per-path、無ければ root）。

- Rule: **writeScope per-call coverage + fail-closed**（spec
  R8/M4 — `tools/exec/prepare.rs` + `registry.rs` test）
  - 構築時 gate: `names` 含 `Command` + `!fixed` grant 非空 +
    `write_scope` → `POLICY_VIOLATION`
    `"command is incompatible with worker writeScope"`（文言・
    位置不変 — argv 供給 grant の writeScope 下実行は常拒否）。
  - `prepare_exec_grant` の `outputs` tri-state:
    `Some(nonempty)` → manifest normalize + `claim_contains` で
    `outputs ⊆ writeScope` 検査、非対応は `POLICY_VIOLATION`
    `"process outputs exceed worker writeScope"`。
    `Some([])` → 空 manifest = 副作用非宣言契約で coverage
    trivially 成立・exec 可。`None` → `write_scope` が `Some`
    なら証明不能で `POLICY_VIOLATION`（fail-closed — C5）、
    `None` なら coverage 検査不要で通常実行。
  - `!fixed` grant は構築時 gate で writeScope 下に到達不能
    （prepare 側に防御分岐を残さない — unreachable state）。

- Rule: **pin-fail × degradable 両 surface 対称**（spec R7/
  R11-(e) — `proc_degrade.rs`/`proc_runtime.rs`）
  - scope 構築で `names` 含 `Command` → 全 `!fixed` grant、
    `names` 含 `Process` → 全 `fixed` grant を ambient PATH で
    `pin_exec_grants` — **task executable が pin されるのは本
    issue が初**（per-call lazy resolve `prepare.rs:127-136` 経路
    は除去）。
  - pin 不能は構築 `POLICY_VIOLATION` `exec grant "{name}":
    {reason}` + `details.grant` — ただし失敗 grant の surface
    （`fixed`/`!fixed`）に対応する tool が `degradable` 宣言され
    ていれば当該 tool のみ `names` から除去して縮退。Process・
    Command の両 surface に同規則が効く（Command 側は opt-in
    宣言時のみ差分の意図的拡張 — 現行の無条件 hard fail からの
    R11-(e) 破壊的変更を `proc_degrade.rs` で pin）。
  - 非 opt-in 経路（degradable 未宣言）の pin fail hard fail は
    不変。
  - content gate: `names` 含 `Process` + fixed grant 0 件 →
    degrade/`POLICY_VIOLATION "process exposure requires
    nonempty authorized tasks"`、`names` 含 `Command` + `!fixed`
    0 件 → `"command exposure requires nonempty authorized
    grants"`（文言不変）。
  - pin 後の PATH 差替え・executable 差替えが spawn 前
    `verify_pinned_executable` で `POLICY_VIOLATION`（B1/B2）。

- Rule: **catalog `Command` arm 条件付き解決 + `Process` arm**
  （design (k)/Risks — `catalog.rs` +
  `filesystem_contracts/resource_inference.rs`）
  - `Process` arm — `input.task` → `resolve_exec_grant
    (fixed:true)` → `exec_grant_resources`（宣言 inputs/outputs
    → per-path、無ければ root fallback）。未知名・surface 不
    一致は `POLICY_VIOLATION` 伝播（現行規約不変 —
    `named_task` 検査が literal 名を既要求 `resource_inference.
    rs:131-138`）。
  - `Command` arm — `input.name` が **literal string で `!fixed`
    grant へ resolve する場合のみ** `exec_grant_resources` へ
    委譲。非 literal（`{$input}` 等 dynamic binding の非
    string）・未知名・`fixed` grant 一致は全て input 非依存の
    root manifest fallback へ倒す（error 非伝播 — fail-loud は
    `{$input}` workflow を破壊し、root manifest は sound な
    overclaim）。
  - `create_tool_catalog(None)` 空 catalog 上の `tool:"command"`
    job が root manifest を返す現行契約
    （`resource_inference.rs:496-586`）の回帰維持。

- Rule: **route eligibility + `project_grant` 投影**（spec R9 —
  `route/resolve.rs` + core unit + route run 経路）
  - `ExecPolicyView`（旧 `CommandPolicyView`）経由の viability
    で `policy.grants.get(&route.grant)` 後に
    `is_route_eligible(grant)`（`scope=="trusted" && !fixed`）を
    検査 — `scope=="project"`・`fixed`・不在は全て
    `Viability::NoGrant`（routing≠authorization、D2）。
  - `ResolvedExecPolicy::project_grant(name)` が
    `{route.grant} ∪ {fixed grants}` を残す（非 fixed の他
    grant を隠す C9 confused-deputy 防止 + routed workflow から
    task を呼べる現行挙動保存 — 内蔵 eligibility で二重防御）。
  - viability precheck（`check_exec_execution`）・
    `expected_input {"name":grant,"args":exec_args}`・exec-input
    byte 一致・exit mirror・grant timeout deadline anchor 不変。
  - denied resolved-stem grant は viability で `NoGrant`（error
    ではなく route 不成立 — unpinnable と同分類）。

- Rule: **`tool:"process"` workflow compat（M5）**（spec R6 —
  `cheaper-workflow` 回帰 + `benchmarks`）
  - `{"kind":"tool","tool":"process","input":{"task":…}}` が
    `ports.tools.get("process")`・`tools.allow` の literal
    `process` admission・`item.job.name=="process"` resource
    inference の全経路で `UnknownTool` なしに動く —
    `benchmarks/scenarios/scenarios.json` の `processPolicy`
    scenario が無改訂で動く回帰 AC。
  - `TOOL_ALIASES` 追加なし・`is_scope_gated_tool`/
    `is_mutating_tool`/`optionalTools`/`tools.allow` 照合不変・
    `tools::process::find_cheaper_error`/`OnTimeout`/
    `MAX_PROCESS_OUTPUT_BYTES` re-export path 維持
    （`engine.rs:374`・`rig-adapter/tools.rs:58` 参照）。

- Rule: **invocation parity / CLI 入口 reject**（spec R4/R5 —
  `invocation.rs` + `grant.rs`/`policy.rs` + cli integration）
  - `create_invocation_tool_runtime` が `options.exec` を
    `resolve_exec_policy(.., &exec_deny)` で resolve →
    `inner.exec` 保持（3 check → 2 check 集約: exec/security）。
  - parity: `options.runtime.exec` が `Some` なら
    `resolve_exec_policy(Some(raw), &inner.exec_deny)` →
    `inner.exec` と `PartialEq`、不一致は
    `invalid_binding("exec policy mismatch")`（`InvalidRequest`
    + `details.reason`）。caller 供給 exec の declared
    executable が inner deny に match した error は
    `InvalidRuntimePolicy`（inner code 保持 — security parity
    と同規約）。
  - scope 再注入: caller の `None`/任意 raw に関わらず scope は
    常に `Some(to_value(&inner.exec))` を受け取る
    （`inner.security` と同型）。
  - `--grant`（`grant.rs::build_invocation_layer`）と
    `policy grant add`（`policy.rs::parse_grant_spec`）の両入口
    で `fixed:true`・明示 `scope` key を含む spec を
    `InvalidRequest` で拒否 — `--grant` 合成 grant は
    `!fixed`+`scope:"trusted"` で `command` surface 到達。
    trusted YAML での `fixed`/`scope` 直接 authoring は合法の
    まま（schema は受理、CLI 入口のみ拒否）。
  - merged deny・collision error は `details.layer` を持たない
    （merged-map 評価の帰属不能 — #164 と同構造の accepted
    gap pin）。

- Rule: **serialized 形状 + golden fixture**（spec R11-(d) —
  golden target）
  - `RuntimePolicy` serialize が `{"exec":{grants,command,
    process}}` 形状（`workflow` → `exec` → `route` → `security`
    key 順）を持ち、`process`/`command` top-level key が消える。
  - `policy/{load,schema,restrict,process}.json` が exec 形状へ
    再採取、`merge.json`（raw map merge）/`workflow.json`
    （process/command 参照なし）は不変。`policy_process.rs`
    driver は `resolve_exec_grant`/`exec_grant_resources`/
    `resolve_exec_policy` の unified op へ写像（fixture case の
    op 名は維持可）。
  - `RuntimePolicy`/`ToolRuntimeOptions`/
    `InvocationToolRuntimeOptions` literal churn（`exec:`/`exec:
    None`）が全 test file で追随。

- Rule: **harness conversion seam + 既存契約回帰**
  （`differential.rs` + 全 target）
  - `differential.rs::runtime_options`（:646-698）が
    `spec.process`（authoring shape）→ `build_exec_input` →
    `resolve_exec_policy` → `to_value` → `options.exec` の変換
    を harness 内で行い、`tests/fixtures/process.json`/
    `runtime.json` が**不変**のまま全 case で synthesis 経路を
    回帰する。
  - pin・env_clear・stdin null・process group・lifecycle
    （timeout/kill grace/heartbeat）・exit code 契約・fs
    boundary・denylist・writeScope が `proc_*`/
    `characterization`/`differential`/`contracts` で pass。
  - fixed grant への per-path `declaration.resources` 付与と
    `scheduler.rs:227-233` の PerResource demotion 相互作用は
    実装時に `proc_*`・scheduler test で確認（意図しない並行度
    変化の回帰観点 — design (h) 実装時検証 flag）。

- Rule: **docs contract**（design Docs 更新契約 — doc review）
  - `managed-process.md` — task → `fixed:true` grant 写像・
    scope 構築時 pin（unresolvable は構築 fail/degradable 縮退）・
    `execDenyPatterns` の task 適用・collision
    `InvalidRuntimePolicy`。
  - `command.md` — grant spec 新 optional field
    `fixed`/`inputs`/`outputs`/`scope`・disjoint surface・
    collision 制約・deny uniform。
  - `configuration.md` — `command.grants` 新 field・`process`
    節の loader 写像（project 宣言 task は `scope:"project"`）・
    collision・deny task 適用拡大。
  - `usage.md` — `policy grant` CLI 節の collision 制約 +
    `fixed`/`scope` 宣言の `InvalidRequest` reject（両入口）。
  - `workflow-usage.md` — `tool:"process"` job が `input.task`
    → fixed grant 解決の compat alias として不変で動くこと。
  - `workers.md` — writeScope 下は `outputs ⊆ writeScope` の
    fixed grant のみ exec 可・`!fixed` は構築時拒否 +
    `optionalTools` degradable 宣言 surface tool の pin 失敗
    縮退（両 surface）。
  - `resources/policy.yaml` — 節構成不変（authoring 不変）、
    `security` 節 comment の task 適用補足は任意。

- Rule: **full pipeline**（`mise run check` 全緑 +
  `node scripts/check-doc-ids.mjs` + `audit_docs` 0 error —
  design Verification Notes 末尾）

## Examples

- Example: `{"process":{"tasks":{"emit":{"command":["tool","x"],
  "inputs":["in/**"],"outputs":["out/**"]}}}}` + project 宣言名
  `{"emit"}` → `build_exec_input` が `{fixed:true, executable:
  "tool", argv:["x"], scope:"project", exitCodes:[0]}` の grant を
  生成し、`resolve_exec_policy` の `grants["emit"].scope ==
  Project`。
- Example: `command.grants.a` + `process.tasks.a` の同名 →
  `build_exec_input` が `InvalidRuntimePolicy` +
  `details.{grant:"a",task:"a"}`。同 doc が
  `validate_policy_schema`・`load_policy`・`policy grant add` で
  同一 failure。
- Example: `to_value(&policy.exec)` → `resolve_exec_policy` →
  `PartialEq`（畳み込み済み lifecycle・`fixed`/`scope`/
  `inputs`/`outputs` 明示値がそのまま採用 — parity 冪等）。
- Example: `resolve_exec_grant(policy, "emit", fixed:false)` →
  `PolicyViolation` `"unknown or unauthorized command grant"` +
  `details.grant`（fixed grant は command surface に不可視）。
  `resolve_exec_grant(policy, "g", fixed:true)`（`!fixed` grant
  `g`）→ `"unknown or unauthorized process task"` +
  `details.task`。
- Example: user `execDenyPatterns:["bad.*"]` + project task
  `{"command":["badexe"]}` → `load_policy` が
  `InvalidRuntimePolicy` + `details.{task:"t",executable:"badexe"}`
  （`details.task` 併記 pin）。tempdir pin/verify 経路でも
  `PolicyViolation` `exec grant "{name}": resolved executable
  is denied …`。
- Example: `outputs:Some([])` の fixed grant + `write_scope` →
  coverage 成立で exec 可。`outputs` 未宣言 + `write_scope:
  Some` → `POLICY_VIOLATION`。`!fixed` grant + `write_scope` →
  構築時 `POLICY_VIOLATION "command is incompatible with worker
  writeScope"`。
- Example: `names` 含 `Process` + degradable `process` 宣言 +
  unresolvable task → `process` tool のみ縮退で scope 構築成功。
  degradable 未宣言なら `POLICY_VIOLATION`。`names` 含
  `Command` + degradable `command` + unresolvable grant →
  `command` tool のみ縮退（新規 semantics pin）。
- Example: `{"kind":"tool","tool":"command","input":{"name":
  {"$input":"x"}}}` → catalog `Command` arm が root manifest
  fallback（error 非伝播）。literal `"emit"` で `fixed` grant
  一致も root manifest。literal 名 + 宣言 inputs/outputs の
  `!fixed` grant → per-path manifest。
- Example: `route.grant` が `scope:"project"`/`fixed`/不在 →
  `Viability::NoGrant`。trusted + `!fixed` は viability 解決。
  `project_grant("g")` 後の policy.exec.grants は
  `{"g"} ∪ {fixed grants}`。
- Example: `--grant '{"name":"x","executable":"e","fixed":true}'`
  → `InvalidRequest`（`details.{layer:"invocation",grant:"x"}`）。
  `policy grant add` の spec への `scope` 明示も
  `InvalidRequest`。trusted YAML の `{"fixed":true,
  "scope":"project"}` grant は受理（route NoGrant・process
  surface 到達）。
- Example: authoring-shaped `runtime.process` spec を持つ
  differential fixture が `build_exec_input` 変換経路で
  `options.exec` へ畳まれ、fixture 改訂なしで全 case pass。

## Guarantees

- 実装者が design を再解釈しなくても、統一 schema・provenance・
  collision・disjoint surface・deny 3 適用点・manifest/writeScope
  tri-state・pin-fail×degradable・catalog fallback・route
  eligibility/投影・parity・CLI reject・golden の全項目が unit +
  integration + golden + literal-churn test で機械的に検証できる。
- `fixed`/`scope` は declared-shape 契約として serialized parity
  に載り、provenance は merge 前捕捉で確定的に stamp される。
- collision・deny・pin 不能・writeScope 証明不能・exec-doc 形状
  不正の全 fail-closed 境界が warn-and-skip なしに error で
  観測される。
- `details.task`/`details.grant` の surface 別語彙と
  `details.layer` 無しの merged attribution gap が pin され、
  誤って「修正」されない。
- authoring surface 不変（`tool:"process"`・`policy grant`・
  `--grant`・policy.yaml 2 節）が compat 回帰で担保される。

## Non-goals

- `process`/`command` policy.yaml 節の名称・shape 変更、
  `policy grant`/`--grant`/route `grant` 用語の改名（authoring
  面不変 — Out of Scope）。
- route grant-set 投影（`grants[]`、issue #166）・substitute exec
  mode（issue 5）・trusted flag 発行主体（issue 6）・viability
  条件付き弱化（issue 4）の検証 — 本 TSPEC は単一モデル seam の
  み固定。
- `process` tool の `TOOL_ALIASES` 化（ADR O5c 却下済み）・tool
  数・capability 体系の変更。
- 文書全体の正直性 sweep（issue #169 の scope — 本 change に
  必要な docs 契約のみ）。
- migration guide 本文（issue 7 対象 — 破壊的変更一覧の検証は
  行うが告知文書は対象外）。
- 検証コマンド・実行結果の記録 — task の `## Verification` と
  impl record の責務。

## Risk

- `project_task_names` の merge 後捕捉・`validate_policy_schema`
  への layer 情報流出があると、provenance が誤 stamp または
  validation 成否が layer に依存する — 段 2 配置と ∅ names 呼出
  しを pin しないと silent shadow が復活する。
- `additionalProperties:false`・nested `heartbeat` 検査の脱落が
  あると、authoring 形の direct 投入・unknown key 混入を受理し
  round-trip/parity 不変条件が破れる — exec-doc boundary の
  fail-loud pin が唯一の検出手段。
- fixed grant が command surface ・route 参照へ漏れると argv
  供給を受けない grant が `args`/exec_args で実行された形に
  なり semantics が汚れる — disjoint surface・eligibility の
  pin がないと confused-deputy 防止（C9）・D2 が薄れる。
- pin-fail×degradable の Command 側拡張を pin しないと、意図的
  semantics 拡張（R11-(e)）が regression として「修正」され、
  または非 opt-in 経路へ漏れて現行挙動を破壊する。
- catalog `Command` arm の error 伝播・全 grant per-path 化を
  許すと `{$input}` workflow・空 catalog 契約が破壊される —
  fallback pin がないと fail-loud の false positive を生む。
- golden fixture（load/schema/restrict/process）の `exec` 形状
  再採取を忘れると serialized shape の破壊が silent 化する —
  churn 一覧の file 網羅と golden target が検出する。
- transient compile 境界（T1 後の tools/cli・T2 後の cli）で
  downstream test を実行すると signature 不一致で必敗する —
  各 task の verification を単一 crate に scope しないと誤検証
  する。
