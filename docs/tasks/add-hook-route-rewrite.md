---
id: "TASK-034Jg6vtyvGJ0uGKzCwGcu"
type: "task"
status: "todo"
title: "Add hook route arm and Rewrite verdict"
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
    - "docs/tasks/add-route-resolver-and-payload.md"
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

# Add hook route arm and Rewrite verdict

## Work

`Verdict::Rewrite{command,fallback}` を routing_policy へ追加し、
evaluate_shell に route arm を挿入する — env-prefix 含み・self-bypass・
INTERACTIVE_COMMANDS skip 後に ResolveInput で resolve、runnable
route を `route::payload` で符号化し `current_exe()` anchor の
rewrite command へ。fallback verdict は read-candidate 評価の生
verdict を保持し non-runnable は従来評価へ fallthrough。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase C1)

## Dependencies

- docs/tasks/add-route-resolver-and-payload.md (resolve + payload)

## Affected Files

- crates/cheaper-cli/src/hook/routing_policy.rs

## Done When

- [ ] Rewrite{command,fallback} variant が serde round-trip する
- [ ] runnable route が rewrite command を生成する
- [ ] non-runnable が read-candidate 評価へ fallthrough する
- [ ] fallback deny が rewrite で素通しされない

## Verification

- [ ] evaluate_shell unit test: rewrite/fallthrough/fallback deny
