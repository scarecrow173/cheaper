---
id: "TASK-034MytbiiYw7vec0nRA4xx"
type: "task"
status: "in-progress"
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
- `docs/impl/ir/stabilize-issue-187-cross-platform-verification-and-closeout.md`
- `docs/impl/ir/` one record per upstream implementation task

## Done When

- [ ] Current route test remains on the #157 contract; no additional #187 route code is introduced.
- [ ] Bounded reproduction attempts and exact unresolved causes/dispositions are recorded; independent review finds no confirmed blocking P0/P1.
- [ ] Provisioned Ubuntu POSIX run passes `mise install`, `mise run setup`, and `mise run check`, with host/command/results recorded.
- [ ] Windows run passes `mise install` followed by `mise run check` three consecutive times, with each run recorded.
- [ ] Per-task IRs are complete and `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` reports no blocking finding for these records.
- [ ] Focused document audit and graph exit audit are eligible for terminal routing; no unclassified follow-up blocker remains.

## Verification

- [ ] Ubuntu host: `mise install` exits 0; `mise run setup` exits 0; `mise run check` exits 0.
- [ ] Windows host: `mise install` exits 0 and three sequential `mise run check` runs all exit 0.
- [ ] `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json` exits 0 with no blocking finding for the #187 implementation records.
- [ ] `node .agents/skills/doc-status/scripts/doc_status.js audit --type all --json` exits 0 with no blocking #187 document finding.
