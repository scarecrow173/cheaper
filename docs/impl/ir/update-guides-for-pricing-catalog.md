---
id: "IMPL-034rAty3hDG6bnv9waKpx5"
type: "impl"
status: "completed"
title: "Update guides for pricing catalog"
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
    - "docs/tasks/update-guides-for-pricing-catalog.md"
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

# Update guides for pricing catalog

## Summary

configuration/providers/usage ガイドを pricing catalog・curated exceptions・exclude・`--model`・`cheaper models` の新動作に更新した。

## Context

issue #45 / `docs/tasks/`・`docs/plans/implement-model-pricing-catalog.md`・`docs/designs/model-pricing-catalog-and-curated-exceptions.md` に従う。

## Implementation

### Changes

- `configuration.md`: models.yaml に `exclude` を追記、分類 priority（overrides→exceptions→auto→unclassified）と pricing source の説明に置き換え
- `usage.md`: commands/options 一覧、`--model` の semantics、`models` コマンド節、meta.model の `selection`/`tierSource`
- `providers.md`: 旧 regex 分類表の記述を pricing heuristic + curated exception に更新、`cheaper models` への案内を追加

### Decisions During Implementation

- ドキュメントのみの変更

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

build/test に影響なし（docs のみ）

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
