---
id: "TASK-034zXj9La05SfBh0LjDsHA"
type: "task"
status: "done"
title: "Invocation 共有 ToolScheduler を導入する"
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
  implements:
    - "docs/plans/implement-bounded-tool-scheduler.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-bounded-tool-scheduler.md"
    - "docs/tasks/bounded-concurrency-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/shared-tool-scheduler.md"
  references: []
  defers: []
  deferred-by: []
---

# Invocation 共有 ToolScheduler を導入する
## Goal
全canonical entry pathでinvocation-wide boundとshared/exclusive safetyを保証する。
## Scope
tool-runtime src/test、runtime-pi run/tools registryの最小配線とtests。workflow compiler/CLI/package manifests/依存には触れない。
## Steps
1. bound/serialization/FIFO/cancel/preabort/late-settle/budget/descriptor と Pi/direct 共通cap のfailing testsを追加しREDを確認する。
2. manager queued recordを先に作るToolSchedulerを実装し、queued+signalの同期確認を両boundaryに置く。
3. invocationはmanager+schedulerを共有、standaloneはoptions.concurrency/resolverで作る。registryはisMutatingToolでaccessを導出しpublic descriptorを別objectにする。
4. Piのpolicy wiringを接続する。旧並行ownership testsはshared toolで契約を維持する。
5. focused suites/lint/typecheck/build、ir/evidenceとscoped auditを完了してcommitする。
## Done When
AC1–6/AC8が決定的testでpass、capを実settleまで保持し、filesystem/writeScope/journalの既存testがpassする。
## Verification
- [x] mise exec -- pnpm --filter @cheaper/tool-runtime test がexit0（87 tests）。
- [x] mise exec -- pnpm --filter @cheaper/runtime-pi test がexit0（371 tests）。
- [x] mise exec -- pnpm lint がexit0（既存33 warnings）。
- [x] mise exec -- pnpm typecheck がexit0。
- [x] mise exec -- pnpm build がexit0。

実装記録: ../impl/ir/shared-tool-scheduler.md。coordinator review msg_38cc34bdc53b は APPROVED: implementation、P0/P1 blocker無し。re-entrant admission guard は追加 RED→GREEN を確認した。
## Risks / Follow-ups
per-resource設定でも本sliceはexclusive、#61がleaseを担当。process宣言seamは#59。canonical check deferred to coordinator merge run。
