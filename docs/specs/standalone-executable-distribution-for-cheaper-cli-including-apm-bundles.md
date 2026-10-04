---
id: "SPEC-034z14nGIf6fXRtTskkrk8"
type: "spec"
status: "superseded"
title: "Standalone executable distribution for cheaper CLI including APM bundles"
created: "2026-10-02"
updated: "2026-10-04"
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
  depends-on:
    - "./apm-self-contained-cli-distribution.md"
  blocks: []
  supersedes: []
  superseded-by: ["SPEC-034zWqZR8Dmu9TXTR2qcsg"]
  related:
    - "../adr/distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
    - "../adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
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
---

# Standalone executable distribution for cheaper CLI including APM bundles

2026-10-04: issue #73-A の coordinator 承認で実装済み standalone 要件を approved に整合した。
[version 固定 acquisition spec](version-pinned-launcher-acquisition.md) が binary 不在時の取得契約を refine する。
既存 sibling payload の削除と配布 workflow の移行は #73-B/C の対象。

## What Should Be Built

`cheaper` CLI を、consumer 環境に Node.js の事前インストールを要求しない
platform 別 standalone executable として配布する。配布経路は
GitHub Release 等の通常配布と、APM package payload への同梱の 2 系統で、
両経路が同一の CLI source / build artifact を基準にする。

合わせて、APM hook 経路の Node.js 依存も除去する。現在
`packages/apm/.apm/hooks/` 配下の JS adapter（`pre-read-code*.js`、
`pre-run-command.js`、`routing-policy.js`）は `node` 実行を前提とする
ため、これらの hook 判定・routing ロジックを CLI executable 側の
内部 command へ移し、hook 実行全体を Node.js 非依存にする。

## Why Now

- 現行の APM hook は `node ./pre-read-code*.js "./cli/bin/cheaper.mjs"`
  で起動するため、Node.js を持たない環境では cheaper の delegation が
  使えない。
- Copilot / VS Code 向け hook bundle では nested JSON asset が除外される
  APM 側の制約があり、worker schema 等の外部 asset 配布が脆弱。
  built-in resources を executable に内包すればこの影響を受けにくい。
- toolchain は既に Node 26（mise.toml の locked version）であり、公式の
  standalone executable 経路が利用可能な状態にある。

## Users and Value

- **consumer 環境の parent coding agent / end user**: Node.js 非インストール
  環境でも APM hook 経由で bulk-read 等の delegation が動作する。
- **通常配布の利用者**: GitHub Release から単一 binary を取得するだけで
  cheaper CLI を実行できる。
- **cheaper メンテナ**: 配布形態が増えても CLI の実装・検証は単一の
  artifact に集約される。

## Scope

### In Scope

- platform 別 standalone executable（darwin-arm64/x64、linux-arm64/x64、
  windows-x64 を最低ラインとし、必要に応じて target を追加できる構成）
- built-in resources（`workers/`・input/output schema・`policy.yaml`・
  `version.json`）を executable に埋め込み、runtime から read-only
  file として参照できること
- 通常 Node 実行と standalone 実行で built-in resource root を正しく
  解決できること（既存 `CliHooks.builtinDir` seam を利用した最小差分）
- 現行 esbuild single bundle を中間成果物として再利用し、standalone /
  APM の両配布が同一 executable を使う build pipeline
- APM package への platform 別 executable 同梱と、platform を判定する
  薄い launcher（POSIX / PowerShell）
- hook adapter / routing policy の CLI 内部 command への移植と、
  `hooks.json` から executable 呼び出しへの切り替え
- GitHub Release workflow への artifact 接続と smoke test
- README / usage / parent-agent integration docs の新配布形態への整合

### Out of Scope

- APM root `bin/` executable deploy を canonical path にすること
  （Claude Code + user-scope 専用のため採用しない。将来 Claude Code 向け
  convenience として PATH 導線を追加する余地は残す）
- model / provider / auth の振る舞いの再設計
- 現段階で不要な backward compatibility layer
- macOS codesigning / notarization の本格運用（必要可否の確認と CI 方針の
  決定まで。詳細運用は release 手順側の課題）
- Epic #68（Tool Runtime / Workflow Engine）との機能連携。resource root
  解決が将来動く可能性への過度な最適化も行わない

## Requirements

- standalone executable は Node.js が PATH に無い環境で起動し、
  `--version`・`count-tokens`・`models`・`auth status` 等の non-execution
  path と worker run の envelope contract が現行 CLI と一致する
- built-in worker definition・JSON schema・policy が executable 内
  resource から解決でき、外部 `workers/` / schema file の欠落に依存しない
- standalone distribution と APM distribution が同じ platform executable
  artifact を利用し、別々に CLI を bundle しない
- APM install 後、supported target で platform launcher 経由の executable
  呼び出しが動作し、最終状態で hook 実行に Node.js を要求しない
- target 別 hook contract（Windsurf / Copilot / Claude Code / Cursor の
  stdin/stdout/exit contract）の回帰が test で固定される
- CI が platform executable の smoke test と APM bundle freshness を
  検証する

## Acceptance Criteria

- [ ] GitHub Release から取得した `cheaper` / `cheaper.exe` が Node.js が
      PATH に無い環境で起動できる
- [ ] `--version`、`count-tokens`、`models`、`auth status` が standalone
      binary で動作する
- [ ] built-in worker definition / JSON schema / policy が standalone
      binary から解決できる
- [ ] worker run の stdout 1-envelope / stderr diagnostics / exit code
      contract が現行 CLI と一致する
- [ ] APM install 後、supported target で cheaper executable を実行できる
- [ ] APM-installed cheaper の worker 実行が外部 `workers/` / schema
      assets の欠落に依存しない
- [ ] 最終状態で APM hook 実行に Node.js の事前インストールを要求しない
- [ ] standalone distribution と APM distribution が同じ platform
      executable artifact を利用する
- [ ] CI が platform executable の smoke test と APM bundle freshness を
      検証する
- [ ] README / usage / parent-agent integration docs が新しい配布形態と
      一致する

## Deferred Design Concerns

- Claude Code 向け bare `cheaper` PATH 導線 | APM canonical path が hook
  bundle sibling executable に決まったため | marketplace plugin 等で便利
  性要求が上がった時点 | なし
- executable の起動速度最適化（snapshot / code cache） | VFS との併用不可
  制約があるため初版では使わない | Node 側で VFS + cache 併用が可能に
  なった時点 | 大きな bundle で起動が遅い場合に体感劣化の余地

## Sources and Relations

- issue #71: 背景・目的・方針・実装順・AC の一次情報
- discovery `standalone-cheaper-executable-distribution-without-node-js`:
  代替案比較と SEA VFS の検証結果
- 先行 spec `apm-self-contained-cli-distribution` / ADR
  `distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun`:
  現行 esbuild bundle の APM 同梱構造（本 spec はその上位の配布形態を
  追加する。関係は depends-on / related で表現し、supersede しない）
