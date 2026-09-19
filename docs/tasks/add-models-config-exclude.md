---
id: "TASK-034rzWIEgBvwU5S4KYY68N"
type: "task"
status: "todo"
title: "Add models.yaml exclude with union merge"
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
    - "docs/test-specs/model-exclude-and-explicit-selection.md"
  references: []
  defers: []
  deferred-by: []
---

# Add models.yaml exclude with union merge

## Work

`MODELS_CONFIG_SCHEMA` に `exclude: string[]` を追加し、
`ModelConfig.exclude` を実装する。user/project 層の `exclude` は
union（concat + dedupe）で合成し、`mergePolicyLayers` の array
replace には乗せない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-model-pricing-catalog.md`

## Dependencies

- なし

## Affected Files

- `packages/core/src/model/config.ts`
- `packages/core/test/model/config.test.ts`

## Done When

- [ ] `models.yaml` の `exclude` が schema 検証を通る
- [ ] user/project の exclude が union される
- [ ] `exclude` 未指定時は空配列で後方互換

## Verification

- [ ] `pnpm -F @cheaper/core test` で config の単体テストが pass
