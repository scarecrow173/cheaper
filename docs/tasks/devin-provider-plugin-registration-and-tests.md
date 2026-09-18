---
id: "TASK-034r05iHWkNF7zNWvK4AnL"
type: "task"
status: "done"
title: "Devin provider plugin registration and tests"
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
    - "docs/plans/devin-local-provider-plugin-implementation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/vendor-devin-protocol-modules-and-cli-credentials-adapters.md"
  blocks:
    - "docs/tasks/devin-docs-status-sync-bundle-and-final-verification.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/devin-local-provider-plugin.md"
    - "docs/specs/devin-local-bundled-provider.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/devin-local-bundled-provider.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHWkNF7zNWvK4AnL: Devin provider plugin registration and tests

## Work

`createDevinProviderPlugin` を実装して
`bundledProviderPlugins()` に登録し、fake runner による
unit test 一式を追加する。

### 1. `plugin.ts`（新規）

- `createDevinProviderPlugin(deps?: { runner?: DevinCliRunner })`
- `register(runtime, context)`:
  - `dataDir = devinDataDir(context.userConfigDir)`
  - `cli = createDevinCli({ env: context.env, dataDir,
    runner: deps?.runner })`
  - `runtime.registerProvider("devin", { name: "Devin Local",
    api: "devin-local", baseUrl, models: FALLBACK_MODELS,
    oauth: {...}, streamSimple: streamDevin,
    refreshModels })`（DESIGN-034r05iHy2gdnp1MdNwuw6 の shape）
- `bundledProviderPlugins()` に `createDevinProviderPlugin()`
  を追加

### 2. test（`packages/runtime-pi/test/providers/devin/`）

fake runner + tmpdir `CHEAPER_CONFIG_DIR` で:

- `plugin.test.ts`:
  - `register` が `registerProvider("devin", ...)` を呼ぶ
    （spy or 実 ModelRuntime）
  - `bundledProviderPlugins()` に `devin` が含まれる
  - `createModelRuntime` 経由で `devin` provider が
    `getProviders()` に現れる
  - oauth.login が fake runner 経由で `devin auth login`
    （argv 分離・`XDG_DATA_HOME` が dataDir）を起動し、
    fake が書いた credentials.toml から credential を返す
  - `refreshModels` が fake catalog を `devin models list
    --format json` から取得し model list を返す
  - `devin` provider が `listProviderAuth` に現れる
- `cli.test.ts`:
  - `DEVIN_CLI` env 解決、missing CLI → actionable error
  - child env に `XDG_DATA_HOME`/`LOCALAPPDATA`（win32）が
    dataDir、`process.env` 不変
  - `version()` が `devin version` を呼ぶ
- `credentials.test.ts`:
  - TOML parse（apiKey・apiServerUrl・default url）
  - dataDir 外の path を読まない（global path 非参照）
  - login: 既存 credentials + status logged-in → CLI 起動
    なしで返す
  - login: credentials 無し → `auth login` spawn → 再読
  - login 後 credentials.toml 無し → exit code + path 入り
    error
- `models.test.ts`:
  - catalog JSON → family→model 変換（thinkingLevelMap・
    cost・contextWindow・maxTokens・input）
  - `modelsFromCatalog(null)` → FALLBACK
  - malformed JSON・exit != 0 → error
- `context-map.test.ts` / stream smoke:
  - `mapContextToChat`（system・toolCall・image・toolResult）
  - `streamDevin` は `options.apiKey` 無しで error event
    （network 非依存）

### 3. logout 後 availability

- stored credential 削除後に devin model が `getAvailable`
  から消えることを確認（実 ModelRuntime + fake runner +
  tmpdir config で integration）

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/devin-local-provider-plugin-implementation-plan.md`
- `docs/designs/devin-local-provider-plugin.md`

## Dependencies

- TASK-034r05iHWkNF7zNyXgVkrI（module 実装が前提）

## Affected Files

- `packages/runtime-pi/src/providers/devin/plugin.ts`（新規）
- `packages/runtime-pi/src/provider-plugin.ts`
- `packages/runtime-pi/src/index.ts`
- `packages/runtime-pi/test/providers/devin/*.test.ts`（新規）

## Done When

- [x] `bundledProviderPlugins()` に `devin` が登録される
- [x] `registerProvider("devin", { oauth, streamSimple,
      refreshModels })` が呼ばれる
- [x] fake runner で login/catalog/data-dir 隔離を検証
- [x] missing CLI・malformed catalog の error test
- [x] logout 後 availability の test

## Verification

- [x] `pnpm --filter @cheaper/runtime-pi test` green
- [x] `pnpm typecheck` green
