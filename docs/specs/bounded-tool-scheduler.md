---
id: "SPEC-034zWd1FiG9tv3X6uaAMep"
type: "spec"
status: "approved"
title: "ToolScheduler の bounded operation scheduling"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
  implements: []
  implemented-by: ["../plans/implement-bounded-tool-scheduler.md"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: ["../adr/bounded-tool-scheduler.md"]
  refined-by: []
  derives-from: ["../discovery/bounded-tool-scheduler.md"]
  derived-by: ["../designs/bounded-tool-scheduler.md", "../plans/implement-bounded-tool-scheduler.md"]
  verifies: []
  verified-by: ["../test-specs/bounded-concurrency-policy.md", "../test-specs/shared-tool-scheduler.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# ToolScheduler の bounded operation scheduling
## Problem
Pi sibling call と Workflow direct tool call が operation 全体の上限と mutation 順序を共有していない。
## Audience and Value
Worker/Workflow 利用者が filesystem safety と cancellation を維持して bounded parallel read を使える。
## Scope
read/grep/find/ls/count_tokens は shared、write/edit は exclusive。invocation の最大同時 operation 数を制限し、mutation 同士と read/mutation を同時実行しない。
## Non-Goals
Workflow DAG scheduling、per-path lease、process 実装、metrics/envelope/persistence、CLI option、dependency 追加は対象外。
## Requirements
共有 ADR D2 の execution.concurrency は maxOperations 正整数、mutations serialized | per-resource、既定 8/serialized。#61 未実装時は per-resource 設定でも保守的 exclusive 実行を維持する。
trusted builtin→user は replace、project は min cap/serialized 優先。legacy policy には trusted merge 後・project 前に欠けた concurrency field を補完する。partial omission に default を適用しない。
complete policy は concurrency 全体の省略を legacy 互換で許容するが、present 時は maxOperations と mutations を両方必須とする。field 単位の省略は partial policy のみ許容する。
## Acceptance Criteria
- AC1: Pi adapter と Workflow-style direct scope の独立 read は configured bound まで overlap し、共通 invocation cap を超えない。
- AC2: mutation 同士と read/mutation は overlap せず submission 順で実行する。
- AC3: queue の owner/root/context abort と TIMEOUT が handler を開始せず first-cause reason を保持する。
- AC4: bound 超過待機は queued record で可視、capacity 獲得時だけ running となり createdAt/startedAt で wait を観測できる。
- AC5: maxToolCalls/rejection/meter は一回だけ記録され、同時 start の budget 超過は operation を作らない。
- AC6: filesystem/denylist/writeScope/MutationJournal の既存 suite が通る。cancel grace detach は実 settle まで capacity を保持する。
- AC7: complete/partial validation、normalize、trusted/restrict merge、load、CLI-style restriction の境界が pin される。
- AC8: 公開 descriptor execute と Pi execute と direct execute が同一 scheduling boundary を通る。
