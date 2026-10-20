---
id: "TASK-034Jg6ieFXYQcEldcUVKri"
type: "task"
status: "done"
title: "Add command grant exitCodes and nonzero-exit result reporting"
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
    - "docs/test-specs/command-exitcodes-and-options.md"
  references: []
  defers: []
  deferred-by: []
---

# Add command grant exitCodes and nonzero-exit result reporting

## Work

`command.grants.<name>.exitCodes`(default `[0]`)を policy schema
へ追加し、command tool が宣言 exit の完了を error ではなく result
(exitCode details 付き)として返すよう result 化する。route 向け
`accept_any_exit_code` option の基盤となる機構。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase A1)
- docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md

## Dependencies

- なし(Phase A の独立項目)

## Affected Files

- crates/cheaper-core/src/policy/command.rs
- crates/cheaper-tools/src/tools/command/mod.rs

## Done When

- [ ] exitCodes schema が parse/検証され default `[0]` が適用される
- [ ] 宣言 exitCodes 内の非 0 exit が result(state=completed +
      exitCode details)として返る
- [ ] 未宣言の非 0 exit は従来通り error(state=failed)のまま

## Verification

- [ ] command tool unit test: exit 0/宣言非 0/未宣言非 0 の 3 分岐
- [ ] policy schema contract test: exitCodes parse・default
