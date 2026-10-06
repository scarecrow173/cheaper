---
id: "DISC-034AQMgmHD0r6s13EoPlQ9"
type: "discovery"
status: "resolved"
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
    - "docs/ideas/workflow-orchestration-benchmark.md"
  derived-by:
    - "docs/specs/workflow-orchestration-benchmark.md"
    - "docs/adr/benchmark-harness-placement-and-accounting.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Benchmark workflow orchestration overhead

## Exploration Goal

Which workflow usage patterns provide enough measured cheaper-side benefit to offset definition, interface discovery, and orchestration overhead? Issue #67 requires repeatable evidence across simple calls, fan-out/fan-in, tool-to-worker chains, large process artifacts, manual parent orchestration, dynamic versus saved definitions, and full-definition versus compact-interface discovery.

## Key Issues and Assumptions

- Parent-agent token usage is reportable only when the parent runtime exposes observed usage; cheaper-side metrics cannot stand in for it.
- Workflow runs return child metadata in envelope `jobs[].meta` and do not write usage.jsonl; direct CLI invocations may write usage.jsonl entries. Benchmark aggregation must treat each source as a distinct evidence domain.
- Seeded deterministic fixtures are suitable for repeatable cheaper-side token/byte counts, while wall time and concurrency need explicit scope and repeat policy.
- The harness must live outside hot `scripts/`; its final placement is a design-gate decision.

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Standalone benchmark package | Isolated fixtures and runner; easy to rerun independently | Adds package/build surface and may require a hot package manifest | maybe |
| Workflow-package benchmark fixtures | Close to engine and artifact APIs; can reuse package tests | May mix performance harness with the engine's correctness suite | maybe |
| Root `benchmarks/` workspace-neutral harness | Keeps benchmarks outside runtime packages and avoids `scripts/` | Needs an explicit dependency/runtime strategy | maybe |

## Tentative Conclusions and Hypotheses

Hypothesis: report measured workflow overhead and cheaper-side work as separate dimensions, and derive usage guidance from paired scenarios instead of claiming a universal savings threshold. The required scenarios and measurement constraints are confirmed by issue #67; harness placement and exact parent-orchestration protocol remain unresolved.

## Open Questions

- None for the proposed design: harness at root `benchmarks/`, plain Node runner via built CLI and JSON envelopes, `CHEAPER_TEST_FAKE_STREAM` with `NODE_ENV=test`, 2 warmups plus 15 repetitions reporting median/p95, and scripted direct invocation count as a turns proxy. Parent tokens remain unavailable absent external telemetry.

## Promotion Candidates

- [x] spec-doc needed: reproducible scenarios, measurement integrity, rerun behavior, and evidence-based usage guidance.
- [x] adr-doc needed: benchmark harness placement and accounting boundaries if they constrain package/runtime design.

## Sources and Relations

Issue #67 and its 2026-10-04 triage decisions are the authoritative requirements source. No external model-cost or parent-token measurements have been collected in this discovery phase.
