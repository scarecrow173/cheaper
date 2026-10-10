---
id: "TSPEC-034z1OZC4sSkRlvZfFfIXt"
type: "test-spec"
status: "superseded"
title: "Standalone executable contract parity"
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
  superseded-by: ["TSPEC-034DmUtUjWCThCnJulok0c"]
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

# Standalone executable contract parity

## Purpose

issue #71 の SEA standalone executable が、通常の Node.js 実行
（`node cheaper.mjs` / `pnpm dev` 経路）と同一の CLI contract を持つ
ことを保証する意図を記録する。consumer が Node.js を持たない環境でも
既存挙動が損なわれないことを守る。配布形態が廃止された場合にのみ
retire する。

## Feature

Standalone executable での CLI command 実行（contract parity）

## Rules

- Node.js が PATH に存在しない環境でも executable が起動し command を
  完了できる
- stdout は worker run 系 command で 1 つの result envelope JSON のみを
  出力し、diagnostics は stderr にのみ出る
- exit code は通常 Node 実行の CLI と同一規則（成功 0、usage error、
  runtime error 等）に従う
- `--version` / `count-tokens` / `models` / `auth status` は provider
  認証を要さず standalone で動作する
- executable 起動は外部ファイル（`workers/`・`package.json` 等の real FS
  asset）を要求しない

## Examples

- PATH から Node.js を除去した shell で `cheaper-windows-x64.exe
  --version` を実行すると、packages/cli/package.json の version 文字列
  が stdout に出力され exit 0 で終了する
- 同条件で `count-tokens <file>` を実行すると token 数 envelope が
  stdout に 1 件だけ出力される
- 存在しないコマンドを実行した場合の stderr diagnostics と exit code
  が通常 Node 実行と一致する

## Guarantees

- spec の AC「worker stdout は 1 result envelope」「diagnostics は
  stderr」「exit code は既存 CLI と同一」を standalone 形態でも満たす
- `useVfs` の VFS asset は read-only として扱われ、write 試行は
  `EROFS` 相当で失敗する前提が壊れない

## Non-goals

- provider への実接続（OAuth フロー・実 provider smoke）は検証しない
- 起動速度・binary size の最適化目標は検証しない（size 記録のみ）
- cross-platform matrix 全 target の網羅（Phase 1 は current host のみ）

## Risk

- この保証が落ちると、Node.js 非依存配布という issue #71 の主目的が
  成立しなくなる。また envelope/exit code の silent な乖離は
  parent-agent の hook/委譲契約を破壊する
