---
id: "DESIGN-034MxlLMD6p8niznXIqVhZ"
type: "design"
status: "approved"
title: "Issue #187: POSIX suite residual flake stabilization"
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
    - "../specs/stabilize-posix-suite-residual-flakes-issue-187.md"
    - "../discovery/posix-suite-residual-flakes-issue-187.md"
  derived-by:
    - "../plans/stabilize-issue-187-posix-suite-residual-flakes.md"
  verifies: []
  verified-by: []
  references:
    - "../specs/deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md"
    - "../specs/deterministic-proc-process-tests-issue-180.md"
    - "../adr/resolve-route-test-flake-via-test-side-determinism.md"
    - "../adr/resolve-proc-process-test-flake-at-the-test-boundary.md"
  defers: []
  deferred-by: []
---

# Issue #187: POSIX suite residual flake stabilization

## Context

Issue #187 combines independent observations: an Ubuntu stall in the resources
abort-window test, a route-run assertion later corrected by #157, an `ETXTBSY`
while executing a copied PATH shim during hook configuration testing, and a
Windows process-liveness assertion. Evidence does not establish a shared cause.

The resources stall heads `5e28c40d414a0ae8142c7b5bbc992e6d21b497b9` (run
`40149087832`) and `55741984401e5c84d59fd53bada6b197925f9e2c` (run
`40153767045`) both contain the post-registration `signal.reason()` recheck,
waiter removal, response, and deferred pump; `865765f6` is an ancestor of both.
The historical stall therefore cannot be attributed to a missing recheck. The
current regression test holds the manager mutex and sleeps 50 ms, which does not
prove the worker reached the intended lock-wait phase.

The route failure head `51f2f4c` is an ancestor of #157 fix `9de23f8`. That fix
removed the child marker assertion; the current test verifies
`ToolExecutionFailure` and `routed command did not complete`. Treat the route
symptom as covered by the existing correction and make no #187 route change.

For hook configuration, `ETXTBSY` was reported by `Command::output` during the
capability check of a unique temporary copied PATH shim. The failure-time exact
path, writer, and timestamps were not retained; synchronous copy and unique
temporary directories do not establish overlap. On Windows,
`wait_for(|| !alive(pid))` returned true before the final `assert!(!alive(pid))`
failed. The current `alive` helper searches `tasklist` output by PID substring
and ignores command exit status. This dead-to-alive observation makes the test
oracle suspect but does not prove runtime cleanup failed.

## Scope

### In Scope

- Make the resources regression test synchronize on the relevant waiter phase,
  enforce a bounded watchdog, and report enough phase/lock state to diagnose a
  timeout.
- Keep the current production abort-reason recheck and rejection behavior. A
  runtime change requires new evidence of a production-owned invariant failure.
- Preserve the route test correction as-is and record the historical SHA
  disposition.
- Add bounded, failure-only diagnostics for the hook PATH shim capability check
  sufficient to identify the exact executable, file metadata/timestamps, child
  status, stdout, and stderr.
- Replace or validate the Windows liveness oracle using an existing stable
  process identity/handle if the current test's child lifecycle allows it;
  capture bounded process identity and wait observations.
- For `ETXTBSY` or Windows liveness that cannot be causally localized within a
  bounded investigation, document the evidence and an independently reviewed
  supported disposition.

### Out of Scope

- Retry-on-`ETXTBSY`, test retries, skips, disabled coverage, weakened behavior
  assertions, or timeout extension as a standalone fix.
- New runtime behavior for resources, process cleanup, or hook configuration
  without causal evidence of a production invariant violation.
- Changes to the route-run test already corrected by #157.
- Issue #188 Node cleanup, issue #162 host setup, and unrelated suite failures.

## Components and Boundaries

- `cheaper-tools::resources` test owns synchronization of its controlled race
  and its bounded wait/reporting. The resource manager owns abort state and
  waiter cleanup; production code remains unchanged unless evidence identifies
  a violated manager invariant.
- The hook configuration test owns the copied-executable fixture and reports
  invocation details on failure. It does not infer a concurrent writer from
  `ETXTBSY` alone.
- The Windows process test owns a monotonic child-termination oracle where
  possible. Runtime cleanup owns termination behavior, but an unreliable
  `tasklist` sample is insufficient to assign a cleanup defect.
- The route test remains under the accepted #157 correction.

## Data and Control Flow

### Resources abort-window regression

The observed source order in `ResourceManager::request` is: signal reason
pre-check (before enqueue), waiter enqueue under the first `inner` lock, abort
listener registration, then listener-id attachment and the post-registration
`signal.reason()` recheck under a second `inner` lock. The current test's
mutex-from-start only demonstrates that it can block the first enqueue lock; it
does not synchronize the intended post-precheck abort schedule.

