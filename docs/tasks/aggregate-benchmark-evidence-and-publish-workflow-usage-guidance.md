---
id: "TASK-034AR8FLSgTI6VLWpQZrQS"
type: "task"
status: "done"
title: "Benchmark evidence 集計と workflow usage guidance"
created: "2026-10-05"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "benchmarks/lib/aggregate.mjs"
      - type: "file"
        path: "benchmarks/results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json"
      - type: "file"
        path: "docs/guides/workflow-usage.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
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
  references: []
  defers: []
  deferred-by: []
---

# Benchmark evidence 集計と workflow usage guidance

## 作業

Task 1 の observation を集計し、usage source の二重計上を防ぎ、実測結果に基づく workflow/direct/saved/interface の選択指針を公開する。

## 完了内容

- aggregate は warm-up を除外し、median/p95、反復で安定した token/byte、arm 差分を算出する。不安定または欠落した値は `null` とする。
- workflow usage は `jobs[].meta` のみから集計し、direct Worker envelope を workflow aggregate に加算しない。
- 親 usage unavailable を明示し、scripted direct CLI invocation を actual parent turn として扱わない。
- 7 scenario / 17 arm、289 observation、2 warm-up + 15 measured の evidence artifact を保存した。
- 日本語 guide に net loss、retained/interface byte reduction、計測不能範囲、再実行手順を記載した。

## 検証

- `mise exec -- node --test --test-reporter=spec benchmarks/test/*.test.mjs` — 13 tests pass。
- `mise exec -- pnpm lint` — 0 exit、既存 warnings 38 件を表示。
- `mise exec -- pnpm typecheck` — pass。
- `mise exec -- pnpm build` — pass。
- canonical `mise run check` は coordinator merge run に defer。
