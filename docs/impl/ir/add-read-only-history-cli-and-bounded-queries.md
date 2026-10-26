---
id: "IMPL-034MHsQwuzT8Ysc0cdXxZW"
type: "impl"
status: "in-progress"
title: "Add read-only History CLI and bounded queries"
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
    - "docs/tasks/execution-history-query-cli.md"
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

# Add read-only History CLI and bounded queries

## Summary

Starting task `TASK-034MGHz9sAMkZkvZW0WFgZ` from the approved implementation plan. The bounded History read API is provided by the completed core task; this slice adds only the CLI surface and its contract tests.

## Context

Follow the approved plan, the execution-history design, and bounded query TSPEC. Keep list/show/trace read-only, return no cursor for incomplete scans, and fail filtered verification if no named tests are registered.

## Implementation

### Changes

Pending implementation. The existing core query API is consumed without editing its owner files.

### Decisions During Implementation

CLI task ownership excludes `src/history/query.rs`; that file is owned by the completed core task. Integration registration is owned by this task.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

Task graph confirms the History CLI task is dependency-ready after the completed core task. Source verification is pending.

## Risks

CLI grammar, formatting, and integration verification remain pending.

## Follow-ups

After focused CLI tests pass, close this task and proceed with other dependency-ready owners.
