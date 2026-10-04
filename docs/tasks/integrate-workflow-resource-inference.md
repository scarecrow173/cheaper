---
id: "TASK-034zTTjWYMNKiiVb4ZZ6LV"
type: "task"
status: "done"
title: "Workflow compiler に resource inference を接続する"
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
    - "../impl/ir/workflow-resource-inference-compilation.md"
  depends-on:
    - "../plans/implement-workflow-resource-inference.md"
    - "implement-workflow-resource-inference.md"
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
    - "../test-specs/workflow-resource-inference.md"
  references: []
  defers: []
  deferred-by: []
---

# Workflow compiler に resource inference を接続する

## Work
注入時の全 job manifest と optional annotations の clone 保存を TDD で接続。全 resources 省略時も manifest が付く observable change を pin。

## Upstream
approved plan slice 2、#64 CompiledJob.manifest、共有 ADR D4/D5。

## Affected Files
workflow/src/compiler.ts、workflow/test/compiler.test.ts、chain 文書/generated indexes、slice IR。

## Dependencies
canonical inference task done。未注入 + 非空 explicit resources の INVALID_REQUEST は維持。

## Done When
dedupe/cycle/clone、focused suites/required checks、scoped exit audit と durable report。

## Verification
- [x] `mise exec -- pnpm --filter @cheaper/workflow test` exits 0。
- [x] `mise exec -- pnpm --filter @cheaper/tool-runtime test` exits 0。
- [x] `mise exec -- pnpm lint` exits 0。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。

## Scope
canonical check deferred to coordinator merge run。CLI/bundled payload は変更しない。
