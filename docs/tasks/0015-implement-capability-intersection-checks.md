---
id: "TASK-0015"
type: "task"
status: "todo"
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

# Implement capability intersection checks

## Work

PLAN-0001 step 15。capability 交差: requested tools ⊄ policy.tools.allow → `POLICY_VIOLATION`（欠けた tool 名を details に）、effective tools = requested 全部、tier > maxTier → `POLICY_VIOLATION`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0013-implement-runtime-policy-loading-order.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/policy/capability.ts`
- `packages/core/test/policy/capability.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] tools 欠落 details・tier 超過の各テストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/policy/capability.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- model 選択は TASK-0020、tool 実行は runtime-pi。
