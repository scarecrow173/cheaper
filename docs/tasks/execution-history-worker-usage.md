---
id: "TASK-034MGHzYDxo5gnB4UShW7P"
type: "task"
status: "done"
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

- `crates/cheaper-cli/src/run.rs`, `crates/cheaper-cli/src/cli.rs`, `crates/cheaper-cli/src/usage_log.rs`, `crates/cheaper-cli/tests/integration/history.rs`

## Done When

- [x] Standalone Worker execution identity is issued before worker/model/input/policy resolution; terminal record covers returned outcomes.
- [x] The standalone Worker passes its current execution/trace values through the optional managed-child carrier; unknown/pre-start utilities remain unrecorded.
- [x] Usage row count and #184 aggregation/refetch semantics are unchanged; correlation fields are additive and optional.
- [x] Pre-start utility/error paths remain unrecorded and History failure does not affect exit/stdout.
- [x] Usage-bearing failure projects only observed resolved model and bounded turn/tool-call metrics from `PartialRunMeta`.
- [x] History adds `usageRecordRef` only after the matching Usage row append succeeds; append failure and pre-usage failure omit the reference.

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration standalone_run_and_usage_share_one_execution_identity` exits 0 with the inherited-hint ownership assertion (1 passed).
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration history::` exits 0 (28 passed, 0 failed, 3 ignored); covers Worker success, usage-bearing failure, append failure, and pre-usage failure reference semantics.
