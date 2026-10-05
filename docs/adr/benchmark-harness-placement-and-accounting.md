---
id: "ADR-034AQOixbRjL2Fjn1hwUUF"
type: "adr"
status: "accepted"
title: "Benchmark harness placement and accounting"
created: "2026-10-05"
updated: "2026-10-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/67"
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
  related:
    - "docs/specs/workflow-orchestration-benchmark.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/workflow-orchestration-benchmark.md"
  derived-by:
    - "docs/designs/workflow-orchestration-benchmark.md"
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Benchmark harness placement and accounting

## Context and Problem Statement

Issue #67 needs a repeatable benchmark runner and results. Root `scripts/` is hot and must not be changed; `pnpm-workspace.yaml`, package manifests, and `mise.toml` are also hot. The workspace only includes `packages/*`, so a new root benchmark package cannot import workspace packages without changing hot configuration. Workflow child usage is exposed in the returned envelope's `jobs[].meta`; the workflow path does not call `recordUsage`. Direct `cheaper run` invocations may write run entries to usage.jsonl.

## Decision Drivers

* Avoid changes to hot paths and workspace configuration.
* Keep the harness independently rerunnable with deterministic fixtures and no new dependency.
* Preserve the distinction between workflow envelope aggregates, direct-run usage records, and externally observed parent usage.

## Considered Options

* Root `benchmarks/` with a plain Node runner that spawns the built CLI and parses JSON envelopes.
* Fixture directory inside `packages/workflow` or `packages/cli`, invoked by the existing package test tooling.
* Standalone package with its own manifest and CLI/runtime dependencies.

## Decision Outcome

Proposed option: root `benchmarks/` with a plain Node runner invoking the already built CLI as a child process, because it avoids hot workspace/package files, keeps benchmark scenarios out of correctness suites, and adds no dependency. This proposal requires coordinator approval before implementation.

### Consequences

* Good, because placement is isolated and reproducible without changing workspace topology.
* Bad, because the runner must build first and communicate through the CLI envelope rather than direct package imports.

### Confirmation

Confirm the runner is under root `benchmarks/`, spawns the built CLI using `node`, parses JSON envelopes, and does not require edits to hot configuration. Scenario 1 includes a seeded Node filesystem operation as the no-delegation arm and may add one `cheaper run` fake-stream Worker arm if it fits the same harness. Multi-step manual comparison uses scripted direct CLI invocations. Workflow aggregate output comes only from the workflow envelope and child `jobs[].meta`; workflow runs do not create usage.jsonl entries. Direct-arm usage.jsonl observations remain distinct supporting evidence. Parent usage is included only from an observed external source.

## Implementation Plan

* Affected paths: proposed `benchmarks/` runner, scenarios, schema, fixtures, results, and usage guide.
* Patterns to follow: spawn `packages/cli/dist/cli.js`, capture JSON stdout, use existing `CHEAPER_TEST_FAKE_STREAM` test seam with `NODE_ENV=test`, and set deterministic seed/response inputs.
* Patterns to avoid: root `scripts/`, package manifests, pnpm workspace edits, `mise.toml`, fake parent-token savings, and usage.jsonl changes for workflows.
* Dependencies or configuration: no new dependency or hot configuration; run `pnpm build` before `node benchmarks/run.mjs`.

## Verification

* [ ] `pnpm build` followed by `node benchmarks/run.mjs` emits versioned per-scenario result JSON and aggregate report.
* [ ] Workflow aggregation reads one envelope and never adds workflow jobs as usage.jsonl records; direct-arm records are separately labelled.

## Pros and Cons of the Options

### Option 1

* Good, because no package import/configuration coupling is introduced. Bad, because CLI JSON is the integration seam.

### Option 2

* Good, because package-local tests can import APIs directly. Bad, because it couples benchmark fixtures to package suites and package build/test conventions.

## More Information

The benchmark runner must set `NODE_ENV=test` and `CHEAPER_TEST_FAKE_STREAM` to use the existing fake stream seam in `packages/cli/src/cli.ts`; array responses are wrapped through runtime-pi `createFakePi`. The workflow path returns `jobs[].meta` from `packages/cli/src/workflow-run.ts`; unlike direct run, it does not call `recordUsage`. Scenario 1 compares a workflow tool job with identical direct Node work, and optionally one fake `cheaper run` Worker. This ADR remains accepted.
