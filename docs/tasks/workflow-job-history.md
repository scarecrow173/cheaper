---
id: "TASK-034MGHCIrNt0wvnqKvOLTP"
type: "task"
status: "done"
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
    - "TSPEC-034MGCzUAhlbOn8zU2Mnal"
  references: []
  defers: []
  deferred-by: []
---

# Observe Workflow Job lifecycle

## Work

Add a History-neutral Workflow engine observer and CLI adapter that records authoritative Job transitions with distinct IDs and the actual Workflow parent. Keep CLI History types out of the workflow engine and isolate observer errors from execution.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-execution-history-for-issue-192.md`

## Dependencies

- `docs/tasks/execution-history-core.md` (`TASK-034MGH8FHwescRINQne4sz`)

## Affected Files

- `crates/cheaper-cli/src/workflow_run.rs`, `crates/cheaper-workflow/src/engine.rs`
- `crates/cheaper-workflow/tests/engine.rs`

## Done When

- [ ] Job lifecycle evidence covers success/failure/cancel/timeout/skipped/blocked using engine transitions.
- [ ] Each Job gets a distinct ID linked to its actual parent Workflow ID; observer failure does not change outcomes.
- [ ] Internal Job Worker work is not mislabeled as standalone `kind:run`.

## Verification

- [ ] `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -p cheaper-workflow --test engine job_history:: -- --list` lists named tests; then the same command without `-- --list` exits 0.
