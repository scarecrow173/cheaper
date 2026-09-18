---
id: "TASK-034r05iHK8bA7gTkcKo0xf"
type: "task"
status: "done"
title: "Implement capability intersection checks"
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

# Implement capability intersection checks

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 15。capability 交差: requested tools ⊄ policy.tools.allow → `POLICY_VIOLATION`（欠けた tool 名を details に）、effective tools = requested 全部、tier > maxTier → `POLICY_VIOLATION`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-runtime-policy-loading-order.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/policy/capability.ts`
- `packages/core/test/policy/capability.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] tools 欠落 details・tier 超過の各テストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/policy/capability.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- model 選択は TASK-034r05iHK8bA7gYVLOiMzO、tool 実行は runtime-pi。
