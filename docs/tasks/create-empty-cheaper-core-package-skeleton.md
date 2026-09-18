---
id: "TASK-034r05iHK8bA7gD3wnFQ3l"
type: "task"
status: "done"
title: "Create empty @cheaper/core package skeleton"
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
    - "docs/tasks/scaffold-pnpm-monorepo-root-configuration.md"
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

# Create empty @cheaper/core package skeleton

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 2。`@cheaper/core` の空パッケージ: package.json（type module, exports `./dist/index.js`, deps: ajv, yaml; devDeps は root 共有）、tsconfig.json（base 継承）、空の index.ts。空 vitest が pass する状態まで。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/scaffold-pnpm-monorepo-root-configuration.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/package.json`
- `packages/core/tsconfig.json`
- `packages/core/src/index.ts`
- `packages/core/test/`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] `tsc -b` と空 vitest が pass

## Verification

- [ ] `mise exec -- pnpm -r build` exits 0
- [ ] `mise exec -- pnpm --filter @cheaper/core test` exits 0

## Out of Scope

- 各 src モジュールの実装は TASK-034r05iHK8bA7gEdDgIUDn 以降。
