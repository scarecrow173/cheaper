---
id: "TASK-034AaXZ7nc5LQuUvG9iFY8"
type: "task"
status: "todo"
title: "実 I/O と Pi job lease を配線する"
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
    - "docs/tasks/implement-resource-manager-admission.md"
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
# 実 I/O と Pi job lease を配線する
## Work
read/count/walker と Pi write/edit/ls に observer、session 前 ready を配線する。
## Classification
- normal-plan-task
## Upstream Documents
承認済み docs/plans/implement-resource-manager-job-leases.md。
## Dependencies
A の検証と実装 review 完了。
## Affected Files
plan の File Map の当該 slice。hot files は対象外。
## Done When
- [ ] linked test-spec の当該契約テストが pass。
- [ ] coordinator implementation review 承認。
## Verification
- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi test` exits 0。
- [ ] `mise exec -- pnpm lint` exits 0。
- [ ] `mise exec -- pnpm typecheck` exits 0。
- [ ] `mise exec -- pnpm build` exits 0。

