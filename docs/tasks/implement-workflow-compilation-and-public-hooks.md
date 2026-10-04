---
id: "TASK-034zQvM02qLs0os3ej72pP"
type: "task"
status: "in-progress"
title: "Implement workflow compilation and public hooks"
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
    - "docs/tasks/implement-workflow-binding-and-definition-contracts.md"
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

# Workflow compilation と public hooks を実装する

## Work

1. compiler/interface tests に AC1/AC3/AC4/AC5 と resource/snapshot tests を作成。
2. targeted tests を実行し red を確認。
3. compiler/interface/index を実装。
4. targeted tests/build を実行し green を確認。
5. IR に実在 schema 互換 fixture/guidance を記録。
6. canonical check、index maintenance、exit-audit を実行。

## Scope / Dependencies

先行 binding/definition task に依存。src/compiler.ts,interface.ts,index.ts と test/compiler.test.ts,interface.test.ts。hot files、engine、CLI は編集しない。

## Done When

全 AC と plan の validation/snapshot/manifest 契約を満たし、chain terminal と exit-audit の証跡がある。

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/workflow test` exits 0。
- [ ] `mise exec -- pnpm --filter @cheaper/workflow build` exits 0。
- [ ] `mise run check` exits 0（既知 timeout の場合のみ対象単独 pass 証跡）。

