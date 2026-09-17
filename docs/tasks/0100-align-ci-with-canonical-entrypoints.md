---
id: "TASK-0100"
type: "task"
status: "todo"
title: "Align CI with canonical setup/check entrypoints and locked install"
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
    - "docs/plans/0017-repo-owned-setup-check-entrypoints.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0017-repo-owned-setup-check-entrypoints.md"
    - "docs/tasks/0099-define-mise-setup-check-tasks-and-lockfile-setting.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0022-repo-owned-setup-check-entrypoints.md"
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

# TASK-0100: Align CI with canonical entrypoints

## Work

`.github/workflows/ci.yml` を canonical contract へ揃える
（DESIGN-0022「CI」節）。

### 変更内容

- `jdx/mise-action@v4` の `with:` に `install_args: "--locked"` を
  追加し、locked install を明示する（`mise.lock` 存在時の自動
  `--locked` と同じ効果を action バージョンに関わらず保証）。
- 個別 step（`pnpm install --frozen-lockfile` / `pnpm build` /
  `pnpm lint` / `pnpm typecheck` / `pnpm test` / bundle freshness
  step）を削除し、以下に集約する:

```yaml
- run: mise run setup
- run: mise run check
```

- コメントは必要最小限に留める。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0017-repo-owned-setup-check-entrypoints.md`
- `docs/designs/0022-repo-owned-setup-check-entrypoints.md`

## Dependencies

- `docs/tasks/0099-define-mise-setup-check-tasks-and-lockfile-setting.md`
  （`mise run setup`/`check` の定義が前提）

## Affected Files

- `.github/workflows/ci.yml`

## Done When

- [ ] `install_args: "--locked"` がある
- [ ] 個別 pnpm step と bundle freshness step が canonical task に
      集約されている
- [ ] `mise run setup` / `mise run check` を呼んでいる

## Verification

- [ ] yaml が valid（`pnpm lint` の biome 対象外だが push 後の CI
      で確認。ローカルでは actionlint 相当がなければ diff 目視 +
      worktree 検証の `mise run` 成功で代替）
