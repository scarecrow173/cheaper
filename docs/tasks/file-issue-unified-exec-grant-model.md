---
id: "TASK-034Kvxwfa7svHN4AuX95dA"
type: "task"
status: "done"
title: "起票 issue 3: command.grants / process.tasks の単一 exec grant モデル統合"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/151"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  implemented-by: []
  depends-on:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
    - "docs/tasks/file-issue-exec-contract-content-audit-removal.md"
  blocks:
    - "docs/tasks/file-issue-route-grant-set-and-record-split.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "docs/tasks/record-issue-urls-and-verify-investigation-criteria.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/file-permission-model-redesign-implementation-issues.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/permission-model-redesign-issue-decomposition.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
  defers: []
  deferred-by: []
---

# 起票 issue 3: command.grants / process.tasks の単一 exec grant モデル統合

## Work

PLAN-034KvwX4x7oaZlaECqhHc1 の T3。DESIGN-034KuDkWOmNXiBddEQ4XWK
「§実装 issue 分解」の issue 3 を `gh` CLI で
`scarecrow173/cheaper` へ起票し、発行 URL を本 task の
「起票結果」欄へ記録する。

issue title / body は下記を**そのまま**使う — design 決定を
再解釈・再導出しない。body 内の `#<ISSUE-1>` placeholder は、
T1(`docs/tasks/file-issue-exec-contract-content-audit-removal.md`)
の起票結果に記録された実番号で置換してから起票する。

### 手順

1. `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 3/7" --state all --json number,title,url`
   で重複確認。既に存在すれば作成を skip し、その URL を
   起票結果へ記録して完了とする。
2. T1 task doc の起票結果から issue 1 の番号を取得し、下記 body
   内の `#<ISSUE-1>` を `#<実番号>` へ置換する。
3. 置換後の body を temp file へ書き出し、
   `gh issue create --repo scarecrow173/cheaper --title "権限モデル再設計 3/7: command.grants / process.tasks の単一 exec grant モデル統合" --body-file <temp file>`
   で起票する。
4. 発行 URL・番号を本 task の「起票結果」欄に記録する。

### issue title

`権限モデル再設計 3/7: command.grants / process.tasks の単一 exec grant モデル統合`

### issue body

````markdown
## 背景

