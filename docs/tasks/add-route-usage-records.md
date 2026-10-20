---
id: "TASK-034Jg6uR3fxSVODX2EdmEz"
type: "task"
status: "done"
title: "Add route and route-refetch usage records"
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
  implements:
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-usage-records.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route and route-refetch usage records

## Work

usage_log に `kind:"route"`(route id・durationMs・exitCode・
logRef)と `kind:"route-refetch"` record を追加し、`cheaper
usage` の kind 別集計へ独立 join する。env/credential を含めず、
best-effort append を維持。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase B4)

## Dependencies

- なし(Phase B の独立項目)

## Affected Files

- crates/cheaper-cli/src/usage_log.rs
- crates/cheaper-cli/src/usage_report.rs

## Done When

- [ ] route/route-refetch record が append・parse される
- [ ] usage report 集計へ独立 join される
- [ ] 既存 run/count 集計を破壊しない

## Verification

- [ ] usage_log/usage_report unit test
