---
id: "TASK-0013"
type: "task"
status: "done"
title: "Implement runtime policy loading order"
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
    - "docs/tasks/0012-implement-runtime-policy-merge.md"
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

# Implement runtime policy loading order

## Work

PLAN-0001 step 13。§23 のロード順（built-in → user → project → CLI）で load+merge。`packages/cli/policy.yaml`（built-in policy、§20 canonical 値）を作成し load テストの built-in fixture として使う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0012-implement-runtime-policy-merge.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/policy/load.ts`
- `packages/core/test/policy/load.test.ts`
- `packages/cli/policy.yaml`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] ロード順・merge 結果・built-in policy.yaml が schema を通るテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/policy/load.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- CLI restriction は TASK-0014。
