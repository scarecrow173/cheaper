---
id: "TASK-034MGHAM3LVWSPo5RwaMkv"
type: "task"
status: "todo"
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
- `crates/cheaper-tools/src/invocation.rs`, `registry.rs`, `tools/exec/mod.rs`, `tools/proc/`, `command/`, `process/`

## Done When

- [ ] Only explicit carrier values survive the managed `env_clear` boundary; ambient environment is not inherited.
- [ ] Correlation is marked unverified and never treated as causation or authorization.
- [ ] History projection preserves official typed IDs and omits unsafe identities without changing catalog/model behavior.

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools proc::tests` exits 0 with tests for carrier propagation and existing child environment contracts.
