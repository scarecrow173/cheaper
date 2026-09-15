---
id: "TASK-0004"
type: "task"
status: "todo"
title: "Implement duration parser"
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
    - "docs/tasks/0002-create-empty-cheaper-core-package-skeleton.md"
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

# Implement duration parser

## Work

PLAN-0001 step 4。`parseDuration`: `^[1-9][0-9]*(ms|s|m|h)$` → ms 変換の唯一の箇所。不正文字列は `INVALID_REQUEST` 相当のエラーを返す設計（呼び出し側でマップ）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0002-create-empty-cheaper-core-package-skeleton.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/duration.ts`
- `packages/core/test/duration.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] ms/s/m/h 各単位と不正入力のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/duration.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- policy の timeoutMs 正規化利用は TASK-0011 以降。
