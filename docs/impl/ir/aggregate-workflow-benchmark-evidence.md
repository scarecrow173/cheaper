---
id: "IMPL-034ATHURho1jepnqtCXbcf"
type: "impl"
status: "in-progress"
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

Workflow envelope は usage.jsonl に workflow aggregate を書かない。direct CLI records を別 arm の evidence として扱い、workflow child usage と二重計上しない。

## Implementation

### Changes

- `aggregateBenchmarkResult` が warm-up を除外し、arm ごとの observation count、median/p95、安定 dimension、比較 delta を生成する。
- 7 scenario、17 arm、289 observations の JSON evidence artifact をまとめた。
- guide は wall time、Worker usage、materialized prompt bytes、retained bytes、definition/interface bytes を別々に示し、親 usage unavailable を明記する。

### Decisions During Implementation

- compact interface bytes は result payload のみ計上し、envelope `durationMs` を除いて再現性を保った。
- 観測できない値や反復間で変動した値は平均値で補完せず `null` にする。
- Interface byte reduction は parent token saving と呼ばず、byte dimension のみ報告する。

## Related Experiments

### Adopted

- Seeded campaign `e45d35b7-2fa6-416b-946a-70ca844b9c80` を採用した。2 warm-up + 15 measured の全 arm が揃う。

### Rejected

- 不要な計測実験はなし。

## Validation

- `mise exec -- node --test --test-reporter=spec benchmarks/test/*.test.mjs` — 13 tests pass。
- `mise exec -- pnpm typecheck` — pass。
- `mise exec -- pnpm build` — pass。
- `mise exec -- pnpm lint` — 0 exit、repository-wide 38 warnings を表示。
- canonical `mise run check` は coordinator merge run に defer。

## Risks

- Wall time は local CLI起動を含む単一 OS/Node environment の測定であり、他 provider・OS へ一般化しない。
- 親 token/turn usage は未観測。

## Follow-ups

- benchmark source、scheduler、prompt、model が変わったら runner を同じ反復数で再実行する。
