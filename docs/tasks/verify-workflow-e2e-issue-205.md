---
id: "TASK-034ZwWqvlfhc5tBcqkkFUR"
type: "task"
status: "done"
title: "verify workflow discovery+authoring E2E and APM distribution (issue 205)"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
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
    - "docs/plans/verify-workflow-e2e-issue-205.md"
  implemented-by:
    - "docs/impl/ir/verify-workflow-discovery-authoring-e2e-and-apm-distribution-issue-205.md"
  depends-on:
    - "docs/plans/verify-workflow-e2e-issue-205.md"
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
    - "docs/test-specs/workflow-e2e-verification-contract.md"
  references:
    - "docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl"
  defers: []
  deferred-by: []
---

# verify workflow discovery+authoring E2E and APM distribution (issue 205)

## Work

Run the issue #205 verification matrix end-to-end in isolated scratch
environments and record measured evidence:

1. CLI E2E in a scratch workspace + scratch `CHEAPER_CONFIG_DIR` with
   `target/debug/cheaper.exe`: absent-catalog `workflow list` → author
   `file-inspect.json` (tool-only `count_tokens`+`grep`) →
   `workflow interface` pass → catalog entry → `workflow list`
   `status:"ok"` → `workflow run` via `--input <file>` / `--input -`
   returning public outputs only.
2. Negative battery reproducing every message in the L2 skill's error
   table + the #201 catalog two-tier validation (fail-closed manifest
   errors vs per-entry diagnostics) + `--limit` bounds + inline
   `--input` rejection + no named-lookup-by-id.
3. Permission non-regression: `command`-job workflow fails closed at
   run (`POLICY_VIOLATION`, exit 6) while `interface` passes
   statically; read-only commands write no history records; workspace
   `.cheaper/` untouched.
4. L1 evidence inputs verified read-only: `count-tokens`,
   `route check` (exit class 1=warning), `route resolve`.
5. APM distribution: `apm install` in scratch consumers
   (`agent-skills`, `windsurf`, `claude`) with a `path:` dep on
   `packages/apm`; all 7 SKILL.md deploy per target with parseable
   front matter; only the active target's hook bundle lands.
6. Docs consistency: guides vs skill bodies vs `--help` vs
   `catalog.rs` constants; #201-#204 index rows vs front matter
   (already synced on main — verified).
7. Doc chain (discovery/spec/design/plan/task/test-spec/impl-record)
   + experiment log; `doc_status audit` blocking 0, `git diff --check`
   clean, `mise run check`.

No billable model calls; no grant/policy/Route changes; no repo
`.cheaper/` writes; scratch dirs live under the OS temp dir.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/verify-workflow-e2e-issue-205.md`
- `docs/designs/workflow-e2e-verification.md`
- `docs/specs/workflow-e2e-verification-issue-205.md`

## Dependencies

- `target/debug/cheaper.exe` build (`mise exec -- cargo build -p cheaper-cli`)
- `apm` CLI ≥0.32 for the scratch consumers (0.33.0 used)

## Affected Files

- `docs/discovery/workflow-e2e-verification-issue-205.md`
- `docs/specs/workflow-e2e-verification-issue-205.md`
- `docs/designs/workflow-e2e-verification.md`
- `docs/plans/verify-workflow-e2e-issue-205.md`
- `docs/tasks/verify-workflow-e2e-issue-205.md`
- `docs/test-specs/workflow-e2e-verification-contract.md`
- `docs/impl/ir/verify-workflow-discovery-authoring-e2e-and-apm-distribution-issue-205.md`
- `docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl`
- README index rows for the six doc dirs

## Done When

- [x] Issue matrix rows each replayed as a fixture or verified as a
  skill-text invariant, outcomes recorded in the experiment log
- [x] Permission boundary non-regression proven (fail-closed
  `command` job; read-only commands write nothing)
- [x] All 7 skills deploy per APM target with parseable front matter
- [x] `doc_status audit --blocking` = 0; `git diff --check` clean;
  `mise run check` pass or honestly-reported env flake

## Verification

- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` → `"blocking": 0`
- [x] `git diff --check` → no output
- [x] `mise run check` → pass (env flake policy: rerun failed filter alone)
- [x] Experiment log appended with measured outcomes
