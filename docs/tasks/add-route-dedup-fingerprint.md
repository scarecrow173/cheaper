---
id: "TASK-034Jg6wIaUwhyIQnDJRAuu"
type: "task"
status: "todo"
title: "Add route fingerprint to hook dedup cache"
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
    - "docs/test-specs/hook-dedup-route-fingerprint.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route fingerprint to hook dedup cache

## Work

shell verdict cache key へ effective routes 内容 hash を含め、
routes 変更で stale verdict を invalidation する。Rewrite verdict
(env 含有の可能性)は persist/serve 対象外とし Allow/Deny dedup
のみ維持。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase C3)

## Dependencies

- docs/tasks/add-route-declaration-loader.md (EffectiveRoutes)

## Affected Files

- crates/cheaper-cli/src/hook/eval_cache.rs

## Done When

- [ ] routes 内容 hash が cache key 入力に含まれる
- [ ] Rewrite verdict が persist/serve されない
- [ ] Allow/Deny dedup が従来通り動く

## Verification

- [ ] eval_cache unit test: fingerprint 差分・Rewrite 非 persist
