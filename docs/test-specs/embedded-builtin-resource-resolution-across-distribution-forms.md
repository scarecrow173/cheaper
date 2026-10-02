---
id: "TSPEC-034z1P09dg86lO9uB31cUi"
type: "test-spec"
status: "proposed"
title: "Embedded builtin resource resolution across distribution forms"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md"
  derived-by: []
  verifies:
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Embedded builtin resource resolution across distribution forms

## Purpose

built-in resources（`workers/`・`policy.yaml`・`version.json`）が
`builtinDir` を起点とする同一の解決規則で、通常 Node 実行 / APM
bundle / SEA standalone executable の 3 配布形態すべてで解決されること
を保証する意図を記録する。resource-root 解決規則が廃止・置換された
場合にのみ retire する。

## Feature

builtinDir を起点とした built-in resource の解決

## Rules

- `workers/` の worker definition と JSON schema は `builtinDir` からの
  相対解決で読める
- `policy.yaml` は `builtinDir` 直下から解決される
- `version.json` は `builtinDir` 基準で解決され、`package.json` への
  直接依存を持たない
- SEA executable では `builtinDir` は VFS root（`import.meta.dirname`）
  を指し、real FS の `workers/` 等が存在しなくても解決できる
- 通常 Node 実行と APM bundle 実行は従来どおり real FS 上の
  `builtinDir` で同じ規則を使う（表現ドリフトさせない）

## Examples

- fake/test runtime で standalone executable に対し built-in worker を
  指定した run を実行すると、worker definition が VFS 内 `workers/`
  から解決され schema load が成功する
- real FS に `workers/` を置かないディレクトリで executable を起動しても
  built-in worker resolution が成立する
- `--version` が VFS 内 `version.json` から version を返す

## Guarantees

- 3 配布形態で `builtinDir` 経由の解決規則が一貫し、形式ごとの別解決
  パスが増殖しない
- executable が外部 worker/schema asset へ依存しない（APM 配布の
  nested asset 制約を回避できる）

## Non-goals

- `.node` native addon の VFS load（不可能なことが確定している）
- user/project config dir の探索規則そのもの（`resolveConfigDirs` の
  builtin 以外の経路は既存仕様）
- tiktoken WASM の VFS 内 load 可否の最終判定方法（PoC で実測し、
  失敗時は real FS 退避設計に移す）

## Risk

- この保証が落ちると SEA executable が外部 asset に依存して「単一
  executable 配布」が崩れ、APM payload でも同じ問題が再発する。
  builtinDir 解決が形態ごとに分岐すると保守不能な表現ドリフトになる
