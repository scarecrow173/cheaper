---
id: "TASK-0019"
type: "task"
status: "done"
title: "Implement model tier classification"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/0017-implement-model-tier-ordering.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement model tier classification

## Work

PLAN-0001 step 19。tier 分類: overrides > built-in 正規表現表（DESIGN-0003 の high/low/medium 表、model id 小文字で high→low→medium の順に評価）> cost heuristic（cost.output USD/1M: <2 low、≤20 medium、>20 high、欠落/0 は medium）> medium。Pi 型に依存しない最小 shape `{ provider, id, cost?: {input, output}, maxTokens }` を core 側で定義。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0003-model-resolver.md`

## Dependencies

- `docs/tasks/0017-implement-model-tier-ordering.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/model/classify.ts`
- `packages/core/test/model/classify.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 各分類経路・優先順位・cost 欠落のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/model/classify.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 候補選択と fallback は TASK-0020。
