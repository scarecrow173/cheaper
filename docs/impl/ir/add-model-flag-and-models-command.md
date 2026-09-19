---
id: "IMPL-034rAtxE2k5HhaePoxBbI3"
type: "impl"
status: "completed"
title: "Add --model flag and models command"
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
    - "docs/tasks/add-cli-model-flag-and-models-command.md"
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

# Add --model flag and models command

## Summary

CLI に `--model <provider>/<id>` option と `cheaper models` コマンド（`--provider`/`--tier` filter）を追加した。

## Context

issue #45 / `docs/tasks/`・`docs/plans/implement-model-pricing-catalog.md`・`docs/designs/model-pricing-catalog-and-curated-exceptions.md` に従う。

## Implementation

### Changes

- `args.ts`: `model`/`provider`/`tier` option、`ModelsCliArgs`、`models` コマンド parse、`--model` の `provider/id` 形式検証と command 別 option 検証
- `cli.ts`: run deps へ `explicitModel` 伝播、`executeModels`（getAvailable → exclude/pricing/classification 適用 → `result.models[]` utility envelope）、HELP 更新
- `text-format.ts`: `models` の text 表示（available/excluded・tier・tierSource・pricing source/freshness）

### Decisions During Implementation

- models 一覧は worker run を伴わないため utility envelope（worker:null）
- 各 model entry は `{provider,id,excluded,tier,tierSource,pricing}`。pricing は unpriced 時 null（error にしない）

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

`pnpm -r build`・`pnpm -F @cheaper/cli exec vitest run test/args.test.ts test/main.test.ts test/text-format.test.ts` 全 pass

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
