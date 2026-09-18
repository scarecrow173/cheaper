---
id: "DESIGN-034r05iHy2gdnoRcfSVFq3"
type: "design"
status: "approved"
title: "Repository-owned setup/check entrypoints via mise TOML tasks"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/repo-owned-setup-contract-for-fresh-checkouts.md"
    - "../adr/canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/repo-owned-setup-contract-for-fresh-checkouts.md"
  implemented-by:
    - "../plans/repo-owned-setup-check-entrypoints.md"
  depends-on:
    - "./dev-apm-project-and-distributable-package-layout.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/repo-owned-setup-contract-for-fresh-checkouts.md"
    - "../adr/canonical-setup-check-entrypoints-as-mise-toml-tasks.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# DESIGN-034r05iHy2gdnoRcfSVFq3: Repository-owned setup/check entrypoints via mise TOML tasks

## Context

ADR-034r05iHrZUE6WeDAi63F3 Option 1 に従い、canonical setup/verification contract を
`mise.toml` の TOML task として定義する。mise toolchain 再現は既存
`mise.lock`（全 5 tool lock 済み・Git 管理済み）と
`lockfile = true` setting の明示で実現し、CI は mise-action の
locked install と canonical task の呼び出しに揃える。

## mise.toml

```toml
[settings]
minimum_release_age = "7d"
lockfile = true          # mise.lock の維持を明示（install/use が
                         # locked version を優先し lockfile を更新）

[env]
UV_EXCLUDE_NEWER = "1 week"

[tools]
...（現行維持: node=26 / pnpm=11 / python=3.12 / uv=0.11 /
     pipx:apm-cli=0.28.0 pin）

[tasks.setup]
description = "Restore Node/APM dependencies and build outputs for a fresh checkout"
run = [
  "pnpm install --frozen-lockfile",
  "apm install",
  "pnpm build",
]

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

設計上の要点:

- `setup` は mise 管理 toolchain 上で実行される前提。`mise run` が
  task 実行前に `mise.toml` の toolset を解決・install するため、
  toolchain 復元は task 内 command として列挙しない（declarative
  toolset + lockfile 優先解決で実現）。
- `check` は `setup` に依存宣言しない。両者は「checkout を開発
  可能にする」「セットアップ済み checkout を検証する」という別
  責務であり、外部は `setup` → `check` の順で呼ぶ contract。
- bundle freshness は CI の現行 step と同一 2 command を `check`
  task の末尾に含め、CI 側の個別 step を廃止して重複を解消する。
- `git diff --exit-code` は Windows 環境でも `git` が PATH にあれば
  動作する（dev machine は git 前提。mise task の shell は cmd で
  あり問題ない）。
- Ollama/Docker・credential 復元・worktree 操作は task に含めない。

## CI（`.github/workflows/ci.yml`）

```yaml
- uses: jdx/mise-action@v4
  with:
    minimum_release_age: 7d
    install_args: "--locked"

- run: mise run setup
- run: mise run check
```

- `install_args: "--locked"` は mise.lock 存在時の自動
  `--locked`（v4.1.0+）と同じ効果を、action の新旧に関わらず明示
  的に保証する。
- 個別の `pnpm install/build/lint/typecheck/test` step と
  bundle freshness step は canonical task へ集約して削除する。
- CI 上の `git diff --exit-code` は checkout 直後で user.name 等
  未設定でも動作する（diff のみ）。

## README / AGENTS.md

- README セットアップ節: `mise run setup`（toolchain・Node deps・
  APM deps・build outputs を復元）と `mise run check`（CI 相当の
  検証）の 2 行を canonical contract として記述し、内部 command
  の内訳を短く添える。`mise install`/`apm install`/`pnpm install`
  の個別列挙は canonical ではなくなった旨を反映する。
- AGENTS.md セットアップ節: 同じ contract を記載し、fresh
  worktree では既存の `node_modules`/`dist` の存在を前提にしない
  こと、worktree 作成・sibling 探索・生成物流用を Agent の責務に
  しないことを明記する。

## File Map

| File | Change |
| --- | --- |
| `mise.toml` | `[settings]` に `lockfile = true`、`[tasks.setup]` / `[tasks.check]` を追加 |
| `.github/workflows/ci.yml` | `install_args: "--locked"` + `mise run setup` / `mise run check` へ集約 |
| `README.md` | セットアップ節を canonical contract に更新 |
| `AGENTS.md` | セットアップ節を canonical contract + worktree 前提注意に更新 |

## Risks and Trade-offs

- **`mise run` の toolset 自動 install**: `mise run` は task 実行前に
  toolset を解決するが、環境によっては `MISE_AUTO_INSTALL` が
  off の場合 tool 不在で失敗し得る。その場合はユーザーが
  `mise install` を先に実行する必要がある — README に setup が
  toolchain install も兼ねる旨ではなく「mise が toolchain を
  用意した上で task が走る」契約であることを正確に書く。
- **check の非冪等的時間コスト**: `pnpm test` は e2e を含み数十秒。
  CI 相当の canonical contract として許容（issue 要件）。
- **apm install の network 依存**: fresh setup では APM remote dep
  の取得が必要（`apm.lock.yaml` の SHA により再現）。これは既存
  contract であり変更しない。
- **lockfile drift**: `mise install`/`mise use` が `mise.lock` を
  更新し得る。CI の `--locked` で drift が顕在化した場合は
  `mise lock` で更新して commit する運用（README に触れる程度）。

## Verification Notes

- clean-state 検証は `git worktree add` による fresh checkout で
  実施する（committed state のみで `node_modules`/`dist`/
  `apm_modules`/deploy 生成物がない状態）。その worktree で
  `mise run setup` → `mise run check` を実行し exit 0 を記録する。
- `pnpm test` は hermetic（user config 非依存）を維持することを
  既存 suite で確認済み。provider credential は setup に含めない。
