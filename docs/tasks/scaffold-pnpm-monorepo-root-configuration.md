---
id: "TASK-034r05iHK8bA7gC9ZdvYF9"
type: "task"
status: "done"
title: "Scaffold pnpm monorepo root configuration"
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

# Scaffold pnpm monorepo root configuration

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 1。ルート package.json（private, `packageManager: pnpm@11`, `pnpm.onlyBuiltDependencies: []`, scripts: build=`pnpm -r build` / test=`pnpm -r test` / lint=`biome check .` / typecheck=`tsc -b`）、pnpm-workspace.yaml（packages/*）、tsconfig.base.json（NodeNext/ES2023/strict/exactOptionalPropertyTypes/noUncheckedIndexedAccess）、tsconfig.json（references）、biome.json、vitest.workspace.ts を作成し、.gitignore に dist/・node_modules/ を追加。root devDeps は DESIGN-034r05iHrZUE6Wzyjd1NS8 どおり typescript, vitest, @types/node, @biomejs/biome を `pnpm add -D -w`（minimum_release_age 7d 遵守）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/repository-structure-toolchain.md`

## Dependencies

- なし（plan の先頭タスク。PLAN 自体への依存は front matter の depends-on に記載）

## Affected Files

- `package.json`
- `pnpm-workspace.yaml`
- `tsconfig.base.json`
- `tsconfig.json`
- `biome.json`
- `vitest.workspace.ts`
- `.gitignore`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] `pnpm install` が ignored build scripts の exit 1 を出さず exit 0（onlyBuiltDependencies 明示）

## Verification

- [ ] `mise exec -- pnpm install` exits 0
- [ ] `mise exec -- pnpm -r build` exits 0

## Out of Scope

- packages/core の中身は TASK-034r05iHK8bA7gD3wnFQ3l 以降。
