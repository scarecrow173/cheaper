---
id: "TASK-034r05iHK8bA7gPbLk4c5Q"
type: "task"
status: "done"
title: "Implement runtime policy merge"
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
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/implement-runtime-policy-schemas.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement runtime policy merge

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 12。object deep merge / scalar replace / array 全体 replace を実装。models.yaml（overrides deep merge・preferences.<tier> array replace）と同じ規則を共有できる形にする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-runtime-policy-schemas.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/policy/merge.ts`
- `packages/core/test/policy/merge.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] deep merge・scalar replace・array replace の各テストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/policy/merge.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 適用順序は TASK-034r05iHK8bA7gQBQpBmLM。
