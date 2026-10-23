---
id: "DISC-034KNX4OP8iBFRKTREaigz"
type: "discovery"
status: "resolved"
title: "unified exec grant model — command.grants / process.tasks の統合（#165）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/unified-exec-grant-model.md"
    - "../adr/unified-exec-grant-model.md"
    - "../designs/unified-exec-grant-model.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
  defers: []
  deferred-by: []
---

# unified exec grant model — command.grants / process.tasks の統合（issue #165）

## Exploration Goal（seed — briefing 前の最小記録）

issue #165（#151 実装分解 3/7）。`command.grants` と
`process.tasks` は同一概念（宣言済み exec の許可 + argv 契約 +
policy 管理）の二重実装であり、schema・resolve・pin・spawn・
test・文書が全て二重で不整合余地が構造的に存在する。#163 で
両側の内容審査分岐が除去された今、単一 exec grant モデルへ
統合する（Option D capability-grant 統合 — 親 design で採用
決定済み）。

### 統一 grant の保持情報（issue 本文）

`{name, executable, fixed?: argv固定, argv?: pattern, inputs?,
outputs?, cwd/env 宣言, lifecycle 宣言, scope: trusted|project}`

### Merge Fidelity 決定（全採用 — 親 design §Merge Fidelity）

- **M1** 統一 schema は optional `inputs`/`outputs` — 持つ grant は
  per-path manifest（`process.rs:392-396` の reads/writes/produces
  同等）+ declared-path deny・produces ownership・writeScope
  coverage 検査（`prepare.rs:71-122`）。持たない grant は従来の
  conservative root manifest（`command.rs:488-498`）。outputs 空
  = `ClaimMode::Shared`、非空 = `Exclusive`。
- **M2** fixed-argv は `fixed: true` marker — task 由来 argv のみ、
  model 入力は `{name}` のみ（args 不許可）。
- **M3** project 層の `process` 節を維持（deprecate しない）—
  project 宣言 tasks は provenance `project` の project-scoped
  grant namespace へ写像。route grant 参照（`route.grant`・
  `grants[]`・`substitute.grant`）は trusted-scoped のみ解決
  （A3 invariant 保全）。
- **M4** writeScope × exec — coverage check は **`fixed: true` かつ
  `outputs ⊆ writeScope` の grant のみ** per-call 適用。argv 供給
  grant は writeScope 下で拒否維持（C5）。runtime 構築時
  fail-closed（`registry.rs:1133-1142`）維持。
- **M5** `tool:"process"` workflow 互換 — canonical vocabulary に
  `process` を残し `input.task` を merged fixed grant 名へ解決
  する compat alias（UnknownTool fail-closed E7 に落とさない）。

### 統合対象

統一 grant schema（M1/M2）・`process.tasks` → grant loader 写像
（provenance scope — M3）・tool surface の `command` 統一 +
`tool:"process"` alias（M5）・writeScope per-call coverage（M4）・
pin/spawn/manifest/pool 機構の単一化。route の `project_grant`
投影・viability・record も統一モデルのみを見る（#166 と整合）。

### Acceptance Criteria（issue 本文）

- 統一 schema が `fixed`・optional `inputs`/`outputs`・
  provenance scope を保持（M1/M2）
- project `process.tasks` → `fixed:true` + provenance `project`
  grant 写像、project-scoped grant が route 参照を満たせない（M3）
- writeScope 下で `outputs ⊆ writeScope` の fixed grant は exec 可、
  outputs 超過 task・argv 供給 grant は拒否（M4）
- `tool:"process"` 既存 workflow が UnknownTool で落ちない（M5）
- 既存契約 test（pin・env_clear・stdin null・lifecycle・fs
  boundary・writeScope）が回帰なく pass

### seed の前提

- deps: #163（merged — PR #172）。#164 も完了済み（PR #173）で
  `security.execDenyPatterns` が grants に適用される — 統合後の
  grant モデルと deny の相互作用を briefing で確認。
- file:line 引用は issue 本文のもの（main `59088c2` 時点 —
  #163/#164 後の line drift を briefing で実測修正する）。

## Key Issues and Assumptions

briefing で解決した設計論点（決定は ADR-034KOpPArDTHQ8W1c81Q0B、
受入条件は SPEC-034KOpKfZpegX2MLPD0uq6 に昇華）:

- **D1 統一 schema 形状** — 単一 `ExecGrant` 型
  `{executable, fixed, argv?, inputs?, outputs?, cwd, env,
  timeout, killGrace, maxOutputBytes, exitCodes, scope}` +
  `ResolvedExecPolicy { grants, command: ExecDefaults,
  process: ExecDefaults }`。`RuntimePolicy` に `exec` field を
  新設し `process`/`command` field は除去。
