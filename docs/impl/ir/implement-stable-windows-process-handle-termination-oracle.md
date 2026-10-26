---
id: "IMPL-034MyAvYhFVOfkiKM4JOO3"
type: "impl"
status: "completed"
title: "Implement stable Windows process handle termination oracle"
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
    - "docs/tasks/use-stable-windows-process-handle-termination-oracle.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement stable Windows process handle termination oracle

## Summary

In progress. The Windows process test now captures one test-local process
handle before abort and uses it for the pre-abort liveness check, normal
termination wait, and separate watchdog fixture cleanup. The test retains the
exact abort-reason assertion. The task remains open for coordinator review and
commit.

## Context

Follows `docs/tasks/use-stable-windows-process-handle-termination-oracle.md`,
the approved #187 plan, its process-handle test-spec, and the accepted
deterministic test-boundary ADR.

## Implementation

### Changes

- Added a `#[cfg(windows)]` kernel32 helper in
  `crates/cheaper-tools/tests/proc_command.rs`; it opens with
  `SYNCHRONIZE | PROCESS_TERMINATE` and owns the handle through an RAII guard.
- `abort_kills_and_preserves_reason` opens and checks the handle after the
  fixture PID file appears and before abort. The abort result is guarded by a
  20-second Tokio timeout. If it expires, the test aborts the JoinHandle and
  never awaits it again; fixture cleanup separately calls `TerminateProcess`
  and waits on that same handle for at most five seconds, reporting both API
  results and any Win32 wait error.
- On the normal result path, the test preserves the exact
  `stop managed command` assertion and waits for `WAIT_OBJECT_0` on the same
  handle for at most 20 seconds. `tasklist` remains available for other tests
  through the shared helper but is no longer this test's oracle. Non-Windows
  hosts retain the existing PID liveness behavior.
- No dependency or production API was added.

### Decisions During Implementation

- Used test-local kernel32 FFI as directed by the design, avoiding a crate
  dependency. Win32 wait return codes are reported directly; `GetLastError` is
  captured immediately on failed APIs.
- The requested feature-qualified focused cargo command is not accepted by
  Cargo for `-p cheaper-tools` because those dependency feature names are not
  features of that package. Ran the equivalent package test command without
  those unrelated feature selectors instead.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- Passed on the Windows host:
  `mise exec -- cargo test -p cheaper-tools --test proc_command abort_kills_and_preserves_reason`
  (1 passed, 34 filtered out; repeated after formatting).
- Passed for the owned file:
  `mise exec -- rustfmt --check --edition 2021 crates/cheaper-tools/tests/proc_command.rs`.
- The task's feature-qualified cargo command was attempted and rejected before
  compilation: `cheaper-tools` does not contain features
  `cheaper-cli/dev-seam` or `cheaper-runtime/dev-seam`.
- `mise exec -- cargo fmt --check` reported formatting differences in the
  concurrently owned `crates/cheaper-cli/tests/integration/hook_config.rs` as
  well as the initial formatting in this file during concurrent authoring. The
  final whole-worktree `mise exec -- cargo fmt --check` passes.

## Risks

- The watchdog-expiry branch is bounded and reports cleanup outcomes, but the
  focused successful run did not exercise this exceptional branch.

## Follow-ups

- Independent A/B review, Validator, and Judge accepted this implementation
  slice with no confirmed P0/P1. The coordinator commit is in progress.
