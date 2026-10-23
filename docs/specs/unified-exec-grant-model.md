---
id: "SPEC-034KOpKfZpegX2MLPD0uq6"
type: "spec"
status: "proposed"
title: "unified exec grant model — command.grants / process.tasks の統合"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
    - "../discovery/unified-exec-grant-model-issue-165.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/unified-exec-grant-model.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/unified-exec-grant-model-issue-165.md"
  derived-by:
    - "../designs/unified-exec-grant-model.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../specs/opt-in-exec-deny-patterns.md"
    - "../specs/managed-process-canonical-tool.md"
    - "../specs/policy-gated-command-tool.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
  defers: []
  deferred-by: []
---

# unified exec grant model — command.grants / process.tasks の統合

## What Should Be Built

`command.grants` と `process.tasks` は「宣言済み executable の許可
+ argv 契約（固定 or pattern）+ policy 管理」の同一概念の二重実装
であり、schema・resolve・pin・manifest・spawn・op declaration・
test・文書が全て二重で、不整合の余地が構造的に存在する。これを
**単一の exec grant model** へ統合する（親 design
`docs/designs/permission-model-responsibility-redesign.md` の
Option D capability-grant 統合 + Merge Fidelity M1-M5 — issue
#151 分解 3/7）。

統一対象は resolved model と機構層であり、authoring surface は
不変とする:

- policy.yaml の `process` 節・`command` 節は維持（deprecate 無し）。
- loader が `process.tasks` を `fixed:true` + provenance scope 付き
  grant へ写像し、`command.grants` と同じ unified grant 集合へ畳む。
- tool surface は `command`（`{name,args}` → `!fixed` grant）に統一
  され、`process` は `input.task` → `fixed` grant 名を解決する
  実装層 compat alias として canonical vocabulary に残る。

## Why Now

- #163（内容審査除去、merged）で両側の審査分岐が同時に消え、
  統合の差分コストが最小になった。
- #164（opt-in `security.execDenyPatterns`、merged）で deny
  predicate が確立 — 統合後の grant 集合への適用規則を本 issue で
  定める必要がある（#164 は task 適用を deferred していた）。
- #166（route grant-set 投影）・#169（文書正直性 sweep）が
  統一モデルの形に依存するため、本 issue が先に確定する。

## Users and Value

- **operator/user**: grant/task の宣言方法は変わらず、内部で一貫
  した承認・pin・deny・manifest 規則が適用される。
- **開発者**: exec 承認の実装経路が一本化し、deny/pin/lifecycle/
  writeScope の不整合バグが構造的に防げる。route・workflow の
  後続変更（#166・#169）が単一モデル上で設計できる。

## Scope

### In Scope

- 統一 grant 型 `ExecGrant`（`{executable, fixed, argv?, inputs?,
  outputs?, cwd, env, timeout, killGrace, maxOutputBytes,
  exitCodes, scope}`）と `ResolvedExecPolicy`。
- `process.tasks` → grant の loader 写像（provenance scope・
  lifecycle 畳み込み・name collision 検査）。
- `RuntimePolicy.exec` への統一（`process`/`command` field 除去）
  と、それに伴う invocation parity・`ToolRuntimeOptions`・
  `ToolCtx`・`ToolCatalog`・route view・runtime/workflow 配管の
  一本化。
- pin/spawn/manifest/pool/op-declaration/prepare の単一化
  （shared `tools::proc` 層の上に unified exec path）。
- `tool:"process"` compat alias（`input.task` → fixed grant）。
- writeScope（M4）・route eligibility（trusted+非 fixed）・
  `execDenyPatterns` uniform 適用の規則。
- 上記に伴う既存契約 test・golden fixture の移行。

### Out of Scope

- `process`/`command` policy.yaml 節の名称・shape の変更
  （authoring 面は不変）。
- `policy grant` CLI・`--grant` flag・`route grant` 用語の改名。
- route grant-set 投影・viability 条件付き弱化・record split
  （issue 4/#166）・substitute exec mode（issue 5）・trusted flag
  発行主体（issue 6）— 本 issue はそれらが乗る単一モデルのみ定義。
- tool 数・capability 体系の変更（`process`/`command` の 2 名は
  不変）。

