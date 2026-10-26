---
id: "IMPL-034MJrFxQn7LGWrgkx4ECq"
type: "impl"
status: "completed"
title: "Wire Route and refetch execution history"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-cli/src/route/run.rs }
      - { type: file, path: crates/cheaper-cli/src/route/logs.rs }
      - { type: file, path: crates/cheaper-cli/src/workflow_run.rs }
      - { type: file, path: crates/cheaper-cli/tests/integration/main.rs }
      - { type: file, path: crates/cheaper-cli/tests/integration/route_history.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/execution-history-route.md"
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

# Wire Route and refetch execution history

## Summary

Implemented best-effort Route and refetch History lifecycles while preserving Route log and Usage ownership. Each actual Route run receives an execution identity after Runnable resolution, and the Workflow gets a verified parent edge to that Route.

## Context

Implements TASK-034MGHBRUesH1Zj3pJiqH7 from the approved Issue 192 plan. The execution identity/Usage ownership TSPEC requires distinct Route/refetch execution IDs and preserves runId/logRef. The correlation TSPEC prohibits inferring parent edges except where Route owns the routed Workflow lifecycle.

## Implementation

### Changes

`route/run.rs` now records a Route start only after `prepare` returns a runnable route, closes returned outcomes with a terminal record, and adds the Route execution identity to the existing Route Usage row. Recognized no-match/no-grant/not-runnable outcomes write a decision-only record. `route/logs.rs` writes an independent refetch start before the log read, terminalizes success and failure, and correlates the existing Usage row with the refetch identity and source `logRef`. `workflow_run.rs` accepts the Route identity only through `RoutedRun` and creates the Workflow identity as its child; existing per-Job identities and correlations remain in their observer. `route_history.rs` covers Route/Workflow parent identity, decision-only rejection, independent refetch IDs, terminal behavior, and read-only resolve.

### Decisions During Implementation

Route IDs are created only after `prepare` confirms Runnable. Invalid/unmatched Route failures write a decision record only when the error carries a recognized finite reason; parse failures remain outside Route execution evidence. Refetch keeps only the established nonempty `logRef` as its source join, with no History traversal or inferred parent relation. History and Usage writes are independent and best-effort.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

Passed: `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_history::` — 7 passed, 0 failed. The registered list contained 6 tests before the persistence-failure case was added; the final run included all 7. Coverage includes unchanged Route output/Usage when History writes fail. Earlier focused attempts exposed and fixed fixture-only issues: an invalid all-zero log ref, workflow `schemaVersion`, and raw Route output parsing. `git diff --check` and the Implementation Record audit passed.

## Risks

The focused Route tests ran on Windows. POSIX process-level behavior and the broad workspace verification contract were not run in this slice. History/Usage persistence remains best-effort; the product result does not depend on write success.

## Follow-ups

Coordinator to perform shared diff review, index synchronization, and commit handling.
