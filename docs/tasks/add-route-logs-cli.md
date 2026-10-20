---
id: "TASK-034Jg6HVZoKBgwGDZT8ZT6"
type: "task"
status: "done"
title: "Add route logs CLI arm"
created: "2026-10-20"
updated: "2026-10-21"
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
    - "docs/tasks/add-route-log-store.md"
    - "docs/tasks/add-route-usage-records.md"
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
    - "docs/test-specs/route-logs.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route logs CLI arm

## Work

`route::logs` — `route logs <runId> [--offset]` の CLI arm、runId
検証、RouteLogReader bounded read、`kind:"route-refetch"` usage
記録。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase D3)

## Dependencies

- docs/tasks/add-route-log-store.md
- docs/tasks/add-route-usage-records.md

## Affected Files

- crates/cheaper-cli/src/route/logs.rs
- crates/cheaper-cli/src/args.rs / cli.rs

## Done When

- [x] offset/eof semantics が既存 ArtifactReader 規約に従う
- [x] invalid runId が拒否される
- [x] route-refetch record が append される

## Verification

- [x] route logs contract test(offset/eof/invalid-id)
