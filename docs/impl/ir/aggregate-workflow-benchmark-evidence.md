---
id: "IMPL-034ATHURho1jepnqtCXbcf"
type: "impl"
status: "completed"
title: "Aggregate workflow benchmark evidence"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "benchmarks/lib/aggregate.mjs"
      - type: "file"
        path: "benchmarks/README.md"
      - type: "file"
        path: "docs/guides/workflow-usage.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "benchmarks/results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json"
  implements:
    - "docs/tasks/aggregate-benchmark-evidence-and-publish-workflow-usage-guidance.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/build-seeded-workflow-benchmark-fixtures-and-runner.md"
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
    - "docs/test-specs/workflow-benchmark-accounting-integrity.md"
  references:
    - "docs/adr/benchmark-harness-placement-and-accounting.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Aggregate workflow benchmark evidence

## Summary

Seeded observations を統合し、usage accounting と bytes/timing 比較に基づく日本語 usage guide を作成した。

## Context

Workflow envelope は `usage.jsonl` に workflow aggregate を書かない。direct CLI records は別 arm の evidence として扱い、workflow child usage と二重計上しない。

## Implementation

### Changes Made

- `aggregateBenchmarkResult` が warm-up を除外し、arm ごとの observation count、median/p95、安定 dimension、比較 delta を生成する。
- 7 scenario、17 arm、289 observations の JSON evidence artifact を記録した。
- Guide は wall time、Worker usage、materialized prompt bytes、retained bytes、definition/interface bytes を分け、親 usage が未観測であることを明記する。

### Decisions During Implementation

- Compact interface bytes は result payload のみ計上し、envelope `durationMs` を除いて再現性を保った。
- 観測できない値や反復間で変動した値は平均値で補完せず `null` にする。
- Interface byte reduction は parent token saving と呼ばず、byte dimension のみ報告する。

## Related Experiments

### Adopted

- Seeded campaign `e45d35b7-2fa6-416b-946a-70ca844b9c80` を採用した。全 arm で warm-up 2 回と measured 15 回が揃う。

### Rejected

- 追加の計測実験は行っていない。

## Validation

- `mise exec -- node --test --test-reporter=spec benchmarks/test/*.test.mjs` — 13 tests pass。
- `mise exec -- pnpm typecheck` — pass。
- `mise exec -- pnpm build` — pass。
- `mise exec -- pnpm lint` — 0 exit、repository-wide 38 warnings。
- canonical `mise run check` は coordinator merge run に defer。
- Coordinator review: commit range `6671872..7b2b3b4` approved for implementation.

## Risks

- Wall time は local CLI 起動を含む単一 OS/Node environment の測定であり、他 provider・OS へ一般化しない。
- 親 token/turn usage は未観測であり、節約量を推定していない。

## Follow-ups

- Benchmark source、scheduler、prompt、model が変わったら runner を同じ反復数で再実行する。
