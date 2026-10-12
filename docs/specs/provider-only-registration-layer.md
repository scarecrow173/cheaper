---
id: "SPEC-034r05iHK8bA7gz3bFqgXr"
type: "spec"
status: "implemented"
title: "Provider-only registration layer without Pi extensions"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/26"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./pi-state-under-cheaper-config-dir.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./auth-login-command.md"
    - "../designs/model-resolver.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/v2-productization-initiative-briefing.md"
  derived-by:
    - "./devin-local-bundled-provider.md"
    - "../designs/provider-only-registration-layer.md"
    - "../adr/provider-only-registration-via-explicit-bundled-plugin-contract.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Provider-only registration layer without Pi extensions

## What Should Be Built

Pi built-in に存在しない model provider を cheaper が明示的に
bundle した **provider plugin** として `ModelRuntime` へ登録
できる、provider-only の registration layer を `runtime-pi`
に追加する。

```text
Pi built-in providers
        │
        ▼
   ModelRuntime
        ▲
        │ registerProvider / registerNativeProvider
        │
Cheaper provider plugins（bundled のみ）
        │
        └── devin（follow-up issue）
```

Pi の general-purpose extension loader は有効化しない。
Worker session 側の `CheaperResourceLoader` は引き続き
extension / skill / prompt template / theme / AGENTS.md を
一切読み込まない。

## Why Now

Worker の実行環境を最小化するため、`CheaperResourceLoader`
は外部 resource を読まない設計になっている（DESIGN-034r05iHrZUE6WvaQsRRsM）。
一方で Devin Local 等の Pi built-in に無い provider を追加
する拡張点が必要になった（Issue #26、#27 が依存）。

Pi 0.85.1 の `ModelRuntime` は extension loader を介さず
`registerProvider()` / `registerNativeProvider()` を直接
呼べるため、general extension を有効化せずに provider 登録
だけを cheaper の明示的な初期化処理として実現できる。
#27（Devin Local provider）の前段として、この契約を先に
固定する。

## Users and Value

- cheaper の利用者: bundled provider が Pi built-in provider
  と同じ `cheaper auth status/login/logout`・model
  availability・tier resolution に参加し、別コマンドや
  別設定を意識せずに使える。
- parent agent / 開発者: provider 追加が「bundled plugin を
  1 つ書く」に限定され、任意コードを Worker runtime へ
  持ち込む経路は増えない。

## Scope

### In Scope

- `runtime-pi` 内の provider plugin contract
  （`id` + `register(runtime, context)`）と、
  `createModelRuntime()` が bundled plugin を
  `ModelRuntime` へ登録する初期化経路。
- provider plugin が最初の auth status / model availability /
  model resolution **より前**に登録される初期化順序の保証。
- bundled plugin が既存の `cheaper auth status`・
  `auth status <provider>`・`auth login <provider>`・
  `auth logout <provider>` から通常 provider と同様に
  扱えること（provider 別 CLI command は追加しない）。
- plugin provider が通常の model availability / tier
  resolution（`resolvePiModel` → core `resolveModel`）に
  参加できること。
- plugin が model provider 登録・provider 固有 auth flow・
  model catalog / refresh logic・`streamSimple` 等の model
  transport を登録できること。

### Out of Scope

- Pi general extension の discovery / loading
  （`CheaperResourceLoader.getExtensions()` を一般 extension
  loader に差し替えない）。
- external npm provider package の自動 discovery / load。
  将来許可する場合も明示 allowlist / explicit configuration
  を別途設計する（npm package は任意 Node.js code を実行
  できるため、Provider API の公開だけでは sandbox 代替に
  ならない）。
- plugin 経由の Pi tool / slash command / keyboard shortcut /
  Worker prompt・skill / session event handler / theme /
  AGENTS.md の持ち込み。
- Devin 固有の provider 実装（Issue #27）。
- `CheaperResourceLoader` の空 resource 契約の変更
  （`extensions: []`・`skills: []`・`prompts: []`・
  `themes: []`・`AGENTS: []` を維持する）。

## Requirements

1. **contract**: provider plugin の内部 contract を
   `runtime-pi` に定義する。plugin は `id` と
   `register(runtime, context)` を持ち、返すのは登録
   完了の Promise/void のみ。
2. **bundled のみ**: 登録対象は cheaper が明示的に
   bundle / import する plugin のみ。external discovery・
   config からの任意 module load は行わない。
3. **初期化順**: plugin の `register` は `ModelRuntime`
   の最初の availability/model resolution より前に完了
   する。必要に応じて create 時 refresh を遅延し、登録後
   に refresh する。
4. **auth 統合**: plugin provider が既存 auth CLI の全
   action（status / login / logout）から通常 provider と
   同じ metadata・auth flow で扱える。
5. **resource 隔離**: provider registration は
   `CheaperResourceLoader` の空 resource 契約を変えない。
   plugin は Worker resource（tool/skill/prompt/theme/
   AGENTS.md）を追加しない。
6. **built-in 不変**: plugin を 1 つも登録しない構成は
   現行と同一に動作する（Pi built-in provider の挙動を
   壊さない）。

## Acceptance Criteria

- [ ] provider-only plugin contract が `runtime-pi` に存在する。
- [ ] bundled provider plugin を `ModelRuntime` へ登録できる。
- [ ] Pi built-in provider の挙動を壊さない。
- [ ] `CheaperResourceLoader` は general Pi extensions を
      引き続き読み込まない。
- [ ] plugin provider が既存 `cheaper auth ...` 経由で
      扱える。
- [ ] plugin provider が通常の model availability / tier
      resolution に参加できる。
- [ ] provider registration の unit/integration test がある。

## Sources and Relations

- Issue #26: 要件の起点（責務境界・初期化順・CLI/auth 統合・
  non-goals・acceptance criteria）。
- SPEC-034r05iHK8bA7gxlRVZtAv / ADR-034r05iHrZUE6Wn59hpaOg: `createModelRuntime()` の userConfigDir
  解決と env seam — plugin context が再利用する初期化経路。
- SPEC-034r05iHK8bA7ghSWRbNU8 / ADR-034r05iHrZUE6W71rbZ6aR: `cheaper auth` の provider auth 統合 —
  plugin provider が乗る対象。
- DESIGN-034r05iHrZUE6WvaQsRRsM: `CheaperResourceLoader` の最小 resource 設計 —
  維持すべき制約。
- `packages/runtime-pi/src/model-runtime.ts`: `createModelRuntime()`
  と `ModelCatalog`。
- `packages/runtime-pi/src/testsupport.ts`: `createFakePi` の
  `registerNativeProvider` 利用例。
