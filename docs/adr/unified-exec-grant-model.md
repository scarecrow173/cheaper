---
id: "ADR-034KOpPArDTHQ8W1c81Q0B"
type: "adr"
status: "proposed"
title: "unified exec grant model — 単一 exec grant 実装への統合"
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
    - "../specs/unified-exec-grant-model.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/managed-process-canonical-tool.md"
    - "../specs/policy-gated-command-tool.md"
  refines:
    - "../specs/opt-in-exec-deny-patterns.md"
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
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "../specs/unified-exec-grant-model.md"
    - "../discovery/unified-exec-grant-model-issue-165.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# unified exec grant model — 単一 exec grant 実装への統合

## Context and Problem Statement

`command.grants` と `process.tasks` は「宣言済み executable の許可
+ argv 契約（固定 or pattern）+ policy 管理」の同一概念を、schema・
resolve・pin・manifest・spawn・op declaration・test・文書の全てで
二重実装している（`crates/cheaper-core/src/policy/{process,command}.rs`・
`crates/cheaper-tools/src/tools/{process,command}/` — 全 inventory は
`docs/discovery/unified-exec-grant-model-issue-165.md` 参照）。
不整合の余地が構造的に存在し、#163 の内容審査除去で両側の審査
分岐が消えた今が統合差分コスト最小のタイミングである。親 design
（`docs/designs/permission-model-responsibility-redesign.md`）は
Option D capability-grant 統合と Merge Fidelity M1-M5 を採用済み
であり、本 ADR はその配下で統一モデルの具体的形を決定する。

未決の主要論点: 統一 schema 形状・fixed argv 表現・provenance
表現・loader 写像箇所・tool surface 戦略・`execDenyPatterns` の
task 適用（#164 で deferred）・route eligibility・legacy 型の帰結・
migration 戦略。

## Decision Drivers

- **不変条件**: A1-A7（層順序・invocation key 制限・ProjectPartial
  command/route 節除外・RestrictConcurrency・argv pattern・cwd/env
  境界）、B1/B2（pin + spawn 前再検証）、C1-C5（env_clear・stdin
  null・lifecycle・fail-closed admission・writeScope 副作用制限）、
  D2（routing≠authorization）— 親 design の invariant 一覧に従う。
- **Merge Fidelity**: M1（optional inputs/outputs + per-path
  manifest）・M2（`fixed` marker）・M3（project `process` 節維持 +
  project-scoped namespace・route 参照は trusted のみ）・M4
  （writeScope coverage 維持）・M5（`tool:"process"` compat alias）。
- **fail-closed 原則**: 承認の silent 喪失・weakening を生まない
  （collision・deny・pin・unknown grant/task の全て fail-loud）。
- **後方互換**: policy.yaml authoring surface・`tool:"process"`
  workflow・`policy grant`/`--grant` CLI は不変でなければならない。
- **KISS/DRY**: 機構の単一化を優先し、namespace・pool・budget 等の
  意味論差分は明示的に残す（false abstraction を避ける）。

## Considered Options

### O1 統一 schema 形状

- **O1a 単一 grants map + `fixed`/`scope` flags**（採用）—
  `BTreeMap<String, ExecGrant>` に全 grant を格納し、`fixed` が
  argv 契約種別、`scope` が provenance を表す。親 design の
  「merged grant 集合」「単一の `command.grants` モデルへ畳み込み」
  の記述に合致。
- **O1b 2 map 維持（型のみ統一）** — `grants`（trusted・非
  fixed）と `tasks`（fixed・scoped）を別 map で保持。collision
  が構造的に起きず scope check も不要だが、`input.task`/
  `input.name` が別 map を見るため resolution 層の統合が不完全で、
  M3 の「project-scoped grant namespace」を map 分離で表現すると
  統一モデルの benefit（単一 resolve/pin/deny 経路）が薄れる。
- **O1c `fixed` を argv 値で表現（`argv` 2 意味化なし）** —
  `fixed` field を `Vec<String>`（固定 argv 列）にして
  `fixed:bool` を持たない案。`fixed` 有無で argv 契約が分かれる
  点は同じだが、`Option<Vec>` vs `bool+Option<Vec>` で serialized
  shape・schema・round-trip が複雑化し、`fixed:false` の意味が
  「argv 個数無制約」と混線しやすい。

### O2 fixed argv の保持

