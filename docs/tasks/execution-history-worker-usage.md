---
id: "TASK-034MGHzYDxo5gnB4UShW7P"
type: "task"
status: "todo"
title: "Wire standalone Worker and Usage correlation"
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
  references: []
  defers: []
  deferred-by: []
---

# Wire standalone Worker and Usage correlation

## Work

Instrument the standalone Worker owner at the approved pre-resolution boundary, terminalize every returned outcome best-effort, and add optional execution/parent/trace IDs to existing Usage rows without changing owner cardinality or #184 aggregation.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-execution-history-for-issue-192.md`

## Dependencies

- `docs/tasks/execution-history-core.md` (`TASK-034MGH8FHwescRINQne4sz`)

## Affected Files

- `crates/cheaper-cli/src/run.rs`, `crates/cheaper-cli/src/usage_log.rs`

## Done When

- [ ] Standalone Worker execution identity is issued before worker/model/input/policy resolution; terminal record covers returned outcomes.
- [ ] Usage row count and #184 aggregation/refetch semantics are unchanged; correlation fields are additive and optional.
- [ ] Pre-start utility/error paths remain unrecorded and History failure does not affect exit/stdout.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration history::` exits 0 with nonempty matching tests.