- **D2 fixed marker** — `fixed: bool`。`command` surface は
  `!fixed` のみ、`process` alias は `fixed` のみ解決する
  disjoint surface。
- **D3 provenance 表現** — resolved grant 上の
  `scope: trusted|project` field。project 宣言 task 名集合を
  loader が捕捉して `project` へ写像。
- **D4 loader 写像箇所** — `load_policy` の project merge 後・
  `normalize_policy` 内で `process.tasks` → `fixed:true` grant
  へ写像する統一 resolver。invocation parity・
  `validate_policy_schema` 再 resolve も同一 resolver を通す。
- **D5 tool surface** — `command` を canonical exec surface、
  `process` を canonical vocabulary に残した実装層 alias
  （vocabulary alias ではなく shared exec 機構への薄い
  surface — `input.task` → `fixed` grant 名解決）。
- **D6 deny 相互作用** — `security.execDenyPatterns` を unified
  grant 全体へ uniform に 3 適用点で適用（#164 の task 除外
  決定を本 merge で吸収）。
- **D7 route scope check** — grant 参照は
  `scope==trusted && !fixed` のみ解決。`project_grant` 投影は
  `{route.grant} ∪ {fixed grants}` を残す（routed workflow の
  task 呼出し挙動を保存）。
- **D8 writeScope(M4)** — 構築時 fail-closed gate を surface
  別に維持（`Command`+writeScope=reject 不変）+ per-call
  coverage を unified prepare へ移設。
- **D9 pin** — 統一 `pin_exec_grant` を surface 到達可能な全
  grant へ適用（task 由来 grant は初めて pin される —
  fail-closed 強化として明記）。
- **D10 型の帰結** — `ResolvedCommandPolicy`/`ResolvedProcessPolicy`
  → `ResolvedExecPolicy` + `ExecGrant`。`resolve_process_policy`
  は section resolver として残存、`ProcessTask`/
  `resolve_process_task`/`process_task_resources`・`CommandGrant`/
  `resolve_command_grant`/`command_grant_resources` は統一版へ置換。
- **D11 lifecycle/pool** — per-grant lifecycle は全 grant 畳み込み
  済み（task 由来は process section 値 + `exitCodes:[0]`）、
  heartbeat/maxConcurrent/pool は origin 別に保存
  （fixed→`process`、非 fixed→`command`）。
- **D12 collision rule** — `command.grants` 名と `process.tasks`
  名の cross-section 衝突は `InvalidRuntimePolicy` fail-loud
  （新規制約として明記）。project task による trusted task の
  shadow は従来 deep-merge 規約を保存（merged spec →
  `scope:project`）。

## Side-by-side Inventory（実測 file:line — 2026-10-23 時点、#163/#164 後）

issue 本文の line 参照を全て再計測した。drift 訂正を含む対応表:

### policy schema / resolve

| 側 | 現行実装 | 観測 |
| --- | --- | --- |
| task | `crates/cheaper-core/src/policy/process.rs` `ProcessTask`:16-22（`command:Vec<String>` 固定 argv 全体・`inputs`/`outputs`・`cwd`・`env` — lifecycle field 無し） | `command[0]`=executable、`command[1..]`=argv。model は task 名以外を供給しない |
| grant | `crates/cheaper-core/src/policy/command.rs` `CommandGrant`:36-49（`executable`・`argv:Option<Vec<String>>`・`cwd`/`env`・per-grant `timeout`/`killGrace`/`maxOutputBytes`/`exitCodes`） | model が `args` を供給し argv pattern で照合 |
| task schema | `process.rs` `process_policy_properties()`:60-96 — `tasks` map（required `command`/`inputs`/`outputs`、max 128、name pattern `^[a-z][a-z0-9_-]{0,63}$`） | `inputs`/`outputs` は maxItems 256・相対 path |
| grant schema | `command.rs` `command_policy_properties()`:86-128 — `grants` map（required `executable` のみ、`argv`/`cwd`/`env` + per-grant lifecycle override） | `argv` 省略=個数無制約 |
| section lifecycle | `process.rs`:53-56 と `command.rs` が `DEFAULT_TIMEOUT`/`MAX_OUTPUT_BYTES`/`KILL_GRACE`/`HEARTBEAT_INTERVAL` を共有（`process.rs:50-56` コメント §(a)） | `{maxConcurrent, timeout, maxOutputBytes, killGrace, heartbeat:{interval}}` 同型 |
| task resolve | `resolve_process_policy`:188-300（section defaults・cwd/inputs/outputs 相対検査・env 規則・`tasks` へ JSON value 格納）/`resolve_process_task`:303-317（`POLICY_VIOLATION` "unknown or unauthorized process task"・`details.task`） | resolved task は `Map<String,Value>` の JSON、per-task lifecycle 畳み込み無し |
| grant resolve | `resolve_command_policy`:239-396（deny 適用点 1 `:300-312`・`ArgPattern::compile` 検証 `:316-323`・per-grant lifecycle 畳み込み `:351-383`）/`resolve_command_grant`:401-414（`POLICY_VIOLATION`・`details.grant`） | `BTreeMap<String,CommandGrant>` |
| argv pattern | `ArgPattern`:154-226 — `compile`:172-199（wildcard 位置検査のみ、内容審査は #163 で除去済み）/`matches`:204-225 | literal exact・trailing `*` prefix・末尾 bare `*` rest の 3 種 |
| task resources | `process_task_resources`:340-363 — `reads=inputs`・`writes=outputs`・`produces=outputs`、globstar `**` → `Subtree` | per-path manifest（issue 本文 `process.rs:392-396` → 実測 `:340-363`） |
| grant resources | `command_grant_resources`:419-429 — workspace root subtree `reads`+`writes`、`produces: None` | conservative root manifest（issue 本文 `command.rs:488-498` → 実測 `:419-429`） |
| grant 投影 | `ResolvedCommandPolicy::project_grant`:73-79 — `grants` を `route.grant` 1 件へ truncate（C9 confused-deputy 防止） | `policy.process` は投影対象外 → routed workflow は全 task を呼べる現行挙動 |

