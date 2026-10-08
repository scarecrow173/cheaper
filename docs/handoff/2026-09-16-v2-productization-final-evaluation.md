# V2 productization 最終評価 — 16 問への回答

指示 §61「最終評価で答えるべき問い」への具体的回答。
検証時点: `feature/restructure-builtin-workers` @ 実装完了（全 baseline green）。

## 1. Parent Coding Agent はどのタイミングで cheaper へ delegation するのか

2 経路:

- **能動**: APM skill（`cheaper-bulk-read` / `cheaper-code-write` /
  `cheaper-doc-write`）が WHEN/WHY を定義し、parent が大 file の要約・
  機械的変更を必要とした時点で bundled CLI を呼ぶ。
- **強制（safety net）**: Windsurf `pre_read_code` hook が read 前に
  `count-tokens` を実行し、threshold 超過 file を exit 2 で block して
  bulk-read への guidance を返す（`.apm/hooks/pre-read-code.js`）。
  `tool_info.file_path`（absolute）は workspace-relative POSIX path へ
  正規化して guidance へ埋め込み、workspace 外は fail-open。guidance は
  非実行型テキストのみ（shell command の raw interpolation なし）。

## 2. token 数はどこで・どの tokenizer で測定されるか

`@cheaper/core` の `countFileTokens`（`src/token/count.ts`）が唯一の実装。
tokenizer は `js-tiktoken@1.0.21` / `o200k_base`（ADR-034r05iHrZUE6W3U1rbJq4）。
結果に `{name, implementation, version}` を含む。

## 3. CLI と count_tokens tool は同一実装か

はい。`count-tokens` CLI（cli.ts）も runtime `count_tokens` tool
（runtime-pi/tools/count-tokens.ts）も `countFileTokens` を呼ぶ。
hook は bundled CLI 経由で同じ実装に到達する。parity e2e
（exec.test.ts "count_tokens tool and count-tokens CLI return the same
count"）で tool 結果と CLI 出力の一致を検証済み。

## 4. large file の threshold と実測根拠

**8192 tokens**（`CHEAPER_READ_TOKEN_THRESHOLD` で上書き可）。根拠は
DISC-034r05iHy2gdnpb598KIca / ADR-034r05iHrZUE6W4XOqUBih / `docs/impl/exp/bulk-read-delegation-benchmark.jsonl`:

- 本 repo 293 file 計測: >8k は lockfile 2 件のみ（corpus の 0.7%）
- Ollama 実 provider で 6.3k file の bulk-read → parent 受信 ~260 tok
  （96% 削減）を実測
- cost break-even は ~2k だが、~15s の委譲 latency と編集対象 file への
  false positive block を避けるため保守値に確定

## 5. large source が parent context へ入る前に止められているか

はい。delegation-loop e2e（`packages/cli/test/e2e/delegation-loop.test.ts`）
が deploy 済み bundle で実証: `pre_read_code` が大 file read を exit 2 で
block → bundled `bulk-read` が圧縮 `{summary, findings}` のみ返却。
file 本文は envelope に含まれないことを assert 済み。

## 6. 4 skills の責務分離

- `cheaper-usage`: CLI 全体の HOW（envelope、exit code、config、bundle path）
- `cheaper-bulk-read`: 大 file/多 file の調査→圧縮回答（WHEN: 内容ではなく
  判断材料が要るとき）
- `cheaper-code-write`: 機械的コード変更（tests/DTO/boilerplate）
- `cheaper-doc-write`: 決定済み内容の文書化

worker skill は実行方法のみを記述し、CLI の詳細は `cheaper-usage` へ
委譲して重複を避ける。

## 7. 3 workers の担当

- `bulk-reader`: read-only（read/grep/find/ls/count_tokens）。
  質問に対し圧縮結果 `{summary, findings, relevantFiles, suggestedReads,
  openQuestions}` を返す。mutation 不可
- `code-writer`: `targets` 内への機械的コード変更。`writeScope` で
  targets 外 write を runtime が拒否
- `doc-writer`: `targets` 内への文書作成。同じく scope 強制

## 8. なぜ cheap-writer を残していないのか

実運用前で利用者がおらず、code/doc の責務分離が最終設計として正しいため。
旧名を残すと「どちらを使うか」の曖昧さと schema 互換負債が残る。
`cheap-write` alias も削除済み（rejection test で非存在を保証）。