- **O2a `fixed:bool` + `argv` を契約値として共用**（採用）—
  fixed grant の `argv` は宣言 argv そのもの（literal）、非
  fixed grant の `argv` は pattern。flag で意味が決まる 1 field。
- **O2b 別 field（`fixed_argv`/`args`）** — task argv を独立
  field に分離。意味が明確だが grant shape が増え、task 由来・
  非 fixed の両方で null が増える。`argv` 1 本で「argv 契約」を
  表す方が schema が小さい。

### O3 provenance 表現

- **O3a resolved grant の `scope` field（`trusted|project`）**（採用）
  — serialized parity check・route eligibility・deny・diagnostics
  の全てが grant 自身で判断できる。
- **O3b loader metadata（別集合）** — `HashSet<String>` の
  project 宣言名を別途保持。resolved grant の self-contained 性が
  失われ、serialized `exec` の round-trip では provenance を
  再導出できない（parity check が破れる）。
- **O3c scope を task にのみ付与** — `command.grants` は scope
  field 無しで trusted 扱い。型が不均一になり、`command.grants`
  で `scope:"project"` を declare する self-weakening 経路も
  表現できない。

### O4 loader 写像箇所

- **O4a `load_policy` + `normalize_policy` の統一 resolver**（採用）
  — project merge 前に宣言名集合を捕捉し、merged `process` 節と
  `command` 節を `resolve_exec_policy` へ渡す。`validate_policy_schema`
  の再 resolve・invocation parity・`policy grant` write 経路も同一
  resolver を通るため collision/deny が全入口で一致する。
- **O4b `resolve_process_policy` 内部で写像** — section resolver
  だけでは project 由来情報が届かず provenance を付けられない
  （resolver は layer 非依存 input のみ受ける設計）。
- **O4c `RuntimePolicy` 構築後の別 pass** — `normalize_policy`
  の後段で `process.tasks` を grants へ追記。2 pass の不整合窓
  （collision が normalize の schema resolve で捕捉されない）が
  残る。

### O5 tool surface 戦略

- **O5a disjoint surface + 実装層 alias**（採用）— `command` は
  `!fixed` grant のみ解決、`process` は `fixed` grant のみ解決する
  canonical entry として残し、内部で unified exec 機構を共有。
  M2 の「`{name}` のみ入力」契約と現行 `process`/`command` の
  details schema 差分を保存できる。
- **O5b 統一 surface** — `command` が `args:[]` で fixed grant も
  呼べるようにする。M2 の「args 不許可」を schema で表せず
  （`{name}` と `{name,args}` の区別が per-grant に依存）、
  process の details 契約とも乖離する。
- **O5c `process` を vocabulary alias へ格下げ** —
  `TOOL_ALIASES` へ移し canonical `command` へ解決。worker
  `tools:`・`tools.allow`・`optionalTools` の literal 照合が全て
  `command` 名へ変わり、authoring/workflow 面の破壊的変更になる
  （M5 に反する）。

### O6 `execDenyPatterns` 適用範囲

- **O6a unified grant 全体へ uniform 適用**（採用）— 3 適用点
  （resolve declared・pin resolved・spawn 前再検証）を全 grant で
  共有。#164 が「動的 argv を渡す grant の executable」を縛ると
  定義した deny semantics を、統合後は「exec grant の executable」
  へ一般化する。
- **O6b 非 fixed のみ（#164 除外維持）** — task executable に
  deny が効かない。project task が operator 宣言の deny
  executable を走らせる hole が merge 後も残り、統合の趣旨
  （trust boundary を scope field で表現）と矛盾する。
- **O6c scope=trusted のみ** — provenance で deny 適用を分岐。
  trusted task（user 層宣言）は deny が効くべきであり、project
  task にも効かせないと hole が残る点で O6b と同じ欠点。

### O7 route eligibility

- **O7a `scope=="trusted" && !fixed`**（採用）— `route.grant`・
  将来の `grants[]`/`substitute.grant` は trusted non-fixed grant
  のみ解決。A3（ProjectPartial command 節除外）が保証する
  「route は trusted 宣言の grant のみ使える」を scope field で
  明示化し、fixed grant が exec_args で誤参照されることも防ぐ。
- **O7b scope のみ検査（fixed 不問）** — fixed grant への参照は
  `check_exec_execution` の argv 照合で自然に失敗するため動作は
  同じだが、`expected_input`/`executed` record が「argv 供給を
  受けない grant」を実行した形になり semantics が汚れる。
