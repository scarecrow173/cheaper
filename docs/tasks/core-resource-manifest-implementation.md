---
id: "TASK-034zQHzU6hKHiX9SIpdYgp"
type: "task"
status: "todo"
title: "Core resource manifest implementation"
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
    - "docs/plans/implement-core-resource-manifest-normalization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-core-resource-manifest-normalization.md"
    - "docs/tasks/core-resource-manifest-tests.md"
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
    - "docs/test-specs/core-resource-manifest-normalization.md"
  references: []
  defers: []
  deferred-by: []
---


# Core resource manifest implementation

## Goal

承認済み design の core normalization と predicates を実装する。

## Work Items

- [ ] 対応 IR を開く。
- [ ] resources/normalization.ts、predicates.ts、index.ts を作成する。
- [ ] core/src/index.ts の既存 resources/types.js export 行を resources/index.js に置換する（既存型 export を resources/index.ts が保持）。
- [ ] focused GREEN と core build を確認する。

## Done When

types.ts / paths.ts / 依存 / hot file を変更せず named tests が pass。

## Verification

- [ ] mise exec -- pnpm --filter @cheaper/core exec vitest run test/resources.test.ts が exit 0。
- [ ] mise exec -- pnpm --filter @cheaper/core build が exit 0。

