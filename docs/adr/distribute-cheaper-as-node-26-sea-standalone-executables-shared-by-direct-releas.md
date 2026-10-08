---
id: "ADR-034z1xMK43jRs3D79Urhpc"
type: "adr"
status: "superseded"
title: "Distribute cheaper as Node 26 SEA standalone executables shared by direct release and APM hook bundle payload"
created: "2026-10-02"
updated: "2026-10-08"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/71"
    - "../discovery/standalone-cheaper-executable-distribution-without-node-js.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by:
    - "ADR-034zWr0viJFoTiUALvhE6N"
    - "migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  related:
    - "./one-dispatch-release-workflow.md"
    - "./distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  refines: []
  refined-by:
    - "./version-pinned-launcher-acquisition.md"
  derives-from:
    - "../discovery/standalone-cheaper-executable-distribution-without-node-js.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://nodejs.org/api/single-executable-applications.html"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Distribute cheaper as Node 26 SEA standalone executables shared by direct release and APM hook bundle payload

2026-10-04: issue #73-A の coordinator 承認により実装済み SEA 決定を accepted に整合した。
[version 固定 acquisition ADR](version-pinned-launcher-acquisition.md) が取得経路を refine する。
SEA/runtime の選択と既存 sibling payload は維持し、payload 削除は #73-C に委ねる。

## Context and Problem Statement

現行の `cheaper` 配布は 2 つの弱点を持つ。

1. consumer 環境に Node.js の事前インストールを要求する。APM hook は
   `node ./pre-read-code*.js "./cli/bin/cheaper.mjs"` として起動し、
   hook adapter（`pre-read-code*.js`、`pre-run-command.js`、
   `routing-policy.js`）自体も Node.js 上で動く。
2. Copilot / VS Code 向け hook bundle では nested JSON asset が除外される
   APM 側の制約があり、`workers/` や JSON schema のような外部 asset
   配布が脆弱。

issue #71 はこれを解消し、`cheaper` を Node.js 非依存の standalone
executable として、GitHub Release の通常配布と APM package payload の
両経路で同一 artifact を使って配布することを要求する。

## Decision Drivers

* consumer の runtime に Node.js を要求しないこと（hook 実行を含む）
* 配布経路が増えても CLI source / build artifact を分岐させないこと
* built-in resources（`workers/`・schema・`policy.yaml`・`version.json`）
  が executable に内包され、runtime から read-only file として参照できる
  こと
* toolchain（mise.toml の locked Node 26）と runtime semantics が現行
  Node.js と一致すること
* APM root `bin/` executable deploy は Claude Code + user-scope 専用で
  あり、cross-agent の canonical path に使えないこと（先行 discovery
  `apm-self-contained-cli-distribution` で確認済み）

## Considered Options

* Node 26 SEA（`mainFormat: "module"` + `useVfs: true`）
* Node.js 前提の現行 esbuild bundle 配布を維持
* `bun build --compile` / `deno compile` による別 runtime 単一 binary
* `pkg` / `nexe` / boxednode 等の非公式 packager

## Decision Outcome

Chosen option: "Node 26 SEA（`mainFormat: "module"` + `useVfs: true`）"。
toolchain が既に Node 26 であり、公式の `--build-sea` と VFS asset 埋め
込みで runtime semantics を変えずに Node.js 非依存を達成できるため。

採用構成:

* SEA config は `mainFormat: "module"`、`useVfs: true`、
  `useSnapshot: false`、`useCodeCache: false` から始める。
  `useVfs` は snapshot/codeCache と併用不可（Node docs 確認済み）であり、
  cross-compile でも両者は false 必須のため制約は実質なし。
* 現行 `scripts/bundle-cli.mjs` の esbuild single bundle を中間成果物と
  して再利用し、`bundle` → `executable` → `APM distribution tree` の
  責務を script ごとに分離する。standalone / APM の両配布は同一
  executable artifact を使い、別々に CLI を bundle しない。
* built-in resources は SEA assets として VFS に埋め込み、
  `node:fs` で read-only 参照する。resource root 解決は既存
  `CliHooks.builtinDir` seam を使い、SEA 用 entrypoint（例:
  `main-sea.ts` が `import.meta.dirname` を渡す）で最小差分に抑える。
  `packageVersion()` も VFS 内 `version.json` を読めるようにする。
* APM payload には platform 別 executable を hook bundle の sibling
  asset として配置し、platform を判定する薄い launcher（POSIX sh /
  PowerShell）を `hooks/` 直下に置く。APM root `bin/` は使わない。
* hook adapter / routing policy は `cheaper hook <event> --target <agent>`
  系の CLI 内部 command に移植し、`hooks.json` は launcher 経由で
  executable を呼ぶ形に変更する。旧 JS hook runtime file は移行後に削除
  する。

### Consequences

* Good, because consumer 環境に Node.js が不要になり、APM nested asset
  制約の影響も built-in resource の executable 内包で回避できる。
* Good, because standalone / APM 両経路が同一 artifact を使うため検証
  対象が一本化される。
* Bad, because SEA VFS は Stability 1.0（Early development）であり、
  Node minor upgrade での挙動変化を CI の smoke test で監視する必要が
  ある。
* Bad, because `useCodeCache` / `useSnapshot` が使えず起動速度最適化の
  余地が限られ、`.node` native addon / WASM の VFS 読み込みには
  個別対応が要る可能性がある。
* Bad, because platform 別 binary の生成・（macOS では）署名・release
  workflow 接続という配布基盤の保守コストが増える。

