---
id: "TSPEC-034AJ8HPIAVAf1tzuWKVa5"
type: "test-spec"
status: "proposed"
title: "Workflow parent safety と Worker child permission の寿命"
created: "2026-10-05"
updated: "2026-10-06"
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
    - "../designs/one-shot-workflow-execution-and-cli-composition.md"
  derived-by: []
  verifies:
    - "../specs/one-shot-workflow-engine-and-run-cli.md"
  verified-by: []
  references: ["../test-specs/resource-manager-job-leases.md"]
  defers: []
  deferred-by: []
---

# Workflow parent safety と Worker child permission の寿命

## Purpose

job safety lease とliteral writeScopeを混同せず、invocation teardownが未settleの保護を壊さない保証を残す。lease ownership の代替契約導入時のみ退役を再評価する。

## Feature

Workflow parent safety と Worker child permission の寿命。

## Rules

- Rule: parent compiled safety の中で child writes/produces を canonical literal exact writeClaims に narrow。
- Rule: acquireJobLease caller がreleaseを所有し invocation dispose が一括解放しない。
- Rule: child token/operation capacity は actual settle まで、engine parent は executor actual finally まで保持。
- Rule: per-Worker budget/journal/timeout は他 Worker の lifetime を所有しない。

## Examples

- Example: mkdir ancestor は safetyに含まれてもchildのsibling write permissionにならない。
- Example: detached executorはchild token有/無の双方でinvocation dispose前後にparentを保持、actual finallyでrelease。
- Example: scope.ready自己取得leaseはscope.disposeで解放しsingle-workercleanupを維持。
- Example: Aのchild timeoutはBとoverall invocationをcancelしない。

## Guarantees

canonical ResourceAccess/ResourceManager containment/readStrictnessを再利用し、observed I/Oとjournalを同じscopeへ結線する。

## Non-goals

OS sandbox、ResourceManagerアルゴリズム変更、実モデル品質。

## Risk

early releaseによる競合write、scope拡大、他jobの意図しないabort。
