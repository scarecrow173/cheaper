---
id: "PLAN-034AR2f7p2NSUDtUhSuLp6"
type: "plan"
status: "approved"
title: "Implement workflow orchestration benchmarks"
created: "2026-10-05"
updated: "2026-10-05"
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
    - "docs/specs/workflow-orchestration-benchmark.md"
  implemented-by:
    - "docs/tasks/build-seeded-workflow-benchmark-fixtures-and-runner.md"
    - "docs/tasks/aggregate-benchmark-evidence-and-publish-workflow-usage-guidance.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-orchestration-benchmark.md"
    - "docs/designs/workflow-orchestration-benchmark.md"
    - "docs/adr/benchmark-harness-placement-and-accounting.md"
  verified-by:
    - "docs/test-specs/workflow-benchmark-scenario-determinism.md"
    - "docs/test-specs/workflow-benchmark-accounting-integrity.md"
  derived-by:
    - "docs/tasks/build-seeded-workflow-benchmark-fixtures-and-runner.md"
    - "docs/tasks/aggregate-benchmark-evidence-and-publish-workflow-usage-guidance.md"
  verifies: []
  references: []
  defers: []
  deferred-by: []
---

# Implement workflow orchestration benchmarks Implementation Plan

## Goal

Build an isolated, seeded CLI benchmark harness, structured result aggregation, and usage guide proving workflow net loss and net benefit cases without inventing parent-agent savings.

## Scope

One implementation stream: fixtures, CLI runner, aggregation, and guidance share one versioned result contract and need coordinated validation. Keep all new runtime-facing files under root `benchmarks/` and documentation under `docs/guides/`; no runtime/API/configuration changes.

## Upstream Documents

- `docs/specs/workflow-orchestration-benchmark.md`
- `docs/adr/benchmark-harness-placement-and-accounting.md`

## Design Inputs

- `docs/designs/workflow-orchestration-benchmark.md`

## File Map

- `benchmarks/run.mjs` - spawn built CLI, enforce fake-stream environment, collect repetitions, and write scenario results.
- `benchmarks/scenarios/*.json` - seeded equivalent direct, dynamic, saved, and interface-discovery scenarios, including guaranteed gain/loss fixture pairs and scenario 1's Node direct-tool baseline.
- `benchmarks/lib/*.mjs` - result schema validation, aggregation, timing/environment capture, and usage.jsonl separation.
- `benchmarks/results/*.json` - versioned evidence outputs with raw observations and aggregate dimensions.
- `benchmarks/README.md` - copy-paste rerun contract, `pnpm build && node benchmarks/run.mjs`.
- `docs/guides/workflow-usage.md` - evidence-based workflow/direct/saved/interface selection guidance.

## Implementation Sequence

- [ ] Define fixture pairs and versioned result schema; guarantee token/byte net-loss and net-benefit cases.
- [ ] Implement CLI-spawning runner with fake stream, warmups, 15 measured runs, environment capture, structured artifacts, and scenario 1 Node baseline; add a single fake `cheaper run` comparator only if it needs no new harness machinery.
- [ ] Implement aggregation and guide from actual fixture results, preserving workflow envelope versus direct usage.jsonl boundaries.
- [ ] Add durable report, rerun instructions, and evidence-based guide; validate cross-references and representative committed result.

Dependency graph: fixture schema → runner → aggregation → guide/report. Each later step consumes the prior step's result contract; implementation remains one stream.

## Dependencies and Risks

- Requires built CLI from `pnpm build`; runner path is `node benchmarks/run.mjs`.
- Wall time varies by machine; capture OS/arch, Node, CPU, revision, repetitions, median and p95; use deterministic tokens/bytes as primary regression signal.
- Fixture schema drift can invalidate comparisons; version schema and fail closed on incomplete/mismatched observations.
- Parent usage unavailable unless separately observed; never estimate token savings.

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Result artifact overwrite loses prior evidence | low | medium | use immutable run IDs and never overwrite existing result paths |
| CLI envelope/schema changes break harness | medium | medium | validate `schemaVersion`, fail closed, and rerun after CLI changes |
| Optional provider or environment unavailable | low | low | use fake stream and local seeded fixtures; no external service required |
| Change breaks public workflow API | low | low | no runtime/API edits are in scope; run CLI package checks if that boundary changes |

Rollback: benchmark artifacts and docs are additive and can be reverted as a unit; no database, public API, or runtime migration is planned. Preserve prior result IDs when revising fixture schema.

## Task Breakdown

- Task 1: fixture/result schema and deterministic CLI runner; establishes the common result contract.
- Task 2: aggregation, representative evidence artifact, rerun docs, and usage guide; depends on Task 1 output.

## Verification

- [ ] `mise exec -- pnpm build` succeeds before running harness.
- [ ] `mise exec -- node benchmarks/run.mjs` emits versioned results with two warmups and 15 measured repetitions, median/p95, and environment fields.
- [ ] Repeating with same build and fixture revision preserves cheaper-side token/byte counts; paired fixtures show net loss and net benefit.
- [ ] Direct arm usage.jsonl is separate; workflow result derives once from JSON envelope jobs[].meta; absent parent usage remains unavailable.
- [ ] `mise exec -- pnpm lint`, `mise exec -- pnpm typecheck`, and `mise exec -- pnpm build` pass; run focused CLI tests if runner integration touches CLI behavior.
- Behavior guarantees belong in linked test-specs, created after plan approval and before task breakdown; runner validation commands are added to the task and executed before report publication.

## Open Questions / Gaps

- Test-spec and task documents are created after plan approval; acceptance scope is fixed by the linked spec.
- Result artifacts should include a small representative committed baseline; full timing samples may be generated locally and need a policy confirmation in task review.

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution; root benchmark runner and result contract form one coherent stream.
- Independent work streams: none.
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- Review focus: verify fake-stream harness can exercise all required workflow/worker fixtures without relaxing `NODE_ENV=test`, ensure direct usage records never merge into workflow aggregates, and confirm no hot files or dependency changes are needed.
