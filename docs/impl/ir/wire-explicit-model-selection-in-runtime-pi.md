---
id: "IMPL-034rAiLq0ZN6b8HGh62TPA"
type: "impl"
status: "completed"
title: "Wire explicit model selection in runtime-pi"
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
    - "docs/tasks/wire-explicit-model-in-runtime.md"
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

# Wire explicit model selection in runtime-pi

## Summary

runtime-pi に explicit model 解決経路を追加し、`meta.model` に `selection`/`tierSource` を伝播させた。

## Context

issue #45 / `docs/tasks/`・`docs/plans/implement-model-pricing-catalog.md`・`docs/designs/model-pricing-catalog-and-curated-exceptions.md` に従う。

## Implementation

### Changes

- `model-runtime.ts`: `toModelInfo` が cacheRead/cacheWrite を写す、`ResolvedPiModel` に `tierSource`/`pricing`/`selection`、`resolveExplicitPiModel` 追加（共通後段 `toResolvedPiModel` に集約）
- `run.ts`: `RunWorkerDeps.explicitModel?: string`（`provider/id`、不正形式は INVALID_REQUEST）、`WorkerRunMeta.model` に `selection`/`tierSource`

### Decisions During Implementation

- `--model` は tier 候補選択のみ bypass し exclude/availability/maxTier は core resolver に委譲
- ref parse は最初の `/` のみで分割（model id が `/` を含みうるため）

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

`pnpm -r build`・`pnpm -r test` 全 pass（core 274 / runtime-pi 322 / cli 157）

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
