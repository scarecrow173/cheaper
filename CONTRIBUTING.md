# Contributing

`cheaper` のリポジトリ開発に参加するための入口。

## セットアップ

fresh clone / fresh worktree では repository-owned contract として
以下を実行する（`mise.toml` の `[tasks.*]` で定義、ADR-0020）:

```bash
mise run setup   # toolchain + 依存 + build outputs を復元
mise run check   # CI と同等の検証（lint / typecheck / test / bundle freshness）
```

`mise run setup` は `pnpm install --frozen-lockfile` →
`apm install` → `pnpm build` を実行する。toolchain 自体は
`mise.toml` + `mise.lock`（lockfile 優先解決）で再現される。
fresh checkout で `mise` の trust を求められた場合は `mise trust`
を実行する。

`mise run check` は CI と同等の検証を実行する: `pnpm lint` →
`pnpm typecheck` → `pnpm test` → `node scripts/bundle-cli.mjs`
（bundle 再生成）→ `git diff --exit-code -- packages/apm/.apm/hooks/cli`
（bundle freshness）。CLI ソースを変更した場合は bundle 再生成と
そのコミットが必要。

## テスト

```bash
pnpm test            # 全 package の unit + e2e
pnpm lint            # Biome
pnpm typecheck       # tsc -b（project references）
```

e2e の一部は隔離された APM consumer へ `apm install` して実際の
deploy を検証する。

### 実 provider 検証

`packages/cli/test/e2e/real-provider-ollama.test.ts` は Ollama
コンテナとモデルが利用可能なときに限り自動実行され、条件を
満たさない環境では自動 skip される（ADR-0024）。

```bash
docker compose up -d                       # Ollama コンテナ起動
docker compose exec ollama ollama pull qwen3.5:9b   # 既定モデル
curl http://localhost:11434/v1/models      # 稼働確認
pnpm --filter @cheaper/cli test            # 実 provider e2e が実行される
docker compose down                        # 停止（volume・モデルは残る）
```

`CHEAPER_E2E_OLLAMA_MODEL` で検証モデルを上書きできる。

NVIDIA Container Toolkit の無い環境では `docker-compose.yml` の
`deploy:` セクション（GPU パススルー）が原因で `docker compose up`
が失敗する場合がある。その場合は `deploy:` を外すか toolkit を
install する。

## リポジトリ構造

- `packages/cli` — `cheaper` CLI・built-in worker・policy 既定値
- `packages/core` — worker 定義・config・policy・token count 等の
  コアロジック
- `packages/runtime-pi` — Pi runtime adapter（model 解決・auth・
  実行・usage 計測）
- `packages/apm` — distributable APM package（consumer へ deploy
  される skills/hooks/bundled CLI）
- `apm.yml`（root） — development-only の APM project。root の
  `apm install` は cheaper 自身の skill/hook を deploy しない
  （ADR-0019）

## 設計・開発記録

`docs/` は document-driven development の記録を含む。分類と
読み方は [docs/README.md](docs/README.md) を参照。

Coding Agent を開発に使う場合は `AGENTS.md`（エージェント向けの
指示: 環境・検証・境界）を参照する。

## コミット

- branch: `feature/`・`bugfix/`・`hotfix/`・`release/`・`chore/`
  prefix の kebab-case
- commit message: Conventional Commits（`feat:`・`fix:`・`docs:`・
  `chore:` 等）
- commit は論理単位で分割し、無関係な変更を混ぜない
