---
id: "IMPL-034MCWkH7NVRe2ZeOyrzXk"
type: "impl"
status: "completed"
title: "Synchronize Node boundary records after issue 188"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
      - type: "file"
        path: "docs/discovery/node-dev-tooling-inventory-issue-182.md"
      - type: "file"
        path: "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
      - type: "file"
        path: "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
      - type: "file"
        path: "docs/tasks/synchronize-node-boundary-docs-after-issue-188.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/synchronize-node-boundary-docs-after-issue-188.md"
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

# Synchronize Node boundary records after issue 188

## Summary

Synchronized the accepted Node boundary policy with issue #188's verified F1
completion. `AGENTS.md`, the parent design, the historical inventory, and the
accepted ADR now distinguish the three former ambient-Node sites from the
remaining repository-tooling Node requirements. The accepted ADR decision,
Node pin, and unrelated T1/T3 gates remain intact.

## Context

Implements Task `TASK-034MCBY6d6KRQNWwa96Qoo` after the cheaper-tools and
workflow tasks passed their Node-free target tests. The parent design and ADR
are updated as factual follow-up records, not new Node toolchain decisions.

## Implementation

### Changes

Updated the parent design's §1.3, §§3.1–3.3, §5, and current risk descriptions.
The design assigns the real bare-name lookup/pin/spawn proof to
`node_task_resolves_via_path`, labels differential absolute-path translation as
output parity only, and records the workflow test binary fixture without a
silent skip. The inventory retains its 2026-10-25 site findings as a dated
snapshot and appends the 2026-10-26 current disposition. The ADR received a
dated follow-up stating F1 is complete while Node remains pinned for repo
tooling and T1/T3 remain outstanding. `AGENTS.md` now states the same current
three-site behavior and preserves the Node removal gate. Canonical spec,
design, plan, and task indexes were regenerated; the IR README was updated by
the IR generator.

### Decisions During Implementation

Kept historical inventory and ADR decision-time claims with explicit dates
instead of rewriting their original evidence. Updated the living design and
AGENTS guidance to current behavior. No `CONTRIBUTING.md` Node-free test
claim needed correction; the narrow search found no stale three-site/fail/skip
statement there.

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

Focused `doc_status audit` results had zero blockers and zero target findings
for the issue188 spec, design, plan, test spec, and tasks, plus the updated
parent design, inventory, and ADR. `audit_impl_record.js --dir docs/impl/ir
--json` reported 192 records and zero findings before this final IR metadata
update; it will be rerun after completion. Canonical index plans were reviewed
and spec/design/plan/task indexes applied with no blocked actions. The stale
claim search found only dated decision-time/history entries and the conditional
F4 description; all current-state docs are consistent. The frozen process and
runtime JSON comparison against base `e5e26284de8a21ac9433e6470811ffd049448853`
exited 0.

## Risks

Inventory counts remain a historical 2026-10-25 snapshot; current state is
documented separately. Node remains necessary for repository scripts,
code-generation, and deployed upstream doc-suite tooling.

## Follow-ups

No Task 3 follow-up. T1 and T3 remain broader Node-toolchain removal gates and
are not claimed complete by issue #188.
