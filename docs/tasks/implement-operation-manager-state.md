---
id: "TASK-034zTjT0KVd4s2TRC4Ux1B"
type: "task"
status: "done"
title: "Implement operation manager state lifecycle"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/implement-operation-manager-lifecycle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-operation-manager-lifecycle.md"
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
    - "docs/test-specs/operation-lifecycle-settlement.md"
  references: []
  defers: []
  deferred-by: []
---

# OperationManager の state lifecycle を実装する

## Goal

manager 単体で六状態、first-cause、実 settle、owner cancellation と bounded drain を保証する。

## Scope

operation.ts、barrel export、operation.test.ts。scheduler/lease は作らない。

## Steps

1. transition matrix と actual settle/finite drain tests を書く。
2. focused red の未実装 failure を確認する。
3. manager を実装する。
4. focused green と型/build を実行する。
5. IR に evidence を記録して logical commit する。

## Done When

state matrix、terminal timestamp、timeout mapping、queued/running cancel、unsettled/late settle、owner admission closure が検証される。

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/tool-runtime exec vitest run test/operation.test.ts` exits 0（50 tests）。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。