## Requirements

### R1 — 統一 grant schema

resolved policy は単一の grant map を持つ。grant は:

```text
ExecGrant {
  executable: string            // task: command[0] / grant: executable
  fixed: bool                   // M2 — true: 宣言 argv のみ（args 不許可）
  argv: Option<Vec<String>>     // fixed: task の command[1..] /
                                // 非 fixed: argv pattern（省略=個数無制約）
  inputs: Option<Vec<String>>   // M1 — task: inputs 必須相当 /
                                // grant: optional 宣言
  outputs: Option<Vec<String>>  // 同上
  cwd: string                   // workspace-relative（"." default）
  env: map                      // declared env（規則は現行共通）
  timeout / killGrace / maxOutputBytes / exitCodes  // 畳み込み済み
  scope: "trusted" | "project"  // provenance（M3）
}
```

- `fixed:true` grant は task 由来（project `process.tasks` 写像）
  または trusted 層が `command.grants` で `fixed` を宣言したもの。
  model 入力は `{task}`/`{name}` のみで `args` を受け付けない。
- `fixed:false` grant は model 供給 `args` を `argv` pattern で
  照合する（`argv` 省略=個数無制約・現行規約不変）。

### R2 — resource manifest（M1）

- `inputs`/`outputs` を持つ grant は per-path manifest を生成する
  （`process_task_resources` 同等: `reads=inputs`・`writes=outputs`・
  `produces=outputs`、`**` 末尾 → `Subtree`）。
- `inputs`/`outputs` を持たない grant は conservative root manifest
  （workspace root subtree の `reads`+`writes`、`produces` なし —
  `command_grant_resources` 同等）。
- `outputs` 空 → `ClaimMode::Shared`、非空 → `ClaimMode::Exclusive`
  （現行 `process_operation_declaration` の規則を全 grant に写す）。
- op declaration の pool: `fixed` → `"process"` pool
  （`process` section defaults の `maxConcurrent`）、`!fixed` →
  `"command"` pool（`command` section defaults）— budget 意味を
  変えずに機構のみ統一。

### R3 — lifecycle

- per-grant `timeout`/`killGrace`/`maxOutputBytes`/`exitCodes` は全
  grant で畳み込み済み値を保持。task 由来 grant は `process`
  section の defaults + `exitCodes:[0]`（現行 process 挙動）、
  grant 由来は per-grant override ?? `command` section defaults
  （現行 command 挙動）。
- `heartbeat.interval`・`maxConcurrent` は origin 別の section
  defaults として `ResolvedExecPolicy` 上に保持する（`fixed` →
  `process` block、`!fixed` → `command` block）。

### R4 — loader 写像（M3）

- `load_policy` は project merge 前に ProjectPartial で validate
  済みの project doc から `process.tasks` の宣言名集合を捕捉する。
- merged `process` 節の `resolve_process_policy` 結果を走査し、
  各 task を fixed grant へ写像する:
  `executable=command[0]`、`argv=command[1..]`（内部実行用 — argv
  pattern ではない）、`inputs`/`outputs`/`cwd`/`env` 写し、
  lifecycle=section defaults、`exitCodes=[0]`、
  `scope = 名が project 宣言名集合に含まれるなら "project"、
  さもなくば "trusted"`。
- `command.grants` は `fixed:false`（authoring で `fixed:true` を
  宣言した場合はその値）+ `scope:"trusted"`（authoring で
  `scope:"project"` は自己弱体化として受理）として畳み込む。
- `command.grants` 名と `process.tasks` 名の衝突は
  `InvalidRuntimePolicy` で fail-loud（新規制約 —
  `details.grant`/`details.task` で両宣言を指す）。同一 resolve
  経路は `validate_policy_schema` の section 再 resolve・
  invocation parity・`policy grant` の `write_validated` にも
  適用され、どの入口でも同一 failure が観測できる。
- project task 名が trusted task 名と衝突する場合は従来の
  deep-merge 規約を保存する（merged spec → `scope:"project"`。
  route grant 参照不能な project scope でのみ上書きが効くため
  A3 invariant を弱めない）。

### R5 — RuntimePolicy / parity 配管

