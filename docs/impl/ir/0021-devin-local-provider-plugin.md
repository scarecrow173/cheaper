---
id: "IMPL-0021"
type: "impl"
status: "completed"
title: "Devin Local provider plugin (PLAN-0026)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/cli.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/credentials.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/models.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/wire.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/jwt.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/metadata.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/context-map.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/stream.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/plugin.ts"
      - type: "file"
        path: "packages/runtime-pi/src/providers/devin/NOTICE"
      - type: "file"
        path: "packages/runtime-pi/test/providers/devin/cli.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/providers/devin/credentials.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/providers/devin/models.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/providers/devin/context-map.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/providers/devin/plugin.test.ts"
    modified:
      - type: "file"
        path: "packages/runtime-pi/src/provider-plugin.ts"
      - type: "file"
        path: "packages/runtime-pi/src/index.ts"
      - type: "file"
        path: "packages/runtime-pi/test/provider-plugin.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/model-runtime.test.ts"
      - type: "file"
        path: "docs/guides/providers.md"
      - type: "file"
        path: "docs/guides/configuration.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/bin/cheaper.mjs"
  implements:
    - "docs/tasks/0121-vendor-devin-protocol-modules-and-cli-credentials-adapters.md"
    - "docs/tasks/0122-devin-provider-plugin-registration-and-tests.md"
    - "docs/tasks/0123-devin-docs-status-sync-bundle-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0026-devin-local-provider-plugin-implementation-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0031-devin-local-provider-plugin.md"
    - "docs/specs/0026-devin-local-bundled-provider.md"
    - "docs/adr/0029-devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0026-devin-local-bundled-provider.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Devin Local provider plugin (PLAN-0026)

## Summary

Devin CLI / Devin Local の model 群を provider id `devin`
として cheaper に bundled provider plugin で追加した。
upstream `pi-devin`（MIT）の protocol 実装を
`packages/runtime-pi/src/providers/devin/` に vendor + 改変し、
auth は Devin CLI へ委譲、model catalog は
`devin models list --format json` の live 取得、stream は
vendored Connect/protobuf transport（`streamSimple`）で実装した。
Pi extension・slash command・event handler は持ち込まず、
credential は cheaper 専用 data dir と Pi CredentialStore に
閉じ込められる（global Devin credential・Devin Desktop
credential は暗黙利用しない）。

## Context

TASK-0121/0122/0123（PLAN-0026 / SPEC-0026 / ADR-0029 /
DESIGN-0031）に従う。Issue #27。Issue #26 の
provider-only registration contract（ADR-0028）の最初の
consumer。

## Implementation

### Changes

- `src/providers/devin/cli.ts`（新規、adapted）:
  `DevinCliRunner` seam（argv 分離起動、shell string 不使用）、
  `createDevinCli`（`DEVIN_CLI` env → `devin` on PATH）、
  `devinChildEnv`（`XDG_DATA_HOME=dataDir`、win32 は
  `LOCALAPPDATA` 併用。`process.env` 不変）、
  `DevinCliNotFoundError`（actionable message）
- `src/providers/devin/credentials.ts`（新規、adapted）:
  `devinDataDir`（`<userConfigDir>/providers/devin/data`）、
  `devinCredentialsPath`（`<data>/devin/credentials.toml`）、
  `readDevinCredentials`（isolated dir のみ）、
  `devinAuthStatus`、`loginWithDevinCli`（`devin auth login`
  を inherit stdio で実行後に credentials.toml を読む）
- `src/providers/devin/models.ts`（vendored + adapted）:
  `devin models list` 出力の parse、family→Pi model 変換
  （thinking suffix → `thinkingLevelMap`、cost/contextWindow/
  maxTokens 写像）、`FALLBACK_MODELS`（bootstrap）
- `src/providers/devin/{wire,jwt,metadata,context-map,stream}.ts`
  （vendored、NOTICE に MIT attribution）: Connect/protobuf
  wire、JWT mint、client metadata（`devin-desktop`）、
  context→chat 変換、`streamDevin`（`streamSimple`）
- `src/providers/devin/plugin.ts`（新規）:
  `createDevinProviderPlugin()` — `registerProvider("devin",
  {...})` で oauth（login/refreshToken/getApiKey）・
  `streamSimple`・`refreshModels`（credential 無しは
  FALLBACK、有りは live catalog）を登録。login 直後に
  live catalog で再登録
- `src/provider-plugin.ts`: `bundledProviderPlugins()` が
  `createDevinProviderPlugin()` を返す
- `src/index.ts`: plugin factory・`DevinCliRunner`・
  `DevinCliNotFoundError` を export
- `test/providers/devin/`: fake runner seam で CLI 解決・
  env 隔離・credentials parse・catalog 変換・context map・
  plugin 登録・login/logout・live catalog を検証（実 Devin
  アカウント不要）
- `docs/guides/{providers,configuration}.md`: Devin Local
  provider・`DEVIN_CLI`・data dir 隔離を追記

### Decisions / Notes

- vendored file は cheaper の strict 設定
  （`exactOptionalPropertyTypes`・`noUncheckedIndexedAccess`）
  に合わせて optional prop に `| undefined` を許容する等の
  型修正のみ行い、protocol 動作は upstream を保持。
- `refreshModels` は `credential` 無しの phase で FALLBACK を
  返し（CLI subprocess を起動しない）、credential 有りで
  `devin models list` を呼ぶ。login 直後は plugin 内で
  `registerProvider` を再実行して live catalog を即時反映。
- `getApiKey` は credentials.toml（rotated token 拾い）を
  優先し stored credential に fallback（DESIGN-0031 通り）。
  stream は `options.apiKey` のみ使用。
- `metadata.ts` の `PRODUCT_JSON` は macOS の Devin.app
  product.json（存在時のみ version 検出に使用、credential
  非依存の read-only）— upstream 動作を保持。
- `logout` は Pi credential 削除のみ。credentials.toml は
  isolated dir に残り次回 login で再利用（global 汚染なし）。

## Verification

- `pnpm typecheck`: pass
- `pnpm --filter @cheaper/runtime-pi test`: 305 pass
  （devin 5 ファイル 24 テスト含む）
- `pnpm --filter cheaper test`: 155 pass（e2e 含む）
- `pnpm lint`: pass（既存 warning 6 件のみ）
- bundled CLI `auth status devin` が `methods:["oauth"]`・
  `configured:false` を返すことを実機確認
- `pnpm bundle` 再生成、`mise run check` 予定