Use a private `#[cfg(test)]` field on `ResourceManager` containing an
instance-scoped, one-shot hook; clone copies share the hook with the same inner
instance. A private setter is available only to this unit test. Invoke the hook
immediately after the real successful signal pre-check and before oneshot
creation/enqueue. The hook sends a reached-phase message and waits on a
test-owned bounded release channel. It has no global state, adds no public API,
and compiles out of production builds.

1. Acquire the fixture's held lease and configure the instance hook before
   starting the worker thread/runtime.
2. The worker calls the real `request` path. The test receives the phase message,
   proving the real pre-check returned no reason and that execution is paused
   before queue enqueue.
3. Abort with the original reason; release the hook. The request then traverses
   the actual enqueue, listener registration, and post-registration recheck.
   This deterministically exercises the post-precheck/pre-enqueue gap. It does
   **not** claim the worker is queued on or contending for the second mutex.
4. Wrap the request future in `tokio::time::timeout`. Bound both the phase
   receive and result receive. Report phase, elapsed time, and expected event on
   timeout. On normal completion assert rejection with the original reason.
5. After observing the result, poll `JoinHandle::is_finished()` only until the
   outer deadline; call `join()` only after it returns true. On timeout, release
   the phase hook and fail without an unbounded blocking `join()`. The worker's
   request and hook waits are independently bounded; its manager instance is
   test-local.
6. Submit an independent request to the same manager and require it to
   complete, proving the waiter cleanup did not poison future use.

### Hook executable diagnostic

1. Preserve the existing copy-then-execute sequence and legacy-conflict warning
   assertion.
2. Branch explicitly on `Command::output()`:
   - `Err(io::Error)` means spawn/exec did not produce a child. Record
     `ErrorKind`, `raw_os_error`, bounded display message, exact source and
     copied executable paths, and bounded metadata/timestamps sampled around
     invocation. Do not report child PID/status/stdout/stderr for this branch.
   - `Ok(Output)` means a child was spawned. Record its `ExitStatus` and bounded
     stdout/stderr separately. A nonzero status is not an `io::Error` spawn
     failure.
3. Run the bounded focused test and the reported parallel-load condition when
   available. A captured failure identifies its actual branch and artifact;
   do not retry it away.
4. If no causal `Err` can be reproduced, record the bounded conditions and
   available evidence as an unresolved disposition. Do not claim a writer/race
   or prescribe a product fix without an identified overlapping operation.

### Windows process-liveness diagnostic

1. After the child writes its PID file and before calling `abort`, open a
   test-local Win32 process handle with
   `OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, FALSE, pid)`. Immediately
   verify `WaitForSingleObject(handle, 0) == WAIT_TIMEOUT`; this establishes
   that the opened process object is still running before abort.
2. Start a monotonic timer, call abort, and await `pending` under a finite
   20-second outer watchdog. On success, retain the exact abort-reason
   assertion; then wait on the same process handle for at most 20 seconds.
   `WAIT_OBJECT_0` proves termination of the opened process identity;
   `WAIT_TIMEOUT` is a bounded liveness failure; `WAIT_FAILED` records
   `GetLastError`.
3. If the pending-result watchdog expires, report operation name, phase
   (`await-abort-result`), elapsed time, allowed budget, and PID. Call
   `pending.abort()` without awaiting/joining it again. Then perform separate,
   bounded fixture cleanup: call `TerminateProcess` on the same process handle
   and wait on it for at most 5 seconds. Record cleanup API result and error
   codes; do not treat cleanup as successful verification of runtime cleanup.
4. Wrap the handle in a test-local RAII guard immediately after opening it so
   it is closed on success, assertion failure, watchdog expiry, and cleanup
   errors. Declare the required `kernel32` functions in a `#[cfg(windows)]`
   test helper; no crate dependency or production API change is needed. Keep the
   unsafe block local with documented handle validity/ownership. `tasklist`
   output is optional supplemental diagnostics and never the sole oracle.
5. If `OpenProcess` cannot obtain a handle or the platform contract makes this
   unsafe/incompatible, record the exact error and request an independently
   reviewed alternative disposition; do not silently fall back to PID
   substring matching.

## Non-Functional Constraints

- Every synchronization and wait is bounded. Timeout diagnostics must identify
  the expected phase and elapsed budget.
- The post-abort `pending` result has a finite outer watchdog. Its expiry never
  triggers a second unbounded await/join; fixture cleanup has a separate finite
  budget and cannot count as runtime-termination verification.
- The Windows process handle closes on every path, including assertion,
  watchdog, and fixture-cleanup failures.
- Keep abort reason, future manager usability, hook warning behavior, and
  process termination assertions intact.
- Diagnostics are failure-only and bounded in size; do not emit environment
  secrets or unrelated process data.
