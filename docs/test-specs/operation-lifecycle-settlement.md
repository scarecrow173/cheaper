---
id: "TSPEC-034zTjObSJGccLlnwmHwVm"
type: "test-spec"
status: "draft"
title: "Operation lifecycle settlement guarantees"
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
    - "docs/designs/operation-manager-lifecycle.md"
  derived-by: []
  verifies:
    - "docs/specs/operation-manager-lifecycle.md"
    - "docs/adr/operation-manager-settlement.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Operation lifecycle の settle guarantees

## Purpose

cancel 要求を停止証明に置き換える退行を検出する。六状態と実 settle/ownership の契約が維持される間は本保証を残す。

## Feature

invocation 内の operation が abort/disposal と独立に実際の settle を観測する。

## Rules

- Rule state: queued→running/cancelled/timed_out、running→completed/failed/cancelled/timed_out。terminal からの遷移は不正。
- Rule first-cause: owner/call/controller の composite signal の先行 reason が勝つ。TIMEOUT code、TimeoutError code、TimeoutError name は timed_out、他は cancelled。
- Rule settle: running cancel の時刻は finishedAt ではない。queued terminal と実 settle terminal の両方で finishedAt を stamp。
- Rule ownership: owner cancellation は admission gate を閉じ、他 owner を停止しない。
- Rule bounded drain: 既存 abortGraceMs（default 5000、promptWithAbortGrace と同じ有限 grace 契約）で返る。grace 超過は owned=false/settlement=unsettled/detachedAt を固定 field とし、finishedAt absent。

## Examples

- Example queued timeout: createdAt=10、cancelAt=20 → timed_out、startedAt absent、finishedAt=20。
- Example running signal: start=20、cancel=30 では running、settle=40 で cancelled/finishedAt=40。実結果/失敗も取得できる。
- Example timeout code mapping: reason.code=TIMEOUT → named assertion timed_out。
- Example TimeoutError code/name mapping: code=TimeoutError と name=TimeoutError の両 branch → named assertions timed_out。
- Example other reason: MAX_TURNS_EXCEEDED/MAX_TOOL_CALLS_EXCEEDED/ABORTED → cancelled、元 code を保持。
- Example non-cooperative executor: grace 超過で unsettled/owned=false/detachedAt present、遅延 fulfill/reject の観測で terminal。
- Example sibling: A dispose 後も B execute と新規 owner C が成功する。

## Guarantees

全 state transition matrix、terminal 不変、finite grace 検証、root/owner/call cancellation、pre-aborted signal、actual settle を runtime tests で pin する。
handler 実行前の不正入力/未知 tool/budget rejection は operation を作らず、handler failure は result/throw を維持する。

## Non-goals

process kill の実停止保証、scheduler/lease、Workflow DAG、metrics、provider semantic evaluation。

## Risk

この保証が失われると未停止 mutation を成功裏の disposal と誤報し、sibling Worker を無関係に停止させる。
