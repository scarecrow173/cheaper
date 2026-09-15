---
id: "TASK-0037"
type: "task"
status: "done"
title: "Implement runWorker orchestration with limits and abort classification"
created: "2026-09-15"
updated: "2026-09-15"
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
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
    - "docs/tasks/0031-add-scripted-fake-stream-test-fixture.md"
    - "docs/tasks/0032-implement-pi-session-creation-with-maxtokens-injection.md"
    - "docs/tasks/0034-implement-output-repair-session.md"
    - "docs/tasks/0035-implement-usage-aggregation-from-message-end-events.md"
    - "docs/tasks/0036-implement-modelruntime-adapter-to-core-model-shape.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement runWorker orchestration with limits and abort classification

## Work

PLAN-0002 step 16。`runWorker(deps)` オーケストレーション: `deps.stream?: StreamFn` seam（`session.agent.streamFunction` へ注入）、systemPrompt 構成（Worker body + Output contract 節）、maxToolCalls/maxTurns/timeout/signal による制限と abort（abort 猶予 5s の Promise.race）、abort 後分類（TIMEOUT/ABORTED/MAX_*_EXCEEDED/PARTIAL_MUTATION は journal 非空時・cause に元エラー）、usage/meta 集計、session.dispose()。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0004-pi-execution-engine.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0031-add-scripted-fake-stream-test-fixture.md` — 依存先（Dependency Graph）
- `docs/tasks/0032-implement-pi-session-creation-with-maxtokens-injection.md` — 依存先（Dependency Graph）
- `docs/tasks/0034-implement-output-repair-session.md` — 依存先（Dependency Graph）
- `docs/tasks/0035-implement-usage-aggregation-from-message-end-events.md` — 依存先（Dependency Graph）
- `docs/tasks/0036-implement-modelruntime-adapter-to-core-model-shape.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/run.ts`
- `packages/runtime-pi/test/run.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] timeout/abort/maxTurns/maxToolCalls/PARTIAL_MUTATION/repair/transientRetries の fake stream テストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/run.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- CLI 側の signal ハンドリング・envelope 化は PLAN-0003。
