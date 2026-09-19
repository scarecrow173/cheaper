---
id: "IMPL-034rArskSHUOoayxrLW4s8"
type: "impl"
status: "completed"
title: "Add usage telemetry e2e coverage docs and bundle regen"
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
    - "../../tasks/add-e2e-coverage-docs-bundle-regen-and-final-verification.md"
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

# Add usage telemetry e2e coverage docs and bundle regen

## Summary

bulk-reader の savings を検証する e2e カバレッジを追加し、
usage guide / AGENTS.md を更新、APM bundled CLI を再生成した。

## Context

TASK-034rzRv1K2fmi2Igbdu5HG。PLAN-034rzPZHq5JFQ3CVZCVLJo の
最終タスク（e2e・docs・bundle・canonical verification）。

## Implementation

### Changes

- `packages/cli/test/e2e/fixtures/bulk-read-savings-fake.js`
  （新規）: 実際に `read` tool call を行う fake stream fixture。
- `packages/cli/test/e2e/exec.test.ts`: bundled CLI 経由で
  bulk-reader run の `meta.savings` と `cheaper usage` 集計を
  検証する e2e を追加。
- `docs/guides/usage.md`（新規）: `cheaper usage` の使い方と
  actual / measured / estimated の定義・限界を記述。
- `AGENTS.md`: 「集計コマンドは未実装」の記述を `cheaper usage`
  への案内へ更新。
- `node scripts/bundle-cli.mjs` で
  `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` を再生成。

### Decisions During Implementation

- e2e は bundled CLI（`dist/` → bundle 成果物）経路で検証し、
  APM consumer が受け取る挙動と一致させた。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise run check` 全 pass（lint・version・doc-id・typecheck・
  test 186 件・bundle 再生成・bundle freshness diff check）。
- 実 Ollama e2e（bulk-read / code-write / doc-write）pass。

## Risks

なし。

## Follow-ups

なし。