### spawn / pin / op declaration

| 側 | 現行実装 | 観測 |
| --- | --- | --- |
| task op decl | `tools/process/mod.rs` `process_operation_declaration`:103-118 — outputs 空=`ClaimMode::Shared`/非空=`Exclusive`(:108-112)、pool `{name:"process", maxConcurrent}` | 実測で :103-118（issue 本文 `process/mod.rs:108-112` は ClaimMode 行のみ） |
| grant op decl | `tools/command/mod.rs` `command_operation_declaration`:125-144 — 常時 `ClaimMode::Exclusive`(:134)、pool `{name:"command", maxConcurrent}`、root manifest | issue 本文 `command/mod.rs:135-143` → 実測 :125-144 |
| task input schema | `process_schema`:51-66 — `{task}` required、additionalProperties false | details schema :83-98（`{task,state,exitCode,signal,stdout,stderr}`） |
| grant input schema | `command_schema`:51-70 — `{name,args}` required、additionalProperties false | details schema :90-113（`{grant,argv,state,exitCode,signal,durationMs,stdout,stderr,logRef?,outputTokens?,outputTokensTruncated?}` — spec-mandated 差分） |
| task prepare | `tools/process/prepare.rs` `prepare`:53-138 — `prepare_cwd`:58-64 → manifest normalize :65-70 → 宣言 path deny :71-77 + normalized claim deny :78-95 → writeScope coverage :96-122 → executable resolve :123-136 | issue 本文 `prepare.rs:71-95`/`:96-122` は実測と一致 |
| task execute | `process/mod.rs` `execute`:213-263 — task 解決 → prepare → `ProcLimits`(section lifecycle + `accepted_exit_codes:[0]`):230-236 → `ProcPrepared{argv: task.command[1..]}`:237-242 → `ProcIdentity{task,prefix:"process",report_duration_ms:false}`:245-253 → `proc::run`:254-262 | 固定 argv・shell 不使用 |
| grant execute | `command/mod.rs` `execute`:460-551 — `check_command_execution`:488 → `ProcLimits`(per-grant folded lifecycle):500-513 → route `extra_env` 合成 :516-521 → `ProcPrepared{argv: args}`:522-527 → `ProcIdentity{grant,argv,prefix:"command",report_duration_ms:true}`:532-541 → `proc::run`:542-550 | model 供給 args |
| 事前 check | `check_command_execution`:402-439 — grant 解決 → `authorize_args`:444-458（`ArgPattern::compile` + `matches`）→ cwd（route `exec_cwd` override 込み）→ pin map 検索 + `verify_pinned_executable` | `route::resolve` と `command::execute` で共有する B#3 seam |
| pin | `pin_command_grants`:168-181（scope 構築・`names` 含 `Command` 時のみ）/`pin_command_grant`:190-217（`resolve_process_executable` + deny 適用点 2 `:208-215`） | task 側は per-call `prepare.rs:127-136` で都度 resolve — pin 機構を持たない |
| spawn 前再検証 | `verify_pinned_executable`:228-253 — `native_executable` 再解決 + pinned path 同一性 + deny 適用点 3 `:245-251` | B1/B2 fail-closed |
| 共有 spawn 層 | `tools/proc/mod.rs` `ProcLimits`:127-136・`ProcPrepared`:140-145・`ProcIdentity`:151-165・`minimal_env`:175-217・`denied_declared_path`:221-230・`prepare_cwd`:236-262・`prepare_exec_cwd`:268-290 / `proc/executable.rs` `native`:67-83・`native_executable`:90-92・`resolve_process_executable`:103-141 | spawn・env_clear・stdin null・process group・bounded output・tree cleanup は両側で既に共有 |

