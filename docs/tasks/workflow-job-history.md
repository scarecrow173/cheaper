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
- `crates/cheaper-workflow/tests/engine.rs`, `crates/cheaper-cli/tests/workflow.rs`

## Done When

- [x] Job lifecycle evidence covers success/failure/cancel/timeout/skipped/blocked using engine transitions.
- [x] Each Job gets a distinct ID linked to its actual parent Workflow ID; observer failure does not change outcomes.
- [x] Internal Job Worker work is not mislabeled as standalone `kind:run`; resolved provider/model identity is copied only from observed executor metadata.

## Verification

- [x] `mise exec -- cargo test -p cheaper-workflow --test engine job_history::` exits 0 (7 passed), including output-schema failure identity retention.
- [x] `mise exec -- cargo test -p cheaper-workflow --lib resolved_model_identity_tests` exits 0 (2 passed).
- [x] CLI adapter unit tests passed 3/3 and `--test workflow` passed 19/19 with dev-seam features.
- [x] Standalone Workflow missing/malformed definition tests verify one started/terminal pair and preserved command failure.
