---
id: "SPEC-034r05iHK8bA7gpcCJQya9"
type: "spec"
status: "implemented"
title: "Repository-owned setup contract for fresh checkouts/worktrees"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/apm-self-contained-cli-distribution.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/repo-owned-setup-check-entrypoints.md"
  depends-on:
    - "./separate-dev-apm-project-from-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/separate-dev-apm-project-via-nested-distributable-package.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# SPEC-034r05iHK8bA7gpcCJQya9: Repository-owned setup contract for fresh checkouts

## Problem

fresh clone / Git worktree から開発を開始する際、`node_modules/`・
`dist/`・`apm_modules/`・target への deploy 生成物は Git 管理外で
存在しない。外部の開発実行基盤（CI・Coding Agent・worktree 管理
基盤）が cheaper 固有の内部手順（`pnpm install → apm install →
pnpm build`）を知らなくても、repository-owned の canonical
entrypoint を呼ぶだけで再現可能に開発可能状態へ到達できる必要が
ある。現状は canonical entrypoint がなく、README/AGENTS.md の
セットアップ説明には CI が実際に行う package dependency install
（`pnpm install`）が抜けている不整合もある。

## Requirements

- `mise run setup` を canonical setup entrypoint とし、mise.lock に
  基づく toolchain、Node.js dependencies（`pnpm install
  --frozen-lockfile`）、APM dependencies（`apm install`）、
  TypeScript build outputs（`pnpm build`）を復元する
- `mise run check` を canonical verification entrypoint とし、
  `pnpm lint` / `pnpm typecheck` / `pnpm test` と bundle freshness
  （`node scripts/bundle-cli.mjs` + `git diff --exit-code --
  packages/apm/.apm/hooks/cli`）を実行する
- `mise.toml` + `mise.lock` の組み合わせで toolchain を再現し、
  floating spec だけに依存しない。`mise.lock` は Git 管理を継続し、
  CI でも lockfile を尊重する
- `pnpm-lock.yaml`・`apm.lock.yaml` を canonical lockfile として
  fresh checkout で変更なく復元する
- CI が同じ repository-level contract（`mise run setup` /
  `mise run check`）を利用する
- README / AGENTS.md が同じ contract を参照する
- setup は「既に存在する checkout を、その checkout 単体で開発
  可能にする」ことだけを担当する

## Non-Goals

- worktree の作成・削除・lifecycle 管理、sibling/main worktree
  探索、branch lifecycle 管理
- worktree 間での `node_modules` / `dist` / APM generated artifact
  の共有や symlink
- Makefile/Taskfile 等の追加 orchestration layer、Dev Container、
  Nix、Docker の通常開発必須化
- user credential / provider config の repository 内管理や複製
- `docker compose up`（Ollama）を通常 setup に含めること

## Acceptance Criteria

- [ ] `mise run setup` が fresh checkout / worktree の canonical
      setup entrypoint として `mise.toml` に定義されている
- [ ] `mise run setup` 完了後に通常の開発・テストへ移れる
- [ ] `mise run check` が CI 相当の canonical verification
      entrypoint として定義されている
- [ ] `mise.lock` により toolchain が再現可能（locked mode /
      lockfile 優先解決）
- [ ] package dependencies が `pnpm-lock.yaml` から frozen で再現
      可能
- [ ] APM dependencies が `apm.lock.yaml` から再現可能
- [ ] CI が `mise run setup` / `mise run check`（canonical
      entrypoint）を利用し lockfile を尊重する
- [ ] README / AGENTS.md が同じ contract を参照する
- [ ] worktree lifecycle 管理・orchestration は実装していない
- [ ] Docker/Ollama が通常 setup の必須 dependency になっていない
- [ ] clean-state（node_modules / dist / APM deploy なし）から
      `mise run setup` → `mise run check` が成功する実測記録がある
- [ ] user credential / provider config を setup に組み込んでいない
      （test は hermetic を維持）
