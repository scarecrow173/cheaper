---
id: "TASK-034r05iHK8bA7hi27rKIih"
type: "task"
status: "done"
title: "Implement usage aggregation from message end events"
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
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
    - "docs/tasks/scaffold-runtime-pi-package-with-pi-import-smoke-test.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement usage aggregation from message end events

## Work

PLAN-034r05iHy2gdnpIrX0mTIb step 14。`message_end`（assistant）の `message.usage` を逐次加算し `meta.usage = {input, output, cacheRead, cacheWrite, totalTokens, costUsd}` を生成。`auto_retry_start` を `meta.transientRetries` として計上するカウンタもここ。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/pi-execution-engine.md`

## Dependencies

- `docs/tasks/scaffold-runtime-pi-package-with-pi-import-smoke-test.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/usage.ts`
- `packages/runtime-pi/test/usage.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] usage 集計・transientRetries 計上のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/usage.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- meta 全体の組み立ては run/envelope 側。
