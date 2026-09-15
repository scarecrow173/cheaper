---
id: "TASK-0044"
type: "task"
status: "todo"
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

# Add built-in worker JSON schemas

## Work

PLAN-0003 step 6。DESIGN-0006/0007 の表を JSON Schema 2020-12 に落とす 4 ファイル。path 共通 pattern は `$defs`（`^(?!/)(?![A-Za-z]:)(?!\.\.(/|$))(?!.*/\.\.(/|$)).+$`）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0006-bulk-reader-contract.md`
- `docs/designs/0007-cheap-writer-contract.md`
- `docs/specs/0002-bulk-reader-built-in-worker.md`
- `docs/specs/0003-cheap-writer-built-in-worker.md`

## Dependencies

- `docs/tasks/0039-scaffold-cheaper-cli-package-with-bin-entry.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/workers/schemas/bulk-reader.input.schema.json`
- `packages/cli/workers/schemas/bulk-reader.output.schema.json`
- `packages/cli/workers/schemas/cheap-writer.input.schema.json`
- `packages/cli/workers/schemas/cheap-writer.output.schema.json`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 4 schema が Ajv `validateSchema`（draft 2020-12）を通るテストが green

## Verification

- [ ] `mise exec -- pnpm --filter cheaper test -- test/workers.test.ts` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- worker md 本体は TASK-0045。
