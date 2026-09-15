---
id: "TASK-0045"
type: "task"
status: "todo"
title: "Add built-in worker definitions"
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
    - "docs/tasks/0044-add-built-in-worker-json-schemas.md"
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

# Add built-in worker definitions

## Work

PLAN-0003 step 7。built-in Worker 定義: bulk-reader（tools read/grep/find/ls、tier low、input/output schema 参照、英語 Markdown 指示: role・手順・出力規則・禁止事項）と cheap-writer（tools + write/edit、tier low、targets 外を書かない指示・unresolved への報告）。changed files は LLM に報告させない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md`
- `docs/designs/0006-bulk-reader-contract.md`
- `docs/designs/0007-cheap-writer-contract.md`
- `docs/specs/0002-bulk-reader-built-in-worker.md`
- `docs/specs/0003-cheap-writer-built-in-worker.md`

## Dependencies

- `docs/tasks/0044-add-built-in-worker-json-schemas.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/cli/workers/bulk-reader.md`
- `packages/cli/workers/cheap-writer.md`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 両 md が core validate を通り tools/tier/schema 参照が設計どおりのテストが green

## Verification

- [ ] `mise exec -- pnpm --filter cheaper test -- test/workers.test.ts` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- 実行検証は e2e（TASK-0048）。
