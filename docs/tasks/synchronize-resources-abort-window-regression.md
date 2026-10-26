---
id: "TASK-034MysZhQHVTITiwnle4wH"
type: "task"
status: "done"
title: "Synchronize resources abort-window regression"
created: "2026-10-26"
updated: "2026-10-26"
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
    - "docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md"
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
    - "docs/test-specs/resources-abort-window-phase-synchronization.md"
  references: []
  defers: []
  deferred-by: []
---

# Synchronize resources abort-window regression

## Work

In `crates/cheaper-tools/src/resources.rs`, add a private instance-scoped
`#[cfg(test)]` one-shot phase seam immediately after the actual successful
pre-check and before enqueue. Use it to test the existing abort path under a
controlled schedule: pause at the phase, abort, release, and continue through
real enqueue/listener/recheck code. Bound phase/release/request/result/worker
completion waits, join only after `is_finished()`, preserve the exact abort
reason and later manager usability, and avoid claiming second-lock contention.

The regression must compile against the seam and fail on a bounded behavior or
phase assertion if the real transition does not satisfy the test contract; a
missing-symbol compile error is not red evidence. If the current runtime
contract passes once deterministically scheduled, record that the historical
stall did not reproduce and do not infer a production bug.

For a meaningful detector check, first run against the current runtime. If it
passes, temporarily bypass only the existing post-registration abort recheck
as an uncommitted local mutation, run the focused test and require a bounded
abort-result behavior failure, then restore the recheck and rerun green. Record
this mutation check separately from the historical stall reproduction
attempt; never commit the mutation.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md`
- `docs/specs/stabilize-posix-suite-residual-flakes-issue-187.md`
- `docs/designs/issue-187-posix-suite-residual-flake-stabilization.md`
- `docs/adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md`

## Dependencies

- Requires the approved plan and linked test-spec; no other code task is a prerequisite.

## Affected Files

- `crates/cheaper-tools/src/resources.rs`

## Done When

- [x] Phase hook is private, `#[cfg(test)]`, one-shot, and scoped to one manager instance.
- [x] The worker signals only after the real pre-check succeeds and before enqueue; test resumes through real enqueue/listener/recheck logic.
- [x] Every test wait is bounded; on timeout diagnostics name the phase and budget, and join occurs only after `is_finished()`.
- [x] Original abort reason and subsequent manager usability remain asserted; no second-lock contention is claimed.
- [x] Controlled mutation check (baseline passes) is bounded, fails at the behavioral abort-result assertion, and is fully restored before the green run; it is not described as historical stall reproduction.
- [x] Record exact focused run outcome in this task's Implementation Record; a passing current invariant is not described as reproduction of the historical stall.

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools --lib abort_landing_before_listener_registration_still_rejects` exits 0 and exercises the phase-controlled test.
- [x] `mise exec -- rustfmt --check --edition 2021 crates/cheaper-tools/src/resources.rs` exits 0.

## Final Verification Follow-up

- The first canonical Windows `mise run check` exposed a test-only Clippy type-complexity warning. A cfg(test) alias fixes it; focused package Clippy and full formatting checks pass. Canonical checks will be repeated on the corrected commit.
