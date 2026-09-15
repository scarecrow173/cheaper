---
id: "TASK-0010"
type: "task"
status: "done"
title: "Implement worker resolver with origin precedence"
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
    - "docs/tasks/0006-implement-config-directory-resolution.md"
    - "docs/tasks/0009-implement-worker-definition-validation.md"
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

# Implement worker resolver with origin precedence

## Work

PLAN-0001 step 10。`resolveWorker(name, {cwd, userConfigDir, builtinDir})`: name 検証 → project 存在すれば parse（失敗は即エラー、fallback なし）→ user → builtin → `WORKER_NOT_FOUND`。`--worker-file` 経路: cwd 基準解決（絶対可・workspace 外可）、不在は `WORKER_NOT_FOUND`、stem/name 一致必須、named resolution を行わない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`

## Dependencies

- `docs/tasks/0006-implement-config-directory-resolution.md` — 依存先（Dependency Graph）
- `docs/tasks/0009-implement-worker-definition-validation.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/worker/resolve.ts`
- `packages/core/test/worker/resolve.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 優先順位・no-fallback・--worker-file 独立経路のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/worker/resolve.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- builtin workers の実体は PLAN-0003（TASK-0045）。
