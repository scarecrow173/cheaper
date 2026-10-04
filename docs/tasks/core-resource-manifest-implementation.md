---
id: "TASK-034zQHzU6hKHiX9SIpdYgp"
type: "task"
status: "done"
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

- [x] 対応 IR を source 変更前に開いた。
- [x] resources/normalization.ts、predicates.ts、index.ts を作成した。
- [x] core barrel の既存 resources export 行を置換し、固定型 export を維持した。
- [x] focused GREEN（77 pass / 2 POSIX skip）と core build（exit 0）を確認した。

## Done When

types.ts / paths.ts / 依存 / hot file を変更せず named tests が pass。

## Verification

- [x] mise exec -- pnpm --filter @cheaper/core exec vitest run test/resources.test.ts が exit 0。
- [x] mise exec -- pnpm --filter @cheaper/core build が exit 0。