### runtime 構築 / tool 登録

| 側 | 現行実装 | 観測 |
| --- | --- | --- |
| 構築入口 | `registry.rs` `create_tool_runtime`:1095 — `ToolRuntimeOptions.process`/`command`/`security` の raw JSON を別々に resolve（process :1110-1112・exec_deny :1124-1129・command :1130-1132） | `InvocationToolRuntimeOptions` も同 3 field（`invocation.rs`:62-82） |
| process gate | `registry.rs`:1113-1123 — `names` 含 `Process` かつ `tasks` 空 → degrade or `POLICY_VIOLATION` "process exposure requires nonempty authorized tasks" | content-gate（design §(e)） |
| command gate | `registry.rs`:1133-1160 — 空 grants→degrade/fail(:1140-1149)、**非空 grants + `write_scope` → `POLICY_VIOLATION` "command is incompatible with worker writeScope"(:1150-1159)** | issue 本文 `registry.rs:1133-1142` → writeScope 分岐の実測は **:1150-1159**（gate 全体は :1133-1160） |
| pin 構築 | `registry.rs`:1219-1235 — `names` 含 `Command` のみ `pin_command_grants`（pin 不能は構築 fail） | `Process` 側 pin 無し（dormant task は不問） |
| catalog | `registry.rs`:1238 `ToolCatalog::from_resolved_policy(process)` / `catalog.rs` — `policy: ResolvedProcessPolicy`:43、Process arm :60-70（`input.task`→`process_task_resources`）、Command arm :75（input 非依存 root manifest）、`NAMES`:123-133、input schemas :363-364、details schemas :489-490、`from_resolved_policy`:596 | catalog は process policy のみ所有（command の manifest は input 非依存のため policy 不要の現行形） |
| op decl dispatch | `registry.rs`:878-897 — `ToolName::Process`→`process_operation_declaration(tool_ctx.process, input.task)`、`ToolName::Command`→`command_operation_declaration(tool_ctx.command, input.name)` | :927 `is_managed` = `Process|Command`（managed subprocess tracking は両側共通済み） |
| ToolCtx | `registry.rs`:220-243 — `process`/`command`/`command_pins`/`exec_deny` を別 field。`process()`/`command()` options builder :312-338 | `ToolCtx.command()` が `route_context` を注入（process 側は持たない） |
| registered def | `registry.rs`:633-643 — `Process`→`process_tool_definition(ctx.process())`、`Command`→`command_tool_definition(ctx.command())` | description は task/grant 名一覧埋め込み（`process/mod.rs`:138-147・`command/mod.rs`:312-321） |
| vocabulary | `crates/cheaper-core/src/tools/vocabulary.rs` — `TOOL_NAMES`(:11-21) に `process`/`command` が canonical、`ToolName` enum :27-37、`resolve` :74-77、`TOOL_ALIASES` :81-88（`Read`/`Grep`/`Glob`/`Write`/`Edit` のみ — exec alias は現状無し）、`is_scope_gated_tool` :99-101（両 exec tool のみ true） | worker `tools:`・`tools.allow`・`optionalTools` は literal 名照合 |
| capability | `capability.rs` `check_capabilities`:17-62 — worker tools ⊆ `policy.tools.allow` + tier 上限 | `is_mutating_tool`（`worker/validate.rs`:326-331）は `Write|Edit|Process|Command` — `savings.directlyComparable` 影響 |
| optionalTools | `worker/validate.rs`:254-292 — scope-gated tool のみ opt-in 可 | gated 集合は `is_scope_gated_tool` に依存 |
| invocation parity | `invocation.rs` — `create_worker_scope` の declared-shape parity: process :229-235・command :242-248・security :255-261、再注入 :285-304、`create_invocation_tool_runtime` の resolve :423-436、`acquire_job_lease` の catalog :172 | parity は serialize→再 resolve→`PartialEq` 比較 |

### loader / layer 境界

