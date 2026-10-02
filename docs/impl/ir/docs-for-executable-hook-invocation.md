---
id: "IMPL-034z57kKSICF9N4av5oZ0f"
type: "impl"
status: "completed"
title: "docs for executable hook invocation"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/update-docs-for-executable-based-hook-invocation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# docs for executable hook invocation

## Summary

ユーザー向け doc を executable 経由の hook 呼び出しに合わせて
更新した。`cheaper.mjs` は手動 fallback として残る位置づけを
明記し、hook 計測が `usage.jsonl` に記録されない決定も文書化した。

## Context

- task: docs/tasks/update-docs-for-executable-based-hook-invocation.md

## Implementation

### Changes

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`: transitional
  記述を除去し、hook 登録が `cheaper hook <event> --target <agent>`
  で launcher/exe を直接起動することを明記。`cheaper.mjs` は
  platform executable が無い環境向けの Node 必須 fallback として
  残置。target 別の functional/inert 記述を現行挙動（Windsurf /
  Copilot / Claude / Cursor が機能、Codex は upstream gap で
  file-read のみ不可）に更新し、`usage.jsonl` 非記録を追記
- `docs/guides/parent-agent-integration.md`: adapter file 名の言及を
  `cheaper hook <event> --target <agent>` 呼び出しに置換。閾値
  セクションに hook が Node 非依存であること・`cheaper.mjs`
  fallback・usage.jsonl 非記録を追記。検証コマンドを launcher /
  exe 形式に更新。「count-tokens 子 process」前提の記述を
  同一計測経路という表現に修正
- `docs/guides/usage.md`: 記録対象外として `hook` を明示

### Decisions During Implementation

- ADR/design/spec/discovery など履歴 doc は当時の決定記録として
  改訂しない（canonical record の原則）。更新はユーザー向け
  guide と配布 skill に限定

## Validation

- 記述の正確性は task 2–4 の実機検証結果（各 target の deny
  contract・deploy 実測）に基づく

## Follow-ups

- task 6: JS adapter 削除と最終検証
