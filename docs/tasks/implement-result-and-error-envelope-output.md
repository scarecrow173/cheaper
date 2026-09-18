---
id: "TASK-034r05iHK8bA7hr4HQSPhD"
type: "task"
status: "done"
title: "Implement result and error envelope output"
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
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
    - "docs/tasks/scaffold-cheaper-cli-package-with-bin-entry.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement result and error envelope output

## Work

PLAN-034r05iHy2gdnpJFiMWhkY step 4。success/error envelope（DESIGN-034r05iHrZUE6WvDuYjZB4 の JSON 形どおり schemaVersion 1・meta・error.cause/details）を組み立て stdout に envelope 1 つ + 改行のみ出力。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/cli-contract-envelope-exit-codes.md`

## Dependencies

- `docs/tasks/scaffold-cheaper-cli-package-with-bin-entry.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/src/envelope.ts`
- `packages/cli/test/envelope.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] success/error 両形・末尾改行・meta 欠損時 null のテストが green

## Verification

- [x] `mise exec -- pnpm --filter cheaper test -- test/envelope.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- text 整形は TASK-034r05iHQeQpDpo8GaLTOj、stdout 純度の e2e 検証は TASK-034r05iHQeQpDpsYVkvsPX。
