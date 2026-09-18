---
id: "TASK-034r05iHK8bA7gYVLOiMzO"
type: "task"
status: "done"
title: "Implement model candidate resolution and fallback"
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
    - "docs/tasks/implement-models-yaml-config-schema-and-merge.md"
    - "docs/tasks/implement-model-tier-classification.md"
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

# Implement model candidate resolution and fallback

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 20。`requested > maxTier` → `POLICY_VIOLATION`。候補 = available ∩ tier、順序 = preferences パターン順 → `(cost.input + cost.output)` 昇順 → provider → id。空なら `tierFallback: higher` で 1 段ずつ上へ（maxTier まで）、無ければ `MODEL_TIER_UNAVAILABLE`（details: 各 tier 候補数）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/model-resolver.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/implement-models-yaml-config-schema-and-merge.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-model-tier-classification.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/model/resolve.ts`
- `packages/core/test/model/resolve.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 優先順位・fallback・unavailable details のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/core test -- test/model/resolve.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- stderr 通知と available 取得は cli/runtime-pi 側。
