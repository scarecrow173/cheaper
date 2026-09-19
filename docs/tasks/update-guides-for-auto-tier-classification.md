---
id: "TASK-nG7Lngq2AuUQEzF33IkneZ"
type: "task"
status: "done"
title: "Update user guides for auto tier classification"
created: "2026-09-20"
updated: "2026-09-20"
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
    - "docs/plans/implement-auto-tier-classification.md"
  implemented-by: []
  depends-on:
    - "TASK-vhu8Z2b2Ylehk8LEqt43Ke"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Update user guides for auto tier classification

## Work

- `docs/guides/workers.md`: `cheaper.model.workload.outputPerInput`
  schema・built-in 初期値・未指定時の default を記載
- `docs/guides/configuration.md`: `cheaper models --worker` と
  workload の説明を追記（該当セクションがある場合）
- `docs/guides/usage.md`: `meta.model` の explainability field
  （effectiveCost / effectiveCostPercentile / workload）を追記

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-auto-tier-classification.md`
- `docs/designs/auto-tier-classification.md`

## Dependencies

- `TASK-vhu8Z2b2Ylehk8LEqt43Ke`（document 対象の実装が前提）
