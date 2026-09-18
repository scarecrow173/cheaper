---
id: "TASK-0119"
type: "task"
status: "done"
title: "Provider plugin tests and resource isolation verification"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/0025-provider-only-registration-layer-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0118-add-provider-plugin-contract-and-registration-order.md"
  blocks:
    - "docs/tasks/0120-docs-status-sync-bundle-and-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0030-provider-only-registration-layer.md"
    - "docs/specs/0025-provider-only-registration-layer.md"
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
---

# TASK-0119: Provider plugin tests and resource isolation verification

## Work

TASK-0118 の contract と初期化順序を検証する unit/
integration test を追加する。

### 1. `packages/runtime-pi/test/provider-plugin.test.ts`（新規）

- fake plugin（`register` で `registerNativeProvider` する
  test double、`createFakePi` の provider を流用可）を
  `providerPlugins` option で注入:
  - `register` が runtime 実体と context
    （`env`・`userConfigDir`）を受け取る
  - plugin provider が返り値 runtime の `getProviders()` /
    `getAvailable()` に現れる（最初の availability 解決より
    前に登録済み）
  - `listProviderAuth(runtime)` に plugin provider が現れる
    （auth CLI がそのまま扱えることの unit 相当）
  - `providerPlugins: []` 注入で plugin 無し構成と同一
  - register が throw → `createModelRuntime` が reject
- test env は `CHEAPER_CONFIG_DIR` → tmpdir 隔離（既存
  `model-runtime.test.ts` と同じ方針）

### 2. `packages/runtime-pi/test/model-runtime.test.ts`（追加）

- `ModelRuntime.create` の spy で plugin 有無に関わらず
  `refreshOnCreate: false` が渡ること、および plugin 登録
  後に `refresh` が await される順序を検証
- caller 明示 `refreshOnCreate: false` で manual refresh が
  走らないこと

### 3. resource 隔離の確認

- `resource-loader.test.ts` の既存 test（空 extensions/
  skills/prompts/themes/AGENTS）が引き続き green であること
  を確認（本 task で変更しない。regression として残す）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0025-provider-only-registration-layer-implementation-plan.md`
- `docs/designs/0030-provider-only-registration-layer.md`

## Dependencies

- TASK-0118（contract・factory の実装が前提）

## Affected Files

- `packages/runtime-pi/test/provider-plugin.test.ts`（新規）
- `packages/runtime-pi/test/model-runtime.test.ts`

## Done When

- [x] fake plugin が `getAvailable` / `getProviders` /
      `listProviderAuth` に現れる test がある
- [x] plugin 無し構成（`providerPlugins: []`）で現行と同一
- [x] register throw → `createModelRuntime` reject の test
- [x] `refreshOnCreate: false` create → register → await
      refresh の順序 test
- [x] `CheaperResourceLoader` の空 resource test が green

## Verification

- [x] `pnpm --filter @cheaper/runtime-pi test` green
- [x] `pnpm typecheck` green
