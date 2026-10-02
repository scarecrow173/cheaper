---
id: "TSPEC-034z3yKWLolrImpMY8UMhZ"
type: "test-spec"
status: "proposed"
title: "APM executable payload layout and launcher dispatch contract"
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
    - "docs/designs/apm-executable-payload-layout-and-platform-launchers.md"
  derived-by: []
  verifies:
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# APM executable payload layout and launcher dispatch contract

## Purpose

issue #71 Phase 3 の APM payload が「launcher 経由で platform 別
executable が起動し、consumer の Node.js 非依存を損なわない」ことを
保証する意図を記録する。launcher が誤判定や silent no-op を起こさず、
executable の引数・exit code を忠実に伝播することを守る。payload
layout が再設計された場合にのみ retire する。

## Feature

APM bundle 内 launcher 経由の platform executable 起動

## Rules

- `packages/apm/.apm/hooks/` に `cheaper`（POSIX）と `cheaper.ps1`
  （PowerShell）が存在し、いずれも外部依存を持たない
- POSIX launcher は `uname -s` / `uname -m` から
  `cli/bin/<platform>-<arch>/cheaper` を解決して `exec` する。
  未対応 platform では stderr に通知して非 0 で終了する
  （silent no-op ではない）
- PowerShell launcher は `cli/bin/win32-x64/cheaper.exe` を起動し、
  引数を転送して `$LASTEXITCODE` を伝播する
- `cli/bin/<platform>/` 配下の executable は `.gitattributes` の
  LFS rule で管理され、repo には pointer のみ入る
- `hooks.json` と既存 JS adapter は本 spec の対象範囲で不変
  （Phase 4 で移行）
- `build-apm-bundle.mjs` は `--exe-dir` の executable を命名規則
  `cheaper-<platform>-<arch>[.exe]` → `cli/bin/<platform>/cheaper[.exe]`
  に配置する。`--exe-dir` 未指定時は executable を配置しない

## Examples

- POSIX 環境で `hooks/cheaper --version` を実行すると launcher が
  `linux-x64` 等を判定して `cli/bin/linux-x64/cheaper --version` に
  `exec` し、version 文字列と exit 0 が返る
- Windows で `pwsh hooks/cheaper.ps1 count-tokens file.md` を実行
  すると `cli/bin/win32-x64/cheaper.exe` に転送され envelope が返る
- `git add` した `cli/bin/win32-x64/cheaper.exe` が LFS pointer
  （`version https://git-lfs...`）として index に入る
- darwin host で launcher を実行すると「unsupported platform」を
  stderr に出して非 0 exit（macOS binary 未同梱のため）

## Guarantees

- launcher は business logic を持たず argv・exit code をそのまま
  伝播する
- executable が存在しない platform では fail する（hook が黙って
  no-op にならない）
- LFS 管理により git object store に巨大 binary が直に入らない

## Non-goals

- `hooks.json` が launcher を指す移行（Phase 4 の `cheaper hook`
  subcommand 化で実施）
- macOS target の payload
- APM install 実環境での e2e（本 spec は local 再現と構造検証まで）

## Risk

- この保証が落ちると、consumer の hook が platform 判定ミスや
  silent 失敗で機能しなくなる。LFS 化を外すと repo が binary で
  肥大化する
