---
id: "ADR-034r2qodyqNY09yFc1ftZd"
type: "adr"
status: "accepted"
title: "provider/auth guide は auth status を canonical surface として auth type 別に記述する"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/provider-auth-docs-runtime-sync.md"
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
    - "./absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
    - "./pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./readme-as-user-entrypoint-with-canonical-guides.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/provider-auth-docs-runtime-sync.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 30. provider/auth guide は auth status を canonical surface として auth type 別に記述する

## Context and Problem Statement

`docs/guides/providers.md` が runtime の auth surface と drift
している（Issue #33 / SPEC-034r2qodmutbif9L0DLskN）。決めるべき
こと: (a) OAuth 対応 provider の例示をどう記述するか、(b)
`auth status` の役割をどう定義して model 確認経路と分離するか、
(c) provider 対応状況を静的列挙と動的確認のどちらで示すか。

## Decision Drivers

- Issue #33: OAuth 例に `xai` を含めず API-key 側として扱うこと、
  `auth status` を model 一覧取得と読ませないことが指定される。
- `auth status` の実 output contract は
  `result.providers[].{providerId,name,methods,subscription,
  configured}` であり、model catalog を含まない
  （`packages/cli/src/auth.ts`）。
- Pi runtime の provider/auth surface は version 更新・custom
  `models.json` provider 追加で変わり得るため、guide の静的列挙は
  drift しやすい。
- 「同じ canonical information を複数文書へ詳細に複製しない」
  が本リポジトリの設計原則。

## Considered Options

### provider の auth type 記述方法

- **A: auth type を明確に分け、OAuth 例は実際に OAuth login
  可能な provider に限定する**（採用 — issue 指定）。`auth
  status` が各 provider の login method を返す canonical
  command であることを示し、例示は runtime で確認できるものに
  限定。xAI は API-key 側として扱う。
- **B: 全 provider を auth type 別に静的網羅する**。Pi runtime
  更新や `models.json` custom provider で即座に drift する。
- **C: provider 名を一切列挙しない**。具体例がなく導線として
  弱い。

### `auth status` と model 確認の分離

- **A: `auth status` は auth capability / configured state の
  確認コマンドと明記し、model の実利用確認は worker 実行結果の
  `meta.model` で説明する**（採用）。実 output contract と一致。
- **B: `auth status` で model 一覧も確認できるような記述を残す**。
  実装が model catalog を返さないため誤導。

## Decision Outcome

Chosen option: "auth type を明確に分けて OAuth 例を限定し、
`auth status` を auth capability の canonical 確認 command と
して位置づける", because Issue #33 がこの方針を approved
requirement として指定し、かつ実 output contract と一致する
唯一の選択肢であるため。

### Consequences

* Good, because guide が runtime の auth surface と一致し、
  `auth status` を canonical 確認 command として導線を張る
  ことで将来の provider 追加にも追従できる。
* Good, because xAI が OAuth provider として誤って案内されず、
  API-key provider と OAuth provider の混同が解消される。
* Bad, because provider ごとの login method を静的に網羅しない
  ため、利用者は `auth status` 実行が必要になる（意図的な
  trade-off — drift 防止を優先）。

### Confirmation

`docs/guides/providers.md` の記述と `cheaper auth status` の
実 output（`result.providers[].methods` 等）を突合し、`xai` が
OAuth 例に含まれないこと・model 一覧取得を示唆する記述がない
ことを確認する。

## Implementation Plan

* Affected paths: `docs/guides/providers.md`（主対象）、整合確認
  は `docs/guides/usage.md`・`docs/guides/configuration.md`・
  `README.md`・`packages/apm/.apm/skills/cheaper-usage/SKILL.md`
* Patterns to follow: 実 output contract に基づく記述、canonical
  command への導線（`auth status`）、`meta.model` による実利用
  確認
* Patterns to avoid: provider 対応状況の静的網羅、`auth status`
  で model 一覧を得られるような記述、xAI の OAuth 例示

## Verification

* [ ] `cheaper auth status` / `cheaper auth status xai` の実 output
  と guide の記述が一致する
* [ ] `rg "xai"` が guide の OAuth 例示に残っていない
* [ ] `auth status` で model 一覧を確認する旨の記述が残っていない
* [ ] `mise run check` が pass する

## Pros and Cons of the Options

### auth type 記述: Option A（採用）

* Good, because runtime で確認可能な例に限定するため drift しにくい。
* Good, because `auth status` を canonical 確認 command として
  示すことで網羅的列挙が不要になる。
* Bad, because 利用者が各 provider の method を知るには `auth
  status` 実行が必要。

### auth type 記述: Option B（静的網羅）

* Good, because guide だけで全 provider の method が分かる。
* Bad, because Pi runtime の version 更新や `models.json` custom
  provider で直ちに drift し、canonical source としての信頼を
  損なう。

### auth type 記述: Option C（列挙なし）

* Good, because drift リスクがゼロ。
* Bad, because 具体例がなく OAuth login の手順イメージを
  利用者が掴みにくい。

## More Information

- Issue #33: https://github.com/scarecrow173/cheaper/issues/33
- SPEC-034r2qodmutbif9L0DLskN（本 ADR の上流 spec）
- 実装 contract: `packages/runtime-pi/src/auth.ts`
  （`describeProviderAuth` / `listProviderAuth`）、
  `packages/cli/src/auth.ts`（`executeAuth` の status 分岐）

