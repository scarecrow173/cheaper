---
id: "DESIGN-034zSP4AJ6aRzdtIxTHW1Y"
type: "design"
status: "approved"
title: "OperationManager の state と bounded drain"
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
  derives-from:
    - "docs/specs/operation-manager-lifecycle.md"
    - "docs/adr/operation-manager-settlement.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# OperationManager の state と bounded drain

## Context and Scope

共有 ADR D1/D3/D7 に準拠。manager は tool-runtime に配置し invocation が 1 個所有する。
standalone createToolRuntime は private manager を所有し、invocation の scope は同じ manager を借り ownerId を固定する。
scheduler、ResourceManager/lease、metrics/envelope、Workflow 統合は含まない。core 型と hot file は変更しない。

## Components and Boundaries

- operation.ts: OperationState と検証可能な state transition、OperationManager、snapshot、cancel/drain/dispose。clock は performance.now()、test は clock port を使う。
- registry.ts: validation/admission 後に manager へ登録し、実 handler Promise の fulfilled/rejected で settle を通知。既存 accounting/meter の wrapper を維持する。
- invocation.ts: 1 manager を scope に注入。owner ごとの abort/drain、root signal の全 cancel、root dispose。外部へ manager deps bag を出さず facade の readonly operation snapshot を提供する。
- runtime-pi/run.ts: requestAbort が CheaperError(code, reason) を signal.reason に渡すだけの最小 wiring。run signature/meta/result は変更しない。

## Data and Control Flow

1. create/queue は queued record と per-operation AbortController を作る。per-operation signal は owner scope signal + call-context signal + operation controller の composite。first-cause reason wins。owner cancel はその owner の admission gate も閉じる。ID は manager 内連番、ownerId/kind/toolName/resourceClaims と createdAt を固定する。
2. start は queued のみ許可。startedAt を記録し state=running。complete/fail は running の実 settle 通知で、結果/失敗を保存する。
3. cancel は queued を即 terminal にし、running は cancellationRequestedAt/reason と signal.abort のみ。全 terminal 遷移（queued の同期 cancel/timeout を含む）で finishedAt を stamp。既存 terminal への明示 cancel は invalid transition、cancelAll は terminal を skip。
4. reason.code=TIMEOUT または reason.name=TimeoutError を timed_out とし、他は cancelled。first-cause を変えない。run budget reason の code は保持する。
5. tracked run の settle は cancel の有無で completed/failed または cancelled/timed_out を選ぶ。settlement=settled と finishedAt はこの時だけ記録する。
6. drain は対象 owner を cancel して allSettled と既存 abortGraceMs（default 5000）の finite grace を race。未 settle は settlement=unsettled、owned=false と detachedAt を記録し、state=running/finishedAt absent のまま戻る。runtime-owned active は 0。
7. late settle は detached record を terminal にするが ownership は復活させない。execute の result/throw は元どおり返し、unhandled rejection を残さない。
8. dispose は新規 admission を封じ、signal listener を解除する。borrowed manager の sibling/root は child dispose で停止しない。

## Admission and Observability

OperationManager.run は accounting の admitted callback 内から呼び、UNKNOWN_TOOL/INVALID_INPUT/limit は records に残さない。
handler 内の boundary 拒否は failure として記録する。既存 budget は toolCalls と rejectedToolCalls を両方増加するため、この区別を維持する。
toolCalls は admission 時、rejectedToolCalls は拒否時の既存規則を維持する。
内部 snapshot に signal、state、timestamps、cancellation reason、settlement、owned、terminal result/error が含まれる。
resource claims と返却 record の mutable lifecycle fields は copy し、consumer が state を直接変更できないようにする。result は executor が返した値を保持し、深い immutable value の契約は導入しない。
この snapshot は内部 runtime consumer 用で parent-visible polling API ではない。

## Risks and Trade-offs

grace 超過の detached operation は filesystem/process の継続可能性を持つ。cancelled や finished を偽装せず明示的に記録する。
実停止が保証できない処理は後続 process executor が自分の cleanup/kill/settle を実装する。
records は invocation の lifetime 内に限り保持し永続化しない。

## Verification Notes

- 六状態の全 transition matrix と first-cause/late-settle tests。
- 非協力 handler と owner scope の bounded disposal、finite grace validation、sibling independence。
- canonical input/result/meter/journal accounting regressions、run timeout/signal/budget reason tests。
- canonical check deferred to coordinator merge run。

## Graph Trace

mode=run-until-yield、probe-to-briefing。既存 accepted ADR を references で再利用し briefing-profile は dispatch の明示 chain で override。
coordinator msg_df657964221d: shared ADR は references のみ、必要時は local ADR、focus-required が残れば delegate 順を手動維持し plan focus で再開する承認済み例外。
新規 chain の artifacts を focus とし、design approval-required で coordinator review に yield する。

2026-10-04 coordinator ask receipt: APPROVED: spec/design approval（P0/P1 なし）。per-op composite signal、terminal stamp、既存 grace、既存 accounting を上記へ反映。共有 ADR は承認 reply の「references のままでも可」に従い references に保持する。
