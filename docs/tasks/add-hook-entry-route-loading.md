---
id: "TASK-034Jg6xqeFUtHyZ5MxHgzu"
type: "task"
status: "todo"
title: "Add hook entry route loading and eval supply"
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
    - "docs/tasks/add-route-declaration-loader.md"
    - "docs/tasks/add-hook-route-rewrite.md"
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
    - "docs/test-specs/hook-route-rewrite.md"
  references: []
  defers: []
  deferred-by: []
---

# Add hook entry route loading and eval supply

## Work

hook entry(mod.rs/cli.rs)で effective routes + command policy を
1 firing 1 回 load し、`HookDeps`/`EvaluateOptions` 経由で
evaluate_shell へ供給する — per-candidate reload を避ける。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase C4)

## Dependencies

- docs/tasks/add-route-declaration-loader.md
- docs/tasks/add-hook-route-rewrite.md

## Affected Files

- crates/cheaper-cli/src/hook/mod.rs
- crates/cheaper-cli/src/cli.rs

## Done When

- [ ] hook firing あたり 1 回の routes/policy load に集約される
- [ ] eval 側が ResolveInput 用の両者を受け取れる

## Verification

- [ ] hook entry contract test(load 回数・供給経路)
