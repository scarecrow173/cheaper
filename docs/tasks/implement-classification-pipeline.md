---
id: "TASK-034rzWJcbIzVxc1Lhk5bxB"
type: "task"
status: "done"
title: "Implement classification pipeline and explicit model resolution"
created: "2026-09-19"
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
    - "docs/plans/implement-model-pricing-catalog.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-model-pricing-catalog.md"
    - "docs/tasks/add-model-pricing-metadata-layer.md"
    - "docs/tasks/add-models-config-exclude.md"
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
    - "docs/test-specs/model-tier-classification-pipeline.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement classification pipeline and explicit model resolution

## Work

`classify.ts` を pipeline 化して `{ tier, source: TierSource }` を
返すようにし、`BUILTIN_TABLE` regex を削除して curated exceptions
に置き換える（auto 分類は #46 の scope であり本 task では実装
しない）。`resolve.ts` で exclude 適用・`ResolvedModel` への
`tierSource`/`pricing` 追加・`resolveExplicitModel` を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-model-pricing-catalog.md`
- `docs/specs/model-pricing-catalog.md`

## Dependencies

- `docs/tasks/add-model-pricing-metadata-layer.md`
- `docs/tasks/add-models-config-exclude.md`

## Affected Files

- `packages/core/src/model/classify.ts`
- `packages/core/src/model/resolve.ts`
- `packages/core/test/model/classify.test.ts`
- `packages/core/test/model/resolve.test.ts`

## Done When

- [ ] priority が override > exception > unclassified（`auto` は
      #46 seam で #45 では生成しない）
- [ ] exclude が resolver candidate set と explicit 指定の双方に効く
- [ ] `resolveExplicitModel` が exclude/availability/maxTier を
      bypass しない
- [ ] unclassified が medium 扱いで `source: "unclassified"`

## Verification

- [ ] `pnpm -F @cheaper/core test` で classify/resolve の単体
      テストが pass
