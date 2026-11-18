---
id: "SPEC-034ZwWcCiBMmFi9qIIBUhp"
type: "spec"
status: "implemented"
title: "Workflow E2E and APM distribution verification contract for issue 205"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/205"
    - "https://github.com/scarecrow173/cheaper/issues/200"
  changes:
    added:
      - {"type":"doc","path":"docs/discovery/workflow-e2e-verification-issue-205.md"}
      - {"type":"doc","path":"docs/specs/workflow-e2e-verification-issue-205.md"}
      - {"type":"doc","path":"docs/designs/workflow-e2e-verification.md"}
      - {"type":"doc","path":"docs/plans/verify-workflow-e2e-issue-205.md"}
      - {"type":"doc","path":"docs/tasks/verify-workflow-e2e-issue-205.md"}
      - {"type":"doc","path":"docs/test-specs/workflow-e2e-verification-contract.md"}
      - {"type":"doc","path":"docs/impl/ir/verify-workflow-discovery-authoring-e2e-and-apm-distribution-issue-205.md"}
      - {"type":"doc","path":"docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl"}
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./workflow-catalog-issue-201.md"
    - "./l0-workflow-candidate-trigger-issue-202.md"
    - "./workflow-discover-skill-issue-203.md"
    - "./workflow-create-skill-issue-204.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/workflow-e2e-verification-issue-205.md"
  derived-by:
    - "../designs/workflow-e2e-verification.md"
    - "../test-specs/workflow-e2e-verification-contract.md"
  verifies: []
  verified-by:
    - "../test-specs/workflow-e2e-verification-contract.md"
  references:
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
    - "docs/guides/usage.md"
  defers: []
  deferred-by: []
---

# Workflow E2E and APM distribution verification contract for issue 205

## What Should Be Built

A cross-cutting verification — and its recorded evidence — for epic
#200's workflow discovery+authoring path. Deliverables are (a) an
executed scenario matrix over the real shipped binary and APM package,
covering the positive path (catalog list → author → `interface`
validate → register → list → `workflow run`) and the negative /
permission / invalid cases the issue enumerates, (b) APM consumer
verification that `packages/apm` deploys all 7 skills plus the active
target's hook bundle, and (c) a docs-consistency pass proving guides,
skill bodies, CLI help, and doc indexes agree — plus the issue's own
doc chain and an evidence record with measured outputs.

## Why Now

#201-#204 each landed with unit/integration coverage of their own
slice; nothing yet proves the assembled conduit a parent agent would
actually exercise — L0 trigger conditions as shipped skill text, L1's
read-only inspection commands (`workflow list`, `workflow interface`,
`route check`, `route resolve`, `count-tokens`, `usage`), L2's
authoring+validation+catalog-registration loop, and the unchanged
explicit-path execution boundary. Epic #200 closes on this issue, so
the evidence must be measured, not asserted.

## Users and Value

Maintainers get a replayable verification record (experiment log +
impl record) instead of a claim; parent-agent consumers get confidence
that the distributed skills and the documented contracts match the
shipped CLI. Users keep the approval boundary — the verification
itself proves nothing is persisted, granted, or executed without
explicit action.

## Scope

### In Scope

- Positive path E2E with the real debug binary in a scratch workspace
  + scratch `CHEAPER_CONFIG_DIR`: absent-catalog `workflow list`,
  authored Workflow JSON v1 → `workflow interface` pass, catalog entry
  registration visible as `status:"ok"`, tool-only `workflow run`
  (`grep`/`count_tokens` — no grants, no model calls, no side
  effects) returning only public outputs.
- Negative/invalid cases from the issue matrix reproducible as CLI
  fixtures: malformed JSON, invalid v1 shape, unknown tool/worker,
  binding violations (`$input`/`$from`/pointer/wrapper/cycle),
  catalog duplicate/missing/out-of-workspace/malformed entries,
  corrupted manifest fail-closed, `--limit` bounds, `--input` inline
  JSON rejection, no named-lookup by catalog id.
- Permission non-regression: `command`-job workflow fails at run with
  `POLICY_VIOLATION` absent a grant; `interface` still passes
  statically; read-only commands write no history records and touch no
  workspace `.cheaper/` state.
- APM distribution: `apm install` in scratch consumers for
  `agent-skills` and a harness target; all 7 SKILL.md files deploy
  with parseable front matter; per-target hook bundles land only on
  their own target.
- Docs consistency: guides' claims (7 skills, L0→L1→L2 chain, catalog
  contract, `--input` contract, exit codes, `--limit` bounds, hook
  deploy paths) vs shipped skill bodies, `catalog.rs`, and CLI
  `--help`; doc index rows vs front matter for the #201-#204 chain.
- The issue's doc chain (discovery/spec/design/plan/task/test-spec/
  impl-record) + experiment log with measured outputs.

### Out of Scope

- Billable model calls and worker-job execution (covered by engine
  tests and the `CHEAPER_E2E_OLLAMA` gate).
- Grant/policy/Route modifications — the `command` job is verified to
  fail closed, not to run.
- Real consumer-agent UI auto-firing (per-target runtime gap,
  documented limitation identical to the issue's own caveat).
- New CLI commands, skill text changes, or runtime behavior changes —
  this issue verifies; it does not extend the path.
- #142 Route auto-rewrite changes (issue non-goal).

## Requirements

- Every scenario in the issue's matrix is either replayed as a CLI/APM
  fixture with a recorded outcome, or verified as a skill-text
  invariant with the shipped SKILL.md quoted — unverifiable items are
  recorded as unverified, never asserted.
- Evidence is measured: envelopes, exit codes, deploy trees, and byte
  counts are captured into `docs/impl/exp/*.jsonl`.
- No persistent change outside the repo working tree: scratch
  workspace and `CHEAPER_CONFIG_DIR` live under the OS temp dir;
  repo `.cheaper/` is never written.
- `mise run check` and `doc_status` audit must pass; environment
  flakes are rerun in isolation and reported honestly.

## Acceptance Criteria

- [x] Positive/negative/permission/invalid cases from the issue matrix
  replay as fixtures — recorded in the experiment log.
- [x] Execution path reuses `workflow interface`/`workflow run` and
  compiler/policy unchanged; the catalog confers no special
  permissions (`command` job fails closed without grant).
- [x] Run output carries public outputs only; `jobs[]` metadata
  carries `{id,kind,status,meta}` — no private DAG, prompts, or large
  intermediate values.
- [x] Read-only verification commands (`workflow list`,
  `workflow interface`, `route check`, `route resolve`,
  `count-tokens`) write no history records; L0 is skill text only —
  no hook, no per-turn CLI/LLM cost.
- [x] `apm install` deploys all 7 skills per target; hook bundles
  deploy only on matching targets; SKILL.md front matter parses.
- [x] Guides/skill bodies/CLI help/doc indexes agree; index rows for
  the #201-#204 chain verified synced (already fixed on main).
- [x] `mise run check` + `doc_status audit --blocking` pass; env
  flakes rerun in isolation.

## Deferred Design Concerns

- Real per-target agent auto-firing E2E — requires live consumer
  runtimes; revisit only if a consumer reports a firing regression
  the documented manual fallback cannot cover.

## Sources and Relations

Issue #205 (parent epic #200; upstream issues #201-#204). Discovery at
`docs/discovery/workflow-e2e-verification-issue-205.md`; the four
landed issues' specs listed in `related`; verified per
`docs/test-specs/workflow-e2e-verification-contract.md`.
