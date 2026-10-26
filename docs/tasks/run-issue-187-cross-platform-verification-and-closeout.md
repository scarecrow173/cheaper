---
id: "TASK-034MytbiiYw7vec0nRA4xx"
type: "task"
status: "done"
title: "Run issue 187 cross-platform verification and closeout"
created: "2026-10-26"
updated: "2026-10-26"
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
    - "docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md"
    - "docs/tasks/synchronize-resources-abort-window-regression.md"
    - "docs/tasks/separate-hook-capability-error-diagnostics.md"
    - "docs/tasks/use-stable-windows-process-handle-termination-oracle.md"
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
    - "docs/test-specs/resources-abort-window-phase-synchronization.md"
    - "docs/test-specs/hook-capability-diagnostic-error-shape.md"
    - "docs/test-specs/windows-process-handle-termination-oracle.md"
  references: []
  defers: []
  deferred-by: []
---

# Run issue 187 cross-platform verification and closeout

## Work

After the three code tasks are done, review the combined change and the
remaining evidence for resources, hook `ETXTBSY`, Windows liveness, and the
existing #157 route disposition. Run and record the canonical POSIX and
repeated Windows verification gates. Create the per-task Implementation Record
for this task before its first action and update the #187 discovery and
implementation records with exact commands, host identities, outcomes, and
supported dispositions. Do not report non-recurrence as root-cause resolution.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/stabilize-issue-187-posix-suite-residual-flakes.md`
- `docs/specs/stabilize-posix-suite-residual-flakes-issue-187.md`
- `docs/designs/issue-187-posix-suite-residual-flake-stabilization.md`
- `docs/adr/prefer-deterministic-test-boundary-diagnosis-for-residual-suite-flakes.md`

## Dependencies

- Depends on the resources, hook-diagnostics, and Windows-process implementation tasks; all three must be done.

## Affected Files

- `docs/discovery/posix-suite-residual-flakes-issue-187.md`
- `docs/impl/ir/run-issue-187-cross-platform-verification-and-closeout.md`
- `docs/impl/ir/implement-stable-windows-process-handle-termination-oracle.md`

## Done When

- [x] Current route test remains on the #157 contract; no additional #187 route code is introduced.
- [x] Bounded reproduction attempts and exact unresolved causes/dispositions are recorded; independent review finds no confirmed blocking P0/P1.
- [x] Provisioned Ubuntu POSIX run passes `mise install`, `mise run setup`, and `mise run check`, with host/command/results recorded.
- [x] Windows run passes `mise install` followed by `mise run check` three consecutive times, with each run recorded; run 1's numeric exit code was not retained, while its full pipeline completed without an error marker.
- [x] Per-task IRs are complete and the focused implementation-record audit reports no blocking finding.
- [x] Graph followup route and exit audit reach terminal completion after required document lineage is complete.

## Verification

- [x] Ubuntu workflow `40314463285`, code SHA `660cf138541bb77c0318885ad9b50ffb2dbe7dde`: `mise install`, `mise run setup`, and `mise run check` succeeded.
- [x] Windows: three sequential `mise run check` runs on SHA `16f922a994afe8c41205bdf40eb02759f0f4d670`; runs 2 and 3 have explicit exit 0, run 1's numeric exit was not retained despite full successful pipeline output. The intervening Unix-only fixture change is not compiled on Windows.
- [x] Focused implementation-record audit reports no findings; full doc-status audit reports zero blocking findings (existing non-blocking global warnings remain).
- [x] Graph route `followup-triage-terminal` reaches `exit-audit`; `exit-audit-to-complete` returns no blockers after the all-document audit.
