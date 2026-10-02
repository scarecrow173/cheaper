---
id: "IMPL-034z3CD7wXV3MNmCy9JnHs"
type: "impl"
status: "completed"
title: "add platform launchers to hooks payload"
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
  implements:
    - "docs/tasks/add-posix-and-powershell-platform-launchers-to-hooks-payload.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# add platform launchers to hooks payload

## Summary

payload launcher 2 本を `packages/apm/.apm/hooks/` に追加した。
`cheaper`（POSIX sh）が `uname -s`/`uname -m` で target を判定して
`cli/bin/<target>/cheaper` に `exec` 委譲し、`cheaper.ps1` が
`cli/bin/win32-x64/cheaper.exe` を起動して `$LASTEXITCODE` を伝播する。
docs/plans/assemble-apm-executable-payload-with-platform-launchers.md task 1。

## Context

issue #71 Phase 3。launcher は dispatch のみを担い business
logic を持たない設計（DESIGN-034z1EgnQwax4VamK2xmdT）。

## Implementation

### Changes

- `hooks/cheaper`: uname 対応表で `linux|darwin|win32` ×
  `x64|arm64` を判定。Git Bash（MINGW/MSYS/CYGWIN）は win32-x64
  exe に委譲。未対応 OS/arch や executable 不在は stderr + 非 0
  exit（silent no-op 禁止）
- `hooks/cheaper.ps1`: sibling `cli/bin/win32-x64/cheaper.exe` を
  `&` 起動し `@args` 転送・`exit $LASTEXITCODE`
- POSIX launcher に git index で exec bit（100755）を付与

### Decisions During Implementation

- Git Bash 環境を `win32` にマッピングした（Windows 上の
  sh 系 hook shell が MINGW を返すため、Windows payload での
  動作確認に必須）
- fail 動作は明示的 stderr + exit 1。hook が黙って無効化
  されるより診断可能な失敗を優先

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `hooks/cheaper --version`（Git Bash）→ `1.0.0` exit 0
  （win32-x64 exe へ委譲を確認）
- `pwsh hooks/cheaper.ps1 --version` → `1.0.0` exit 0

## Risks

- 実 Linux/macOS での `uname` 経路は local では未実測
  （win32 mapping のみ確認）。linux target は release/CI での
  検証に委ねる
- `hooks.json` はまだ Node adapter を指す（transitional）

## Follow-ups

- Phase 4 で hooks.json を `cheaper hook` / launcher 経由に
  移行
