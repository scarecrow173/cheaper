---
id: "TASK-034r05iHK8bA7hbds7QpLZ"
type: "task"
status: "done"
title: "Implement tool registry mapping vocabulary to Pi tools"
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
    - "docs/tasks/implement-boundary-checked-tool-operations-with-atomic-writes.md"
    - "docs/tasks/implement-sandboxed-grep-tool.md"
    - "docs/tasks/implement-sandboxed-find-tool.md"
    - "docs/tasks/implement-tool-call-budget-wrapper.md"
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

# Implement tool registry mapping vocabulary to Pi tools

## Work

PLAN-034r05iHy2gdnpIrX0mTIb step 8。`ToolName` → Pi `ToolDefinition` 生成: read/write/edit/ls は `createReadToolDefinition` 等に boundary operations を注入、grep/find は自前 tool、全 tool を budget wrap。effectiveToolNames に従い allowlist 用の名前集合を返す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md`
- `docs/designs/pi-execution-engine.md`

## Dependencies

- `docs/tasks/implement-boundary-checked-tool-operations-with-atomic-writes.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-sandboxed-grep-tool.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-sandboxed-find-tool.md` — 依存先（Dependency Graph）
- `docs/tasks/implement-tool-call-budget-wrapper.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/tools/registry.ts`
- `packages/runtime-pi/test/tools/registry.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] vocabulary 全 tool の生成・operations 注入・budget wrap のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/tools/registry.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- session への渡し方は TASK-034r05iHK8bA7he1WXwcBs。
