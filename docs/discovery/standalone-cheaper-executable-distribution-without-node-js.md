---
id: "DISC-034z139D701zQBBGzr0upE"
type: "discovery"
status: "resolved"
title: "Standalone cheaper executable distribution without Node.js"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/71"
    - "https://nodejs.org/api/single-executable-applications.html"
    - "../ideas/distribute-cheaper-as-standalone-executables-including-apm-bundles.md"
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
  superseded-by: []
  related:
    - "./apm-self-contained-cli-distribution.md"
  refines: []
  refined-by: []
  derives-from:
    - "../ideas/distribute-cheaper-as-standalone-executables-including-apm-bundles.md"
  derived-by:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "../adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  verifies: []
  verified-by: []
  references:
    - "./windsurf-cascade-hooks-and-apm-hook-deployment.md"
    - "./cross-agent-hook-distribution-and-contract-matrix.md"
  defers: []
  deferred-by: []
---

# Standalone cheaper executable distribution without Node.js

## Exploration Goal

`cheaper` CLI を Node.js の事前インストールなしに実行できる standalone
executable として配布する方法を確定する。GitHub Release の通常配布に
加え、APM package の payload にも同梱し、APM 経由の利用でも Node.js
非依存にすることが scope。

trigger は issue #71。現行は `packages/apm/.apm/hooks/` 配下の JS
adapter（`pre-read-code*.js` 等）が `node ./cli/bin/cheaper.mjs` を
起動する構成で、consumer 環境に Node.js が必須。加えて Copilot /
VS Code 向け hook bundle では nested JSON asset が除外される APM 側の
制約があり、worker schema 等の外部 asset 配布に弱点がある。

## Key Issues and Assumptions

- **runtime の同一性**: 配布形態を増やしても CLI の振る舞いを分岐させない。
  現行 `scripts/bundle-cli.mjs` の esbuild single bundle を中間成果物と
  して再利用し、standalone / APM の両配布が同一 executable artifact を
  使う前提。
- **built-in resources の同梱**: `workers/`・`policy.yaml`・
  `version.json` を executable に埋め込み、runtime から read-only file
  として参照する必要がある。
- **builtinDir 解決**: 現行 `defaultBuiltinDir()` は
  `import.meta.url` 基準で bundle layout を前提とする。SEA VFS では
  `import.meta.dirname` が VFS 内の mount point を指すため、resource root
  解決を execution mode 非依存にする seam が要る（`CliHooks.builtinDir`
  は既存）。
- **dynamic import 経路**: Pi / OAuth 周辺に dynamic import を含む経路
  がある。esbuild bundle 時に静的に解決不能な import が残ると SEA 内で
  失敗するため、bundle 単一ファイル化の段階で全依存を内包できているかの
  検証が前提。
- **native addon / WASM**: SEA VFS から `.node` native addon は直接
  load できない（`process.dlopen` は real FS が必要）。tiktoken の WASM
  等の asset 読み込み経路が VFS 対応可能かは未検証。
- **hook adapter の脱 Node**: CLI 本体を executable 化しても
  `pre-read-code*.js` / `pre-run-command.js` / `routing-policy.js` が
  Node.js を要求するため、hook 判定ロジックを `cheaper hook ...`
  subcommand として executable 側へ寄せる必要がある。
- **APM の bin/ は使わない**: APM root `bin/` executable deploy は
  Claude Code + user-scope 専用（既存 discovery
  `apm-self-contained-cli-distribution` で確認済み）。cross-agent の
  canonical path は hook bundle の sibling asset + platform launcher。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Node 26 SEA + `useVfs` | toolchain が既に Node 26（mise.toml）で一致。`--build-sea` が v25.5+ で built-in。VFS（v26.9+）で `node:fs` 経由の asset read・`import.meta.dirname`・static/dynamic import が bundled assets に解決される。runtime semantics が現行と同一 | VFS は Stability 1.0（Early development）。`useSnapshot`/`useCodeCache` と併用不可（起動速度最適化は使えない）。platform ごとの binary 生成と（macOS なら）署名が要る | yes |
