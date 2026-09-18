---
id: "TASK-034r05iHWkNF7zNyXgVkrI"
type: "task"
status: "done"
title: "Vendor Devin protocol modules and CLI/credentials adapters"
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
    - "docs/plans/devin-local-provider-plugin-implementation-plan.md"
  blocks:
    - "docs/tasks/devin-provider-plugin-registration-and-tests.md"
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/devin-local-provider-plugin.md"
    - "docs/specs/devin-local-bundled-provider.md"
    - "docs/adr/devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
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

# TASK-034r05iHWkNF7zNyXgVkrI: Vendor Devin protocol modules and CLI/credentials adapters

## Work

DESIGN-034r05iHy2gdnp1MdNwuw6 に従い、`packages/runtime-pi/src/providers/devin/`
配下に upstream `pi-devin`（MIT）の protocol module を
vendoring し、cheaper の isolated data dir・runner seam に
適合させた adapter を実装する。

### 1. vendored module（ほぼ流用・attribution header 付き）

- `wire.ts`: protobuf varint/tag/string/message encode・
  field iterator・Connect frame
- `metadata.ts`: client metadata build（ide=`devin-desktop`
  ・version）。`PRODUCT_JSON` の macOS path 参照は残して
  よい（存在しなければ fallback version）
- `jwt.ts`: `mintUserJwt`・`getCachedUserJwt`（host+apiKey
  key の module cache）
- `context-map.ts`: `mapContextToChat`（system→user collapse
  は stream 側）
- `stream.ts`: `streamDevin`（Connect-RPC GetChatMessage
  stream → AssistantMessageEventStream）。`options.apiKey`
  と `options.env?.DEVIN_API_SERVER_URL` のみ使用。file・
  global path は読まないこと

各 file の先頭に upstream 出所 + MIT attribution comment
を付ける。

### 2. `cli.ts`（adapted）

- `DevinCliRunner` interface（`run(args, { env, timeoutMs,
  inheritStdio }) → { stdout, stderr, code }`）
- default runner: `execFile`/`spawn`（argv 分離、shell
  string 不使用、`maxBuffer`・timeout 付き）
- `createDevinCli({ env, dataDir, runner? })`:
  - binary 解決: `env.DEVIN_CLI` → PATH（runner 側の PATH
    解決に委ね、`spawn` ENOENT を actionable error に変換）
  - child env: `{ ...env, XDG_DATA_HOME: dataDir }`、win32
    は `LOCALAPPDATA: dataDir` 併用。`process.env` 不変
  - `version()`: `devin version`（失敗は null）
- 未検出 error: `Devin CLI not found. Install the Devin CLI,
  or set DEVIN_CLI to the devin binary.` 系

### 3. `credentials.ts`（adapted）

- `devinDataDir(userConfigDir)` =
  `<userConfigDir>/providers/devin/data`
- `devinCredentialsPath(dataDir)` =
  `<dataDir>/devin/credentials.toml`
- `readDevinCredentials(dataDir)`（TOML 文字列 parse →
  apiKey/apiServerUrl/webappHost/apiUrl）。global path は
  参照しない
- `loginWithDevinCli(cli, dataDir)`:
  1. credentials あり + `devin auth status` logged-in →
     既存 credential
  2. `devin auth login` を `inheritStdio` + isolated env で
     実行
  3. credentials.toml 再読 → 無ければ exit code + path 入り
     actionable error

### 4. `models.ts`（adapted）

- upstream の catalog 型・`variantKey`・
  `thinkingFromSuffix`・`parseCost`・`familyToModels`・
  `resolveModelUid`・`FALLBACK_MODELS` を流用
- `loadCliCatalog(cli)`: `models list --format json` を
  isolated env で実行。exit != 0 → stderr 込み error、
  JSON parse 失敗 → malformed catalog error
- `modelsFromCatalog(catalog)` → `ProviderModelConfig[]`

### 5. attribution

- vendored file header comment + `NOTICE`（repo root）に
  upstream `pi-devin`（kashyab12、MIT）の copyright 表記
  を追加。repo に NOTICE が無ければ新規作成

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/devin-local-provider-plugin-implementation-plan.md`
- `docs/designs/devin-local-provider-plugin.md`

## Dependencies

- なし（chain の先頭）

## Affected Files

- `packages/runtime-pi/src/providers/devin/{wire,metadata,
  jwt,context-map,stream,cli,credentials,models}.ts`（新規）
- `NOTICE`（新規または追記）

## Done When

- [x] vendored module が global path を参照しない
      （credentials.toml は dataDir 経由のみ）
- [x] runner seam で child process が差し替え可能
- [x] child env に `XDG_DATA_HOME=<dataDir>` が渡り
      `process.env` は不変
- [x] `loadCliCatalog` が argv 分離で `models list
      --format json` を呼ぶ
- [x] attribution が存在する

## Verification

- [x] `pnpm typecheck` green
- [x] `pnpm lint` で新規 error なし
