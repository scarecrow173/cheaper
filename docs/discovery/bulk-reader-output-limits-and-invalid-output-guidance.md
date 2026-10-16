---
id: "DISC-NXzrfJIbmACuqdlsNsxtWN"
type: "discovery"
status: "resolved"
title: "bulk-reader 出力フィールド上限と INVALID_OUTPUT 回復指針の文書化"
created: "2026-10-17"
updated: "2026-10-17"
owners: ["scarecrow173"]
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/document-bulk-reader-output-limits-in-consumer-skill.md"
    - "../specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
    - "../adr/document-bulk-reader-output-limits-in-consumer-skill.md"
---

# bulk-reader 出力フィールド上限と INVALID_OUTPUT 回復指針の文書化

## 背景

Issue #108。consumer 環境（cheaper 1.0.0 / Windows / `devin/swe-1.7-lightning`）
で `bulk-reader` が `INVALID_OUTPUT`（exit 6）で失敗した。schema error は
`/findings/4/locations/0/excerpt`・`/findings/5/locations/0/excerpt` が
600 文字上限を超過したもので、`error.details.rawOutput` は先頭 ~2000 文字の
診断用抜粋のみ。親 agent 向け skill は入力上限を記すが出力フィールド上限を
記さず、`rawOutput` の扱いや bounded な再試行指針も未記載だった。

Issue は runtime 変更ではなく文書化を要求している（tool policy・turn cap の
緩和や、未検証出力の自動受理は明示的に除外）。

## 確認した事実

### bulk-reader 出力スキーマの上限

正本: `crates/cheaper-cli/resources/workers/schemas/bulk-reader.output.schema.json`
（Rust binary へ embedded、consumer は実行時に直接読めない）。

| フィールド | 上限 |
|---|---|
| `summary` | string 1–1500 文字 |
| `findings` | 最大30件 |
| `findings[].title` | 120 文字 |
| `findings[].detail` | 1200 文字 |
| `findings[].confidence` | enum: `high`/`medium`/`low` |
| `findings[].locations` | 最大10件 |
| `findings[].locations[].path` | workspace 相対（`/`・`X:`・`..` 禁止 pattern） |
| `findings[].locations[].lines` | `{start, end}` ともに整数 ≥1 |
| `findings[].locations[].excerpt` | **600 文字**（issue での失敗原因） |
| `relevantFiles` / `suggestedReads` | 各最大30件 / 10件、`reason` 300 文字 |
| `openQuestions` | 最大10件、各300 文字 |

worker prompt（`crates/cheaper-cli/resources/workers/bulk-reader.md:65`）は
「600 文字・20 行」と指示しているが、親 agent（skill の読み手）には
見えない。

### INVALID_OUTPUT の契約

- exit code 6・`error.retryable: false`（`crates/cheaper-core/src/errors.rs`）
- `error.details.errors`: schema error 先頭20件（`repair.rs` の
  `MAX_DETAIL_ERRORS`）
- `error.details.rawOutput`: raw 出力の先頭2000文字（`MAX_DETAIL_RAW`）—
  **切り詰められた診断用抜粋であり、検証済み result ではない**
- `meta.repairs` は消費した repair attempt 数。bundled policy の
  `repairAttempts: 1`（`crates/cheaper-cli/resources/policy.yaml`）なので
  1回の自動修復に失敗すると INVALID_OUTPUT になる（issue の `repairs=1`
  と一致）

### 既存の discovery 手段

worker の出力 schema を dump する CLI コマンドは**存在しない**
（`workflow interface` は workflow file 専用）。なお embedded schema は
実行時に `<temp>/cheaper-builtin-<version>/workers/schemas/` へ実体化される
ため consumer マシン上で物理的には読めるが、undocumented な内部経路であり
案内する対象ではない。したがって上限値は skill 本文へ直接記載する必要が
ある。schema 値が変われば skill 文も追随する責務が生じる（drift 検知機構が
無い点は既知のリスク）。

### skill 配置と drift

- `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` が consumer 向け正本
- `.agents/skills/` は **gitignore された APM deploy 領域**であり tracked
  file ではない。repo 内の `cheaper-bulk-read`/`cheaper-usage` コピーは既に
  drift している（例: FILE_TOO_LARGE 節欠落）が、deploy artifact のため
  本件の変更対象には含めない（手動同期はローカル環境のみに作用し、
  `apm install` で上書きされうる — 別途恒久対応の余地あり）

## 未確定・スコープ外

- runtime 側の挙動変更（上限値の変更、rawOutput の full 出力、repair
  attempts の増加）は issue が明示的に除外
- schema dump コマンドの新設は別 issue 候補（本件では行わない）
