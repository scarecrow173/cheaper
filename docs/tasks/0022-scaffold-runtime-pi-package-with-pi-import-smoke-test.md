---
id: "TASK-0022"
type: "task"
status: "done"
title: "Scaffold runtime-pi package with Pi import smoke test"
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
    - "docs/tasks/0021-export-core-public-api-and-run-full-verification.md"
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

# Scaffold runtime-pi package with Pi import smoke test

## Work

PLAN-0002 step 1。`@cheaper/runtime-pi` scaffold（deps: @cheaper/core workspace, @earendil-works/pi-coding-agent@0.85.1, @earendil-works/pi-ai@0.85.1, typebox, diff）。smoke test で `turn_end`/`message_end`/`tool_execution_start`/`auto_retry_start` イベント名と `createReadTool`/`createWriteTool`/`createEditTool`/`createLsTool`/`defineTool`/`SessionManager.inMemory`/`SettingsManager.inMemory`/`createAgentSession` の型存在を確認。差異があれば停止して報告。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0004-pi-execution-engine.md`
- `docs/designs/0008-repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/0021-export-core-public-api-and-run-full-verification.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/package.json`
- `packages/runtime-pi/tsconfig.json`
- `packages/runtime-pi/src/index.ts`
- `packages/runtime-pi/test/smoke.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] Pi import smoke test が green（差異があれば実装せず停止・報告）

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/smoke.test.ts` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- session 生成・tool 実装は TASK-0023 以降。
