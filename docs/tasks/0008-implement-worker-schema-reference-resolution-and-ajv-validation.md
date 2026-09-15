---
id: "TASK-0008"
type: "task"
status: "done"
title: "Implement worker schema reference resolution and Ajv validation"
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
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/0003-implement-core-error-model-and-exit-code-table.md"
    - "docs/tasks/0007-implement-worker-frontmatter-parser.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement worker schema reference resolution and Ajv validation

## Work

PLAN-0001 step 8。`SchemaRef`（`{file}`|`{inline}` exactly one）解決: file は Worker ファイルのディレクトリ基準、拡張子 .json/.yaml/.yml、欠落は `*_SCHEMA_NOT_FOUND`。`ajv/dist/2020`（strict/allErrors）で `validateSchema`、他 draft の `$schema`・コンパイル失敗は `INVALID_*_SCHEMA`。input/output 両用。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0003-implement-core-error-model-and-exit-code-table.md` — 依存先（Dependency Graph）
- `docs/tasks/0007-implement-worker-frontmatter-parser.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/worker/schema.ts`
- `packages/core/test/worker/schema.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] exactly one・基準解決・拡張子・draft 違反・コンパイル失敗の各テストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/worker/schema.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- schema に対する実行時入力検証は TASK-0009 の validate フローと cli 側。
