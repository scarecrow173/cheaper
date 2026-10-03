---
id: "IMPL-034zCh129JMqV5Z5ta5wG6"
type: "impl"
status: "completed"
title: "Add deterministic sibling-parallelism pin tests to runtime-pi"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/tasks/add-deterministic-sibling-parallelism-pin-tests-to-runtime-pi.md"
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

# Add deterministic sibling-parallelism pin tests to runtime-pi

## Summary

pinned `pi-agent-core@0.85.1` の sibling tool-call 並列 semantics を
決定的に pin する 2 つの test file を追加した。4 つの sibling test
（並列 overlap 実行・call 順 `toolResult` emit・`stopReason: "length"`
全員未実行 error・同一ファイル write→edit の `withFileMutationQueue`
直列化）は初回実行で全て green — discovery の静的調査結果を runtime
実測で裏付けた。

## Context

TASK-034zBIYG4bskj0L6ueomFK（plan Phase A、T1–T5）。issue #55 /
SPEC-034zA513QKarQrr5SQtgp7 / DESIGN-034zAKcWuB8SuFkOePsubg に従う。

## Implementation

### Changes

- `packages/runtime-pi/test/sibling-parallelism.test.ts`（新規）:
  `createFakePi` + `customTools`/`toolNames`/`streamFn` seam 経由で実
  Pi agent loop を駆動する 4 test。instrumented `ToolDefinition` は
  `defineTool` + `Type.Object({})`（schema 不適合による `kind:
  "immediate"` 誤診を防止）。gate 待機は全て約 1s の
  `Promise.race` deadline 付き + `it` timeout 10s。
- `packages/runtime-pi/test/no-execution-mode.test.ts`（新規）:
  `src/**/*.ts` を再帰走査し、comment / string literal strip 済みで
  `toolExecution` / `executionMode` identifier が存在しないことを
  assert（#57 ToolScheduler 導入余地の pin）。

### Decisions During Implementation

- result 順序 test は sleep ではなく reverse-completion gate
  （`probe_i` が `probe_{i+1}` の完了 deferred を await）で完了順を
  確定的に逆転させた。`toolResult` message role は camelCase
  `"toolResult"`（`"tool_result"` は extension event 名）。
- 補助 assert（全 `tool_execution_start` が最初の
  `tool_execution_end` より前）は `kind: "immediate"` を除く
  prepared-call 限定の不変条件として、marker 数 === N の事前確認と
  組み合わせた。

## Related Experiments

### Adopted

なし（known-solution task。Experiment Log は未作成）。

### Rejected

なし。

## Validation

- `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/sibling-parallelism.test.ts test/no-execution-mode.test.ts`
  → exit 0、5 test 全 pass（初回 green）
- `mise exec -- pnpm exec biome check packages/runtime-pi` → exit 0

## Risks

- 「全 N 個同時 in-flight」は現行 Pi の unbounded `Promise.all` を
  pin する厳格基準。将来 Pi が batch 並行度 cap を導入した場合は
  「少なくとも 2 個の overlap」への緩和を要する（test コメントに
  明記済み）。
- write→edit 直列化 test は PASS 経路が決定的だが regression 検出側は
  timing 依存（test コメントに強化選択肢を併記済み）。

## Follow-ups

- Task B（prompt guidance + bundle regen）、Task C（execution-engine
  文書追記）、Task D（canonical verification）へ進行。