### Confirmation

issue #71 の Acceptance Criteria（spec と同一内容）を満たすこと。
具体的には Node.js が PATH に無い環境での executable 起動、
`--version` / `count-tokens` / `models` / `auth status` / worker run
envelope contract の現行互換、APM install 後の launcher 経由実行、
CI での platform executable smoke test と APM bundle freshness 検証。

## Implementation Plan

* Affected paths:
  * `scripts/bundle-cli.mjs`（canonical JS bundle 責務のみに整理）
  * 新規 `scripts/build-executable.mjs`（SEA config/assets/platform
    executable 生成）
  * 新規 `scripts/build-apm-bundle.mjs`（APM distribution tree 生成）
  * `packages/cli/src/cli.ts`・新規 `main-sea.ts` 相当の entrypoint
    （`builtinDir` を `import.meta.dirname` で注入）
  * `packages/cli/src/version.ts`（VFS 内 `version.json` 参照）
  * `packages/apm/.apm/hooks/`（`cheaper` / `cheaper.ps1` launcher、
    `cli/bin/<platform>/` 配下の executable、更新後 `hooks.json`）
  * hook adapter / routing policy の CLI 内部 command 化
    （`packages/cli` 側に `cheaper hook <event> --target <agent>` 系）
  * GitHub Release workflow / CI（platform executable smoke test・APM
    bundle freshness）
* Patterns to follow:
  * `CliHooks.builtinDir` の既存 seam を使った最小差分の resource root
    解決。execution mode ごとの過剰な互換 layer は作らない
  * 現行 `pnpm bundle` → esbuild single bundle の pipeline を維持し、
    その出力を SEA 入力として再利用する
  * hook の target 別 stdin/stdout/exit contract は現行 adapter の
    振る舞いをそのまま CLI 内部 command へ移植し、contract test で
    回帰を固定する
* Patterns to avoid:
  * standalone と APM で別々に CLI bundle を生成すること
  * APM root `bin/` executable deploy を canonical path にすること
  * SEA 用に Node 実行時の振る舞いを分岐させる互換 layer
  * `useSnapshot` / `useCodeCache` の有効化（`useVfs` と併用不可）
* Dependencies or configuration:
  * Node 26（`--build-sea`、VFS は v26.9+）— mise.toml locked version
  * injection は `--build-sea` または postject 系ツール（build script で
    吸収）
  * esbuild（root devDependency、既存）

## Verification

* [ ] Node.js が PATH に無い環境で generated executable が起動し、
      `--version` / `count-tokens` / `models` / `auth status` が動作する
* [ ] built-in worker definition / JSON schema / policy が executable 内
      resource から解決できる
* [ ] worker run の stdout 1-envelope / stderr diagnostics / exit code が
      現行 CLI と一致する
* [ ] APM install 後、supported target で launcher 経由の executable 実行
      が動作し、hook 実行に Node.js を要求しない
* [ ] target 別 hook contract の回帰 test が通る
* [ ] CI が platform executable smoke test と APM bundle freshness を
      検証する
* [ ] standalone / APM 両配布が同一 executable artifact を使うことを
      build script 構成で確認できる

## Pros and Cons of the Options

### Node 26 SEA（`mainFormat: "module"` + `useVfs: true`）

* Good, because toolchain の Node 26 と一致し、runtime semantics が現行
  Node.js と同一のまま配布できる。
* Good, because VFS（v26.9+）により built-in resources を `node:fs` の
  read-only file として扱え、`import.meta.dirname` 起点の既存
  `builtinDir` パターンを最小差分で再利用できる。
* Good, because ESM main script と static/dynamic import が bundled
  assets に解決される公式経路がある。
* Bad, because VFS は Stability 1.0 であり early development の挙動変化
  リスクを CI で監視する必要がある。
* Bad, because `useSnapshot` / `useCodeCache` と併用不可で起動速度
  最適化が制限され、native addon / WASM は VFS から直接 load できない。

### Node.js 前提の現行 esbuild bundle 配布を維持

* Good, because 変更不要で現行 pipeline がそのまま使える。
* Bad, because issue の目的（consumer の Node.js 非依存）を満たさず、
  APM nested asset 制約も残る。

### `bun build --compile` / `deno compile`

* Good, because 単一 binary 生成と embedded asset が成熟している。
* Bad, because runtime が Node.js ではなくなり、Pi SDK / tiktoken /
  OAuth 系依存の互換性検証が全てやり直しになる。配布物の動作が dev 時と
  変わるリスクが大きい。

### `pkg` / `nexe` / boxednode 等の非公式 packager

* Good, because かつて実績があった。
* Bad, because pkg は archived、nexe は unmaintained、boxednode は
  Node source build が重い。Node 26 の公式 SEA + VFS が存在する以上
  採用理由がない。

## More Information

- issue #71: 背景・方針・4 phase の実装順・AC の一次情報
- Node.js v26 API docs — Single executable applications: `--build-sea`
  （v25.5+）、`useVfs`（v26.9+）の asset/VFS semantics、ESM entry
  point、`useCodeCache`/`useSnapshot` 制約
- 先行 ADR `distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-
  inside-the-apm-hook-bun`: esbuild single bundle は本決定でも中間
  成果物として継続利用するため supersede せず `related` で接続する
- 先行 discovery `apm-self-contained-cli-distribution`: APM `bin/` が
  Claude Code + user-scope 専用である調査結果
