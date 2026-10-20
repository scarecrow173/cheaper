---
id: "TASK-034Jg6jKK9lLmnOPTaQhAW"
type: "task"
status: "todo"
title: "Add route context scope options and single-grant policy projection"
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

# Add route context scope options and single-grant policy projection

## Work

`InvocationToolRuntimeOptions` 系へ `extra_env`/`exec_cwd`/
`accept_any_exit_code` scope option を追加し registry →
`ctx.command()` へ注入。resolved `RuntimePolicy.command.grants`
を対象 grant 1 件へ投影してから Shared/invocation を構築し、
worker scope の parity check を維持する。`RouteLogReader` の
artifact port 継承経路を確保する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase A3)

## Dependencies

- docs/tasks/add-command-grant-exitcodes.md (accept_any_exit_code の
  result 化機構)

## Affected Files

- crates/cheaper-tools/src/invocation.rs
- crates/cheaper-tools/src/registry.rs
- crates/cheaper-tools/src/tools/command/mod.rs

## Done When

- [ ] scope option が registry 経由で command tool へ届く
- [ ] resolved policy の grants map が単一 grant へ投影される
- [ ] 投影外 grant を input.name で参照した job が失敗する
- [ ] worker job の invocation parity check が通る

## Verification

- [ ] scope option 注入の contract test
- [ ] 単一 grant 投影 + parity check の contract test
