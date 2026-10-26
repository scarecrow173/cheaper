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
  policy grant add <name> <json>  Add a command grant to user policy
  policy grant list          List command grants in user policy
  policy grant remove <name> Remove a command grant from user policy
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
| `--grant <json>` | その invocation のみ有効な command grant を注入する（repeatable。`run`・`workflow run` のみ。詳細は [command ガイド](./command.md)） |
| `--cwd <path>` | workspace root（既定: カレントディレクトリ） |
| `--timeout <duration>` | policy timeout を narrow する（例: `30s`, `5m`）。policy 上限は越えられない。`run` では進捗でリセットされる stall window、`workflow run` では全体の絶対期限として解釈される（詳細は [configuration ガイド](./configuration.md)） |
| `--resume <path>` | budget 枯渇（`MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED`）時に保存された checkpoint から run を再開する（`run`・worker alias のみ） |
| `--model <provider>/<id>` | `run` では tier 解決を bypass する model 直接指定、`usage` では model filter |
| `--provider <id>` | `models` の出力・`usage` の集計を provider で絞る |
| `--tier <low\|medium\|high>` | `models` の出力を effective tier で絞る |
| `--worker <name>` | `usage` では worker filter、`models` では auto tier 分類をその worker の workload profile で計算する |
| `--format <json\|text>` | stdout の形式（既定: `json`） |
| `--quiet` | stderr の diagnostics を抑制する |
| `--log <level>` | `.cheaper/logs` + stderr への logging を有効化（`off`/`error`/`warn`/`info`/`debug`/`trace`、env `CHEAPER_LOG`） |
| `--log-format <text\|json>` | ログ出力形式（env `CHEAPER_LOG_FORMAT`） |
| `--help` / `--version` | help / version 表示 |

`--method`・`--type` は `auth login` 専用（下記参照）。
`--since`・`--compare-model` は `usage` 専用（後述）。

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
  要約）が出る。`--quiet` で抑制できる。`--grant` 付きの run は
  開始時に `grant: <names>` の diagnostic 行を出す — invocation
  grant の適用を policy 由来のみの run と区別する audit surface
  の一面で、もう一面は log 有効時の `cheaper.run.tool_scope`
  span の `grants.invocation` field（`usage.jsonl` の record
  shape は不変）。
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

## policy grant（command grant の管理）

user-scope `<userConfigDir>/policy.yaml` の `command.grants` を
管理する utility command。対象は user policy のみで、project
policy（`.cheaper/policy.yaml`）は触れない。

```bash
cheaper policy grant add git-read '{"executable":"git","argv":["log","*"]}'
cheaper policy grant list        # {grants:[{name,executable}]}（name 順）
cheaper policy grant remove git-read
```

- `add <name> <json>`: spec は `command.grants` の map value shape
  の JSON object（`name` key は含めない — map key として載る）。
  書き込み前に policy load と同一の検査（schema・deny list
  （opt-in — `security.execDenyPatterns` 宣言時のみ）・
  argv pattern の compile・env/cwd 規則・grant/task 名衝突）を
  通し、invalid spec は file を変更せず error を返す。`fixed:true`
  ・明示 `scope` を含む spec は CLI 入口規約として `InvalidRequest`
  で拒否される（`policy grant` は `!fixed` command grant のみ登録
  する契約 — `fixed:false`・`inputs`/`outputs` の宣言は受理）。
  executable の存在確認は add 時に
  は行わない（pin は scope 構築時の runtime 検査）。既存の同名
  grant がある場合は `InvalidRequest`（置き換えは `remove` →
  `add`）。`process.tasks` の key と衝突する name は unified
  `exec` namespace の規約として `InvalidRuntimePolicy`
  （`details.{grant,task}`）で拒否される。
- `remove <name>`: 存在しない name は `InvalidRequest` の error
  envelope を返す。
- write path は atomic（temp + rename、POSIX `0600`）。caveat:
  read-modify-write のため並行する `add`/`remove` や手編集が
  割り込むと last-wins で lost update しうる。また serde_yml
  rewrite が file 内の comment・key 順・整形を保存しない —
  hand-editing と併用する場合は注意。
- spec-json は cmdline に一時的に載る（`--grant` flag と同種の
  露出 caveat — secret は spec に含めず、grant `env` の値として
  file 側に書く。詳細は [command ガイド](./command.md) の
  cmdline 露出と secrets）。
