---
id: "SPEC-034r2qodmutbif9L0DLskN"
type: "spec"
status: "implemented"
title: "Correct provider and auth documentation to match runtime capabilities"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/33"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/provider-auth-guide-runtime-sync.md"
  depends-on:
    - "./auth-login-command.md"
    - "./user-facing-docs-information-architecture.md"
    - "./pi-state-under-cheaper-config-dir.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./provider-only-registration-layer.md"
    - "./devin-local-bundled-provider.md"
    - "../adr/provider-auth-guide-auth-status-canonical-surface-auth-type.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Correct provider and auth documentation to match runtime capabilities

## What Should Be Built

`docs/guides/providers.md`（README / configuration / usage /
parent-agent skill から参照される canonical な provider/auth guide）
を、現在 pin している Pi runtime
（`@earendil-works/pi-ai@0.85.1` /
`@earendil-works/pi-coding-agent@0.85.1`）と cheaper CLI の
auth contract に一致させる。

## Why Now

Issue #33: main の実装と `docs/guides/providers.md` を突き合わせた
結果、2 つの drift が確認された。

1. OAuth 対応 provider の例に `xai` が含まれている。issue の
   判定では xAI はこの OAuth 例に含めるべきではなく、API-key
   provider 側として扱う。API-key provider と OAuth provider を
   混同しない記述に直す。
2. 「実際に `auth status` で確認して model 一覧を確認する」と
   読める記述があるが、`auth status`
   （`packages/runtime-pi/src/auth.ts` / `packages/cli/src/auth.ts`）
   が返すのは credential metadata・provider id/name・login
   methods・subscription flag・configured state であり、model
   catalog は返さない。実装 contract とずれている。

後続の parent-agent instruction 同期（Issue #34）がこの guide を
canonical source として参照するため、先にここを正す。

## Users and Value

- `cheaper` の利用者: provider/auth の設定手順を誤解しない。
- parent Coding Agent（配布 skill 経由）: CLI surface を正確に把握する。
- リポジトリ開発者: guide が runtime contract と一致することを
  信頼できる。

## Scope

### In Scope

- `docs/guides/providers.md` の修正:
  - OAuth 対応 provider の例を、現在の Pi runtime で実際に OAuth
    login 可能な provider に限定し、xAI を外す
  - API-key provider と OAuth provider を明確に分ける
  - xAI を説明する場合は API-key 側として扱う
  - `cheaper auth status [provider]` が auth capability /
    configured state の確認コマンドであることを明記する
  - model の実利用確認は worker 実行後の `meta.model` 等、現在
    実際に観測できる経路で説明する
  - provider 対応状況を静的に断定しすぎず、`auth status` が
    runtime の provider/auth surface を確認する canonical
    command であることを示す
- 整合確認（矛盾があれば最小限の修正）:
  - `docs/guides/usage.md`
  - `docs/guides/configuration.md`
  - `README.md`
  - `packages/apm/.apm/skills/cheaper-usage/SKILL.md`

### Out of Scope

- code behavior の変更（implementation code・test・hook script は
  変更しない）
- 配布 skill の全面同期（Issue #34 で扱う）
- runtime の provider/auth surface 自体の変更

## Requirements

- guide の記述は `cheaper auth status` の実 output contract
  （`result.credentials` + `result.providers[].{providerId,name,
  methods,subscription,configured}`）と一致すること
- xAI が OAuth provider として案内されないこと
- `auth status` が model 一覧を返すような記述がないこと
- model/provider の実利用確認方法が実際の CLI output contract
  （worker 実行結果 envelope の `meta.model`）と一致すること
- README・usage・configuration guide と矛盾しないこと

## Acceptance Criteria

- [ ] `docs/guides/providers.md` の OAuth 例示が runtime の auth
  capability と一致し、xAI が OAuth provider として案内されない
- [ ] `auth status` が model catalog を返すような記述が除去され、
  auth capability / configured state の確認コマンドであることが
  明記される
- [ ] model の実利用確認が `meta.model` 等の観測可能な経路で
  説明される
- [ ] `auth status` が provider/auth surface を確認する canonical
  command として示される
- [ ] README / usage / configuration / cheaper-usage skill と
  矛盾しない
- [ ] code behavior は変更しない（docs のみの差分）

## Sources and Relations

- Issue #33（approved requirement）:
  https://github.com/scarecrow173/cheaper/issues/33
- auth surface の実装: `packages/runtime-pi/src/auth.ts`、
  `packages/cli/src/auth.ts`
- 関連 spec: `auth-login-command`（cheaper auth command）、
  `pi-state-under-cheaper-config-dir`（credential 保存先）、
  `user-facing-docs-information-architecture`（guide の責務分離）
- 後続: Issue #34 は本 spec で正した guide を canonical source
  として distributed skill へ反映する
