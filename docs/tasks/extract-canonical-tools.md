---
id: "TASK-034zQsmDSYTqJOZ8CjKrGN"
type: "task"
status: "done"
title: "Canonical handlers と Pi adapter を抽出する"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
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
    - "docs/plans/implement-canonical-tool-runtime.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-canonical-tool-runtime.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/canonical-tool-runtime.md"
  references: []
  defers: []
  deferred-by: []
---

# Canonical handlers と Pi adapter を抽出する

## Scope

T1–T4: Pi-free handlers/catalog/schema、journal 正本、accounting 一元化、helper 注入と compatibility exports。指定所有ファイルのみ変更する。

## Steps

1. test を作成する。
2. targeted test の red を確認する。
3. approved plan に従い実装する。
4. targeted test と型検証を実行する。
5. evidence を ir に記録して論理変更を commit する。

## Done When

同一 fixture parity、malformed、accounting、journal compatibility と D7 ownership の該当保証が pass する。

## Verification

- [x] `mise exec -- pnpm exec vitest run packages/tool-runtime/test packages/runtime-pi/test/tools packages/runtime-pi/test/tool-runtime-contract.test.ts` exits 0
- [x] `mise exec -- pnpm typecheck` exits 0

