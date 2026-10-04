---
id: "PLAN-034zWZgd5LEtlTQNGC5O8x"
type: "plan"
status: "in-progress"
title: "ToolScheduler と concurrency policy を実装する"
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
    - "docs/specs/bounded-tool-scheduler.md"
  implemented-by: ["../tasks/bounded-concurrency-policy.md","../tasks/shared-tool-scheduler.md"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/bounded-tool-scheduler.md"
    - "docs/designs/bounded-tool-scheduler.md"
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/bounded-concurrency-policy.md","../test-specs/shared-tool-scheduler.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# ToolScheduler と concurrency policy を実装する
## Goal
承認済み spec/design と共有 ADR D1/D2/D7 の通り invocation の operation parallelism を制限する。
## Scope and Split
policy と admission は同じ上限の定義と enforcement を結ぶ一つの実装 stream。2 task の依存 DAG（policy → scheduler/wiring）として inline 実行する。追加 dispatch は行わず、review は coordinator が担当する。
## File Map
| Files | Responsibility |
| --- | --- |
| packages/core/src/policy/{schema,merge,load,restrict}.ts | 共通 resolver、legacy/partial schema、trusted/project 段階、restriction |
| packages/core/test/policy/** | schema/merge/load/restrict の境界 test |
| packages/tool-runtime/src/scheduler.ts | queued→admission→running、FIFO shared/exclusive、abortable 待機、settle による枠解放 |
| packages/tool-runtime/src/{registry,invocation,index}.ts | authoritative scheduler ownership、known tool 分類、public descriptor wrapper |
| packages/tool-runtime/test/** | bounded overlap/ordering/first-cause/preabort/late-settle/budget/bypass |
| packages/runtime-pi/src/{run,tools/registry}.ts | concurrency policy wiring と standalone options |
| packages/runtime-pi/test/tool-scheduler.test.ts | Pi sibling と Workflow-style direct scope の共有上限を決定的検証 |
## Dependency Graph
| Task | Depends on | Done When |
| --- | --- | --- |
| bounded-concurrency-policy | — | core policy focused suite GREEN |
| shared-tool-scheduler | bounded-concurrency-policy | tool-runtime/Pi focused suite GREEN、lint/typecheck/build pass |
## Steps
1. policy invariant tests を書き focused run で RED を確認する。
2. schema/resolver、merge mode、load 段階、restrict API を実装し focused GREEN を確認する。
3. policy の ir/evidence と task status を更新し論理的変更を commit する。
4. scheduler/adapter の invariant tests を書き focused RED を確認する。
5. scheduler と invocation/registry、Pi 最小 wiring を実装する。旧並行 ownership tests は shared operation に改め不変条件を維持する。
6. focused suites と lint/typecheck/build を実行し ir/evidence/status を更新して commit する。
7. follow-up を分類、managed indexes を plan→apply、chain+共有ADRの scoped exit-audit を実行する。
## Verification Matrix
| Contract | Check |
| --- | --- |
| complete/partial/legacy、trusted+restriction | mise exec -- pnpm --filter @cheaper/core exec vitest run test/policy |
| FIFO bound、queued cancel、late settle、public入口 | mise exec -- pnpm --filter @cheaper/tool-runtime test |
| Pi/direct shared bound と filesystem/writeScope/journal | mise exec -- pnpm --filter @cheaper/runtime-pi test |
| format/type/build | mise exec -- pnpm lint; mise exec -- pnpm typecheck; mise exec -- pnpm build |
canonical check deferred to coordinator merge run。bundle payload 再生成・CLI/Workflow compiler 変更・依存変更は行わない。
## Risks and Rollback
FIFO head-of-line 待機と broad exclusive は意図的保守性。unsettled handler は実 settle まで枠を保持する。complete explicit concurrency は両 field 必須、legacy は省略可。
policy/wiring の論理 commit を個別 revert でき、persistence/migration は無い。preabort terminal record を enqueue/start しないことを pin する。
## Approval
coordinator が plan gate を承認（msg_73d177e6ac2d）。test-spec-task gate も coordinator 承認済み。policy/scheduling の意図文書と2 taskを起点に実装する。
