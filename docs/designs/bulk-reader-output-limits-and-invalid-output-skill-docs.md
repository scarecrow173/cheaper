---
id: "DESIGN-giTr4G7NzwSzWJTVgBo3ax"
type: "design"
status: "approved"
title: "bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書設計"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/108"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
  implemented-by: []
  depends-on:
    - "../adr/document-bulk-reader-output-limits-in-consumer-skill.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
    - "../adr/document-bulk-reader-output-limits-in-consumer-skill.md"
    - "../discovery/bulk-reader-output-limits-and-invalid-output-guidance.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/document-bulk-reader-output-limits-in-consumer-skill.md"
    - "../specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書設計

## 概要

`cheaper-bulk-read` skill 正本（`packages/apm/.apm/skills/`）に
出力フィールド制約表と INVALID_OUTPUT 時の回復指針を追加し、
`cheaper-usage` の exit code 節に `error.details` の補足を入れる。
skill 表の数値と schema JSON の drift は node test で検知する。
runtime・schema・policy は変更しない。

`.agents/skills/` は gitignore された APM deploy 領域であり tracked
file でないため、本変更の対象外（commit できず `apm install` で
上書きされうる）。

## 変更点

### 1. `cheaper-bulk-read/SKILL.md`（`packages/apm/` 側のみ）

「Consuming the result」節を拡張し、次の2節を追加する。

#### a. 出力フィールド制約表

`bulk-reader.output.schema.json` の値をそのまま表記:

| フィールド | 制約 |
|---|---|
| `summary` | string 1–1500 文字（必須） |
| `findings[]` | 最大30件 |
| `findings[].title` / `.detail` | 120 / 1200 文字 |
| `findings[].confidence` | `high`/`medium`/`low` |
| `findings[].locations[]` | 最大10件 |
| `locations[].path` | workspace 相対（絶対・`..` 不可） |
| `locations[].lines` | `{start,end}` ≥1（任意） |
| `locations[].excerpt` | **600 文字** |
| `relevantFiles[]` / `suggestedReads[]` | 30 / 10 件、`reason` 300 文字 |
| `suggestedReads[].lines` | `{start,end}` ≥1（任意） |
| `openQuestions[]` | 10 件、各300 文字 |

併記する注意:

- 5つの top-level フィールド（`summary`/`findings`/`relevantFiles`/
  `suggestedReads`/`openQuestions`）はすべて required で、各 object は
  `additionalProperties: false` — 未定義キーの追加や必須キー欠落も
  INVALID_OUTPUT になる。
- 制約の正本は
  `crates/cheaper-cli/resources/workers/schemas/bulk-reader.output.schema.json`
  で binary へ embedded されており、worker schema を dump する公式 CLI
  手段はない。

#### b. `INVALID_OUTPUT` 時の回復指針

- `error.code === "INVALID_OUTPUT"`、exit 6、`retryable: false`。
- `error.details.errors`: schema / JSON parse error の先頭20件
  （repair 試行後は最終試行のもの）— 違反フィールド特定の手掛かり。
- `error.details.rawOutput`: worker 生出力を先頭2000文字に切り詰めた
  **診断用抜粋。検証済み result ではない**（空の場合もある）。
  この中の `suggestedReads` 等を採用してはいけない。
- 推奨回復（順序）:
  1. `details.errors` で超過箇所を確認し、`task`/`paths` を絞るか
     `maxFindings` を下げて再実行。
  2. 繰り返し失敗する場合、consumer 側 policy で
     `output.repairAttempts`（bundled 既定1、小モデルには例外的に
     `docs/guides/configuration.md` で 4 が例示される）や
     `output.maxTokens` を見直す。
  3. それでも駄目なら直接 read またはより小さい分割委譲へ
     フォールバックする。

### 2. `cheaper-usage/SKILL.md`（`packages/apm/` 側のみ）

exit code 表の直後（既存の exit 2 補足と同じ位置関係）に短い補足を
追加。worker `run`/alias 経路では `INVALID_OUTPUT`（exit 6）の
`error.details` が `errors`（先頭20件）と `rawOutput`（先頭2000文字の
診断用抜粋、検証済み result ではない）を持つこと、`workflow run` では
別 shape（`rawOutput` なし）になり得ることを一言で区別する。

### 3. drift 検知 test

`scripts/test/` に node test を1本追加: `bulk-reader.output.schema.json`
から主要な `maxLength`/`maxItems`/`enum` 値を読み、SKILL.md 記載の
数値と照合する。`node --test scripts/test/` 経路で `mise run check`
に組み込まれる。

## 非対象

- runtime 変更、上限値の変更、repairAttempts の bundled 既定値変更、
  schema dump コマンド、他 worker skill への同様の記載、
  `.agents/skills/` の同期。

## 検証

- 差分が skill 文書2件と新規 test 1件のみであること。
- 新規 node test が pass すること（`node --test scripts/test/`）。
- `mise run check`（doc-id チェック・biome 含む）。
