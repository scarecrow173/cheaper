---
id: "IMPL-034A9TTHbXQNav6LxGvo49"
type: "impl"
status: "completed"
title: "Operation execution metrics"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: [{type: "file", path: "packages/tool-runtime/src/execution-metrics.ts"}, {type: "file", path: "packages/tool-runtime/test/execution-metrics.test.ts"}]
    modified: [{type: "file", path: "packages/tool-runtime/src/operation.ts"}, {type: "file", path: "packages/tool-runtime/src/invocation.ts"}, {type: "file", path: "packages/tool-runtime/src/index.ts"}, {type: "file", path: "packages/runtime-pi/src/run.ts"}, {type: "file", path: "packages/cli/src/envelope.ts"}]
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/operation-execution-metrics.md"
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

# Operation execution metrics

## Summary

共有 ADR D3 の execution snapshot を runtime timestamps と lifecycle high-water から生成し、Worker success/error と CLI envelope に接続した。

## Context

[task](../../tasks/operation-execution-metrics.md) / [plan](../../plans/operation-execution-metrics.md) と承認済み spec/ADR/design に従う。IR は code 変更後に作成した（task start 前作成の workflow 要件から逸脱したが、実装/検証の実時点を以下に記録する）。

## Implementation

### Changes

ExecutionMetrics と純粋 duration reducer は tool-runtime に単一定義。OperationManager が owner/global peak を観測し、invocation が clock と snapshot を公開する。Worker は owner集合の snapshot を buildMeta/outer catch に付加する。CLI は execution の null/zero を区別し forwarding する。usage.jsonl と scheduler は変更していない。

### Decisions During Implementation

0 duration は duration union へ寄与しないが maxConcurrency は実 start/finish 順を観測する。cancel/detach は実 settle まで running 区間を閉じない。prototype key の byTool を Map→Object.fromEntries で安全に構成する。owned clock は operationClock→invocation.clock、borrowed clock は提供 runtime のもの。snapshot は teardown 前。outer catch は existing runMeta の usage/savings などを保持したまま execution を最新の owner snapshot で上書きする。

## Related Experiments

### Adopted

なし。既知 plan の実装と focused acceptance tests のため exp は作成しない。

### Rejected

なし。

## Validation

- tool-runtime 全 suite: 6 files / 97 tests GREEN（新規 10 tests を含む）。
- runtime-pi focused: 4 files / 51 tests GREEN、最終 run.test.ts 41 tests GREEN。
- CLI focused: 4 files / 46 tests GREEN。
- 新規 tests の RED: metrics 10、Worker 4、CLI forwarding 1（API/field 未実装）。
- pnpm lint / typecheck / build GREEN。lint は既存 33 warnings。
- canonical check deferred to coordinator merge run。実 provider smoke は対象外、fake provider の検証。

## Risks

interval sort は O(n log n)、record と owner peak は invocation lifetime に保持。union/peak は Worker snapshot の加算で Workflow aggregate せず invocation の集合から算出する。

## Follow-ups

coordinator msg_95dfc0113a8b: APPROVED implementation（10 契約 file:line 検証、blocking findings なし）。scoped exit-audit pass: chain findings 0、IR findings 0、共有 ADR blocking 0（既存 nonblocking warnings 2 は維持）。global blocking 48 は scope 外。common brief の caller signal により exit route を完了。logical commit を完了して coordinator に報告する。
