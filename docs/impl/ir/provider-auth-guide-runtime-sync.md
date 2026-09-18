---
id: "IMPL-034r2qodykm6M1ReYSbIkc"
type: "impl"
status: "completed"
title: "provider/auth guide runtime sync (PLAN-034r2qodyxvN3g3xktwPOF)"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/providers.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/sync-providers-guide-with-runtime-auth-surface.md"
    - "docs/tasks/verify-guide-consistency-and-run-doc-audit.md"
  implemented-by: []
  depends-on:
    - "docs/plans/provider-auth-guide-runtime-sync.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/provider-auth-docs-runtime-sync.md"
    - "docs/designs/provider-auth-guide-runtime-sync.md"
    - "docs/adr/provider-auth-guide-auth-status-canonical-surface-auth-type.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/provider-auth-docs-runtime-sync.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# provider/auth guide runtime sync (PLAN-034r2qodyxvN3g3xktwPOF)

## Summary

Issue #33: `docs/guides/providers.md` を pin 済み Pi runtime
（`@earendil-works/pi-ai@0.85.1`）と cheaper CLI の auth
contract に同期した。OAuth 例示から `xai` を除去し auth type
を明確化、`auth status` を auth capability / configured
state の canonical 確認コマンドとして明記し、model 一覧取得を
示唆する記述を `meta.model` 経由の実利用確認に改めた。
`packages/apm/.apm/skills/cheaper-usage/SKILL.md` の credential
保存先の矛盾（"configured in the project"）を最小修正した。

## Context

- Issue #33（approved requirement）に基づく documentation-only
  修正。code behavior は変更しない。
- 上流: SPEC-034r2qodmutbif9L0DLskN /
  ADR-034r2qodyqNY09yFc1ftZd /
  DESIGN-034r2qodyo7KRfBsGRJQty /
  PLAN-034r2qodyxvN3g3xktwPOF。

## Implementation

### Changes

- `docs/guides/providers.md`:
  - `openai-codex` 節の OAuth 例示列挙から `xai` を除去し、
    OAuth login は `auth status` が `methods` に `oauth` を
    示す provider で利用できる旨へ改めた（例示は
    `anthropic` / `github-copilot` / `openrouter` /
    `kimi-coding` / `radius` / bundled `devin`）。
  - API key 型は `--type api_key` 対話入力または provider 別
    環境変数（例: `XAI_API_KEY`）で credential 解決する旨を
    明記し、auth type を明確に分離した。
  - 「`auth status` で確認して model 一覧を確認する」記述を
    削除し、`auth status` が auth capability・configured
    state・credential metadata を返す確認コマンドであり
    model catalog は返さないことを明記。
  - model の実利用確認は worker 実行の result envelope
    `meta.model` で行う旨を記述。
  - 「その他の provider」節を、`auth status` が provider/
    auth surface の canonical 確認 command である表現に更新。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`:
  - "Provider credentials and the model map are configured in
    the project" → credential は user-scope
    `<userConfigDir>/pi/`（`cheaper auth` 管理）で project
    には置かない旨に最小修正。skill の全面同期は Issue #34。

### Decisions During Implementation

- xAI の扱い: issue 指定どおり OAuth 例示から外し、API-key
  側の例（`XAI_API_KEY` 環境変数）としてのみ言及した。
  runtime の `auth status xai` は `["api_key","oauth"]` を
  返すため、例示は網羅ではなく `auth status` を canonical
  確認経路として示す構成で吸収した（ADR-034r2qodyqNY09yFc1ftZd）。
- 整合確認: usage.md・configuration.md・README.md は
  providers.md への委譲構造で矛盾なし。cheaper-usage
  SKILL.md の credential 記述のみ矛盾を検出し最小修正した。

## Validation

- `cheaper auth status` / `cheaper auth status xai` の実
  output（`result.providers[].{providerId,name,methods,
  subscription,configured}`）と guide の記述を突合。
- `rg "xai" docs/guides/providers.md` — OAuth 例示文脈に
  残っていないことを確認（`XAI_API_KEY` の api_key 側言及のみ）。
- `rg "model 一覧" docs/guides/providers.md` — 0 件。
- `audit_docs.js --type all` no findings。
- `mise run check` pass。

## Risks

- Pi runtime の version 更新で OAuth 例示が再び drift する
  可能性は残るが、`auth status` を canonical 確認 command
  として導線を張る構成で影響を限定している。

## Related Experiments

### Adopted

なし。

## Follow-ups

- Issue #34: distributed `cheaper-usage` skill および
  `parent-agent-integration.md` の全面同期（hook matrix・
  auth command surface・credential 保存先・fail-open/closed
  contract）。本 impl で最小修正した SKILL.md の残りの同期は
  #34 で実施する。