| Node.js 前提のまま維持 | 変更不要。現行 esbuild bundle がそのまま動く | issue の目的（Node.js 非依存）を満たさない。Copilot nested asset 制約も残る | no |
| `bun build --compile` | 単一 binary 生成が成熟。embedded assets 対応 | runtime が Node ではなく Bun になり、Pi SDK / tiktoken / OAuth 系依存の互換性検証が全てやり直し。配布物の動作が dev 時と変わるリスク | no |
| `deno compile` | 同上（単一 binary） | Node API compat layer 越しの実行になり runtime 差分が大きい。ESM/node: 互換の検証コストが高い | no |
| `pkg` / `nexe` / boxednode 等の旧来ツール | 実績がある | pkg は archived、nexe は unmaintained、boxednode は Node source からの build が重い。Node 26 の公式 SEA + VFS が存在する今選ぶ理由がない | no |

## Tentative Conclusions and Hypotheses

- (hypothesis) Node 26 SEA `mainFormat: "module"` + `useVfs: true` +
  `useSnapshot: false` + `useCodeCache: false` が canonical build path。
  `useVfs` は snapshot/codeCache と併用不可だが、issue の想定 config と
  一致し、cross-compile でもこれらは false 必須のため制約は実質なし。
- (confirmed, Node docs) VFS 有効時、main script は VFS root から実行
  され `import.meta.dirname` / `import.meta.url` が VFS 内 path を指す。
  `node:fs` の read 系と directory 走査が使え、write は `EROFS`。
  相対・bare specifier の static/dynamic import は bundled assets に
  解決される（`.mjs` または asset 内 `package.json` で ESM 判定）。
- (hypothesis) `main-sea.ts` を追加して `runCli(..., { builtinDir:
  import.meta.dirname })` を渡す最小差分で、通常 Node 実行と SEA 実行の
  resource root を分離できる。`packageVersion()` も VFS 内
  `version.json` を読めるようにする。
- (hypothesis) hook adapter / routing policy は `cheaper hook
  <event> --target <agent>` 系の CLI 内部 command へ移植し、
  `hooks.json` は platform launcher 経由で executable を呼ぶ形にする。
- (confirmed, Node docs) cross-compile（例: 別 host から darwin-arm64
  向け）では `useCodeCache`/`useSnapshot` は false 必須。

## Open Questions

- esbuild bundle に残る dynamic import / WASM load（tiktoken）が SEA VFS
  で動くか。PoC（issue Phase 1）で検証する。
- executable のサイズと起動時間。`useCodeCache` が使えないため bundle
  minify 済みであることが重要になる可能性。
- macOS 配布での codesigning / notarization の要否と CI での扱い。
- `cheaper hook` 移行時の target 別 hook contract（Windsurf / Copilot /
  Claude Code / Cursor の stdin/out contract）の回帰をどう固定するか。
- APM package 内 binary の freshness を CI がどう検証するか（現行の
  bundle 鮮度チェック相当）。

## Promotion Candidates

- [ ] spec-doc needed: standalone executable 配布の要件・AC（起動・
      built-in resource 解決・envelope/exit code 互換・APM install 後の
      Node 非依存）を formalize する
- [ ] adr-doc needed: Node 26 SEA + VFS を canonical build path に選ぶ
      決定（代替: Bun/Deno compile、旧来 packager、現状維持）と、APM
      payload / hook adapter 移行の構造を記録する

## Sources and Relations

- issue #71: 背景・方針・4 phase の実装順・AC の一次情報
- Node.js v26 API docs — Single executable applications: `--build-sea`、
  SEA config、VFS、ESM entry point、codeCache/snapshot 制約の根拠
- 先行 discovery `apm-self-contained-cli-distribution`: APM `bin/` が
  Claude Code + user-scope 専用で canonical path に使えないこと、esbuild
  bundle が APM hook bundle に同梱される現行構成の根拠
