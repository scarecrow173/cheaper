---
id: "SPEC-034zSPnVDi6fpHVc9ELgLW"
type: "spec"
status: "approved"
title: "Invocation-scoped OperationManager lifecycle"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/56"]
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
  derives-from: ["../discovery/operation-manager-lifecycle.md"]
  derived-by: ["../designs/operation-manager-lifecycle.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Invocation-scoped OperationManager lifecycle

## What Should Be Built

Invocation 内の tool 操作を共有 runtime が追跡する。consumer は Pi Worker と後続 Workflow tool adapter。
共有 ADR D1 の package 境界、D3 の実 settle までの running 区間、D7 の owned/borrowed lifetime を守る。

## Why Now

#63 の pending 集合だけでは cancel 要求と実 settle を区別できず、timeout・失敗・遅延処理を観測できない。

## Scope

### In Scope

- 六状態、stable invocation ID、ownerId/kind/tool、monotonic timestamp、abort linkage、optional core ResourceClaim、result/error metadata。
- create/queue/start/complete/fail/cancel、owner/global cancellation、bounded drain/dispose。
- canonical tool invocation の admission 後の追跡と run abort の first-cause 連動。

### Out of Scope

- scheduler、resource arbitration/lease、Workflow engine、metrics/envelope、永続化、daemon、parent polling、Worker wake。
- canonical input/result・journal・meter の意味変更、Pi tool implementation の書換え、依存追加。

## Required Behavior

created operation は queued。queued → running または cancelled/timed_out、running → completed/failed/cancelled/timed_out のみ。
terminal からの遷移は拒否する。cancel は first-cause を保持し、queued は未開始なので即 terminal、running は signal を abort して settle 観測まで running を保つ。
cancel 後の fulfilled/rejected settle は requested reason に対応する terminal に確定し、実際の結果/失敗も保持する。
無協力 handler は grace 超過時に未 settle と明示し runtime ownership を終了する。finished timestamp は付与せず、実停止を主張しない。late settle は追跡 record を terminal 化する。
scope disposal は自身の owner のみを終了し、root disposal は全 owner を終了する。既存 execute Promise の戻り値/throw は維持する。
admission 拒否（未知 tool、不正入力、上限超過）は operation に数えない。handler 内の boundary 拒否は admitted operation の failure として記録し、既存 rejectedToolCalls accounting を保持する。

## Acceptance Criteria

- [ ] AC1: 六状態の全組合せで許可/拒否が検証され、terminal は不変。
- [ ] AC2: root/child/call AbortSignal が対象だけを abort し first-cause の timeout/cancel reason を保持する。
- [ ] AC3: cancel 要求では running を終了しない。settle で terminal timestamp/result/error が観測できる。
- [ ] AC4: grace 超過は未 settle を明示し owned active=0 で finite return。late settle を安全に観測する。
- [ ] AC5: root/child dispose は idempotent、pre-aborted signal と budget trigger が fail closed、sibling ownership は維持される。
- [ ] AC6: canonical result/journal/meter と拒否 accounting の既存 focused tests が GREEN。

## Validation

tool-runtime focused suite、必要な runtime-pi abort tests、lint/typecheck/build、chain + shared ADR の scoped exit-audit。
canonical check deferred to coordinator merge run。
