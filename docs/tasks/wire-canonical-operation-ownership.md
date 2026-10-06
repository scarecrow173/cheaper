---
id: "TASK-034zTjXTqV7nvYYkf1bUH9"
type: "task"
status: "done"
title: "Wire canonical operation ownership and abort"
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
    - "docs/tasks/implement-operation-manager-state.md"
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

# canonical operation ownership と abort を接続する

## Goal

invocation 内の scope が同じ manager を利用し、run first-cause と canonical handler の実 settle を追跡する。

## Scope

tool-runtime registry/invocation と tests、runtime-pi/run.ts の reason 渡しと run tests のみ。

## Steps

1. scope/root/call cancellation、late settle、result/accounting と run signal reason の tests を書く。
2. focused red を確認する。
3. registry と invocation を manager に接続する。
4. runAbort に first-cause reason を渡す。
5. focused suite と lint/typecheck/build を実行する。
6. IR と chain status を更新して commit、index regeneration と scoped exit-audit を行う。

## Done When

owned active=0 の finite disposal、sibling isolation、pre-abort/budget admission closure、canonical result/journal/meter を維持する。

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/tool-runtime test` exits 0（74 tests）。
- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/run.test.ts test/tools/registry.test.ts test/tool-runtime-contract.test.ts` exits 0（69 tests）。
- [x] `mise exec -- pnpm lint` exits 0（既存 18 warnings）。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。
- [x] scoped doc-status の blocking=0（担当 chain + 共有 ADR、2026-10-04）。
