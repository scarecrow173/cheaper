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
  tool-operate           Alias for: run tool-operator
  count-tokens <path>    Count canonical tokens of a workspace file
  workflow interface <path> Show the public workflow inputs and outputs
  workflow run <path>       Run a one-shot workflow
  usage                  Aggregate usage.jsonl (worker cost / measured savings)
  models                 List available models with tier and pricing info
  auth status [provider] Show stored credentials and provider readiness
  auth login <provider>  Sign in to a provider (OAuth; e.g. openai-codex)
  auth logout <provider> Remove a provider credential
  hook <event>           Agent hook entry (internal; --target <agent>)
```

`bulk-read` / `code-write` / `doc-write` / `tool-operate` は
`run bulk-reader` 等のエイリアス。built-in 以外の worker も `run <worker>` で呼べる
（[workers ガイド](./workers.md)参照）。`workflow` は tool call
と worker 実行を宣言的な DAG へ束ねて 1 回で実行する系で、
使い分けの指針は [workflow ガイド](./workflow-usage.md) を参照。
`hook` は parent agent の hook event から呼ばれる内部 entry で、
直接利用は想定しない（[parent-agent 連携ガイド](./parent-agent-integration.md)）。

## 共通 option

| Option | 意味 |
| --- | --- |
| `--input <file\|->` | task input JSON の読み元。`-`（既定）は stdin |
| `--worker-file <path>` | 明示的な worker file を実行する（`run` のみ） |
| `--cwd <path>` | workspace root（既定: カレントディレクトリ） |
| `--timeout <duration>` | policy timeout を narrow する（例: `30s`, `5m`）。policy 上限は越えられない |
| `--model <provider>/<id>` | `run` では tier 解決を bypass する model 直接指定、`usage` では model filter |
| `--provider <id>` | `models` の出力・`usage` の集計を provider で絞る |
| `--tier <low\|medium\|high>` | `models` の出力を effective tier で絞る |
| `--worker <name>` | `models` の auto tier 分類をその worker の workload profile で計算する |
| `--format <json\|text>` | stdout の形式（既定: `json`） |
| `--quiet` | stderr の diagnostics を抑制する |
| `--log <level>` | `.cheaper/logs` + stderr への logging を有効化（`off`/`error`/`warn`/`info`/`debug`/`trace`、env `CHEAPER_LOG`） |
| `--log-format <text\|json>` | ログ出力形式（env `CHEAPER_LOG_FORMAT`） |
| `--help` / `--version` | help / version 表示 |

`--method`・`--type` は `auth login` 専用（下記参照）。
`--since`・`--worker`・`--compare-model` は `usage` 専用（後述）。

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
  `"unclassified"`）。`tierSource === "auto"` のときは分類根拠の
  `effectiveCost`・`effectiveCostPercentile`・
  `workload.outputPerInput` も入る）・`usage`（input/output/
  cacheRead/cacheWrite/totalTokens/costUsd）・`turns`・`toolCalls`・
  `repairs`・`changes` 等が入る。`changes` は runtime が計測した
  filesystem 変更の要約。`savings` は親コンテキスト offload の実測
  （`sourceTokens` / `returnedTokens` / `avoidedContextTokens`、
  後述の「実測 savings」）。
  - `usage` は provider 報告値の passthrough。prompt cache を
    報告する provider（devin 等）では `input` は uncached 分
    のみで、cache-hit 分は `cacheRead` に入る — prompt 全体量は
    `input + cacheRead` で読む。`totalTokens` は `input +
    output` で `cacheRead` を含まない（issue #86）。
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

# tool-operate: 操作を反復させて evidence 付きの結果を取得
echo '{"task": "テスト suite を実行して結果を報告して",
       "context": ["named task \"test\" が使える環境"],
       "constraints": ["新規 install はしない"]}' \
  | cheaper tool-operate --cwd /path/to/workspace

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
cheaper models --worker bulk-reader   # bulk-reader の workload で auto 分類
```

runtime が認識する available catalog の各 model について、
`excluded`（models.yaml の `exclude` 適用結果）・effective
`tier`・`tierSource`・`pricing`（input/output 価格・`source`
（`runtime-catalog` または `bundled-snapshot`）・snapshot version /
更新日などの freshness）を `result.models[]` で返す。pricing を
持たない model は `pricing: null` になる（error にはしない）。
`tierSource` が `"auto"` の model には分類根拠として
`effectiveCost`・`effectiveCostPercentile`・
`workload.outputPerInput` も付く。`--worker` を指定するとその
worker の `cheaper.model.workload.outputPerInput` を ranking の
基準に使う（省略時は既定 `1.0`）。

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
- `auth`・`usage`・`--help`・`--version` 等の utility command は
  記録対象外。
- `hook`（agent hook 経路）は token 計測を行っても記録対象外。
  hook 呼び出しは高頻度かつ infra 的な性質のため、telemetry には
  載せない設計としている。

run record には加えて `turns`・`toolCalls`・`savings`（実測
offload。`directlyComparable` flag を含む）が載る。いずれも
optional で、古い cheaper が書いた record には存在しない
（`savings` 内の `directlyComparable` も同様）。count record
には `savings` はない（offload の概念がないため）。

record schema は additive-only で、新旧の record は同一
usage.jsonl に混在しうる。旧 writer の record（上記 optional
field なし）も新しい `cheaper usage` がそのまま集計でき、
逆に新 writer の record も旧 `cheaper usage` が既知 field
のみ読んで集計できる（未知 field は無視される）。ただし
`usage.costUsd` の算出元は新旧で異なる（後述）ため、混在
file の costUsd 合計は「同じ算出式の値の和」ではない点に
注意。

