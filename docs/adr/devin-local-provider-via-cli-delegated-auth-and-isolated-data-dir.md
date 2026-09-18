---
id: "ADR-034r05iHrZUE6WqcIaH1CU"
type: "adr"
status: "accepted"
title: "Devin Local provider は Devin CLI 委譲 auth・isolated data dir・vendored stream で実装する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/devin-local-bundled-provider.md"
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
    - "./provider-only-registration-via-explicit-bundled-plugin-contract.md"
    - "./pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/devin-local-bundled-provider.md"
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

# 29. Devin Local provider は Devin CLI 委譲 auth・isolated data dir・vendored stream で実装する

## Context and Problem Statement

Issue #27 は Devin Local model access を `devin` provider
として追加することを要求する。認証・model catalog は
Devin CLI（`devin auth`・`devin models list`）に委譲し、
model transport は Devin model backend protocol（Connect-RPC
+ protobuf）への `streamSimple` として実装する。決めるべき
こと: (a) Devin CLI credential の隔離方法、(b) stream
実装の入手元（自作 vs upstream 再利用 vs npm 依存）、
(c) plugin 内での credential 責務分担。

## Decision Drivers

- Issue #27: Devin CLI を cheaper 専用 data dir 環境の
  child process として実行し、credential を cheaper の
  user-scope storage 配下へ閉じ込める。global credential・
  Devin Desktop credential の暗黙利用禁止。`process.env`
  変更禁止・argv 分離起動。
- upstream `pi-devin`（kashyab12）は MIT license で公開
  されており、Issue #27 が「実装は参考にしてよい。直接
  再利用する場合は upstream license / attribution を適切
  に維持すること」と明示する。
- `pi-devin` は Pi extension package であり、そのまま
  install すると slash command・event handler まで登録
  される — provider 部分だけを取り出す必要がある
  （SPEC-034r05iHK8bA7gz3bFqgXr の責務境界）。
- Devin model backend は Connect-RPC + 非公開 protobuf
  schema で、wire encoding は upstream が実地検証済み。
  自作すると protocol 把握の費用と誤りリスクが大きい。

## Considered Options

### Devin CLI credential の隔離方法

- **A: `devin` child に `XDG_DATA_HOME=<userConfigDir>/
  providers/devin/data`（Windows では `LOCALAPPDATA` 併用）
  の env override を渡す**（採用）。Devin CLI は XDG/
  platform rules で `data/devin/credentials.toml` を書く
  ため、cheaper は path を探索せず env で閉じ込められる。
  `process.env` は変更せず child env のみ差し替える。
- **B: credential file を既定 path から copy**。global
  credential の暗黙 import にあたり Issue #27 が明示禁止。
- **C: `HOME`/`USERPROFILE` ごと差し替える**。強すぎる
  isolation — CLI が HOME 配下に置く他の正当な data まで
  隔離 dir に混在し、副作用範囲が不透明になる。

### stream 実装の入手元

- **α: `pi-devin` を npm 依存として追加し provider 部分を
  import**。同 package は library export を意図していない
  extension package で、internal module への依存は壊れ
  やすい。dependency 追加は repo ルール上も要確認。
- **β: `pi-devin` の実装を参考に stream を自作する**。
  MIT license だが protobuf schema・JWT mint・metadata
  field mapping は upstream が実地検証済みの値で、自作は
  費用と protocol 誤りリスクが大きい。
- **γ: `pi-devin` の source を vendoring し cheaper の
  隔離・plugin 契約に適合させる**（採用）。MIT license で
  attribution 維持可能。`wire.ts`・`jwt.ts`・`metadata.ts`・
  `context-map.ts`・`stream.ts` はほぼ流用し、
  `cli.ts`・`credentials.ts`・`models.ts` は cheaper の
  isolated data dir・plugin context・`registerProvider`
  経路に改変する。`extensions/index.ts`（slash command・
  event handler）は持ち込まない。

### credential 責務分担

- **i: stream が credentials.toml を直接読む**。transport が
  任意の global auth file を探索する構造になり、Issue #27
  の「credential resolution は Pi/Cheaper auth layer に任せ
  る」に反する。
- **ii: auth adapter のみが credentials.toml を読み、
  credential は Pi CredentialStore に永続化して stream は
  `options.apiKey` のみを使う**（採用）。upstream の
  `streamDevin` も同じ設計。login 時に `devin auth login`
  → credentials.toml を読み OAuthCredentials を返し Pi が
  auth.json に永続化。`getApiKey` は credentials.toml を
  再読して最新値を使う。

## Decision Outcome

- `devin` plugin は `bundledProviderPlugins()` に登録され、
  `runtime.registerProvider("devin", { oauth, streamSimple,
  refreshModels })` を呼ぶ。
- Devin CLI child process には `XDG_DATA_HOME=<userConfigDir>/
  providers/devin/data`（win32 は `LOCALAPPDATA` 併用）を
  渡す。credentials.toml は `<dataDir>/devin/credentials.toml`
  からのみ読み、global path は探索しない。
- `devin` binary は `DEVIN_CLI` env → PATH の順で解決し、
  argv 分離で spawn する。未検出時は install/set `DEVIN_CLI`
  を促す actionable error。
- login は `devin auth login` を inherit stdio で実行し、
  Devin CLI 自身の flow をそのまま使う。完了後に
  credentials.toml を読み OAuthCredentials として返す。
- catalog は `refreshModels` で `devin models list --format
  json` を取得し family/variant → Pi model definition へ
  変換する。bootstrap 用の最小 model list（upstream の
  FALLBACK_MODELS 相当）を初期登録し、live catalog を優先。
- vendored source は upstream `pi-devin`（MIT）由来を
  header comment と NOTICE で明記する。
- test seam は child-process runner adapter（execFile/spawn
  の差し替え）とし、fake `devin` executable も利用可能に
  する。
