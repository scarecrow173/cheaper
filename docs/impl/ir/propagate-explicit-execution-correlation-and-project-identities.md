---
id: "IMPL-034MHTsREMda1OP0dJWNbt"
type: "impl"
status: "completed"
title: "Propagate explicit execution correlation and project identities"
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
    - "docs/tasks/execution-history-carrier-catalog.md"
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

# Propagate explicit execution correlation and project identities

## Summary

Starting task `TASK-034MGHAM3LVWSPo5RwaMkv` from the approved implementation plan. The History carrier types and identity projection API are provided by the completed core slice.

## Context

Follow the approved plan and correlation/privacy TSPEC. Propagate only explicit optional correlation values through managed runtime/tool boundaries; project provider/model identity for History without changing catalog acceptance or resolution.

## Implementation

### Changes

- Added a plain typed `ExecutionCorrelationCarrier` through runtime and tool options; managed process spawning sets only explicit correlation values after `env_clear`.
- Added History-only provider/model identity projection using official resolved IDs, omitting unsafe values without changing catalog acceptance or resolution.
- Shared Workflow run state maps already-issued Job IDs to child execution identities. The observer records the mapping at Started; worker/tool executors reuse it without holding a lock across async work. Missing mappings yield no carrier.
- Preserved `None` defaults at standalone Worker and pure Route resolve call sites; their authoritative identity wiring belongs to subsequent tasks.

### Decisions During Implementation

Keep correlation unverified; do not extend carrier injection to external/user commands or treat identifiers as authorization. Reuse the existing Workflow Job ID only; do not fabricate an ID when observer state is missing.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

Focused verification passed: `cheaper-tools proc::tests` (11), `cheaper-runtime catalog::` (22 unit + 12 catalog contract), and CLI Workflow Job history test (1). Sensitivity RED checks were temporary post-implementation mutations and were restored before the green runs. `git diff --check` passed.

## Risks

Standalone Worker/Route callers still need to supply authoritative execution identities in their own plan tasks.

## Follow-ups

Continue with standalone Worker/Usage and Route lifecycle owners.
