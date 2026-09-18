---
id: "TASK-034r05iHK8bA7h7H6VxKdG"
type: "task"
status: "done"
title: "Implement sandboxed find tool"
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
    - "docs/tasks/implement-sandboxed-directory-walker.md"
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

# Implement sandboxed find tool

## Work

PLAN-034r05iHy2gdnpIrX0mTIb step 6。`defineTool` + TypeBox で自前 find: params `{pattern: string(glob, `path.matchesGlob`), path?: ".", maxResults?(default 200, max 1000)}` → 相対 path 行。walker を使用。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md`

## Dependencies

- `docs/tasks/implement-sandboxed-directory-walker.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/tools/find.ts`
- `packages/runtime-pi/test/tools/find.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] glob 一致・maxResults・相対 path 出力・spawn 未呼出のテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/tools/find.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- registry への登録は TASK-034r05iHK8bA7hbds7QpLZ。
