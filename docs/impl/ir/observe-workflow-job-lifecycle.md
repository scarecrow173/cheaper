---
id: "IMPL-034MHsRcZJcZ9DZ3fO6rvi"
type: "impl"
status: "completed"
title: "Observe Workflow Job lifecycle"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-cli/src/workflow_run.rs }
      - { type: file, path: crates/cheaper-workflow/src/engine.rs }
      - { type: file, path: crates/cheaper-workflow/tests/engine.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/workflow-job-history.md"
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

# Observe Workflow Job lifecycle

## Summary

Starting task `TASK-034MGHCIrNt0wvnqKvOLTP` from the approved implementation plan. The shared History API is available from the completed core task; this slice adds neutral engine observation and the CLI adapter.

## Context

Follow the approved plan and Workflow Job lifecycle TSPEC. The engine must not depend on CLI History types; observer failures must not affect workflow results.

## Implementation

### Changes

Added a History-neutral engine observer with Started and terminal callbacks at actual dispatch/finish boundaries. CLI adapter records Workflow parent, distinct Job IDs, Job worker identity/metrics, and decision-only skipped/blocked/admission failures. Observer errors are isolated from workflow execution.

### Decisions During Implementation

Use a History-neutral observer interface at the engine boundary and adapt it in the CLI owner.

## Related Experiments

### Adopted

No experiment log; the accepted design specified the observer boundary.

### Rejected

None.

## Validation

Initial guarded list confirmed 5 `job_history::` tests; focused engine suite passed 5/5 after adding `start_paused = true` to the timeout fixture. CLI adapter guarded list confirmed 2 tests; focused adapter suite passed 2/2. Scoped rustfmt and `git diff --check` passed.

## Risks

Verified on Windows. Full canonical workspace check remains for final integration.

## Follow-ups

Task complete. Final independent branch review will cover cross-surface behavior and canonical verification.