親 issue #151 の実装分解 3/7。`command.grants` と `process.tasks`
は同一概念(宣言済み exec の許可 + argv 契約 + policy 管理)の
二重実装であり、schema・resolve・pin・spawn・テスト・文書が全て
二重で不整合の余地が構造的に存在する。内容審査の除去
(#<ISSUE-1>)で両側の審査分岐が同時に消えるタイミングで、単一
exec grant モデルへ統合する(Option D capability-grant 統合 —
採用決定済み)。

## Scope

統一 grant モデルは次の情報を保持する:
`{name, executable, fixed?: argv固定, argv?: pattern, inputs?,
outputs?, cwd/env 宣言, lifecycle 宣言, scope: trusted|project}`

Merge Fidelity 決定(design「§Merge Fidelity」— 全て採用。
単純な畳み込みは task 側情報を落として manifest・writeScope・
ClaimMode の semantics を壊すため、以下を個別に守る):

- **(M1) 統一 grant schema は optional `inputs`/`outputs` を持つ。**
  持つ grant は per-path manifest(`process_task_resources` 同等 —
  `process.rs:392-396` の per-path `reads`/`writes`/`produces`)を
  生成し、declared-path deny 検査(`prepare.rs:71-95`)・produces
  ownership・writeScope coverage 検査(`prepare.rs:96-122`)を
  駆動する。持たない grant は従来の conservative root manifest
  (`command.rs:488-498` の monolithic reads+writes、produces
  無し)を使う。`outputs` が空の grant は `ClaimMode::Shared`
  (`process/mod.rs:108-112`)、非空なら `Exclusive`(grant 側の
  常時 `Exclusive` は `command/mod.rs:135-143`)。merge は
  descope しない。
- **(M2) fixed-argv は `fixed: true` marker(仮名)で表現する。**
  fixed grant は task 由来の宣言 argv のみを実行し、model が
  供給する tool input は `{name}` のみ(`args` 不許可)。「task
  が固定 argv」「grant が argv pattern + model 供給 args」という
  2 種の契約を同一 schema・同一 exec surface で表現する。
- **(M3) project 層の exec 宣言経路 — `process` 節を
  project-authorable に維持する。** 現状
  `policy_project_partial_schema`(`schema.rs:340-355`)は
  `process` 節を許し `command` 節を除外する — この形を維持し、
  loader が project 宣言 `process.tasks` を provenance `project`
  の project-scoped grant namespace へ写像する(deprecate しない)。
  route の grant 参照(`route.grant`・grant-set `grants[]`・
  `substitute.grant`)は trusted-scoped grant のみを解決し、
  project-scoped grant は route の firing/authorization を満たせ
  ない(A3 trusted-layer invariant を弱めない — 両経路を保全)。
- **(M4) writeScope × exec — coverage check 経路を維持する。**
  writeScope 下の worker が exec できるのは **`fixed: true` かつ
  宣言 `outputs ⊆ writeScope` の grant のみ** — coverage check を
  呼び出し grant 単位で適用する。argv を供給する通常 grant は
  writeScope 下で引き続き拒否(C5「resource 層が観測不能な副作用
  を宣言させない」invariant の per-call 形。runtime 構築時の
  fail-closed 検査 `registry.rs:1133-1142` は維持。migrated
  task は現行 process semantics を保持)。
- **(M5) `tool:"process"` workflow 互換 — alias として維持する。**
  `{"kind":"tool","tool":"process","input":{"task":…}}` を持つ
  既存 workflow(`benchmarks/scenarios/scenarios.json`・golden
  fixture)が `UnknownTool` fail-closed(E7)に落ちてはならない。
  `process` は canonical tool vocabulary に残し、`input.task` を
  merged grant 集合の fixed grant 名へ解決する compat surface
  として統一 exec 機構を通す。

統合対象: 統一 grant schema(M1/M2)・`process.tasks` → grant の
loader 写像(provenance scope 付き — M3)・tool surface を
`command` に統一 + `tool:"process"` alias(M5)・writeScope 下の
per-call coverage check 経路(M4)・pin/spawn/manifest/pool 機構の
単一化。route の `project_grant` 投影・viability・record も統一
grant モデルのみを見る形になる(#<ISSUE-4> と整合)。

## Acceptance Criteria

- [ ] 統一 grant schema が `fixed`・optional `inputs`/`outputs`・
      provenance scope を保持する(M1/M2)
- [ ] project 宣言 `process.tasks` が `fixed: true` +
      provenance `project` の grant へ写像され、project-scoped
      grant が route grant 参照(`route.grant`・`grants[]`・
      `substitute.grant`)を満たせない(M3)
- [ ] writeScope 下の worker が `outputs ⊆ writeScope` の fixed
      grant を exec でき、outputs が writeScope を超える task・
      argv 供給 grant が拒否される統合テスト(M4)
- [ ] `{"kind":"tool","tool":"process","input":{"task":…}}` を持つ
      既存 workflow が `UnknownTool` で落ちずに動く後方互換テスト
      (M5)
- [ ] 既存契約テスト(pin・env_clear・stdin null・lifecycle・fs
      boundary・writeScope)が回帰なく pass

## Dependencies

- depends on #<ISSUE-1>(内容審査除去後に統合の差分コストが最小に
  なるタイミング)

## 参照

- 親 issue: https://github.com/scarecrow173/cheaper/issues/151
- design: `docs/designs/permission-model-responsibility-redesign.md`
  (DESIGN-034KuDkWOmNXiBddEQ4XWK)
- spec: `docs/specs/permission-model-responsibility-redesign.md`
  (SPEC-034KuqPkTK2b4mHsykH9A5)
- ADR: `docs/adr/enforce-declared-contracts-over-content-audit.md`
  (ADR-034KuqZelggTIb8Z8K9QB8)
- discovery:
  `docs/discovery/permission-model-responsibility-redesign.md`
  (DISC-034KuleA9g82Ir4xWfmTYq)
````

### 起票結果

- URL: https://github.com/scarecrow173/cheaper/issues/165
- 番号: #165

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/file-permission-model-redesign-implementation-issues.md`
- `docs/designs/permission-model-responsibility-redesign.md`
- `docs/specs/permission-model-responsibility-redesign.md`
- `docs/adr/enforce-declared-contracts-over-content-audit.md`

## Dependencies

- `docs/tasks/file-issue-exec-contract-content-audit-removal.md`
  (T1 — issue 1 の番号を deps 記述へ引用するため)

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×1
- `docs/tasks/file-issue-unified-exec-grant-model.md`
  (起票結果欄の記録)

## Done When

- [x] issue が `scarecrow173/cheaper` に起票済み(または重複確認で
      既存を検出し skip)
- [x] issue body が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` の 4 節を持ち、deps が
      `#<ISSUE-1>` placeholder ではなく実番号で記述されている
- [x] body に merge fidelity 決定 M1-M5 が全て含まれる
      (TSPEC-034Kvx6yD4sfTj8KFkFIF3 の issue 3 Rule)
- [x] 起票 URL・番号が本 task の起票結果欄に記録されている

## Verification

- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 3/7" --state all --json number,title,url`
      がちょうど 1 件を返す
- [ ] `gh issue list --repo scarecrow173/cheaper --search "権限モデル再設計 3/7" --state all --json body --jq ".[0].body"`
      の出力が `## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照` を全て含み、`<ISSUE` を含まない
