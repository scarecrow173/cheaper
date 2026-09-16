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
- provider / model の設定方法は [docs/guides/providers.md](docs/guides/providers.md) を参照

## ステータス

V1 実装済み。fake stream に加え、Ollama（Docker）を実 provider とした smoke 検証済み。
設計・検証の記録は `docs/`（spec / adr / design / plan / task / impl）を参照。
