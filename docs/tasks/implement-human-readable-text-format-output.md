---
id: "TASK-034r05iHQeQpDpo8GaLTOj"
type: "task"
status: "done"
title: "Implement human-readable text format output"
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

# Implement human-readable text format output

## Work

PLAN-034r05iHy2gdnpJFiMWhkY step 5。`--format text`: 1 行目 `ok`/`error <code>`、worker・model・usage・changes の表、続けて result/error を整形 JSON。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/cli-contract-envelope-exit-codes.md`

## Dependencies

- `docs/tasks/scaffold-cheaper-cli-package-with-bin-entry.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/src/text-format.ts`
- `packages/cli/test/text-format.test.ts`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] text 形式の各行・表のテストが green

## Verification

- [x] `mise exec -- pnpm --filter cheaper test -- test/text-format.test.ts` exits 0
- [x] `mise exec -- pnpm biome check .` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- json が既定であることの e2e 検証は TASK-034r05iHQeQpDpsYVkvsPX。
