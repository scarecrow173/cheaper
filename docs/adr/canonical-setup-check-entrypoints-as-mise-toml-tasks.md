---
id: "ADR-034r05iHrZUE6WeDAi63F3"
type: "adr"
status: "accepted"
title: "canonical setup/check entrypoint を mise TOML task として mise.toml に定義する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/repo-owned-setup-contract-for-fresh-checkouts.md"
    - "https://github.com/scarecrow173/cheaper/issues/7"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./separate-dev-apm-project-via-nested-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/repo-owned-setup-contract-for-fresh-checkouts.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/repo-owned-setup-contract-for-fresh-checkouts.md"
  derived-by:
    - "../designs/repo-owned-setup-check-entrypoints.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 20. canonical setup/check entrypoint を mise TOML task として mise.toml に定義する

## Context and Problem Statement

SPEC-034r05iHK8bA7gpcCJQya9 は外部基盤が `mise run setup` / `mise run check` だけを
呼べばよい repository-owned contract を要求する。この contract を
どこに・どの形で定義するかが決定事項である。

確認済みの事実は次のとおりである。

- mise の正式な lockfile 機構は `mise.lock` で、存在すれば
  `mise install` は floating spec より locked version を優先する。
  本 repo の `mise.lock` は既に全 5 tool（node・pnpm・python・uv・
  pipx:apm-cli）を lock し Git 管理済み。
- `jdx/mise-action@v4`（≥4.1.0）は repo の `mise.lock` を検出して
  `mise install --locked` を自動実行する。
- mise の task は `mise.toml` の `[tasks.<name>]`（TOML task）または
  `mise-tasks/` 等の standalone script で定義でき、`run` は単一
  command または command 配列を取る。
- 既存 CI は mise-action で toolchain を入れた後、個別に
  `pnpm install` / `pnpm build` / `pnpm lint` / `pnpm typecheck` /
  `pnpm test` / bundle freshness を列挙している。

## Decision Drivers

- 外部基盤が cheaper 固有の内部手順を知る必要がないこと
  （repository-owned contract）。
- Declarative・Deterministic・Reproducible・CI/local parity。
- KISS・YAGNI: 追加 orchestration layer（Makefile/Taskfile）や
  新しい script file を増やさない。
- `setup` と `check` の責務を不必要に重複させない。

## Considered Options

- **Option 1**: `mise.toml` に `[tasks.setup]` / `[tasks.check]` を
  TOML task として定義し、既存 `package.json` scripts / CI の内部
  command を呼び出す形で集約する。
- **Option 2**: `mise-tasks/setup`・`mise-tasks/check` の standalone
  script file を新設する。
- **Option 3**: mise task を持たず、`package.json` scripts
  （`pnpm setup` / `pnpm check`）だけを canonical contract にする。

## Decision Outcome

**Option 1** を採用する。

理由は次のとおり。

- TOML task は declarative で、mise.lock と同じ `mise.toml` という
  単一の repository-owned config に収まる。内部 command の列挙は
  package.json scripts（`pnpm build`/`lint`/`typecheck`/`test`）と
  既存 script（`scripts/bundle-cli.mjs`）を再利用するだけで、新しい
  実装や重複ロジックを持たない（DRY）。
- Option 2 の standalone script は env 差異（Windows の shell）を
  吸収する必要が生じる複雑な処理向きであり、今回の直列 command
  には過剰。新規 file を増やすだけの価値がない。
- Option 3 では mise toolchain（`pnpm` 自体）が未 install の fresh
  checkout で entrypoint が成立しない。`mise run` は mise 管理の
  toolchain 上で task を実行するため bootstrap 順序が自然に保証
  される。
- `setup`/`check` を相互依存させない（`check` に `depends =
  ["setup"]` を付けない）。`check` は「セットアップ済み checkout
  の検証」という責務に留め、責務の重複を避ける。外部基盤は
  `setup` → `check` の順で呼ぶ contract を README/AGENTS.md で
  示す。
- mise の locked mode については、repo `mise.lock` 存在時の
  lockfile 優先解決と mise-action の自動 `--locked` を使い、
  `mise.toml` の `[settings]` に `lockfile = true` を追加して
  lockfile の維持を明示する。CI 側も `install_args: "--locked"` で
  明示的に locked install とする。

### Positive Consequences

- 人間・Coding Agent・CI・外部基盤が同一 entrypoint を利用できる。
- toolchain → Node deps → APM deps → build outputs の復元順序が
  1 箇所に declarative に表現される。

### Negative Consequences

- `check` は `setup` 未実施の checkout では失敗し得る
  （`pnpm test` は `dist/` を必要とするため）。contract として
  `setup` 先行を文書で明示して吸収する。

## Links

- [mise.lock Lockfile](https://mise.jdx.dev/dev-tools/mise-lock.html)
- [TOML-based Tasks](https://mise.en.dev/tasks/toml-tasks)
- [jdx/mise-action — Lock Files](https://github.com/jdx/mise-action#lock-files)
