---
id: "IMPL-034zTprBbdY0tsPiphc2wE"
type: "impl"
status: "completed"
title: "Operation manager state lifecycle"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-operation-manager-state.md"
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

# Operation manager state lifecycle

## Summary

approved plan の manager slice を実装・検証した。code 変更前に IR を開いた。

## Context

TASK-034zTjT0KVd4s2TRC4Ux1B、PLAN-034zTcqSwE6JHL46EJBnKP、共有 D1/D3/D7。

## Implementation

### Changes

operation.ts と state contract tests、barrel export。invocation の OperationManager=never alias を実装 export に置き換える。

### Decisions During Implementation

Active stack: test-driven-development（red/green）、git-commit（logical commit）、impl-doc（evidence）。dispatch の明示 chain に従い profile routing と追加 worker は使わない。maxHops=11、taskBudgetCount=2、commitGate baseline は bdacfc0 と既存 docs dirty 集合。

## Related Experiments

### Adopted

なし。承認済み設計を実装する。

### Rejected

なし。

## Validation

setup exit 0、既存 tool-runtime baseline 3 files/18 tests pass。
新規 operation.test.ts は 50 tests red（export 未実装の assertion failure）を確認後、50 tests green（2026-10-04 14:00 JST）。
最初の green 候補は 49 pass/1 failure: toMatchObject に startedAt=undefined を指定すると field absent と一致しない test assertion の問題。toBeUndefined に分離して修正し、product の未開始 field absent 契約を維持した。
`mise exec -- pnpm typecheck`、`mise exec -- pnpm build` は exit 0。targeted biome check --write は exit 0（2 files を format）。
同一 snapshot の直接改変は state/error/claims の内部 record を変えないことを self-review した。canonical wiring は次の task に委ねる。

## Risks

無協力 executor の実停止は保証しない。unsettled を明示する。

## Follow-ups

後続 canonical wiring task に接続する。canonical check deferred to coordinator merge run。