| 側 | 現行実装 | 観測 |
| --- | --- | --- |
| load 順 | `load.rs` `load_policy`:104-261 — builtin Complete validate → user Partial merge(Replace) → invocation → trusted 再 resolve → project ProjectPartial merge(RestrictConcurrency) → Complete validate → `normalize_policy` | A1 層順序 + fail-loud load |
| invocation guard | `load.rs`:120-151 — 合成 layer は `command.grants` のみ許可（key 制約 :125-151）、同名 collision fail-loud :165-182 | A2 invocation 層 key 制限 |
| 再 resolve | `load.rs` — `resolve_process_policy`(merged):198-205・`resolve_exec_deny_list`(merged security):206-212・`resolve_command_policy`(merged+deny):213-220 | deny 適用点 1 が invocation 合成 grant を含む merged map に発火 |
| project 境界 | `schema.rs` `policy_project_partial_schema`:351-366 — `process` 節許可(:357)・`command` 節除外（properties に `command` 無し :354-364） | A3 trusted-only grant authoring — project merge は `load.rs`:237-255（`RestrictConcurrency`） |
| merge 規則 | `merge.rs` `merge_policy_layers`:42-109 — `RestrictConcurrency` は `process.maxConcurrent`/`maxOutputBytes`/`timeout`/`killGrace`/`heartbeat.interval` を min clamp（:20-36）。`process.tasks`/`command.grants` は通常 deep-merge（同名 task は field 単位で project override） | project task 名の provenance は merge 前に捕捉する必要がある |
| normalize | `schema.rs` `normalize_policy`:516-… — `resolve_process_policy`(:589)・`resolve_command_policy`(raw command + exec_deny :585-590) を別々に呼び `RuntimePolicy.process`/`command` へ格納 | unified 化で `exec` field へ一本化する統合点 |
| schema 再 resolve | `schema.rs`:437-446 — `validate_policy_schema` が `resolve_process_policy`/`resolve_command_policy` を再実行して section 解決失敗を `InvalidRuntimePolicy` shape で返す | unified resolver もここに接続しないと collision 検査が validate 経路に届かない |

### route / hook

| 側 | 現行実装 | 観測 |
| --- | --- | --- |
| viability view | `route/resolve.rs` `CommandPolicyView`:70-82 — `policy: &ResolvedCommandPolicy` + root/symlink/denylist/exec_deny | supply が `policy.command` 参照を供給（`route/supply.rs`:45-53・load :70-134） |
| grant 解決+precheck | `route/resolve.rs`:310-343 — `policy.grants.get(&route.grant)`:314 → `pin_command_grant` pre-pin :317-326 → `CommandToolOptions`+`check_command_execution`:327-343 | issue 本文 `resolve.rs:304-335` → 実測 :310-343 |
| project_grant | `route/run.rs`:121 — `supply.policy.command.project_grant(&route.grant)` で routed workflow 用 policy を単一 grant へ投影 | `policy.process` は投影されず全 task が routed workflow から呼べる現行挙動 |
| exec-input 一致 | `route/run.rs`:176 `expected_input = {"name":route.grant,"args":exec_args}` / `workflow_run.rs`:476-489 `ctx.job.name=="command"` かつ `job_id=="exec"` で byte 一致検査（D11） | anchor は `route.grant`（issue 4/5 で substitute retarget） |
| grant timeout | `route/run.rs`:356-364 — `policy.command.grants[route.grant].timeout` → deadline | unified 化で `policy.exec.grants` 参照へ |

### workflow / resource inference

| 側 | 現行実装 | 観測 |
| --- | --- | --- |
| tool 名検証 | `workflow/schema.rs`:259-291 — `kind:tool` job は `ports.tools.get(job.name)` で未知 tool を `unknown tool` fail-closed(E7)。worker capability tool も同 port で検査 :287-291 | `tool:"process"` は catalog の canonical entry |
| job executor | `cli/workflow_run.rs` `ToolJobExecutor`:440-560 — `ToolName::from_canonical`:458、`names: vec![name]` の job scope :514、`tools.allow` admission :604-621、worker job は `check_capabilities`:630 | `tool:"process"` job は `policy.tools.allow` に `process` 必須 |
| process job 特別扱い | `resource_inference.rs` — `item.job.name == "process"` :123、`item.process` は process job 限定 :124-125、named-task input 必須 :131-137、catalog `tools.get("process").resources(input)` :142-152、declaration path :164-180（`ProcessResourceDeclaration{task,inputs,outputs}` — `types.rs`:249-254・`ResourceInferenceJob.process`:301） | compiler は `process: None`（`compiler.rs`:439 — declaration seam は外部/test 注入用） |
| error 分類 | `engine.rs`:374 `tools::process::find_cheaper_error` | re-export path 維持（`process/mod.rs`:40-45） |

### CLI / invocation layer

| 側 | 現行実装 | 観測 |
| --- | --- | --- |
| `--grant` | `cli/grant.rs` `build_invocation_layer`:44-99 — `{"version":1,"command":{"grants":{...}}}` 合成 layer | run 経路 `cli/run.rs`:178-180・warn :206-214 |
| `policy grant` | `cli/policy.rs` — `grant_add`:118-147・`grant_remove`:152-172・`write_validated`:254-266（`validate_policy_schema(Partial)` → `resolve_command_policy` 系規則 + deny 照合） | `command.grants` 管理面（user file） |
| invocation options 構築 | `runtime/run/mod.rs` — `policy.process`→`options.process`:337/:496、`policy.command`→`command`:340/:497、`policy.security`→`security`:344/:500 | serialize→parity 経路 |
| workflow 経路 | `cli/workflow_run.rs`:241-261 — `shared.policy.{process,command,security}` を同型 serialize（job scope では `None` で parity skip + 再注入 :506-511） | unified 化で `exec` 一つへ |
| catalog 構築 | `cli/workflow_context.rs`:224 `ToolCatalog::from_resolved_policy(policy.process)`・`:315-318` | unified policy 供給へ変更点 |

