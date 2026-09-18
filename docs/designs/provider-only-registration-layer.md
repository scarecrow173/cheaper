---
id: "DESIGN-034r05iHy2gdnp0zOYnFNU"
type: "design"
status: "approved"
title: "Provider-only registration layer"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/provider-only-registration-layer.md"
    - "../adr/provider-only-registration-via-explicit-bundled-plugin-contract.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/provider-only-registration-layer.md"
    - "../adr/provider-only-registration-via-explicit-bundled-plugin-contract.md"
  implemented-by: []
  depends-on:
    - "./pi-state-paths-under-userconfigdir-pi.md"
    - "./pi-execution-engine.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./auth-command.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/provider-only-registration-layer.md"
    - "../adr/provider-only-registration-via-explicit-bundled-plugin-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Provider-only registration layer

## Context

SPEC-034r05iHK8bA7gz3bFqgXr / ADR-034r05iHrZUE6WoaNDNjvX の実装設計。`runtime-pi` に provider-only
の plugin contract を追加し、`createModelRuntime()` が
bundled plugin を `ModelRuntime` へ登録する。Pi の
extension loader は有効化せず、`CheaperResourceLoader` の
空 resource 契約（DESIGN-034r05iHrZUE6WvaQsRRsM）は変更しない。runtime 生成
経路は `cli.ts`（run）と `auth.ts`（auth）の 2 箇所のみで、
いずれも `createModelRuntime({ env })` を通るため、登録処理
を factory に閉じれば全経路に自動的に波及する。

## Scope

### In Scope

- `packages/runtime-pi/src/provider-plugin.ts`（新規）:
  plugin contract・context・bundled registry
- `packages/runtime-pi/src/model-runtime.ts`:
  `providerPlugins` option と初期化順序（deferred refresh）
- `packages/runtime-pi/src/index.ts`: 新型の export
- `packages/runtime-pi/test/provider-plugin.test.ts`（新規）:
  contract・登録順・resource 隔離の test
- `packages/runtime-pi/test/model-runtime.test.ts`:
  plugin 登録経路の test 追加
- `packages/cli/test/e2e/`（必要な範囲）: plugin provider が
  auth CLI から見えることの確認

### Out of Scope

- `CheaperResourceLoader` の変更
- external plugin discovery / allowlist 機構
- Devin 固有 provider の実装（Issue #27）
- CLI への provider 別コマンド追加

## Design

### 新規 module: `provider-plugin.ts`

```ts
import type { ModelRuntime } from "@earendil-works/pi-coding-agent";

/** plugin が provider 登録に使う最小 context。credential 値は渡さない。 */
export interface ProviderPluginContext {
  /** runtime 生成時に注入された env（`process.env` 既定）。 */
  readonly env: NodeJS.ProcessEnv;
  /** cheaper の user-scope config dir。plugin 固有の永続 data はこの配下。 */
  readonly userConfigDir: string;
}

/**
 * cheaper が明示的に bundle する provider plugin の contract。
 * register が触れる API は ModelRuntime の provider 登録面のみ。
 * tool / slash command / prompt / theme / event handler の
 * 登録面は contract に存在しない。
 */
export interface CheaperProviderPlugin {
  readonly id: string;
  register(
    runtime: ModelRuntime,
    context: ProviderPluginContext,
  ): void | Promise<void>;
}

/**
 * cheaper が明示的に bundle する provider plugin の一覧。
 * #27 で devin plugin をここへ追加する。discovery はしない。
 */
export function bundledProviderPlugins(): readonly CheaperProviderPlugin[] {
  return [];
}
```

### 初期化順序（`createModelRuntime`）

```text
1. resolveUserConfigDir({ env }) → resolvePiStatePaths → mkdir(0700)
2. ModelRuntime.create({ authPath, modelsPath, modelsStorePath,
     refreshOnCreate: false, ...rest })
3. for plugin of (providerPlugins ?? bundledProviderPlugins()):
     await plugin.register(runtime, { env: env ?? process.env,
                                      userConfigDir })
4. caller が refreshOnCreate: false を明示していなければ
     await runtime.refresh({ allowNetwork, signal })
5. return runtime
```

