---
id: "TASK-0014"
type: "task"
status: "todo"
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
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/0013-implement-runtime-policy-loading-order.md"
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

# Implement runtime policy CLI restriction

## Work

PLAN-0001 step 14。CLI restriction は V1 で `--timeout` のみ。`--timeout > policy.timeout` は `POLICY_VIOLATION`。他の policy field の CLI 上書きは拒否。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0013-implement-runtime-policy-loading-order.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/policy/restrict.ts`
- `packages/core/test/policy/restrict.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] --timeout 許可/超過拒否のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/policy/restrict.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- CLI 引数解析そのものは PLAN-0003。
