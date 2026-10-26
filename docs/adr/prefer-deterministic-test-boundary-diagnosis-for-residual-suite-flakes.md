---
id: "ADR-034MxsBHBCHSdprnlD1r3r"
type: "adr"
status: "accepted"
title: "Prefer deterministic test-boundary diagnosis for residual suite flakes"
created: "2026-10-26"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/187"
    - "https://github.com/scarecrow173/cheaper/issues/187#issuecomment-6394277590"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/posix-suite-residual-flakes-issue-187.md"
  derived-by:
    - "../plans/stabilize-issue-187-posix-suite-residual-flakes.md"
    - "../test-specs/resources-abort-window-phase-synchronization.md"
    - "../test-specs/hook-capability-diagnostic-error-shape.md"
    - "../test-specs/windows-process-handle-termination-oracle.md"
  verifies: []
  verified-by: []
  references:
    - "../specs/stabilize-posix-suite-residual-flakes-issue-187.md"
    - "../specs/deterministic-proc-process-tests-issue-180.md"
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "resolve-proc-process-test-flake-at-the-test-boundary.md"
    - "resolve-route-test-flake-via-test-side-determinism.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Prefer deterministic test-boundary diagnosis for residual suite flakes

## Context and Problem Statement

Issue #187 groups multiple intermittent failures without evidence of a shared
cause. The resources test's historical Ubuntu stalls occurred on commits that
already contained the post-registration abort reason recheck and waiter
cleanup. Its current mutex + 50 ms sleep does not synchronize the worker phase.
The route failure is already addressed by #157: failed SHA `51f2f4c` precedes
fix `9de23f8`, which removed the child marker assertion. The current route test
checks `ToolExecutionFailure` and `routed command did not complete`.

The hook failure is `ETXTBSY` from `Command::output` while executing a unique
temporary copied PATH shim. No exact failure path, writer, or timestamps were
captured. The Windows report is a dead-to-alive observation: a `tasklist`
substring check first returned not alive inside `wait_for`, then alive in the
following assertion. Neither report proves a production defect. Maintainers
need a repeatable way to control test-owned timing and bounded evidence before
changing runtime behavior.

## Decision Drivers

* Preserve the behavior contracts already asserted by each regression test.
* Pause a test at a proven production-code phase boundary without a global
  hook or public API.
* Bound every wait and produce failure evidence that identifies the first
  failed transition.
* Require causal evidence before changing production lifecycle behavior.
* Avoid a shared fix inferred from unrelated symptoms or one successful rerun.
* Preserve error-shape distinctions so diagnostics describe only observed
  child/process data.

## Considered Options

* **Synchronize and diagnose at each test-owned boundary.** Add a test-owned
  handshake for the resources phase, a bounded watchdog, and failure-only
  operation/identity diagnostics for hook and Windows tests. Keep production
  behavior unchanged unless these tests establish a production invariant
  failure.
* **Change production lifecycle behavior based on plausible race hypotheses.**
  Add retries, longer waits, or altered abort/cleanup behavior to resources,
  process cleanup, or executable invocation without first locating a violated
  invariant.
* **Contain the symptoms in the test harness.** Retry intermittent cases,
  skip under load, or weaken timing/termination assertions so CI completes.

## Decision Outcome

Chosen option: "Synchronize and diagnose at each test-owned boundary",
because it repairs only proven test synchronization or observation defects and
collects evidence before selecting a production owner. Existing #157 route
correction remains the route disposition. The resources historical stall is
post-recheck and unlocalized; it is not evidence that the current recheck is
missing. `ETXTBSY` and the Windows oracle remain independent until evidence
establishes otherwise. The resources test hook pauses after the actual signal
pre-check succeeds and before enqueue, then resumes through the real enqueue,
listener-registration, and recheck code. This proves the pre-check/enqueue
schedule gap only; it does not claim contention on the second listener mutex.

For the Windows process test, use a test-local
`OpenProcess(SYNCHRONIZE | PROCESS_TERMINATE)` handle opened after the PID file
appears and before abort, verify the handle is not already signaled, and use a
bounded wait on that same process object after abort. The terminate access right
is required only because watchdog-expiry fixture cleanup may call
`TerminateProcess` on this same identity; no PID-only kill is used. `tasklist`
is supplemental diagnostics and never the sole oracle. For
hook command diagnostics, separate spawn/exec `Err(io::Error)` from spawned
`Ok(Output)` results; the error branch has no child/status/stdout/stderr.

### Consequences

* Good, because a deterministic resources test can prove it reached the
  listener-registration wait window and can fail within a diagnostic budget.
* Good, because exact executable and process observations help distinguish
  fixture/test-oracle failures from production failures.
* Bad, because test-only phase synchronization may require a narrow seam that
  must be checked against the real control flow.
* Bad, because bounded reproduction may not reproduce historical `ETXTBSY` or
  Windows observations; those cases can remain unresolved with explicit
  evidence and a reviewed disposition.

### Confirmation

Implementation conforms when the resources test proves the targeted phase,
asserts the original abort reason and later manager usability, and has a bounded
watchdog; hook and Windows checks retain their behavior assertions and emit
bounded, identity-specific failure evidence; no retry/skip/assertion weakening
is used; and any production change traces to a reproduced production-owned
invariant failure.

## Implementation Plan

