---
id: "DESIGN-034z1EfqYaEBfyNvqZyFSD"
type: "design"
status: "approved"
title: "SEA executable build pipeline and embedded resource resolution"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "../adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./cli-bundle-layout.md"
    - "./dev-apm-project-and-distributable-package-layout.md"
    - "./apm-executable-payload-layout-and-platform-launchers.md"
    - "./cheaper-hook-subcommands-replacing-node-based-hook-adapters.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "../adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://nodejs.org/api/single-executable-applications.html"
  defers: []
  deferred-by: []
---

# SEA executable build pipeline and embedded resource resolution

## Context

SPEC-034z14nGIf6fXRtTskkrk8 / ADR-034z1xMK43jRs3D79Urhpc の実装設計
（issue #71 の Phase 1・2 に相当）。現行 `scripts/bundle-cli.mjs` が生成
する esbuild single-file ESM bundle を中間成果物として再利用し、
Node 26 SEA で platform 別 standalone executable を生成する。
built-in resources（`workers/`・`policy.yaml`・`version.json`）は SEA
assets として VFS に埋め込み、runtime から `node:fs` の read-only file
として参照する。

## Scope

### In Scope

- `scripts/bundle-cli.mjs` の責務分離: canonical JS bundle 生成と APM
  tree 生成を分け、bundle を SEA 入力として再利用する
- 新規 `scripts/build-executable.mjs`: SEA config 生成・asset 収集・
  `node --build-sea` / injection・platform 別 executable 出力
- SEA 用 entrypoint（`main-sea.ts` 相当）と `builtinDir` /
  `packageVersion()` の VFS 対応 resource root 解決
- built-in resources の SEA assets 化（`workers/`・`policy.yaml`・
  `version.json`）
- current host（開発機）での executable 生成 PoC と動作確認
- platform 別 artifact 命名規則の固定と GitHub Release workflow への接続、
  release artifact smoke test

### Out of Scope

- APM payload layout・launcher・`hooks.json` 変更（DESIGN-034z1EgnQwax4VamK2xmdT）
- hook adapter / routing policy の CLI subcommand 化
  （DESIGN-034z1EhuztlLSb2x2XXxgD）
- `useSnapshot` / `useCodeCache`（`useVfs` と併用不可）
- macOS codesigning / notarization の本格運用
- 複数 JS file を VFS 内 module として配置する設計（bundle は単一 ESM
  file のまま）

## Components and Boundaries

```text
CLI source (packages/cli)
   ↓ esbuild single ESM bundle（既存 scripts/bundle-cli.mjs の前半部）
canonical JS bundle
   ↓ scripts/build-executable.mjs
   ├─ sea-config.json（main / output / mainFormat / useVfs / assets）
   ├─ asset staging（workers/・policy.yaml・version.json）
   └─ node --build-sea または postject 系 injection
        ↓
   cheaper-<platform>-<arch>[.exe]
```

- **`scripts/bundle-cli.mjs`**: esbuild bundle の生成だけを責務とする。
  APM tree への配置は `build-apm-bundle.mjs` 側へ移す（後段 design 参照）。
- **`scripts/build-executable.mjs`**: target platform ごとに
  sea-config を生成し、bundle + assets を含む executable を出力する。
  target 指定と出力 dir の受け取りは argv/env ではなく明示的な script
  option とする。
- **SEA 用 entrypoint**: `import.meta.dirname`（VFS root を指す）を
  `runCli(..., { builtinDir })` に渡す薄い entry を追加し、通常 Node
  実行の `defaultBuiltinDir()` と分岐させない。runtime 側は
  `CliHooks.builtinDir` の既存 seam のみを使う。
- **`packageVersion()`**: builtinDir 基準で `version.json` を探索する
  形に揃え、SEA / 通常 / APM bundle の 3 形態で同一の解決規則を使う。

## Data and Control Flow

- build 時: `pnpm bundle`（canonical bundle）→ `build-executable.mjs`
  が platform matrix を回して executable を出力 → size / smoke 結果を
  記録
- runtime 時: executable 起動 → injected main script が VFS root から
  実行される → `import.meta.dirname` = `builtinDir` → `resolveConfigDirs`
  経由で `workers/`・`policy.yaml`・`version.json` を VFS 内 read-only
  file として解決
- SEA assets は `node:sea` の individual API ではなく VFS 経由で
  `node:fs` read のみ使う（write は `EROFS` で失敗する前提を test で固定）

## Non-Functional Constraints

- Node 26（`--build-sea` v25.5+・VFS v26.9+、mise.toml の locked
  version を前提）
- SEA config 初期値: `{ "mainFormat": "module", "useVfs": true,
  "useSnapshot": false, "useCodeCache": false }`。Pi / OAuth 周辺の
  dynamic import 経路があるため `useCodeCache` は立てない
  （`useCodeCache: true` では `import()` が動作しない）
- cross-compile target でも `useSnapshot` / `useCodeCache` は false 必須
- artifact 命名: `cheaper-darwin-arm64` / `cheaper-darwin-x64` /
  `cheaper-linux-arm64` / `cheaper-linux-x64` / `cheaper-windows-x64.exe`
  を最低ラインとし、target 追加は matrix 拡張で対応
- executable の size と起動時間を記録する（最適化は本設計の対象外だが
  計測値は残す）

## Risks and Trade-offs

- SEA VFS は Stability 1.0（Early development）。Node minor upgrade で
  挙動が変わる可能性 → CI の platform executable smoke test で検知する
- esbuild bundle に残る dynamic import / WASM（tiktoken）が VFS で動くか
  は未検証 → Phase 1 PoC で current host の executable を作り、
  built-in worker resolution・schema load まで到達することを確認してから
  後続 phase に進む
- executable は OS/arch ごとの artifact 行列になるため、release
  workflow の責務が増える → target matrix を data として分離し追加を
  容易にする

## Deferred Design Concerns

- `useCodeCache` / `useSnapshot` による起動最適化 | VFS と併用不可 |
  Node 側で併用可能になった時点 | 大きな bundle の起動遅延が残る
- native addon / WASM の VFS 内 load（`process.dlopen` は real FS
  必須のため VFS 外への退避が要る）| PoC で tiktoken が動くなら不要 |
  実際に VFS 内 load が必要になった時点 | tokenizer 等が使えない

## Verification Notes

- [ ] Node.js が PATH に無い環境で current host 用 executable が起動し、
      `--version` / `count-tokens` / `models` / `auth status` が動く
- [ ] built-in worker resolution / JSON schema load が executable 内
      resource から行える（fake/test runtime 経由で確認）
- [ ] worker run の stdout 1-envelope / stderr diagnostics / exit code が
      現行 CLI と一致する
- [ ] executable size を記録し、release artifact smoke test が CI で走る
- [ ] bundle → executable → APM tree の責務分離後も `pnpm bundle`
      の現行利用が壊れない
