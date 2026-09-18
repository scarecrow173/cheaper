---
id: "TASK-0118"
type: "task"
status: "done"
title: "Add provider plugin contract and registration order"
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
    - "docs/plans/0025-provider-only-registration-layer-implementation-plan.md"
  blocks:
    - "docs/tasks/0119-provider-plugin-tests-and-resource-isolation.md"
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
---

# TASK-0118: Add provider plugin contract and registration order

## Work

DESIGN-0030 に従い、provider-only plugin contract を新設し、
`createModelRuntime()` が bundled plugin を最初の auth/model
resolution より前に登録する初期化順序を実装する。

### 1. `packages/runtime-pi/src/provider-plugin.ts`（新規）

- `ProviderPluginContext`（`env: NodeJS.ProcessEnv`・
  `userConfigDir: string`）
- `CheaperProviderPlugin`（`id: string`・
  `register(runtime, context): void | Promise<void>`）
- `bundledProviderPlugins(): readonly CheaperProviderPlugin[]`
  — 現時点では空 array を返す。#27 で devin plugin を追加
- JSDoc で plugin の責務境界を明記（provider metadata /
  auth flow / catalog / transport の登録のみ。tool・prompt・
  theme・event handler・extension discovery は対象外）

### 2. `packages/runtime-pi/src/model-runtime.ts`

- `CreateCheaperModelRuntimeOptions` に
  `providerPlugins?: readonly CheaperProviderPlugin[]` を追加
- 初期化順序を DESIGN-0030 どおりに変更:
  1. `ModelRuntime.create({ authPath, modelsPath,
     modelsStorePath, refreshOnCreate: false, ...rest })`
     （caller 明示の `refreshOnCreate` は `...rest` で
     上書き可能だが plugin 登録順のため既定は `false`）
  2. `for plugin of (providerPlugins ?? bundledProviderPlugins())`
     → `await plugin.register(runtime, { env: env ??
     process.env, userConfigDir })`
  3. caller が `refreshOnCreate === false` を明示していな
     ければ `await runtime.refresh(...)` — create 時と同値
     の `allowNetwork`・`modelRefreshTimeoutMs`・`signal`
     合成を mirror する:
     - `allowNetwork = rest.allowModelNetwork === true &&
       (env ?? process.env).PI_OFFLINE === undefined`
     - timeout は `allowNetwork && modelRefreshTimeoutMs` の
       ときのみ `AbortController` + `setTimeout` で
       `AbortSignal.any([rest.signal, controller.signal])`
- plugin `register` の throw はそのまま伝播（fail closed）

### 3. `packages/runtime-pi/src/index.ts`

- `CheaperProviderPlugin`・`ProviderPluginContext`・
  `bundledProviderPlugins` を export

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0025-provider-only-registration-layer-implementation-plan.md`
- `docs/designs/0030-provider-only-registration-layer.md`

## Dependencies

- なし（chain の先頭）

## Affected Files

- `packages/runtime-pi/src/provider-plugin.ts`（新規）
- `packages/runtime-pi/src/model-runtime.ts`
- `packages/runtime-pi/src/index.ts`

## Done When

- [x] `CheaperProviderPlugin` / `ProviderPluginContext` /
      `bundledProviderPlugins` が export される
- [x] `createModelRuntime` が `refreshOnCreate: false` で
      create → plugin register → await refresh の順で動く
- [x] caller 明示の `refreshOnCreate: false` は manual
      refresh を省略する
- [x] plugin register の throw が fail closed で伝播する

## Verification

- [x] `pnpm typecheck` green
- [x] `pnpm --filter @cheaper/runtime-pi test` green（既存
      test が plugin 無し構成の不変性を担保）
