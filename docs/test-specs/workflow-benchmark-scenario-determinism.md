---
id: "TSPEC-034AR5IRUTHkZBxfcQJFgM"
type: "test-spec"
status: "proposed"
title: "Workflow benchmark scenario determinism"
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
    - "docs/designs/workflow-orchestration-benchmark.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-orchestration-benchmark.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow benchmark scenario determinism

## Purpose

Preserve repeatability and meaningful paired comparisons as fixtures or runner internals change. Retire only if seeded regression evidence is no longer part of the benchmark contract.

## Feature

Feature: Reproducible workflow orchestration benchmark scenarios

## Rules

- Rule: Every required scenario family executes through a seeded fixture with stable token and byte dimensions.
- Rule: Paired fixtures construct at least one workflow net-loss and one net-benefit case on measured cheaper-side token/byte dimensions.
- Rule: Timing records host environment, two warmups, 15 measured repetitions, median, and p95; timing variance does not overwrite deterministic dimensions.

## Examples

- Example: Given the same build and fixture revision, when the runner runs twice, scenario and aggregate token/byte counts match while wall-time observations may vary.
- Example: Given a single simple tool call and a paired multi-stage scenario, when comparing workflow and direct execution, one constructed fixture loses and the other benefits on token/byte dimensions.
- Example: Given fan-out/fan-in or large intermediate stdout, when a seeded run executes, its result retains job counts, bytes, concurrency, and timing statistics.

## Guarantees

- Seven issue-defined scenario families remain represented and individually identified.
- Reruns record fixture, fake response, model/prompt, seed, and build revision identifiers.
- Missing runner observations fail as incomplete evidence rather than zero.

## Non-goals

- Live-provider semantic quality and provider-general latency claims.
- Parent-agent tokens not directly observed by a parent runtime.

## Risk

- Fixture drift or unstable token/byte output would make runtime regressions hard to distinguish from benchmark noise.
