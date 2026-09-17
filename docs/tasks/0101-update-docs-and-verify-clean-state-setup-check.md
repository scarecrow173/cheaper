---
id: "TASK-0101"
type: "task"
status: "done"
title: "Update README/AGENTS for canonical contract and verify clean-state setup/check"
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
    - "docs/tasks/0100-align-ci-with-canonical-entrypoints.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0022-repo-owned-setup-check-entrypoints.md"
    - "docs/specs/0017-repo-owned-setup-contract-for-fresh-checkouts.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0017-repo-owned-setup-contract-for-fresh-checkouts.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0101: Update docs and verify clean-state setup/check

## Work

PLAN-0017 の仕上げ task。README・AGENTS.md を canonical contract
に更新し、fresh checkout 相当の clean-state で `mise run setup` →
`mise run check` を実測して結果を記録する。

### 1. `README.md`

セットアップ節を以下の contract に更新する:

- `mise run setup`: fresh checkout を開発可能にする（mise toolchain
  + `pnpm install --frozen-lockfile` + `apm install` + `pnpm build`
  を復元）
- `mise run check`: CI 相当の検証（lint・typecheck・test・bundle
  freshness）
- 個別の `mise install`/`apm install`/`pnpm install` 列挙を
  canonical ではなく内部実装として短く添える形に整理する

### 2. `AGENTS.md`

- セットアップ節を `mise run setup` / `mise run check` に更新
- fresh worktree では既存 `node_modules`/`dist` の存在を前提に
  しないこと、worktree 作成・sibling 探索・生成物流用を Agent の
  責務にしないことを明記する

### 3. clean-state 検証

`git worktree add` で本ブランチの fresh checkout を作り、そこで:

```bash
mise run setup
mise run check
```

を順に実行し、exit 0 を記録する。`node_modules`/`dist`/
`apm_modules`/APM deploy 生成物のない状態からの復元を確認する。
検証後は worktree を `git worktree remove` で削除する。

### 4. status 同期

- SPEC-0017 → `implemented`
- PLAN-0017 → `completed`
- TASK-0099〜0101 → `done`
- 各 index の status を同期

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0017-repo-owned-setup-check-entrypoints.md`
- `docs/designs/0022-repo-owned-setup-check-entrypoints.md`
- `docs/specs/0017-repo-owned-setup-contract-for-fresh-checkouts.md`

## Dependencies

- `docs/tasks/0100-align-ci-with-canonical-entrypoints.md`
  （canonical task と CI が確定している前提）

## Affected Files

- `README.md`
- `AGENTS.md`
- `docs/specs/0017-*.md`・`docs/plans/0017-*.md`・
  `docs/tasks/0099..0101-*.md` の status と各 index

## Done When

- [x] README が `mise run setup` / `mise run check` の contract を
      説明
- [x] AGENTS.md が同じ contract と worktree 前提注意を記載
- [x] clean-state（fresh worktree）で `mise run setup` →
      `mise run check` が exit 0 の実測記録がある
- [x] SPEC-0017/PLAN-0017/TASK-0099..0101 と各 index の status が
      同期されている

## Verification

- [x] clean-state で `mise run setup` が exit 0
- [x] clean-state で `mise run check` が exit 0
- [x] `audit_docs.js --type all` が no findings

## Verification results

2026-09-18 実施（Windows / node v26.8.2 / pnpm 11.0.0 / mise 2026.5.11）。
`git worktree add /tmp/cheaper-fresh HEAD` で作った fresh checkout
（`node_modules`・`dist`・`apm_modules`・APM deploy 生成物なし、
committed state のみ）で実測。

| Command | Result |
|---------|--------|
| `mise trust` | fresh checkout では mise の trust 機構により初回のみ必要（標準挙動） |
| `mise run setup` | exit 0 — `pnpm install --frozen-lockfile`（175 packages、14.9s）→ `apm install`（25 deps を lock 済み SHA で復元、36.5s、cheaper-* の self-deploy なし）→ `pnpm build`（3 packages 全 Done） |
| `mise run check` | exit 0 — `pnpm lint`（warnings のみ）→ `pnpm typecheck` → `pnpm test`（core 270 + runtime-pi 255 + cli 131 = 656 tests 全 pass）→ `node scripts/bundle-cli.mjs` → `git diff --exit-code -- packages/apm/.apm/hooks/cli`（clean） |

検証後 `git worktree remove` 済み。setup 内で apm が repo root の
`.apm/`（self-install）を deploy しないことも合わせて確認
（dev deps のみ integrated）。
