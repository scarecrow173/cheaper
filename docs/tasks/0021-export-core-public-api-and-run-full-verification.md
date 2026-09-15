---
id: "TASK-0021"
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
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/0010-implement-worker-resolver-with-origin-precedence.md"
    - "docs/tasks/0015-implement-capability-intersection-checks.md"
    - "docs/tasks/0016-implement-workspace-path-resolution-with-symlink-policy.md"
    - "docs/tasks/0020-implement-model-candidate-resolution-and-fallback.md"
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

# Export core public API and run full verification

## Work

PLAN-0001 step 21。`index.ts` で公開 API を export し、`pnpm -r build && pnpm -r test && pnpm biome check .` が全 pass。`@cheaper/core` の package.json が `@earendil-works` を含まないことを `node -e` で assert するテストも含める。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0008-repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/0010-implement-worker-resolver-with-origin-precedence.md` — 依存先（Dependency Graph）
- `docs/tasks/0015-implement-capability-intersection-checks.md` — 依存先（Dependency Graph）
- `docs/tasks/0016-implement-workspace-path-resolution-with-symlink-policy.md` — 依存先（Dependency Graph）
- `docs/tasks/0020-implement-model-candidate-resolution-and-fallback.md` — 依存先（Dependency Graph）

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

- runtime-pi/cli パッケージは PLAN-0002/0003。
