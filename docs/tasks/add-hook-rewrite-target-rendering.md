---
id: "TASK-034Jg6w6lRkQeutruaa2Wd"
type: "task"
status: "todo"
title: "Add hook target rendering for Rewrite and codex split"
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

# Add hook target rendering for Rewrite and codex split

## Work

targets.rs — `HookTarget::Codex` variant 追加、claude/devin の
`hookSpecificOutput.updatedInput` への Rewrite render、codex 含む
他 target への fallback verdict render。apm-hook-codex の hooks.json
を `--target codex` へ分離し claude 側から除去。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase C2)

## Dependencies

- docs/tasks/add-hook-route-rewrite.md (Rewrite variant)

## Affected Files

- crates/cheaper-cli/src/hook/targets.rs
- packages/apm-hook-codex/.apm/hooks.json
- packages/apm-hook-claude/.apm/hooks.json

## Done When

- [ ] claude/devin target が updatedInput.command を emit する
- [ ] codex/windsurf target が fallback verdict のみ render する
- [ ] codex hooks.json が --target codex を登録する

## Verification

- [ ] targets render unit test(claude/devin/codex/windsurf)