- Reproduction attempts must be bounded. Lack of recurrence is recorded as
  uncertainty, not proof of resolution.
- No dependency or public runtime API is introduced by this design. The
  resources hook is `#[cfg(test)]` and instance-scoped; Windows FFI is confined
  to the integration test.

## Risks and Trade-offs

- The one-shot phase hook sits after the real pre-check and before enqueue. It
  proves that abort occurs inside that schedule gap while the real request body
  continues, but it does not prove contention on the second listener mutex.
- `Command::output` `Err(io::Error)` has no child or output streams; mixing its
  fields with `Ok(Output)` would fabricate failure evidence. Keep the branches
  separate in diagnostic output.
- Capturing executable metadata and timestamps still may not identify an
  external writer. If no writer evidence is available, the outcome remains
  unresolved.
- The process handle is opened while the PID-file child is expected to be live
  and before abort; a zero-time wait checks that precondition. The same handle
  then avoids PID reuse and non-monotonic process-list sampling for termination
  evidence.
- Bounded diagnostics reduce ambiguity without guaranteeing historical
  nondeterministic failures recur in the available environment.

## Deferred Design Concerns

- Any production resource-manager or process-cleanup change | deferred because
  the historical resources stall occurred with the recheck present and the
  Windows failure may be a non-monotonic test oracle | reopen only with a
  reproducer or causal event trace proving a production invariant violation |
  speculative behavior change could alter unrelated consumers.
- A root-cause fix for hook `ETXTBSY` | deferred until a bounded failure captures
  a writer or another causal filesystem/process operation | reopen when exact
  executable and overlapping operation evidence exists | retries could hide a
  fixture or runtime defect.

## Verification Notes

- [ ] Resources phase synchronization uses the test-only instance hook after
  actual pre-check success and before enqueue; tests exercise the real enqueue,
  listener registration, and recheck code. Timeout bounds phase wait, request
  future, result wait, and worker-finished polling; join happens only after
  `is_finished()`. Assert original reason and subsequent manager usability.
- [ ] Hook diagnostics distinguish `Err(io::Error)` fields (`ErrorKind`, raw OS
  code, message, exact path, metadata/timestamps; no child/status/output) from
  `Ok(Output)` status and bounded stdout/stderr. Retain the legacy warning
  assertion.
- [ ] Windows opens a `PROCESS_SYNCHRONIZE | PROCESS_TERMINATE` handle before
  abort and confirms it is live. Bound `pending` result wait to 20 seconds with
  operation/phase/elapsed/allowed-budget/PID diagnostics; on expiry, abort the
  task without another await/join, then run separate `TerminateProcess` plus
  same-handle wait bounded to 5 seconds. The normal result path preserves the
  exact abort-reason assertion and waits up to 20 seconds on that same handle
  for termination. RAII closes the handle on every path; `tasklist` is
  supplemental only.
- [ ] Confirm historical route SHA ancestry and retain the current
  `ToolExecutionFailure` / `routed command did not complete` assertions without
  an additional route code change.
- [ ] Independent review each causal claim or unresolved disposition before
  implementation approval. No product-code change is authorized by this approved design.

## Review and Approval Record

- Round 1 Judge: `ACCEPT WITH NON-BLOCKING NOTES`, with one confirmed P1 on
  the resources handshake proof. The design was corrected to use a
  test-owned, instance-scoped one-shot phase hook after the actual successful
  pre-check and before enqueue; it makes no second-listener-mutex contention
  claim.
- Round 2 Reviewer A and Reviewer B completed independent reviews. Validator
  ledger: `0 confirmed / 0 rejected / 0 unverified`.
- Final Judge: `ACCEPT`; no confirmed P0/P1. The accepted design keeps the
  route disposition on #157, and leaves the hook `ETXTBSY` and historical
  resource stall causes unresolved pending bounded evidence/disposition.
- Approval: user-delegated approval authority exercised after blockers were
  cleared. This approves the design as implementation input; it does not
  authorize product code or implementation execution.

### Required Full Verification Matrix

| Host | Commands | Required result | Current evidence |
| --- | --- | --- | --- |
| Provisioned Ubuntu POSIX CI | `mise install`; `mise run setup`; `mise run check` | Full canonical pipeline passes; record host, each command, and result | Pending. WSL Node is unavailable, so it cannot stand in for the canonical POSIX full check. |
| Windows host | `mise install`; then `mise run check` three consecutive times | All three canonical checks pass; record host, run order, commands, and each result | Pending. Prior focused Windows tests passed once only; that is not the repetition gate. |

The WSL resources target (1/1) and `resources::tests` module (15/15) passed in
the supplied focused probe. This is focused evidence only; it does not satisfy
either full-verification row.
