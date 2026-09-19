---
id: "TASK-034rzRtRHEEtyWvIINcDnL"
type: "task"
status: "todo"
title: "Measure tool output tokens and expose meta.savings"
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

# Measure tool output tokens and expose meta.savings

## Work

DESIGN-034rzNZ2BZGTPuOwHmyqh4 に従い、tool output の token 計測
（meter）と `meta.savings` の組み立てを実装する。

### 1. core: `countTextTokens`

- `packages/core/src/token/count.ts` に
  `export function countTextTokens(text: string): number` を追加。
  module scope の `encoding`（o200k_base）を再利用。

### 2. runtime-pi: tool meter

- `packages/runtime-pi/src/tools/meter.ts`（新規）:
  `wrapToolMeter(tools)` が各 `ToolDefinition.execute` の戻り値
  `content[].text` を `countTextTokens` で encode して累積する
  `ToolMeter { sourceTokens }` を返す。error throw は計上しない。
- `createSandboxedTools`（registry.ts）で `wrapToolBudget` の
  内側（budget 通過後の成功 output のみ計測）に meter を挟み、
  `SandboxedTools.meter` として公開する。

### 3. runtime-pi: `meta.savings`

- `WorkerRunMeta` に `savings?: { sourceTokens: number;
  returnedTokens: number; avoidedContextTokens: number }` を追加。
- `run.ts` の `buildMeta` に result を渡せる経路を作り、success
  時は `returnedTokens = countTextTokens(JSON.stringify(result))`、
  `avoidedContextTokens = max(0, source - returned)`。error 経路は
  `returnedTokens: 0`。
- `packages/cli/src/envelope.ts` の meta 型は `WorkerRunMeta`
  ベースなので追従のみ（optional field）。

### 4. unit test

- meter: text output の token 和・error 非計上・0 件。
- run.ts: fake stream + tool call で `meta.savings` の 3 field。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/usage-savings-telemetry-implementation-plan.md`
- `docs/designs/usage-savings-telemetry.md`
- `docs/specs/usage-and-savings-telemetry-aggregation.md`
- `docs/adr/measured-offload-tool-output-token-estimated-savings-pricing-seam.md`

## Dependencies

- なし（本 plan の先頭 task）

## Affected Files

- `packages/core/src/token/count.ts`（modify）
- `packages/core/test/*`（modify/add）
- `packages/runtime-pi/src/tools/meter.ts`（add）
- `packages/runtime-pi/src/tools/registry.ts`（modify）
- `packages/runtime-pi/src/usage.ts`・`run.ts`（modify）
- `packages/runtime-pi/test/*`（add/modify）

## Done When

- [ ] `countTextTokens` が core から export される
- [ ] `meta.savings` が success/error 双方で組み立てられる
- [ ] unit test pass・typecheck・lint pass

## Verification

- [ ] `pnpm --filter @cheaper/runtime-pi test` pass
- [ ] `pnpm --filter @cheaper/core test` pass
- [ ] `pnpm typecheck`・`pnpm lint` pass