- utility command のため `usage.jsonl` には記録されず、success
  envelope の `result` は `{grant:{name,executable}}`（`list` は
  `{grants:[...]}`）で `tokens` key を含まない。

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
- `auth`・`usage`・`policy`・`--help`・`--version` 等の utility
  command は記録対象外。
- `hook`（agent hook 経路）は token 計測を行っても記録対象外。
  hook 呼び出しは高頻度かつ infra 的な性質のため、telemetry には
  載せない設計としている。

run record には加えて `version`（run を実行した binary の
product version — workspace `Cargo.toml` の canonical version。
新旧版の識別用）・`turns`・`toolCalls`・`savings`（実測
offload。`directlyComparable` flag を含む）・`repairs`
（INVALID_OUTPUT の repair 消費数。non-null のみ）・
`execution`（run に適用された**実効** policy echo
`{timeoutMs, maxTurns, maxToolCalls}` — `--timeout` narrow・
policy 層 merge を通った解決後の値。`maxTurns` は `null`
許容で、これは「未設定」ではなく policy 上の turn 上限なしを
表す）が載る。なお record の `execution` は policy echo で
あり、envelope `meta.execution`（`ExecutionMetrics` = 観測
metrics。record には載らない）とは同名だが別概念 — 混同
しないこと。いずれも optional で、古い cheaper が書いた
record には存在しない（`savings` 内の `directlyComparable`
も同様）。count record には `savings`・`execution` はない
（offload・policy echo の概念がないため）。

error record（usage-bearing failure）の `error` object は
`code` に加えて、条件に合うものだけ以下を持つ（allowlist
固定・この順）:

- `reason` — abort 語彙（`timeout`・`signal`・`maxTurns`・
  `maxToolCalls`）に一致する `error.details.reason` のみ。
- `message` — `error.message` の先頭 300 文字。
- `schemaErrors` — `error.details.errors` の先頭 5 件を
  `{instancePath ≤200 文字, message ≤120 文字}` で写す
  （欠落 key はその key を省略）。
- `errorMessage` — `error.details.errorMessage` の先頭
  300 文字（PROVIDER_FAILURE の下位分類用）。
- `checkpoint` — `error.details.checkpoint` の
  `{path, observationsCount}` のみ。budget 枯渇（`MAX_TURNS_`/
  `MAX_TOOL_CALLS_EXCEEDED`）時の resume 参照。保存は
  best-effort で、abort record でも `checkpoint` key が載ら
  ない経路がある。

allowlist 外（`rawOutput`・`rawOutputPath`・`exitCode`・
`retryable`・`cause`・その他 details key）は durable log に
記録されない（model 生出力・payload 参照を残さない境界）。

