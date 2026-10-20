---
id: "TASK-034Jg6ui9tEt8GFLDetq57"
type: "task"
status: "done"
title: "Add route log store and RouteLogReader"
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
    - "docs/test-specs/route-logs.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route log store and RouteLogReader

## Work

`route::log_store` — `<userConfigDir>/route-logs/` への
`<runId>.log.tmp` streaming + rename・0600・runId 採番・
retention sweep(TTL 30 日/200 file/256 MiB、`*.tmp` 除外)・
`RouteLogReader`(ArtifactReader impl、runId 検証共有)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md (Phase B3)

## Dependencies

- なし(Phase B の独立項目)

## Affected Files

- crates/cheaper-cli/src/route/log_store.rs
- crates/cheaper-cli/src/route/mod.rs

## Done When

- [ ] tmp write → rename・0600 permission が pass
- [ ] sweep が TTL/count/size を満たし `*.tmp` を残す
- [ ] RouteLogReader の ref 検証で traversal/invalid id が拒否される

## Verification

- [ ] log_store unit test: write/rename・sweep・ref 検証
