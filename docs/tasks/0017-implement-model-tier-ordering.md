---
id: "TASK-0017"
type: "task"
status: "done"
title: "Implement model tier ordering"
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
    - "docs/tasks/0002-create-empty-cheaper-core-package-skeleton.md"
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

# Implement model tier ordering

## Work

PLAN-0001 step 17。`Tier = low|medium|high` の順序 `low < medium < high` と比較ヘルパー。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0003-model-resolver.md`

## Dependencies

- `docs/tasks/0002-create-empty-cheaper-core-package-skeleton.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/model/tier.ts`
- `packages/core/test/model/tier.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 比較・大小関係のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/model/tier.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 分類・選択は TASK-0019/0020。