- `RuntimePolicy` に `exec: ResolvedExecPolicy` を新設し、
  `process`/`command` field は除去する。
- `ToolRuntimeOptions`・`InvocationToolRuntimeOptions` の
  `process`/`command` raw JSON field を `exec` 一本へ置き換え、
  caller は `serde_json::to_value(&policy.exec)` を渡す。
- invocation parity は serialized `exec` の再 resolve→`PartialEq`
  比較で検査する（現行の process/command 2 check を exec 1 check
  へ集約。`security` parity は不変）。
- `ResolvedExecPolicy` の serialized shape は再 resolve で
  冪等に round-trip すること（resolved grant の `fixed`/`scope`/
  `inputs`/`outputs`/畳み込み済み lifecycle・section defaults
  block を入力 schema が受理する）。

### R6 — tool surface（M5 + disjoint 分離）

- `command` tool: `{name, args}` — `fixed:false` grant のみ解決。
  `input.name` が fixed grant に一致する場合は未知 grant として
  `POLICY_VIOLATION`（`details.grant`）。
- `process` tool: canonical vocabulary に残す実装層 alias —
  `{task}` schema・task 名埋め込み description・
  `{task,state,exitCode,signal,stdout,stderr}` details shapeは
  現行不変。`input.task` は `fixed:true` grant のみ解決し、
  `!fixed` grant への一致は未知 task として
  `POLICY_VIOLATION`（`details.task`）。
- workflow `{"kind":"tool","tool":"process","input":{"task":…}}`
  は `ports.tools.get("process")`・admission（`tools.allow` の
  literal `process`）・resource inference（`item.job.name ==
  "process"` + `tools.get("process").resources(input)`）の各経路が
  不変で動く。
- `ToolCatalog` は unified policy を持ち、`Process` arm は
  `input.task` → fixed grant resources（per-path or root fallback
  by declaration）、`Command` arm は `input.name` → grant
  resources（宣言 `inputs`/`outputs` があれば per-path、無ければ
  root manifest）を返す。
- error envelope は surface 別に保存（fixed surface:
  "unknown or unauthorized process task"/`details.task`・
  "process outputs exceed worker writeScope" 等。非 fixed:
  "unknown or unauthorized command grant"/`details.grant` 等）。
- `is_scope_gated_tool`/`is_mutating_tool`/`optionalTools`・
  `tools.allow` 照合は両名不変。content gate は surface 到達
  可能な subset で評価（`Process`→`any(fixed)`、
  `Command`→`any(!fixed)`）。

### R7 — pin / spawn / prepare 統一

- `pin_exec_grant`（仮名 — 現 `pin_command_grant` と同型）を
  unified grant へ適用。scope 構築時に `names` 含 `Command` →
  `!fixed` grant 群、`names` 含 `Process` → `fixed` grant 群を
  ambient PATH で pin し pin 不能は構築 fail（degradable 時は
  縮退）。**task executable が pin されるのは本 issue が初**で、
  per-call `prepare.rs:127-136` の lazy resolve 経路は除去する。
- `verify_pinned_executable`（spawn 前再検証）を両 surface で
  共有。`check_exec_execution`（仮名 — 現
  `check_command_execution` を一般化）が grant 解決→argv 照合
  （非 fixed のみ）→cwd→pin verify の単一 predicate で、
  `command::execute` と `route::resolve` の両方が呼ぶ。
- unified prepare（現 `tools/process/prepare.rs` の責務を
  grant 化）: cwd 解決 → manifest normalize → declared-path deny
  （`inputs`/`outputs` 保持 grant のみ）→ writeScope coverage
  （`fixed` grant のみ — R8）→ spawn 準備。
- `ProcLimits`/`ProcPrepared`/`ProcIdentity`/`minimal_env`/
  `prepare_cwd`/`prepare_exec_cwd`/tree cleanup/bounded output は
  既存 `tools::proc` 層をそのまま共有。identity は surface 別
  （fixed: `details.task`・`"process {name}"`・`report_duration_ms:
  false`。非 fixed: `details.{grant,argv}`・`"command {name}"`・
  `report_duration_ms: true`）を保持し details schema 契約を
  変えない。

