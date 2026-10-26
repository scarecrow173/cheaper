---
id: "IMPL-034MyAvYhFVOfkiKM4JOO3"
type: "impl"
status: "in-progress"
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

In progress. This record is open before code changes for the Windows process
oracle task. Planned scope is a same-handle, bounded termination check and
separate watchdog cleanup; no implementation result is claimed yet.

## Context

Follows `docs/tasks/use-stable-windows-process-handle-termination-oracle.md`,
the approved #187 plan, its process-handle test-spec, and the accepted
deterministic test-boundary ADR.

## Implementation

### Changes

No code changes yet. Record the Win32 handle helper, wait budgets, cleanup
result, and RAII ownership as implementation proceeds.

### Decisions During Implementation

<!-- Record important implementation-time decisions. -->

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

<!-- Record the checks that were run. -->

## Risks

<!-- Record remaining risks. -->

## Follow-ups

<!-- Record follow-up tasks or next actions. -->
