---
id: "TASK-034AaXUraSXQKdlQ8s1p3p"
type: "task"
status: "done"
title: "Lease と atomic admission を実装する"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-resource-manager-job-leases.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-resource-manager-job-leases.md"
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
    - "docs/test-specs/resource-manager-job-leases.md"
  references: []
  defers: []
  deferred-by: []
---
# Lease と atomic admission を実装する
## Work
ResourceManager、scope controller、invocation/scheduler/registry を実装する。
## Classification
- normal-plan-task
## Upstream Documents
承認済み docs/plans/implement-resource-manager-job-leases.md。
## Dependencies
承認済み plan/spec/design。
## Affected Files
plan の File Map の当該 slice。hot files は対象外。
## Done When
- [x] linked test-spec の当該契約テストが pass。
- [x] coordinator implementation review 承認。
## Verification
- [x] `mise exec -- pnpm --filter @cheaper/tool-runtime test` exits 0。
- [x] `mise exec -- pnpm lint` exits 0。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。
