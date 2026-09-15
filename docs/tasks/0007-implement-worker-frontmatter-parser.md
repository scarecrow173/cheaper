---
id: "TASK-0007"
type: "task"
status: "done"
title: "Implement worker frontmatter parser"
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

# Implement worker frontmatter parser

## Work

PLAN-0001 step 7。先頭行 `---` から次の `---` までを `yaml` で parse、残りを body。frontmatter 欠落・未閉鎖・YAML エラー・body trim 後空は `INVALID_WORKER_DEFINITION`。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0003-implement-core-error-model-and-exit-code-table.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/worker/parse.ts`
- `packages/core/test/worker/parse.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 正常/異常各ケースのテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/worker/parse.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- frontmatter の field 検証は TASK-0009。