### R8 — writeScope（M4）

- `names` 含 `Command` + `!fixed` grant 非空 + `write_scope` →
  scope 構築時 `POLICY_VIOLATION`（現行 `registry.rs:1150-1159`
  と同型の fail-closed を維持 — opt-in tool にも hard のまま）。
- `names` 含 `Process` + `write_scope` → 構築は許可、per-call
  coverage を unified prepare で検査（`fixed:true` grant の
  宣言 `outputs ⊆ writeScope` でないと `POLICY_VIOLATION` —
  現行 `prepare.rs:96-122` と同型）。
- argv 供給 grant（`!fixed`）は writeScope 下で常に拒否 —
  C5「resource 層が観測不能な副作用を宣言させない」invariant
  を per-call に写す形を維持。

### R9 — route 境界

- grant 参照（`route.grant`・将来の grant-set `grants[]`・
  `substitute.grant`）は `scope=="trusted" && !fixed` の grant のみ
  解決 — project-scoped または fixed grant への参照は
  `Viability::NoGrant`（存在しない扱い）。
- `project_grant` 投影は routed workflow 用 policy を
  `{route.grant} ∪ {fixed grants}` へ限定する形へ一般化
  （非 fixed の他 grant を隠す C9 confused-deputy 防止を保存し、
  routed workflow から task を呼べる現行挙動も保存）。
- viability precheck（`check_exec_execution`）・`expected_input`
  `{name:route.grant,args:exec_args}`・exec-input byte 一致・
  exit mirror・grant timeout deadline anchor は不変。
- route context（`extra_env`/`exec_cwd`/`accept_any_exit_code`/
  `log_tee`）は非 fixed surface のみに注入される（現行不変）。

### R10 — `security.execDenyPatterns`（D6）

- compiled deny predicate を unified grant 全体へ uniform に適用
  （#164 が task 適用を deferred していた分岐を merge で解消）:
  1. `resolve_exec_policy` — declared basename 照合（現
     `command.rs:300-312` 相当。matched spec は
     `InvalidRuntimePolicy` + `details.{grant,executable}` で
     policy load fail）。
  2. `pin_exec_grant` — resolved basename 再評価。
  3. `verify_pinned_executable` — spawn 前再適用。
- 既定=空で現行 no-op 不変。denied project task は policy load
  失敗（denied grant と同じ fail-closed — opt-in 宣言の効果が
  exec surface 全体に及ぶ operator 契約と一致）。

### R11 — type fate / migration

- `ResolvedCommandPolicy`・`ResolvedProcessPolicy`（top-level
  resolved model として）は `ResolvedExecPolicy` + `ExecGrant`
  へ置き換え。`resolve_process_policy` は `process` section の
  boundary validation + defaults の section resolver として残し
  （`tasks` map の検査・env/cwd 規則・RestrictConcurrency 適用後の
  merged 値を供給）。`ProcessTask`/`resolve_process_task`/
  `process_task_resources`・`CommandGrant`/`resolve_command_grant`/
  `command_grant_resources` は unified 版へ置き換え。
- policy.yaml 移行: 不要 — `process` 節・`command` 節の
  authoring shape は不変。loader が内部で unified grant へ写像。
- 破壊的変更（migration guide 対象、issue 7 と整合）:
  a) `command.grants`×`process.tasks` 同名の collision は
     `InvalidRuntimePolicy`（現行は別 namespace で共存可能）。
  b) `execDenyPatterns` が task executable にも効く（opt-in
     pattern を設定している operator のみ影響）。
  c) task executable が scope 構築時 pin で fail-closed 化
     （unresolvable task は per-call error → 構築 error/degrade）。
  d) `RuntimePolicy` serialized 形状（`exec` field — 内部 wire
     + golden fixture のみ）。
- `policy grant add|list|remove`・`--grant` flag・route `grant`
  用語は不変（`command.grants` authoring 面を操作し続ける）。

## Acceptance Criteria

- [ ] 統一 grant schema が `fixed`・optional `inputs`/`outputs`・
      `scope` provenance を保持し、`process.tasks` 由来 grant は
      `fixed:true` で model 入力 `{task}` のみ受付（M1/M2）。
