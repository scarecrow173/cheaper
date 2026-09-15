---
id: "TASK-0024"
type: "task"
status: "done"
title: "Implement boundary-checked tool operations with atomic writes"
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
    - "docs/tasks/0023-implement-mutation-journal.md"
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

# Implement boundary-checked tool operations with atomic writes

## Work

PLAN-0002 step 3。boundary 付き `ReadOperations`/`WriteOperations`/`EditOperations`/`LsOperations`: 受け取った絶対 path を `resolveWorkspacePath`（core）に通し、違反時 `Error("Access denied: path is outside the workspace")`。write/edit は `.<name>.cheaper-tmp-<rand>` + rename の原子書き込み（tmp path も boundary 判定を通す）で journal と連携。tmp 残留は finally で unlink。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0002-workspace-filesystem-boundary-sandboxed-tools.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0023-implement-mutation-journal.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/tools/operations.ts`
- `packages/runtime-pi/test/tools/operations.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] workspace 外拒否・symlink in/out・create 許容・原子書き込み・tmp 残留なしのテストが green

## Verification

- [x] `mise exec -- pnpm --filter @cheaper/runtime-pi test -- test/tools/operations.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- Pi tool への登録は registry（TASK-0029）。
