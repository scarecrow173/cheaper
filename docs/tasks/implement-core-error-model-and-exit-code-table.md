---
id: "TASK-034r05iHK8bA7gEdDgIUDn"
type: "task"
status: "done"
title: "Implement core error model and exit code table"
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
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  implemented-by: []
  depends-on:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
    - "docs/tasks/create-empty-cheaper-core-package-skeleton.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/scaffold-monorepo-and-implement-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement core error model and exit code table

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 3。`CheaperError { code, message, exitCode, retryable, details?, cause? }` と code→exitCode 対応表を DESIGN-034r05iHrZUE6WrdtQIOmE の表どおり実装。retryable は PROVIDER_FAILURE/TIMEOUT/MODEL_TIER_UNAVAILABLE のみ true。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/create-empty-cheaper-core-package-skeleton.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/errors.ts`
- `packages/core/test/errors.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 全 error code が対応表に存在し exitCode/retryable が DESIGN-034r05iHrZUE6WrdtQIOmE と一致するテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/errors.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- envelope への変換は packages/cli（PLAN-034r05iHy2gdnpJFiMWhkY）。
