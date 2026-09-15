# AGENTS.md

## プロジェクト概要

`cheaper` は、親 Coding Agent（Claude Code / Codex / Copilot / Cursor 等）から
処理を安価なモデルへ委譲するための汎用 Worker Runtime。
frontier model のコストを抑えるため、巨大コンテキストの処理や機械的なコード生成を
安価なモデル側で実行する。

## セットアップ

```bash
mise install   # mise.toml のツールチェーンをインストール
apm install    # apm.lock.yaml の SHA に基づき依存を再現インストール
```

