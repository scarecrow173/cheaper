---
id: "TASK-034rzWI7U1txBCpGr7WZaJ"
type: "task"
status: "todo"
title: "Add model pricing metadata layer and bundled data"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/implement-model-pricing-catalog.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-model-pricing-catalog.md"
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
    - "docs/test-specs/pricing-metadata-and-models-command.md"
  references: []
  defers: []
  deferred-by: []
---

# Add model pricing metadata layer and bundled data

## Work

`packages/core/src/model/pricing.ts` に `ModelPricing`・
`PricingSnapshot`・`resolvePricing` を実装し、bundled data module
`data/pricing-snapshot.ts` と `data/model-exceptions.ts` を追加する。
`ModelInfo.cost` に `cacheRead`/`cacheWrite` optional field を
追加する。`index.ts` から export する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-model-pricing-catalog.md`
- `docs/designs/model-pricing-catalog-and-curated-exceptions.md`

## Dependencies

- なし（core model 層の起点 task）

## Affected Files

- `packages/core/src/model/pricing.ts`（新規）
- `packages/core/src/model/data/pricing-snapshot.ts`（新規）
- `packages/core/src/model/data/model-exceptions.ts`（新規）
- `packages/core/src/model/classify.ts`（`ModelInfo.cost` 拡張のみ）
- `packages/core/src/index.ts`

## Done When

- [ ] `resolvePricing` が runtime cost → snapshot → unpriced の順で
      `ModelPricing` を返す
- [ ] exceptions が `provider/model` exact と `provider/*` wildcard
      を解釈し、entry が reason/source/checkedAt を持つ
- [ ] bundled data が esbuild bundle に埋め込める TS module である
      （runtime file IO なし）

## Verification

- [ ] `pnpm -F @cheaper/core test` で pricing/exceptions の単体
      テストが pass
