---
id: "DESIGN-034r2qodyo7KRfBsGRJQty"
type: "design"
status: "approved"
title: "provider/auth guide runtime sync design"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/provider-auth-docs-runtime-sync.md"
    - "../adr/provider-auth-guide-auth-status-canonical-surface-auth-type.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/provider-auth-docs-runtime-sync.md"
    - "../adr/provider-auth-guide-auth-status-canonical-surface-auth-type.md"
  implemented-by:
    - "../plans/provider-auth-guide-runtime-sync.md"
  depends-on:
    - "./auth-command.md"
    - "./pi-state-paths-under-userconfigdir-pi.md"
    - "./user-facing-docs-information-architecture.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./devin-local-provider-plugin.md"
    - "./provider-only-registration-layer.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/provider-auth-docs-runtime-sync.md"
    - "../adr/provider-auth-guide-auth-status-canonical-surface-auth-type.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# provider/auth guide runtime sync design

## Context

`docs/guides/providers.md` は README・usage・configuration・
parent-agent skill から参照される canonical provider/auth guide。
現行の記述が pin 済み Pi runtime
（`@earendil-works/pi-ai@0.85.1` /
`@earendil-works/pi-coding-agent@0.85.1`）と cheaper CLI の
auth contract から drift している（Issue #33 /
SPEC-034r2qodmutbif9L0DLskN）。本 design は guide の記述を
runtime contract に合わせるための修正構造を定める。code
behavior は変更しない。

## Scope

### In Scope

- `docs/guides/providers.md` の以下の修正:
  - `openai-codex` 節の OAuth 例示列挙から `xai` を外し、
    auth type（oauth / api_key）を明確に分けた記述へ改める
  - 「`auth status` で model 一覧を確認する」と読める記述を、
    auth capability / configured state の確認コマンドという
    実 contract に合わせる
  - model の実利用確認を worker 実行結果 envelope の
    `meta.model` で説明する
  - `auth status` を provider/auth surface の canonical 確認
    command として位置づける
- 整合確認（矛盾があれば最小修正）: `docs/guides/usage.md`、
  `docs/guides/configuration.md`、`README.md`、
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md`

### Out of Scope

- implementation code・test・hook script の変更
- 配布 skill の全面同期（Issue #34 の scope）
- `models.json` schema や provider plugin contract の説明追加

## Components and Boundaries

- `docs/guides/providers.md`: canonical guide。auth type 別の
  login 手順・credential 保存先・`auth status` の役割・
  model 確認経路を実 contract で記述する。
- `packages/runtime-pi/src/auth.ts` /
  `packages/cli/src/auth.ts`: 記述の突合先。`auth status` は
  `result.credentials`（`{providerId,type}`）と
  `result.providers[]`（`{providerId,name,methods,subscription,
  configured}`）を返し、model catalog は返さない。
- `docs/guides/usage.md`: `cheaper auth` の CLI surface
  （`status`/`login`/`logout`・`--method`・`--type`）を既に
  記述。矛盾がないか確認のみ。

## Data and Control Flow

- OAuth login: `cheaper auth login <provider>`（`--method` で
  flow 選択、`--type oauth`）。browser auth_url flow と device
  code flow は provider が提示する method に従う。
- API-key login: `cheaper auth login <provider> --type api_key`
  で secret prompt に対話入力。`XAI_API_KEY` 等の env var
  resolve は Pi provider 側の api_key auth が行う。
- 確認: `cheaper auth status [provider]` は provider の auth
  capability（login method・subscription flag）と configured
  state、credential metadata を返す。model catalog は返さない。
- model 実利用確認: worker 実行（例 `cheaper bulk-read`）の
  result envelope `meta.model`（provider / id / tier）で
  観測する。

## Non-Functional Constraints

- provider 対応状況の静的網羅を避け、`auth status` を
  canonical 確認 command として示す（drift 防止）。
- xAI を OAuth provider として案内しない。xAI に言及する場合は
  API-key 側の例として扱う。
- guide は現行 runtime の contract と一致し、README・usage・
  configuration・cheaper-usage skill と矛盾しない。

## Risks and Trade-offs

- risk: 例示から xAI を外しても `auth status xai` が
  `["api_key","oauth"]` を返すため、reader が guide と runtime
  output の差に気づく可能性。mitigation: 例示は「例」であり
  網羅ではないこと、`auth status` が canonical surface 確認
  command であることを明記する。
- risk: 他 guide（usage/configuration）との矛盾残存。
  mitigation: 整合確認対象を実読みして最小修正する。
- trade-off: login method の静的網羅をしないため、利用者は
  `auth status` 実行が必要。drift 防止を優先する意図的な
  選択（ADR-034r2qodyqNY09yFc1ftZd）。

## Deferred Design Concerns

- 配布 skill（`packages/apm/.apm/skills/`）の全面同期は
  Issue #34 で扱う。本 design は cheaper-usage SKILL.md の
  整合確認のみ行う。
