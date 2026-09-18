---
id: "SPEC-034r05iHK8bA7gAHzYnfP8"
type: "spec"
status: "implemented"
title: "Devin Local as a bundled Pi provider"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/27"
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
    - "./provider-only-registration-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./auth-login-command.md"
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
  spec:
    priority: "should"
---

# Devin Local as a bundled Pi provider

## What Should Be Built

Devin CLI / Devin Local の model access（Devin Cloud Agent
API ではない）を、`devin` provider として SPEC-034r05iHK8bA7gz3bFqgXr の
provider-only registration layer 経由で bundled plugin に
追加する。

```text
cheaper auth login devin
        │
        ▼
Devin provider auth adapter
        │
        ▼
devin auth login（cheaper 専用 data dir 環境の child process）
        │
        ▼
Devin CLI credentials（<userConfigDir>/providers/devin/data/）
        │
        ▼
Pi CredentialStore（<userConfigDir>/pi/auth.json）
        │
        ▼
ModelRuntime / Worker
```

認証・catalog は Devin CLI に委譲し、Devin model backend
protocol への接続は provider 固有 `streamSimple` で実装する。
upstream `pi-devin`（MIT）の実装を参考・一部再利用する
（license / attribution を維持）。

## Why Now

cheaper の主要利用対象として Devin Local を追加したい
（Issue #27）。#25（Pi state の userConfigDir 集約）と
#26（provider-only registration layer）が揃ったため、
最初の bundled provider として実装できる。

## Users and Value

- cheaper の利用者: `cheaper auth login devin` で Devin CLI
  の通常 login flow を使い、`devin` provider の model が
  通常の tier/model resolution に参加する。
- parent agent: Worker 実行に Devin Local model を利用
  できる。

## Scope

### In Scope

- `devin` provider id の bundled `CheaperProviderPlugin`
- Devin CLI への認証委譲（`devin auth status`・
  `devin auth login`・`devin models list --format json`）
- Devin child process 専用の isolated data dir
  （`<userConfigDir>/providers/devin/data/`、
  `XDG_DATA_HOME` 等の platform rules に従う env override）
- live model catalog の取得と Pi model definition への変換
  （family/variant・thinking level map・context window・
  max output tokens・input capability・cost metadata）
- Devin model backend protocol への `streamSimple` 実装
  （upstream `pi-devin` を参考・MIT attribution 維持）
- `devin` binary の PATH 解決・`DEVIN_CLI` env override・
  未検出時の actionable error・`devin version` diagnostics
- fake CLI / child-process adapter stub による automated test
- docs（install prerequisite・login 方法・credential
  location・experimental/compatibility caveat）

### Out of Scope

- Devin Cloud Agent API / API v3
- `cog_...` PAT / Service User API key 対応
- cloud Devin session の作成・操作
- `pi-devin-acp` / ACP runtime
- Devin の server-side agent loop への task delegation
- Devin Desktop credential の自動 import / copy
- general-purpose Pi extension support
- global `~/.local/share/devin` credential の暗黙利用
- project-local `.cheaper/` への credential 保存
- real-account test の必須化（opt-in smoke のみ）

## Requirements

1. **登録**: `devin` provider は `bundledProviderPlugins()`
   経由で登録され、general Pi extension・Devin slash
   command はロードされない。
2. **auth**: `cheaper auth login devin` は Devin CLI 自身の
   login flow（child process、isolated env）を使う。独自
   OAuth 再実装はしない。account/CLI-backed auth であり
   API-key login と混同しない。
3. **保存場所**: Devin CLI credential は cheaper 専用
   data dir 配下（`<userConfigDir>/providers/devin/data/`）
   に閉じ込める。`global process.env` は変更せず child
   process にだけ env override を渡す。
4. **catalog**: `devin models list --format json` を live
   source of truth とし、auth 後・refresh 時に再取得する。
   bootstrap model metadata は最小限に留め、live catalog
   を必ず優先する。
5. **transport**: provider stream は model request/response
   の変換のみを責務とし、credential resolution は
   Pi/Cheaper auth layer（`options.apiKey`）に任せる。
   任意の global auth file を stream が直接探索しない。
6. **CLI dependency**: `devin` binary は PATH 解決 +
   `DEVIN_CLI` override。argv 分離で child process を
   起動し（shell string を使わない）、未検出時は
   actionable error、`devin version` を diagnostics に
   利用できる。
7. **tests**: 実 Devin account を必須にしない。fake CLI /
   stub で auth status・login invocation・data dir・
   catalog parsing・model mapping・stream mapping・
   missing CLI・malformed catalog・logout 後 availability
   を検証する。

## Acceptance Criteria

- [ ] bundled `devin` provider が provider-only layer から
      登録される
- [ ] `cheaper auth login devin` が Devin CLI の login flow
      を利用する
- [ ] Devin CLI credential が cheaper user config 配下へ
      保存される
- [ ] global Devin credential / Devin Desktop credential を
      暗黙利用しない
- [ ] live model catalog を `devin models list --format json`
      から取得できる
- [ ] catalog が Pi model definitions へ変換される
- [ ] Devin models が通常の cheaper model/tier resolution
      に参加する
- [ ] provider 固有 streaming が Worker execution で動作する
- [ ] general Pi extension / Devin slash command はロード
      されない
- [ ] fake CLI/stub を使った automated tests がある
- [ ] docs に install prerequisite・login 方法・credential
      location・experimental caveat を記載する

## Sources and Relations

- Issue #27: 要件の起点（対象範囲・auth 委譲・data dir・
  catalog 変換・transport 責務・test 方針・non-goals）。
- SPEC-034r05iHK8bA7gz3bFqgXr / ADR-034r05iHrZUE6WoaNDNjvX / DESIGN-034r05iHy2gdnp0zOYnFNU: provider-only
  registration layer — 登録経路。
- SPEC-034r05iHK8bA7gxlRVZtAv / ADR-034r05iHrZUE6Wn59hpaOg: userConfigDir/pi 集約 — plugin
  context の `userConfigDir` の由来。
- upstream `pi-devin`（kashyab12、MIT）: Devin CLI auth・
  catalog 変換・Connect-RPC stream の参照実装。
