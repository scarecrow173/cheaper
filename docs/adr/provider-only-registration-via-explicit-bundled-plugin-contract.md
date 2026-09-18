---
id: "ADR-034r05iHrZUE6WoaNDNjvX"
type: "adr"
status: "accepted"
title: "provider 拡張は Pi extension loader ではなく bundled provider plugin の明示 register で行う"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/provider-only-registration-layer.md"
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
    - "./pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/provider-only-registration-layer.md"
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

# 28. provider 拡張は Pi extension loader ではなく bundled provider plugin の明示 register で行う

## Context and Problem Statement

`CheaperResourceLoader` は Worker 実行時に Pi の extension /
skill / prompt template / theme / AGENTS.md を一切読み込まない
（DESIGN-034r05iHrZUE6WvaQsRRsM）。この制約は Worker 環境の最小化のために維持
したい一方、Pi built-in に無い provider（Devin Local 等、
Issue #27）を追加する拡張点が必要になった。Pi 0.85.1 の
`ModelRuntime` は `registerProvider(providerId, config)` /
`registerNativeProvider(provider)` を ResourceLoader 経由
なしに公開している。決めるべきこと: (a) provider 拡張の
経路（Pi extension loader vs 直接 register）、(b) 登録対象の
決定方法（discovery vs bundled list）、(c) 初期化順序の
保証方法。

## Decision Drivers

- Issue #26: general extension を有効化せず provider 登録
  だけを cheaper の明示的な初期化処理として行うこと、
  `CheaperResourceLoader.getExtensions()` を差し替えない
  こと、登録対象は bundled plugin のみとすることが明示
  されている。
- npm package は任意 Node.js code を実行できるため、
  Provider API の公開だけでは extension sandbox の代替に
  ならない（Issue #26 の指摘）。任意 module の load 経路を
  作らないことが Worker 隔離モデルの前提。
- plugin provider は最初の auth status / model resolution
  より前に登録されている必要がある（Issue #26 の初期化順
  要件）。`ModelRuntime.create()` は既定で create 時に
  refresh を実行するため、順序を制御する必要がある。
- `createModelRuntime()` は ADR-034r05iHrZUE6Wn59hpaOg で CLI・auth 双方の
  唯一の生成経路として集約済みであり、ここに登録処理を
  置けば全経路に自動的に波及する。

## Considered Options

### provider 拡張の経路

- **A: Pi extension loader を有効化し provider を extension
  として読み込む**。extension は tool・slash command・
  session event handler・theme 等も登録できてしまい、
  Worker 環境最小化の制約を破る。任意コード持ち込み経路に
  なるため Issue #26 が明示的に禁止。
- **B: cheaper 定義の provider plugin contract 経由で
  `ModelRuntime.registerProvider` / `registerNativeProvider`
  を直接呼ぶ**（採用）。plugin が触れる API は
  `ModelRuntime` の provider 登録面のみで、tool/prompt/
  resource の持ち込みは型レベルで表現不能になる。
- **C: `models.json` に provider を書いてもらう**。Pi
  native の provider config 経路であり plugin 契約は要ら
  ないが、provider 固有の auth flow・streamSimple・refresh
  logic を宣言的に持ち込めず、Devin CLI 連携のような
  code-backed provider を実現できない。

### 登録対象の決定方法

- **α: config/dir scan による discovery**。任意 module を
  load する経路になり sandbox を破るため不可（Issue #26
  non-goal）。
- **β: `runtime-pi` 内の bundled plugin registry（明示
  import）**（採用）。登録対象は cheaper が bundle した
  plugin に限定され、test は `providerPlugins` option で
  fake plugin を注入できる。
- **γ: CLI flag / env で plugin path を受け取る**。任意
  code load 経路になり Issue #26 の趣旨に反する。

### 初期化順序の保証方法

- **i: `ModelRuntime.create()` 後に plugin を register
  するだけ**。create 時 refresh は built-in のみで走り、
  plugin provider の availability は register 内部の
  fire-and-forget refresh に依存する — 最初の model
  resolution との間に race が残る。
- **ii: `refreshOnCreate: false` で create → plugin 登録 →
  `runtime.refresh()` を await して返す**（採用）。plugin
  登録が最初の availability 解決より前に確実に完了する。
  create 時 refresh の semantics（`allowModelNetwork`・
  `modelRefreshTimeoutMs`・`signal`）は cheaper 側で同値に
  mirror し、plugin が無い構成の外部挙動は現行と同一に
  保つ。caller が `refreshOnCreate: false` を明示した場合は
  manual refresh を省略する（static model のみを望む caller
  の契約を壊さない）。

## Decision Outcome

- `runtime-pi` に `CheaperProviderPlugin` contract
  （`id: string`・`register(runtime, context)`）と
  `ProviderPluginContext`（`env`・`userConfigDir`）を
  定義する。context は credential 値ではなく config 解決
  に必要な最小情報のみを渡す。
- `createModelRuntime()` は `providerPlugins` option
  （既定: bundled registry、現時点では空）を受け取り、
  `ModelRuntime.create({ refreshOnCreate: false })` →
  各 plugin の `register` を順に await → create 時と同値の
  `runtime.refresh()` を await して runtime を返す。
- plugin の `register` が throw した場合は runtime 生成を
  fail closed で失敗させる（半登録状態で進まない）。
- `CheaperResourceLoader` は変更しない。plugin は Worker
  resource を追加できない。
- external provider package の discovery / allowlist 機構は
  本 ADR では扱わない（将来別途設計）。
