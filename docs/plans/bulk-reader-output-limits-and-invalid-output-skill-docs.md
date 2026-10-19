---
id: "PLAN-ANDDozesHfjJTzvFeRN6bf"
type: "plan"
status: "completed"
title: "bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書化計画"
test-spec-skip: "scripts/test/ の skill 記載値照合 node test と mise run check（crates/cheaper-cli/tests/ を含む workspace test）で検証済みのため、独立した test-spec artifact は作成していない。"
created: "2026-10-17"
updated: "2026-10-19"
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
    - "docs/specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
  implemented-by: []
  depends-on:
    - "docs/designs/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/document-bulk-reader-output-limits-in-consumer-skill.md"
    - "docs/discovery/bulk-reader-output-limits-and-invalid-output-guidance.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
    - "docs/designs/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/adr/document-bulk-reader-output-limits-in-consumer-skill.md"
  defers: []
  deferred-by: []
---

# bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書化計画

## Goal

`cheaper-bulk-read` skill に出力フィールド制約表と INVALID_OUTPUT 回復
指針を追加し、`cheaper-usage` の exit code 節を補足する。skill 表の
数値を schema JSON と照合する node test を追加し drift を CI 検知する。
runtime・schema・policy は変更しない（issue #108）。

## Scope

| In | Out |
|---|---|
| `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` | `.agents/skills/`（untracked deploy 領域） |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | runtime/schema/policy 変更 |
| `scripts/test/` 新規 drift チェック test 1件 | 他 worker skill、schema dump コマンド |

## File Map

- `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` — 「Consuming the
  result」拡張: 出力制約表 + INVALID_OUTPUT 回復節を追加（design §1）
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — exit code 表直後に
  `error.details` 補足（worker run/alias 経路の `errors`/`rawOutput`、
  workflow run との shape 差）（design §2）
- `scripts/test/bulk-reader-skill-limits.test.mjs` — schema JSON と
  SKILL.md 記載値の照合 test（design §3）

## Verification Matrix

| 変更 | 検証 |
|---|---|
| skill 記載値 = schema 値 | 新規 node test（`node --test scripts/test/`） |
| 文書のみの変更 | `git diff` の scope 確認 |
| 全体回帰 | `mise run check`（biome・doc-id 含む） |

## Risks

- 数値の手書き転記ミス → node test で検知
- `mise run check` の biome が .mjs のフォーマットを指摘 → `biome --write` で対応

## Rollback

文書＋test のみ — revert で完全に戻せる。
