---
id: "TASK-0099"
type: "task"
status: "done"
title: "Define mise setup/check TOML tasks and lockfile setting"
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

# TASK-0099: Define mise setup/check TOML tasks

## Work

DESIGN-0022「mise.toml」節の通り、`mise.toml` に canonical
entrypoint を定義する。

### 1. `[settings]` に `lockfile = true` を追加

`minimum_release_age = "7d"` の直下に `lockfile = true` を追加し、
`mise.lock` の維持を明示する（コメントは日本語で簡潔に）。

### 2. `[tasks.setup]` を定義

```toml
[tasks.setup]
description = "Restore Node/APM dependencies and build outputs for a fresh checkout"
run = [
  "pnpm install --frozen-lockfile",
  "apm install",
  "pnpm build",
]
```

### 3. `[tasks.check]` を定義

```toml
[tasks.check]
description = "Verify the checkout: lint, typecheck, test, bundle freshness"
run = [
  "pnpm lint",
  "pnpm typecheck",
  "pnpm test",
  "node scripts/bundle-cli.mjs",
  "git diff --exit-code -- packages/apm/.apm/hooks/cli",
]
```

`check` に `depends = ["setup"]` は付けない（責務分離。ADR-0020）。

### 4. 動作確認

- `mise tasks` で `setup` / `check` が一覧されること
- `mise run setup` が exit 0（現在の checkout は既に setup 済みの
  ため冪等確認となる）
- `mise run check` は TASK-0101 の clean-state 検証で実施するが、
  必要ならここでも軽く確認する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0017-repo-owned-setup-check-entrypoints.md`
- `docs/designs/0022-repo-owned-setup-check-entrypoints.md`

## Dependencies

- なし（PLAN-0017 の先頭 task）

## Affected Files

- `mise.toml`

変更しない file（明示）: `mise.lock`（既存 lock を維持）、
`package.json` scripts、`.github/workflows/ci.yml`（TASK-0100）、
README/AGENTS.md（TASK-0101）。

## Done When

- [x] `[settings]` に `lockfile = true` がある
- [x] `[tasks.setup]` が `pnpm install --frozen-lockfile` /
      `apm install` / `pnpm build` を含む
- [x] `[tasks.check]` が lint/typecheck/test/bundle freshness を
      含み `depends` を持たない
- [x] `mise tasks` に両 task が表示され `mise run setup` が exit 0

## Verification

- [x] `mise tasks` で一覧確認
- [x] `mise run setup` が exit 0
