---
id: "IMPL-034rAiJgmYQKgmJC2mF9zN"
type: "impl"
status: "completed"
title: "Add models.yaml exclude with union merge"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/tasks/add-models-config-exclude.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add models.yaml exclude with union merge

## Summary

models.yaml schema version 1 に `exclude` を追加し、user/project 層で union 合成するようにした。

## Context

issue #45 / `docs/tasks/`・`docs/plans/implement-model-pricing-catalog.md`・`docs/designs/model-pricing-catalog-and-curated-exceptions.md` に従う。

## Implementation

### Changes

- `src/model/config.ts`: `ModelConfig.exclude?: string[]`、schema に `exclude: string[]`、merge は `mergePolicyLayers` の array-replace に乗せず層ごとに収集して concat+dedupe（Set）

### Decisions During Implementation

- exclude は deny list なので project 層が user の deny を置き換えないよう union を選択（design 通り）
- 未指定時は `undefined` のまま（後方互換）

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

`pnpm -F @cheaper/core test`（config.test.ts の union テスト含む全 pass）

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
