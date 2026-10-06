---
id: "DESIGN-034zWclUCjCIH7M27XEu4j"
type: "design"
status: "approved"
title: "ToolScheduler の共有 admission 設計"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../specs/bounded-tool-scheduler.md", "../adr/bounded-tool-scheduler.md"]
  derived-by: ["../plans/implement-bounded-tool-scheduler.md", "../test-specs/bounded-concurrency-policy.md", "../test-specs/shared-tool-scheduler.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# ToolScheduler の共有 admission 設計
## Context
共有 ADR D1 package 依存、D2 policy、D3 timestamps、D7 invocation ownership を維持する。overview は読み取りのみ。
## Scope
canonical scheduler と core policy、runtime-pi 最小 wiring。Workflow compiler は変更せず invocation.createWorkerScope の direct execution を Workflow adapter port として検証する。
## Non-Goals
#59 managed process、#61 lease、#62 engine、#58 metrics は実装しない。
## Components
ToolScheduler は OperationManager と concurrency options を受け取り、run(options, declaration, execute) を提供する。declaration は shared/exclusive access を明示する seam で、process pool は #59 が追加する。
declaration.access は shared | exclusive literal とし、既知 tool は caller 宣言を受けず registry が core isMutatingTool から導出する。untyped operation（将来の process）のみ declaration seam を使う。
invocation は manager+scheduler を一つずつ所有し全 owner scope に同じ service を渡す。standalone runtime も scheduler を必ず作る。scope cap を別の上限として重ねず invocation policy が authoritative。
standalone runtime は options.concurrency を値源とし、省略値は core の共有 resolveConcurrencyPolicy の既定 8/serialized とする。bound scope は invocation の policy を値源とする。
## Data and Control Flow
validation → 同期 createToolBudget.run reservation → scheduler.run で manager.create queued → FIFO admission → manager.start → meter/handler → manager.complete/fail → finally capacity release。
public definitions は execute が runtime.execute に戻る descriptor。raw registered handler は private closure に残す。
public descriptor は raw registered definition と別 object にし、raw execute を書き換えず無限再帰を防ぐ。
read は maxOperations まで同時実行。exclusive head は running=0 まで待ち、その後一件のみ。後続 read は writer を追い越さない。
queue abort listener は待機 entry を除去し original composite signal.reason で reject する。OperationManager 自身が queued cancellation/first cause を確定する。running cancel で release しない（late settle の finally でのみ release）。
manager.create は既 terminal record を返し得るため enqueue 前と admission critical section の両方で最新 record.state === queued と signal.aborted を同期確認し、非 queued または abort 済みなら signal.reason で reject して start しない。pre-abort の facade 拒否も original signal.reason を返す。queue abort 後の pump は microtask に回し root cancel の伝播途中に他 record を start しない。
## Policy Changes (approval required)
core policy/schema.ts に共有 ConcurrencyPolicy 型と default、legacy optional complete schema と field optional partial schema、normalized execution.concurrency を追加。
complete policy は concurrency 全体の省略のみ legacy 互換で許容し、concurrency present 時は maxOperations と mutations の両 field を必須とする。partial policy のみ各 field の省略を許容する。
既存 typed fixture との互換のため RuntimePolicy.execution.concurrency は optional とし canonical resolver が欠落 field を default 補完する（normalizePolicy の返却には常に存在）。
merge.ts は options.mode=replace | restrict-concurrency を追加（default replace）。restriction は D2 exact paths のみ、他 scalar/array は現行規則。
load.ts は trusted user merge 後 default 補完、project restrict merge。restrict.ts に optional concurrency restriction を追加し上限緩和を POLICY_VIOLATION とする。runtime-pi/run.ts は policy concurrency を invocation に渡す。
default 補完と値検証は core resolveConcurrencyPolicy の一実装に集約し、load/normalize/restrict と runtime の読み取り側すべてが同じ resolver を使う。
## Trade-offs
global shared/exclusive は per-path 並列性を持たないが、write/read races を避ける。FIFO は head-of-line 待機を許容し fairness を優先。unsettled handler は capacity を消費し続けるが cancellation に従わない handler で cap を破らない。
## Test Strategy
deferred handler と fake clock で overlap/queue/cancel/late settle を決定的検証。Pi helper の実 filesystem suite と direct invocation suite、core policy focused suite を実行する。
## Approval
coordinator が spec-design-policy を承認（msg_f31bba98b959）。core schema/merge/load/restrict + tests を含む。canonical check deferred to coordinator merge run。
