---
id: "TASK-0049"
type: "task"
status: "done"
title: "Run full build test and lint verification"
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
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
    - "docs/tasks/0048-add-fake-stream-hook-and-e2e-execution-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Run full build test and lint verification

## Work

PLAN-0003 step 11。`pnpm -r build && pnpm -r test && pnpm biome check .` の全 pass を確認し、失敗があれば該当 task に戻って修正する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0008-repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/0048-add-fake-stream-hook-and-e2e-execution-tests.md` — 依存先（Dependency Graph）

## Affected Files

- なし（検証のみ）

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 3 コマンド全て exit 0

## Verification

- [x] `mise exec -- pnpm -r build && mise exec -- pnpm -r test` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0

## Out of Scope

- ドキュメント記録は TASK-0050/0051。