- **O7c grant 側に `routable` flag** — explicit opt-in を別 field
  で持つ案。新しい authoring 面が増え、`scope+!fixed` で十分に
  表現できる責務を重複させる（YAGNI）。

### O8 `project_grant` 投影

- **O8a `{route.grant} ∪ {fixed grants}` を残す**（採用）— 現行は
  `policy.command` のみ投影され `policy.process` は全 task が
  routed workflow から呼べる。統一後も同じ可視性を保存
  （confused-deputy 防止は非 fixed grant のみ隠す形で維持）。
- **O8b `{route.grant}` のみ** — routed workflow から task が
  消える（現行挙動の破壊 — project workflow が自身の task を
  呼べなくなる回帰）。
- **O8c `{route.grant} ∪ {project-scoped fixed grants}`** —
  trusted task 由来の fixed grant も隠す。現行（trusted task も
  routed workflow から呼べる）より弱い投影になり回帰。

### O9 legacy 型の帰結

- **O9a `ResolvedExecPolicy` + `ExecGrant` 新設、`RuntimePolicy.exec`**
  （採用）— `process`/`command` field 除去、`process` section
  resolver（`resolve_process_policy`）は validation/defaults の
  中間段として残し `ProcessTask`/`resolve_process_task`/
  `process_task_resources`・`CommandGrant`/`resolve_command_grant`/
  `command_grant_resources` は unified 版へ置換。
- **O9b `ResolvedCommandPolicy` を拡張して `process` field 追加**
  — command 型に task 由来 grant を混ぜる命名が誤解を生み、
  section 別 defaults（`process`/`command` 両方の
  maxConcurrent/heartbeat）をどこに置くか不明確。
- **O9c `ResolvedProcessPolicy`・`ResolvedCommandPolicy` を残し
  両者を `RuntimePolicy` に保持** — resolved 層が二重のまま残り
  unified 化の利益が消失。

### O10 migration 戦略

- **O10a authoring surface 不変 + loader 写像 + fail-loud collision**
  （採用）— `process`/`command` 節は不変で内部 resolved model のみ
  統一。破壊的変更は collision fail-loud・deny task 適用・
  task pin・serialized 形状に限定し migration guide（issue 7）で
  告知。
- **O10b policy.yaml 側も統一** — `process` 節を `command` 節へ
  畳み込む。M3（project `process` 節維持）に反し、project が
  exec を declare する唯一の経路が消える。
- **O10c 移行期の dual-read** — 新旧両 schema を受理し warn で
  移行を促す。内部型が統一されるため新旧 resolved model の
  共存は不要（loader 写像で旧 authoring を吸収できる）で
  YAGNI。

## Decision Outcome

採用: O1a + O2a + O3a + O4a + O5a + O6a + O7a + O8a + O9a +
O10a。

- 統一 grant `{executable, fixed, argv?, inputs?, outputs?, cwd,
  env, timeout, killGrace, maxOutputBytes, exitCodes,
  scope:trusted|project}` + `ResolvedExecPolicy { grants,
  command: ExecDefaults, process: ExecDefaults }` を
  `RuntimePolicy.exec` に載せる。
- task→grant 写像は `load_policy` 内で project 宣言名集合を
  捕捉し、`resolve_exec_policy`（仮名）が merged `process` 節 +
  `command` 節を single map へ畳み込む（collision fail-loud）。
- `command`（`{name,args}`→`!fixed`）/`process`（`{task}`→`fixed`）
  の disjoint surface を実装層 alias で維持。
- deny は 3 適用点を unified grant 全体へ uniform 適用。
- route eligibility は `scope=="trusted" && !fixed`、
  `project_grant` 投影は `{route.grant} ∪ {fixed grants}`。
- `ResolvedExecPolicy` + `ExecGrant` 新設で legacy 型を置換、
  authoring surface 不変のまま loader 写像で後方互換を確保。

### Consequences

* Good, because: schema・resolve・pin・manifest・spawn・deny・
  writeScope・route 経路が単一実装へ収束し、#163/#164 後の
  drift リスクを構造的に解消する。#166 の grant-set 投影・
  #169 の文書 sweep が乗る単一モデルが確定する。
* Good, because: authoring surface（policy.yaml・`policy grant`
  CLI・`--grant`・`tool:"process"` workflow）が不変で、operator/
  workflow 作者への破壊的変更を最小化できる。
