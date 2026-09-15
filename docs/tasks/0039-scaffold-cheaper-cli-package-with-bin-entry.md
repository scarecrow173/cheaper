---
id: "TASK-0039"
type: "task"
status: "todo"
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
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
    - "docs/tasks/0038-export-runtime-pi-public-api-and-run-full-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Scaffold cheaper CLI package with bin entry

## Work

PLAN-0003 step 1。`cheaper` パッケージ scaffold: package.json（name `cheaper`, bin `./bin/cheaper.js`, deps: @cheaper/core, @cheaper/runtime-pi workspace, yaml）、tsconfig、`bin/cheaper.js`（`#!/usr/bin/env node` → `../dist/main.js` import）、`--version` が動く最小 main。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0005-cli-contract-envelope-exit-codes.md`
- `docs/designs/0008-repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/0038-export-runtime-pi-public-api-and-run-full-verification.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/package.json`
- `packages/cli/tsconfig.json`
- `packages/cli/bin/cheaper.js`
- `packages/cli/src/main.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] `node bin/cheaper.js --version` が exit 0

## Verification

- [ ] `mise exec -- pnpm -r build` exits 0
- [ ] `node packages/cli/bin/cheaper.js --version` exits 0

## Out of Scope

- 引数解析の全体は TASK-0040、実行配線は TASK-0046。
