---
id: "SPEC-034A8u2GqFTklBoQi0i1kG"
type: "spec"
status: "approved"
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
  source: ["https://github.com/scarecrow173/cheaper/issues/58"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../discovery/operation-execution-metrics.md"]
  derived-by: ["../designs/operation-execution-metrics.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Operation execution metrics

## What Should Be Built

Worker と後続 Workflow に、観測済み operation の deterministic execution snapshot を提供する。

## Why Now

#58 は concurrency を実測で評価する。共有 ADR D3 と issue Triage を正本とする。

## Users and Value

Worker/Workflow の利用者が queue と実行 overlap を診断できる。

## Scope

### In Scope

- execution: operations/byState/byTool/maxConcurrency/operationMs/activeMs/overlapMs/queueMs。
- 六状態の全 key（0 を含む）、toolName 無しの場合は kind に集計。
- queued 区間は createdAt→startedAt または terminal/snapshot、running 区間は startedAt→実 settle または snapshot。進行中区間は同一 monotonic clock の snapshot 時刻に clamp。
- operationMs は running duration 合計、activeMs は区間 union、overlapMs は差分、queueMs は待機合計。maxConcurrency は lifecycle で観測した最大 running 数。同じ clock 値で start/finish しても観測値を保持する。duration の union は半開区間。
- Worker は ownerId の集合、Workflow は invocation 全体から sum/union/max を再計算する。Worker snapshot の単純加算は禁止。
- success/error の両 meta に同型を追加。service 未生成は null、生成済み 0 件は全値 0 の snapshot。

### Out of Scope

親 agent token 推定、critical path/speedup 推定、usage.jsonl、backend/dashboard/tuning、scheduler 動作変更。

## Requirements

operations=sum(byState)=sum(byTool)。cancelled/timed_out と unsettled を実 lifecycle のまま集計。既存 usage/turns/toolCalls/savings は保持する。

## Acceptance Criteria

- [x] controlled clock で serial/overlap、idle gap、boundary 同時刻、queued cancel、running cancel/detach/late settle を検証。
- [x] owner scoped と invocation 全体の union/max を検証。
- [x] success/error/pre-session error の snapshot と envelope null/zero を検証。
- [x] 既存 scheduler suite を無修正で pass。usage/savings 回帰を検証。

## Sources and Relations

issue #58 と共有 ADR D1/D3/D7。
