---
id: "TASK-034Jg6j7EASBiaIqXlc07u"
type: "task"
status: "done"
title: "Add proc full-output tee sink for route logging"
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
    - "docs/tasks/add-command-grant-exitcodes.md"
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
    - "docs/test-specs/command-exitcodes-and-options.md"
  references: []
  defers: []
  deferred-by: []
---

# Add proc full-output tee sink for route logging

## Work

`proc::run` に optional `ProcTee` byte sink を追加し、command tool
の bounded output とは別に全量出力を log file へ streaming する。
`CommandToolOptions.log_tee` を追加し tee 失敗時は postprocess
縮退(record only)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase A2)

## Dependencies

- なし(Phase A の独立項目)

## Affected Files

- crates/cheaper-tools/src/tools/proc/mod.rs
- crates/cheaper-tools/src/tools/proc/monitor.rs
- crates/cheaper-tools/src/tools/command/mod.rs

## Done When

- [ ] tee 有効時に全量 byte が sink へ逐次書かれる
- [ ] bounded result と tee 全量の整合(prefix 一致)
- [ ] tee write 失敗が command 結果を壊さず postprocess 記録に縮退

## Verification

- [ ] proc/command unit test: tee 量・truncate 後の bounded 整合・
      sink 失敗注入
