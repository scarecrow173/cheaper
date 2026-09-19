---
id: "IMPL-034rArrb2uZO7zl0EEFqea"
type: "impl"
status: "completed"
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
    - "../../tasks/measure-tool-output-tokens-and-expose-meta-savings.md"
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

# Measure tool output tokens and expose meta.savings

## Summary

tool output の token 計測（ToolMeter）を導入し、worker run の
`meta.savings`（`sourceTokens` / `returnedTokens` /
`avoidedContextTokens`）を組み立てる実装を行った。

## Context

TASK-034rzRtRHEEtyWvIINcDnL。DESIGN-034rzNZ2BZGTPuOwHmyqh4 /
ADR-034rzGnXqru47VgHNL16cS に従い、offload の実測値を run meta に
載せる。

## Implementation

### Changes

- `packages/core/src/token/count.ts`: `countTextTokens(text)` を
  追加（module scope の o200k_base encoding を再利用）。
- `packages/runtime-pi/src/tools/meter.ts`（新規）:
  `wrapToolMeter` が各 `ToolDefinition.execute` の戻り値
  `content[].text` を encode して `sourceTokens` に累積。
  throw した call は計上しない。
- `packages/runtime-pi/src/tools/registry.ts`: meter を
  `wrapToolBudget` の内側に挟み、`SandboxedTools.meter` として
  公開。
- `packages/runtime-pi/src/usage.ts`: `WorkerSavings` 型を追加。
- `packages/runtime-pi/src/run.ts`: `buildMeta(result?)` が
  `savings` を組み立てる。success 経路は result を渡し
  `returnedTokens = countTextTokens(JSON.stringify(result))`、
  error 経路は `returnedTokens: 0`。

### Decisions During Implementation

- meter は budget 内側に配置し、budget 超過で reject された call
  の output を計上しない構成とした。
- `meta.usage.input` は turn 累積を含むため source の計測には
  使わず、tool output text の和を採用した（ADR の決定どおり）。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `vitest run test/tools/meter.test.ts test/run.test.ts` — meter の
  text 和・error 非計上・0 件、および fake stream + 実 read tool
  call で `meta.savings` 3 field・clamp・error 経路
  `returnedTokens: 0` を検証（pass）。
- `pnpm typecheck` / `pnpm lint` pass。

## Risks

- `sourceTokens` は tool output の和であり grep listing 等を含む
  近似値（guide で限界を明記済み）。

## Follow-ups

なし。
