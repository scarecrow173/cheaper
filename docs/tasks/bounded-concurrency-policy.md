---
id: "TASK-034zXj9deJ0LTUdaiGwflX"
type: "task"
status: "done"
title: "Bounded concurrency policy を導入する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-bounded-tool-scheduler.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-bounded-tool-scheduler.md"
  blocks: ["./shared-tool-scheduler.md"]
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/bounded-concurrency-policy.md"
  references: []
  defers: []
  deferred-by: []
---

# Bounded concurrency policy を導入する
## Goal
core の schema/merge/load/restrict に共有 ADR D2 の trusted/restriction semantics を導入する。
## Scope
packages/core/src/policy/{schema,merge,load,restrict}.ts と packages/core/test/policy/**。他 core module、root hot file、CLI、依存は変更しない。
## Steps
1. schema/normalize、min/serialized merge、実ファイル load、restriction の failing tests を追加し RED を確認する。
2. shared resolver と legacy/partial schema を実装する。
3. default補完順とmerge mode、restriction を実装しfocused GREENを確認する。
4. ir/evidence を更新し意味のあるcommitを作成する。
## Done When
trusted/user設定をprojectが緩和できず、legacy/partial omission と既存timeout/scalar/array挙動が維持される。
## Verification
- [ ] mise exec -- pnpm --filter @cheaper/core exec vitest run test/policy がexit0。
- [ ] mise exec -- pnpm --filter @cheaper/core build がexit0。
## Risks / Follow-ups
RuntimePolicy.execution.concurrency は旧typed fixture互換でoptional。normalized policyは常にresolver値を返す。canonical check deferred to coordinator merge run。
