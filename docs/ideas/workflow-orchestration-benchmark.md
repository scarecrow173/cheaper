---
id: "IDEA-034AQJeDtSUKG0S2qSaDBJ"
type: "idea"
status: "promoted"
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
  derives-from: []
  derived-by:
    - "docs/discovery/workflow-orchestration-benchmark.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Benchmark workflow orchestration overhead

## Summary

Benchmark workflow orchestration overhead against the parent-agent turns, context, and intermediate data that workflows may avoid. Use seeded fixtures to produce repeatable positive, negative, and neutral results.

## Problem and Motivation

- Workflow definitions, schema and instruction reading, and orchestration add cost, so workflows are not always cheaper than direct tool calls.
- Issue #67 asks for measured thresholds and guidance for direct calls, dynamic or saved workflows, and public interface discovery.

## Expected Value

- Cheaper users and maintainers can choose invocation style from measured token, byte, concurrency, and timing evidence without inventing parent-token savings.

## Open Questions

- Which package can host a reproducible harness without modifying hot `scripts/` paths?
- Which metrics can the engine expose directly, and how should child usage records avoid double counting?

## Next Action

- [x] Promote to discovery-doc for deeper exploration
- [ ] Promote directly to spec-doc if requirements are clear
- [ ] Park for later reconsideration
- [ ] Discard — reason: <!-- why -->