`allowNetwork` は `ModelRuntime.create` の create 時 refresh
と同値に mirror する:

```ts
const allowNetwork =
  rest.allowModelNetwork === true &&
  (env ?? process.env).PI_OFFLINE === undefined;
```

`modelRefreshTimeoutMs` も同様に `AbortController` + `setTimeout`
で mirror する（create 内部と同じ signal 合成:
`AbortSignal.any([options.signal, controller.signal])`）。

caller が `refreshOnCreate: false` を明示した場合、step 4 を
省略する（static model のみを望む caller の契約を維持）。
plugin の `register` が throw した場合はそのまま伝播し、
runtime 生成は fail closed で失敗する。

`providerPlugins` option の型:

```ts
export interface CreateCheaperModelRuntimeOptions extends CreateModelRuntimeOptions {
  env?: NodeJS.ProcessEnv;
  /** bundled registry を差し替える test seam。省略時は bundled plugin。 */
  providerPlugins?: readonly CheaperProviderPlugin[];
}
```

### plugin ができること / できないこと

`register` に渡すのは `ModelRuntime` 実体だが、contract 上の
意図する利用面は以下に限定する（JSDoc で明記）:

- `runtime.registerProvider(providerId, config)` —
  provider metadata・auth flow・model catalog・`streamSimple`・
  `refreshModels` を `ProviderConfigInput` で登録
- `runtime.registerNativeProvider(provider)` —
  `Provider` object 実装の登録

Worker resource（tool/skill/prompt/theme/AGENTS.md）の登録面は
`ModelRuntime` に存在せず `CheaperResourceLoader` も変更しない
ため、plugin からは構造的に持ち込めない。

### auth / model resolution との統合

`cheaper auth status/login/logout`・`runWorker` はいずれも
`createModelRuntime` 経由で runtime を得るため、CLI 側の
変更は不要。plugin provider は `listProviderAuth`・
`describeProviderAuth`・`loginProvider`・`logoutProvider`・
`getAvailable` → `resolvePiModel` の通常経路にそのまま乗る。

## Error Handling

- plugin `register` の throw → runtime 生成失敗（fail closed）
- plugin 未登録（bundled が空）の構成 → 外部挙動は現行と同一
  （deferred refresh は create 時 refresh と同値に mirror）

## Testing

- `provider-plugin.test.ts`:
  - fake plugin が `register` 呼び出しで runtime と context
    （`env`・`userConfigDir`）を受け取る
  - plugin が `registerNativeProvider` した provider が
    `getAvailable` / `getProviders` に現れる（init 順序保証）
  - `providerPlugins: []` 注入で plugin 無し構成と同一
  - register throw → `createModelRuntime` が reject
- `model-runtime.test.ts`（既存に追加）:
  - `refreshOnCreate: false` で create → 登録 → await refresh
    の順序を spy で検証（plugin provider が最初の
    `getAvailable` 前に登録済み）
- `resource-loader` の既存 test が空 resource 契約を担保
  （変更しないため既存 test でカバー）
- cli e2e: `auth status` に plugin provider が現れることは
  `listProviderAuth` 経由で unit/integration で担保し、
  e2e は既存の `CHEAPER_TEST_FAKE_STREAM`（faux provider を
  `registerNativeProvider` する既存経路）が同等の検証を
  兼ねるため新規 e2e は最小限とする

## Verification

- `pnpm typecheck` green
- `pnpm --filter @cheaper/runtime-pi test` green（新規 test 含む）
- `pnpm --filter cheaper test` green（既存 auth/run 経路不変）
- `pnpm lint` で新規 error/warning なし
- `pnpm bundle` 再生成 → freshness clean
- `mise run check` green
