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

## ステータス

開発初期（設計フェーズ）。実装はこれから。
