---
id: "ADR-034A8u7fBRIGJ7eRFYvg38"
type: "adr"
status: "accepted"
title: "Operation execution metrics"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  source: ["https://github.com/scarecrow173/cheaper/issues/58"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../discovery/operation-execution-metrics.md"]
  derived-by: ["../designs/operation-execution-metrics.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Operation execution metrics

## Context and Problem Statement

共有 ADR D3 は型と意味を固定している。実装の計測責務を lifecycle 所有者で一元化する。

## Decision Drivers

- 同一 clock、owner scope、cancel と settle の区別。
- scheduler の ordering/admission を維持する。

## Considered Options

- lifecycle hook で別 collector に状態をコピーする。
- OperationManager の timestamp records を純粋 reducer で集計する。

## Decision Outcome

duration は records reducer を採用する。maxConcurrency は同時刻/0 duration でも正確にするため manager の lifecycle に passive owner/global running count/high-water を置く。ExecutionMetrics と collectExecutionMetrics は tool-runtime から export し、OperationManager.executionMetrics(ownerId?) が clock と records を供給する。InvocationToolRuntime が委譲する。新しい依存は追加しない。

### Consequences

状態ドリフトを避けられる。snapshot の interval sweep は O(n log n)、既存 records と同じ invocation lifetime の記憶量。snapshot 内の byTool は own property として安全に集計する。

### Confirmation

controlled clock の reducer/lifecycle tests と既存 scheduler tests、runtime/CLI envelope tests、lint/typecheck/build。

## Implementation Plan

tool-runtime execution-metrics.ts、operation.ts、invocation.ts、index.ts、runtime-pi run.ts、CLI envelope.ts と focused tests。
