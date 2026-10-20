---
id: "TASK-034Jg6Irzs6nV7o5ocyN2Q"
type: "task"
status: "done"
title: "Add route subcommand CLI wiring and help docs"
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
    - "docs/tasks/add-route-run-delegated-exec.md"
    - "docs/tasks/add-route-logs-cli.md"
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
    - "docs/test-specs/route-run-delegated-exec.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route subcommand CLI wiring and help docs

## Work

`args.rs`/`cli.rs`/`lib.rs` — `route` subcommand の dispatch
統合、`route run` の generic envelope bypass(raw stdout emit)、
`record_usage` 非対象維持、`resolve_log_root`/`command_name`
更新、HELP text 更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase D4)

## Dependencies

- docs/tasks/add-route-run-delegated-exec.md
- docs/tasks/add-route-logs-cli.md

## Affected Files

- crates/cheaper-cli/src/args.rs
- crates/cheaper-cli/src/cli.rs
- crates/cheaper-cli/src/lib.rs

## Done When

- [x] `cheaper route --help` が全 arm を表示する
- [x] route run が envelope 無し raw stdout を emit する
- [x] dispatch smoke test が pass

## Verification

- [x] CLI dispatch smoke test・help snapshot