record schema は additive-only で、新旧の record は同一
usage.jsonl に混在しうる。旧 writer の record（上記 optional
field なし）も新しい `cheaper usage` がそのまま集計でき、
逆に新 writer の record も旧 `cheaper usage` が既知 field
のみ読んで集計できる（未知 field は無視される）。例として
`kind:"route"` record の `executed`（`{grant, argv}` — 実際に
実行された grant と argv）は additive
field で、既存 reader は無視して集計できる。byte-matched
route では env-prefix-strip 済みの元 exec argv、substitute
mode（routes.yaml の `substitute` 宣言）では宣言 substitute
ペアを記録する（`route`/`commandHash` の宣言 identity は不変）。
ただし `executed.argv` は**平文の引数値**を保持する — 引数値自体に
secret を含む command の routed 実行は usage.jsonl に値が残る
（詳細は [parent-agent 連携ガイド](./parent-agent-integration.md)
の route record 節）。`usage.costUsd` の算出元は新旧で異なる
（後述）ため、混在 file の costUsd 合計は「同じ算出式の値の和」
ではない点に注意。

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
cheaper usage --group-by route                   # route 別 section
cheaper usage --group-by error                   # worker failure code 別 section
cheaper usage --route code-writer                # route bucket を指定 route に限定
cheaper usage --since 168h --group-by route      # 7 日分の route 集計
cheaper usage --format text                     # 人間向け表示
```

`--group-by` は単値 option で、値は `route` または `error`。
`route,error` など複合値・未知値は `INVALID_REQUEST` になる。
`--route` は route-scope だけを絞り、run totals と `byError` には
影響しない。`--since` は run totals に加えて route/refetch bucket にも
適用される。duration は `24h` / `168h` のような時間単位で指定する。

`result.records`・`ok`/`errors`・`usage`・`turns`/`toolCalls`/
`durationMs` は filter 後の recorded `kind:"run"` のみを集計する。
`result.route` は `kind:"route"` と `kind:"route-refetch"` の独立
bucket で、run の worker/provider/model filter は適用しない。
usage.jsonl が無い・破損行が混じる場合もエラーにならない（破損行は
skip、file 不在は 0 件集計）。

`--group-by route` は `byRoute` を追加し、各 route の executions・
成功/失敗・token reduction・joined refetch・duration・計測不能数を返す。
`--route <name>` を併用すると `result.route` と各 row をその route に
絞る。`route.orphanRefetches` と `route.unkeyedRecords` は route window
全体の診断値なので route filter の影響を受けない。
refetch record は同じ window 内の route record の非空 `logRef` で join
する。`--group-by error` は失敗した recorded run を `error.code` 別に
まとめ、欠損/不正 code は `unknown`、空文字 code は空文字の bucket にする。

割合の母数は異なる。`byError[].shareOfRuns` は filter 後の recorded run
総数、`shareOfFailures` はそのうちの失敗 run 数を使う。
`measured` は `savings` を持つ run の実測 offload、`comparable` は
`directlyComparable: true` の run、`estimated` は比較モデルの価格と
comparable 母数に基づく推定値である。route/History の記録を失敗率へ
混ぜず、全 execution の成功率とは解釈しない。

`--format text` は route section を常時表示し、grouping を指定した場合は
`byRoute` または `byError` section も表示する。

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

## Execution History

`cheaper history` は `<userConfigDir>/history/` に保持された実行単位の記録を
読みます。既定の表示形式は `text` です。`list` と `trace` は 1 ページ
50 件を表示し、`--limit` は 1〜500 件、`--cursor` は次ページ用の不透明な
cursor を指定します。

```bash
cheaper history list
cheaper history list --since 2h --kind route --status failed --limit 100
cheaper history list --worker bulk-reader --format json
cheaper history list --cursor <opaque-cursor> --format json
cheaper history show <execution-uuid>
cheaper history show <execution-uuid> --format json
cheaper history trace <trace-uuid> --limit 50 --format text
cheaper history trace <trace-uuid> --cursor <opaque-cursor> --format json
```

`list` は `--since <Nm|Nh>`、`--worker <id>`、`--kind <kind>`、
`--status <status>` で絞れます。kind は `worker`、`workflow`、`job`、
`route`、`hook-decision`、`route-decision`、`route-refetch`、status は
`running`、`completed`、`failed`、`cancelled`、`timeout`、`skipped`、
`blocked` です。`show` は 1 つの execution ID を読み、`trace` は trace ID に
一致する記録をページ表示します。`list` と `trace`
の JSON には `records`、`scanComplete`、`nextCursor` が含まれます。
予算内に走査できない場合は `scanComplete: false` と `nextCursor: null`
を返し、text 表示にも不完全な走査であることを示します。`show` の JSON
には `recordState` があり、terminal が未到着の記録や破損・未検出を区別します。

History の query は read-only です。`list`・`show`・`trace` は保存先を作成せず、
retention sweep、Usage 追記、provider 呼び出し、実行処理を行いません。
保存先が無い場合は空の結果を返します。

History は allowlist のみを保存します。execution/trace/parent ID、分類済みの
kind・event・status・reason・error code、検証済みの worker/provider/model/
workflow/job/route ID、duration や token・turn・tool call・exit code などの
有界な数値、既存 `logRef` と Usage record 参照が対象です。prompt、stdin や
input JSON、tool の引数・出力、raw argv/env、API key、credential、生の error
本文は保存しません。親 ID が外部プロセスから渡された場合は未検証の hint として
区別し、親子関係を推測で確定しません。

History の書き込みは best-effort です。開始・判断・terminal の書き込みに失敗しても
製品処理の結果は変わらず、その記録が欠落したり terminal が無い記録になったりする
場合があります。History の有無から Usage record を作成・補完することはなく、
過去の Usage record を History に backfill しません。両方の記録は独立しています。

保持ポリシーは 30 日・10,000 execution directory・論理 event file 合計 128 MiB
を目標とし、各 event は 8 KiB 以下です。これらは hard quota ではなく、並行書き込み
により一時的に超えることがあります。list/trace の走査は最大 20,000 directory、
event data 128 MiB、500 ms のいずれかまでです。writer の retention sweep も最大
20,000 entry、128 MiB、500 ms のいずれかで打ち切り、容量を確認できないときは
History の新規書き込みを省略します。`show` は指定 ID の directory にある既知の
event file を最大 3 個まで読み、それぞれ 8 KiB を上限にします。
