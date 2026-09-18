---
id: "DESIGN-034r05iHy2gdnp1MdNwuw6"
type: "design"
status: "approved"
title: "Devin Local provider plugin"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/devin-local-bundled-provider.md"
    - "../adr/devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/devin-local-bundled-provider.md"
    - "../adr/devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
  implemented-by: []
  depends-on:
    - "./provider-only-registration-layer.md"
    - "./pi-state-paths-under-userconfigdir-pi.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./auth-command.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/devin-local-bundled-provider.md"
    - "../adr/devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Devin Local provider plugin

## Context

SPEC-034r05iHK8bA7gAHzYnfP8 / ADR-034r05iHrZUE6WqcIaH1CU の実装設計。`devin` provider を
`CheaperProviderPlugin`（DESIGN-034r05iHy2gdnp0zOYnFNU）として実装し、
`bundledProviderPlugins()` に登録する。認証・catalog は
Devin CLI child process（isolated env）へ委譲し、model
transport は upstream `pi-devin`（MIT）を vendoring した
Connect-RPC stream で実装する。

## Scope

### In Scope

- `packages/runtime-pi/src/providers/devin/` 配下の新規
  module 群（vendored + adapted）
- `bundledProviderPlugins()` への `devin` plugin 登録
- NOTICE / attribution
- `packages/runtime-pi/test/providers/devin/` の test
- `docs/guides/providers.md` の Devin セクション

### Out of Scope

- `extensions/index.ts` 相当（slash command・event
  handler）は持ち込まない（ADR-034r05iHrZUE6WoaNDNjvX の責務境界）
- real-account test の必須化
- Devin Cloud Agent API・PAT・ACP

## Design

### Module 構成

```text
packages/runtime-pi/src/providers/devin/
├── wire.ts          # vendored: protobuf wire encode/decode + Connect frame
├── metadata.ts      # vendored: client metadata（ide/version）
├── jwt.ts           # vendored: GetUserJwt + cache
├── context-map.ts   # vendored: Pi Context → chat messages/tools
├── stream.ts        # vendored: streamDevin（Connect-RPC streamSimple）
├── cli.ts           # adapted: binary 解決・child env・runner seam
├── credentials.ts   # adapted: credentials.toml 読み取り・login flow
├── models.ts        # adapted: catalog parse・model 変換・bootstrap list
└── plugin.ts        # CheaperProviderPlugin 実装
```

vendored file（wire/metadata/jwt/context-map/stream）は
upstream `pi-devin`（kashyab12、MIT）由来を header comment
で明記する。LICENSE/NOTICE も追加。

### `cli.ts` — Devin CLI adapter

```ts
export interface DevinCliResult { stdout: string; stderr: string; code: number }
export interface DevinCliRunner {
  /** argv 分離起動。shell string は使わない。 */
  run(args: string[], opts: {
    env: NodeJS.ProcessEnv;
    timeoutMs?: number;
    inheritStdio?: boolean;
  }): Promise<DevinCliResult>;
}

export interface DevinCli {
  bin(): Promise<string>;          // 解決済み binary path（未検出は throw）
  version(): Promise<string | null>;
  run(args: string[], opts?: { timeoutMs?: number; inheritStdio?: boolean }):
    Promise<DevinCliResult>;
}
```

- binary 解決: `env.DEVIN_CLI` → PATH 上の `devin`
  （`execFile("devin", ...)` の PATH 解決に委ね、未検出時は
  install / `DEVIN_CLI` 設定を促す actionable error）
- `createDevinCli({ env, dataDir, runner? })`: runner 未指定
  時は `execFile`/`spawn` の本実装。test は fake runner を
  注入（Issue #27 の child-process adapter stub）
- child env: `{ ...env, XDG_DATA_HOME: dataDir }`、win32 は
  `LOCALAPPDATA: dataDir` 併用。`process.env` は変更しない

### `credentials.ts` — credential 読み取り + login flow

```ts
export function devinDataDir(userConfigDir: string): string
// <userConfigDir>/providers/devin/data

export function devinCredentialsPath(dataDir: string): string
// <dataDir>/devin/credentials.toml（Devin CLI の XDG rules）

export function readDevinCredentials(dataDir: string): DevinCredentials | null
// cheaper data dir 内のみ。global ~/.local/share/devin は読まない
```

`DevinCredentials`（`apiKey`・`apiServerUrl`・`webappHost`・
`apiUrl`・`path`）と TOML 解析は upstream と同じ。

`loginWithDevinCli(cli, dataDir)`:

1. credentials.toml が既にあり `devin auth status` が
   logged-in → 既存 credential を返す
2. `devin auth login` を `inheritStdio` + isolated env で
   実行（Devin CLI 自身の flow をそのまま使う）
3. 完了後 credentials.toml を再読。無ければ actionable
   error（exit code・path を含む）

### `models.ts` — catalog → Pi model 変換

