---
id: "TASK-0043"
type: "task"
status: "todo"
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

# Implement human-readable text format output

## Work

PLAN-0003 step 5。`--format text`: 1 行目 `ok`/`error <code>`、worker・model・usage・changes の表、続けて result/error を整形 JSON。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0005-cli-contract-envelope-exit-codes.md`

## Dependencies

- `docs/tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/src/text-format.ts`
- `packages/cli/test/text-format.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] text 形式の各行・表のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter cheaper test -- test/text-format.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- json が既定であることの e2e 検証は TASK-0047。
