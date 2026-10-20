---
id: "TASK-034Jg6kYud58YUAHEYOIDt"
type: "task"
status: "todo"
title: "Add route declaration schema and three-layer loader"
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
    - "docs/test-specs/route-resolution.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route declaration schema and three-layer loader

## Work

`route::routes` — routes.yaml schema 検証(id・match.command・
target{kind,path,args}・passthroughEnv・trusted・unknown field 拒否)、
builtin → user → project の 3 層 merge、project `trusted` shadow +
suppressed provenance、同一長 match.command 重複の invalid-route 化、
`target.path` 層起点 + traversal 拒否。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase B1)

## Dependencies

- なし(Phase B の起点)

## Affected Files

- crates/cheaper-cli/src/route/mod.rs
- crates/cheaper-cli/src/route/routes.rs

## Done When

- [ ] 3 層 merge・shadow・suppressed provenance が unit test で pass
- [ ] 同一長重複が invalid-route として除外される
- [ ] target.path の絶対/traversal/encoding 拒否が pass

## Verification

- [ ] `cargo test -p cheaper-cli` route::routes 系 unit test
