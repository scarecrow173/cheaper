---
id: "IMPL-034MHsRcZJcZ9DZ3fO6rvi"
type: "impl"
status: "completed"
title: "Observe Workflow Job lifecycle"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-cli/src/workflow_run.rs }
      - { type: file, path: crates/cheaper-workflow/src/engine.rs }
      - { type: file, path: crates/cheaper-workflow/tests/engine.rs }
      - { type: file, path: crates/cheaper-cli/tests/workflow.rs }
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

Added a History-neutral engine observer with Started and terminal callbacks at actual dispatch/finish boundaries. CLI adapter records Workflow parent, distinct Job IDs, Job worker identity/metrics, and decision-only skipped/blocked/admission failures. The engine also carries only resolved provider/model identity observed by a Job executor, including typed metadata retained across output-schema failures; pre-resolution failures remain identity-free. Standalone Workflow History now starts after workspace/config resolution but before grant/policy/workflow loading and compilation; it terminalizes every returned result while preserving command outcome. The routed Workflow child lifecycle remains observer-owned. Observer persistence errors are isolated from workflow execution.

### Decisions During Implementation

Use a History-neutral observer interface at the engine boundary and adapt it in the CLI owner.

## Related Experiments

### Adopted

No experiment log; the accepted design specified the observer boundary.

### Rejected

None.

## Validation

Final verification: `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib workflow_run::history_tests` passed 3/3; the same features with `--test workflow` passed 19/19. `mise exec -- cargo test -p cheaper-workflow --test engine job_history::` passed 7/7, including the output-schema failure identity regression; `mise exec -- cargo test -p cheaper-workflow --lib resolved_model_identity_tests` passed 2/2. Scoped rustfmt and `git diff --check` passed. Earlier smaller runs remain historical diagnostics, not the final counts.

## Risks

Verified on Windows. Full canonical workspace check and Ubuntu/POSIX CI remain final integration gates.

## Follow-ups

Task complete. Final independent branch review will cover cross-surface behavior and canonical verification.
