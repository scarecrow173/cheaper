---
id: "DESIGN-034A8ubRdZsG57LdsNNTEw"
type: "design"
status: "approved"
title: "Operation execution metrics"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  source: ["https://github.com/scarecrow173/cheaper/issues/58"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../specs/operation-execution-metrics.md","../adr/operation-execution-metrics.md"]
  derived-by: ["../plans/operation-execution-metrics.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md","overview.md"]
  defers: []
  deferred-by: []
---

# Operation execution metrics

## Context

共有 ADR D1/D3/D7 の型と lifecycle を変更せず、passive snapshot を追加する。

## Scope

operation 計測、Worker meta、CLI error forwarding と検証。usage.jsonl と scheduler は変更しない。

## Components and Boundaries

- ExecutionMetrics: tool-runtime の単一定義。pure collectExecutionMetrics(records, snapshotAt, observedMaxConcurrency) は六状態を 0 初期化し、toolName??kind で byTool を集計する。
- OperationManager.executionMetrics(ownerId?): 自身の monotonic clock を一度読み、owner filtered records と owner-local の観測済み high-water を reducer に渡す。start/finish の passive counter は scheduling を変更しない。invocation options.clock は deterministic test 用の同一 clock seam。owned Worker は RunWorkerDeps.operationClock を createInvocationToolRuntime.clock へ渡す。borrowed Worker の clock は提供済み invocation のものを使用する。
- InvocationToolRuntime.executionMetrics(ownerId?): manager に委譲する。Workflow は owner 無しで呼ぶ。
- WorkerRunMeta.execution: ExecutionMetrics を追加。buildMeta は既存 usage/savings と一緒に owner snapshot を返す。outer catch は invocation があれば既存 CheaperError の code/details/cause と partial runMeta を保った WorkerRunError に snapshot を付加する。unknown error は既存 envelope と同じ INTERNAL_ERROR とする。service 作成前は従来通り。
- ErrorEnvelopeMeta.execution: WorkerRunMeta の型を参照し、partial execution を forwarding。未生成は null。

## Data and Control Flow

createdAt→start/terminal/snapshot が queue duration。startedAt→finishedAt/snapshot が running duration。終了時刻は snapshotAt 以下に clamp。正の長さの半開区間だけを sweep し、同時刻の delta をまとめる。running duration 合計と union の差が overlapMs、maxConcurrency は manager が観測した high-water を使用する（timestamp だけでは同時刻の 0 duration の順序を復元できない）。cancel 要求と detach では区間を終了しない。

snapshot は scope teardown 前の瞬間値であり、unsettled の将来の実 settle を先取りしない。

## Non-Functional Constraints

snapshot は records を変更せず scheduler の admission に hook を追加しない。manager の start/finish で passive running count/high-water を更新する。model/provider への呼出しや extra turn はない。

## Risks and Trade-offs

0 duration 区間は duration/overlap に寄与しないが、maxConcurrency は lifecycle の high-water で保持する。同時刻境界と serial/overlap の 0 duration を test する。Workflow で Worker snapshot を足すと union/max を誤るため invocation 集合から再計算する。

## Verification Notes

reducer fake clock、scheduler unchanged、Worker success/usage-bearing failure/early failure、CLI forwarding/null、owner isolation と既存 usage/savings 回帰。
