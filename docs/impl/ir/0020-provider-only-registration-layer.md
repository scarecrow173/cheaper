---
id: "IMPL-0020"
type: "impl"
status: "completed"
title: "Provider-only registration layer (PLAN-0025)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/runtime-pi/src/provider-plugin.ts"
      - type: "file"
        path: "packages/runtime-pi/test/provider-plugin.test.ts"
    modified:
      - type: "file"
        path: "packages/runtime-pi/src/model-runtime.ts"
      - type: "file"
        path: "packages/runtime-pi/src/index.ts"
      - type: "file"
        path: "packages/runtime-pi/test/model-runtime.test.ts"
      - type: "file"
        path: "docs/guides/providers.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/bin/cheaper.mjs"
  implements:
    - "docs/tasks/0118-add-provider-plugin-contract-and-registration-order.md"
    - "docs/tasks/0119-provider-plugin-tests-and-resource-isolation.md"
    - "docs/tasks/0120-docs-status-sync-bundle-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0025-provider-only-registration-layer-implementation-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0030-provider-only-registration-layer.md"
    - "docs/specs/0025-provider-only-registration-layer.md"
    - "docs/adr/0028-provider-only-registration-via-explicit-bundled-plugin-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0025-provider-only-registration-layer.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Provider-only registration layer (PLAN-0025)

## Summary

Pi built-in に無い provider を cheaper が明示的に bundle
する plugin として `ModelRuntime` へ登録する provider-only
registration layer を `runtime-pi` に追加した。Pi の
general-purpose extension loader は有効化せず、
`CheaperResourceLoader` の空 resource 契約も変更していない。
plugin は最初の auth status / model resolution より前に
登録されるよう、create 時 refresh を遅延し plugin 登録後に
await refresh する初期化順を導入した。

## Context

TASK-0118/0119/0120（PLAN-0025 / SPEC-0025 / ADR-0028 /
DESIGN-0030）に従う。Issue #26。#27（Devin Local provider）
がこの contract の最初の consumer になる。bundled registry
は現時点で空 — 本変更は拡張点と初期化順の骨格のみを固定
する。

## Implementation

### Changes

- `packages/runtime-pi/src/provider-plugin.ts`（新規）:
  - `CheaperProviderPlugin`（`id`・`register(runtime,
    context)`）— plugin の責務境界を JSDoc で明記
  - `ProviderPluginContext`（`env`・`userConfigDir`）—
    credential 値は渡さない
  - `bundledProviderPlugins()` — 明示 import の registry。
    現時点では空 array を返す（discovery なし）
- `packages/runtime-pi/src/model-runtime.ts`:
  - `CreateCheaperModelRuntimeOptions.providerPlugins` を
    追加（既定: bundled registry。test seam）
  - `ModelRuntime.create({ ..., refreshOnCreate: false })` →
    各 plugin の `register` を順に await → create 時と同値
    の `runtime.refresh()`（`allowModelNetwork && PI_OFFLINE
    未設定` の allowNetwork、`modelRefreshTimeoutMs` の
    AbortController 合成、`signal` 伝播を mirror）を await
  - caller 明示の `refreshOnCreate: false` は manual
    refresh を省略
  - plugin register の throw は伝播（fail closed）
- `packages/runtime-pi/src/index.ts`: 新型を export
- `docs/guides/providers.md`: bundled provider plugin の
  仕組みと extension 非対応を追記

### Decisions / Notes

- `ProviderPluginContext` は `env`・`userConfigDir` のみ
  （YAGNI。`signal` 等は #27 で必要になれば追加）。
- Worker resource（tool/skill/prompt/theme/AGENTS.md）の
  登録面は `ModelRuntime` に存在せず `CheaperResourceLoader`
  も変更しないため、plugin からは構造的に持ち込めない。
- plugin 無し構成の外部挙動は現行と同一（manual refresh は
  create 時 refresh を等値 mirror）。

## Verification

- `pnpm typecheck`: green
- `pnpm lint`（biome）: 新規 error/warning なし
- `pnpm --filter @cheaper/runtime-pi test`: 268 passed（新規
  9 test 追加 — context 受け渡し・plugin provider が
  `getProviders`/`getAvailable`/`listProviderAuth` に現れる
  こと・`providerPlugins: []` で既定同一・register throw で
  reject・登録順序・deferred refresh の順序・caller 明示
  `refreshOnCreate: false`・allowNetwork mirror）
- `pnpm --filter cheaper test`: 既存 auth/run e2e 含め全件
  pass（plugin 無し構成の regression なし）
- `CheaperResourceLoader` の空 resource test は未変更で
  green（resource 隔離の regression 担保）
- `mise run check`: bundle 再生成 + freshness clean を含め
  全 green

## Acceptance Criteria 確認

- [x] provider-only plugin contract が `runtime-pi` に存在
      （`CheaperProviderPlugin`・`ProviderPluginContext`）
- [ ] bundled provider plugin を `ModelRuntime` へ登録できる
      （registry は実装済みだが bundled plugin は #27 で
      追加 — 現時点は空）
- [x] Pi built-in provider の挙動を壊さない（plugin 無し
      構成で全 test green）
- [x] `CheaperResourceLoader` は general Pi extensions を
      引き続き読み込まない（変更なし・既存 test green）
- [x] plugin provider が既存 `cheaper auth ...` 経由で
      扱える（`listProviderAuth` 経路で unit 検証。同一
      factory 経由のため auth/run の双方に波及）
- [x] plugin provider が通常の model availability / tier
      resolution に参加できる（`getAvailable` →
      `resolvePiModel` の通常経路）
- [x] provider registration の unit/integration test がある
