---
id: "IMPL-034z3CEPSQBJaEVBVxxzmL"
type: "impl"
status: "completed"
title: "verify launchers and apm payload"
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
    - "docs/tasks/verify-launchers-and-apm-payload-assembly-locally.md"
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

# verify launchers and apm payload

## Summary

Phase 3 payload の local 検証を完了した。launcher 経由の
executable 起動、bundle 組み立て、LFS pointer 化、`mise run
check` がすべて pass。

## Context

docs/plans/assemble-apm-executable-payload-with-platform-launchers.md task 5。TSPEC-034z3zCdYOkbq0TsOfO8yfeC の
contract を local 再現の範囲で検証。

## Implementation

### Changes

コード変更なし（検証のみ）。

### Decisions During Implementation

- `apm install` 後の deploy 検証は consumer target が絡むため
  本 task では tree 構造と launcher 直接実行までに限定

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `./cheaper --version`（Git Bash、hooks dir）→ `1.0.0`
- `pwsh ./cheaper.ps1 --version` → `1.0.0`・exit code 伝播
- `node scripts/build-apm-bundle.mjs --exe-dir dist/sea` →
  `cli/bin/win32-x64/cheaper.exe` 配置
- `pnpm bundle` → tracked file に diff なし（exe dir 保持）
- `git lfs ls-files` に `cli/bin/win32-x64/cheaper.exe` 列挙、
  index 上は pointer
- `mise run check` 全緑（lint・version・doc-id・typecheck・
  859+ tests・bundle freshness）

## Risks

- linux/darwin 上の POSIX launcher 実挙動は未実測（uname
  分岐は実装済み、Windows 上の Git Bash で win32 経路のみ
  確認）
- APM install 後の deploy path・exec bit は consumer 環境での
  初回 install 検証に依存

## Follow-ups

- Phase 4: `cheaper hook` subcommand 化と hooks.json 移行
- linux runner での launcher smoke は release workflow の拡張
  で扱う余地あり
