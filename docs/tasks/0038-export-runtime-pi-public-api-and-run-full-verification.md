---
id: "TASK-0038"
type: "task"
status: "done"
title: "Export runtime-pi public API and run full verification"
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
    - "docs/tasks/0037-implement-runworker-orchestration-with-limits-and-abort-classification.md"
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

# Export runtime-pi public API and run full verification

## Work

PLAN-0002 step 17。`index.ts` export + `pnpm -r build && pnpm -r test && pnpm biome check .` 全 pass。grep/find が `child_process.spawn` を呼ばないことを `vi.spyOn` 未呼出 assert で確認。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0002-implement-cheaper-runtime-pi-execution-engine-and-sandboxed-tools.md`
- `docs/designs/0008-repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/0037-implement-runworker-orchestration-with-limits-and-abort-classification.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/runtime-pi/src/index.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 全ビルド・テスト・lint green、spawn 未呼出 assert が green

## Verification

- [x] `mise exec -- pnpm -r build && mise exec -- pnpm -r test` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0

## Out of Scope

- cli パッケージは PLAN-0003。
