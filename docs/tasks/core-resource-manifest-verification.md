---
id: "TASK-034zQHAqgNJHPLssPVaFPR"
type: "task"
status: "in-progress"
title: "Core resource manifest verification"
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
    - "docs/tasks/core-resource-manifest-implementation.md"
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


# Core resource manifest verification

## Goal

canonical check と focused exit-audit により配布可能な終端状態を記録する。

## Work Items

- [ ] 対応 IR を開く。
- [ ] mise run check を実行し末尾を report に保存する。
- [ ] 自 chain の terminal status と reciprocal verified-by / verifies relation を更新する。
- [ ] doc-maintenance plan → apply で generated index を再生成する。
- [ ] followup-triage、focused exit-audit、scope 差分を記録する。

## Done When

plan completed / tasks done / impl completed と focused graph complete。残存 boundary limitation は coordinator 引継ぎ済み。

## Verification

- [ ] mise run check が exit 0（既知 cli timeout は該当 file 単独 pass 証拠を併記）。
- [ ] mise exec -- node scripts/check-doc-ids.mjs が exit 0。
- [ ] git diff --check が exit 0。
- [ ] focused graph exit-audit の各 gate が pass。

