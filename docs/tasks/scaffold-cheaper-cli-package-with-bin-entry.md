---
id: "TASK-034r05iHK8bA7hmVmOU1eX"
type: "task"
status: "done"
title: "Scaffold cheaper CLI package with bin entry"
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
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
    - "docs/tasks/export-runtime-pi-public-api-and-run-full-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Scaffold cheaper CLI package with bin entry

## Work

PLAN-034r05iHy2gdnpJFiMWhkY step 1。`cheaper` パッケージ scaffold: package.json（name `cheaper`, bin `./bin/cheaper.js`, deps: @cheaper/core, @cheaper/runtime-pi workspace, yaml）、tsconfig、`bin/cheaper.js`（`#!/usr/bin/env node` → `../dist/main.js` import）、`--version` が動く最小 main。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/cli-contract-envelope-exit-codes.md`
- `docs/designs/repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/export-runtime-pi-public-api-and-run-full-verification.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/package.json`
- `packages/cli/tsconfig.json`
- `packages/cli/bin/cheaper.js`
- `packages/cli/src/main.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] `node bin/cheaper.js --version` が exit 0

## Verification

- [x] `mise exec -- pnpm -r build` exits 0
- [x] `node packages/cli/bin/cheaper.js --version` exits 0

## Out of Scope

- 引数解析の全体は TASK-034r05iHK8bA7hoW2XFV4o、実行配線は TASK-034r05iHQeQpDpro7GvlKQ。
