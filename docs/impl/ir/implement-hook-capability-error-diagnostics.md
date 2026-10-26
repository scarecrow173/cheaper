---
id: "IMPL-034MyAvIcswWUdbS5yKM0n"
type: "impl"
status: "in-progress"
title: "Implement hook capability error diagnostics"
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
    - "docs/tasks/separate-hook-capability-error-diagnostics.md"
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

# Implement hook capability error diagnostics

## Summary

In progress. This record is open before code changes for the hook diagnostics
task. Planned scope is truthful, bounded diagnostics that keep `Err(io::Error)`
separate from `Ok(Output)`; no implementation result is claimed yet.

## Context

Follows `docs/tasks/separate-hook-capability-error-diagnostics.md`, the
approved #187 plan, its hook diagnostic test-spec, and the accepted
deterministic test-boundary ADR.

## Implementation

### Changes

No code changes yet. Record the actual diagnostic fields, path provenance, and
regression coverage as they are implemented.

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