* **Affected paths:**
  `crates/cheaper-tools/src/resources.rs` and its unit test (private
  `#[cfg(test)]` one-shot manager-instance hook); the hook-config capability
  check/error diagnostic owner and `crates/cheaper-cli/tests/integration/hook_config.rs`;
  `crates/cheaper-tools/tests/proc_command.rs`; the #187 discovery, spec,
  design, and relevant indexes.
* **Patterns to follow:** insert the resources hook after the real successful
  signal pre-check and before oneshot creation/enqueue; make clones share only
  that instance hook; use bounded phase/release/result waits and Tokio timeout;
  wait for `JoinHandle::is_finished()` within budget and join only after it is
  finished. Use test-local Win32 `OpenProcess`, `WaitForSingleObject`, and
  `CloseHandle` and `TerminateProcess` declarations with RAII ownership where
  no direct existing dependency is available. Bound the abort-result
  `pending.await` to 20 seconds and include operation, phase, elapsed time,
  allowed budget, and PID in timeout diagnostics. On expiry abort the task
  without another await/join, then run separate fixture cleanup via the same
  process handle with a `TerminateProcess` call and at most a 5-second wait;
  do not count this cleanup as runtime behavior verification. Bound normal
  same-handle termination wait to 20 seconds and close the handle on all paths.
  Use the existing #157 route correction and #180
  process-test boundary guidance where their conditions match; keep
  synchronization and bounded failure assertions in the test owner; preserve
  abort reason, manager reuse, legacy-conflict warning, and child termination
  assertions.
* **Patterns to avoid:** do not use sleep as proof of phase; do not claim the
  pre-enqueue seam proves contention on the later listener mutex; do not infer a
  writer from `ETXTBSY`; do not use non-monotonic PID substring samples as sole
  process-termination proof; do not add retry-on-error, suite retries, skips,
  longer timeout-only changes, weakened assertions, or speculative production
  behavior changes.
* **Dependencies or configuration:** introduce no dependency or new
  configuration. Prefer existing channels, synchronization primitives, and
  process handles. Any required private test seam must be reviewed for
  equivalence with the production transition.

## Verification

* [ ] Resources regression pauses after real pre-check success and before
  enqueue using a private instance-scoped `#[cfg(test)]` one-shot hook, then
  resumes through actual enqueue/listener/recheck logic. It bounds phase and
  result channels, the request future, and worker completion; it joins only
  after `is_finished()`. It asserts original abort reason and later manager use.
* [ ] Hook test retains the warning assertion and branches diagnostics by
  `Command::output()` result: `Err(io::Error)` reports `ErrorKind`, raw OS code,
  message, path, and metadata/timestamps without child/status/output fields;
  `Ok(Output)` reports status and bounded stdout/stderr. Exercise the reported
  parallel condition where available.
* [ ] Windows test opens a `PROCESS_SYNCHRONIZE | PROCESS_TERMINATE` handle
  before abort, verifies its live precondition, and bounds `pending.await` to
  20 seconds with operation/phase/elapsed/allowed-budget/PID diagnostics. On
  expiry it does not await/join again; it aborts the task and performs separate
  `TerminateProcess` fixture cleanup with a same-handle wait bounded to 5
  seconds. The successful result path preserves the original abort reason and
  waits up to 20 seconds on the same handle for termination. RAII closes the
  handle on all paths; `tasklist` is supplemental only.
* [ ] Route test remains on the #157 contract, with no additional #187 code
  change.
* [ ] For each unlocalized symptom, record bounded attempts and a supported
  disposition reviewed independently; do not describe non-recurrence as proof
  of resolution.
* [ ] No production lifecycle behavior changes without a regression that
  demonstrates the violated production invariant before the change.
* [ ] Provisioned Ubuntu POSIX CI runs `mise install`, `mise run setup`, then
  `mise run check`, recording host, commands, and results. WSL without Node is
  not a substitute for this canonical full check.
* [ ] A Windows host runs `mise install` and `mise run check` three consecutive
  times, recording each result. One focused pass is not a substitute.

## Pros and Cons of the Options

### Option 1: Synchronize and diagnose at each test-owned boundary

* Good, because it identifies whether the first broken assumption belongs to a
  test schedule, an observation oracle, or production behavior.
* Bad, because a precise test handshake and useful diagnostics add test code
  and may require a narrowly scoped seam.

### Option 2: Change production lifecycle behavior based on plausible race hypotheses

* Good, because it could fix a real production defect if the suspected
  lifecycle rule is the actual cause.
* Bad, because current evidence does not identify a violated invariant; retries
  or altered semantics could mask the failure and affect unrelated callers.

### Option 3: Contain the symptoms in the test harness

* Good, because it may make a CI run finish despite intermittent observations.
* Bad, because it removes or weakens the regression signal without explaining
  the stall or failure.

## More Information

* [Issue #187](https://github.com/scarecrow173/cheaper/issues/187) and its
  [Windows follow-up](https://github.com/scarecrow173/cheaper/issues/187#issuecomment-6394277590).
* [Issue #157 test determinism ADR](resolve-route-test-flake-via-test-side-determinism.md)
  and [issue #180 process test-boundary ADR](resolve-proc-process-test-flake-at-the-test-boundary.md).
* The #187 discovery and spec link this record as an accepted decision and retain
  the observed SHA ancestry, current assertions, and diagnostic limits.
