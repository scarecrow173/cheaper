---
id: "TASK-0041"
type: "task"
status: "done"
title: "Implement input reading from stdin or file"
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
    - "docs/tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md"
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

# Implement input reading from stdin or file

## Work

PLAN-0003 step 3。`--input <file|->`（既定 `-`）/stdin 読取。TTY で stdin 未指定でも待たず空入力 → `INVALID_REQUEST`。入力は JSON object 必須。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0005-cli-contract-envelope-exit-codes.md`

## Dependencies

- `docs/tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/src/input.ts`
- `packages/cli/test/input.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] stdin・file・空入力・非 object の各テストが green

## Verification

- [x] `mise exec -- pnpm --filter cheaper test -- test/input.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- envelope 化は TASK-0042。
