---
id: "TASK-034zQHzpSRfqiTLbSZYz5S"
type: "task"
status: "done"
title: "Core resource manifest tests"
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


# Core resource manifest tests

## Goal

plan Coverage (a)〜(h) と AC1–5 の契約を failing tests で固定する。

## Work Items

- [x] 対応 IR を in-progress で作成した。
- [x] packages/core/test/resources.test.ts に identity / boundary / fallback / glob / pure predicate 表を記録した。
- [x] focused RED を実行し、未実装 module の失敗を記録した。

## Done When

RED 証拠が IR にあり、全 named case に対応する test が存在する。

## Verification

- [x] mise exec -- pnpm --filter @cheaper/core exec vitest run test/resources.test.ts が未実装 resources/index.js を理由に exit 1（RED）。

## Notes

platform 条件は host と skip を区別。scope 外 boundary は変更しない。
