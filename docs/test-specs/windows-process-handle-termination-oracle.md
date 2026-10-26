---
id: "TSPEC-034MyrGo0H4WXaTySC7fbI"
type: "test-spec"
status: "draft"
title: "Windows process handle termination oracle"
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
    - "docs/adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md"
  derived-by: []
  verifies:
    - "docs/specs/stabilize-posix-suite-residual-flakes-issue-187.md"
    - "docs/designs/issue-187-posix-suite-residual-flake-stabilization.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Windows process handle termination oracle

## Purpose

Preserve reliable evidence that cancellation terminates the exact child owned
by the test. Keep this test while process cancellation is verified on Windows;
retire it only if an equivalent stable process-identity oracle replaces the
handle-based check.

## Feature

Feature: abort completion and child termination use one bounded process identity

## Rules

- Rule: after the PID file appears and before abort, open one process handle
  with `SYNCHRONIZE | PROCESS_TERMINATE` and verify it is initially live.
- Rule: bound the abort result wait to 20 seconds and preserve the exact abort
  reason assertion.
- Rule: on the normal result path, wait at most 20 seconds for termination on
  the same handle; do not sample `tasklist` as the sole oracle.
- Rule: if the abort-result watchdog expires, do not await/join the pending
  operation again. Abort it and use a separate at-most-5-second
  `TerminateProcess` fixture cleanup on the same handle, recording cleanup
  result/error separately from runtime verification.
- Rule: an RAII guard closes the handle on success, assertion failure, timeout,
  and fixture-cleanup error.

## Examples

- Example: Given the child is live on the captured process handle before abort,
  when cancellation completes, then the same handle becomes signaled within
  the normal 20-second budget and the original abort reason is preserved.
- Example: Given the pending result exceeds its 20-second budget, when the
  watchdog fires, then it aborts without a second await and separately reports
  the result of bounded same-handle fixture cleanup.

## Guarantees

- Process identity is stable across the pre-abort liveness check and
  post-abort termination wait, avoiding PID reuse and non-monotonic process-list
  sampling.
- Runtime termination verification and watchdog fixture cleanup remain
  separate outcomes.

## Non-goals

- The historical reason for the single Windows dead-to-alive `tasklist`
  observation.
- Cleanup success as proof of normal runtime cancellation.

## Risk

- An unstable process-list observation can report contradictory liveness and
  can neither establish child identity nor prove termination.
