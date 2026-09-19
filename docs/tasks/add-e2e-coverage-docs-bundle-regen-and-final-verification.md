---
id: "TASK-034rzRv1K2fmi2Igbdu5HG"
type: "task"
status: "done"
title: "Add e2e coverage, docs, bundle regen and final verification"
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
    - "docs/plans/usage-savings-telemetry-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/usage-savings-telemetry-implementation-plan.md"
    - "docs/tasks/add-cheaper-usage-aggregation-command-with-filters-and-estimated-savings.md"
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

# Add e2e coverage, docs, bundle regen and final verification

## Work

e2e 検証・docs・bundle 再生成・最終検証を行う仕上げ task。

### 1. e2e

- fake-stream e2e（tool call を含む scripted fixture）で
  bulk-reader run の `meta.savings` 3 field と usage.jsonl
  record の `savings` を assert（`avoided = max(0, source -
  returned)`）。
- `cheaper usage` の e2e: tmpdir の usage.jsonl を集計し、
  filter・Estimated 非表示（pricing なし）を確認。

### 2. docs

- `docs/guides/usage.md`: `usage` command・指標の 3 区分
  （actual / measured / estimated）・`avoidedContextTokens` の
  定義と限界・pricing.json contract を記載。
- `AGENTS.md`: 計測 log 節に `savings` と `usage` command を
  追記。

### 3. bundle 再生成 + 最終検証

- `pnpm bundle`（scripts/bundle-cli.mjs）で
  `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` を再生成。
- `mise run check` 全通し。
- plan・task の status 同期（task→done、plan→completed）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/usage-savings-telemetry-implementation-plan.md`
- `docs/designs/usage-savings-telemetry.md`
- `docs/specs/usage-and-savings-telemetry-aggregation.md`

## Dependencies

- `docs/tasks/add-cheaper-usage-aggregation-command-with-filters-and-estimated-savings.md`

## Affected Files

- `packages/cli/test/e2e/*`（modify/add）
- `docs/guides/usage.md`（modify）
- `AGENTS.md`（modify）
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`（generated）

## Done When

- [x] fake-stream e2e で `meta.savings`・record `savings` を
      assert できる
- [x] `cheaper usage` e2e が pass
- [x] docs に指標の定義と限界が記載される
- [x] `mise run check` pass

## Verification

- [x] `pnpm test` 全 pass
- [x] `mise run check` pass