- upstream の `DevinCatalog`/`DevinFamily`/`DevinVariant`・
  `variantKey`・`thinkingFromSuffix`・`parseCost`・
  `familyToModels`・`resolveModelUid`・`FALLBACK_MODELS`
  （bootstrap のみ・live catalog 優先）を vendored 流用
- `loadCliCatalog(cli)` は `devin models list --format json`
  を isolated env で実行し JSON parse。exit != 0 は stderr
  込み error、malformed JSON は actionable error
- `modelsFromCatalog` の返却型は `ProviderModelConfig[]`
  （pi-coding-agent 型、`ProviderConfigInput.models` と互換）

### `stream.ts` — vendored transport（ほぼ流用）

- `streamDevin(model, context, options)` をそのまま使う。
  credential は `options.apiKey`（Pi auth layer が
  `getApiKey` 経由で解決）のみを使い、file は読まない
- host は `options.env?.DEVIN_API_SERVER_URL` → 既定
  `https://server.codeium.com`（upstream 同様）

### `plugin.ts` — registration

```ts
export interface DevinProviderPluginDeps {
  /** test seam: child-process runner の差し替え。 */
  runner?: DevinCliRunner;
}

export function createDevinProviderPlugin(
  deps?: DevinProviderPluginDeps,
): CheaperProviderPlugin;
```

`register(runtime, context)`:

```text
dataDir = devinDataDir(context.userConfigDir)
cli = createDevinCli({ env: context.env, dataDir, runner: deps?.runner })
runtime.registerProvider("devin", {
  name: "Devin Local",
  api: "devin-local",
  baseUrl: "https://server.codeium.com",  // protocol 固定（catalog 非依存）
  models: FALLBACK_MODELS,                // bootstrap。live catalog が優先
  oauth: {
    name: "Devin CLI",
    login: async () => {
      const creds = await loginWithDevinCli(cli, dataDir);
      return { refresh: "", access: creds.apiKey,
               expires: Date.now() + 365d };
    },
    refreshToken: async (credentials) =>
      readDevinCredentials(dataDir)
        ? { ...same shape... } : credentials,
    getApiKey: (credentials) =>
      readDevinCredentials(dataDir)?.apiKey ?? credentials.access,
  },
  streamSimple: streamDevin,
  refreshModels: async () =>
    modelsFromCatalog(await loadCliCatalog(cli)),
})
```

`bundledProviderPlugins()` に `createDevinProviderPlugin()`
を追加する。

### auth CLI 統合

`cheaper auth status devin`・`login devin`・`logout devin`
は既存の `listProviderAuth`/`loginProvider`/`logoutProvider`
経路にそのまま乗る（oauth method が method として見える）。
login は `ExtensionOAuthConfig.login` が呼ばれ、内部で
`devin auth login` を inherit stdio で spawn — Devin CLI
自身の対話 flow がそのまま走る。logout は Pi の stored
credential 削除で availability が消える
（credentials.toml は cheaper data dir 内のため残存しても
global 汚染なし。次回 login で再利用される）。

## Error Handling

- `devin` binary 未検出 →
  `Devin CLI not found. Install the Devin CLI, or set DEVIN_CLI`
  系の actionable error（login/catalog 双方）
- `devin auth login` 失敗・credentials.toml 未生成 →
  exit code と期待 path を含む error
- `models list` exit != 0 → stderr 込み error、JSON parse
  失敗 → malformed catalog error
- stream 側の error は `streamDevin` 内で
  `type: "error"` event に変換（upstream 同様）

## Testing

fake runner（`DevinCliRunner` stub）と tmpdir `dataDir` で:

- auth status（credentials.toml 有無 + CLI status 出力）
- login invocation（argv・isolated env（`XDG_DATA_HOME` が
  dataDir）が runner に渡る・`process.env` 不変）
- credentials.toml 解析（TOML 文字列→apiKey/url）
- catalog JSON parsing・family→model 変換（thinking map・
  cost・contextWindow・maxTokens）
- `modelsFromCatalog(null)` → FALLBACK bootstrap
- malformed catalog・`models list` exit != 0 → error
- missing CLI → actionable error
- `register` が `registerProvider("devin", ...)` を呼ぶ
  （spy）・oauth.login が runner 経由で `devin auth login`
  を起動し credential を返す
- `refreshModels` が live catalog で model list を返す
- stream mapping: `mapContextToChat`（system collapse・
  tool_call・image）と `streamDevin` の apiKey 未指定 error
- logout 後 availability（stored credential 削除で
  getApiKey が呼ばれなくなる — plugin 側は
  credentials.toml 非依存の確認まで）

## Verification

- `pnpm typecheck` green
- `pnpm --filter @cheaper/runtime-pi test` green
- `pnpm --filter cheaper test` green（既存経路不変）
- `pnpm lint` で新規 error/warning なし
- `pnpm bundle` 再生成 → freshness clean
- `mise run check` green
