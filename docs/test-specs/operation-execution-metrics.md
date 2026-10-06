---
id: "TSPEC-034A9yAv68yBK2XeFKEToG"
type: "test-spec"
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
  source: []
  implements: []
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
  verifies: ["../specs/operation-execution-metrics.md","../designs/operation-execution-metrics.md"]
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Operation execution metrics

## Purpose

operation execution の観測値が lifecycle と実時間区間に一致することを保証する。

## Feature

owner scoped Worker と invocation 全体が同じ metrics 定義を使用する。

## Rules

1. counts は全六状態を持ち、sum(byState)=sum(byTool)=operations。拒否は計測対象外。
2. queue は開始/terminal/snapshot まで、running は実 settle/snapshot まで。cancel/detach は実停止と見なさない。
3. union は idle gap を含まず、overlap は duration 合計との差。max は同時刻 0 duration でも実 lifecycle high-water。
4. owner と invocation の union/max は集合ごとに計算する。
5. success/error は snapshot を持ち、未生成の error は null、生成済み 0 件は zero。usage/turns/toolCalls/savings を保持する。

## Examples

serial 2×20ms: operation=40/active=40/overlap=0/max=1。overlap [10,40),[20,50): operation=60/active=40/overlap=20/max=2。queued cancel を queue 合計へ含む。running cancel/detach/late settle を区別する。prototype tool key と snapshot mutation を検証する。

## Guarantees

controlled clock で deterministic。scheduler suite を変更せず pass。モデル主張や parent token 推定は execution へ混ぜない。

## Non-goals

実 provider semantic quality、critical path/speedup、parent usage、usage.jsonl。

## Risk

clock tie、snapshot teardown、cross-owner aggregation が誤ると観測値が過大/過小となる。
