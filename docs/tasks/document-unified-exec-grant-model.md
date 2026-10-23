---
id: "TASK-034KQsvyJ7l63g3i4Gjnwl"
type: "task"
status: "done"
title: "unified exec grant model の guide/reference 文書更新"
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
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/unified-exec-grant-model.md"
  implemented-by: []
  depends-on:
    - "docs/plans/unified-exec-grant-model.md"
    - "docs/tasks/add-core-exec-policy-module-and-loader-wiring.md"
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
    - "docs/guides/managed-process.md"
    - "docs/guides/command.md"
    - "docs/guides/configuration.md"
    - "docs/guides/workers.md"
  defers: []
  deferred-by: []
---

# unified exec grant model の guide/reference 文書更新

## Work

plan `docs/plans/unified-exec-grant-model.md`
（PLAN-034KQs6XKnAqahkSVke78h）の T4。T1 で確定した
unified exec grant model の**公開契約**を guide/reference
文書へ反映する。authoring surface（policy.yaml 節名・shape・
`tool:"process"`/`tool:"command"` vocabulary・`policy grant`/
`--grant`）は不変のまま、`process.tasks`/`command.grants` が
単一 `exec` grant namespace へ解決されること・`fixed`/`scope`
field・`execDenyPatterns` の全 grant 適用・writeScope tri-
state・route eligibility を正確に文書化する。code
implementation は前提（T1 の型/schema が語彙・例の source of
truth）だが、T2/T3 の downstream compile には依存しないため
T1 完了後に並列着手可。

design の Documentation Note（M5 互換維持・authoring 例）と
TSPEC の rule を文面 source とし、**実装と矛盾する例・文言
（issue #169 の documentation honesty 観点）を残さない**。

### 1. `docs/guides/managed-process.md`

- `process.tasks` が `fixed:true` grant として unified `exec`
  namespace へ解決されることを追記（executable・argv literal・
  lifecycle default は process section defaults から fold）。
- task name が project policy 由来の場合 `scope:"project"`
  grant として解決される provenance 規則・deep-merge shadow
  でも `scope:"project"` が保存されることを明記。
- `execDenyPatterns` が task executable にも均一適用
  （declared 解決・pin・spawn 前の 3 点）されることを明記。
- writeScope coverage の tri-state（`outputs:Some(nonempty)`
  → coverage 必須・`Some([])` → trivially 成立・`None` →
  `Some(writeScope)` で fail-closed）を正確に記述。
- authoring YAML 例（`process.tasks` shape）は不変で維持。

### 2. `docs/guides/command.md`

- `command.grants` が `!fixed` grant として unified `exec`
  namespace へ解決されること・argv pattern（`ArgPattern`
  契約）が grant 契約概念であることを追記。
- trusted policy YAML では grant spec に `fixed`/`inputs`/
  `outputs`/`scope` を宣言できる（T1 schema 拡張）が、
  `--grant`/`policy grant add` 入口では `fixed:true`/`scope`
  は reject される entry-point validation 規則を区別して記述。
- route 対象は `scope:"trusted" && !fixed` grant のみ
  （route eligibility）— project-scoped/fixed grant は route
  不能であることを明記。
- `execDenyPatterns` が grant executable に declared/pin/
  spawn 前で均一適用されることを明記。

### 3. `docs/guides/configuration.md`

- `process`/`command` 両 section が **shared `exec` grant
  namespace** へ解決される resolver model（section resolvers
  → unified 解決）を section 説明へ追記。
- grant name の cross-section collision（`command.grants` key
  ∩ `process.tasks` key 非空）が `InvalidRuntimePolicy` +
  `details.{grant,task}` で拒否される fail-closed 規則を明記。
- `command` grant spec に `fixed`/`inputs`/`outputs`/`scope`
  field が追加された schema を field reference へ反映
  （`paths` schema・`scope` enum `"trusted"|"project"`）。
- `process.tasks`/`command.grants` の authoring shape・既定
  lifecycle value（`max_concurrent:1`/`timeout:"2m"`/
  `max_output_bytes:65536`/`kill_grace:"2s"`/`heartbeat:
  "30s"`）は不変で維持。
- `security.execDenyPatterns` の適用対象が全 exec grant
  （command grant + task-derived fixed grant）へ拡張された
  ことを明記。

### 4. `docs/guides/usage.md`・`workflow-usage.md`

- `usage.md` — run/worker 実行の `command`/`process` tool
  surface 説明で grant namespace の unified を一文で反映
  （`tool:"command"`/`tool:"process"` canonical 名不変）。
- `workflow-usage.md` — `tool:"process"`/`tool:"command"` の
  workflow `tool` field 使用例は不変で維持。resource
  inference が `exec` grant の `inputs`/`outputs` を参照する
  （`Some([])` = 副作用非宣言契約）ことを明記。

### 5. `docs/guides/workers.md`・`parent-agent-integration.md`

