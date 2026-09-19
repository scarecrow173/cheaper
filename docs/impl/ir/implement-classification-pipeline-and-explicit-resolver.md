---
id: "IMPL-034rAiKtIOGy4Ja8gzLN0C"
type: "impl"
status: "completed"
title: "Implement classification pipeline and explicit resolver"
created: "2026-09-19"
updated: "2026-09-20"
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
    - "docs/tasks/implement-classification-pipeline.md"
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

# Implement classification pipeline and explicit resolver

## Summary

`classifyModel` を `Classification{tier,source}` を返す pipeline に変更し、regex の exhaustive 分類表を削除。`resolveModel` に exclude 適用・`tierSource`/`pricing`/`selection` 返却を追加し、`resolveExplicitModel` を新設した。

## Context

issue #45 / `docs/tasks/`・`docs/plans/implement-model-pricing-catalog.md`・`docs/designs/model-pricing-catalog-and-curated-exceptions.md` に従う。

## Implementation

### Changes

- `src/model/classify.ts`: `TierSource`（explicit-override/curated-exception/auto/unclassified。`auto` は #46 seam で #45 では生成しない）・`BUILTIN_TABLE` 削除・priority `overrides > exceptions > unclassified(medium)`（review 指摘により absolute USD threshold の interim `autoClassify` は除去。auto 分類は #46 scope）
- `src/model/resolve.ts`: exclude を candidate set から先に除去（`isExcludedModel`）、`ResolvedModel` に `tierSource`/`pricing?`/`selection`、新規 `resolveExplicitModel`（exclude→POLICY_VIOLATION/model-excluded、catalog 不在→MODEL_TIER_UNAVAILABLE、effective tier>maxTier→POLICY_VIOLATION/model-exceeds-max-tier）

### Decisions During Implementation

- unclassified の effective tier は従来不変条件どおり medium、`source` で識別可能にした
- `classifyModel` の返り値を `Classification` に変更（呼び出し側は resolve.ts のみで影響小）
- 当初は cost.output 閾値の interim auto classifier を入れていたが、issue #45 追記（自動分類の検証は #46 に分離・採否に依存させない）と review 指摘を受けて除去。`auto` は `TierSource` union の seam としてのみ残す

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

`pnpm -F @cheaper/core test`・`pnpm -r build` 全 pass

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
