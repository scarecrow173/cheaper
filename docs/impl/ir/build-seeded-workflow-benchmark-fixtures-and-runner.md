---
id: "IMPL-034ARlQpkPs620j6TP15Rv"
type: "impl"
status: "completed"
title: "Seeded workflow benchmark fixtures と runner"
created: "2026-10-05"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "benchmarks/scenarios/scenarios.json"
      - type: "file"
        path: "benchmarks/lib/result-schema.mjs"
      - type: "file"
        path: "benchmarks/lib/run-scenario.mjs"
      - type: "file"
        path: "benchmarks/run.mjs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/build-seeded-workflow-benchmark-fixtures-and-runner.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-benchmark-scenario-determinism.md"
  references:
    - "docs/adr/benchmark-harness-placement-and-accounting.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Seeded workflow benchmark fixtures と runner

## Summary

7 scenario family を fake Worker と built CLI で反復測定する root `benchmarks/` harness を実装した。

## Context

Issue #67 は tool-only 単発、fan-out/fan-in、tool/Worker chain、process output、manual sequence、dynamic/saved workflow、definition/interface の再現可能な比較を求める。親 token が測定できない場合は節約として扱わない。

## Implementation

### Changes Made

- Seeded scenario manifest、saved workflow、fake stream、record validation、prompt/timing/critical-path 計測を追加した。
- Built CLI を `NODE_ENV=test` と `CHEAPER_TEST_FAKE_STREAM` subprocess 実行する runner を追加した。
- Fan-out fixture を独立 read tool 2 件 + Worker fan-in 1 件にした。複数 fake Worker の concurrent model resolution は反復中に `MODEL_TIER_UNAVAILABLE` となったため、要件に沿う tool fan-out に変更した。
- Interface size は envelope の `result` payload に限定し、可変 `durationMs` を除いた。
- Process stdout は `/details/stdout/text` から Worker input に bind する。

### Decisions During Implementation

- Direct invocation 数は親 turns の proxy にとどめる。
- Tool-only workflow は workflow envelope の job metadata から集計し、`usage.jsonl` へ加算しない。

## Related Experiments

### Adopted

- Seeded fixtures を実装し、比較 sweep は aggregation record に保存した。

### Rejected

- Concurrent fan-out fake Worker は同一 fake provider の model resolution が反復中に空候補を返したため、tool-job fan-out fixture に置き換えた。

## Validation

- 全 7 scenario を各 arm warm-up 2 回 + measured 15 回で実行し、`benchmarks/results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json` に保存した。
- `mise exec -- node --test --test-reporter=spec benchmarks/test/*.test.mjs` — 13 tests pass。
- `mise exec -- pnpm typecheck` — pass。
- `mise exec -- pnpm build` — pass。
- `mise exec -- pnpm lint` — 0 exit、repository-wide 38 warnings。
- canonical `mise run check` は coordinator merge run に defer。
- Coordinator review: commit range `6671872..7b2b3b4` approved for implementation.

## Risks

- Timing は local CLI 起動を含むため、他 OS/provider への一般化は未検証。
- Parent token/turn usage は未観測。

## Follow-ups

- Scheduler、prompt、model、CLI に変更があれば `benchmarks/run.mjs` を同じ反復数で再実行する。