- `workers.md` — writeScope/`process`/`command` tool 配下の
  worker spec 説明を unified grant model へ整合（既に部分
  更新済みの箇所は検証・齟齬を修正）。writeScope 下で
  `command`（`!fixed` grant）が構築時 gate で拒否される
  （`"command is incompatible with worker writeScope"`）・
  `process` task が `outputs ⊆ writeScope` coverage を要求
  することを正確に記述。
- `parent-agent-integration.md` — delegation/policy 経由の
  tool surface 説明で grant 概念を unified へ整合。

### 6. `crates/cheaper-cli/resources/policy.yaml`

- embedded default policy resource に `exec` model 前提の
  comment/example を反映（`command.grants`/`process.tasks`
  authoring shape は維持、`exec` grant namespace 解決・
  `execDenyPatterns` 適用を doc comment で明記）。binary へ
  embed されるため wording のみ（schema/動作は変えない）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/unified-exec-grant-model.md`
  （PLAN-034KQs6XKnAqahkSVke78h — T4）
- `docs/designs/unified-exec-grant-model.md`
  （DESIGN-034KP4lfgfD7DdGiDV1YuT — Documentation Note・
  authoring surface 維持・M5 互換が source）
- `docs/specs/unified-exec-grant-model.md`
  （SPEC-034KOpKfZpegX2MLPD0uq6 — R1/R5/R10/R11）
- `docs/test-specs/unified-exec-grant-model.md`
  （TSPEC-034KQs1NIL8ahirht6QteV — `fixed`/`scope`・collision・
  deny・writeScope tri-state・route eligibility Rules）
- `docs/tasks/add-core-exec-policy-module-and-loader-wiring.md`
  （TASK-034KQsi5CcOH2MsHWhWWnC — 前提。型/schema の語彙
  source）

## Dependencies

- `docs/tasks/add-core-exec-policy-module-and-loader-wiring.md`
  （T1） — 型名（`ExecGrant`/`fixed`/`scope`/`exec` namespace）・
  schema field（`fixed`/`inputs`/`outputs`/`scope`）・error
  文言の source of truth が必要。T1 で確定する語彙を文書化
  するため T1 完了後に着手。
- T2/T3 の compile には依存しない（文書のみのため T2/T3 と
  並列可 — DAG 上は T1 のみに依存、T5 へ合流）。
- issue #169（documentation honesty sweep）と同方向 — 実装
  と矛盾する例・文言を残さない基準で更新する。

## Affected Files

- `docs/guides/managed-process.md`（modify）
- `docs/guides/command.md`（modify）
- `docs/guides/configuration.md`（modify）
- `docs/guides/usage.md`（modify）
- `docs/guides/workflow-usage.md`（modify）
- `docs/guides/workers.md`（modify — 部分更新済みの検証・整合）
- `docs/guides/parent-agent-integration.md`（modify）
- `crates/cheaper-cli/resources/policy.yaml`（modify — comment/
  example のみ・schema/動作不変）

## Done When

- [x] `managed-process.md` が task→`fixed`/`scope:"project"`
      grant 解決・`execDenyPatterns` 3 点適用・writeScope
      tri-state を正確に記述する
- [x] `command.md` が `!fixed` grant 解決・argv pattern 契約・
      `fixed`/`inputs`/`outputs`/`scope` field・entry-point
      reject・route eligibility（`trusted && !fixed`）・deny
      適用を記述する
- [x] `configuration.md` が shared `exec` namespace・collision
      fail-closed（`details.{grant,task}`）・grant spec の 4
      field 追加・`execDenyPatterns` 全 grant 適用を field
      reference へ反映する
- [x] `usage.md`・`workflow-usage.md`・`workers.md`・
      `parent-agent-integration.md` が unified grant model
      へ整合し、`tool:"process"`/`tool:"command"` canonical
      名・authoring shape を不変で維持する
- [x] `policy.yaml` が `exec` model 前提の comment/example を
      持つ（schema/動作不変）
- [x] 全更新文書が実装語彙（T1 の型/schema/error 文言）と
      矛盾しない（issue #169 honesty 観点 — 検証で drift を
      0 にする）

## Out of Scope

- code implementation（T1/T2/T3 — 本 task は文書のみ）
- `docs/designs/`・`docs/specs/`・`docs/adr/`・plan/test-spec
  本体の更新（既に approved・本 task は guide/reference のみ）
- README index・document frontmatter の status update（T5 で
  実施）
- 新規 guide の追加（既存 file の整合更新のみ）

## Verification

- [x] `node scripts/check-doc-ids.mjs` exits 0（文書 frontmatter/
      index の id/link 整合）
- [x] guide/reference 内の `process.tasks`/`command.grants`/
      `execDenyPatterns`/`writeScope`/`tool:`/`fixed`/`scope`
      記述が T1 の実装語彙・error 文言と一致（grep で drift
      検査 — 実装未確定の語彙は design/spec の確定文言を引用）
- [x] 更新 file に実装と矛盾する例・文言（`process.tasks`
      独立 namespace 前提・`execDenyPatterns` の command 限定
      記述等）が残っていない
