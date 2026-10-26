---
id: "TASK-034MysZSBcJfqWheFs2n1r"
type: "task"
status: "done"
title: "Stabilize process-command test oracles"
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
    - "docs/test-specs/policy-gated-command-tool.md"
  references: []
  defers: []
  deferred-by: []
---

# Stabilize process-command test oracles

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

Canonical Ubuntu CI run `40311338267` on SHA `16f922a` exposed a separate
Unix-only failure in
`security_exec_deny_pin_blast_radius`: its in-workspace helper symlink exists
only to make the resolved executable basename differ from the declared name,
but the shared fixture defaults to `SymlinkPolicy::Deny`. Set only this test's
fixture policy to `WithinWorkspace`, since both symlink and target are inside
the temporary workspace. Preserve the resolved-basename `PolicyViolation`
assertion and make no production policy change.

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
- [x] The Unix-only deny fixture permits only its in-workspace helper symlink, reaches resolved-basename denial, and preserves the original policy assertion.

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools --test proc_command abort_kills_and_preserves_reason` exits 0 on Windows.
- [x] `mise exec -- cargo fmt --check` exits 0.
- [x] WSL baseline on SHA `16f922a`: `security_exec_deny_pin_blast_radius` failed 0/1 with `workspace escape: symlink-denied`.
- [x] WSL green after fixture-only change: the same exact command passed 1/1 with `WithinWorkspace`; production code and denial assertion are unchanged.
- [x] Final canonical Ubuntu workflow `40314463285` passed on corrected code SHA `660cf138541bb77c0318885ad9b50ffb2dbe7dde`. The Unix-only fixture correction is guarded by `#[cfg(unix)]`; the three Windows canonical runs remain valid evidence for the unchanged Windows code path on `16f922a994afe8c41205bdf40eb02759f0f4d670` (run 1 numeric exit not retained; runs 2 and 3 explicit exit 0).
