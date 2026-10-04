---
id: "ADR-034zWjPbCrCggmdSxHvnzR"
type: "adr"
status: "accepted"
title: "Bounded tool scheduler"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
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
  refined-by: ["../specs/bounded-tool-scheduler.md"]
  derives-from: ["../discovery/bounded-tool-scheduler.md"]
  derived-by: ["../designs/bounded-tool-scheduler.md"]
  verifies: []
  verified-by: ["../test-specs/shared-tool-scheduler.md"]
  references: ["./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# ToolScheduler の FIFO shared/exclusive admission

## Context and Problem Statement
共有 ADR D1/D2/D7 が配置と policy と ownership を固定した。本 slice では公平性と cancellation 時の capacity 所有を具体化する。
## Decision Drivers
invocation-wide cap、read/write safety、OperationManager first-cause、non-cooperative handler の実並行上限。
## Considered Options
- adapter-local semaphore: adapter 横断の上限と順序が保証できず却下。
- FIFO shared/exclusive admission を canonical runtime に置く: 所有境界が一つで安全なため採用候補。
- aborted handler を即時 release: unsettled の実並行数が cap を超えるため却下。
## Decision Outcome
coordinator が spec-design-policy を承認（msg_f31bba98b959）。ToolScheduler は OperationManager の queued record を admission 前に作り、FIFO shared/exclusive と manager.start を同一同期 critical section に置く。
running handler が settle する finally でのみ枠を release。abort 待機は listener を除去し original reason で reject。Mutation 判定は core isMutatingTool が正本。
per-resource policy を受理しても #61 がない本 slice は conservative exclusive を維持する。公開 runtime.definitions.execute も facade を通す。
## Consequences
global read/write exclusion は保守的で head-of-line 待機が生じる。process 独自 pool は宣言引数を受ける seam から #59 が追加し、本 slice は実装しない。
## Implementation Plan
scheduler.ts と registry/invocation と core policy を変更。runtime-pi の invocation 作成に policy を渡す。core schema/merge/load/restrict、scheduler cancellation/late-settle、Pi/direct overlap、filesystem suite で検証する。
依存追加・CLI/workflow compiler・root hot file 変更は行わない。
