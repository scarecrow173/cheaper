# CLI 使い方ガイド

`cheaper` は 1-shot の CLI プロセスとして呼び出す。各 run は stdin
から worker の input schema に従う JSON を読み、stdout に 1 行の
JSON envelope を書いて終了する。

## コマンド構成

```bash
cheaper <command> [options]

Commands:
  run <worker>           Run a named worker (or --worker-file <path>)
  bulk-read              Alias for: run bulk-reader
  code-write             Alias for: run code-writer
  doc-write              Alias for: run doc-writer
  count-tokens <path>    Count canonical tokens of a workspace file
  models                 List available models with tier and pricing info
  auth status [provider] Show stored credentials and provider readiness
  auth login <provider>  Sign in to a provider (OAuth; e.g. openai-codex)
  auth logout <provider> Remove a provider credential
```

`bulk-read` / `code-write` / `doc-write` は `run bulk-reader` 等の
エイリアス。built-in 以外の worker も `run <worker>` で呼べる
（[workers ガイド](./workers.md)参照）。

## 共通 option

| Option | 意味 |
| --- | --- |
| `--input <file\|->` | task input JSON の読み元。`-`（既定）は stdin |
| `--worker-file <path>` | 明示的な worker file を実行する（`run` のみ） |
| `--cwd <path>` | workspace root（既定: カレントディレクトリ） |
| `--timeout <duration>` | policy timeout を narrow する（例: `30s`, `5m`）。policy 上限は越えられない |
| `--model <provider>/<id>` | tier 解決を bypass して model を直接指定する（`run` とエイリアスのみ） |
| `--provider <id>` | `models` の出力を provider で絞る |
| `--tier <low\|medium\|high>` | `models` の出力を effective tier で絞る |
| `--format <json\|text>` | stdout の形式（既定: `json`） |
| `--quiet` | stderr の diagnostics を抑制する |
| `--help` / `--version` | help / version 表示 |

`--method`・`--type` は `auth login` 専用（下記参照）。

## stdin / stdout 契約

- **stdin**: worker の input schema に従う JSON 1 つ（各 worker の
  schema は [workers ガイド](./workers.md)）。
- **stdout**: envelope 1 JSON + 末尾改行 1 つのみ。成功時は
  `{"schemaVersion":1,"ok":true,"worker":"...","result":{...},
  "meta":{...}}`、失敗時は `{"ok":false,"error":{"code":...,`
  `"message":...,"exitCode":...,"retryable":...},"meta":{...}}`。
- `meta` には `durationMs`・`model`（`provider`・`id`・`tier`・
  `selection`（`"tier"` | `"explicit"`）・`tierSource`
  （`"explicit-override"` | `"curated-exception"` | `"auto"` |
  `"unclassified"`））・`usage`（input/output/
  cacheRead/cacheWrite/totalTokens/costUsd）・`turns`・`toolCalls`・
  `repairs`・`changes` 等が入る。`changes` は runtime が計測した
  filesystem 変更の要約。
- stderr には diagnostics（`model: provider/id (tier X)` や error
  要約）が出る。`--quiet` で抑制できる。
- exit code は error.code に対応する（0=成功、2=入力不備、
  3=worker/policy 不備、4=policy 違反、5=provider/model 不備、
  6=実行失敗、7=timeout、8=abort、9=部分書き込み、70=内部）。

## 使用例

```bash
# bulk-read: 大量 read を worker へ委譲
echo '{"task": "src/ のエントリポイントを特定して",
       "paths": ["src/**/*.ts"]}' \
  | cheaper bulk-read --cwd /path/to/workspace

# code-write: targets に定義した file を生成/変更
echo '{"task": "User 型に email フィールドを追加",
       "targets": [{"path": "src/types/user.ts", "intent": "modify"}],
       "references": ["src/types/user.ts"],
       "constraints": ["既存の export 形を維持"]}' \
  | cheaper code-write --cwd /path/to/workspace

# doc-write: decisions/facts から document を生成
echo '{"task": "setup 手順を整理して",
       "documentType": "guide",
       "targets": [{"path": "docs/setup.md", "intent": "create"}],
       "decisions": ["セットアップは install.sh に一本化"]}' \
  | cheaper doc-write --cwd /path/to/workspace

# custom worker file を直接実行
echo '{"task": "..."}' | cheaper run --worker-file ./my-worker.md

# input を file から読む
cheaper bulk-read --input task.json --cwd /path/to/workspace

# model を直接指定して実行（tier 解決を bypass）
echo '{"task": "..."}' | cheaper bulk-read --model anthropic/claude-haiku-4-5
```

`--model <provider>/<id>` は tier 候補選択を bypass するが、
`models.yaml` の `exclude`・policy の `maxTier`・catalog の
availability は通常どおり enforce される（`meta.model.selection`
が `"explicit"` として記録される）。

## models

```bash
cheaper models                        # 利用可能な全 model を一覧
cheaper models --provider anthropic   # provider で絞る
cheaper models --tier low             # effective tier で絞る
```

runtime が認識する available catalog の各 model について、
`excluded`（models.yaml の `exclude` 適用結果）・effective
`tier`・`tierSource`・`pricing`（input/output 価格・`source`
（`runtime-catalog` または `bundled-snapshot`）・snapshot version /
更新日などの freshness）を `result.models[]` で返す。pricing を
持たない model は `pricing: null` になる（error にはしない）。

## count-tokens

```bash
cheaper count-tokens src/huge.ts
# → {"ok":true,"result":{"path":"src/huge.ts","tokens":12345,
#    "bytes":...,"tokenizer":{"name":"o200k_base",...}},"meta":{...}}
```

model 解決・worker 実行を伴わない測定専用コマンド。file が
`CHEAPER_COUNT_MAX_BYTES`（default 16 MiB）を超える場合は
`FILE_TOO_LARGE` で失敗する（巨大 file は測定不能として
fail-closed、ADR-034r05iHrZUE6W82VsDbPq）。worker 内で使える `count_tokens` tool と
同一実装。

## auth

provider credential の確認・login・logout。credential 値自体は
stdout/stderr には出さない。

```bash
cheaper auth status            # 全 provider の auth 状態
cheaper auth status openai-codex  # 単一 provider の絞り込み
cheaper auth login openai-codex   # OAuth login（既定）
cheaper auth login openai-codex --method device  # device code flow
cheaper auth login <provider> --type api_key     # API key を対話入力
cheaper auth logout openai-codex  # credential 削除
```

provider 一覧・認証方法の詳細は [providers ガイド](./providers.md)
を参照。

## 計測 log

`run`（worker alias 含む）と `count-tokens` の実行結果は、
記録対象に限り `<userConfigDir>/usage.jsonl` へ計測 record
（worker・model・usage・duration・success/error）を
best-effort で append する。書き込み失敗は stdout・exit code
に影響しない（ADR-034r05iHrZUE6WljM1ZMiX）。

記録対象は以下に限られる:

- `run`: 成功した run と、error envelope が `meta.model` と
  `meta.usage` を持つ失敗（usage-bearing failure）。worker・
  model 解決前の input/config error など usage を持たない失敗は
  記録されない。
- `count-tokens`: token count が成功し `tokens` を返した実行。
  `FILE_TOO_LARGE`・file not found・workspace escape 等の失敗は
  記録されない。
- `auth`・`--help`・`--version` 等の utility command は記録
  対象外。
