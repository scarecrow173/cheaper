---
id: "IMPL-034A64NK6xfzCOIY0zLm75"
type: "impl"
status: "completed"
title: "Bounded concurrency policy"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
  changes:
    added: [{"type":"code","path":"packages/core/test/policy/concurrency.test.ts"}]
    modified: [{"type":"code","path":"packages/core/src/policy/schema.ts"},{"type":"code","path":"packages/core/src/policy/merge.ts"},{"type":"code","path":"packages/core/src/policy/load.ts"},{"type":"code","path":"packages/core/src/policy/restrict.ts"},{"type":"code","path":"packages/core/test/policy/load.test.ts"}]
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/bounded-concurrency-policy.md"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Bounded concurrency policy
## Summary
execution.concurrency の trusted/restriction 境界を実装した。
## Context
TASK-034zXj9deJ0LTUdaiGwflX / PLAN-034zWZgd5LEtlTQNGC5O8x と共有 ADR D2。spec-design-policy、plan、test-spec-task は coordinator 承認済み。
## Implementation
### Changes
shared resolveConcurrencyPolicy が既定8/serializedと値検証を所有する。completeはlegacy全体省略を許すがpresent時両field必須、partialはfield省略可。
user trusted replace後のdefault補完とproject restriction、CLI-style APIのcap/serialized緩和拒否を実装。旧scalar/array merge規則は維持した。
### Decisions During Implementation
RuntimePolicy.execution.concurrencyは承認済みlegacy型互換でoptional。normalize/loadの返却はresolverで補完する。diffで正本とdefault補完順を確認した。
## Related Experiments
### Adopted
なし。既知設計のためexpを作らなかった。
### Rejected
なし。
## Validation
RED: concurrency.test.ts 16 tests中9 failures（既定/schema/load/restrict未実装）、exit1。
GREEN: mise exec -- pnpm --filter @cheaper/core exec vitest run test/policy = 6 files / 60 tests passed、exit0。
mise exec -- pnpm --filter @cheaper/core build = exit0。Biome対象6files修正後にsuite/buildを実行した。
## Risks
per-resourceはpolicy vocabularyのみ、実leaseは#61。canonical check deferred to coordinator merge run。
## Follow-ups
依存task shared-tool-schedulerへ進む。policy本sliceの未解決事項なし。IR indexはbundled CLIに再利用exportが無いため、承認済み例外(2)で当該rowのstatusだけを機械同期する。