## Line Reference Drift 訂正表（issue 本文 → 実測）

| issue 本文 | 実測 | 備考 |
| --- | --- | --- |
| `process.rs:392-396`（task resources） | `policy/process.rs:340-363` | fn 全体 |
| `prepare.rs:71-95`（declared-path deny） | `process/prepare.rs:71-95` | 一致（宣言 path :71-77 + normalized claim :78-95） |
| `prepare.rs:96-122`（writeScope coverage） | `process/prepare.rs:96-122` | 一致 |
| `command.rs:488-498`（root manifest） | `policy/command.rs:419-429` | fn 全体 |
| `process/mod.rs:108-112`（ClaimMode） | `process/mod.rs:108-112` | 一致（fn は :103-118） |
| `command/mod.rs:135-143`（Exclusive） | `command/mod.rs:134` | fn は :125-144 |
| `registry.rs:1133-1142`（writeScope fail-closed） | `registry.rs:1150-1159`（gate 全体 :1133-1160） | **drift 訂正** |
| `schema.rs:340-355`（project partial） | `policy/schema.rs:351-366` | fn 全体 |
| `load.rs:119-150`（invocation guard） | `load.rs:120-151` | 一致 |
| `resolve.rs:304-335`（argv precheck） | `route/resolve.rs:310-343` | grant lookup :314 |
| `route/run.rs:176`（expected_input） | `route/run.rs:176` | 一致 |
| `command.rs:73-80`（project_grant） | `policy/command.rs:73-79` | fn 全体 |
| `command/mod.rs:214-244,273-278`（pin 時 2 段・workspace 再検査） | 除去済み（#163 — pin :190-217 は resolve+deny のみ、verify :228-253 は path 同一性+deny のみ） | 内容審査除去後の現行形 |
| `command.rs:249`（`ArgPattern::compile` 内 metachar） | 除去済み（`compile`:172-199 は wildcard 位置検査のみ） | #163 |
| `process.rs:278`（task argv metachar） | 除去済み（`resolve_process_policy` に metachar 分岐無し — `argv_shell_re`:120-123 は route/resolve 向けに残存） | #163 |

## Alternatives and Comparison

各論点の採否は ADR-034KOpPArDTHQ8W1c81Q0B に詳細。主要軸の比較:

| 論点 | 採用 | 却下候補 | 傾き |
| --- | --- | --- | --- |
| 統一 namespace | 単一 `grants` map + `fixed`/`scope`（merged grant 集合 — 親 design の指示どおり） | 2 map 維持（`grants`+`tasks` で型のみ統一 — namespace 衝突を避けられるが resolution 層の統合が不完全で M3 の "project-scoped grant namespace" 表現と不整合） | yes |
| 名前衝突 | `command.grants`×`process.tasks` 同名は `InvalidRuntimePolicy` fail-loud | silent shadow（片側優先 — 承認の silent 喪失）、複合 key（map 形状が不自然） | yes |
| deny 適用範囲 | unified grant 全体へ uniform（3 適用点） | non-fixed のみ（#164 R9 の除外を温存 — project task が deny executable を走らせる hole を残す）/trusted scope のみ | yes |
| surface 分離 | disjoint — `command`→`!fixed`、`process` alias→`fixed` | 統一 surface（command が `args:[]` で fixed grant も呼べる — M2 の args 不許可契約と曖昧化） | yes |
| task pin | 構築時 pin へ統一（B1/B2 を fixed grant にも適用 — 強化） | lazy per-call resolve 維持（process 現行挙動 — 第二の解決経路が残り pin 機構の統一にならない） | yes |
| writeScope 執行箇所 | 構築時 gate 維持（surface 別）+ per-call coverage を unified prepare へ | per-call のみ（構築時 fail-closed の放棄 — seed "runtime 構築時 fail-closed 維持" と不整合）/eligible grant 0 件で構築 reject（process の現行 per-call 寛容を破壊） | yes |
| `RuntimePolicy` field | `exec` 新設（`process`/`command` 除去 — 正直な命名） | `command` field を拡張維持（task 由来 grant を含むのに command 名は誤解） | yes |

## Tentative Conclusions and Hypotheses

全項目「確認済み」（親 design の採用決定 + 実測裏付け）:

