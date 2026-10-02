---
id: "TASK-034z3zOydB4GluAraLc16E"
type: "task"
status: "done"
title: "Add POSIX and PowerShell platform launchers to hooks payload"
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
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
  implemented-by:
    - "docs/impl/ir/add-platform-launchers-to-hooks-payload.md"
  depends-on:
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Add POSIX and PowerShell platform launchers to hooks payload

## Work

`packages/apm/.apm/hooks/cheaper`（POSIX sh）と `cheaper.ps1`
（PowerShell）を新規作成する。launcher は dispatch のみを担い、
business logic を持たない。

- POSIX `cheaper`: `uname -s`（`Linux`→`linux`）と `uname -m`
  （`x86_64`→`x64`、`aarch64|arm64`→`arm64`）で target を決定し、
  script dir 基準で `cli/bin/<target>/cheaper` に全引数を `exec`。
  未対応 platform または executable 不在時は stderr にメッセージを
  出して非 0 exit（silent no-op 禁止）。macOS は binary 未同梱だが
  `darwin` 判定は表に残し executable 不在で fail する
- `cheaper.ps1`: sibling `cli/bin/win32-x64/cheaper.exe` を `&` で
  起動し `@args` を転送、`exit $LASTEXITCODE`

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/assemble-apm-executable-payload-with-platform-launchers.md
- docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md

## Dependencies

- なし（新規ファイル）

## Affected Files

- `packages/apm/.apm/hooks/cheaper`（新規）
- `packages/apm/.apm/hooks/cheaper.ps1`（新規）

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->
