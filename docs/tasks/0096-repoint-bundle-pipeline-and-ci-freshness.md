---
id: "TASK-0096"
type: "task"
status: "done"
title: "Repoint bundle pipeline and CI freshness check to packages/apm"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/0016-dev-apm-and-distributable-package-separation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0016-dev-apm-and-distributable-package-separation.md"
    - "docs/tasks/0095-move-apm-primitives-to-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0021-dev-apm-project-and-distributable-package-layout.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0096: Repoint bundle pipeline and CI freshness check

## Work

bundled CLI の生成先と CI の freshness check を新 distribution
package path（`packages/apm/.apm/hooks/cli`）へ追従させる。

### 1. `scripts/bundle-cli.mjs`

- `outDir` を `path.join(repoRoot, "packages", "apm", ".apm",
  "hooks", "cli")` に変更する。
- `outDir` を参照するコメント・ログ出力を新 path に合わせて更新する。
- 生成物構成（`bin/cheaper.mjs`・`version.json`・`workers/`・
  `policy.yaml`）と delete+recreate の挙動は変更しない。

### 2. bundle 再生成

```bash
mise exec -- node scripts/bundle-cli.mjs
```

`packages/apm/.apm/hooks/cli/` へ生成されることを確認する。
TASK-0095 の `git mv` で既に同内容が配置されているため、生成差分は
原則ゼロのはず（非決定的な差分が出た場合は再生成物を採用して
commit する）。

### 3. `.github/workflows/ci.yml`

bundle freshness check の `git diff --exit-code -- .apm/hooks/cli`
を `git diff --exit-code -- packages/apm/.apm/hooks/cli` へ変更する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0016-dev-apm-and-distributable-package-separation.md`
- `docs/designs/0021-dev-apm-project-and-distributable-package-layout.md`

## Dependencies

- `docs/tasks/0095-move-apm-primitives-to-distributable-package.md`
  （移動後の path が前提）

## Affected Files

- `scripts/bundle-cli.mjs`
- `.github/workflows/ci.yml`
- `packages/apm/.apm/hooks/cli/`（再生成差分がある場合のみ）

## Done When

- [x] `bundle-cli.mjs` が `packages/apm/.apm/hooks/cli` へ出力する
- [x] bundle 再生成が exit 0 で、新 path に生成物がある
- [x] CI freshness check が新 path を `git diff` 対象にしている

## Verification

- [x] `mise exec -- node scripts/bundle-cli.mjs` が exit 0
- [x] `git diff --exit-code -- packages/apm/.apm/hooks/cli` が
      再生成後に clean（差分があれば commit 済み）
