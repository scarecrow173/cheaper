# cheaper

Claude Code / OpenAI Codex / GitHub Copilot / Cursor などの親 Coding Agent から、
一部の処理を安価なモデルへ委譲するための汎用 Worker Runtime。

## 目的

frontier model に巨大なコンテキストの読み込みや機械的なコード生成を集中させず、
コストの安いモデルへ処理を逃がすことでトークンコストを削減する。

## セットアップ

```bash
mise install   # ツールチェーン (node / pnpm / python / uv / apm-cli)
apm install    # APM 依存パッケージを各ターゲットへデプロイ
```

## 使い方

```bash
mise exec -- pnpm -r build
echo '{"task": "src/ のエントリポイントを特定して", "paths": ["src/**/*.ts"]}' \
  | node packages/cli/bin/cheaper.js bulk-read
```

- `bulk-read` / `code-write` / `doc-write` は built-in worker（`run <worker>` または `--worker-file` で任意 worker も可）
- stdin に worker の input schema に従う JSON を渡すと、stdout に envelope 1 行（`ok`/`result`/`meta` または `error`）が返る
- `cheaper count-tokens <path>` は LLM を起動せず workspace file の canonical token count（js-tiktoken `o200k_base`）を返す。Worker 内の `count_tokens` tool と同一実装

## Parent agent への配布

配布用 APM package は `packages/apm/` に分離されている（repository root の
`apm.yml` は development-only で、root の `apm install` は cheaper 自身の
skill/hook を deploy しない）。consumer はこの package を依存として
install する:

```yaml
dependencies:
  apm:
  - scarecrow173/cheaper/packages/apm@main   # virtual package 参照
  # または local path: - path: ../cheaper/packages/apm
```

`packages/apm/.apm/` に parent agent 向け primitive を同梱している:

- skills: `cheaper-usage`（CLI contract）、`cheaper-bulk-read` /
  `cheaper-code-write` / `cheaper-doc-write`（各 worker への委譲方法）
- hooks: Windsurf `pre_read_code` で read 前に `count-tokens` を実行し、
  threshold（default 8192、`CHEAPER_READ_TOKEN_THRESHOLD` で上書き可）超過
  の file read を block して bulk-read へ誘導する `pre-read-code.js`
- `packages/apm/.apm/hooks/cli/` は `pnpm bundle` で生成する
  self-contained CLI bundle（esbuild single-file + workers/ +
  policy.yaml）。commit 済みの生成物で、`apm install --target windsurf`
  すると `.windsurf/hooks/<dep-name>/cli/bin/cheaper.mjs` として deploy
  され、hook と parent agent の双方が PATH 不要で CLI を利用できる
- provider / model の設定方法は [docs/guides/providers.md](docs/guides/providers.md) を参照

## ステータス

V1 実装済み。fake stream に加え、Ollama（Docker）を実 provider とした smoke 検証済み。
設計・検証の記録は `docs/`（spec / adr / design / plan / task / impl）を参照。
