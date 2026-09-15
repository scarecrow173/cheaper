---
id: "TASK-0026"
type: "task"
status: "todo"
title: "Implement sandboxed grep tool"
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
    - "docs/tasks/0025-implement-sandboxed-directory-walker.md"
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

# Implement sandboxed grep tool

## Work

PLAN-0002 step 5。`defineTool` + TypeBox で自前 grep: params `{pattern: string(JS RegExp), path?: ".", glob?, ignoreCase?, maxResults?(default 100, max 500), context?(0-3)}` → `path:line: text` 行。walker を使用し外部プロセスを spawn しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0002-workspace-filesystem-boundary-sandboxed-tools.md`

## Dependencies

- `docs/tasks/0025-implement-sandboxed-directory-walker.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/tools/grep.ts`
- `packages/runtime-pi/test/tools/grep.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] pattern/glob/ignoreCase/maxResults/context・spawn 未呼出のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/tools/grep.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- find は TASK-0027、registry への登録は TASK-0029。
