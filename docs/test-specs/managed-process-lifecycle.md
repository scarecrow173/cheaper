---
id: "TSPEC-034A9yliLanxBMr6oB85Z7"
type: "test-spec"
status: "approved"
title: "managed process の bounded lifecycle と adapter 共有"
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
    - "../designs/managed-process-canonical-tool.md"
  derived-by: []
  verifies:
    - "../specs/managed-process-canonical-tool.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# managed process の bounded lifecycle と adapter 共有

## Purpose

process が canonical lifecycle と bounded diagnostics を共有することを保つ。実行機能廃止時のみ退役可能。

## Feature

Feature: shell-free process の pool、capture、cancel、Pi/Workflow 共有。

## Rules

- Rule: argv は literal、env は explicit/minimal、stdin は ignore。
- Rule: scheduler global/exclusive/pool の全条件を満たしてから spawn、capacity は実 settle まで保持。
- Rule: stdout/stderr 合計65536 bytes、非ゼロ exit は failure、cancel は元 reason を維持し管理下の tree を終了。
- Rule: Pi と Workflow は同じ executor/result/accounting を使う。

## Examples

- Example: 多量 stdout/stderr と binary 出力が cap 内に収まる。
- Example: pool=1/maxOperations=2 で二番目 process と後続 ls が FIFO queue、queued abort 後 ls が進む。
- Example: running task の parent/child PID が cancel 後に終了する。
- Example: SEA execPath + native Node PATH は native Node に解決、PATH miss/shim は拒否。

## Guarantees

- 非ゼロ exit と spawn error を成功 result に変換しない。Pi/Workflow metadata のみの取得で model/session は作らない。

## Non-goals

- setsid/re-parent 孫、正常終了後の孫残留、OS tracing、sandbox、real-model/provider 検証。

## Risk

- orphan、出力による memory 増大、cap 超過、adapter による二重 accounting が起きる。
