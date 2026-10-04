---
id: "TSPEC-034zTsc3i9wPmxIsvuuyva"
type: "test-spec"
status: "proposed"
title: "workflow-loader-contract"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
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
    - "docs/designs/workflow-file-interface.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-file-interface.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow loader の共通検証契約

## Purpose
file と dynamic definition の意味論の分岐を防ぐ。loader が canonical compiler を使わなくなる設計に変更する場合は保証を移管するまで廃止しない。

## Feature
明示 workflow file の安全な読込と invocation input validation。

## Rules
- Rule: schema/default/reference/DAG は既存 compiler の検証を通る。
- Rule: interface は必須 invocation 値なしで definition を検証する。
- Rule: defaults は clone され、unknown/missing/schema 不正 input を拒否する。
- Rule: 不在/読込不可/escape/deny/non-file は INVALID_REQUEST、malformed JSON は INVALID_INPUT。

## Examples
- Example: required target を宣言した file は load/project に成功し、compile の値省略では INVALID_INPUT。
- Example: schema version 不正、invalid default、cycle の file は同じ raw を compiler へ渡す場合と同じ code。
- Example: object default の返却値を変更しても次回の validation/default を変更しない。
- Example: ../outside.json、policy denied directory、symlink escape は読込前に拒否。

## Guarantees
AC3/4/6/7 を保護し、loader から得た definition/compiler は dynamic compile と同じ DAG/input を返す。

## Non-goals
Worker/tool execution、resource inference 実装、symlink 外部競合の追加保証。

## Risk
parallel validator は definition 表示と実行で異なる契約を作り、boundary 省略は workspace 外の読込を許す。