1. **統一 grant 型** `{executable, fixed, argv?, inputs?, outputs?,
   cwd, env, timeout, killGrace, maxOutputBytes, exitCodes,
   scope:trusted|project}` — M1/M2/M3 を単一 schema で表現。
   `inputs`/`outputs` 保持 grant → per-path manifest
   （`process_task_resources` 同等 + declared-path deny・
   produces ownership・writeScope coverage）、非保持 grant →
   conservative root manifest。outputs 空=`Shared`/非空=`Exclusive`。
2. **loader 写像** — `load_policy` が project-declared task 名
   （ProjectPartial validate 後・merge 前の project doc から
   `process.tasks` keys）を捕捉し、merged `process` 節 +
   `command` 節を統一 resolver（仮 `resolve_exec_policy`）へ
   渡す。task→grant 変換は `fixed:true` + argv=`command[1..]` +
   executable=`command[0]` + lifecycle=process section 値 +
   `exitCodes:[0]` + scope(provenance)。`command.grants` →
   `fixed:false`+`scope:trusted`（authoring で `fixed`/`inputs`/
   `outputs`/`scope` も受理 — resolved shape の round-trip と
   parity check に必要）。
3. **surface disjoint 維持** — `process` tool は実装層 alias:
   canonical vocabulary・`{task}` schema・task 名 description・
   `{task,state,...}` details shape を不変で残し、内部は unified
   exec 機構（fixed grant 解決→unified prepare→`proc::run`）へ
   接続。`command` は `{name,args}` で `!fixed` grant のみ解決。
4. **deny uniform 化** — `resolve_exec_policy`（declared）・
   `pin_exec_grant`（resolved）・`verify_pinned_executable`
   （spawn 前）の 3 点で全 grant へ適用。#164 が除外した task
   適用を merge で吸収 — denied project task は policy load で
   fail（denied grant と同じ fail-closed 挙動）。
5. **writeScope(M4)** — construction gate を surface 別に維持:
   `names` 含 `Command` かつ `!fixed` grant 非空 + writeScope →
   `POLICY_VIOLATION`（現行 :1150-1159 と同型）、`names` 含
   `Process` + writeScope → 構築許可・per-call coverage（unified
   prepare で `fixed && outputs ⊆ writeScope` を検査 — 現行
   `prepare.rs:96-122` 相当を移設）。content gate は
   `any(fixed)`/`any(!fixed)` の subset 判定へ。
6. **pin 統一** — `names` 含 `Command`→`!fixed` grant 群を pin、
   含 `Process`→`fixed` grant 群を pin（per-scope・ambient PATH
   起点）。task executable が pin されるのは本 issue が初 —
   unresolvable task は scope 構築 fail（opt-in degradable 時は
   縮退）へ変わる tightening として明記。per-call の
   `verify_pinned_executable` も両 surface で共有。
7. **route 境界** — `route.grant`（将来の `grants[]`・
   `substitute.grant` も同 predicate）は
   `scope==trusted && !fixed` のみ解決 — project-scoped/fixed
   grant は `Viability::NoGrant`。`project_grant` は
   `{route.grant} ∪ {fixed grants}` を残す投影へ一般化（routed
   workflow が task を呼べる現行挙動保存 — #166 の grant-set
   投影へ整合）。route 実行の `expected_input`・lifecycle
   anchor は不変。
8. **型・配管の帰結** — `RuntimePolicy.exec`（`ResolvedExecPolicy`）
   新設、`process`/`command` field 除去。`ToolRuntimeOptions`・
   `InvocationToolRuntimeOptions`・`ToolCtx`・`InvocationInner`・
   `ToolCatalog`・`CommandPolicyView`→`ExecPolicyView`・
   `workflow_context`/`run`/`workflow_run` の serialize 配管は
   `exec` 一本へ。parity check は serialized `exec` の単一比較へ
   （現行 3 check → exec 1 + security 1 の 2 check）。
9. **migration** — policy.yaml の authoring surface（`process`・
   `command` 節）は不変。破壊的変更は (a) grants×tasks 同名の
   cross-section collision fail-loud、(b) deny patterns の task
   executable への拡大適用、(c) task executable の構築時 pin
   fail-closed 化、(d) `RuntimePolicy` serialized 形状（内部
   wire — golden fixture churn）に限定。issue 7 の migration
   guide 対象。

## Test / Fixture Churn 候補

- `cheaper-core`: `policy/process.rs` unit（:374-494）・
  `policy/command.rs` unit（:431-736）・`capability.rs`（:97-113 の
  RuntimePolicy リテラル — `exec` 形状へ）・`load.rs`/`schema.rs`/
  `merge.rs` unit。golden `policy_process.rs`・`policy_schema.rs`・
  `policy_load.rs`・`policy_merge.rs`・`policy_restrict.rs`・
  `model_resolve.rs`・`worker_resolve.rs` + fixture
  `policy/{process,schema,load,merge,restrict,workflow}.json`
  （serialized `RuntimePolicy` 形状変更で再採取）。
