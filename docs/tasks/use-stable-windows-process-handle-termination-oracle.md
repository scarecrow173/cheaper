---
id: "TASK-034MysZSBcJfqWheFs2n1r"
type: "task"
status: "done"
title: "Use stable Windows process handle termination oracle"
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
    - "docs/test-specs/windows-process-handle-termination-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Use stable Windows process handle termination oracle

## Work

In `crates/cheaper-tools/tests/proc_command.rs`, replace PID-substring
`tasklist` liveness as the termination oracle with a test-local
`OpenProcess(SYNCHRONIZE | PROCESS_TERMINATE)` handle captured after the PID file
appears and before abort. Confirm the handle is live, bound the pending abort
result to 20 seconds with operation/phase/elapsed/allowed-budget/PID
diagnostics, and use the same handle for a normal-path termination wait capped
at 20 seconds. If the pending watchdog expires, do not await/join again; abort
and perform separate fixture cleanup using `TerminateProcess` and a same-handle
wait capped at 5 seconds. Use RAII to close the handle on all paths. `tasklist`
may provide supplemental diagnostics only.

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

- `crates/cheaper-tools/tests/proc_command.rs`

## Done When

- [x] A process handle is opened before abort with the required rights and is verified live before use.
- [x] The abort-result watchdog is bounded to 20 seconds; normal termination wait uses the same handle and is bounded to 20 seconds.
- [x] Watchdog expiry never triggers a second await/join; fixture cleanup is separate, same-handle, and bounded to 5 seconds with API result/error diagnostics.
- [x] RAII closes the handle on success, assertions, timeout, and fixture-cleanup failures; the original abort-reason assertion remains.

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools --test proc_command abort_kills_and_preserves_reason` exits 0 on Windows.
- [x] `mise exec -- cargo fmt --check` exits 0.
