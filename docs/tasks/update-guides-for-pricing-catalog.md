---
id: "TASK-034rzWTSt6UWHtrOMOegau"
type: "task"
status: "todo"
title: "Update guides for pricing catalog"
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
    - "docs/tasks/add-cli-model-flag-and-models-command.md"
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
---

# Update guides for pricing catalog

## Work

`docs/guides/configuration.md` に `models.yaml` の `exclude` と
`overrides` の優先順位、`docs/guides/providers.md` に model
catalog/pricing source の説明、`docs/guides/usage.md` に
`--model` と `cheaper models` の使い方を追記する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-model-pricing-catalog.md`
- `docs/specs/model-pricing-catalog.md`

## Dependencies

- `docs/tasks/add-cli-model-flag-and-models-command.md`

## Affected Files

- `docs/guides/configuration.md`
- `docs/guides/providers.md`
- `docs/guides/usage.md`

## Done When

- [ ] exclude/override/--model/models コマンドが guide から辿れる

## Verification

- [ ] `mise run check` pass（doc audit 含む）