* Bad, because: `command.grants`×`process.tasks` 同名の collision
  が新規 fail-loud 制約になり、現行は別 namespace で共存できた
  config が load fail へ変わる（migration guide 対象）。
* Bad, because: `execDenyPatterns` が task executable にも効く
  ようになり、opt-in pattern を持つ operator は task が load
  fail し得る（deny 適用の拡大は意図的強化）。
* Bad, because: task executable が構築時 pin で fail-closed 化し、
  PATH 変化の per-call 吸収がなくなる（B1/B2 への整合として
  受容。degradable 宣言時は縮退）。
* Bad, because: `RuntimePolicy` serialized 形状が `exec` へ変わり、
  golden fixture・invocation parity・route view・catalog の配管を
  一括移行する必要がある。

### Confirmation

- spec の受入条件（`docs/specs/unified-exec-grant-model.md` の
  Acceptance Criteria）が実装 AC。
- discovery inventory（`docs/discovery/unified-exec-grant-model-issue-165.md`
  の file:line 表）が移行対象の全接点を網羅していること。

## Implementation Plan

* Affected paths: `crates/cheaper-core/src/policy/{exec,process,
  command,schema,load,merge,capability,security}.rs`・
  `crates/cheaper-tools/src/{registry,catalog,invocation}.rs`・
  `crates/cheaper-tools/src/tools/{command,process,proc}/*`・
  `crates/cheaper-cli/src/{grant,policy,run,workflow_context,
  workflow_run,route/*}.rs`・`crates/cheaper-workflow/src/
  {schema,types,resource_inference,compiler,engine}.rs`・
  `crates/cheaper-runtime/src/run/mod.rs`・golden fixture・
  関連 test。
* Patterns to follow: `resolve_process_policy`/`resolve_command_policy`
  の boundary validation + defaults 畳み込み、`pin_command_grant`/
  `verify_pinned_executable` の pin/再検証、`check_command_execution`
  の shared predicate seam、`project_grant` の route 投影、
  invocation parity の serialize→再 resolve→`PartialEq`。
* Patterns to avoid: `process`/`command` の 2 系統 resolve・別々の
  manifest/pool/error 経路の維持、namespace 衝突の silent
  shadow、deny の layer 別適用分岐。
* Dependencies or configuration: 新規 dep なし。`policy/exec.rs`
  module 新設（仮名）。golden fixture・benchmarks・docs の
  churn は実装 issue の AC に含める。

## Verification

* [ ] 統一 grant schema・`fixed`・`scope`・optional
      `inputs`/`outputs` が resolved model に載る
* [ ] `process.tasks` → `fixed:true`+`scope:"project"` 写像・
      route grant 参照が trusted+`!fixed` のみ解決される
* [ ] writeScope 下で `outputs ⊆ writeScope` の fixed grant は
      exec 可、`!fixed` grant・outputs 超過は拒否
* [ ] `tool:"process"` 既存 workflow が `UnknownTool` で落ちない
* [ ] `execDenyPatterns` が unified grant 全体へ uniform に
      適用される（declared/resolved/spawn 前の 3 適用点）
* [ ] `command.grants`×`process.tasks` 同名で
      `InvalidRuntimePolicy` fail-loud
* [ ] `policy.exec` の serialize→再 resolve→`PartialEq` で
      invocation parity が通る
* [ ] 既存契約 test（pin・env_clear・stdin null・lifecycle・
      fs boundary・writeScope）が回帰なく pass

## More Information

- 上流: `docs/discovery/unified-exec-grant-model-issue-165.md`
  （DISC-034KNX4OP8iBFRKTREaigz — 実測 inventory・line drift
  訂正・open questions）
- 親: `docs/designs/permission-model-responsibility-redesign.md`
  （DESIGN-034KuDkWOmNXiBddEQ4XWK — Option D + Merge Fidelity
  M1-M5 + issue 分解 3/7）
- 兄弟: `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6 — 受入条件）
- 前提: `docs/adr/enforce-declared-contracts-over-content-audit.md`
  （宣言契約主義 — 統一モデルの根拠）・`docs/impl/ir/
  opt-in-exec-deny-patterns-issue-164.md`（deny 適用点 — O6 で
  task 除外を吸収）
- 下流: issue #166（route 投影の整合）・#169（docs 正直性 —
  二重記述の収束）
