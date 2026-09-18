---
id: "TASK-034r05iHQeQpDppVaGtCgu"
type: "task"
status: "done"
title: "Add built-in worker JSON schemas"
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

# Add built-in worker JSON schemas

## Work

PLAN-034r05iHy2gdnpJFiMWhkY step 6。DESIGN-034r05iHrZUE6WxkSJi9GF/0007 の表を JSON Schema 2020-12 に落とす 4 ファイル。path 共通 pattern は `$defs`（`^(?!/)(?![A-Za-z]:)(?!\.\.(/|$))(?!.*/\.\.(/|$)).+$`）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/bulk-reader-contract.md`
- `docs/designs/cheap-writer-contract.md`
- `docs/specs/bulk-reader-built-in-worker.md`
- `docs/specs/cheap-writer-built-in-worker.md`

## Dependencies

- `docs/tasks/scaffold-cheaper-cli-package-with-bin-entry.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/workers/schemas/bulk-reader.input.schema.json`
- `packages/cli/workers/schemas/bulk-reader.output.schema.json`
- `packages/cli/workers/schemas/cheap-writer.input.schema.json`
- `packages/cli/workers/schemas/cheap-writer.output.schema.json`

## Done When

- [x] 失敗するテスト（または scaffold 検証コマンド）を書く
- [x] 対象テストを実行し red を確認する
- [x] 最小実装を行う
- [x] 再実行で green を確認する
- [x] `pnpm biome check` と `tsc -b` が pass
- [x] 4 schema が Ajv `validateSchema`（draft 2020-12）を通るテストが green

## Verification

- [x] `mise exec -- pnpm --filter cheaper test -- test/workers.test.ts` exits 0
- [x] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- worker md 本体は TASK-034r05iHQeQpDpq6zeHAT0。
