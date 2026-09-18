---
id: "TASK-034r05iHK8bA7gHSXhJFHL"
type: "task"
status: "done"
title: "Implement config directory resolution"
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

# Implement config directory resolution

## Work

PLAN-034r05iHy2gdnpHMZZk4ZU step 6。`resolveConfigDirs({cwd})`: user = `$CHEAPER_CONFIG_DIR` > `$XDG_CONFIG_HOME/cheaper` > `~/.config/cheaper`（`os.homedir()`）、project = `<cwd>/.cheaper`、builtinDir は引数注入。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/scaffold-monorepo-and-implement-cheaper-core.md`
- `docs/designs/worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md`
- `docs/designs/repository-structure-toolchain.md`

## Dependencies

- `docs/tasks/create-empty-cheaper-core-package-skeleton.md` — 依存先（Dependency Graph）

## Affected Files

- `packages/core/src/config/dirs.ts`
- `packages/core/test/config/dirs.test.ts`

## Done When

- [ ] 失敗するテスト（または scaffold 検証コマンド）を書く
- [ ] 対象テストを実行し red を確認する
- [ ] 最小実装を行う
- [ ] 再実行で green を確認する
- [ ] `pnpm biome check` と `tsc -b` が pass
- [ ] 環境変数の優先順位と project/builtin 解決のテストが green

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/core test -- test/config/dirs.test.ts` exits 0
- [ ] `mise exec -- pnpm biome check .` exits 0
- [ ] `mise exec -- pnpm tsc -b` exits 0

## Out of Scope

- policy/models.yaml の実ロードは TASK-034r05iHK8bA7gQBQpBmLM/0018。
