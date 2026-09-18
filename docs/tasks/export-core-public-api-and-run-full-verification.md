---
id: "TASK-034r05iHK8bA7h0AqAB1Yr"
type: "task"
status: "done"
title: "Export core public API and run full verification"
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
    - "docs/tasks/implement-worker-resolver-with-origin-precedence.md"
    - "docs/tasks/implement-capability-intersection-checks.md"
    - "docs/tasks/implement-workspace-path-resolution-with-symlink-policy.md"
    - "docs/tasks/implement-model-candidate-resolution-and-fallback.md"
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

# Export core public API and run full verification

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 21。`index.ts` で公開 API を export し、`pnpm -r build && pnpm -r test && pnpm biome check .` が全 pass。`@cheaper/core` の package.json が `@earendil-works` を含まないことを `node -e` で assert するテストも含める。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/implement-worker-resolver-with-origin-precedence.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-capability-intersection-checks.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-workspace-path-resolution-with-symlink-policy.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-model-candidate-resolution-and-fallback.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/index.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 全ビルド・テスト・lint green、Pi 非依存 assert が green

## Verification

- [x] `mise exec -- pnpm -r build && mise exec -- pnpm -r test` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0

## Out of Scope

- runtime-pi/cli パッケージは PLAN-034r05iHy2gdnpIrX0mTIb/0003。