## 実測 savings（actual / measured / estimated の区別）

指標は次の 3 種に分かれる。混同しないこと。

- **actual（実測）**: `meta.usage` / record の `usage`。
  worker 実行そのものが消費した token と実コスト（`costUsd`）。
  `costUsd` は provider が報告する cost ではなく、cheaper の
  pricing layer が run 時に解決した価格（`resolvePricing`:
  runtime catalog → bundled snapshot）× token 数から算出する。
  価格を解決できない model は `0` になる。旧 writer の
  record に入っている `costUsd` は provider 報告値の合算
  だったため、新旧 record で算出元が異なる（値がわずかに
  ずれうる）。
- **measured（実測 offload）**: `savings.sourceTokens` は worker
  が成功した tool call の出力テキストを canonical tokenizer
  （o200k_base）で encode した和。`savings.returnedTokens` は
  返却 result JSON の token 数。`savings.avoidedContextTokens =
  max(0, sourceTokens - returnedTokens)` は「親コンテキストに
  戻らなかった token 量」の近似。失敗 run は `returnedTokens:
  0`（途中まで offload した実測として全量計上）。
- **estimated（推定）**: `cheaper usage --compare-model` で算出
  される `estimated*` 系値のみ。実測ではなく comparison model
  の価格に基づく概算。

限界:

- `sourceTokens` は tool output の和であり、grep の file 一覧や
  error text も含む。親が同じ量を消費したとは限らない近似値。
- `savings.directlyComparable` は「source↔returned が親 model の
  直接実行と比較可能か」の flag。`write`/`edit` tool または
  `writeScope` を持つ mutation 系 worker（code-writer /
  doc-writer 等）では返却 result が ingest 内容の圧縮表現に
  ならないため `false` になる。`process` / `command` を宣言する
  worker（tool-operator）も mutating tool 判定のため常に
  `false` — 判定は worker の **宣言** tools を見る保守的なもの
  であり、`cheaper.optionalTools` で縮退して実際には実行
  しなかった run でも `false` のままになる（child process の
  副作用が観測不能なため estimated 母数に入れない）。
  `savings` 自体は全 worker で
  計測されるが、estimated の母数には `directlyComparable: true`
  の record のみ入る（flag を持たない旧 record は非 comparable
  扱い）。
- estimated 値は comparison model と pricing metadata が揃った
  ときだけ出る。価格不明の場合に推定値を捏造しない。

## usage（集計コマンド）

`usage.jsonl` を read-only で集計する。`usage` 実行自体は
記録されない。

```bash
cheaper usage                                   # 全 record を集計
cheaper usage --since 24h                       # 期間 filter
cheaper usage --worker bulk-reader              # worker filter
cheaper usage --provider ollama                 # provider filter
cheaper usage --model ollama/qwen3.5:9b         # model filter
cheaper usage --compare-model openai/gpt-5      # estimated 算出
cheaper usage --format text                     # 人間向け表示
```

集計対象は `kind:"run"` record のみ。usage.jsonl が無い・破損行
が混じっていてもエラーにならない（破損行は skip、file 不在は
0 件集計）。`result` には `records`（件数・ok/errors 内訳）・
`usage` 合計・`turns`/`toolCalls`/`durationMs` 合計・`measured`
（savings を持つ record のみを母数とする実測 offload）が入る。

### estimated savings

`--compare-model <provider>/<id>` を指定すると、core の pricing
layer（`resolvePricing`）で comparison model の価格を解決する。
参照順は次の通り:

1. runtime catalog の `model.cost`（`source: "runtime-catalog"`。
   catalog に対象 model があれば常に最優先）
2. `<userConfigDir>/pricing.json`（core `PricingSnapshot` と同一
   形式の user 提供 snapshot。`source: "pricing.json"`）
3. bundled snapshot（`source: "bundled-snapshot"`）

次のすべてが揃ったときのみ `estimated` section を出力する:

1. `--compare-model` が指定されている
2. 上記のいずれかで対象 model の価格が解決できる
3. 価格の `updatedAt` が 30 日以内（それ以上古い・parse 不可は
   stale。`updatedAt` を持たない runtime-catalog 価格は stale に
   しない）
4. `savings.directlyComparable: true` の run が 1 件以上ある
   （write 系 worker のみでは算出しない）

算出式は `estimatedParentEquivalentCostUsd = (comparable.sourceTokens ×
input価格 + comparable.returnedTokens × output価格) / 1,000,000`
（親 model で同量を直接処理した場合の概算）、`estimatedSavingsUsd =
parentEquivalent - comparable run の実 worker costUsd 合計`。
`--format text` では `Estimated` prefix の行として出る。

`pricing.json` の形式は bundled snapshot と同じ:

```json
{
  "version": "YYYY-MM-DD",
  "updatedAt": "YYYY-MM-DD",
  "models": {
    "<provider>/<id>": {
      "input": 3.0,
      "output": 15.0,
      "cacheRead": 0.3,
      "cacheWrite": 3.75,
      "updatedAt": "YYYY-MM-DD"
    }
  }
}
```

`cacheRead`・`cacheWrite`・entry の `updatedAt` は省略可。不正な
entry は skip され、file 全体が不正・不在の場合は bundled
snapshot へ fall back する。価格単位は USD / 1M tokens 固定。
