---
id: "TASK-034zTn9hnvGsV4G8kGiHvT"
type: "task"
status: "done"
title: "Implement bounded artifact store"
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
    - "docs/plans/implement-run-scoped-artifact-store.md"
  implemented-by: ["IMPL-034zTneiPeW23igPepp7ph"]
  depends-on:
    - "docs/plans/implement-run-scoped-artifact-store.md"
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
    - "docs/test-specs/run-scoped-artifact-boundaries.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement bounded artifact store

## Work

承認済み plan の A→B→C→D→E を単一 store contract slice として実行する。先に実 contract test を追加、RED を確認、独立 port と workflow store を実装し GREEN にする。

## Affected Files

packages/tool-runtime/src/artifact.ts と index.ts、packages/workflow/src/artifacts.ts と index.ts、packages/workflow/test/artifacts.test.ts。

## Dependencies and Scope

spec/design/plan は coordinator 承認済み。caps は engine が供給する。CLI/core/runtime-pi/既存 tool の変更、依存追加、push/PR は範囲外。

## Done When

AC1-AC7 の focused test が GREEN、型/format/build pass、IR に evidence を記録、scoped audit pass。

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/workflow test` exits 0
- [x] `mise exec -- pnpm --filter @cheaper/tool-runtime test` exits 0
- [x] `mise exec -- pnpm lint` exits 0
- [x] `mise exec -- pnpm typecheck` exits 0
- [x] `mise exec -- pnpm build` exits 0

## Notes

canonical check deferred to coordinator merge run。CLI test は CLI 無変更のため実行対象外。
