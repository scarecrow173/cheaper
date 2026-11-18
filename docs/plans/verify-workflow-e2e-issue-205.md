---
id: "PLAN-034ZwWjd2amHIXlsAuiHtg"
type: "plan"
status: "completed"
title: "Verify workflow discovery+authoring path end-to-end for issue 205"
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
    - "docs/specs/workflow-e2e-verification-issue-205.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-e2e-verification-issue-205.md"
    - "docs/designs/workflow-e2e-verification.md"
  derived-by:
    - "docs/tasks/verify-workflow-e2e-issue-205.md"
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-e2e-verification-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Verify workflow discovery+authoring path end-to-end for issue 205 Implementation Plan

## Goal

Execute the #205 verification matrix against the shipped binary and
APM package in isolated scratch environments, record measured
evidence, and ship the issue's doc chain — no runtime or skill-body
changes.

## Scope

One verification stream: CLI E2E → APM distribution → docs
consistency → doc chain + evidence. No code changes land; deliverables
are `docs/` artifacts only.

## Upstream Documents

- `docs/specs/workflow-e2e-verification-issue-205.md`

## Design Inputs

- `docs/designs/workflow-e2e-verification.md`

## File Map

- `docs/discovery/workflow-e2e-verification-issue-205.md` — scope analysis
- `docs/specs/workflow-e2e-verification-issue-205.md` — verification contract
- `docs/designs/workflow-e2e-verification.md` — fixture/isolation approach
- `docs/plans/verify-workflow-e2e-issue-205.md` — this plan
- `docs/tasks/verify-workflow-e2e-issue-205.md` — task
- `docs/test-specs/workflow-e2e-verification-contract.md` — Given/When contract
- `docs/impl/ir/verify-workflow-discovery-authoring-e2e-and-apm-distribution-issue-205.md` — impl record
- `docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl` — measured evidence
- `$TEMP/cheaper-e2e-205/` — scratch workspace + config dir + APM consumers (uncommitted)

## Implementation Sequence

- [x] Build `target/debug/cheaper.exe` (`mise exec -- cargo build -p cheaper-cli`)
- [x] Scratch workspace + `CHEAPER_CONFIG_DIR`; absent-catalog `workflow list`
- [x] Author `file-inspect.json`; `workflow interface` pass; register in `catalog.json`; `workflow list` shows `status:"ok"` (json + text)
- [x] `workflow run` via `--input <file>` and `--input -`; confirm public-outputs-only result and `jobs[]` metadata shape
- [x] Negative battery: malformed JSON, v1 shape, unknown tool/worker, `$input`/`$from`/wrapper/pointer/cycle violations
- [x] Catalog battery: duplicate/missing/out-of-workspace/malformed entries, corrupted manifest, field caps, `--limit` bounds
- [x] Permission probes: `command` job fails closed; `workflow run <id>` no named lookup; inline `--input` rejected; read-only commands write no history
- [x] L1 inputs: `count-tokens`, `route check`, `route resolve` verified read-only
- [x] APM consumers (`agent-skills`, `windsurf`, `claude`): `apm install`, deployed trees, SKILL.md front matter, hook bundles
- [x] Docs consistency pass + index/front-matter scan
- [x] Doc chain + experiment log; `doc_status audit`, `git diff --check`, `mise run check`; commits

## Dependencies and Risks

- Windows env flakes (proc/timing tests under parallel load) — rerun
  failed filters in isolation and report honestly.
- G: drive placeholder hydration can transiently hide files —
  re-stat before claiming missing content.

## Task Breakdown

- `docs/tasks/verify-workflow-e2e-issue-205.md` (single task —
  verification is one continuous pass)

## Verification

- [x] `node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` → blocking 0
- [x] `git diff --check` clean
- [x] `mise run check` full pass (flakes rerun individually)

## Open Questions / Gaps

- Per-target runtime auto-firing unverifiable in this environment —
  recorded as a documented limitation, not a failure.

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution (single-agent verification pass)
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- Evidence lives in `docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl`;
  reviewer focus: do the recorded outcomes match the issue matrix rows,
  and are unverifiable items honestly labeled.
