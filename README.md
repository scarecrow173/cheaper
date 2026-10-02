# cheaper

`cheaper` は、親 Coding Agent（Claude Code / Codex / Copilot /
Cursor / Windsurf 等）から機械的・大 context の処理を安価な
モデルへ委譲するための汎用 Worker Runtime。特定の parent agent
や provider に依存しない。

## Why cheaper?

frontier model は高コストなので、巨大な file の読み込みや機械的
なコード生成・document 生成を直接させると token cost が膨らむ。
cheaper はそうした処理を安価なモデル上の worker へ逃がし、
frontier model は判断と統合に集中させる。

```text
親 Coding Agent が巨大 file を読む
        ↓
frontier model が大きな context / token を消費する

cheaper を使うと
        ↓
大きな read・機械的な生成を安価な worker へ委譲する
        ↓
親 agent は判断と統合に集中できる
```

## Core capabilities

- **built-in worker**: `bulk-read`（大規模 read の圧縮・抽出）・
  `code-write`（機械的なコード生成・変更）・`doc-write`
  （document 生成）
- **custom worker**: 自分で定義した worker を `run` で実行できる
- **count-tokens**: workspace file の token 数を LLM なしで測定
- **provider / model 解決**: Pi runtime 経由で任意の provider
  （Ollama・OpenAI 系・その他 OpenAI 互換 endpoint）の model を
  tier で解決
- **parent-agent 連携**: skills / hooks 経由で親 agent から
  委譲できる

## How it works

```text
Parent Coding Agent
        ↓  （task を stdin へ、envelope を stdout から）
     cheaper
        ↓  （worker 定義 + input schema + policy）
   Worker Definition
        ↓
   Runtime / Provider
        ↓
   low-cost model
```

親 agent は `cheaper` を 1-shot CLI として呼ぶ。cheaper が
worker の実行・tool 制約・model 解決・schema 検証を担い、
構造化された result と meta（usage・cost・実行統計）を返す。

## Basic usage

```bash
echo '{"task": "src/ のエントリポイントを特定して",
       "paths": ["src/**/*.ts"]}' \
  | cheaper bulk-read

cheaper code-write < task.json
cheaper doc-write < task.json
cheaper count-tokens src/huge.ts
cheaper auth status openai-codex
```

stdin に worker の input schema に従う JSON を渡すと、stdout に
envelope（`ok`/`result`/`meta` または `error`）1 行が返る。
command・option・exit code の詳細は
[usage ガイド](docs/guides/usage.md) を参照。

## Built-in workers

| worker | 目的 |
| --- | --- |
| `bulk-read` | 大きな file・多数の file を read-only で解析し、圧縮された構造化 findings を返す |
| `code-write` | 機械的・pattern ベースのコード生成・変更を workspace に適用する |
| `doc-write` | decisions/facts から document を生成する |

input schema・tool・write scope 等の詳細は
[workers ガイド](docs/guides/workers.md) を参照。

## Configuration

- **provider / auth**: Pi の `models.json`・credential
  （`cheaper auth` で管理）
- **tier 割当**: `models.yaml`（model → `low`/`medium`/`high`）
- **実行 policy**: `policy.yaml`（timeout・tools・model・output）

user config は `~/.config/cheaper`（`CHEAPER_CONFIG_DIR` で変更
可）、project 単位の override は `<cwd>/.cheaper/`。詳細は
[configuration ガイド](docs/guides/configuration.md) と
[providers ガイド](docs/guides/providers.md) を参照。

## Parent-agent 連携

skills（`cheaper-usage`・各 worker 向け skill）と hooks
（大きな read を threshold で検出して bulk-read へ誘導）を
配布する。hook は deploy 済みの cheaper executable が
`cheaper hook <event> --target <agent>` で処理し、Windsurf /
Copilot / Claude / Cursor で動作する。Node.js は不要
（executable 同梱のため）。Codex では file-read hook が
upstream の hook event 未提供のため shell-read のみ有効
（設計記録参照）。install 方法・target 別挙動・確認手順は
[parent-agent 連携ガイド](docs/guides/parent-agent-integration.md)
を参照。

## Documentation

- [使い方](docs/guides/usage.md) — CLI command・envelope・auth
- [設定](docs/guides/configuration.md) — config dir・models.yaml・
  policy.yaml・環境変数
- [provider / 認証](docs/guides/providers.md) — models.json・
  OAuth・tier 割当
- [workers](docs/guides/workers.md) — built-in worker・custom
  worker・schema・policy
- [parent-agent 連携](docs/guides/parent-agent-integration.md) —
  skills・hooks・target 別挙動
- [Contributing](CONTRIBUTING.md) — リポジトリ開発の setup・
  test・構造
- [docs index](docs/README.md) — 設計・実装・検証記録の索引

## Development

リポジトリ開発の setup・test・構造は
[CONTRIBUTING.md](CONTRIBUTING.md) を参照。Coding Agent を使う
場合は `AGENTS.md`（agent 向け指示）も参照。

## Project status

実験的（experimental）な段階で、CLI contract・worker schema・
配布形式は変更される可能性がある。設計・検証の記録は
[docs/](docs/README.md) を参照。
