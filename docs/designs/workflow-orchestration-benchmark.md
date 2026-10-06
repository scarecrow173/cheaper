---
id: "DESIGN-034AQQsGub6ocuyzZg3Fuv"
type: "design"
status: "approved"
title: "Design workflow orchestration benchmark harness"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-orchestration-benchmark.md"
    - "docs/adr/benchmark-harness-placement-and-accounting.md"
  derived-by:
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
    - "docs/test-specs/workflow-benchmark-scenario-determinism.md"
    - "docs/test-specs/workflow-benchmark-accounting-integrity.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Design workflow orchestration benchmark harness

## Context

Issue #67 requires repeatable evidence about orchestration cost and avoided work, including cases where workflow use is wasteful. Workflow runs return `jobs[].meta` in the JSON envelope and do not write usage.jsonl; direct CLI runs can write usage.jsonl records. Neither is parent-agent usage.

## Scope

### In Scope

- Seeded fixtures and runner in proposed root `benchmarks/`, separate from hot `scripts/` and package manifests. Because root `benchmarks/` is not a workspace package, `run.mjs` spawns the built CLI (`node packages/cli/dist/cli.js` or the established built entry) and parses JSON envelopes instead of importing workspace packages.
- Structured per-run observations and an aggregate report for the seven scenario families in the spec.
- Evidence-based usage guide linking recommendations to result identifiers.

### Out of Scope

- Automatic optimizer, provider-general cost claims, or inferred parent-agent token savings.
- Changes to workflow runtime, CLI, scheduler, or policy contracts to accommodate benchmarks.

## Components and Boundaries

- Scenario fixtures define equivalent work and workflow definitions/inputs for direct, dynamic, and saved paths.
- Runner uses `CHEAPER_TEST_FAKE_STREAM` with `NODE_ENV=test` to route both `run` and `workflow run` through deterministic `createFakePi` responses; fixtures record response, prompt, and model identifiers. Other CLI execution remains out-of-process.
- Workflow aggregate record is derived once from its workflow envelope and job metadata. It is not sourced from or written into usage.jsonl. Direct-arm usage.jsonl records can support that arm but are not added as second benchmark aggregates.
- Aggregator reports observed parent usage only when an external parent runtime supplies it; otherwise result schema stores `parentUsage: unavailable`.
- Report generation calculates token/byte deltas and measured timing/concurrency with scope labels; no synthetic parent savings.

## Data and Control Flow

- Fixture scenario → spawned built CLI → parsed JSON envelope → run metadata and raw per-job observations → validated aggregation → guide tables and machine-readable `benchmarks/results/<run-id>.json` artifact.
- Dynamic/saved and full-definition/interface pairs share scenario work so deltas reflect discovery/invocation form.
- Manual comparison uses two bounded arms: scenario 1 also measures an identical seeded Node filesystem operation as the no-delegation baseline; where practical, it pairs that with a single `cheaper run` fake-stream Worker doing one tool call. Multi-step manual orchestration uses a scripted sequence of direct CLI invocations matching fixture work. Invocation count is a turns proxy only, not observed parent turns or tokens; parent token fields stay unavailable absent runtime telemetry.

## Non-Functional Constraints

- Seed and fixture/model/prompt versions are recorded; token and byte dimensions are deterministic for seeded runs.
- Record host OS/arch, Node version, CPU count, command/build revision, seed, and fixture/model/prompt IDs. Run 2 warmups plus 15 measured repetitions per scenario; report median and p95 wall time, while critical path and max concurrency come from runtime metrics, not wall-time inference.
- Runner failures and missing fields produce incomplete evidence, never zero cost.
- Deterministic regression runs require no external provider.

## Risks and Trade-offs

- Execution environments change wall time; state environment and repetitions and use deterministic token/byte counts as regression anchors.
- Direct child runs can appear in usage.jsonl while workflow children do not; distinct benchmark result records must use the envelope as the sole workflow aggregate source.
- Root benchmark entry point is a repository convention requiring coordinator approval before implementation.

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- Parent runtime adapters | current benchmark interface may not expose usage | revisit when runtime telemetry exists | parent token comparison remains unavailable.

## Verification Notes

- [ ] Coordinator approves `benchmarks/` and the record separation rules.
- [ ] Two clean invocations of `node benchmarks/run.mjs` with identical revision and seed reproduce token/byte outputs; timing reports 2 warmups, 15 repetitions, median and p95.
- [ ] Result schema excludes missing parent usage and never combines one workflow-envelope aggregate with child usage records.
- [ ] Runner sets `NODE_ENV=test` and `CHEAPER_TEST_FAKE_STREAM` and captures JSON envelopes from the built CLI.
