---
id: "SPEC-034AQOi1TPvVMDkNrhfTM5"
type: "spec"
status: "approved"
title: "Benchmark workflow orchestration overhead"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/workflow-orchestration-benchmark.md"
  derived-by:
    - "docs/designs/workflow-orchestration-benchmark.md"
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-benchmark-scenario-determinism.md"
    - "docs/test-specs/workflow-benchmark-accounting-integrity.md"
  references:
    - "docs/adr/benchmark-harness-placement-and-accounting.md"
  defers: []
  deferred-by: []
---

# Benchmark workflow orchestration overhead

## What Should Be Built

Provide a reproducible benchmark harness and evidence-based usage guide comparing workflow orchestration with direct or parent-managed invocation. Measure observable cheaper-side cost and work without presenting unobserved parent-agent usage as savings.

## Why Now

Workflow creation, interface discovery, and orchestration add overhead. Users lack evidence to decide when that overhead is outweighed by reduced Worker input, intermediate data transfer, and parent-agent orchestration effort.

## Users and Value

Cheaper users choosing between direct tools, dynamic workflows, saved workflows, and interface discovery; maintainers detecting regressions after scheduler, prompt, or model changes.

## Scope

### In Scope

- Seeded scenarios cover a simple call; fan-out plus Worker fan-in; tool-to-Worker chain; process plus analysis with large intermediate output; practical parent-managed orchestration; dynamic versus saved workflows; and full definition versus compact interface discovery.
- Record workflow/node counts, tool-only and Worker jobs, Worker turns/tool calls/input/output/cached tokens, retained and materialized bytes, wall time/critical path/max concurrency, definition/interface sizes, and discovery mode.
- Keep scenario definitions, runner, raw observations, and aggregation rerunnable after runtime changes.
- Provide a guide selecting workflow, direct invocation, saved workflow, or compact interface discovery from collected evidence.

### Out of Scope

- Automatic cost optimization or forcing all invocations through workflows.
- Fabricated avoided-parent token savings or treating child usage.jsonl records as workflow aggregates.
- Claims that seeded fixtures establish real-model semantic quality or universal provider latency.

## Requirements

- Deterministic fixture mode reproduces token and byte counts and identifies fixture/model/prompt inputs.
- Timing and concurrency reports label execution scope and distinguish measured wall time from critical path.
- Parent usage is recorded only when the parent runtime supplies an observed value; otherwise it is explicitly unavailable.
- Workflow runs produce one benchmark aggregate artifact from envelope `jobs[].meta`; they are not written to usage.jsonl. Direct orchestration child invocations may produce usage.jsonl run records, which are supporting observations rather than separate benchmark aggregate records.
- The paired fixture set is constructed to guarantee at least one net-loss and one net-benefit result on measured cheaper-side token/byte dimensions; conclusions do not assert a universal threshold.
- Manual comparison includes a seeded Node filesystem operation for scenario 1 as a no-delegation baseline, plus a single `cheaper run` fake-stream Worker doing one tool call if it fits the existing harness; multi-step work uses a scripted sequence of direct CLI invocations matching fixture work. Invocation count is a turns proxy, never a parent-agent token estimate. Parent usage remains unavailable unless independently observed.
- Result artifact uses a versioned JSON schema under `benchmarks/results/`, with scenario ID, fixture revision, mode, repetitions, environment, envelope and job metadata, byte counts, and parentUsage `observed|unavailable`.
- Harness location and invocation are subject to design approval; avoid hot `scripts/` and package manifests without approval.

## Acceptance Criteria

- [ ] Seven required scenario families have repeatable seeded fixtures and emit structured cheaper-side token, byte, and timing observations.
- [ ] Constructed fixture pairs demonstrate one net-loss and one net-benefit on measured token/byte dimensions without inferred parent savings.
- [ ] Parent usage is directly observed and separately reported or explicitly unavailable.
- [ ] Workflow aggregates derive from one workflow envelope and are excluded from usage.jsonl aggregation; direct-arm usage records are not added again as separate aggregate totals.
- [ ] Usage guidance cites result identifiers and states observation limits.
- [ ] A documented command reruns scenarios and regenerates results after prompt, scheduler, or model changes.

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

- Live-provider semantic quality and broad latency comparisons | seeded fixtures establish repeatability but not provider generality | revisit when a controlled provider matrix is available | fixture timing could otherwise be overgeneralized.

## Sources and Relations

Issue #67 defines scenario coverage, measurements, integrity constraints, and acceptance criteria. A linked ADR draft proposes harness placement and accounting boundaries for coordinator review.
