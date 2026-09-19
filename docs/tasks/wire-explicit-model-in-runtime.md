---
id: "TASK-034rzWSt1O4qIT7hk0bnds"
type: "task"
status: "done"
title: "Wire explicit model selection and meta in runtime-pi"
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
    - "docs/tasks/implement-classification-pipeline.md"
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
    - "docs/test-specs/model-exclude-and-explicit-selection.md"
  references: []
  defers: []
  deferred-by: []
---

# Wire explicit model selection and meta in runtime-pi

## Work

`toModelInfo` が `cost.cacheRead`/`cacheWrite` を写すようにし、
`resolveExplicitPiModel` を追加して `resolvePiModel` の返却に
`tierSource`/`pricing`/`selection` を含める。`run.ts` に
`deps.explicitModel` を追加し、`WorkerRunMeta.model` に
`selection` と `tierSource` を記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-model-pricing-catalog.md`

## Dependencies

- `docs/tasks/implement-classification-pipeline.md`

## Affected Files

- `packages/runtime-pi/src/model-runtime.ts`
- `packages/runtime-pi/src/run.ts`
- `packages/runtime-pi/test/` 関連テスト

## Done When

- [ ] `deps.explicitModel` 指定時に tier resolver を bypass する
- [ ] `meta.model.selection` が `"tier"`/`"explicit"` を取る
- [ ] `meta.model.tierSource` が記録される

## Verification

- [ ] `pnpm -F @cheaper/runtime-pi test` pass
