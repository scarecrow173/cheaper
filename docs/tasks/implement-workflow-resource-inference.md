---
id: "TASK-034zTTj0uuzqIXG4cbMoIO"
type: "task"
status: "done"
title: "Canonical workflow resource inference を実装する"
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
    - "../plans/implement-workflow-resource-inference.md"
  implemented-by:
    - "../impl/ir/workflow-resource-inference-implementation.md"
  depends-on:
    - "../plans/implement-workflow-resource-inference.md"
  blocks:
    - "integrate-workflow-resource-inference.md"

  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/workflow-resource-inference.md"
  references: []
  defers: []
  deferred-by: []
---

# Canonical workflow resource inference を実装する

## Work
optional catalog descriptor と canonical inference を TDD で実装。IR→RED→implementation→GREEN→commit。

## Upstream
approved plan slice 1、local ADR、共有 ADR D1/D4/D5。

## Affected Files
tool-runtime/src/catalog.ts、workflow/src/resource-inference.ts/types.ts/index.ts、workflow/test/resource-inference.test.ts。

## Dependencies
approved plan。後続 compiler slice は本 task 完了後。

## Done When
全 manifest、source/fallback 分離、edge/conflict/理由、duplicate error、process seam が pass。

## Verification
- [x] `mise exec -- pnpm --filter @cheaper/workflow test -- test/resource-inference.test.ts` exits 0。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。
- [x] `mise exec -- pnpm lint` exits 0。

## Scope
core/runtime-pi/CLI/hot files と process executor は変更しない。
