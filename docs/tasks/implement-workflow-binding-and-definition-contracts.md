---
id: "TASK-034zQvLk1Av5981u4ci8q4"
type: "task"
status: "done"
title: "Implement workflow binding and definition contracts"
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
    - "docs/plans/implement-workflow-contract-compiler.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-workflow-contract-compiler.md"
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
    - "docs/test-specs/workflow-compiler-contract-guarantees.md"
  references: []
  defers: []
  deferred-by: []
---

# Workflow binding と definition contracts を実装する

## Work

1. bindings.test.ts に literal/non-scan/Pointer と invalid wrappers の tests を作成。
2. targeted test を実行し red を確認。
3. types/json/bindings/schema/dag を実装。
4. bindings tests と workflow build を実行し green を確認。

## Scope / Dependencies

approved plan と test-spec に従う。src/types.ts,json.ts,bindings.ts,schema.ts,dag.ts と test/bindings.test.ts を所有。catalog は port のみ。engine と他 package は編集しない。

## Done When

strict JSON/binding/schema/DAG 契約が implementation と tests に存在し、IR に red/green 証跡がある。

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/workflow test -- test/bindings.test.ts test/no-pi-deps.test.ts` exits 0。
- [x] `mise exec -- pnpm --filter @cheaper/workflow build` exits 0。
