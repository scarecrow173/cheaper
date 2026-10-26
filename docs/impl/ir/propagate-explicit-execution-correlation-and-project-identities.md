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
    added:
      - { type: file, path: crates/cheaper-runtime/src/catalog/history_identity.rs }
    modified:
      - { type: file, path: crates/cheaper-cli/src/route/resolve.rs }
      - { type: file, path: crates/cheaper-cli/src/workflow_run.rs }
      - { type: file, path: crates/cheaper-runtime/src/catalog/mod.rs }
      - { type: file, path: crates/cheaper-runtime/src/run/boundary.rs }
      - { type: file, path: crates/cheaper-runtime/src/run/mod.rs }
      - { type: file, path: crates/cheaper-runtime/tests/ollama_e2e.rs }
      - { type: file, path: crates/cheaper-runtime/tests/run_dev_seam.rs }
      - { type: file, path: crates/cheaper-runtime/tests/run_fixture_test.rs }
      - { type: file, path: crates/cheaper-tools/src/invocation.rs }
      - { type: file, path: crates/cheaper-tools/src/registry.rs }
      - { type: file, path: crates/cheaper-tools/src/tools/command/mod.rs }
      - { type: file, path: crates/cheaper-tools/src/tools/exec/mod.rs }
      - { type: file, path: crates/cheaper-tools/src/tools/proc/mod.rs }
      - { type: file, path: crates/cheaper-tools/src/tools/proc/spawn.rs }
      - { type: file, path: crates/cheaper-tools/src/tools/proc/tests.rs }
      - { type: file, path: crates/cheaper-tools/src/tools/process/mod.rs }
      - { type: file, path: crates/cheaper-tools/src/tools/process/tests.rs }
      - { type: file, path: crates/cheaper-tools/tests/characterization.rs }
      - { type: file, path: crates/cheaper-tools/tests/differential.rs }
      - { type: file, path: crates/cheaper-tools/tests/proc_command.rs }
      - { type: file, path: crates/cheaper-tools/tests/proc_degrade.rs }
      - { type: file, path: crates/cheaper-tools/tests/proc_invocation.rs }
      - { type: file, path: crates/cheaper-tools/tests/proc_process.rs }
      - { type: file, path: crates/cheaper-tools/tests/proc_runtime.rs }
      - { type: file, path: crates/cheaper-workflow/tests/filesystem_contracts/artifacts.rs }
      - { type: file, path: crates/cheaper-workflow/tests/process.rs }
      - { type: file, path: crates/cheaper-rig-adapter/tests/adapter.rs }
      - { type: file, path: docs/tasks/execution-history-carrier-catalog.md }
      - { type: file, path: docs/impl/ir/README.md }
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
