---
id: "TASK-034r2qodyq0oWVUNGquMgr"
type: "task"
status: "done"
title: "Sync providers.md guide with runtime auth surface"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
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
    - "docs/plans/provider-auth-guide-runtime-sync.md"
  implemented-by: []
  depends-on:
    - "docs/plans/provider-auth-guide-runtime-sync.md"
  blocks:
    - "docs/tasks/verify-guide-consistency-and-run-doc-audit.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/provider-auth-guide-runtime-sync.md"
    - "docs/specs/provider-auth-docs-runtime-sync.md"
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
---

# TASK-034r2qodyq0oWVUNGquMgr: Sync providers.md guide with runtime auth surface

## Work

`docs/guides/providers.md` を現行 runtime の auth contract に
同期する（Issue #33 / SPEC-034r2qodmutbif9L0DLskN）。

### 1. OAuth 例示の修正

- `openai-codex` 節の「OAuth 対応 provider は `anthropic` /
  `github-copilot` / `openrouter` / `xai` 等も同じ手順」から
  `xai` を外す。
- auth type を明確に分ける: OAuth login 可能な provider の例は
  runtime で確認できるものに限定し、api_key 型の login は
  `--type api_key` の対話入力（例: `XAI_API_KEY` 等の env var
  resolve は provider 側 api_key auth が行う）として説明する。
- xAI を説明する場合は API-key 側として扱い、OAuth provider
  として案内しない。

### 2. `auth status` の contract 修正

- 「実際に `auth status` で確認して model 一覧を確認する」と
  読める記述を、`auth status [provider]` が auth capability
  （login method・subscription flag）と configured state・
  credential metadata を返す確認コマンドであるという実
  contract に改める。
- `result.providers[].{providerId,name,methods,subscription,
  configured}` / `result.credentials[].{providerId,type}` の
  output shape に沿って説明する。

### 3. model 確認経路

- model の実利用確認は worker 実行（例: `cheaper bulk-read`）の
  result envelope `meta.model` で観測する旨を記述する。

### 4. canonical command 位置づけ

- provider 対応状況を静的に断定せず、`cheaper auth status`
  が runtime の provider/auth surface を確認する canonical
  command であることを示す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/provider-auth-guide-runtime-sync.md`
- `docs/specs/provider-auth-docs-runtime-sync.md`
- `docs/adr/provider-auth-guide-auth-status-canonical-surface-auth-type.md`
- `docs/designs/provider-auth-guide-runtime-sync.md`

## Dependencies

- なし（plan の先頭 task）

## Affected Files

- `docs/guides/providers.md`（modify）

## Done When

- [x] OAuth 例示から `xai` が除去され、auth type が明確に分けられている
- [x] `auth status` が model 一覧を返すような記述がない
- [x] `auth status` が auth capability / configured state の
  確認コマンドとして明記されている
- [x] model 実利用確認が `meta.model` で説明されている
- [x] `auth status` が provider/auth surface の canonical 確認
  command として示されている

## Verification

- [x] `rg "xai" docs/guides/providers.md` — OAuth 例示文脈に
  残っていないこと
- [x] `cheaper auth status` / `cheaper auth status xai` の実
  output と記述が一致すること
- [x] `rg "auth status" docs/guides/providers.md` の記述が実
  contract（credential・methods・subscription・configured）と
  一致すること
