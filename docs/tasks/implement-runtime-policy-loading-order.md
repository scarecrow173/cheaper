---
id: "TASK-034r05iHK8bA7gQBQpBmLM"
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
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/implement-runtime-policy-merge.md"
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

# Implement runtime policy loading order

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 13。§23 のロード順（built-in → user → project → CLI）で load+merge。`packages/cli/policy.yaml`（built-in policy、§20 canonical 値）を作成し load テストの built-in fixture として使う。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-runtime-policy-merge.md` — 依存先（Dependency Graph）

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

- CLI restriction は TASK-034r05iHK8bA7gRZ6ARuGy。
