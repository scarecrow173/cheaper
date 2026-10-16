---
id: "SPEC-7GjKOMzR6m3HfBdpPmzSIe"
type: "spec"
status: "implemented"
title: "bulk-reader 出力フィールド上限と INVALID_OUTPUT 回復指針の文書化"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/bulk-reader-output-limits-and-invalid-output-guidance.md"
    - "../adr/document-bulk-reader-output-limits-in-consumer-skill.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/bulk-reader-output-limits-and-invalid-output-guidance.md"
  derived-by:
    - "../adr/document-bulk-reader-output-limits-in-consumer-skill.md"
    - "../designs/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
---

# bulk-reader 出力フィールド上限と INVALID_OUTPUT 回復指針の文書化

## 背景と目的

Issue #108。`bulk-reader` の出力が schema 上限（特に
`findings[].locations[].excerpt` の600文字）を超えると、1回の自動修復
（`repairAttempts: 1`）に失敗した時点で `INVALID_OUTPUT`（exit 6、
non-retryable）が親 agent へ返る。現在の consumer 向け skill 文書は入力上限
を記すが、**出力フィールド上限を記していない**。親 agent は失敗後も
`error.details.rawOutput` が「検証済み result」ではなく切り詰められた
診断用抜粋（先頭2000文字）であることも文書から分からず、結果として
回復方針を持たない。

目的は、親 agent が (a) 出力上限を事前に知って指示を制御し、(b) INVALID_OUTPUT
発生時に安全で境界のある再試行を行えるよう、正確な文書を skill に追加する
こと。

## スコープ

**対象:**

- `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` — bulk-reader の出力
  フィールド上限、`rawOutput` の意味、bounded な再試行指針を追加
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — exit 6 /
  `INVALID_OUTPUT` の `error.details`（`errors`・`rawOutput`）に関する
  簡潔な説明を追加（既存 exit code 表の補足）

**注:** `.agents/skills/` は gitignore された APM deploy 領域であり
tracked file ではないため、本件の変更対象に含めない（discovery 参照）。

**除外:**

- runtime 挙動の変更（上限値変更、`rawOutput` の全文返却、repair attempts の
  増加、未検証出力の受理）
- worker schema を dump する CLI コマンドの新設（別 issue 候補）
- tool policy・turn cap の緩和

## 要件

1. bulk-read skill が bulk-reader 出力スキーマのフィールド制約を表で記す
   （`bulk-reader.output.schema.json` と一致する値のみ）。上限値に加え、
   schema が closed object（`additionalProperties: false`）であり全ての
   top-level フィールドが required である点も一言で触れる
2. 上限の正本位置（`crates/cheaper-cli/resources/workers/schemas/`、
   binary へ embedded）と、worker schema を dump する公式 CLI 手段が
   存在しないことを明記する
3. worker `run`（alias 含む）で `INVALID_OUTPUT`（exit 6）が返った場合の
   `error.details` を説明する — `errors`（schema/parse error の先頭20件、
   repair 試行後は最終試行のもの）と `rawOutput`（**検証済み result では
   ない**、先頭2000文字に切り詰められた診断用抜粋で空の場合もある）
4. bounded な再試行指針を記す — `rawOutput` を result として採用せず、
   scope を狭めた再実行（task/paths の絞り込み・`maxFindings` 低下）、
   policy の `output.repairAttempts`/`output.maxTokens` を consumer 側で
   上げる選択肢（`docs/guides/configuration.md`）、または直接 read への
   フォールバックに留める
5. skill 文に記した上限値が schema と drift しないよう、値を schema JSON
   と照合する小さな自動チェック（node test）を追加する

## Acceptance Criteria

- [ ] `cheaper-bulk-read` SKILL.md に `excerpt` ≤600 文字を含む出力
      フィールド制約が記載され、値が `bulk-reader.output.schema.json` と一致
- [ ] `INVALID_OUTPUT` で `rawOutput` が「検証済み結果ではなく診断用の
      先頭2000文字の抜粋」であることが明記されている
- [ ] 修復失敗後の推奨手順（scope を狭めた再実行 / `repairAttempts` 等の
      consumer 設定 / 直接 read へのフォールバック）が記載され、unvalidated
      output の採用を推奨しない
- [ ] skill 表の上限値を schema JSON と照合する自動チェックが
      `mise run check` 経路（node test）で検証される
- [ ] runtime コード・schema・policy に変更がない

## Open Items

- **pre-implementation blocker なし**。以下は本件スコープ外の残課題:
  - 他 worker（`tool-operator` の `evidence[].detail` ≤600 等、
    `code-writer`/`doc-writer`）も同じ INVALID_OUTPUT 露出を持つが、
    各 skill への上限記載は別 issue 候補
  - worker schema を dump する CLI コマンド（Option C）は別 issue 候補
  - `.agents/skills/` の drift 恒久対策（untracked deploy 領域）も別件
