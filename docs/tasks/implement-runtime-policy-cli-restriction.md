---
id: "TASK-034r05iHK8bA7gRZ6ARuGy"
type: "task"
status: "done"
title: "Implement runtime policy CLI restriction"
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
    - "docs/tasks/implement-runtime-policy-loading-order.md"
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

# Implement runtime policy CLI restriction

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 14。CLI restriction は V1 で `--timeout` のみ。`--timeout > policy.timeout` は `POLICY_VIOLATION`。他の policy field の CLI 上書きは拒否。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-runtime-policy-loading-order.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/policy/restrict.ts`
- `packages/core/test/policy/restrict.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] --timeout 許可/超過拒否のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/policy/restrict.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- CLI 引数解析そのものは PLAN-034r05iHy2gdnpJFiMWhkY。
