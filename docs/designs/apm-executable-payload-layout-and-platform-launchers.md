---
id: "DESIGN-034z1EgnQwax4VamK2xmdT"
type: "design"
status: "draft"
title: "APM executable payload layout and platform launchers"
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
  depends-on:
    - "./sea-executable-build-pipeline-and-embedded-resource-resolution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./dev-apm-project-and-distributable-package-layout.md"
    - "./cli-bundle-layout.md"
    - "./cross-agent-distribution-verification.md"
    - "./sea-executable-build-pipeline-and-embedded-resource-resolution.md"
    - "./cheaper-hook-subcommands-replacing-node-based-hook-adapters.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "../adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# APM executable payload layout and platform launchers

## Context

SPEC-034z14nGIf6fXRtTskkrk8 / ADR-034z1xMK43jRs3D79Urhpc の実装設計
（issue #71 の Phase 3 に相当）。APM root の `bin/` executable deploy は
Claude Code + user-scope 専用で cross-agent の canonical path に使えない
ため、platform 別 executable を hook source bundle の sibling asset と
して `packages/apm/.apm/hooks/` 配下に同梱し、薄い platform launcher
経由で呼び出す。

## Scope

### In Scope

- `packages/apm/.apm/hooks/` の最終 payload layout: `hooks.json` +
  platform launcher（`cheaper` / `cheaper.ps1`）+ `cli/bin/<platform>/`
  配下の platform 別 executable
- POSIX launcher: `uname -s` / `uname -m` で platform/arch を判定し、
  対応 executable に `exec` で引数をそのまま引き渡す
- PowerShell launcher: `win32-x64` の `cheaper.exe` を起動し
  exit code を伝播する
- `scripts/build-apm-bundle.mjs`: canonical JS bundle と platform
  executable を APM distribution tree に組み立てる責務
- APM install 後の path / executable mode / launcher 経由実行の target 別
  検証
- `cheaper-usage` skill の CLI location guidance を executable 基準へ更新
- APM bundle の binary freshness を CI が検証する仕組み

### Out of Scope

- `cheaper hook ...` subcommand 自体の設計・実装（DESIGN-034z1EhuztlLSb2x2XXxgD）
- `hooks.json` の command を native executable 呼び出しへ書き換える
  最終移行（同 design で実施）
- SEA executable 自体の build 詳細（DESIGN-034z1EfqYaEBfyNvqZyFSD）
- APM root `bin/` を使う配布（採用しない）
- Claude Code marketplace plugin 向けの PATH 導線

## Components and Boundaries

```text
packages/apm/.apm/hooks/
├── hooks.json
├── cheaper            # POSIX platform selector（薄い launcher）
├── cheaper.ps1        # Windows/PowerShell launcher
└── cli/
    └── bin/
        ├── darwin-arm64/cheaper
        ├── darwin-x64/cheaper
        ├── linux-arm64/cheaper
        ├── linux-x64/cheaper
        └── win32-x64/cheaper.exe
```

- **platform launcher**: dispatch のみを担う。business logic・hook
  payload の解釈は持たない。POSIX 版は `uname` で `darwin-arm64` 等を
  決定して `exec`、PowerShell 版は sibling `win32-x64/cheaper.exe` を
  起動して `$LASTEXITCODE` を伝播する。
- **`scripts/build-apm-bundle.mjs`**: `bundle-cli.mjs` の canonical
  bundle 出力と `build-executable.mjs` の platform executable を入力に、
  APM 配布 tree を組み立てる。standalone 配布と同じ executable
  artifact を再生成せずコピーする。
- **executable mode**: POSIX binary / launcher の実行 bit を APM
  install 後の target で確認する。権限が落ちる場合は install 後検証で
  検出する。

## Data and Control Flow

- build 時: `pnpm bundle` → `build-executable.mjs`（各 platform）→
  `build-apm-bundle.mjs` が launcher + executable + hooks.json を
  `packages/apm/.apm/hooks/` へ配置 → APM package に同梱
- consumer 実行時: agent の native hook → launcher → platform 判定 →
  `cli/bin/<platform>/cheaper[.exe]` に argv を委譲 → executable が
  通常 CLI として動作
- transitional 状態では adapter JS が残る期間があるが、canonical な終状態は
  launcher → executable 一経路とする

## Non-Functional Constraints

- APM install 後の consumer 環境に Node.js を要求しない（最終状態）
- launcher は外部依存を持たない POSIX sh / PowerShell で書く
- hooks の個別 target contract 差分は executable 側の `cheaper hook`
  subcommand が吸収し、launcher は target 非依存に保つ
- executable / launcher の path・mode は APM install の deploy 結果に
  依存するため、target 別の install 検証を必須とする

## Risks and Trade-offs

- launcher の薄さと hooks.json の command 表現がずれると hook が no-op
  化する → APM install e2e で各 target の deploy/config assert を持つ
- platform 追加時に launcher の判定表と binary 配置規約がずれるリスク
  → target matrix を build script 側の data に寄せ、launcher は命名規約
  だけを頼りにする
- APM package が肥大化する（platform 数 × executable size）→ 対象
  platform は spec の最低ラインから始めて必要時に拡張する

## Deferred Design Concerns

- executable mode / quarantine 属性の platform 別扱い | install 検証で
  実挙動を確認してから | 初回 APM install 検証で問題が出た時点 |
  consumer 環境で実行不能になる
- launcher の debug logging | fail-open 運用上の診断性 | hook が期待
  通り発火しない報告があった時点 | 障害切り分けが難しい

## Verification Notes

- [ ] `apm install` 後に supported target で launcher 経由の executable
      が起動できる
- [ ] POSIX / Windows で platform 判定と引数・exit code の伝播が確認
      できる
- [ ] hooks bundle 内の executable が外部 `workers/` / schema assets への
      依存なしに built-in resource を解決できる
- [ ] APM bundle の binary freshness を CI が検証する
- [ ] `cheaper-usage` skill の CLI location guidance が executable 基準に
      更新されている
