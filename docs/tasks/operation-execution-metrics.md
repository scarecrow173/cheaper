---
id: "TASK-034A9yFQqZLtHt5ibjhE1h"
type: "task"
status: "done"
title: "Operation execution metrics"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  source: []
  implements: ["../plans/operation-execution-metrics.md", "../specs/operation-execution-metrics.md"]
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../plans/operation-execution-metrics.md"]
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/operation-execution-metrics.md"]
  references: []
  defers: []
  deferred-by: []
---

# Operation execution metrics

## Goal

承認済み plan に従い deterministic execution snapshot と success/error meta を実装する。

## Scope

tool-runtime reducer/lifecycle/high-water/invocation clock、runtime-pi meta、CLI forwarding、tests/fixtures と例示 docs。

## Steps

- [x] controlled clock の metrics/owner/lifecycle tests を RED→GREEN。
- [x] Worker/CLI success/error tests を RED→GREEN。
- [x] 既存 scheduler/usage/savings 回帰、lint/typecheck/build、review と scoped exit-audit。

## Verification

- mise exec -- pnpm --filter @cheaper/tool-runtime test
- mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/run.test.ts test/tool-scheduler.test.ts test/sibling-parallelism.test.ts test/usage.test.ts
- mise exec -- pnpm --filter ./packages/cli exec vitest run test/envelope.test.ts test/main.test.ts test/text-format.test.ts test/usage-log.test.ts
- mise exec -- pnpm lint / pnpm typecheck / pnpm build

canonical check deferred to coordinator merge run。