- `cheaper-tools`: `proc_process.rs`・`proc_command.rs`・
  `proc_invocation.rs`・`proc_runtime.rs`・`proc_degrade.rs`・
  `characterization.rs`・`differential.rs` + fixture
  `process.json`/`runtime.json`（options field `process`/`command`
  →`exec`）。
- `cheaper-workflow`: `tests/process.rs`・`compile_contracts/*`・
  `filesystem_contracts/resource_inference.rs`（process 推論経路 —
  `tools.get("process")` 不変で動く回帰確認 + `known_process` 分岐）。
- `cheaper-cli`: `run_dev_seam.rs`（deny/opt-in 系 :1053,:1093,
  :1152,:1262 付近）・`tests/integration/*`・`workflow.rs`
  （:519,:604 付近）・route 系（`route/run.rs`:712-736 は
  `policy.exec.grants` 参照へ）。
- `cheaper-runtime`: `run_dev_seam.rs`・`contracts/*`・
  `ollama_e2e.rs`・`dev/fixture.rs`（RuntimePolicy リテラル）。
- `benchmarks`: `scenarios.json`（`processPolicy` scenario 2 件 +
  `tool:"process"` jobs — authoring surface 不変のため無改訂で
  動く後方互換 AC）、`lib/cli.mjs`:58-64（`policy.process` 書込み —
  authoring surface 名のまま維持）。
- docs: `guides/managed-process.md`・`guides/command.md`・
  `guides/configuration.md`（process/command 節 + deny 記述）・
  `guides/usage.md`（`policy grant` CLI）・`guides/workflow-usage.md`
  （`tool:"process"`）・`guides/workers.md`（optionalTools/
  writeScope）— issue 7 の正直性 sweep に含める。

## Open Questions

- ~~統一 schema の `scope` 表現（enum field vs loader metadata）~~
  → resolved grant の `scope` field として解決（D3）。
- ~~deny の task 適用~~ → uniform 適用へ解決（D6）。
- 実装期の命名（`policy/exec.rs` module 名・`ExecGrant`/
  `ResolvedExecPolicy`/`ExecPolicyView`/`ExecToolOptions`）—
  design で確定、本 briefing は仮名で記述。
- `policy grant` CLI・`--grant` flag・`route grant` の用語は
  `command`/`grant` のまま維持でよいか → 維持（surface 名の
  変更は authoring 破壊的変更になるため対象外。#166/#169 で
  用語 sweep の整合を確認）。
- 構築時 pin へ移行した task executable が PATH 変化を拾わなく
  なる影響（session 中の PATH 変更 — 現行は per-call 解決で
  ambient 変化を吸収）。pin は B1/B2 の意図的強化として
  受容するが、degradable 縮退との組合せ（pin fail→`Process`
  縮退 or 構築 fail）は design で明示が必要 — spec の受入条件へ
  記録。
- `process` section heartbeat（`process.heartbeat.interval`）と
  `command` section heartbeat の per-grant 配線は origin 別
  保存で維持（D11）— 単一 heartbeat への統一は budget 意味が
  変わるため対象外。

## Promotion Candidates

- [x] spec-doc needed: yes → `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6 — 統一 schema・provenance・
  loader 写像・tool surface・writeScope/route/deny 意味論・
  受入条件）
- [x] adr-doc needed: yes → `docs/adr/unified-exec-grant-model.md`
  （ADR-034KOpPArDTHQ8W1c81Q0B — schema 形状・fixed marker・
  provenance・写像箇所・alias 戦略・deny・route eligibility・
  型の帰結・migration の決定と代替案）

## Sources and Relations

- issue #165・親 issue #151 chain（design/spec/ADR/discovery）
- #163 IR（内容審査除去後のコード状態）・#164 IR（execDenyPatterns）
- 派生: SPEC-034KOpKfZpegX2MLPD0uq6（`docs/specs/unified-exec-grant-model.md`）・
  ADR-034KOpPArDTHQ8W1c81Q0B（`docs/adr/unified-exec-grant-model.md`）
- 実測対象: `crates/cheaper-core/src/policy/{process,command,schema,load,merge,capability,security}.rs`・
  `crates/cheaper-tools/src/{registry,catalog,invocation}.rs`・
  `crates/cheaper-tools/src/tools/{command,process,proc}/*`・
  `crates/cheaper-cli/src/{grant,policy,run,workflow_context,workflow_run}.rs`・
  `crates/cheaper-cli/src/route/{resolve,run,supply}.rs`・
  `crates/cheaper-workflow/src/{schema,types,resource_inference,compiler,engine}.rs`・
  `crates/cheaper-runtime/src/run/mod.rs`
