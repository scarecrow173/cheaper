---
id: "TASK-034MGHBRUesH1Zj3pJiqH7"
type: "task"
status: "todo"
title: "Record Route and refetch lifecycle evidence"
created: "2026-10-27"
updated: "2026-10-27"
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
    - "docs/plans/implement-execution-history-for-issue-192.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-execution-history-for-issue-192.md"
    - "TASK-034MGH8FHwescRINQne4sz"
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
    - "TSPEC-034MGCxlMYuOemMCKIx9RZ"
    - "TSPEC-034MGCCC9F0R4NwYCGFOlk"
  references: []
  defers: []
  deferred-by: []
---

# Record Route and refetch lifecycle evidence

## Work

Wire authoritative Route run and refetch lifecycle records, typed pure resolution evidence, and logRef correlation. Inspection utilities remain write-free; preserve existing runId/logRef semantics.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-execution-history-for-issue-192.md`

## Dependencies

- `docs/tasks/execution-history-core.md` (`TASK-034MGH8FHwescRINQne4sz`)

## Affected Files

- `crates/cheaper-cli/src/route/run.rs`, `route/logs.rs`, `route/resolve.rs`
- `crates/cheaper-cli/tests/integration/route_history.rs`

## Done When

- [ ] Route runs and refetches receive distinct authoritative IDs and retain current runId/logRef contracts.
- [ ] Refetch joins only by logRef; pure resolve/check utilities and `route_cmd.rs` do not write History or Usage.
- [ ] Best-effort persistence cannot alter Route outcomes.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_history::` exits 0 with registered tests.
