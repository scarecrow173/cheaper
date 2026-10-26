---
id: "TASK-034MGHAM3LVWSPo5RwaMkv"
type: "task"
status: "done"
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
    - "docs/plans/implement-execution-history-for-issue-192.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-execution-history-for-issue-192.md"
    - "TASK-034MGH8FHwescRINQne4sz"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "TSPEC-034MGCFe4bqwpXhKCEYgRA"
  references: []
  defers: []
  deferred-by: []
---

# Propagate explicit execution correlation and project identities

## Work

Propagate explicit optional correlation values through runtime and tools options to managed process environments after `env_clear`; implement History-only provider/model identity projection that omits unsafe values without changing catalog acceptance or resolution.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-execution-history-for-issue-192.md`

## Dependencies

- `docs/tasks/execution-history-core.md` (`TASK-034MGH8FHwescRINQne4sz`)

## Affected Files

- `crates/cheaper-runtime/src/run/mod.rs`, `crates/cheaper-runtime/src/catalog/`
- `crates/cheaper-runtime/src/run/boundary.rs` and `crates/cheaper-cli/src/workflow_run.rs` for neutral carrier plumbing and reuse of the already-issued Job identity; `run.rs` uses a temporary `None` default until the Worker/Usage owner supplies its identity.
- `crates/cheaper-cli/src/route/resolve.rs` receives only the required `execution_correlation: None` options default; preserve its pure no-write behavior.
- `crates/cheaper-tools/src/invocation.rs`, `registry.rs`, `tools/exec/mod.rs`, `tools/proc/`, `command/`, `process/`
- Existing `RunWorkerDeps`, `ToolRuntimeOptions`, and `ExecToolOptions` test literal sites under `crates/cheaper-runtime/tests/`, `crates/cheaper-tools/src/`, `crates/cheaper-tools/tests/`, and `crates/cheaper-workflow/tests/`; add only `None` defaults to preserve existing behavior.
- `crates/cheaper-rig-adapter/tests/adapter.rs`; add the optional carrier's `None` default to preserve the existing adapter fixture behavior.

## Done When

- [x] Only explicit carrier values survive the managed `env_clear` boundary; ambient environment is not inherited.
- [x] Correlation is marked unverified and never treated as causation or authorization.
- [x] Existing Workflow Job identity is reused as the explicit child carrier after its Started transition; no Job/Workflow identity is fabricated if the map has no entry.
- [x] Existing carrier and catalog callers compile with default `None`; the later standalone Worker/Route lifecycle tasks replace their defaults with authoritative current identities.
- [x] History projection preserves official typed IDs and omits unsafe identities without changing catalog/model behavior.

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools proc::tests` exited 0: 11 passed. A temporary mutation disabling carrier env injection produced 2 expected failures, then was restored.
- [x] `mise exec -- cargo test -p cheaper-runtime catalog::` exited 0: 22 unit and 12 catalog contract tests passed. A temporary mutation disabling History identity projection caused the official-ID assertion to fail, then was restored.
- [x] `mise exec -- cargo test -p cheaper-cli --lib workflow_job_history` exited 0: 1 passed, verifying unique child IDs and explicit Job parent correlation.
- [x] `git diff --check` exited 0. Standalone Worker and Route execution identity wiring remains in their separate plan tasks.
