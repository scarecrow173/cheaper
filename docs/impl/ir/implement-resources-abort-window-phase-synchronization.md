---
id: "IMPL-034MyAvsjlrMD8xqio3RpN"
type: "impl"
status: "completed"
title: "Implement resources abort window phase synchronization"
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
    - "docs/tasks/synchronize-resources-abort-window-regression.md"
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

# Implement resources abort window phase synchronization

## Summary

In progress. This record is open before code changes for the resources
abort-window task. Planned scope is the private test-only phase seam and its
bounded phase-controlled regression. The phase seam and regression are
implemented; focused current-runtime and controlled-mutation evidence is
recorded below. Historical Ubuntu stall reproduction remains unverified.

## Context

Follows `docs/tasks/synchronize-resources-abort-window-regression.md`, the
approved #187 plan, its resources test-spec, and the accepted deterministic
test-boundary ADR.

## Implementation

### Changes

Added an instance-shared `#[cfg(test)]` one-shot `FnOnce` hook consumed after
the actual successful signal pre-check and before oneshot/enqueue. The test
uses a bounded phase signal and release channel, aborts while paused, resumes
through real enqueue/listener/recheck code, awaits request result with an outer
budget, waits for `JoinHandle::is_finished()` before joining, and verifies a
later request using the same worker runtime. It does not claim second-listener
mutex contention or cross-runtime manager reuse.

### Decisions During Implementation

The first follow-up request was attempted on the original test runtime and
timed out after the abort branch scheduled its deferred repump on the worker
runtime. The follow-up was moved into that same worker runtime, where manager
reuse passes. This is a test runtime-lifetime observation only; it is not
claimed as the historical issue cause or a new cross-runtime contract.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo test -p cheaper-tools --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib abort_landing_before_listener_registration_still_rejects` — rejected before compilation because `cheaper-tools` declares no such features; no test ran.
- `mise exec -- cargo test -p cheaper-tools --lib abort_landing_before_listener_registration_still_rejects` — initial harness attempt failed because `tokio::time::timeout` was constructed outside a runtime; moved timeout construction inside `async`.
- Same focused command — passed, 1 test. An intermediate follow-up-on-original-runtime attempt timed out; moved manager reuse into the request worker's still-live runtime, then focused test passed.
- Controlled mutation: temporarily changed only the post-registration recheck condition to `Option::<AbortReason>::None`, then the focused test failed at the bounded abort-result assertion (`Elapsed`, 5-second budget), as expected. This mutation was restored immediately and is not retained.
- Same focused command after restore — passed, 1 test.
- `mise exec -- rustfmt --check --edition 2021 crates/cheaper-tools/src/resources.rs` — passed.
- Independent review found the outer result channel budget (8 seconds) could expire before two sequential 5-second request budgets and worker completion. Raised it to 13 seconds and added elapsed/allowed-budget phase diagnostics.
- `RESULT_TIMEOUT` is 13 seconds (two 5-second request budgets); the worker has a separate 3-second completion budget, so total post-release ceiling is 16 seconds. The focused test passed after the change (1 passed).
- Follow-up review found that an inner request timeout would report only a static `expect`; it now reports `phase=abort-request` and the 5-second elapsed/allowed budget. The focused test passed after this change (1 passed).
- Inspected final source to confirm no mutation bypass remains. Full workspace verification has not run.

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