## 9. 親 Agent は cheaper CLI をどう呼ぶか

APM install 後の確定的な deployed path 経由:

```bash
node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs <worker> --cwd .
```

stdin に task JSON、stdout に envelope、`--format text` で人間向け出力。
APM 配布により PATH 設定・repo build・npm install は不要（self-contained
single-file bundle）。

## 10. provider/model の選択

Worker frontmatter が logical tier（`low`/`medium`/`high`）のみ宣言。
実 model は `models.yaml` の `overrides`（provider/id → tier）と
`preferences`（tier → pattern 順）+ Pi `models.json` の provider 定義で
実行時に resolve される。policy `model.maxTier` が上限を強制。
worker/配布物は provider 非依存。

provider 認証は `cheaper auth login <provider>`（SPEC-034r05iHK8bA7ghSWRbNU8）が Pi の
`ModelRuntime.login` を吸収して実行 — standalone `pi` install は不要。
`--type api_key` の対話 key 入力は raw-mode no-echo で読み、
credential 値は stdout/stderr/envelope のいずれにも出ない（test で保証）。
OpenAI ChatGPT subscription（`openai-codex` OAuth）での実測: login →
`bulk-read` が `gpt-5.5` で完走（`docs/impl/exp/openai-codex-subscription-e2e.jsonl`。`gpt-5.4` 系・codex-spark は
ChatGPT account Codex では非対応、`gpt-5.5` / `gpt-5.6-*` / `gpt-6-astra`
が対応確認済み）。

## 11. remote provider へ送ってはいけないデータの守り方

- workspace boundary: 全 tool が `resolveWorkspacePath` 経由で workspace 外
  access を拒否（symlink policy 含む）
- privacy deny（SPEC-034r05iHK8bA7ggPjqIA9E）: built-in `DEFAULT_DENY_PATHS`（`.env*`、`*.pem`、
  `id_rsa`、`.ssh/**` 等）+ `filesystem.denyPaths` が read/write/edit/ls/
  count_tokens を拒否し、grep/find/ls の結果からも除外
- worker output schema が file 本文を返さない構造を強制

## 12. writer の意図外 mutation の防止・検出

- **防止**: `cheaper.writeScope: targets` → write/edit/mkdir が targets の
  正規化 path 集合に限定される（違反は tool error + `rejectedToolCalls`）
- **検出**: MutationJournal が before/after を実測し `meta.changes` に
  報告。model の自己申告は信頼しない設計
- prompt の禁止事項は defense in depth として残置

## 13. 実際に削減できた token/cost

実測（`docs/impl/exp/bulk-read-delegation-benchmark.jsonl`、Ollama qwen3.5:4b）:

| file tokens | parent 受信 | 削減率 |
|---|---|---|
| 3,013 | ~1,230 | 59% |
| 6,267 | ~260 | 96% |

frontier $2.5/Mtok・cheap $0.4/Mtok の代表的価格で 6.3k file の read は
直接 $0.0157 → 委譲 ~$0.0026（約 6x 削減）+ parent context 6k tokens の
温存。本番 deployment では `meta.usage.costUsd` が run ごとに記録される。

## 14. cheaper を使わない方がよい条件

- 小さい file（< ~2k tokens）の read: 直接 read の方が速く安い
- 正確な file 内容が必要な作業（編集対象の精読、diff 作成）
- 判断そのもの（設計判断・仕様決定）は parent の責務
- latency 重視の対話的場面（委譲は spawn + model 推論で数秒〜）
- secret 系 file を扱う作業（deny list でそもそも worker は読めない）

## 15. Runtime 変更なしの Worker 追加

維持されている。Worker は `*.md` frontmatter（name/description/tools/
cheaper.{model.tier, input/output schema, writeScope}）+ JSON schema の
宣言のみで、project `.cheaper/workers/`・user config・`--worker-file`・
builtin の 4 経路から resolve される。runtime コード変更は不要
（`code-writer`/`doc-writer` 追加自体がその実証）。

## 16. 不要な compatibility code

残していない。`cheap-writer`/`cheap-write` は alias ごと削除、worker
schema・policy・vocabulary も旧名への互換経路なし。最終設計へ直接向かう
構成（旧名は rejection test で非存在を保証）。