- [ ] project `process.tasks` は `scope:"project"` fixed grant へ
      写像され、route の grant 参照（trusted+非 fixed のみ解決）
      を満たせない（M3 — project-scoped grant への参照は
      `Viability::NoGrant`）。
- [ ] writeScope 下の worker が `outputs ⊆ writeScope` の fixed
      grant を exec でき、outputs 超過・`!fixed` grant は拒否
      （M4 — `names` 含 `Command`+writeScope は構築時
      `POLICY_VIOLATION` を維持）。
- [ ] `{"kind":"tool","tool":"process","input":{"task":…}}` を持つ
      既存 workflow（`benchmarks/scenarios/scenarios.json`・
      golden fixture）が `UnknownTool` なしで動く（M5）。
- [ ] `execDenyPatterns` に match する task executable は policy
      load で `InvalidRuntimePolicy`（declared deny）、pin・spawn
      前再検証も全 grant へ uniform に効く（#164 適用点 1-3 の
      統合）。
- [ ] `command.grants` 名と `process.tasks` 名の衝突は
      `InvalidRuntimePolicy` fail-loud（load・
      `validate_policy_schema`・`policy grant add` の各経路で
      同じ failure）。
- [ ] routed workflow は `{route.grant}` + fixed grants を
      `policy.exec` から見る（他の非 fixed grant は投影外 —
      confused-deputy 防止保存 + task 呼出し現行挙動保存）。
- [ ] task executable が pin 不能（PATH 解決失敗・denylist）の
      scope は構築時 `POLICY_VIOLATION`（`Process` が degradable
      宣言されている場合は縮退）。
- [ ] 既存契約 test（pin・env_clear・stdin null・process group・
      lifecycle・fs boundary・writeScope・deny）が回帰なく pass。
- [ ] serialized `RuntimePolicy` の `exec` が invocation parity
      check で round-trip する（worker scope の
      `to_value(&policy.exec)` → 再 resolve → 一致）。

## Deferred Design Concerns

- `policy exec` への CLI 改名 | 現行 `policy grant` の用語維持で
  十分 | 用語の統一需要が発生した場合 | authoring 破壊的変更に
  なるため本 issue では対象外。
- `!fixed` grant の `inputs`/`outputs` を trusted 層が declare
  した場合の semantics — schema は受理するが writeScope 下では
  argv 供給として拒否される（M4 の規則で矛盾なく閉じる。
  resource 推論は per-path manifest を使う）。
- `process` alias を vocabulary alias（`TOOL_ALIASES`）へ移す案 —
  canonical 名のまま残す方が worker `tools:`/`tools.allow`/
  `optionalTools` の literal 照合を変えずに済む。alias 表現は
  実装層に留める決定（ADR-034KOpPArDTHQ8W1c81Q0B 参照）。
- issue 5 の substitute mode で `substitute.grant` が fixed grant
  を指す経路 — route eligibility の `!fixed` 制約で NoGrant に
  なるため問題化しないが、固定 argv への substitute は設計上
  意味を持たない（issue 5 側の設計論点として残置）。

## Sources and Relations

- 上流: `docs/discovery/unified-exec-grant-model-issue-165.md`
  （DISC-034KNX4OP8iBFRKTREaigz — 全 inventory・line drift
  訂正・論点一覧）
- 親 design: `docs/designs/permission-model-responsibility-redesign.md`
  （Option D 統合決定 + Merge Fidelity M1-M5 + issue 分解 3/7 +
  Data/Control Flow の loader 写像記述）
- 前提実装: `docs/impl/ir/exec-contract-content-audit-removal-issue-163.md`
  （#163 merged）・`docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md`
  （#164 merged — deny predicate・適用点・task 除外 deferred）
- 関連 spec: `docs/specs/managed-process-canonical-tool.md`（process
  tool 契約）・`docs/specs/policy-gated-command-tool.md`（grant
  契約・pin B1/B2）・`docs/specs/opt-in-exec-deny-patterns.md`
  （deny semantics — task 適用は本 spec R10 で拡張）
- 下流依存: issue #166（route grant-set 投影 — R9 の統一投影面を
  使用）・issue #169（文書正直性 sweep — 二重実装記述の収束）
