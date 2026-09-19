---
id: "IMPL-034rzZLq1nbnzN4CvDSAk5"
type: "impl"
status: "completed"
title: "Add model pricing metadata layer and bundled data"
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
    - "docs/tasks/add-model-pricing-metadata-layer.md"
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

# Add model pricing metadata layer and bundled data

## Summary

normalized pricing metadata contract（`ModelPricing`/`resolvePricing`/`PricingSnapshot`）と bundled 補完 snapshot、curated exceptions、model ref pattern 共有 helper を `packages/core` に追加した。

## Context

issue #45 / `docs/tasks/`・`docs/plans/implement-model-pricing-catalog.md`・`docs/designs/model-pricing-catalog-and-curated-exceptions.md` に従う。

## Implementation

### Changes

- `src/model/pricing.ts`: `ModelPricing`（input/output/cacheRead/cacheWrite・unit=usd-per-1m-tokens・currency=USD・source・snapshotVersion/updatedAt）と `resolvePricing`（runtime catalog 優先 → bundled snapshot 補完 → unpriced=undefined）
- `src/model/data/pricing-snapshot.ts`: bundled `PRICING_SNAPSHOT`（versioned、初版は models 空）
- `src/model/exceptions.ts`: `ModelException`（tier/reason/source/checkedAt）・`MODEL_EXCEPTIONS`（`ollama/*`→low）・`findModelException`（exact 優先・wildcard は辞書順決定的）
- `src/model/ref.ts`: `modelKey`/`matchModelPattern`/`parseModelRef`（provider/id pattern の共有解釈）
- `ModelInfo.cost` に cacheRead/cacheWrite を拡張、index export 追加

### Decisions During Implementation

- zero-price（cost=0）は「無料」という有効な観測として runtime-catalog source で返す（low 自動分類はしないのは classify 側の責務）
- bundled snapshot は初版空。Pi catalog（models.dev 由来）が canonical で欠損のみ補完する ADR の通り exhaustive data を持たない

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

`pnpm -F @cheaper/core test`（pricing.test.ts 9 件含む全 pass）

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
