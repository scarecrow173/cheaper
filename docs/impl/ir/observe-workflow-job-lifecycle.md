---
id: "IMPL-034MHsRcZJcZ9DZ3fO6rvi"
type: "impl"
status: "in-progress"
title: "Observe Workflow Job lifecycle"
created: "2026-10-27"
updated: "2026-10-27"
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

Pending implementation. No source changes are recorded at task start.

### Decisions During Implementation

Use a History-neutral observer interface at the engine boundary and adapt it in the CLI owner.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

Task graph confirms the Workflow lifecycle task is dependency-ready after the completed core task. Source verification is pending.

## Risks

Lifecycle transition wiring and focused verification remain pending.

## Follow-ups

After named lifecycle tests pass, close this task and proceed with remaining dependency-ready owners.
