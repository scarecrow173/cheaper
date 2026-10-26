---
id: "TASK-034MCBOZ7xsFWR2eROLiV5"
type: "task"
status: "done"
title: "Remove ambient Node from workflow fixture"
created: "2026-10-26"
updated: "2026-10-26"
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
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
  implemented-by: []
  depends-on:
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
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
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  references: []
  defers: []
  deferred-by: []
---

# Remove ambient Node from workflow fixture

## Work

Replace workflow process test Node discovery and silent return with a
self-reexecuted exact test-binary fixture child. Preserve producer/consumer,
materialization, declared output, and resource assertions; execute the test
under a PATH where Node is absent.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/remove-ambient-node-from-rust-tests-issue-188.md`
- `docs/specs/remove-ambient-node-test-coupling-issue-188.md`
- `docs/designs/remove-ambient-node-from-rust-tests-issue-188.md`

## Dependencies

- Requires the approved issue188 plan; no task dependency.

## Affected Files

- `crates/cheaper-workflow/tests/process.rs`
- `docs/impl/ir/remove-ambient-node-from-workflow-fixture.md`

## Done When

- [x] Workflow process test executes its fixture child with Node absent and
  does not silently skip.
- [x] Workflow producer/consumer, output, and resource contract assertions
  remain active and pass.

## Verification

- [x] In WSL with process-local PATH from `rustup which cargo/rustc`, verified
  Node absent, run `cargo test -p cheaper-workflow --test process compiles_policy_process_producer_and_executes_materialized_jobs -- --exact`.
- [x] Run `mise exec -- cargo test -p cheaper-workflow --test process compiles_policy_process_producer_and_executes_materialized_jobs -- --exact` on Windows.
