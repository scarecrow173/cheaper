---
id: "TSPEC-034Jg5WCkcYfanmav19X1o"
type: "test-spec"
status: "draft"
title: "Route and route-refetch usage records"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
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
  verifies:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route and route-refetch usage records

## Purpose

usage.jsonl の `kind:"route"`/`"route-refetch"` record と
`cheaper usage` 独立集計を pin する。

## Feature

route run が delegated 実行の計測 record を継続記録する。

## Rules

- route record は route id・durationMs・exitCode・logRef(runId)を
  持ち、success/usage-bearing failure を対象とする。
- route-refetch は route logs の読取回数を記録する。
- env 値・credential は record に含めない。
- usage report は kind 別集計へ route/route-refetch を独立 join する。
- 書き込みは best-effort で exit code/stdout を変えない。

## Examples

- routed exec 後に usage.jsonl へ kind:"route" が 1 行 append される。

## Guarantees

- 既存 run/count record 集計を破壊しない。

## Non-goals

- worker-level savings 推定の route 拡張。

## Risk

- record 欠落は delegated 実行の計測を失わせる。
