---
id: "TSPEC-034zXj8LyfXfYFb1U3o2tb"
type: "test-spec"
status: "approved"
title: "共有 operation admission と cancellation"
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
  refined-by: []
  derives-from:
    - "docs/designs/bounded-tool-scheduler.md"
  derived-by: []
  verifies:
    - "docs/specs/bounded-tool-scheduler.md"
    - "docs/adr/bounded-tool-scheduler.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 共有 operation admission と cancellation
## Purpose
adapter ごとの補償を復活させず、全入口で invocation-wide cap と mutation safety を維持する。
## Feature
canonical FIFO shared/exclusive scheduling と OperationManager の ownership。
## Rules
- Rule: shared read は cap まで overlap、exclusive mutation と全 operation は overlap しない。後続 read は待機 writer を追い越さない。
- Rule: budget を一回予約後に queued record を作り、capacity 取得時だけ running に遷移する。
- Rule: enqueue 前と admission critical section の queued+signal 確認で事前 abort/terminal を start しない。
- Rule: queued abort は original first-cause reason を保持し handler を呼ばない。running abort/grace detach は実 settle まで枠を解放しない。
- Rule: public descriptor/Pi/direct caller は同じ runtime.execute を通り二重 budget/meter を作らない。
## Examples
- Example: cap2の3 read は2 running+1 queued、1 settleで3件目がrunningになる。Pi sibling/direct scope 間でも cap2を共有する。
- Example: read→write→read は writer を追い越さず、write/edit は全 owner 間で直列化される。
- Example: queued context/owner/root cancel と TIMEOUT は queued interval と terminal reason を残し、他 owner の枠を誤って解放しない。
- Example: pre-aborted composite を渡すと record はcancelled/timed_outのままで、cancelled→running の invalid transition に変換されない。
- Example: cap1のnon-cooperative handler はgrace後も枠を保持し、late settle後だけ次のownerを開始できる。
- Example: same-turn sibling budget超過はoperation無しで拒否され、filesystem拒否はrejection1回、成功textはmeter1回。
- Example: public descriptor は registered definition と別 object。公開 execute をdecoratorに差し替えても内部raw handlerは変わらず、一度だけfacadeを通る。
- Example: standalone options.concurrency を省略すると shared operation は8件overlapし、9件目はqueued。
- Example: queued write(A)→edit(B)→write(C) は owner が異なっても submission order A→B→C で実行する。
## Guarantees
AC1–6/AC8 の overlap、global ordering、first-cause、record visibility、accounting と filesystem/writeScope/MutationJournal を保つ。
## Non-goals
per-path lease、process pool、Workflow DAG scheduling、実モデル smoke、全repo canonical check は保証しない。
## Risk
root cancellation の伝播途中のpumpと detach即release は安全性を壊す。決定的 gates/clock と既存 filesystem tests を使う。
