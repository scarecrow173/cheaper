---
id: "IMPL-034A6gXDwnMxGVaPiuSCxb"
type: "impl"
status: "completed"
title: "Shared tool scheduler"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/57"]
  changes:
    added: [{"type":"code","path":"packages/tool-runtime/src/scheduler.ts"},{"type":"code","path":"packages/tool-runtime/test/scheduler.test.ts"},{"type":"code","path":"packages/runtime-pi/test/tool-scheduler.test.ts"}]
    modified: [{"type":"code","path":"packages/tool-runtime/src/registry.ts"},{"type":"code","path":"packages/tool-runtime/src/invocation.ts"},{"type":"code","path":"packages/tool-runtime/src/index.ts"},{"type":"code","path":"packages/tool-runtime/src/operation.ts"},{"type":"code","path":"packages/tool-runtime/test/invocation.test.ts"},{"type":"code","path":"packages/runtime-pi/src/run.ts"},{"type":"code","path":"packages/runtime-pi/src/tools/registry.ts"}]
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/shared-tool-scheduler.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Shared tool scheduler

## Summary

全 owner が共有する FIFO ToolScheduler と standalone の同一 admission を実装した。read/shared は cap まで並行、mutation/exclusive は全 running operation の settle を待つ。

## Context

TASK-034zXj9La05SfBh0LjDsHA / PLAN-034zWZgd5LEtlTQNGC5O8x、承認済み DESIGN-034zWclUCjCIH7M27XEu4j と shared ADR D1/D2/D7 に従う。core policy task は f136611 で完了済み。

## Implementation

### Changes

manager.create の queued record を capacity 待ちより先に作り、enqueue/admission 両境界で最新 queued state と signal を確認する。queued abort は waiter と listener を除去し composite signal.reason を返す。root/owner abort の同期伝播中の admission を避けるため abort 後 pump を microtask に回す。
running の abort/grace detach は capacity を解放せず、実 execute の finally だけが解放する。既知 tool の access は core isMutatingTool を registry が呼び出して決める。
public definitions は raw handler と別 object の execute wrapper にし、validation → 同期 budget reservation → scheduler → meter/raw handler を一回通す。runtime-pi は standalone invocation に policy concurrency を渡し、Pi adapter と Workflow-style direct scope は同じ scheduler を共有する。

### Decisions During Implementation

ownership test の並行 write fixture を shared ls に改め、元の concurrent cancellation/late-settle 契約を保持した。mutation 排他は scheduler test に独立して固定する。invocation policy が scope override に対して authoritative であることも検証した。
Pi fixture の fauxToolCall 引数順と edit fixture の edits 配列を catalog/library contract に修正した。修正済み Pi fixture は一時的に baseline admission を復元して RED を確認し、try/finally で source を戻して再 build した（生成物は commit 対象外）。最初の baseline build は旧 ToolScheduler placeholder と新 export の重複で失敗したため、旧 placeholder のみ除いた baseline で確認した。
追加 dispatch 禁止と coordinator review 担当の明示に従い、requesting-code-review の subagent dispatch は coordinator ask に置き換える。
この supervised dispatch は明示された task/skill 手順を持つため、implementation-flow の SUBAGENT-STOP を適用し profile 再生成は行わない。TDD / incremental-implementation / impl-doc / git-commit / requesting-code-review を inline で実行する。

## Related Experiments

### Adopted

なし。既知設計の実装のため exp は作らなかった。

### Rejected

なし。

## Validation

RED: scheduler.test.ts 12/12 failures、exit1。修正済み Pi fixture の baseline admission は direct/a/b/c 全4件を開始し cap2を破り、queued call が running となる2 failures、exit1。
GREEN: mise exec -- pnpm --filter @cheaper/tool-runtime test = 5 files / 87 tests passed、exit0（新規13件を含む）。
GREEN: mise exec -- pnpm --filter @cheaper/runtime-pi test = 34 files / 371 tests passed、exit0（新規2件と filesystem/denylist/writeScope/MutationJournal suites を含む）。Pi sibling 4件に budget3 を設定し、受理3件/拒否1件、onLimit一回、meter accepted resultのみも固定した。
mise exec -- pnpm lint = exit0、既存 workflow の warning33件。mise exec -- pnpm typecheck = exit0、mise exec -- pnpm build = exit0。
initial typecheck は optional concurrency の exactOptionalPropertyTypes で失敗し、legacy policy の undefined を option 型で許可した後 GREEN。
coordinator review は msg_38cc34bdc53b で APPROVED: implementation、P0/P1 blocker無し。P2 の re-entrant enqueue は追加 RED（1 failure / 13 tests）で abort 伝播中に新 caller の admission が起きることを確認し、pumpDeferred guard を追加して全87件を GREEN にした。キャンセル対象 handler の開始自体は再現しなかったが、guard は伝播終了まで全 admission を保留する。修正後 lint/typecheck/build と Pi371件も exit0。
OperationManager.run が bound を持たない低水準 lifecycle API であることを source comment に明記した。残りの P2 notes は以下の Risks に記録し、blocker として扱わない。
scoped exit-audit は chain と共有 ADR に blocking 0件。共有 ADR の既存 reciprocal warnings 2件、全体の既存 blocking 41件は範囲外（test-spec-invalid-verifies-target 12 / test-spec-missing-verifies 3 / traceability-missing-upstream 15 / unresolved-legacy-reference 11）。

## Graph Checkpoint

run-until-yield、focus PLAN-034zWZgd5LEtlTQNGC5O8x、frozen taskBudgetCount=2、maxHops=11。
probe-to-briefing → briefing-to-design → design-to-planning → planning-to-task-graph → task-graph-to-implementation は承認と policy commit f136611 により完了済み。
implementation-retry は本 scheduler task の検証と review を完了した。保持した commit baseline は f136611 / dirty=[]、waiver は無い。本記録を含む論理変更の commit 後に clean worktree を確認して checkpoint を閉じる。Phase E review は msg_38cc34bdc53b を ask し、timeout 後も同じ question を resume して承認を受領した。
quota recovery 後の実装 entry projection は同じ edge で active scheduler を選択した。元の task entry route は f181952 / f136611 の文書 snapshot を一時的に再 projection して復元し、try/finally で current canonical files を戻した。最終報告は完全な route、stable fingerprint、typed outcomes と最終 yield を保持する。

## Risks

FIFO head-of-line と global mutation 排他は意図的保守性。non-cooperative handler は実 settle まで cap を保持する。per-resource vocabulary も現時点では exclusive。canonical check deferred to coordinator merge run。bundled CLI は再生成していない。
coordinator の non-blocking P2: 公開 low-level manager を外部から不正遷移させる misuse path では signal.reason が undefined、または complete/fail throw により record が unsettled になり得る。invocation の manager は private で、この misuse は canonical runtime path から行えない。
legacy wrapToolBudget / wrapToolMeter は既に accounting 済みの canonical runtime/Pi descriptors に併用しない（二重計上になる）。本 wiring は併用していない。

## Follow-ups

process poolは#59、resource leaseは#61、Workflow DAG engineは#62、metricsは#58へ既存非目標として分類する。scope内の未実装事項は無い。impl/ir index は承認済み例外(2)で自分のrow statusだけを同期する。
